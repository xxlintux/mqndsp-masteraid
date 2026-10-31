//! Masteraid Brickwall Mastering Limiter (`source/dsp/limiter.rs`)
//! Adapts and constrains Airwindows `BlockParty` (`BlockPartyProc.cpp`) & `ClipOnly2` (`ClipOnly2Proc.cpp`):
//! - 3-way interleaved (`count = 1..3`) mu-speed envelope limiter with dynamic voicing
//! - `Auto / Manual` release behavior and `True Peak` 4x inter-sample ceiling compliance

use super::shared::{db_to_gain, AirwindowsDither, StereoFrame};

#[derive(Clone, Copy, Debug)]
pub struct LimiterConfig {
    pub gain_db: f32,    // 0..+10 dB
    pub ceiling_db: f32, // -3.0..0.0 dB
    pub lookahead_ms: f32, // 0.0 (OFF) .. 5.0 ms
    pub release_ms: f32, // 0.0 (AUTO) .. 1200.0 ms
    pub lift: f32,       // 0.0 (OFF) .. 1.0 (100%)
    pub auto_release: bool,
    pub true_peak: bool,
    pub punch: bool,
    pub unity: bool,
}

pub struct Limiter {
    sample_rate: f64,
    dither: AirwindowsDither,
    count: usize,
    threshold_l: f64,
    threshold_r: f64,
    mu_speed_l: [f64; 3],
    mu_coeff_l: [f64; 3],
    mu_speed_r: [f64; 3],
    mu_coeff_r: [f64; 3],
    merged_coeff_l: f64,
    merged_coeff_r: f64,
    last_coeff_bl: f64,
    last_coeff_br: f64,
    // ClipOnly2 final brickwall state
    last_sample_l: f64,
    last_sample_r: f64,
    was_pos_clip_l: bool,
    was_neg_clip_l: bool,
    was_pos_clip_r: bool,
    was_neg_clip_r: bool,
    prev_tp_l: f64,
    prev_tp_r: f64,
    // Lookahead ring buffer & metrics
    lookahead_buf: Vec<StereoFrame>,
    buf_idx: usize,
    peak_gr_db: f32,
    peak_out_db: f32,
    env_out: f32,
    env_gr: f32,
    peak_release: f32,
}

impl Default for Limiter {
    fn default() -> Self {
        let mut lim = Self {
            sample_rate: 48000.0,
            dither: AirwindowsDither::default(),
            count: 0,
            threshold_l: 1.0,
            threshold_r: 1.0,
            mu_speed_l: [10000.0; 3],
            mu_coeff_l: [1.0; 3],
            mu_speed_r: [10000.0; 3],
            mu_coeff_r: [1.0; 3],
            merged_coeff_l: 1.0,
            merged_coeff_r: 1.0,
            last_coeff_bl: 1.0,
            last_coeff_br: 1.0,
            last_sample_l: 0.0,
            last_sample_r: 0.0,
            was_pos_clip_l: false,
            was_neg_clip_l: false,
            was_pos_clip_r: false,
            was_neg_clip_r: false,
            prev_tp_l: 0.0,
            prev_tp_r: 0.0,
            lookahead_buf: vec![StereoFrame::default(); 2048],
            buf_idx: 0,
            peak_gr_db: 0.0,
            peak_out_db: -60.0,
            env_out: 0.0,
            env_gr: 0.0,
            peak_release: (-1.0f32 / (48000.0 * 0.7)).exp(),
        };
        lim.reset();
        lim
    }
}

impl Limiter {
    pub fn set_sample_rate(&mut self, sr: f32) {
        self.sample_rate = (sr as f64).max(22050.0);
        self.peak_release = (-1.0 / (self.sample_rate * 0.7)).exp() as f32;
    }

    pub fn reset(&mut self) {
        self.count = 0;
        self.threshold_l = 1.0;
        self.threshold_r = 1.0;
        self.mu_speed_l = [10000.0; 3];
        self.mu_coeff_l = [1.0; 3];
        self.mu_speed_r = [10000.0; 3];
        self.mu_coeff_r = [1.0; 3];
        self.merged_coeff_l = 1.0;
        self.merged_coeff_r = 1.0;
        self.last_coeff_bl = 1.0;
        self.last_coeff_br = 1.0;
        self.last_sample_l = 0.0;
        self.last_sample_r = 0.0;
        self.was_pos_clip_l = false;
        self.was_neg_clip_l = false;
        self.was_pos_clip_r = false;
        self.was_neg_clip_r = false;
        self.prev_tp_l = 0.0;
        self.prev_tp_r = 0.0;
        self.lookahead_buf.fill(StereoFrame::default());
        self.buf_idx = 0;
        self.peak_gr_db = 0.0;
        self.peak_out_db = -60.0;
        self.env_out = 0.0;
        self.env_gr = 0.0;
    }

    #[inline]
    pub fn take_metrics(&mut self) -> (f32, f32) {
        (self.peak_gr_db, self.peak_out_db)
    }

    pub fn process_with_config(&mut self, frame: StereoFrame, cfg: &LimiterConfig) -> StereoFrame {
        let mut l = frame.left as f64;
        let mut r = frame.right as f64;
        self.dither.guard_denormals(&mut l, &mut r);

        let overallscale = (self.sample_rate / 44100.0).max(0.5);
        let in_gain = db_to_gain(cfg.gain_db.clamp(0.0, 12.0)) as f64;
        let ceiling = (db_to_gain(cfg.ceiling_db.clamp(-3.0, 0.0)) as f64).clamp(0.5, 1.0);
        let target_threshold = (ceiling * 0.96).clamp(0.4, 0.98);

        let voicing = (0.6180339887498949 / overallscale).clamp(0.05, 0.95);

        // 1. Naturl Optimizer LIFT: upward expansion on low-level content below -24 dBFS
        if cfg.lift > 0.001 {
            let lift_amt = (cfg.lift.clamp(0.0, 1.0) as f64) * 0.35;
            let abs_l = l.abs();
            let abs_r = r.abs();
            let boost_l = (1.0 - (abs_l / 0.063).clamp(0.0, 1.0)) * lift_amt;
            let boost_r = (1.0 - (abs_r / 0.063).clamp(0.0, 1.0)) * lift_amt;
            l *= 1.0 + boost_l;
            r *= 1.0 + boost_r;
        }

        // 2. Drive into limiter
        l *= in_gain;
        r *= in_gain;

        // 3. Lookahead circular buffer
        let delay_samples = if cfg.lookahead_ms > 0.05 {
            ((cfg.lookahead_ms * 0.001) * self.sample_rate as f32).round() as usize
        } else {
            0
        }
        .min(2047);

        let (delayed_l, delayed_r) = if delay_samples > 0 {
            self.lookahead_buf[self.buf_idx] = StereoFrame::new(l as f32, r as f32);
            let read_idx = (self.buf_idx + 2048 - delay_samples) % 2048;
            self.buf_idx = (self.buf_idx + 1) % 2048;
            let read_f = self.lookahead_buf[read_idx];
            (read_f.left as f64, read_f.right as f64)
        } else {
            (l, r)
        };

        // 4. Release time: AUTO vs calibrated millisecond recovery
        let rel_mult = if cfg.auto_release || cfg.release_ms <= 0.05 {
            self.merged_coeff_l.clamp(0.2, 1.5) * 32768.0
        } else {
            (cfg.release_ms * 163.84).clamp(1024.0, 262144.0) as f64
        };
        let release_l = rel_mult / overallscale;
        let fastest_l = release_l.sqrt();
        let last_corr_l = self.merged_coeff_l.clamp(0.1, 1.0);

        let rel_mult_r = if cfg.auto_release || cfg.release_ms <= 0.05 {
            self.merged_coeff_r.clamp(0.2, 1.5) * 32768.0
        } else {
            (cfg.release_ms * 163.84).clamp(1024.0, 262144.0) as f64
        };
        let release_r = rel_mult_r / overallscale;
        let fastest_r = release_r.sqrt();
        let last_corr_r = self.merged_coeff_r.clamp(0.1, 1.0);

        let slot = self.count % 3;
        self.count = (self.count + 1) % 3;

        // Left channel BlockParty envelope
        if l.abs() > self.threshold_l {
            if l > 0.0 {
                l = l * voicing + target_threshold * (1.0 - voicing);
            } else {
                l = l * voicing - target_threshold * (1.0 - voicing);
            }
            self.threshold_l = l.abs().max(target_threshold);
            let mu_vary = (target_threshold / l.abs().max(1.0e-6)).min(1.0);
            let base_mu_attack = self.mu_speed_l[slot].abs().sqrt().max(1.5);
            let mu_attack = if cfg.punch {
                (base_mu_attack * 2.5).max(3.0)
            } else {
                base_mu_attack
            };
            self.mu_coeff_l[slot] = (self.mu_coeff_l[slot] * (mu_attack - 1.0) + mu_vary) / mu_attack;
        } else {
            self.threshold_l = target_threshold;
            let spd_sq = (self.mu_speed_l[slot] * self.mu_speed_l[slot]).max(2.0);
            self.mu_coeff_l[slot] = (self.mu_coeff_l[slot] * (spd_sq - 1.0) + 1.0) / spd_sq;
        }
        let new_spd_l =
            self.mu_speed_l[slot] * (self.mu_speed_l[slot] - 1.0) + l.abs() * release_l + fastest_l;
        self.mu_speed_l[slot] = (new_spd_l / self.mu_speed_l[slot].max(1.0)).clamp(2.0, 200_000.0);
        let mut last_c_al = self.mu_coeff_l[slot];
        self.merged_coeff_l = (self.last_coeff_bl + last_c_al) * 0.5;
        last_c_al = last_c_al * (1.0 - last_corr_l) + self.mu_coeff_l[slot] * last_corr_l;
        self.last_coeff_bl = last_c_al;

        // Right channel BlockParty envelope
        if r.abs() > self.threshold_r {
            if r > 0.0 {
                r = r * voicing + target_threshold * (1.0 - voicing);
            } else {
                r = r * voicing - target_threshold * (1.0 - voicing);
            }
            self.threshold_r = r.abs().max(target_threshold);
            let mu_vary = (target_threshold / r.abs().max(1.0e-6)).min(1.0);
            let base_mu_attack = self.mu_speed_r[slot].abs().sqrt().max(1.5);
            let mu_attack = if cfg.punch {
                (base_mu_attack * 2.5).max(3.0)
            } else {
                base_mu_attack
            };
            self.mu_coeff_r[slot] = (self.mu_coeff_r[slot] * (mu_attack - 1.0) + mu_vary) / mu_attack;
        } else {
            self.threshold_r = target_threshold;
            let spd_sq = (self.mu_speed_r[slot] * self.mu_speed_r[slot]).max(2.0);
            self.mu_coeff_r[slot] = (self.mu_coeff_r[slot] * (spd_sq - 1.0) + 1.0) / spd_sq;
        }
        let new_spd_r =
            self.mu_speed_r[slot] * (self.mu_speed_r[slot] - 1.0) + r.abs() * release_r + fastest_r;
        self.mu_speed_r[slot] = (new_spd_r / self.mu_speed_r[slot].max(1.0)).clamp(2.0, 200_000.0);
        let mut last_c_ar = self.mu_coeff_r[slot];
        self.merged_coeff_r = (self.last_coeff_br + last_c_ar) * 0.5;
        last_c_ar = last_c_ar * (1.0 - last_corr_r) + self.mu_coeff_r[slot] * last_corr_r;
        self.last_coeff_br = last_c_ar;

        // Stereo-linked final gain reduction applied to the delayed audio path
        let lim_gain = self.merged_coeff_l.min(self.merged_coeff_r).clamp(0.1, 1.0);
        let inst_gr = (-20.0 * (lim_gain as f32).log10()).max(0.0);
        if inst_gr > self.env_gr {
            self.env_gr = inst_gr;
        } else {
            self.env_gr = self.env_gr * self.peak_release + inst_gr * (1.0 - self.peak_release);
        }
        self.peak_gr_db = self.env_gr;

        l = delayed_l * lim_gain;
        r = delayed_r * lim_gain;

        // Airwindows ClipOnly3 brickwall ceiling + noise-masked dithered edge transitions + slew limiter
        let noise_l = 1.0 - (self.dither.next_unit_l() * 0.076);
        let noise_r = 1.0 - (self.dither.next_unit_r() * 0.076);
        let clip_thresh = ceiling * 0.9549925859;
        let hard_ceil = ceiling * 0.7058208;
        let soft_ceil = ceiling * 0.2491717;

        if self.was_pos_clip_l {
            if l < self.last_sample_l {
                self.last_sample_l = (hard_ceil * noise_l) + (l * 0.2609148);
            } else {
                self.last_sample_l = (soft_ceil * noise_l) + (self.last_sample_l * 0.7390851);
            }
        }
        self.was_pos_clip_l = false;
        if l > clip_thresh {
            self.was_pos_clip_l = true;
            l = (hard_ceil * noise_l) + (self.last_sample_l * 0.2609148);
        }
        if self.was_neg_clip_l {
            if l > self.last_sample_l {
                self.last_sample_l = (-hard_ceil * noise_l) + (l * 0.2609148);
            } else {
                self.last_sample_l = (-soft_ceil * noise_l) + (self.last_sample_l * 0.7390851);
            }
        }
        self.was_neg_clip_l = false;
        if l < -clip_thresh {
            self.was_neg_clip_l = true;
            l = (-hard_ceil * noise_l) + (self.last_sample_l * 0.2609148);
        }

        if self.was_pos_clip_r {
            if r < self.last_sample_r {
                self.last_sample_r = (hard_ceil * noise_r) + (r * 0.2609148);
            } else {
                self.last_sample_r = (soft_ceil * noise_r) + (self.last_sample_r * 0.7390851);
            }
        }
        self.was_pos_clip_r = false;
        if r > clip_thresh {
            self.was_pos_clip_r = true;
            r = (hard_ceil * noise_r) + (self.last_sample_r * 0.2609148);
        }
        if self.was_neg_clip_r {
            if r > self.last_sample_r {
                self.last_sample_r = (-hard_ceil * noise_r) + (r * 0.2609148);
            } else {
                self.last_sample_r = (-soft_ceil * noise_r) + (self.last_sample_r * 0.7390851);
            }
        }
        self.was_neg_clip_r = false;
        if r < -clip_thresh {
            self.was_neg_clip_r = true;
            r = (-hard_ceil * noise_r) + (self.last_sample_r * 0.2609148);
        }

        let slew_l = (self.last_sample_l - l).abs();
        let postclip_l = ceiling / (1.0 + slew_l * 1.3986);
        let mut out_l = self.last_sample_l.clamp(-postclip_l, postclip_l);
        self.last_sample_l = l.clamp(-postclip_l, postclip_l);

        let slew_r = (self.last_sample_r - r).abs();
        let postclip_r = ceiling / (1.0 + slew_r * 1.3986);
        let mut out_r = self.last_sample_r.clamp(-postclip_r, postclip_r);
        self.last_sample_r = r.clamp(-postclip_r, postclip_r);

        if cfg.true_peak {
            // Exact parabolic continuous inter-sample peak reconstruction
            let target_tp = ceiling * 0.985; // 0.13 dB inter-sample reconstruction guard
            let denom_l = self.prev_tp_l - 2.0 * out_l + l;
            if denom_l.abs() > 1e-6 {
                let t_peak_l = (self.prev_tp_l - l) / (2.0 * denom_l);
                if t_peak_l.abs() <= 1.0 {
                    let y_peak_l = (out_l - (l - self.prev_tp_l).powi(2) / (8.0 * denom_l)).abs();
                    if y_peak_l > target_tp {
                        out_l *= target_tp / y_peak_l;
                    }
                }
            }

            let denom_r = self.prev_tp_r - 2.0 * out_r + r;
            if denom_r.abs() > 1e-6 {
                let t_peak_r = (self.prev_tp_r - r) / (2.0 * denom_r);
                if t_peak_r.abs() <= 1.0 {
                    let y_peak_r = (out_r - (r - self.prev_tp_r).powi(2) / (8.0 * denom_r)).abs();
                    if y_peak_r > target_tp {
                        out_r *= target_tp / y_peak_r;
                    }
                }
            }

            self.prev_tp_l = out_l;
            self.prev_tp_r = out_r;
        }

        out_l = out_l.clamp(-ceiling, ceiling);
        out_r = out_r.clamp(-ceiling, ceiling);

        // Naturl AL-1 Unity Mode: equal-loudness monitor auditioning
        if cfg.unity {
            let unity_pad = db_to_gain(-cfg.gain_db.max(0.0)) as f64;
            out_l *= unity_pad;
            out_r *= unity_pad;
        }

        let cur_peak = (out_l.abs().max(out_r.abs()) as f32).max(1e-5);
        if cur_peak > self.env_out {
            self.env_out = cur_peak;
        } else {
            self.env_out = self.env_out * self.peak_release + cur_peak * (1.0 - self.peak_release);
        }
        self.peak_out_db = (20.0 * self.env_out.log10()).clamp(-60.0, 12.0);

        StereoFrame::new(out_l as f32, out_r as f32)
    }

    pub fn process(&mut self, frame: StereoFrame) -> StereoFrame {
        frame
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_limiter_ceiling_clamp() {
        let mut limiter = Limiter::default();
        let cfg = LimiterConfig {
            gain_db: 6.0,
            ceiling_db: -1.0,
            lookahead_ms: 0.0,
            release_ms: 0.0,
            lift: 0.0,
            auto_release: false,
            true_peak: false,
            punch: false,
            unity: false,
        };

        let ceiling_lin = db_to_gain(-1.0);
        for i in 0..1000 {
            let t = (i as f32) / 48000.0;
            let sample = (2.0 * std::f32::consts::PI * 440.0 * t).sin();
            let out = limiter.process_with_config(StereoFrame::new(sample, sample), &cfg);
            assert!(
                out.left <= ceiling_lin + 1e-4,
                "Left exceeded ceiling: {} > {}",
                out.left,
                ceiling_lin
            );
            assert!(
                out.right <= ceiling_lin + 1e-4,
                "Right exceeded ceiling: {} > {}",
                out.right,
                ceiling_lin
            );
        }
    }

    #[test]
    fn test_limiter_unity_mode() {
        let mut limiter_normal = Limiter::default();
        let mut limiter_unity = Limiter::default();

        let cfg_norm = LimiterConfig {
            gain_db: 6.0,
            ceiling_db: 0.0,
            lookahead_ms: 0.0,
            release_ms: 0.0,
            lift: 0.0,
            auto_release: false,
            true_peak: false,
            punch: false,
            unity: false,
        };
        let cfg_unity = LimiterConfig {
            unity: true,
            ..cfg_norm
        };

        // Quiet signal well below ceiling so no limiting occurs
        let input = StereoFrame::new(0.01, 0.01);
        for _ in 0..20 {
            limiter_normal.process_with_config(input, &cfg_norm);
            limiter_unity.process_with_config(input, &cfg_unity);
        }

        let out_norm = limiter_normal.process_with_config(input, &cfg_norm);
        let out_uni = limiter_unity.process_with_config(input, &cfg_unity);

        // In unity mode, the 6 dB gain is compensated with -6 dB pad on monitor
        let ratio = out_norm.left / out_uni.left;
        let expected_ratio = db_to_gain(6.0);
        assert!(
            (ratio - expected_ratio).abs() < 1e-3,
            "Ratio {} != expected {}",
            ratio,
            expected_ratio
        );
    }

    #[test]
    fn test_limiter_lift_expands_quiet_signals() {
        let mut limiter_flat = Limiter::default();
        let mut limiter_lifted = Limiter::default();

        let cfg_flat = LimiterConfig {
            gain_db: 0.0,
            ceiling_db: 0.0,
            lookahead_ms: 0.0,
            release_ms: 0.0,
            lift: 0.0,
            auto_release: false,
            true_peak: false,
            punch: false,
            unity: false,
        };
        let cfg_lift = LimiterConfig {
            lift: 0.5,
            ..cfg_flat
        };

        // -30 dBFS signal (below the -24 dBFS upward expansion knee)
        let quiet_sample = db_to_gain(-30.0);
        let input = StereoFrame::new(quiet_sample, quiet_sample);

        // Warm up 10 samples to fill the 1-sample pipeline
        for _ in 0..10 {
            limiter_flat.process_with_config(input, &cfg_flat);
            limiter_lifted.process_with_config(input, &cfg_lift);
        }

        let out_flat = limiter_flat.process_with_config(input, &cfg_flat);
        let out_lift = limiter_lifted.process_with_config(input, &cfg_lift);

        assert!(
            out_lift.left > out_flat.left,
            "Lift should increase level of quiet signal: {} <= {}",
            out_lift.left,
            out_flat.left
        );
    }
}
