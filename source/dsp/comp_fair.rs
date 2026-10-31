//! Masteraid Fair-Inspired Fast V-MU Mastering Compressor (`source/dsp/comp_fair.rs`)
//! Adapts Airwindows `Pressure5` (`Pressure5Proc.cpp`) & `VariMu` (`VariMuProc.cpp`):
//! - Interleaved dual-envelope (`flip`) mu-coefficient detector (`muSpeedA/B`, `muCoefficientA/B`)
//! - Slew-modulated `mewiness` soft-to-firm variable-mu knee
//! - Sidechain HPF (`OFF..250 Hz`), Parallel `MIX` (`0..100%`), `MAKEUP` gain, and stereo GR telemetry

use super::shared::{db_to_gain, gain_to_db, AirwindowsBiquad, AirwindowsDither, StereoFrame};

#[derive(Clone, Copy, Debug)]
pub struct CompConfig {
    pub threshold_db: f32,
    /// Input drive ahead of the vari-mu stage (Fair INPUT GAIN).
    pub input_gain_db: f32,
    pub ratio: f32,
    pub attack_ms: f32,
    pub release_ms: f32, // <= 0.0 means AUTO program-dependent release
    pub sc_hpf_hz: f32,
    pub mix_pct: f32,
    pub makeup_db: f32,
    /// Engage program-dependent auto makeup (driven by the smoothed gain reduction).
    pub auto_makeup: bool,
}

pub struct CompFair {
    sample_rate: f64,
    flip: bool,
    dither: AirwindowsDither,
    sc_hpf: AirwindowsBiquad,
    last_sc_freq: f32,
    // Airwindows Pressure5 / VariMu dual-interleaved state (Left & Right)
    mu_speed_al: f64,
    mu_speed_bl: f64,
    mu_coeff_al: f64,
    mu_coeff_bl: f64,
    mu_speed_ar: f64,
    mu_speed_br: f64,
    mu_coeff_ar: f64,
    mu_coeff_br: f64,
    last_sample_l: f64,
    last_sample_r: f64,
    slew_max: f64,
    pub gr_db_l: f32,
    pub gr_db_r: f32,
    // Auto makeup state
    gr_avg: f64,
    auto_gain: f64,
    gr_avg_coeff: f64,
    auto_coeff: f64,
    pub auto_makeup_db: f32,
}

impl Default for CompFair {
    fn default() -> Self {
        let mut c = Self {
            sample_rate: 48000.0,
            flip: false,
            dither: AirwindowsDither::default(),
            sc_hpf: AirwindowsBiquad::default(),
            last_sc_freq: -1.0,
            mu_speed_al: 10000.0,
            mu_speed_bl: 10000.0,
            mu_coeff_al: 1.0,
            mu_coeff_bl: 1.0,
            mu_speed_ar: 10000.0,
            mu_speed_br: 10000.0,
            mu_coeff_ar: 1.0,
            mu_coeff_br: 1.0,
            last_sample_l: 0.0,
            last_sample_r: 0.0,
            slew_max: 0.0,
            gr_db_l: 0.0,
            gr_db_r: 0.0,
            gr_avg: 1.0,
            auto_gain: 1.0,
            gr_avg_coeff: 0.00001,
            auto_coeff: 0.0001,
            auto_makeup_db: 0.0,
        };
        c.reset();
        c
    }
}

impl CompFair {
    pub fn set_sample_rate(&mut self, sr: f32) {
        let new_sr = (sr as f64).max(22050.0);
        if (new_sr - self.sample_rate).abs() > 1.0 {
            self.sample_rate = new_sr;
            self.last_sc_freq = -1.0;
            // ~2 s average of the gain reduction: auto makeup is gain staging, not an envelope.
            self.gr_avg_coeff = 1.0 - (-1.0 / (new_sr * 2.0)).exp();
            // ~250 ms ramp on the applied makeup so engaging it never clicks.
            self.auto_coeff = 1.0 - (-1.0 / (new_sr * 0.25)).exp();
        }
    }

    pub fn reset(&mut self) {
        self.flip = false;
        self.sc_hpf.reset();
        self.mu_speed_al = 10000.0;
        self.mu_speed_bl = 10000.0;
        self.mu_coeff_al = 1.0;
        self.mu_coeff_bl = 1.0;
        self.mu_speed_ar = 10000.0;
        self.mu_speed_br = 10000.0;
        self.mu_coeff_ar = 1.0;
        self.mu_coeff_br = 1.0;
        self.last_sample_l = 0.0;
        self.last_sample_r = 0.0;
        self.slew_max = 0.0;
        self.gr_db_l = 0.0;
        self.gr_db_r = 0.0;
        self.gr_avg = 1.0;
        self.auto_gain = 1.0;
        self.auto_makeup_db = 0.0;
    }

    pub fn process_with_config(&mut self, frame: StereoFrame, cfg: &CompConfig) -> StereoFrame {
        // INPUT GAIN: drive the tube stage. On the real 670 this attenuator sits ahead of the
        // amplifier that compresses, so it changes how hard the signal hits the detector without
        // touching the threshold itself.
        let drive = db_to_gain(cfg.input_gain_db) as f64;
        let mut l = frame.left as f64 * drive;
        let mut r = frame.right as f64 * drive;
        self.dither.guard_denormals(&mut l, &mut r);

        let dry_l = l;
        let dry_r = r;

        let overallscale = (self.sample_rate / 44100.0).max(0.5);

        // Sidechain HPF (for bass-heavy mastering material)
        let (sc_l, sc_r) = if cfg.sc_hpf_hz > 15.0 {
            if (cfg.sc_hpf_hz - self.last_sc_freq).abs() > 0.5 {
                self.last_sc_freq = cfg.sc_hpf_hz;
                self.sc_hpf.set_highpass(
                    cfg.sc_hpf_hz as f64,
                    std::f64::consts::FRAC_1_SQRT_2,
                    self.sample_rate,
                );
            }
            self.sc_hpf.process_single(l, r)
        } else {
            (l, r)
        };

        // THRESHOLD is a real level, not a knob position: the detector clamps everything above it,
        // so a -12 dB setting starts working at -12 dBFS. The knob's endpoints are a per-model
        // calibration in the editor; here we only respect the detector's usable window.
        let threshold = (db_to_gain(cfg.threshold_db) as f64).clamp(0.01, 0.99);

        // Time constants constrained to FAIR fast variable-mu behavior
        let att_factor = (cfg.attack_ms.clamp(0.1, 30.0) as f64 / 10.0)
            .sqrt()
            .clamp(0.15, 2.5);
        let rel_norm = if cfg.release_ms <= 0.0 {
            0.45 // Auto multi-stage release
        } else {
            (cfg.release_ms.clamp(50.0, 2000.0) as f64 / 2000.0).powf(0.35)
        };
        let release = ((1.28 - (1.0 - rel_norm) * 0.75).powi(5) * 32768.0) / overallscale;
        let fastest = release.sqrt() * att_factor;

        // Stereo-linked with 80% link + 20% independent L/R so GR L and GR R needles breathe naturally
        let abs_l = sc_l.abs();
        let abs_r = sc_r.abs();
        let max_lr = abs_l.max(abs_r);
        let sense_l = max_lr * 0.80 + abs_l * 0.20;
        let sense_r = max_lr * 0.80 + abs_r * 0.20;

        // Pressure5 lines 79-151: mewiness + interleaved flip detector
        let ratio_norm = ((cfg.ratio.clamp(1.2, 20.0) - 1.2) / 18.8) as f64;
        let mewiness = (0.35 + ratio_norm * 0.55 + self.slew_max * 0.15).clamp(0.15, 0.98);

        // ButterComp2 hot-signal release slowdown: prevents low-frequency ripple/distortion on heavy bass
        let hot_slow_l = 1.0 + sense_l * 0.35;
        let hot_slow_r = 1.0 + sense_r * 0.35;

        if self.flip {
            // Left A
            if sense_l > threshold {
                let mu_vary = (threshold / sense_l.max(1.0e-6)).min(1.0);
                let mu_attack = (self.mu_speed_al.abs().sqrt() * att_factor).max(1.5);
                self.mu_coeff_al = (self.mu_coeff_al * (mu_attack - 1.0) + mu_vary) / mu_attack;
            } else {
                let spd_sq = (self.mu_speed_al * self.mu_speed_al).max(2.0);
                self.mu_coeff_al = (self.mu_coeff_al * (spd_sq - 1.0) + 1.0) / spd_sq;
            }
            let new_spd_l =
                self.mu_speed_al * (self.mu_speed_al - 1.0) + (sense_l * release + fastest) / hot_slow_l;
            self.mu_speed_al = (new_spd_l / self.mu_speed_al.max(1.0)).clamp(2.0, 200_000.0);

            // Right A
            if sense_r > threshold {
                let mu_vary = (threshold / sense_r.max(1.0e-6)).min(1.0);
                let mu_attack = (self.mu_speed_ar.abs().sqrt() * att_factor).max(1.5);
                self.mu_coeff_ar = (self.mu_coeff_ar * (mu_attack - 1.0) + mu_vary) / mu_attack;
            } else {
                let spd_sq = (self.mu_speed_ar * self.mu_speed_ar).max(2.0);
                self.mu_coeff_ar = (self.mu_coeff_ar * (spd_sq - 1.0) + 1.0) / spd_sq;
            }
            let new_spd_r =
                self.mu_speed_ar * (self.mu_speed_ar - 1.0) + (sense_r * release + fastest) / hot_slow_r;
            self.mu_speed_ar = (new_spd_r / self.mu_speed_ar.max(1.0)).clamp(2.0, 200_000.0);
        } else {
            // Left B
            if sense_l > threshold {
                let mu_vary = (threshold / sense_l.max(1.0e-6)).min(1.0);
                let mu_attack = (self.mu_speed_bl.abs().sqrt() * att_factor).max(1.5);
                self.mu_coeff_bl = (self.mu_coeff_bl * (mu_attack - 1.0) + mu_vary) / mu_attack;
            } else {
                let spd_sq = (self.mu_speed_bl * self.mu_speed_bl).max(2.0);
                self.mu_coeff_bl = (self.mu_coeff_bl * (spd_sq - 1.0) + 1.0) / spd_sq;
            }
            let new_spd_l =
                self.mu_speed_bl * (self.mu_speed_bl - 1.0) + (sense_l * release + fastest) / hot_slow_l;
            self.mu_speed_bl = (new_spd_l / self.mu_speed_bl.max(1.0)).clamp(2.0, 200_000.0);

            // Right B
            if sense_r > threshold {
                let mu_vary = (threshold / sense_r.max(1.0e-6)).min(1.0);
                let mu_attack = (self.mu_speed_br.abs().sqrt() * att_factor).max(1.5);
                self.mu_coeff_br = (self.mu_coeff_br * (mu_attack - 1.0) + mu_vary) / mu_attack;
            } else {
                let spd_sq = (self.mu_speed_br * self.mu_speed_br).max(2.0);
                self.mu_coeff_br = (self.mu_coeff_br * (spd_sq - 1.0) + 1.0) / spd_sq;
            }
            let new_spd_r =
                self.mu_speed_br * (self.mu_speed_br - 1.0) + (sense_r * release + fastest) / hot_slow_r;
            self.mu_speed_br = (new_spd_r / self.mu_speed_br.max(1.0)).clamp(2.0, 200_000.0);
        }

        let (raw_c_l, raw_c_r) = if self.flip {
            (
                self.mu_coeff_al.clamp(0.05, 1.0),
                self.mu_coeff_ar.clamp(0.05, 1.0),
            )
        } else {
            (
                self.mu_coeff_bl.clamp(0.05, 1.0),
                self.mu_coeff_br.clamp(0.05, 1.0),
            )
        };
        self.flip = !self.flip;

        let coeff_l = (raw_c_l * raw_c_l) * mewiness + raw_c_l * (1.0 - mewiness);
        let coeff_r = (raw_c_r * raw_c_r) * mewiness + raw_c_r * (1.0 - mewiness);

        self.gr_db_l = gain_to_db(coeff_l as f32).min(0.0);
        self.gr_db_r = gain_to_db(coeff_r as f32).min(0.0);

        let makeup = if cfg.auto_makeup {
            self.update_auto_makeup(coeff_l, coeff_r, cfg.mix_pct, true)
        } else {
            self.update_auto_makeup(coeff_l, coeff_r, cfg.mix_pct, false);
            db_to_gain(cfg.makeup_db) as f64
        };
        let wet = (cfg.mix_pct / 100.0).clamp(0.0, 1.0) as f64;

        let comp_l = l * coeff_l * makeup;
        let comp_r = r * coeff_r * makeup;

        let out_l = dry_l * (1.0 - wet) + comp_l * wet;
        let out_r = dry_r * (1.0 - wet) + comp_r * wet;

        let slew_l = (out_l - self.last_sample_l).abs();
        let slew_r = (out_r - self.last_sample_r).abs();
        self.slew_max = slew_l.max(slew_r);
        self.last_sample_l = out_l;
        self.last_sample_r = out_r;

        StereoFrame::new(out_l as f32, out_r as f32)
    }

    pub fn process(&mut self, frame: StereoFrame) -> StereoFrame {
        frame
    }

    /// Tracks a slow average of the linked gain reduction and returns the makeup that restores
    /// unity for the current parallel-mix setting. Also runs when disengaged so re-engaging it
    /// starts from a settled value instead of a jump.
    fn update_auto_makeup(
        &mut self,
        coeff_l: f64,
        coeff_r: f64,
        mix_pct: f32,
        engaged: bool,
    ) -> f64 {
        let reduction = ((coeff_l + coeff_r) * 0.5).clamp(0.02, 1.0);
        self.gr_avg += (reduction - self.gr_avg) * self.gr_avg_coeff;
        self.gr_avg = self.gr_avg.clamp(0.02, 1.0);

        let target = if engaged {
            let wet = (mix_pct / 100.0).clamp(0.0, 1.0) as f64;
            let denom = ((1.0 - wet) + wet * self.gr_avg).clamp(0.02, 1.0);
            // Hard ceiling of +24 dB so a pathological GR can never run away.
            (1.0 / denom).min(15.85)
        } else {
            1.0
        };
        self.auto_gain += (target - self.auto_gain) * self.auto_coeff;
        self.auto_makeup_db = if engaged {
            gain_to_db(self.auto_gain as f32)
        } else {
            0.0
        };
        self.auto_gain
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_cfg(threshold_db: f32) -> CompConfig {
        CompConfig {
            threshold_db,
            input_gain_db: 0.0,
            ratio: 4.0,
            attack_ms: 1.0,
            release_ms: 500.0,
            sc_hpf_hz: 0.0,
            mix_pct: 100.0,
            makeup_db: 0.0,
            auto_makeup: false,
        }
    }

    /// Runs a steady 1 kHz tone at `level_db` dBFS through a fresh compressor and returns the
    /// settled gain reduction (negative dB).
    fn gr_for(cfg: &CompConfig, level_db: f32) -> f32 {
        let mut comp = CompFair::default();
        let amp = 10f32.powf(level_db / 20.0);
        let step = 1000.0 * std::f32::consts::TAU / 48_000.0;
        let mut phase = 0.0f32;
        for _ in 0..96_000 {
            let x = amp * phase.sin();
            let _ = comp.process_with_config(StereoFrame::new(x, x), cfg);
            phase += step;
        }
        comp.gr_db_l
    }

    #[test]
    fn threshold_is_a_level_so_higher_settings_compress_less() {
        // THRESHOLD is a real level, not a normalised knob position. On a -6 dBFS tone the bottom
        // of the Fair span (-26 dB) limits hard, while the top (-6 dB) is exactly at the
        // signal peak and therefore does nothing.
        let hot = gr_for(&base_cfg(-26.0), -6.0);
        let cold = gr_for(&base_cfg(-6.0), -6.0);
        assert!(
            hot < -12.0,
            "expected heavy limiting at the low end, got {hot}"
        );
        assert!(
            cold > -0.5,
            "expected no gain reduction at the high end, got {cold}"
        );
        assert!(hot < cold, "{hot} should compress more than {cold}");
    }

    #[test]
    fn gain_reduction_follows_the_level_over_threshold() {
        // The Fair clamps the output at the threshold, so the reduction grows with how far
        // the signal sits above it: 2 dB over, 6 dB over, then 12 dB over.
        let two = gr_for(&base_cfg(-20.0), -18.0);
        let six = gr_for(&base_cfg(-20.0), -14.0);
        let twelve = gr_for(&base_cfg(-20.0), -8.0);
        assert!(two < -1.0 && two > -4.0, "2 dB over threshold gave {two}");
        assert!(six < two - 2.0, "{six} should exceed {two}");
        assert!(twelve < six - 2.0, "{twelve} should exceed {six}");
    }

    #[test]
    fn input_gain_drives_the_stage_without_moving_the_threshold() {
        let mut cfg = base_cfg(-20.0);
        let unity = gr_for(&cfg, -6.0);
        cfg.input_gain_db = 10.0;
        let driven = gr_for(&cfg, -6.0);
        assert!(
            driven < unity - 1.0,
            "input gain should add gain reduction: {unity} -> {driven}"
        );
        // The threshold itself never moved, so the extra reduction is the extra level, not a
        // sensitivity change: it should be in the same ballpark as the drive.
        assert!(
            (driven - (unity - 10.0)).abs() < 4.0,
            "10 dB of drive should add roughly 10 dB of reduction: {unity} -> {driven}"
        );
    }
}
