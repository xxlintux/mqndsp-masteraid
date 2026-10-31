//! Masteraid V-MU Inspired Mastering Glue Compressor (`source/dsp/comp_vmu.rs`)
//! Adapts Airwindows `VariMu` (`VariMuProc.cpp`):
//! - Squared-sample RMS-coupled envelope (`squaredSampleL/R`) with `previousL/R` peak-hold memory
//! - Smooth quadratic variable-mu gain law `coefficient = (muCoeff + muCoeff^2) * 0.5`
//! - Sidechain HPF, Parallel `MIX`, `MAKEUP`, and stereo GR telemetry

use super::comp_fair::CompConfig;
use super::shared::{db_to_gain, gain_to_db, AirwindowsBiquad, AirwindowsDither, StereoFrame};

pub struct CompVMu {
    sample_rate: f64,
    flip: bool,
    dither: AirwindowsDither,
    sc_hpf: AirwindowsBiquad,
    last_sc_freq: f32,
    previous_l: f64,
    previous_r: f64,
    mu_speed_al: f64,
    mu_speed_bl: f64,
    mu_coeff_al: f64,
    mu_coeff_bl: f64,
    mu_speed_ar: f64,
    mu_speed_br: f64,
    mu_coeff_ar: f64,
    mu_coeff_br: f64,
    pub gr_db_l: f32,
    pub gr_db_r: f32,
    gr_avg: f64,
    auto_gain: f64,
    gr_avg_coeff: f64,
    auto_coeff: f64,
    pub auto_makeup_db: f32,
}

impl Default for CompVMu {
    fn default() -> Self {
        let mut c = Self {
            sample_rate: 48000.0,
            flip: false,
            dither: AirwindowsDither::default(),
            sc_hpf: AirwindowsBiquad::default(),
            last_sc_freq: -1.0,
            previous_l: 0.0,
            previous_r: 0.0,
            mu_speed_al: 10000.0,
            mu_speed_bl: 10000.0,
            mu_coeff_al: 1.0,
            mu_coeff_bl: 1.0,
            mu_speed_ar: 10000.0,
            mu_speed_br: 10000.0,
            mu_coeff_ar: 1.0,
            mu_coeff_br: 1.0,
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

impl CompVMu {
    pub fn set_sample_rate(&mut self, sr: f32) {
        let new_sr = (sr as f64).max(22050.0);
        if (new_sr - self.sample_rate).abs() > 1.0 {
            self.sample_rate = new_sr;
            self.last_sc_freq = -1.0;
            self.gr_avg_coeff = 1.0 - (-1.0 / (new_sr * 2.0)).exp();
            self.auto_coeff = 1.0 - (-1.0 / (new_sr * 0.25)).exp();
        }
    }

    pub fn reset(&mut self) {
        self.flip = false;
        self.sc_hpf.reset();
        self.previous_l = 0.0;
        self.previous_r = 0.0;
        self.mu_speed_al = 10000.0;
        self.mu_speed_bl = 10000.0;
        self.mu_coeff_al = 1.0;
        self.mu_coeff_bl = 1.0;
        self.mu_speed_ar = 10000.0;
        self.mu_speed_br = 10000.0;
        self.mu_coeff_ar = 1.0;
        self.mu_coeff_br = 1.0;
        self.gr_db_l = 0.0;
        self.gr_db_r = 0.0;
        self.gr_avg = 1.0;
        self.auto_gain = 1.0;
        self.auto_makeup_db = 0.0;
    }

    pub fn process_with_config(&mut self, frame: StereoFrame, cfg: &CompConfig) -> StereoFrame {
        // INPUT GAIN drive, same staging as the Fair path.
        let drive = db_to_gain(cfg.input_gain_db) as f64;
        let mut l = frame.left as f64 * drive;
        let mut r = frame.right as f64 * drive;
        self.dither.guard_denormals(&mut l, &mut r);

        let dry_l = l;
        let dry_r = r;

        let overallscale = (2.0 * self.sample_rate / 44100.0).max(1.0);

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

        // V-MU compares energy (sample squared) against the threshold, so the dB setting is
        // converted to power. The floor is low enough that a -30 dB setting still bites on quieter
        // programme material, which is what a vari-mu stage is expected to do.
        let threshold = ((db_to_gain(cfg.threshold_db) as f64).powi(2)).clamp(0.0005, 0.99);

        let att_scale = (cfg.attack_ms.clamp(0.1, 30.0) as f64 / 5.0)
            .sqrt()
            .clamp(0.35, 3.0);
        let rel_norm = if cfg.release_ms <= 0.0 {
            0.55
        } else {
            (cfg.release_ms.clamp(50.0, 2000.0) as f64 / 2000.0).powf(0.4)
        };
        let release = ((1.15 - (1.0 - rel_norm) * 0.72).powi(5) * 32768.0) / overallscale;
        let fastest = release.sqrt() * att_scale;

        // VariMu lines 83-92: two-sample peak-squared energy detector
        let sq_l = if sc_l.abs() > self.previous_l.abs() {
            self.previous_l * self.previous_l
        } else {
            sc_l * sc_l
        };
        self.previous_l = sc_l;

        let sq_r = if sc_r.abs() > self.previous_r.abs() {
            self.previous_r * self.previous_r
        } else {
            sc_r * sc_r
        };
        self.previous_r = sc_r;

        // Stereo glue linking (85% linked for mastering stereo image stability)
        let sq_max = sq_l.max(sq_r);
        let sense_sq_l = sq_max * 0.85 + sq_l * 0.15;
        let sense_sq_r = sq_max * 0.85 + sq_r * 0.15;

        // ButterComp2 hot-signal release slowdown: prevents low-frequency ripple/distortion on heavy bass
        let hot_slow_l = 1.0 + sense_sq_l.sqrt() * 0.35;
        let hot_slow_r = 1.0 + sense_sq_r.sqrt() * 0.35;

        if self.flip {
            if sense_sq_l > threshold {
                let mu_vary = (threshold / sense_sq_l.max(1.0e-6)).min(1.0);
                let mu_attack = (self.mu_speed_al.abs().sqrt() * att_scale).max(2.0);
                self.mu_coeff_al = (self.mu_coeff_al * (mu_attack - 1.0) + mu_vary) / mu_attack;
            } else {
                let spd_sq = (self.mu_speed_al * self.mu_speed_al).max(2.0);
                self.mu_coeff_al = (self.mu_coeff_al * (spd_sq - 1.0) + 1.0) / spd_sq;
            }
            let new_spd_l =
                self.mu_speed_al * (self.mu_speed_al - 1.0) + (sense_sq_l * release + fastest) / hot_slow_l;
            self.mu_speed_al = (new_spd_l / self.mu_speed_al.max(1.0)).clamp(2.0, 250_000.0);

            if sense_sq_r > threshold {
                let mu_vary = (threshold / sense_sq_r.max(1.0e-6)).min(1.0);
                let mu_attack = (self.mu_speed_ar.abs().sqrt() * att_scale).max(2.0);
                self.mu_coeff_ar = (self.mu_coeff_ar * (mu_attack - 1.0) + mu_vary) / mu_attack;
            } else {
                let spd_sq = (self.mu_speed_ar * self.mu_speed_ar).max(2.0);
                self.mu_coeff_ar = (self.mu_coeff_ar * (spd_sq - 1.0) + 1.0) / spd_sq;
            }
            let new_spd_r =
                self.mu_speed_ar * (self.mu_speed_ar - 1.0) + (sense_sq_r * release + fastest) / hot_slow_r;
            self.mu_speed_ar = (new_spd_r / self.mu_speed_ar.max(1.0)).clamp(2.0, 250_000.0);
        } else {
            if sense_sq_l > threshold {
                let mu_vary = (threshold / sense_sq_l.max(1.0e-6)).min(1.0);
                let mu_attack = (self.mu_speed_bl.abs().sqrt() * att_scale).max(2.0);
                self.mu_coeff_bl = (self.mu_coeff_bl * (mu_attack - 1.0) + mu_vary) / mu_attack;
            } else {
                let spd_sq = (self.mu_speed_bl * self.mu_speed_bl).max(2.0);
                self.mu_coeff_bl = (self.mu_coeff_bl * (spd_sq - 1.0) + 1.0) / spd_sq;
            }
            let new_spd_l =
                self.mu_speed_bl * (self.mu_speed_bl - 1.0) + (sense_sq_l * release + fastest) / hot_slow_l;
            self.mu_speed_bl = (new_spd_l / self.mu_speed_bl.max(1.0)).clamp(2.0, 250_000.0);

            if sense_sq_r > threshold {
                let mu_vary = (threshold / sense_sq_r.max(1.0e-6)).min(1.0);
                let mu_attack = (self.mu_speed_br.abs().sqrt() * att_scale).max(2.0);
                self.mu_coeff_br = (self.mu_coeff_br * (mu_attack - 1.0) + mu_vary) / mu_attack;
            } else {
                let spd_sq = (self.mu_speed_br * self.mu_speed_br).max(2.0);
                self.mu_coeff_br = (self.mu_coeff_br * (spd_sq - 1.0) + 1.0) / spd_sq;
            }
            let new_spd_r =
                self.mu_speed_br * (self.mu_speed_br - 1.0) + (sense_sq_r * release + fastest) / hot_slow_r;
            self.mu_speed_br = (new_spd_r / self.mu_speed_br.max(1.0)).clamp(2.0, 250_000.0);
        }

        // VariMu lines 207-223: quadratic variable-mu gain curve scaled by ratio
        let (c_l, c_r) = if self.flip {
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

        let ratio_exp = ((cfg.ratio.clamp(1.2, 20.0) - 1.0) / 4.0).clamp(0.3, 2.0) as f64;
        let coeff_l = ((c_l + c_l * c_l) * 0.5).powf(ratio_exp);
        let coeff_r = ((c_r + c_r * c_r) * 0.5).powf(ratio_exp);

        self.gr_db_l = gain_to_db(coeff_l as f32).min(0.0);
        self.gr_db_r = gain_to_db(coeff_r as f32).min(0.0);

        let makeup = if cfg.auto_makeup {
            self.update_auto_makeup(coeff_l, coeff_r, cfg.mix_pct, true)
        } else {
            self.update_auto_makeup(coeff_l, coeff_r, cfg.mix_pct, false);
            db_to_gain(cfg.makeup_db) as f64
        };
        let wet = (cfg.mix_pct / 100.0).clamp(0.0, 1.0) as f64;

        let out_l = dry_l * (1.0 - wet) + (l * coeff_l * makeup) * wet;
        let out_r = dry_r * (1.0 - wet) + (r * coeff_r * makeup) * wet;

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
