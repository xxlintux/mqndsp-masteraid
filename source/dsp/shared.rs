use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};

#[derive(Clone, Copy, Debug, Default)]
pub struct StereoFrame {
    pub left: f32,
    pub right: f32,
}

impl StereoFrame {
    #[inline]
    pub fn new(left: f32, right: f32) -> Self {
        Self { left, right }
    }
}

#[inline]
pub fn db_to_gain(db: f32) -> f32 {
    10.0_f32.powf(db / 20.0)
}

#[inline]
pub fn gain_to_db(gain: f32) -> f32 {
    20.0 * gain.max(1.0e-6).log10()
}

/// Lock-free atomic f32 for real-time audio-to-GUI metering (Input L/R, GR L/R, LUFS, True Peak).
#[derive(Debug)]
pub struct AtomicF32 {
    bits: AtomicU32,
}

impl Default for AtomicF32 {
    fn default() -> Self {
        Self::new(0.0)
    }
}

impl AtomicF32 {
    pub fn new(val: f32) -> Self {
        Self {
            bits: AtomicU32::new(val.to_bits()),
        }
    }

    #[inline]
    pub fn store(&self, val: f32) {
        self.bits.store(val.to_bits(), Ordering::Relaxed);
    }

    #[inline]
    pub fn load(&self) -> f32 {
        f32::from_bits(self.bits.load(Ordering::Relaxed))
    }
}

/// Shared real-time metering bridge between `MasteringChain` and the GUI / Live Viewer.
#[derive(Debug)]
pub struct ConsoleMeters {
    pub input_peak_db_l: AtomicF32,
    pub input_peak_db_r: AtomicF32,
    pub input_rms_db_l: AtomicF32,
    pub input_rms_db_r: AtomicF32,
    pub comp_gr_db_l: AtomicF32,
    pub comp_gr_db_r: AtomicF32,
    /// Peak amount of clipping currently occurring in the mastering clipper (0 dB when below ceiling).
    pub clipper_clip_db: AtomicF32,
    pub clipper_clip_db_l: AtomicF32,
    pub clipper_clip_db_r: AtomicF32,
    /// Instantaneous Gain Reduction applied by the brickwall limiter (0 dB when untouched).
    pub limiter_gr_db: AtomicF32,
    /// Output peak level exiting the brickwall limiter.
    pub limiter_out_peak_db: AtomicF32,
    /// Makeup the compressor's auto-makeup engine is currently applying (0 dB when disengaged).
    pub comp_auto_makeup_db: AtomicF32,
    pub output_lufs: AtomicF32,
    pub output_lufs_mom: AtomicF32,
    pub output_lufs_int: AtomicF32,
    pub output_true_peak_db: AtomicF32,
    /// Set by the GUI (Alt+Click on the LUFS display) to restart the integrated LUFS measurement.
    pub reset_integrated: AtomicBool,
    /// AUTO learn progress per coloration stage: 0 = idle, 0..1 = measuring, 1 = frozen.
    pub eq_col_learning: AtomicF32,
    pub comp_col_learning: AtomicF32,
    /// Set by the GUI (hold on AUTO) to make the stage measure its key again.
    pub eq_col_relearn: AtomicBool,
    pub comp_col_relearn: AtomicBool,
    /// Set by the GUI (double-click on AUTO) to drop the frozen result for the current key.
    pub eq_col_clear: AtomicBool,
    pub comp_col_clear: AtomicBool,
    /// Advanced once per audio block. The GUI uses it to tell a stopped transport (no blocks,
    /// frozen telemetry) from a genuinely steady signal.
    pub process_counter: AtomicU64,
}

impl Default for ConsoleMeters {
    fn default() -> Self {
        Self {
            input_peak_db_l: AtomicF32::new(-60.0),
            input_peak_db_r: AtomicF32::new(-60.0),
            input_rms_db_l: AtomicF32::new(-60.0),
            input_rms_db_r: AtomicF32::new(-60.0),
            comp_gr_db_l: AtomicF32::new(0.0),
            comp_gr_db_r: AtomicF32::new(0.0),
            clipper_clip_db: AtomicF32::new(0.0),
            clipper_clip_db_l: AtomicF32::new(0.0),
            clipper_clip_db_r: AtomicF32::new(0.0),
            limiter_gr_db: AtomicF32::new(0.0),
            limiter_out_peak_db: AtomicF32::new(-60.0),
            comp_auto_makeup_db: AtomicF32::new(0.0),
            output_lufs: AtomicF32::new(-60.0),
            output_lufs_mom: AtomicF32::new(-60.0),
            output_lufs_int: AtomicF32::new(-60.0),
            output_true_peak_db: AtomicF32::new(-60.0),
            reset_integrated: AtomicBool::new(false),
            eq_col_learning: AtomicF32::new(0.0),
            comp_col_learning: AtomicF32::new(0.0),
            eq_col_relearn: AtomicBool::new(false),
            comp_col_relearn: AtomicBool::new(false),
            eq_col_clear: AtomicBool::new(false),
            comp_col_clear: AtomicBool::new(false),
            process_counter: AtomicU64::new(0),
        }
    }
}

/// Airwindows-style xorshift PRNG + subnormal/denormal guard (`fpdL` / `fpdR`).
#[derive(Clone, Copy, Debug)]
pub struct AirwindowsDither {
    pub fpd_l: u32,
    pub fpd_r: u32,
}

impl Default for AirwindowsDither {
    fn default() -> Self {
        Self {
            fpd_l: 171_717_171,
            fpd_r: 987_654_321,
        }
    }
}

impl AirwindowsDither {
    #[inline]
    pub fn guard_denormals(&mut self, l: &mut f64, r: &mut f64) {
        if l.abs() < 1.18e-23 {
            self.fpd_l ^= self.fpd_l << 13;
            self.fpd_l ^= self.fpd_l >> 17;
            self.fpd_l ^= self.fpd_l << 5;
            *l = (self.fpd_l as f64) * 1.18e-17;
        }
        if r.abs() < 1.18e-23 {
            self.fpd_r ^= self.fpd_r << 13;
            self.fpd_r ^= self.fpd_r >> 17;
            self.fpd_r ^= self.fpd_r << 5;
            *r = (self.fpd_r as f64) * 1.18e-17;
        }
    }

    #[inline]
    pub fn next_unit_l(&mut self) -> f64 {
        self.fpd_l ^= self.fpd_l << 13;
        self.fpd_l ^= self.fpd_l >> 17;
        self.fpd_l ^= self.fpd_l << 5;
        (self.fpd_l as f64) / (u32::MAX as f64)
    }

    #[inline]
    pub fn next_unit_r(&mut self) -> f64 {
        self.fpd_r ^= self.fpd_r << 13;
        self.fpd_r ^= self.fpd_r >> 17;
        self.fpd_r ^= self.fpd_r << 5;
        (self.fpd_r as f64) / (u32::MAX as f64)
    }
}

/// Airwindows-style Direct-Form II Transposed Stereo Biquad with interleaved `flip` state
/// (adapted from `Baxandall2Proc.cpp`, `Coils2Proc.cpp`, and `ToTape6Proc.cpp`).
#[derive(Clone, Copy, Debug, Default)]
pub struct AirwindowsBiquad {
    pub b0: f64,
    pub b1: f64,
    pub b2: f64,
    pub a1: f64,
    pub a2: f64,
    // State A (when flip == true)
    pub s_al1: f64,
    pub s_al2: f64,
    pub s_ar1: f64,
    pub s_ar2: f64,
    // State B (when flip == false)
    pub s_bl1: f64,
    pub s_bl2: f64,
    pub s_br1: f64,
    pub s_br2: f64,
}

impl AirwindowsBiquad {
    pub fn reset(&mut self) {
        self.s_al1 = 0.0;
        self.s_al2 = 0.0;
        self.s_ar1 = 0.0;
        self.s_ar2 = 0.0;
        self.s_bl1 = 0.0;
        self.s_bl2 = 0.0;
        self.s_br1 = 0.0;
        self.s_br2 = 0.0;
    }

    pub fn set_lowpass(&mut self, freq_hz: f64, q: f64, sample_rate: f64) {
        let fc = (freq_hz / sample_rate).clamp(0.0001, 0.49);
        let k = (std::f64::consts::PI * fc).tan();
        let norm = 1.0 / (1.0 + k / q + k * k);
        self.b0 = k * k * norm;
        self.b1 = 2.0 * self.b0;
        self.b2 = self.b0;
        self.a1 = 2.0 * (k * k - 1.0) * norm;
        self.a2 = (1.0 - k / q + k * k) * norm;
    }

    pub fn set_highpass(&mut self, freq_hz: f64, q: f64, sample_rate: f64) {
        let fc = (freq_hz / sample_rate).clamp(0.0001, 0.49);
        let k = (std::f64::consts::PI * fc).tan();
        let norm = 1.0 / (1.0 + k / q + k * k);
        self.b0 = norm;
        self.b1 = -2.0 * self.b0;
        self.b2 = self.b0;
        self.a1 = 2.0 * (k * k - 1.0) * norm;
        self.a2 = (1.0 - k / q + k * k) * norm;
    }

    pub fn set_bandpass(&mut self, freq_hz: f64, q: f64, sample_rate: f64) {
        let fc = (freq_hz / sample_rate).clamp(0.0001, 0.49);
        let k = (std::f64::consts::PI * fc).tan();
        let norm = 1.0 / (1.0 + k / q + k * k);
        self.b0 = (k / q) * norm;
        self.b1 = 0.0;
        self.b2 = -self.b0;
        self.a1 = 2.0 * (k * k - 1.0) * norm;
        self.a2 = (1.0 - k / q + k * k) * norm;
    }

    pub fn set_peaking(&mut self, freq_hz: f64, q: f64, gain_db: f64, sample_rate: f64) {
        let fc = (freq_hz / sample_rate).clamp(0.0001, 0.49);
        let a = 10.0_f64.powf(gain_db / 40.0);
        let w0 = 2.0 * std::f64::consts::PI * fc;
        let alpha = w0.sin() / (2.0 * q.max(0.15));
        let cos_w0 = w0.cos();
        let a0 = 1.0 + alpha / a;
        self.b0 = (1.0 + alpha * a) / a0;
        self.b1 = (-2.0 * cos_w0) / a0;
        self.b2 = (1.0 - alpha * a) / a0;
        self.a1 = (-2.0 * cos_w0) / a0;
        self.a2 = (1.0 - alpha / a) / a0;
    }

    pub fn set_low_shelf(&mut self, freq_hz: f64, q: f64, gain_db: f64, sample_rate: f64) {
        let fc = (freq_hz / sample_rate).clamp(0.0001, 0.49);
        let a = 10.0_f64.powf(gain_db / 40.0);
        let w0 = 2.0 * std::f64::consts::PI * fc;
        let alpha = w0.sin() / (2.0 * q.max(0.15));
        let cos_w0 = w0.cos();
        let two_sqrt_a_alpha = 2.0 * a.sqrt() * alpha;
        let a0 = (a + 1.0) + (a - 1.0) * cos_w0 + two_sqrt_a_alpha;
        self.b0 = (a * ((a + 1.0) - (a - 1.0) * cos_w0 + two_sqrt_a_alpha)) / a0;
        self.b1 = (2.0 * a * ((a - 1.0) - (a + 1.0) * cos_w0)) / a0;
        self.b2 = (a * ((a + 1.0) - (a - 1.0) * cos_w0 - two_sqrt_a_alpha)) / a0;
        self.a1 = (-2.0 * ((a - 1.0) + (a + 1.0) * cos_w0)) / a0;
        self.a2 = ((a + 1.0) + (a - 1.0) * cos_w0 - two_sqrt_a_alpha) / a0;
    }

    pub fn set_high_shelf(&mut self, freq_hz: f64, q: f64, gain_db: f64, sample_rate: f64) {
        let fc = (freq_hz / sample_rate).clamp(0.0001, 0.49);
        let a = 10.0_f64.powf(gain_db / 40.0);
        let w0 = 2.0 * std::f64::consts::PI * fc;
        let alpha = w0.sin() / (2.0 * q.max(0.15));
        let cos_w0 = w0.cos();
        let two_sqrt_a_alpha = 2.0 * a.sqrt() * alpha;
        let a0 = (a + 1.0) - (a - 1.0) * cos_w0 + two_sqrt_a_alpha;
        self.b0 = (a * ((a + 1.0) + (a - 1.0) * cos_w0 + two_sqrt_a_alpha)) / a0;
        self.b1 = (-2.0 * a * ((a - 1.0) + (a + 1.0) * cos_w0)) / a0;
        self.b2 = (a * ((a + 1.0) + (a - 1.0) * cos_w0 - two_sqrt_a_alpha)) / a0;
        self.a1 = (2.0 * ((a - 1.0) - (a + 1.0) * cos_w0)) / a0;
        self.a2 = ((a + 1.0) - (a - 1.0) * cos_w0 - two_sqrt_a_alpha) / a0;
    }

    #[inline]
    pub fn process_single(&mut self, l: f64, r: f64) -> (f64, f64) {
        let out_l = l * self.b0 + self.s_al1;
        self.s_al1 = l * self.b1 - out_l * self.a1 + self.s_al2;
        self.s_al2 = l * self.b2 - out_l * self.a2;

        let out_r = r * self.b0 + self.s_ar1;
        self.s_ar1 = r * self.b1 - out_r * self.a1 + self.s_ar2;
        self.s_ar2 = r * self.b2 - out_r * self.a2;

        (out_l, out_r)
    }

    #[inline]
    pub fn process_interleaved(&mut self, l: f64, r: f64, flip: bool) -> (f64, f64) {
        if flip {
            let out_l = l * self.b0 + self.s_al1;
            self.s_al1 = l * self.b1 - out_l * self.a1 + self.s_al2;
            self.s_al2 = l * self.b2 - out_l * self.a2;

            let out_r = r * self.b0 + self.s_ar1;
            self.s_ar1 = r * self.b1 - out_r * self.a1 + self.s_ar2;
            self.s_ar2 = r * self.b2 - out_r * self.a2;
            (out_l, out_r)
        } else {
            let out_l = l * self.b0 + self.s_bl1;
            self.s_bl1 = l * self.b1 - out_l * self.a1 + self.s_bl2;
            self.s_bl2 = l * self.b2 - out_l * self.a2;

            let out_r = r * self.b0 + self.s_br1;
            self.s_br1 = r * self.b1 - out_r * self.a1 + self.s_br2;
            self.s_br2 = r * self.b2 - out_r * self.a2;
            (out_l, out_r)
        }
    }
}

/// Slow stereo-linked RMS matcher used by the coloration stages.
///
/// The calibration tables remove the bulk of a model's gain offset, but the exact amount depends
/// on programme material (and the transformers lose a little level that varies with level). This
/// trims the residual: a ~400 ms power detector compares the stage input with the stage output and
/// a ~150 ms smoother moves a correction that is clamped to +/-3 dB. It is deliberately far slower
/// than any musical envelope, so it can only act as gain staging, never as compression.
#[derive(Clone, Copy, Debug)]
pub struct AutoLevel {
    detector_coeff: f64,
    smooth_coeff: f64,
    in_pow: f64,
    out_pow: f64,
    gain: f64,
    counter: u32,
}

impl Default for AutoLevel {
    fn default() -> Self {
        Self {
            detector_coeff: 0.0001,
            smooth_coeff: 0.001,
            in_pow: 0.0,
            out_pow: 0.0,
            gain: 1.0,
            counter: 0,
        }
    }
}

impl AutoLevel {
    /// Correction is only computed once per this many samples to keep `sqrt` off the hot path.
    const BLOCK: u32 = 32;
    /// Never correct by more than this, so a wrong table can never become a gain swing.
    const MAX_DB: f64 = 3.0;
    /// Below this input power (~-70 dBFS) the detector holds instead of chasing noise.
    const GATE_POW: f64 = 1.0e-7;

    pub fn set_sample_rate(&mut self, sr: f32) {
        let sr = (sr as f64).max(22050.0);
        self.detector_coeff = 1.0 - (-1.0 / (sr * 0.40)).exp();
        self.smooth_coeff = 1.0 - (-(Self::BLOCK as f64) / (sr * 0.15)).exp();
    }

    pub fn reset(&mut self) {
        self.in_pow = 0.0;
        self.out_pow = 0.0;
        self.gain = 1.0;
        self.counter = 0;
    }

    /// Feed the stage input and the *untrimmed* stage output, and apply the returned gain to the
    /// output. Feedforward, so the correction cannot oscillate.
    pub fn process(&mut self, in_l: f64, in_r: f64, out_l: f64, out_r: f64) -> f64 {
        let in_inst = (in_l * in_l + in_r * in_r) * 0.5;
        let out_inst = (out_l * out_l + out_r * out_r) * 0.5;
        self.in_pow += (in_inst - self.in_pow) * self.detector_coeff;
        self.out_pow += (out_inst - self.out_pow) * self.detector_coeff;

        self.counter += 1;
        if self.counter < Self::BLOCK {
            return self.gain;
        }
        self.counter = 0;

        let target = if self.in_pow > Self::GATE_POW && self.out_pow > 1.0e-12 {
            let limit = db_to_gain(Self::MAX_DB as f32) as f64;
            (self.in_pow / self.out_pow)
                .sqrt()
                .clamp(1.0 / limit, limit)
        } else {
            1.0
        };
        self.gain += (target - self.gain) * self.smooth_coeff;
        self.gain
    }
}
