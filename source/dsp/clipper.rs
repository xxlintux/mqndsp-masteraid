//! Masteraid Mastering Clipper (`source/dsp/clipper.rs`)
//! Adapts and constrains Airwindows `ADClip7` (`ADClip7Proc.cpp`) & `ClipOnly2` (`ClipOnly2Proc.cpp`):
//! - Golden-ratio (`0.61803398875`) soft-knee slew interpolation (`SOFT`) vs `0.739085` cosine clip (`HARD`)
//! - Configurable `2x` / `4x` zero-ringing Hermite oversampling with calibrated `Drive` and `Ceiling`

use super::shared::{db_to_gain, AirwindowsDither, StereoFrame};

#[derive(Clone, Copy, Debug)]
pub struct ClipperConfig {
    pub drive_db: f32,
    pub ceiling_db: f32,
    pub hard_shape: bool, // false = SOFT (ADClip7 golden-ratio knee), true = HARD (ClipOnly2 cosine knee)
    pub oversample_4x: bool, // false = 2X, true = 4X
    /// Remove the drive gain from the output, so Drive only sets how hard the signal hits the
    /// ceiling (clip density) and never lifts the level feeding the limiter.
    pub compensate: bool,
}

pub struct Clipper {
    dither: AirwindowsDither,
    last_sample_l: f64,
    last_sample_r: f64,
    was_pos_clip_l: bool,
    was_neg_clip_l: bool,
    was_pos_clip_r: bool,
    was_neg_clip_r: bool,
    prev_in_l: f64,
    prev_in_r: f64,
    peak_clip_db: f32,
    env_clip_l: f32,
    env_clip_r: f32,
    clip_release: f32,
}

impl Default for Clipper {
    fn default() -> Self {
        Self {
            dither: AirwindowsDither::default(),
            last_sample_l: 0.0,
            last_sample_r: 0.0,
            was_pos_clip_l: false,
            was_neg_clip_l: false,
            was_pos_clip_r: false,
            was_neg_clip_r: false,
            prev_in_l: 0.0,
            prev_in_r: 0.0,
            peak_clip_db: 0.0,
            env_clip_l: 0.0,
            env_clip_r: 0.0,
            clip_release: (-1.0f32 / (48000.0 * 0.3)).exp(),
        }
    }
}

impl Clipper {
    pub fn set_sample_rate(&mut self, sr: f32) {
        let sr = (sr as f64).max(22050.0);
        self.clip_release = (-1.0 / (sr * 0.3)).exp() as f32;
    }

    pub fn reset(&mut self) {
        self.last_sample_l = 0.0;
        self.last_sample_r = 0.0;
        self.was_pos_clip_l = false;
        self.was_neg_clip_l = false;
        self.was_pos_clip_r = false;
        self.was_neg_clip_r = false;
        self.prev_in_l = 0.0;
        self.prev_in_r = 0.0;
        self.peak_clip_db = 0.0;
        self.env_clip_l = 0.0;
        self.env_clip_r = 0.0;
    }

    #[inline]
    pub fn take_metrics(&mut self) -> (f32, f32, f32) {
        let held = self.peak_clip_db;
        self.peak_clip_db = 0.0;
        (self.env_clip_l, self.env_clip_r, held)
    }

    #[inline]
    pub fn take_peak_clip_db(&mut self) -> f32 {
        let v = self.peak_clip_db;
        self.peak_clip_db = 0.0;
        v
    }

    #[inline]
    fn clip_substep(
        &mut self,
        mut l: f64,
        mut r: f64,
        refclip: f64,
        hardness: f64,
        softness: f64,
    ) -> (f64, f64) {
        l = l.clamp(-4.0, 4.0);
        r = r.clamp(-4.0, 4.0);

        let clip_thresh = refclip * 0.9549925859;
        let hard_ref = refclip * hardness;
        let soft_ref = refclip * softness;

        // Airwindows ClipOnly3 state machine: noise-masked dithered clip transitions & slew control
        let noise_l = 1.0 - (self.dither.next_unit_l() * 0.076);
        let noise_r = 1.0 - (self.dither.next_unit_r() * 0.076);

        if self.was_pos_clip_l {
            if l < self.last_sample_l {
                self.last_sample_l = (hard_ref * noise_l) + (l * softness * (1.0 - noise_l));
            } else {
                self.last_sample_l = (soft_ref * noise_l) + (self.last_sample_l * hardness * (1.0 - noise_l));
            }
        }
        self.was_pos_clip_l = false;
        if l > clip_thresh {
            self.was_pos_clip_l = true;
            l = (hard_ref * noise_l) + (self.last_sample_l * softness * (1.0 - noise_l));
        }

        if self.was_neg_clip_l {
            if l > self.last_sample_l {
                self.last_sample_l = (-hard_ref * noise_l) + (l * softness * (1.0 - noise_l));
            } else {
                self.last_sample_l = (-soft_ref * noise_l) + (self.last_sample_l * hardness * (1.0 - noise_l));
            }
        }
        self.was_neg_clip_l = false;
        if l < -clip_thresh {
            self.was_neg_clip_l = true;
            l = (-hard_ref * noise_l) + (self.last_sample_l * softness * (1.0 - noise_l));
        }

        if self.was_pos_clip_r {
            if r < self.last_sample_r {
                self.last_sample_r = (hard_ref * noise_r) + (r * softness * (1.0 - noise_r));
            } else {
                self.last_sample_r = (soft_ref * noise_r) + (self.last_sample_r * hardness * (1.0 - noise_r));
            }
        }
        self.was_pos_clip_r = false;
        if r > clip_thresh {
            self.was_pos_clip_r = true;
            r = (hard_ref * noise_r) + (self.last_sample_r * softness * (1.0 - noise_r));
        }

        if self.was_neg_clip_r {
            if r > self.last_sample_r {
                self.last_sample_r = (-hard_ref * noise_r) + (r * softness * (1.0 - noise_r));
            } else {
                self.last_sample_r = (-soft_ref * noise_r) + (self.last_sample_r * hardness * (1.0 - noise_r));
            }
        }
        self.was_neg_clip_r = false;
        if r < -clip_thresh {
            self.was_neg_clip_r = true;
            r = (-hard_ref * noise_r) + (self.last_sample_r * softness * (1.0 - noise_r));
        }

        let slew_l = (self.last_sample_l - l).abs();
        let postclip_l = refclip / (1.0 + slew_l * 1.3986 * (1.0 - softness));
        let out_l = self.last_sample_l.clamp(-postclip_l, postclip_l);
        self.last_sample_l = l.clamp(-postclip_l, postclip_l);

        let slew_r = (self.last_sample_r - r).abs();
        let postclip_r = refclip / (1.0 + slew_r * 1.3986 * (1.0 - softness));
        let out_r = self.last_sample_r.clamp(-postclip_r, postclip_r);
        self.last_sample_r = r.clamp(-postclip_r, postclip_r);

        (out_l, out_r)
    }

    pub fn process_with_config(&mut self, frame: StereoFrame, cfg: &ClipperConfig) -> StereoFrame {
        let mut l = frame.left as f64;
        let mut r = frame.right as f64;
        self.dither.guard_denormals(&mut l, &mut r);

        let drive_gain = db_to_gain(cfg.drive_db.clamp(-10.0, 12.0)) as f64;
        // Unity drive still loses this much to the oversampled interpolation/average (measured
        // with -18 dBFS pink noise), and AUTO should be level-matched to clipper bypass, not just
        // drive-independent. The residual left above this is the true density-dependent part.
        const INSERTION_TRIM_DB: f64 = 0.36;
        let comp_gain = if cfg.compensate {
            (db_to_gain(INSERTION_TRIM_DB as f32) as f64) / drive_gain
        } else {
            1.0
        };
        let refclip = (db_to_gain(cfg.ceiling_db.clamp(-6.0, 0.0)) as f64).clamp(0.25, 1.0);

        // SOFT uses ADClip7 golden ratio (0.618 softness); HARD uses ClipOnly2 x==cos(x) (0.739 hardness)
        let (hardness, softness) = if cfg.hard_shape {
            (0.7390851332151606, 0.2609148667848394)
        } else {
            (0.38196601125010515, 0.6180339887498949)
        };

        let target_l = l * drive_gain;
        let target_r = r * drive_gain;

        let peak_in_l = target_l.abs();
        let peak_in_r = target_r.abs();
        let inst_db_l = if peak_in_l > refclip {
            (20.0 * (peak_in_l / refclip).log10()).max(0.0) as f32
        } else {
            0.0
        };
        let inst_db_r = if peak_in_r > refclip {
            (20.0 * (peak_in_r / refclip).log10()).max(0.0) as f32
        } else {
            0.0
        };

        if inst_db_l > self.env_clip_l {
            self.env_clip_l = inst_db_l;
        } else {
            self.env_clip_l = self.env_clip_l * self.clip_release + inst_db_l * (1.0 - self.clip_release);
        }

        if inst_db_r > self.env_clip_r {
            self.env_clip_r = inst_db_r;
        } else {
            self.env_clip_r = self.env_clip_r * self.clip_release + inst_db_r * (1.0 - self.clip_release);
        }

        let max_inst = inst_db_l.max(inst_db_r);
        if max_inst > self.peak_clip_db {
            self.peak_clip_db = max_inst;
        }

        // Soft ADClip7 pre-saturation mojo when in SOFT mode
        let shape_sample = |x: f64, is_hard: bool, ceil: f64| -> f64 {
            if is_hard || x.abs() < ceil * 0.65 {
                x
            } else {
                let norm = (x / ceil).clamp(-1.5, 1.5);
                let mojo = norm.abs().powf(0.25);
                if mojo > 0.0 {
                    ((norm * mojo * std::f64::consts::FRAC_PI_2).sin() / mojo) * ceil
                } else {
                    x
                }
            }
        };

        let os_steps = if cfg.oversample_4x { 4 } else { 2 };
        let mut acc_l = 0.0;
        let mut acc_r = 0.0;

        for step in 1..=os_steps {
            let t = (step as f64) / (os_steps as f64);
            let sub_l = shape_sample(
                self.prev_in_l * (1.0 - t) + target_l * t,
                cfg.hard_shape,
                refclip,
            );
            let sub_r = shape_sample(
                self.prev_in_r * (1.0 - t) + target_r * t,
                cfg.hard_shape,
                refclip,
            );
            let (cl, cr) = self.clip_substep(sub_l, sub_r, refclip, hardness, softness);
            acc_l += cl;
            acc_r += cr;
        }

        self.prev_in_l = target_l;
        self.prev_in_r = target_r;

        let out_l = (acc_l / (os_steps as f64)).clamp(-refclip, refclip) * comp_gain;
        let out_r = (acc_r / (os_steps as f64)).clamp(-refclip, refclip) * comp_gain;

        StereoFrame::new(out_l as f32, out_r as f32)
    }

    pub fn process(&mut self, frame: StereoFrame) -> StereoFrame {
        frame
    }
}
