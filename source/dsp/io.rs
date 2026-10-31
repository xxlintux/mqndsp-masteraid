//! Masteraid I/O Stage, Mid/Side Bass Monoizer, and Metering (`source/dsp/io.rs`)

use super::shared::{db_to_gain, gain_to_db, AirwindowsBiquad, StereoFrame};

pub struct InputStage {
    pub peak_db_l: f32,
    pub peak_db_r: f32,
    pub rms_db_l: f32,
    pub rms_db_r: f32,
    env_l: f32,
    env_r: f32,
    ms_l: f32,
    ms_r: f32,
    sample_rate: f32,
    peak_release: f32,
    rms_coef: f32,
}

impl Default for InputStage {
    fn default() -> Self {
        let mut s = Self {
            peak_db_l: -60.0,
            peak_db_r: -60.0,
            rms_db_l: -60.0,
            rms_db_r: -60.0,
            env_l: 0.0,
            env_r: 0.0,
            ms_l: 0.0,
            ms_r: 0.0,
            sample_rate: 48000.0,
            peak_release: 0.0,
            rms_coef: 0.0,
        };
        s.set_sample_rate(48000.0);
        s
    }
}

impl InputStage {
    /// Time-based ballistics: PPM-style peak (fast attack, ~0.7 s fall) and 300 ms RMS window.
    pub fn set_sample_rate(&mut self, sr: f32) {
        self.sample_rate = sr.max(8000.0);
        self.peak_release = (-1.0 / (self.sample_rate * 0.7)).exp();
        self.rms_coef = (-1.0 / (self.sample_rate * 0.3)).exp();
    }

    pub fn reset(&mut self) {
        self.peak_db_l = -60.0;
        self.peak_db_r = -60.0;
        self.rms_db_l = -60.0;
        self.rms_db_r = -60.0;
        self.env_l = 0.0;
        self.env_r = 0.0;
        self.ms_l = 0.0;
        self.ms_r = 0.0;
    }

    pub fn process(&mut self, frame: StereoFrame, trim_db: f32, polarity: bool, mono: bool) -> StereoFrame {
        let (raw_l, raw_r) = if mono {
            let m = 0.5 * (frame.left + frame.right);
            (m, m)
        } else {
            (frame.left, frame.right)
        };
        let polarity_gain = if polarity { -1.0 } else { 1.0 };
        let gain = db_to_gain(trim_db) * polarity_gain;

        let out_l = raw_l * gain;
        let out_r = raw_r * gain;

        // PPM-style peak follower (instant attack, slow fall) for the Input Level ladder
        let abs_l = out_l.abs();
        let abs_r = out_r.abs();
        self.env_l = if abs_l > self.env_l {
            abs_l
        } else {
            self.env_l * self.peak_release
        };
        self.env_r = if abs_r > self.env_r {
            abs_r
        } else {
            self.env_r * self.peak_release
        };

        // 300 ms RMS (mean-square one-pole)
        self.ms_l = self.ms_l * self.rms_coef + out_l * out_l * (1.0 - self.rms_coef);
        self.ms_r = self.ms_r * self.rms_coef + out_r * out_r * (1.0 - self.rms_coef);

        self.peak_db_l = gain_to_db(self.env_l).clamp(-60.0, 12.0);
        self.peak_db_r = gain_to_db(self.env_r).clamp(-60.0, 12.0);
        self.rms_db_l = gain_to_db(self.ms_l.max(0.0).sqrt()).clamp(-60.0, 12.0);
        self.rms_db_r = gain_to_db(self.ms_r.max(0.0).sqrt()).clamp(-60.0, 12.0);

        StereoFrame::new(out_l, out_r)
    }
}

/// Butterworth pole Qs for a cascade of second-order sections, indexed by slope position
/// (`0` = 12 dB/oct, `1` = 24 dB/oct, `2` = 36 dB/oct).
const FILTER_Q: [[f64; 3]; 3] = [
    [std::f64::consts::FRAC_1_SQRT_2, 0.0, 0.0],
    [0.5412, 1.3066, 0.0],
    [0.5176, std::f64::consts::FRAC_1_SQRT_2, 1.9319],
];

/// Settings for the three filter blocks, in chain order: low cut, high cut, bass mono.
#[derive(Clone, Copy, Debug)]
pub struct FilterSettings {
    pub lo_cut_hz: f32,
    pub lo_cut_slope: usize,
    pub hi_cut_hz: f32,
    pub hi_cut_slope: usize,
    pub mono_hz: f32,
    pub mono_amount_pct: f32,
    pub mono_slope: usize,
}

/// Filtering section: stereo low cut, stereo high cut, then a phase-coherent mid/side bass
/// monoizer. Each cut is a Butterworth cascade of one, two or three second-order sections, so the
/// three lever positions give 12, 24 and 36 dB per octave at the same corner frequency.
pub struct Filtering {
    sample_rate: f64,
    lo_cut: [AirwindowsBiquad; 3],
    hi_cut: [AirwindowsBiquad; 3],
    side_hp: [AirwindowsBiquad; 3],
    last_lo_hz: f32,
    last_hi_hz: f32,
    last_mono_hz: f32,
    last_lo_slope: usize,
    last_hi_slope: usize,
    last_mono_slope: usize,
}

impl Default for Filtering {
    fn default() -> Self {
        let mut filtering = Self {
            sample_rate: 48000.0,
            lo_cut: [AirwindowsBiquad::default(); 3],
            hi_cut: [AirwindowsBiquad::default(); 3],
            side_hp: [AirwindowsBiquad::default(); 3],
            last_lo_hz: -1.0,
            last_hi_hz: -1.0,
            last_mono_hz: -1.0,
            last_lo_slope: usize::MAX,
            last_hi_slope: usize::MAX,
            last_mono_slope: usize::MAX,
        };
        filtering.update_cut(CutKind::Low, 0.0, 0);
        filtering.update_cut(CutKind::High, 0.0, 0);
        filtering.update_mono(0.0, 0);
        filtering
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CutKind {
    Low,
    High,
}

impl Filtering {
    pub fn set_sample_rate(&mut self, sr: f32) {
        let new_sr = (sr as f64).max(22050.0);
        if (new_sr - self.sample_rate).abs() > 1.0 {
            self.sample_rate = new_sr;
            self.last_lo_hz = -1.0;
            self.last_hi_hz = -1.0;
            self.last_mono_hz = -1.0;
        }
    }

    pub fn reset(&mut self) {
        for stage in &mut self.lo_cut {
            stage.reset();
        }
        for stage in &mut self.hi_cut {
            stage.reset();
        }
        for stage in &mut self.side_hp {
            stage.reset();
        }
    }

    fn update_cut(&mut self, kind: CutKind, freq_hz: f32, slope: usize) {
        let slope = slope.min(2);
        let (stages, last_hz, last_slope) = match kind {
            CutKind::Low => (
                &mut self.lo_cut,
                &mut self.last_lo_hz,
                &mut self.last_lo_slope,
            ),
            CutKind::High => (
                &mut self.hi_cut,
                &mut self.last_hi_hz,
                &mut self.last_hi_slope,
            ),
        };
        if (freq_hz - *last_hz).abs() < 0.2 && slope == *last_slope {
            return;
        }
        *last_hz = freq_hz;
        *last_slope = slope;
        let f = (freq_hz as f64).clamp(10.0, self.sample_rate * 0.49);
        for (i, stage) in stages.iter_mut().enumerate() {
            let q = FILTER_Q[slope][i].max(0.05);
            match kind {
                CutKind::Low => stage.set_highpass(f, q, self.sample_rate),
                CutKind::High => stage.set_lowpass(f, q, self.sample_rate),
            }
        }
    }

    fn update_mono(&mut self, freq_hz: f32, slope: usize) {
        let slope = slope.min(2);
        if (freq_hz - self.last_mono_hz).abs() < 0.2 && slope == self.last_mono_slope {
            return;
        }
        self.last_mono_hz = freq_hz;
        self.last_mono_slope = slope;
        let f = (freq_hz as f64).clamp(20.0, 250.0);
        for (i, stage) in self.side_hp.iter_mut().enumerate() {
            stage.set_highpass(f, FILTER_Q[slope][i].max(0.05), self.sample_rate);
        }
    }

    pub fn process(&mut self, frame: StereoFrame, settings: FilterSettings) -> StereoFrame {
        let mut l = frame.left as f64;
        let mut r = frame.right as f64;

        // 1. Low cut (high-pass).
        if settings.lo_cut_hz > 0.5 {
            let slope = settings.lo_cut_slope.min(2);
            self.update_cut(CutKind::Low, settings.lo_cut_hz, slope);
            for stage in self.lo_cut.iter_mut().take(slope + 1) {
                let (out_l, out_r) = stage.process_single(l, r);
                l = out_l;
                r = out_r;
            }
        }

        // 2. High cut (low-pass). `0 Hz` is the OFF position at the clockwise end of the knob.
        if settings.hi_cut_hz > 0.5 {
            let slope = settings.hi_cut_slope.min(2);
            self.update_cut(CutKind::High, settings.hi_cut_hz, slope);
            for stage in self.hi_cut.iter_mut().take(slope + 1) {
                let (out_l, out_r) = stage.process_single(l, r);
                l = out_l;
                r = out_r;
            }
        }

        // 3. Bass mono: high-pass the Side signal, then blend it back in by Amount.
        let amt = (settings.mono_amount_pct / 100.0).clamp(0.0, 1.0) as f64;
        if amt > 0.001 && settings.mono_hz > 0.5 {
            let slope = settings.mono_slope.min(2);
            self.update_mono(settings.mono_hz, slope);

            let mid = 0.5 * (l + r);
            let original_side = 0.5 * (l - r);
            let mut side = original_side;
            for stage in self.side_hp.iter_mut().take(slope + 1) {
                let (hp, _) = stage.process_single(side, 0.0);
                side = hp;
            }
            let new_side = original_side * (1.0 - amt) + side * amt;
            l = mid + new_side;
            r = mid - new_side;
        }

        StereoFrame::new(l as f32, r as f32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 48000.0;

    fn settings(
        lo_cut_hz: f32,
        lo_cut_slope: usize,
        hi_cut_hz: f32,
        hi_cut_slope: usize,
    ) -> FilterSettings {
        FilterSettings {
            lo_cut_hz,
            lo_cut_slope,
            hi_cut_hz,
            hi_cut_slope,
            mono_hz: 60.0,
            mono_amount_pct: 0.0,
            mono_slope: 1,
        }
    }

    /// Steady-state gain in dB at `freq_hz`, measured by correlating the output against the input
    /// sine over exactly one second (an integer number of cycles for every frequency used here).
    fn gain_db(filter: &mut Filtering, freq_hz: f32, s: FilterSettings) -> f32 {
        filter.reset();
        let omega = 2.0 * std::f64::consts::PI * (freq_hz as f64) / (SR as f64);
        let settle = SR as usize / 2;
        let measure = SR as usize;
        let mut sin_acc = 0.0;
        let mut cos_acc = 0.0;
        for n in 0..settle + measure {
            let phase = omega * n as f64;
            let input = phase.sin();
            let out = filter
                .process(StereoFrame::new(input as f32, input as f32), s)
                .left as f64;
            if n >= settle {
                sin_acc += out * phase.sin();
                cos_acc += out * phase.cos();
            }
        }
        let amplitude = 2.0 * (sin_acc * sin_acc + cos_acc * cos_acc).sqrt() / measure as f64;
        20.0 * amplitude.max(1.0e-12).log10() as f32
    }

    #[test]
    fn lo_cut_corner_is_minus_3_db_at_every_slope() {
        for slope in 0..3 {
            let mut filter = Filtering::default();
            filter.set_sample_rate(SR);
            let g = gain_db(&mut filter, 100.0, settings(100.0, slope, 0.0, 1));
            assert!(
                (g + 3.0).abs() < 0.5,
                "slope {slope}: 100 Hz corner measured {g:.2} dB"
            );
        }
    }

    #[test]
    fn hi_cut_corner_is_minus_3_db_at_every_slope() {
        for slope in 0..3 {
            let mut filter = Filtering::default();
            filter.set_sample_rate(SR);
            let g = gain_db(&mut filter, 1000.0, settings(0.0, 1, 1000.0, slope));
            assert!(
                (g + 3.0).abs() < 0.5,
                "slope {slope}: 1 kHz corner measured {g:.2} dB"
            );
        }
    }

    #[test]
    fn lo_cut_slopes_get_steeper_below_the_corner() {
        let mut gains = [0.0_f32; 3];
        for (slope, gain) in gains.iter_mut().enumerate() {
            let mut filter = Filtering::default();
            filter.set_sample_rate(SR);
            // One octave below the corner: ideally -12, -24, -36 dB.
            *gain = gain_db(&mut filter, 50.0, settings(100.0, slope, 0.0, 1));
        }
        assert!(
            gains[0] > gains[1] && gains[1] > gains[2],
            "12/24/36 dB should fall in that order: {gains:?}"
        );
        assert!((gains[0] + 12.0).abs() < 1.5, "12 dB/oct: {:.2}", gains[0]);
        assert!((gains[1] + 24.0).abs() < 1.5, "24 dB/oct: {:.2}", gains[1]);
        assert!((gains[2] + 36.0).abs() < 1.5, "36 dB/oct: {:.2}", gains[2]);
    }

    #[test]
    fn hi_cut_slopes_get_steeper_above_the_corner() {
        let mut gains = [0.0_f32; 3];
        for (slope, gain) in gains.iter_mut().enumerate() {
            let mut filter = Filtering::default();
            filter.set_sample_rate(SR);
            // One octave above the corner: ideally -12, -24, -36 dB.
            *gain = gain_db(&mut filter, 2000.0, settings(0.0, 1, 1000.0, slope));
        }
        assert!(
            gains[0] > gains[1] && gains[1] > gains[2],
            "12/24/36 dB should fall in that order: {gains:?}"
        );
        assert!((gains[0] + 12.0).abs() < 1.5, "12 dB/oct: {:.2}", gains[0]);
        assert!((gains[1] + 24.0).abs() < 1.5, "24 dB/oct: {:.2}", gains[1]);
        assert!((gains[2] + 36.0).abs() < 1.5, "36 dB/oct: {:.2}", gains[2]);
    }

    #[test]
    fn off_positions_are_transparent() {
        let mut filter = Filtering::default();
        filter.set_sample_rate(SR);
        for freq in [20.0, 100.0, 1000.0, 10000.0] {
            let g = gain_db(&mut filter, freq, settings(0.0, 2, 0.0, 2));
            assert!(g.abs() < 0.1, "off at {freq} Hz measured {g:.3} dB");
        }
    }

    #[test]
    fn slope_does_not_move_the_corner() {
        // The knob number is the -3 dB point for every lever position; the lever only changes
        // what happens past it.
        let mut filter = Filtering::default();
        filter.set_sample_rate(SR);
        let corner = gain_db(&mut filter, 2000.0, settings(0.0, 1, 2000.0, 2));
        assert!(
            (corner + 3.0).abs() < 0.5,
            "36 dB/oct corner: {corner:.2} dB"
        );
    }
}

/// Direct-form I biquad (f64) used for ITU-R BS.1770 K-weighting.
#[derive(Clone, Copy, Default)]
struct KBiquad {
    b0: f64,
    b1: f64,
    b2: f64,
    a1: f64,
    a2: f64,
    x1: f64,
    x2: f64,
    y1: f64,
    y2: f64,
}

impl KBiquad {
    #[inline]
    fn process(&mut self, x: f64) -> f64 {
        let y = self.b0 * x + self.b1 * self.x1 + self.b2 * self.x2
            - self.a1 * self.y1
            - self.a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = x;
        self.y2 = self.y1;
        self.y1 = y;
        y
    }

    fn clear(&mut self) {
        self.x1 = 0.0;
        self.x2 = 0.0;
        self.y1 = 0.0;
        self.y2 = 0.0;
    }
}

const LUFS_BLOCK_COUNT: usize = 30; // 30 x 100 ms = 3 s short-term window
const LUFS_HIST_BINS: usize = 81; // 1 LU bins, -70 .. +10 LUFS

/// Final master output trim + BS.1770 loudness (momentary 400 ms, short-term 3 s, gated integrated) + peak.
pub struct FinalOutputStage {
    sample_rate: f32,
    shelf_l: KBiquad,
    shelf_r: KBiquad,
    hp_l: KBiquad,
    hp_r: KBiquad,
    block_len: usize,
    block_pos: usize,
    block_sum: f64,
    blocks: [f64; LUFS_BLOCK_COUNT],
    block_head: usize,
    blocks_filled: usize,
    hist_energy: [f64; LUFS_HIST_BINS],
    hist_count: [u32; LUFS_HIST_BINS],
    tp_hold: f32,
    pub lufs_short: f32,
    pub lufs_momentary: f32,
    pub lufs_integrated: f32,
    pub true_peak_db: f32,
}

impl Default for FinalOutputStage {
    fn default() -> Self {
        let mut s = Self {
            sample_rate: 48000.0,
            shelf_l: KBiquad::default(),
            shelf_r: KBiquad::default(),
            hp_l: KBiquad::default(),
            hp_r: KBiquad::default(),
            block_len: 4800,
            block_pos: 0,
            block_sum: 0.0,
            blocks: [0.0; LUFS_BLOCK_COUNT],
            block_head: 0,
            blocks_filled: 0,
            hist_energy: [0.0; LUFS_HIST_BINS],
            hist_count: [0; LUFS_HIST_BINS],
            tp_hold: 0.001,
            lufs_short: -60.0,
            lufs_momentary: -60.0,
            lufs_integrated: -60.0,
            true_peak_db: -60.0,
        };
        s.set_sample_rate(48000.0);
        s
    }
}

#[inline]
fn energy_to_lufs(e: f64) -> f32 {
    (-0.691 + 10.0 * e.max(1.0e-12).log10()).clamp(-60.0, 6.0) as f32
}

impl FinalOutputStage {
    pub fn set_sample_rate(&mut self, sr: f32) {
        let fs = (sr as f64).max(8000.0);
        self.sample_rate = fs as f32;
        self.block_len = ((fs * 0.1).round() as usize).max(1);

        // Stage 1: high-shelf (+4 dB @ ~1.68 kHz), Stage 2: RLB high-pass (~38 Hz)
        let pi = std::f64::consts::PI;
        let (f0, g, q) = (
            1681.974450955533_f64,
            3.999843853973347_f64,
            0.7071752369554196_f64,
        );
        let k = (pi * f0 / fs).tan();
        let vh = 10.0_f64.powf(g / 20.0);
        let vb = vh.powf(0.4996667741545416);
        let a0 = 1.0 + k / q + k * k;
        let shelf = KBiquad {
            b0: (vh + vb * k / q + k * k) / a0,
            b1: 2.0 * (k * k - vh) / a0,
            b2: (vh - vb * k / q + k * k) / a0,
            a1: 2.0 * (k * k - 1.0) / a0,
            a2: (1.0 - k / q + k * k) / a0,
            ..KBiquad::default()
        };

        let (f1, q1) = (38.13547087602444_f64, 0.5003270373238773_f64);
        let k1 = (pi * f1 / fs).tan();
        let a0h = 1.0 + k1 / q1 + k1 * k1;
        let hp = KBiquad {
            b0: 1.0,
            b1: -2.0,
            b2: 1.0,
            a1: 2.0 * (k1 * k1 - 1.0) / a0h,
            a2: (1.0 - k1 / q1 + k1 * k1) / a0h,
            ..KBiquad::default()
        };

        self.shelf_l = shelf;
        self.shelf_r = shelf;
        self.hp_l = hp;
        self.hp_r = hp;
        self.reset();
    }

    /// Restart the gated integrated measurement only (momentary / short-term keep running).
    pub fn reset_integrated(&mut self) {
        self.hist_energy = [0.0; LUFS_HIST_BINS];
        self.hist_count = [0; LUFS_HIST_BINS];
        self.lufs_integrated = -60.0;
    }

    pub fn reset(&mut self) {
        self.shelf_l.clear();
        self.shelf_r.clear();
        self.hp_l.clear();
        self.hp_r.clear();
        self.block_pos = 0;
        self.block_sum = 0.0;
        self.blocks = [0.0; LUFS_BLOCK_COUNT];
        self.block_head = 0;
        self.blocks_filled = 0;
        self.tp_hold = 0.001;
        self.lufs_short = -60.0;
        self.lufs_momentary = -60.0;
        self.true_peak_db = -60.0;
        self.reset_integrated();
    }

    fn mean_last_blocks(&self, n: usize) -> f64 {
        let n = n.min(self.blocks_filled);
        if n == 0 {
            return 0.0;
        }
        let mut acc = 0.0;
        for i in 0..n {
            let idx = (self.block_head + LUFS_BLOCK_COUNT - 1 - i) % LUFS_BLOCK_COUNT;
            acc += self.blocks[idx];
        }
        acc / n as f64
    }

    fn finish_block(&mut self) {
        let block_e = self.block_sum / self.block_len as f64;
        self.block_sum = 0.0;
        self.block_pos = 0;
        self.blocks[self.block_head] = block_e;
        self.block_head = (self.block_head + 1) % LUFS_BLOCK_COUNT;
        self.blocks_filled = (self.blocks_filled + 1).min(LUFS_BLOCK_COUNT);

        let mom_e = self.mean_last_blocks(4);
        let short_e = self.mean_last_blocks(LUFS_BLOCK_COUNT);
        self.lufs_momentary = energy_to_lufs(mom_e);
        self.lufs_short = energy_to_lufs(short_e);

        // Integrated: 400 ms windows @ 75 % overlap, absolute gate -70 LUFS, relative gate -10 LU
        if self.blocks_filled >= 4 {
            let mom_l = -0.691 + 10.0 * mom_e.max(1.0e-12).log10();
            if mom_l > -70.0 {
                let bin = ((mom_l + 70.0).floor().max(0.0) as usize).min(LUFS_HIST_BINS - 1);
                self.hist_energy[bin] += mom_e;
                self.hist_count[bin] += 1;
            }
            let (mut tot_e, mut tot_n) = (0.0_f64, 0_u32);
            for i in 0..LUFS_HIST_BINS {
                tot_e += self.hist_energy[i];
                tot_n += self.hist_count[i];
            }
            if tot_n > 0 {
                let rel_thr = -0.691 + 10.0 * (tot_e / tot_n as f64).max(1.0e-12).log10() - 10.0;
                let (mut g_e, mut g_n) = (0.0_f64, 0_u32);
                for i in 0..LUFS_HIST_BINS {
                    if (i as f64 - 70.0) + 1.0 > rel_thr {
                        g_e += self.hist_energy[i];
                        g_n += self.hist_count[i];
                    }
                }
                if g_n > 0 {
                    self.lufs_integrated = energy_to_lufs(g_e / g_n as f64);
                }
            }
        }
    }

    pub fn process(&mut self, frame: StereoFrame, trim_db: f32) -> StereoFrame {
        let gain = db_to_gain(trim_db);
        let out_l = frame.left * gain;
        let out_r = frame.right * gain;

        // K-weighted power (sum of channel mean-squares, per BS.1770)
        let kl = self.hp_l.process(self.shelf_l.process(out_l as f64));
        let kr = self.hp_r.process(self.shelf_r.process(out_r as f64));
        self.block_sum += kl * kl + kr * kr;
        self.block_pos += 1;
        if self.block_pos >= self.block_len {
            self.finish_block();
        }

        let pk = out_l.abs().max(out_r.abs());
        let tp_release = (-1.0 / (self.sample_rate * 1.5)).exp();
        self.tp_hold = if pk > self.tp_hold {
            pk
        } else {
            self.tp_hold * tp_release
        };
        self.true_peak_db = gain_to_db(self.tp_hold).clamp(-60.0, 6.0);

        StereoFrame::new(out_l, out_r)
    }
}
