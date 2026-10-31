//! Masteraid MASS-Style 4-Band Parallel Passive Mastering EQ (`source/dsp/eq_mass.rs`)
//! Adapts Airwindows `Baxandall2` (`Baxandall2Proc.cpp`) & `EQ` (`EQProc.cpp`) interleaved `flip`
//! capacitor state into a 4-band parallel passive LC network (`LOW`, `LO MID`, `HI MID`, `HIGH`).

use super::shared::{AirwindowsBiquad, AirwindowsDither, StereoFrame};

pub const LOW_FREQS: [f64; 6] = [33.0, 47.0, 68.0, 100.0, 150.0, 220.0];
pub const LMF_FREQS: [f64; 6] = [150.0, 220.0, 330.0, 470.0, 680.0, 1000.0];
pub const HMF_FREQS: [f64; 6] = [680.0, 1000.0, 1500.0, 2200.0, 3300.0, 4700.0];
pub const HF_FREQS: [f64; 6] = [3300.0, 4700.0, 6800.0, 10000.0, 15000.0, 20000.0];

#[derive(Clone, Copy, Debug)]
pub struct MassBandConfig {
    pub bypass: bool,
    pub freq_idx: usize,
    pub gain_db: f32,
    pub q: f32,
    pub is_shelf: bool,
}

pub struct EqMass {
    sample_rate: f64,
    flip: bool,
    dither: AirwindowsDither,
    bands: [AirwindowsBiquad; 4],
    last_freq_idx: [usize; 4],
    last_gain_db: [f32; 4],
    last_q: [f32; 4],
    last_shelf: [bool; 4],
}

impl Default for EqMass {
    fn default() -> Self {
        Self {
            sample_rate: 48000.0,
            flip: false,
            dither: AirwindowsDither::default(),
            bands: [AirwindowsBiquad::default(); 4],
            last_freq_idx: [99; 4],
            last_gain_db: [f32::NAN; 4],
            last_q: [f32::NAN; 4],
            last_shelf: [false; 4],
        }
    }
}

impl EqMass {
    pub fn set_sample_rate(&mut self, sr: f32) {
        let new_sr = (sr as f64).max(22050.0);
        if (new_sr - self.sample_rate).abs() > 1.0 {
            self.sample_rate = new_sr;
            self.last_freq_idx = [99; 4];
        }
    }

    pub fn reset(&mut self) {
        self.flip = false;
        for b in &mut self.bands {
            b.reset();
        }
    }

    fn update_band(&mut self, idx: usize, cfg: &MassBandConfig) {
        let f_idx = cfg.freq_idx.min(5);
        if self.last_freq_idx[idx] == f_idx
            && (self.last_gain_db[idx] - cfg.gain_db).abs() < 1.0e-3
            && (self.last_q[idx] - cfg.q).abs() < 1.0e-3
            && self.last_shelf[idx] == cfg.is_shelf
        {
            return;
        }
        self.last_freq_idx[idx] = f_idx;
        self.last_gain_db[idx] = cfg.gain_db;
        self.last_q[idx] = cfg.q;
        self.last_shelf[idx] = cfg.is_shelf;

        let freq = match idx {
            0 => LOW_FREQS[f_idx],
            1 => LMF_FREQS[f_idx],
            2 => HMF_FREQS[f_idx],
            _ => HF_FREQS[f_idx],
        };

        // Constrain mastering gain & couple Q with passive inductor bandwidth
        let q_val = (cfg.q as f64).clamp(0.35, 5.0);
        let q_gain_scale = 0.85 + 0.15 * (q_val / 1.5).sqrt().clamp(0.6, 1.35);
        let eff_gain_db = (cfg.gain_db as f64) * 0.68 * q_gain_scale;

        if cfg.is_shelf && idx <= 1 {
            self.bands[idx].set_low_shelf(
                freq,
                (q_val * 0.55).clamp(0.35, 1.4),
                eff_gain_db,
                self.sample_rate,
            );
        } else if cfg.is_shelf && idx >= 2 {
            self.bands[idx].set_high_shelf(
                freq,
                (q_val * 0.55).clamp(0.35, 1.4),
                eff_gain_db,
                self.sample_rate,
            );
        } else {
            self.bands[idx].set_peaking(
                freq,
                (q_val * 0.65).clamp(0.30, 3.2),
                eff_gain_db,
                self.sample_rate,
            );
        }
    }

    pub fn process_with_bands(
        &mut self,
        frame: StereoFrame,
        configs: &[MassBandConfig; 4],
    ) -> StereoFrame {
        let mut l = frame.left as f64;
        let mut r = frame.right as f64;
        self.dither.guard_denormals(&mut l, &mut r);

        self.flip = !self.flip;
        let flip = self.flip;

        // Parallel passive tank summation: each active band computes its phase/magnitude delta
        // in parallel from the input bus, then combines with subtle passive coil compression
        let mut delta_sum_l = 0.0;
        let mut delta_sum_r = 0.0;

        for (i, config) in configs.iter().enumerate() {
            self.update_band(i, config);
            if config.bypass || config.gain_db.abs() < 0.01 {
                continue;
            }
            let (bl, br) = self.bands[i].process_interleaved(l, r, flip);
            delta_sum_l += bl - l;
            delta_sum_r += br - r;
        }

        // Subtle passive LC tank interaction (Airwindows-style soft sin saturation on summed band deltas)
        let tank_l = if delta_sum_l.abs() > 1.0e-5 {
            (delta_sum_l * 0.85).sin() / 0.85
        } else {
            delta_sum_l
        };
        let tank_r = if delta_sum_r.abs() > 1.0e-5 {
            (delta_sum_r * 0.85).sin() / 0.85
        } else {
            delta_sum_r
        };

        let out_l = l + tank_l;
        let out_r = r + tank_r;

        StereoFrame::new(out_l as f32, out_r as f32)
    }

    pub fn process(&mut self, frame: StereoFrame) -> StereoFrame {
        frame
    }
}
