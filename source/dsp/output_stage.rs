//! Masteraid Coloration Output Stage (`source/dsp/output_stage.rs`)
//! Adapts and constrains Airwindows MIT algorithms:
//! - `Tube2` for `Triode`, `Push-Pull`, and `Pentode` amplifier models
//! - `PurestDrive` for `FET`, `MOSFET`, and `Diode` solid-state models
//! - `Coils2` for `Steel`, `Nickel`, `Amorphous`, and `Mu-Metal` transformer cores
//! - 3-State Harmonic Drive Intensity (`0 = Off / Subtle`, `1 = On / Normal`, `2 = Smashed Glass / Extreme Drive`)

use super::shared::{db_to_gain, AirwindowsBiquad, AirwindowsDither, AutoLevel, StereoFrame};

/// Measured RMS level offset of each amplifier model at -18 dBFS pink noise, per harmonic
/// intensity (0 = subtle, 1 = normal, 2 = smashed). Cancelling it means the amplifier changes
/// timbre without moving the gain structure. Values come from the offline gain sweep in
/// `examples/measure_gain.rs`.
const AMP_TRIM_DB: [[f64; 3]; 7] = [
    [0.0, 0.0, 0.0],       // Bypass
    [-1.08, -2.10, -2.64], // Triode
    [-0.84, -1.86, -2.77], // PushPull
    [-0.53, -1.38, -2.46], // Pentode
    [-0.05, -0.57, -1.59], // FET
    [-0.05, -0.59, -1.66], // MOSFET
    [0.07, -0.27, -0.80],  // Diode
];

/// Same idea for the transformer cores (amp bypassed during measurement).
const TRANS_TRIM_DB: [[f64; 3]; 5] = [
    [0.0, 0.0, 0.0],    // Bypass
    [0.25, 0.27, 0.26], // Steel
    [0.17, 0.18, 0.15], // Nickel
    [0.09, 0.10, 0.10], // Amorphous
    [0.04, 0.05, 0.05], // MuMetal
];

fn calibrated_trim_db(amp: AmpModel, trans: TransformerModel, intensity: i32) -> (f64, f64) {
    let i = intensity.clamp(0, 2) as usize;
    (
        AMP_TRIM_DB[amp as usize][i],
        TRANS_TRIM_DB[trans as usize][i],
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AmpModel {
    Bypass = 0,
    Triode = 1,
    PushPull = 2,
    Pentode = 3,
    Fet = 4,
    Mosfet = 5,
    Diode = 6,
}

impl AmpModel {
    pub fn from_index(idx: i32) -> Self {
        match idx {
            1 => Self::Triode,
            2 => Self::PushPull,
            3 => Self::Pentode,
            4 => Self::Fet,
            5 => Self::Mosfet,
            6 => Self::Diode,
            _ => Self::Bypass,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransformerModel {
    Bypass = 0,
    Steel = 1,
    Nickel = 2,
    Amorphous = 3,
    MuMetal = 4,
}

impl TransformerModel {
    pub fn from_index(idx: i32) -> Self {
        match idx {
            1 => Self::Steel,
            2 => Self::Nickel,
            3 => Self::Amorphous,
            4 => Self::MuMetal,
            _ => Self::Bypass,
        }
    }
}

pub struct OutputStage {
    sample_rate: f64,
    dither: AirwindowsDither,
    // Airwindows Tube2 & PurestDrive state
    prev_sample_a: f64,
    prev_sample_b: f64,
    prev_sample_c: f64,
    prev_sample_d: f64,
    prev_sample_e: f64,
    prev_sample_f: f64,
    purest_prev_l: f64,
    purest_prev_r: f64,
    // Airwindows Coils2 transformer state
    biquad_a: AirwindowsBiquad,
    biquad_b: AirwindowsBiquad,
    hysteresis_l: f64,
    hysteresis_r: f64,
    last_trans_idx: i32,
    last_intensity: i32,
    /// Level-neutral operation (per-module AUTO switch).
    compensate: bool,
    /// Slow residual trim that catches programme-dependent offset left by the calibration tables.
    /// Disabled while a learned AUTO trim is in charge, so the stage never rides gain.
    auto_level: AutoLevel,
    auto_level_enabled: bool,
    /// Target of the learned residual trim (dB). Smoothed into `residual_db` while processing.
    residual_target_db: f64,
    residual_db: f64,
    residual_coeff: f64,
    /// Reserved distortion actuator: input gain that is compensated again on the way out, so it
    /// changes how hard the stage is driven without moving the level.
    pre_gain_db: f64,
}

impl Default for OutputStage {
    fn default() -> Self {
        let mut s = Self {
            sample_rate: 48000.0,
            dither: AirwindowsDither::default(),
            prev_sample_a: 0.0,
            prev_sample_b: 0.0,
            prev_sample_c: 0.0,
            prev_sample_d: 0.0,
            prev_sample_e: 0.0,
            prev_sample_f: 0.0,
            purest_prev_l: 0.0,
            purest_prev_r: 0.0,
            biquad_a: AirwindowsBiquad::default(),
            biquad_b: AirwindowsBiquad::default(),
            hysteresis_l: 0.0,
            hysteresis_r: 0.0,
            last_trans_idx: -1,
            last_intensity: -1,
            compensate: true,
            auto_level: AutoLevel::default(),
            auto_level_enabled: true,
            residual_target_db: 0.0,
            residual_db: 0.0,
            residual_coeff: 1.0,
            pre_gain_db: 0.0,
        };
        s.auto_level.set_sample_rate(48000.0);
        s.update_residual_coeff();
        s.update_coils_filters(1, 0);
        s
    }
}

impl OutputStage {
    /// Per-module AUTO switch: when on, the stage aims for unity gain so the coloration only
    /// changes timbre. Toggling resets the residual trim so it re-converges from unity.
    pub fn set_compensation(&mut self, on: bool) {
        if self.compensate != on {
            self.compensate = on;
            self.auto_level.reset();
        }
    }

    pub fn compensation(&self) -> bool {
        self.compensate
    }

    /// Enable or disable the slow RMS rider. A learned AUTO trim works on the true residual, so
    /// the rider must stand down: otherwise it would keep moving gain with the programme, which
    /// is exactly what the latch is meant to avoid.
    pub fn set_auto_level_enabled(&mut self, on: bool) {
        if self.auto_level_enabled != on {
            self.auto_level_enabled = on;
            self.auto_level.reset();
        }
    }

    /// Learned residual correction for the current PUSH state (dB). Applied on top of the static
    /// tables and smoothed over ~250 ms so switching PUSH states never clicks.
    pub fn set_residual_trim_db(&mut self, db: f64) {
        self.residual_target_db = db.clamp(-12.0, 12.0);
    }

    /// Reserved for the Phase 2 distortion actuator. The gain is applied before the stage and
    /// removed after it, so only the drive changes, not the level.
    pub fn set_pre_gain_db(&mut self, db: f64) {
        self.pre_gain_db = db.clamp(-6.0, 6.0);
    }

    fn update_residual_coeff(&mut self) {
        let sr = self.sample_rate.max(22050.0);
        self.residual_coeff = 1.0 - (-1.0 / (sr * 0.25)).exp();
    }

    pub fn set_sample_rate(&mut self, sr: f32) {
        let new_sr = (sr as f64).max(22050.0);
        self.auto_level.set_sample_rate(new_sr as f32);
        if (new_sr - self.sample_rate).abs() > 1.0 {
            self.sample_rate = new_sr;
            self.last_trans_idx = -1;
            self.update_residual_coeff();
        }
    }

    pub fn reset(&mut self) {
        self.prev_sample_a = 0.0;
        self.prev_sample_b = 0.0;
        self.prev_sample_c = 0.0;
        self.prev_sample_d = 0.0;
        self.prev_sample_e = 0.0;
        self.prev_sample_f = 0.0;
        self.purest_prev_l = 0.0;
        self.purest_prev_r = 0.0;
        self.biquad_a.reset();
        self.biquad_b.reset();
        self.hysteresis_l = 0.0;
        self.hysteresis_r = 0.0;
        self.auto_level.reset();
        self.residual_db = 0.0;
    }

    fn update_coils_filters(&mut self, trans_idx: i32, intensity: i32) {
        if trans_idx == self.last_trans_idx && intensity == self.last_intensity {
            return;
        }
        self.last_trans_idx = trans_idx;
        self.last_intensity = intensity;

        let drive_boost = match intensity {
            2 => 1.85, // Smashed Glass: deep core saturation
            1 => 1.25, // Normal Push: nominal musical drive
            _ => 0.75, // Subtle / Off: gentle passive transformer presence
        };

        let (core_freq, core_q, hf_lp): (f64, f64, f64) =
            match TransformerModel::from_index(trans_idx) {
                TransformerModel::Steel => (480.0, 0.045 * drive_boost, 19500.0),
                TransformerModel::Nickel => (720.0, 0.028 * drive_boost, 21800.0),
                TransformerModel::Amorphous => (600.0, 0.016 * drive_boost, 22000.0),
                TransformerModel::MuMetal => (550.0, 0.009 * drive_boost, 22000.0),
                TransformerModel::Bypass => (600.0, 0.01, 22000.0),
            };

        self.biquad_a
            .set_bandpass(core_freq, core_q.max(0.008), self.sample_rate);
        self.biquad_b
            .set_lowpass(hf_lp.min(self.sample_rate * 0.48), 0.89, self.sample_rate);
    }

    /// Processes a stereo frame through the selected Amplifier, Transformer, and 3-state Harmonic Drive Intensity
    /// (`intensity`: `0` = Subtle/Off, `1` = Normal/On `+6dB`, `2` = Smashed Glass Extreme `+18dB`).
    pub fn process_with_params(
        &mut self,
        frame: StereoFrame,
        amp_idx: i32,
        trans_idx: i32,
        intensity: i32,
    ) -> StereoFrame {
        let amp = AmpModel::from_index(amp_idx);
        let trans = TransformerModel::from_index(trans_idx);
        if amp == AmpModel::Bypass && trans == TransformerModel::Bypass {
            return frame;
        }

        let mut l = frame.left as f64;
        let mut r = frame.right as f64;
        self.dither.guard_denormals(&mut l, &mut r);
        let pre = db_to_gain(self.pre_gain_db as f32) as f64;
        l *= pre;
        r *= pre;
        let in_l = l;
        let in_r = r;
        let (amp_trim_db, trans_trim_db) = calibrated_trim_db(amp, trans, intensity);
        let amp_trim = db_to_gain(amp_trim_db as f32) as f64;
        let trans_trim = db_to_gain(trans_trim_db as f32) as f64;

        let overallscale = (self.sample_rate / 44100.0).max(0.5);

        // Operating point scaling based on 3-state Harmonic Drive Push button:
        // 0 = Subtle (early nonlinear region), 1 = Normal (+4.5 dB internal drive), 2 = Smashed Glass (+11.5 dB extreme saturation)
        let (drive_gain, wet_blend) = match intensity {
            2 => (3.20, 0.88), // Smashed Glass: extreme harmonic saturation
            1 => (1.65, 0.62), // Pushed ON: warm audible harmonic drive
            _ => (1.08, 0.34), // Idle / Subtle: gentle analog presence
        };

        // =========================================================================
        // STAGE 1: AMPLIFIER TOPOLOGY (Airwindows Tube2 & PurestDrive adaptations)
        // =========================================================================
        if amp != AmpModel::Bypass {
            let dry_l = l;
            let dry_r = r;

            let mut xl = (l * drive_gain).clamp(-1.0, 1.0);
            let mut xr = (r * drive_gain).clamp(-1.0, 1.0);

            if overallscale > 1.9 {
                let stored_l = xl;
                xl = (xl + self.prev_sample_a) * 0.5;
                self.prev_sample_a = stored_l;
                let stored_r = xr;
                xr = (xr + self.prev_sample_b) * 0.5;
                self.prev_sample_b = stored_r;
            }

            match amp {
                AmpModel::Triode | AmpModel::PushPull | AmpModel::Pentode => {
                    // Airwindows Tube2: asymmetric cathode sharpen + powerfactor polynomial + slew hysteresis
                    let (powerfactor, asym_depth, slew_depth): (i32, f64, f64) = match amp {
                        AmpModel::Triode => (2, 0.25, 0.30), // Warm even-order 2nd harmonic emphasis
                        AmpModel::PushPull => (3, 0.06, 0.20), // Balanced symmetric odd-order tube drive
                        AmpModel::Pentode => (5, 0.22, 0.52), // Dense higher-order harmonics + stronger slew
                        _ => (2, 0.20, 0.25),
                    };
                    let asym_pad = powerfactor as f64;
                    let gainscaling = 1.0 / ((powerfactor + 1) as f64);
                    let outputscaling = 1.0 + (1.0 / (powerfactor as f64));

                    // Asymmetric cathode bias section (Tube2 lines 53-68)
                    xl /= asym_pad;
                    let sharpen_l = if -xl > 0.0 {
                        1.0 + (-xl).sqrt()
                    } else {
                        1.0 - xl.sqrt()
                    };
                    xl -= xl * xl.abs() * sharpen_l * asym_depth;
                    xl *= asym_pad;

                    xr /= asym_pad;
                    let sharpen_r = if -xr > 0.0 {
                        1.0 + (-xr).sqrt()
                    } else {
                        1.0 - xr.sqrt()
                    };
                    xr -= xr * xr.abs() * sharpen_r * asym_depth;
                    xr *= asym_pad;

                    // Powerfactor polynomial transfer curve (Tube2 lines 73-84)
                    let mut factor_l = xl;
                    for _ in 0..powerfactor {
                        factor_l *= xl;
                    }
                    if powerfactor % 2 == 1 && xl != 0.0 {
                        factor_l = (factor_l / xl) * xl.abs();
                    }
                    xl = (xl - factor_l * gainscaling) * outputscaling;

                    let mut factor_r = xr;
                    for _ in 0..powerfactor {
                        factor_r *= xr;
                    }
                    if powerfactor % 2 == 1 && xr != 0.0 {
                        factor_r = (factor_r / xr) * xr.abs();
                    }
                    xr = (xr - factor_r * gainscaling) * outputscaling;

                    // Tube2 hysteresis slew (Tube2 lines 95-120, constrained for mastering headroom)
                    let mut slew_l = self.prev_sample_e - xl;
                    self.prev_sample_e = xl;
                    slew_l = if slew_l > 0.0 {
                        1.0 + slew_l.sqrt() * slew_depth
                    } else {
                        1.0 - (-slew_l).sqrt() * slew_depth
                    };
                    xl -= xl * xl.abs() * slew_l * gainscaling * 0.5;

                    let mut slew_r = self.prev_sample_f - xr;
                    self.prev_sample_f = xr;
                    slew_r = if slew_r > 0.0 {
                        1.0 + slew_r.sqrt() * slew_depth
                    } else {
                        1.0 - (-slew_r).sqrt() * slew_depth
                    };
                    xr -= xr * xr.abs() * slew_r * gainscaling * 0.5;
                }
                AmpModel::Fet | AmpModel::Mosfet => {
                    // Airwindows PurestDrive: previous-sample FM-modulated sine saturation
                    // Lets high transients through cleanly while thickening the body
                    let p_intensity = if amp == AmpModel::Fet {
                        (0.45 * drive_gain).min(0.95)
                    } else {
                        (0.26 * drive_gain).min(0.75)
                    };
                    let sin_l = xl.sin();
                    let apply_l = ((self.purest_prev_l + sin_l).abs() * 0.5 * p_intensity).min(1.0);
                    self.purest_prev_l = xl.sin();
                    xl = xl * (1.0 - apply_l) + sin_l * apply_l;

                    let sin_r = xr.sin();
                    let apply_r = ((self.purest_prev_r + sin_r).abs() * 0.5 * p_intensity).min(1.0);
                    self.purest_prev_r = xr.sin();
                    xr = xr * (1.0 - apply_r) + sin_r * apply_r;
                }
                AmpModel::Diode => {
                    // Asymmetric diode clipper with Airwindows bridge-rectifier sine wrap
                    let pos_l = if xl >= 0.0 {
                        (xl * 1.35).sin() / 1.35
                    } else {
                        (xl * 0.95).tanh()
                    };
                    let pos_r = if xr >= 0.0 {
                        (xr * 1.35).sin() / 1.35
                    } else {
                        (xr * 0.95).tanh()
                    };
                    xl = pos_l;
                    xr = pos_r;
                }
                AmpModel::Bypass => {}
            }

            // Auto-compensate drive gain so level stays calibrated while harmonics bloom
            let comp_l = xl / drive_gain.powf(0.78);
            let comp_r = xr / drive_gain.powf(0.78);
            l = dry_l * (1.0 - wet_blend) + comp_l * wet_blend;
            r = dry_r * (1.0 - wet_blend) + comp_r * wet_blend;
        }
        if self.compensate {
            l *= amp_trim;
            r *= amp_trim;
        }

        // =========================================================================
        // STAGE 2: TRANSFORMER CORE HYSTERESIS (Airwindows Coils2 adaptation)
        // =========================================================================
        if trans != TransformerModel::Bypass {
            self.update_coils_filters(trans_idx, intensity);
            let dry_l = l;
            let dry_r = r;

            let (dist_scaling, iir_base, trans_wet, sheen_boost) = match trans {
                TransformerModel::Steel => (0.52 / drive_gain.sqrt(), 0.042, 0.65, 0.0),
                TransformerModel::Nickel => (0.68 / drive_gain.sqrt(), 0.025, 0.58, 0.06),
                TransformerModel::Amorphous => (0.85 / drive_gain.sqrt(), 0.015, 0.45, 0.03),
                TransformerModel::MuMetal => (1.05 / drive_gain.sqrt(), 0.009, 0.32, 0.0),
                TransformerModel::Bypass => (1.0, 0.01, 0.0, 0.0),
            };

            // Coils2 lines 52-84: notch clean midrange, apply magnetic core hysteresis to extremes
            let (clean_mid_l, clean_mid_r) = self.biquad_a.process_single(l, r);
            let mut diff_l = (dry_l - clean_mid_l) / dist_scaling.max(0.08);
            let mut diff_r = (dry_r - clean_mid_r) / dist_scaling.max(0.08);
            let (flt_l, flt_r) = self.biquad_b.process_single(diff_l, diff_r);
            diff_l = flt_l;
            diff_r = flt_r;

            let iir_amount = (iir_base / overallscale).clamp(0.0005, 0.45);
            self.hysteresis_l = self.hysteresis_l * (1.0 - iir_amount) + diff_l * iir_amount;
            diff_l = (diff_l - self.hysteresis_l).clamp(-1.571, 1.571);
            let hyst_clamped_l = self.hysteresis_l.clamp(-1.571, 1.571);

            self.hysteresis_r = self.hysteresis_r * (1.0 - iir_amount) + diff_r * iir_amount;
            diff_r = (diff_r - self.hysteresis_r).clamp(-1.571, 1.571);
            let hyst_clamped_r = self.hysteresis_r.clamp(-1.571, 1.571);

            let coil_l = dry_l
                + (diff_l.sin() - hyst_clamped_l.sin()) * dist_scaling * 0.42
                + (dry_l - clean_mid_l) * sheen_boost;
            let coil_r = dry_r
                + (diff_r.sin() - hyst_clamped_r.sin()) * dist_scaling * 0.42
                + (dry_r - clean_mid_r) * sheen_boost;

            l = dry_l * (1.0 - trans_wet) + coil_l * trans_wet;
            r = dry_r * (1.0 - trans_wet) + coil_r * trans_wet;
        }
        if self.compensate {
            l *= trans_trim;
            r *= trans_trim;
            if self.auto_level_enabled {
                // Mop up whatever the tables could not predict (programme- and level-dependent).
                let trim = self.auto_level.process(in_l, in_r, l, r);
                l *= trim;
                r *= trim;
            }
        }

        // Remove the reserved pre-gain so the actuator only changes drive, never level.
        if (pre - 1.0).abs() > 1.0e-12 {
            l /= pre;
            r /= pre;
        }

        // Learned residual for the current PUSH state, smoothed so state changes never click. It is
        // part of AUTO compensation, so it only acts while the module's AUTO switch is engaged.
        self.residual_db += (self.residual_target_db - self.residual_db) * self.residual_coeff;
        if self.compensate && self.residual_db.abs() > 1.0e-9 {
            let trim = db_to_gain(self.residual_db as f32) as f64;
            l *= trim;
            r *= trim;
        }

        StereoFrame::new(l as f32, r as f32)
    }

    pub fn process(&mut self, frame: StereoFrame) -> StereoFrame {
        self.process_with_params(frame, 1, 1, 0)
    }
}
