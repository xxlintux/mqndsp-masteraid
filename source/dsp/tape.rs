//! Masteraid Mastering Tape Recorder (`source/dsp/tape.rs`)
//! Adapts and constrains Airwindows `ToTape8`
//! with ATR-102 Mastering Tape behavior:
//! - `15 IPS` (warmer, stronger low head-bump, vintage presence) vs `30 IPS` (wide bandwidth, cleaner highs)
//! - `456` (classic warm oxide saturation) vs `GP9` (+3 dB higher flux headroom, tighter lows)
//! - `Dubly` compander dynamics (Dolby A/SR encode & decode) for silky, open top-end air without grit
//! - `Drive`, `Bias` (golden-ratio 9-step slew rate ladder), `Head Bump` (`30..150 Hz`), and `Flutter`
//! - Final `ClipOnly3` noise-masked dithered peak limiter on tape output

use super::shared::{db_to_gain, AirwindowsDither, StereoFrame};
use std::f64::consts::{FRAC_PI_2, PI, TAU};

#[derive(Clone, Copy, Debug)]
pub struct TapeConfig {
    pub is_30_ips: bool,   // false = 15 IPS, true = 30 IPS
    pub is_gp9: bool,      // false = 456 formula, true = GP9 formula
    pub drive_db: f32,     // -10..+10 dB
    pub bias_db: f32,      // -10..+10 dB (controls 9-step golden-ratio slew ladder & under-bias hysteresis)
    pub head_bump_db: f32, // -10..+10 dB
    pub bump_freq_hz: f32, // 30..150 Hz
    pub flutter_pct: f32,  // 0..100 %
    /// Keep the machine level-neutral: normalize the small-signal gain and cancel drive gain
    pub compensate: bool,
}

pub struct Tape {
    sample_rate: f64,
    dither: AirwindowsDither,
    // Dubly dynamic compander state
    iir_enc_l: f64,
    iir_dec_l: f64,
    comp_enc_l: f64,
    comp_dec_l: f64,
    avg_enc_l: f64,
    avg_dec_l: f64,
    iir_enc_r: f64,
    iir_dec_r: f64,
    comp_enc_r: f64,
    comp_dec_r: f64,
    avg_enc_r: f64,
    avg_dec_r: f64,
    // Flutter circular delay lines (1002 samples)
    d_l: [f64; 1002],
    d_r: [f64; 1002],
    sweep_l: f64,
    sweep_r: f64,
    nextmax_l: f64,
    nextmax_r: f64,
    gcount: usize,
    // Bias 9-step golden-ratio slew ladder (27 values: [prev_l, prev_r, thresh] * 9)
    gslew: [f64; 27],
    // MidRoller & Sub-bass servo state
    iir_mid_roller_l: f64,
    iir_low_cutoff_l: f64,
    iir_mid_roller_r: f64,
    iir_low_cutoff_r: f64,
    // HeadBump resonator state
    head_bump_l: f64,
    head_bump_r: f64,
    // hdbA / hdbB: [freq, reso, a0, a1, a2, b1, b2, sL1, sL2, sR1, sR2]
    hdb_a: [f64; 11],
    hdb_b: [f64; 11],
    last_bump_freq: f32,
    last_30_ips: bool,
    // ClipOnly3 safety exit state
    last_sample_l: f64,
    last_sample_r: f64,
    was_pos_clip_l: bool,
    was_neg_clip_l: bool,
    was_pos_clip_r: bool,
    was_neg_clip_r: bool,
}

impl Default for Tape {
    fn default() -> Self {
        let mut t = Self {
            sample_rate: 48000.0,
            dither: AirwindowsDither::default(),
            iir_enc_l: 0.0,
            iir_dec_l: 0.0,
            comp_enc_l: 0.0,
            comp_dec_l: 0.0,
            avg_enc_l: 0.0,
            avg_dec_l: 0.0,
            iir_enc_r: 0.0,
            iir_dec_r: 0.0,
            comp_enc_r: 0.0,
            comp_dec_r: 0.0,
            avg_enc_r: 0.0,
            avg_dec_r: 0.0,
            d_l: [0.0; 1002],
            d_r: [0.0; 1002],
            sweep_l: 0.0,
            sweep_r: 0.0,
            nextmax_l: 0.5,
            nextmax_r: 0.5,
            gcount: 0,
            gslew: [0.0; 27],
            iir_mid_roller_l: 0.0,
            iir_low_cutoff_l: 0.0,
            iir_mid_roller_r: 0.0,
            iir_low_cutoff_r: 0.0,
            head_bump_l: 0.0,
            head_bump_r: 0.0,
            hdb_a: [0.0; 11],
            hdb_b: [0.0; 11],
            last_bump_freq: -1.0,
            last_30_ips: true,
            last_sample_l: 0.0,
            last_sample_r: 0.0,
            was_pos_clip_l: false,
            was_neg_clip_l: false,
            was_pos_clip_r: false,
            was_neg_clip_r: false,
        };
        t.reset();
        t
    }
}

impl Tape {
    pub fn set_sample_rate(&mut self, sr: f32) {
        let new_sr = (sr as f64).max(22050.0);
        if (new_sr - self.sample_rate).abs() > 1.0 {
            self.sample_rate = new_sr;
            self.last_bump_freq = -1.0;
        }
    }

    pub fn reset(&mut self) {
        self.iir_enc_l = 0.0;
        self.iir_dec_l = 0.0;
        self.comp_enc_l = 0.0;
        self.comp_dec_l = 0.0;
        self.avg_enc_l = 0.0;
        self.avg_dec_l = 0.0;
        self.iir_enc_r = 0.0;
        self.iir_dec_r = 0.0;
        self.comp_enc_r = 0.0;
        self.comp_dec_r = 0.0;
        self.avg_enc_r = 0.0;
        self.avg_dec_r = 0.0;
        self.d_l = [0.0; 1002];
        self.d_r = [0.0; 1002];
        self.sweep_l = 0.0;
        self.sweep_r = 0.0;
        self.nextmax_l = 0.5;
        self.nextmax_r = 0.5;
        self.gcount = 0;
        self.gslew = [0.0; 27];
        self.iir_mid_roller_l = 0.0;
        self.iir_low_cutoff_l = 0.0;
        self.iir_mid_roller_r = 0.0;
        self.iir_low_cutoff_r = 0.0;
        self.head_bump_l = 0.0;
        self.head_bump_r = 0.0;
        self.hdb_a = [0.0; 11];
        self.hdb_b = [0.0; 11];
        self.last_sample_l = 0.0;
        self.last_sample_r = 0.0;
        self.was_pos_clip_l = false;
        self.was_neg_clip_l = false;
        self.was_pos_clip_r = false;
        self.was_neg_clip_r = false;
    }

    fn update_head_biquads(&mut self, bump_freq_hz: f32, is_30_ips: bool) {
        if (bump_freq_hz - self.last_bump_freq).abs() < 0.5 && is_30_ips == self.last_30_ips {
            return;
        }
        self.last_bump_freq = bump_freq_hz;
        self.last_30_ips = is_30_ips;

        // 15 IPS shifts head resonance slightly lower; 30 IPS shifts higher and tighter
        let speed_mult = if is_30_ips { 1.10 } else { 0.90 };
        let f_hz = (bump_freq_hz.clamp(30.0, 150.0) as f64) * speed_mult;
        let freq_norm = (f_hz / self.sample_rate).clamp(0.0005, 0.45);

        self.hdb_a[0] = freq_norm;
        self.hdb_b[0] = freq_norm * 0.9375;
        self.hdb_a[1] = 0.6180339887498949; // golden-ratio resonance Q
        self.hdb_b[1] = 0.6180339887498949;

        // Filter A: Bandpass
        let k_a = (PI * self.hdb_a[0]).tan();
        let norm_a = 1.0 / (1.0 + k_a / self.hdb_a[1] + k_a * k_a);
        self.hdb_a[2] = k_a / self.hdb_a[1] * norm_a; // a0
        self.hdb_a[3] = 0.0;                          // a1
        self.hdb_a[4] = -self.hdb_a[2];               // a2
        self.hdb_a[5] = 2.0 * (k_a * k_a - 1.0) * norm_a; // b1
        self.hdb_a[6] = (1.0 - k_a / self.hdb_a[1] + k_a * k_a) * norm_a; // b2

        // Filter B: Bandpass
        let k_b = (PI * self.hdb_b[0]).tan();
        let norm_b = 1.0 / (1.0 + k_b / self.hdb_b[1] + k_b * k_b);
        self.hdb_b[2] = k_b / self.hdb_b[1] * norm_b; // a0
        self.hdb_b[3] = 0.0;                          // a1
        self.hdb_b[4] = -self.hdb_b[2];               // a2
        self.hdb_b[5] = 2.0 * (k_b * k_b - 1.0) * norm_b; // b1
        self.hdb_b[6] = (1.0 - k_b / self.hdb_b[1] + k_b * k_b) * norm_b; // b2
    }

    pub fn process_with_config(&mut self, frame: StereoFrame, cfg: &TapeConfig) -> StereoFrame {
        let mut l = frame.left as f64;
        let mut r = frame.right as f64;
        self.dither.guard_denormals(&mut l, &mut r);

        self.update_head_biquads(cfg.bump_freq_hz, cfg.is_30_ips);

        let overallscale = (self.sample_rate / 44100.0).max(0.5);

        // Formula flux headroom: GP9 has +2.5 dB higher saturation headroom; 456 saturates earlier
        let formula_gain_scale = if cfg.is_gp9 { 0.84 } else { 1.10 };
        let input_gain = (db_to_gain(cfg.drive_db.clamp(-10.0, 10.0)) as f64) * formula_gain_scale;

        // Level-neutral compensation: cancels drive gain cleanly so Drive changes timbre only
        let output_comp = if cfg.compensate {
            1.0 / input_gain
        } else {
            1.0
        };

        // Dubly Compander setup (Dolby SR/A dynamic encode/decode)
        // 30 IPS has wider HF bandwidth; GP9 provides modern high-flux clarity
        let dubly_b: f64 = if cfg.is_gp9 { 0.45 } else { 0.55 };
        let dubly_c: f64 = if cfg.is_30_ips { 0.58 } else { 0.44 };
        let dubly_amount = dubly_b * 2.0;
        let outly_amount = ((1.0 - dubly_b) * -2.0).max(-1.0);
        let iir_enc_freq = ((1.0 - dubly_c) / overallscale).clamp(0.01, 0.98);
        let iir_dec_freq = (dubly_c / overallscale).clamp(0.01, 0.98);
        let iir_mid_freq = (((dubly_c * 0.618) + 0.382) / overallscale).clamp(0.01, 0.98);

        // Flutter (6th-power gentle depth scaling)
        let flut_norm = (cfg.flutter_pct.clamp(0.0, 100.0) / 100.0) as f64;
        let flut_depth = (flut_norm.powi(5) * overallscale * 45.0).min(498.0);
        let flut_freq = (0.015 * flut_norm.powi(2) + 0.005) / overallscale;

        // Bias: -10..+10 dB mapped to -1.0..+1.0
        let bias = (cfg.bias_db.clamp(-10.0, 10.0) / 10.0) as f64;
        let under_bias = if bias < 0.0 {
            (bias.powi(4) * 0.25) / overallscale
        } else {
            0.0
        };
        let mut over_bias = if bias > 0.0 {
            (1.0 - bias).powi(3) / overallscale
        } else {
            1.0 / overallscale
        };

        // 9-step golden-ratio slew ladder (overBias * phi^n)
        for i in 0..9 {
            let thresh_idx = (8 - i) * 3 + 2;
            self.gslew[thresh_idx] = over_bias;
            over_bias *= 1.6180339887498949;
        }

        // HeadBump settings
        let bump_norm = ((cfg.head_bump_db.clamp(-10.0, 10.0) + 10.0) / 20.0) as f64;
        let speed_bump = if cfg.is_30_ips { 0.65 } else { 0.85 };
        let head_bump_drive = (bump_norm * speed_bump * 0.08) / overallscale;
        let head_bump_mix = bump_norm * speed_bump * 0.45;
        let sub_curve = (bump_norm * speed_bump * PI).sin();
        let iir_sub_freq = (sub_curve * 0.008) / overallscale;

        l *= input_gain;
        r *= input_gain;

        // 1. Dubly dynamic compander encode (L & R)
        self.iir_enc_l = (self.iir_enc_l * (1.0 - iir_enc_freq)) + (l * iir_enc_freq);
        let mut high_part_l = (l - self.iir_enc_l) * 2.848 + self.avg_enc_l;
        self.avg_enc_l = (l - self.iir_enc_l) * 1.152;
        high_part_l = high_part_l.clamp(-1.0, 1.0);
        let mut dubly_l = high_part_l.abs();
        if dubly_l > 0.0 {
            let adjust = (1.0 + 255.0 * dubly_l).ln() / 2.40823996531;
            if adjust > 0.0 {
                dubly_l /= adjust;
            }
            self.comp_enc_l = (self.comp_enc_l * (1.0 - iir_enc_freq)) + (dubly_l * iir_enc_freq);
            l += high_part_l * self.comp_enc_l * dubly_amount;
        }

        self.iir_enc_r = (self.iir_enc_r * (1.0 - iir_enc_freq)) + (r * iir_enc_freq);
        let mut high_part_r = (r - self.iir_enc_r) * 2.848 + self.avg_enc_r;
        self.avg_enc_r = (r - self.iir_enc_r) * 1.152;
        high_part_r = high_part_r.clamp(-1.0, 1.0);
        let mut dubly_r = high_part_r.abs();
        if dubly_r > 0.0 {
            let adjust = (1.0 + 255.0 * dubly_r).ln() / 2.40823996531;
            if adjust > 0.0 {
                dubly_r /= adjust;
            }
            self.comp_enc_r = (self.comp_enc_r * (1.0 - iir_enc_freq)) + (dubly_r * iir_enc_freq);
            r += high_part_r * self.comp_enc_r * dubly_amount;
        }

        // 2. Flutter: 1000-sample circular delay line with dual-LFO stereo drift
        if flut_depth > 0.0 {
            if self.gcount >= 1000 {
                self.gcount = 999;
            }
            self.d_l[self.gcount] = l;
            let offset_l = flut_depth + (flut_depth * self.sweep_l.sin());
            self.sweep_l += self.nextmax_l * flut_freq;
            if self.sweep_l > TAU {
                self.sweep_l -= TAU;
                let flut_a = 0.24 + self.dither.next_unit_l() * 0.74;
                let flut_b = 0.24 + self.dither.next_unit_l() * 0.74;
                self.nextmax_l = if (flut_a - (self.sweep_r + self.nextmax_r).sin()).abs()
                    < (flut_b - (self.sweep_r + self.nextmax_r).sin()).abs()
                {
                    flut_a
                } else {
                    flut_b
                };
            }
            let count_l = self.gcount + (offset_l.floor() as usize);
            let idx_l0 = if count_l > 999 { count_l - 1000 } else { count_l };
            let idx_l1 = if count_l + 1 > 999 { count_l + 1 - 1000 } else { count_l + 1 };
            let frac_l = offset_l - offset_l.floor();
            l = self.d_l[idx_l0] * (1.0 - frac_l) + self.d_l[idx_l1] * frac_l;

            self.d_r[self.gcount] = r;
            let offset_r = flut_depth + (flut_depth * self.sweep_r.sin());
            self.sweep_r += self.nextmax_r * flut_freq;
            if self.sweep_r > TAU {
                self.sweep_r -= TAU;
                let flut_a = 0.24 + self.dither.next_unit_r() * 0.74;
                let flut_b = 0.24 + self.dither.next_unit_r() * 0.74;
                self.nextmax_r = if (flut_a - (self.sweep_l + self.nextmax_l).sin()).abs()
                    < (flut_b - (self.sweep_l + self.nextmax_l).sin()).abs()
                {
                    flut_a
                } else {
                    flut_b
                };
            }
            let count_r = self.gcount + (offset_r.floor() as usize);
            let idx_r0 = if count_r > 999 { count_r - 1000 } else { count_r };
            let idx_r1 = if count_r + 1 > 999 { count_r + 1 - 1000 } else { count_r + 1 };
            let frac_r = offset_r - offset_r.floor();
            r = self.d_r[idx_r0] * (1.0 - frac_r) + self.d_r[idx_r1] * frac_r;

            if self.gcount == 0 {
                self.gcount = 999;
            } else {
                self.gcount -= 1;
            }
        }

        // 3. Bias routine: 9-step golden-ratio slew ladder with magnetic hysteresis
        if bias.abs() > 0.001 {
            for x in (0..27).step_by(3) {
                if under_bias > 0.0 {
                    let stuck_l = ((l - (self.gslew[x] / 0.975)).abs() / under_bias).min(1.0);
                    l = (l * stuck_l) + ((self.gslew[x] / 0.975) * (1.0 - stuck_l));
                    let stuck_r = ((r - (self.gslew[x + 1] / 0.975)).abs() / under_bias).min(1.0);
                    r = (r * stuck_r) + ((self.gslew[x + 1] / 0.975) * (1.0 - stuck_r));
                }
                let diff_l = l - self.gslew[x];
                if diff_l > self.gslew[x + 2] {
                    l = self.gslew[x] + self.gslew[x + 2];
                } else if -diff_l > self.gslew[x + 2] {
                    l = self.gslew[x] - self.gslew[x + 2];
                }
                self.gslew[x] = l * 0.975;

                let diff_r = r - self.gslew[x + 1];
                if diff_r > self.gslew[x + 2] {
                    r = self.gslew[x + 1] + self.gslew[x + 2];
                } else if -diff_r > self.gslew[x + 2] {
                    r = self.gslew[x + 1] - self.gslew[x + 2];
                }
                self.gslew[x + 1] = r * 0.975;
            }
        }

        // 4. Core ToTape saturation algorithm (sine low-end saturation & cosine high-end thinning)
        self.iir_mid_roller_l =
            (self.iir_mid_roller_l * (1.0 - iir_mid_freq)) + (l * iir_mid_freq);
        let mut highs_l = l - self.iir_mid_roller_l;
        let mut lows_l = self.iir_mid_roller_l;
        if iir_sub_freq > 0.0 {
            self.iir_low_cutoff_l =
                (self.iir_low_cutoff_l * (1.0 - iir_sub_freq)) + (lows_l * iir_sub_freq);
            lows_l -= self.iir_low_cutoff_l;
        }
        lows_l = lows_l.clamp(-FRAC_PI_2, FRAC_PI_2).sin();
        let mut thinned_high_l = (highs_l.abs() * FRAC_PI_2).min(FRAC_PI_2);
        thinned_high_l = 1.0 - thinned_high_l.cos();
        if highs_l < 0.0 {
            thinned_high_l = -thinned_high_l;
        }
        highs_l -= thinned_high_l;

        self.iir_mid_roller_r =
            (self.iir_mid_roller_r * (1.0 - iir_mid_freq)) + (r * iir_mid_freq);
        let mut highs_r = r - self.iir_mid_roller_r;
        let mut lows_r = self.iir_mid_roller_r;
        if iir_sub_freq > 0.0 {
            self.iir_low_cutoff_r =
                (self.iir_low_cutoff_r * (1.0 - iir_sub_freq)) + (lows_r * iir_sub_freq);
            lows_r -= self.iir_low_cutoff_r;
        }
        lows_r = lows_r.clamp(-FRAC_PI_2, FRAC_PI_2).sin();
        let mut thinned_high_r = (highs_r.abs() * FRAC_PI_2).min(FRAC_PI_2);
        thinned_high_r = 1.0 - thinned_high_r.cos();
        if highs_r < 0.0 {
            thinned_high_r = -thinned_high_r;
        }
        highs_r -= thinned_high_r;

        // 5. Dual cascaded HeadBump biquads with cubic damping
        let mut head_bump_sample_l = 0.0;
        let mut head_bump_sample_r = 0.0;
        if head_bump_mix > 0.0 {
            self.head_bump_l += lows_l * head_bump_drive;
            self.head_bump_l -= self.head_bump_l.powi(3) * (0.0618 / overallscale.sqrt());
            self.head_bump_r += lows_r * head_bump_drive;
            self.head_bump_r -= self.head_bump_r.powi(3) * (0.0618 / overallscale.sqrt());

            // Left Stage A
            let biq_l = (self.head_bump_l * self.hdb_a[2]) + self.hdb_a[7];
            self.hdb_a[7] = (self.head_bump_l * self.hdb_a[3]) - (biq_l * self.hdb_a[5]) + self.hdb_a[8];
            self.hdb_a[8] = (self.head_bump_l * self.hdb_a[4]) - (biq_l * self.hdb_a[6]);
            // Left Stage B
            head_bump_sample_l = (biq_l * self.hdb_b[2]) + self.hdb_b[7];
            self.hdb_b[7] = (biq_l * self.hdb_b[3]) - (head_bump_sample_l * self.hdb_b[5]) + self.hdb_b[8];
            self.hdb_b[8] = (biq_l * self.hdb_b[4]) - (head_bump_sample_l * self.hdb_b[6]);

            // Right Stage A
            let biq_r = (self.head_bump_r * self.hdb_a[2]) + self.hdb_a[9];
            self.hdb_a[9] = (self.head_bump_r * self.hdb_a[3]) - (biq_r * self.hdb_a[5]) + self.hdb_a[10];
            self.hdb_a[10] = (self.head_bump_r * self.hdb_a[4]) - (biq_r * self.hdb_a[6]);
            // Right Stage B
            head_bump_sample_r = (biq_r * self.hdb_b[2]) + self.hdb_b[9];
            self.hdb_b[9] = (biq_r * self.hdb_b[3]) - (head_bump_sample_r * self.hdb_b[5]) + self.hdb_b[10];
            self.hdb_b[10] = (biq_r * self.hdb_b[4]) - (head_bump_sample_r * self.hdb_b[6]);
        }

        l = lows_l + highs_l + (head_bump_sample_l * head_bump_mix);
        r = lows_r + highs_r + (head_bump_sample_r * head_bump_mix);

        // 6. Dubly dynamic compander decode (L & R)
        self.iir_dec_l = (self.iir_dec_l * (1.0 - iir_dec_freq)) + (l * iir_dec_freq);
        let mut dec_high_l = (l - self.iir_dec_l) * 2.628 + self.avg_dec_l;
        self.avg_dec_l = (l - self.iir_dec_l) * 1.372;
        dec_high_l = dec_high_l.clamp(-1.0, 1.0);
        let mut dec_dubly_l = dec_high_l.abs();
        if dec_dubly_l > 0.0 {
            let adjust = (1.0 + 255.0 * dec_dubly_l).ln() / 2.40823996531;
            if adjust > 0.0 {
                dec_dubly_l /= adjust;
            }
            self.comp_dec_l = (self.comp_dec_l * (1.0 - iir_dec_freq)) + (dec_dubly_l * iir_dec_freq);
            l += dec_high_l * self.comp_dec_l * outly_amount;
        }

        self.iir_dec_r = (self.iir_dec_r * (1.0 - iir_dec_freq)) + (r * iir_dec_freq);
        let mut dec_high_r = (r - self.iir_dec_r) * 2.628 + self.avg_dec_r;
        self.avg_dec_r = (r - self.iir_dec_r) * 1.372;
        dec_high_r = dec_high_r.clamp(-1.0, 1.0);
        let mut dec_dubly_r = dec_high_r.abs();
        if dec_dubly_r > 0.0 {
            let adjust = (1.0 + 255.0 * dec_dubly_r).ln() / 2.40823996531;
            if adjust > 0.0 {
                dec_dubly_r /= adjust;
            }
            self.comp_dec_r = (self.comp_dec_r * (1.0 - iir_dec_freq)) + (dec_dubly_r * iir_dec_freq);
            r += dec_high_r * self.comp_dec_r * outly_amount;
        }

        l *= output_comp;
        r *= output_comp;

        // 7. Output Peak Safety: Airwindows ClipOnly3 noise-masked dithered edge transition
        let noise_l = 1.0 - (self.dither.next_unit_l() * 0.076);
        let noise_r = 1.0 - (self.dither.next_unit_r() * 0.076);
        const REFCLIP: f64 = 0.9885; // -0.1 dBFS ceiling
        const CLIP_THRESH: f64 = REFCLIP * 0.9549925859;
        const HARD_REF: f64 = REFCLIP * 0.7058208;
        const SOFT_REF: f64 = REFCLIP * 0.2491717;

        if self.was_pos_clip_l {
            if l < self.last_sample_l {
                self.last_sample_l = (hard_ref_noise(HARD_REF, noise_l)) + (l * 0.2609148);
            } else {
                self.last_sample_l = (soft_ref_noise(SOFT_REF, noise_l)) + (self.last_sample_l * 0.7390851);
            }
        }
        self.was_pos_clip_l = false;
        if l > CLIP_THRESH {
            self.was_pos_clip_l = true;
            l = (hard_ref_noise(HARD_REF, noise_l)) + (self.last_sample_l * 0.2609148);
        }

        if self.was_neg_clip_l {
            if l > self.last_sample_l {
                self.last_sample_l = (-hard_ref_noise(HARD_REF, noise_l)) + (l * 0.2609148);
            } else {
                self.last_sample_l = (-soft_ref_noise(SOFT_REF, noise_l)) + (self.last_sample_l * 0.7390851);
            }
        }
        self.was_neg_clip_l = false;
        if l < -CLIP_THRESH {
            self.was_neg_clip_l = true;
            l = (-hard_ref_noise(HARD_REF, noise_l)) + (self.last_sample_l * 0.2609148);
        }

        if self.was_pos_clip_r {
            if r < self.last_sample_r {
                self.last_sample_r = (hard_ref_noise(HARD_REF, noise_r)) + (r * 0.2609148);
            } else {
                self.last_sample_r = (soft_ref_noise(SOFT_REF, noise_r)) + (self.last_sample_r * 0.7390851);
            }
        }
        self.was_pos_clip_r = false;
        if r > CLIP_THRESH {
            self.was_pos_clip_r = true;
            r = (hard_ref_noise(HARD_REF, noise_r)) + (self.last_sample_r * 0.2609148);
        }

        if self.was_neg_clip_r {
            if r > self.last_sample_r {
                self.last_sample_r = (-hard_ref_noise(HARD_REF, noise_r)) + (r * 0.2609148);
            } else {
                self.last_sample_r = (-soft_ref_noise(SOFT_REF, noise_r)) + (self.last_sample_r * 0.7390851);
            }
        }
        self.was_neg_clip_r = false;
        if r < -CLIP_THRESH {
            self.was_neg_clip_r = true;
            r = (-hard_ref_noise(HARD_REF, noise_r)) + (self.last_sample_r * 0.2609148);
        }

        let slew_l = (self.last_sample_l - l).abs();
        let postclip_l = REFCLIP / (1.0 + slew_l * 1.3986);
        let out_l = self.last_sample_l.clamp(-postclip_l, postclip_l);
        self.last_sample_l = l.clamp(-postclip_l, postclip_l);

        let slew_r = (self.last_sample_r - r).abs();
        let postclip_r = REFCLIP / (1.0 + slew_r * 1.3986);
        let out_r = self.last_sample_r.clamp(-postclip_r, postclip_r);
        self.last_sample_r = r.clamp(-postclip_r, postclip_r);

        StereoFrame::new(out_l as f32, out_r as f32)
    }

    pub fn process(&mut self, frame: StereoFrame) -> StereoFrame {
        frame
    }
}

#[inline(always)]
fn hard_ref_noise(hard_ref: f64, noise: f64) -> f64 {
    hard_ref * noise
}

#[inline(always)]
fn soft_ref_noise(soft_ref: f64, noise: f64) -> f64 {
    soft_ref * noise
}
