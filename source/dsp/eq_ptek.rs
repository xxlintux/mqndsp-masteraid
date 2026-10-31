//! Masteraid PTEK Program Equalizer (`source/dsp/eq_ptek.rs`)
//!
//! Models the real hardware control set (not the 4-band Mass layout):
//!   LOW:   frequency selector, BOOST 0..10, ATTEN 0..10 (shelves share one selector; the
//!          attenuation corner sits above the boost corner, producing the classic "Ptek trick" dip).
//!   HIGH:  BOOST 0..10 at a selectable frequency with BANDWIDTH 0..10 (sharp -> broad),
//!          ATTEN 0..10 with its own 5 / 10 / 20 kHz selector.
//!
//! The filters use the shared Direct-Form II biquad. Unlike the Airwindows `flip` topology, each
//! sample goes through the filter normally, so the selected 3 kHz position really is 3 kHz (the
//! alternating-state trick effectively halves the sample rate and doubles every corner).

use super::shared::{AirwindowsBiquad, AirwindowsDither, StereoFrame};

/// LOW selector: EQP-1A positions 20 / 30 / 60 / 100 Hz plus 40 / 150 Hz extras.
pub const PTEK_LOW_FREQS: [f64; 6] = [20.0, 30.0, 40.0, 60.0, 100.0, 150.0];
/// HIGH BOOST selector: 3 / 4 / 5 / 8 / 10 / 12 / 16 kHz (all EQP-1A positions).
pub const PTEK_HIGH_FREQS: [f64; 7] = [3000.0, 4000.0, 5000.0, 8000.0, 10000.0, 12000.0, 16000.0];
/// HIGH ATTEN selector: 5 / 10 / 20 kHz.
pub const PTEK_ATTEN_FREQS: [f64; 3] = [5000.0, 10000.0, 20000.0];

/// Maximum curve depths at knob position 10 (EQP-1A specification).
const LOW_BOOST_MAX_DB: f64 = 13.5;
const LOW_ATTEN_MAX_DB: f64 = 17.5;
const HIGH_BOOST_MAX_DB: f64 = 18.0;
const HIGH_ATTEN_MAX_DB: f64 = 16.0;

/// One of the four EQP-1A gain stages. The hardware labels these knobs 0..10 (a relative
/// scale, not dB); this is the single place that turns that position into the depth the
/// filter actually applies, so the DSP and the GUI readout can never drift apart.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PtekBand {
    LowBoost,
    LowAtten,
    HighBoost,
    HighAtten,
}

impl PtekBand {
    /// Curve depth reached at knob position 10.
    pub const fn max_db(self) -> f64 {
        match self {
            Self::LowBoost => LOW_BOOST_MAX_DB,
            Self::LowAtten => LOW_ATTEN_MAX_DB,
            Self::HighBoost => HIGH_BOOST_MAX_DB,
            Self::HighAtten => HIGH_ATTEN_MAX_DB,
        }
    }

    /// Maps the hardware's 0..10 knob position to the applied gain in dB. Boost bands are
    /// positive, attenuation bands are negative, matching the sign the filters receive.
    pub fn knob_to_db(self, knob: f32) -> f32 {
        let depth = (knob as f64).clamp(0.0, 10.0) / 10.0 * self.max_db();
        match self {
            Self::LowBoost | Self::HighBoost => depth as f32,
            Self::LowAtten | Self::HighAtten => -depth as f32,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PtekConfig {
    pub low_boost: f32,
    pub low_atten: f32,
    pub low_freq_idx: usize,
    pub high_boost: f32,
    pub bandwidth: f32,
    pub high_freq_idx: usize,
    pub high_atten: f32,
    pub atten_freq_idx: usize,
}

impl Default for PtekConfig {
    fn default() -> Self {
        Self {
            low_boost: 0.0,
            low_atten: 0.0,
            low_freq_idx: 3,
            high_boost: 0.0,
            bandwidth: 5.0,
            high_freq_idx: 4,
            high_atten: 0.0,
            atten_freq_idx: 1,
        }
    }
}

pub struct EqPtek {
    sample_rate: f64,
    dither: AirwindowsDither,
    low_boost: AirwindowsBiquad,
    low_atten: AirwindowsBiquad,
    high_boost: AirwindowsBiquad,
    high_atten: AirwindowsBiquad,
    last_cfg: Option<PtekConfig>,
}

impl Default for EqPtek {
    fn default() -> Self {
        Self {
            sample_rate: 48000.0,
            dither: AirwindowsDither::default(),
            low_boost: AirwindowsBiquad::default(),
            low_atten: AirwindowsBiquad::default(),
            high_boost: AirwindowsBiquad::default(),
            high_atten: AirwindowsBiquad::default(),
            last_cfg: None,
        }
    }
}

impl EqPtek {
    pub fn set_sample_rate(&mut self, sr: f32) {
        let new_sr = (sr as f64).max(22050.0);
        if (new_sr - self.sample_rate).abs() > 1.0 {
            self.sample_rate = new_sr;
            self.last_cfg = None;
        }
    }

    pub fn reset(&mut self) {
        self.low_boost.reset();
        self.low_atten.reset();
        self.high_boost.reset();
        self.high_atten.reset();
    }

    fn update_filters(&mut self, cfg: &PtekConfig) {
        if self.last_cfg == Some(*cfg) {
            return;
        }
        self.last_cfg = Some(*cfg);

        let sr = self.sample_rate;
        let nyq_guard = sr * 0.45;

        let low_f = PTEK_LOW_FREQS[cfg.low_freq_idx.min(5)];
        let low_boost_db = PtekBand::LowBoost.knob_to_db(cfg.low_boost) as f64;
        let low_atten_db = PtekBand::LowAtten.knob_to_db(cfg.low_atten) as f64;
        // Boost shelf at the selected frequency; attenuation shelf corner sits higher, so boosting
        // and attenuating together yields the characteristic bump + dip just above the selector.
        self.low_boost.set_low_shelf(low_f, 0.55, low_boost_db, sr);
        self.low_atten
            .set_low_shelf((low_f * 1.9).min(nyq_guard), 0.45, low_atten_db, sr);

        let high_f = PTEK_HIGH_FREQS[cfg.high_freq_idx.min(6)].min(nyq_guard);
        let high_boost_db = PtekBand::HighBoost.knob_to_db(cfg.high_boost) as f64;
        // BANDWIDTH 0 = sharp (Q 2.8) .. 10 = broad (Q 0.35), exponential sweep
        let bw = (cfg.bandwidth as f64).clamp(0.0, 10.0) / 10.0;
        let q = 2.8 * (0.35_f64 / 2.8).powf(bw);
        self.high_boost.set_peaking(high_f, q, high_boost_db, sr);

        let atten_f = PTEK_ATTEN_FREQS[cfg.atten_freq_idx.min(2)].min(nyq_guard);
        let high_atten_db = PtekBand::HighAtten.knob_to_db(cfg.high_atten) as f64;
        self.high_atten
            .set_high_shelf(atten_f, 0.45, high_atten_db, sr);
    }

    pub fn process_with_config(&mut self, frame: StereoFrame, cfg: &PtekConfig) -> StereoFrame {
        let mut l = frame.left as f64;
        let mut r = frame.right as f64;
        self.dither.guard_denormals(&mut l, &mut r);

        self.update_filters(cfg);

        if cfg.low_boost > 0.01 {
            let (a, b) = self.low_boost.process_single(l, r);
            l = a;
            r = b;
        }
        if cfg.low_atten > 0.01 {
            let (a, b) = self.low_atten.process_single(l, r);
            l = a;
            r = b;
        }
        if cfg.high_boost > 0.01 {
            let (a, b) = self.high_boost.process_single(l, r);
            l = a;
            r = b;
        }
        if cfg.high_atten > 0.01 {
            let (a, b) = self.high_atten.process_single(l, r);
            l = a;
            r = b;
        }

        StereoFrame::new(l as f32, r as f32)
    }

    pub fn process(&mut self, frame: StereoFrame) -> StereoFrame {
        frame
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f64 = 48_000.0;

    /// RMS of a mono sine after the EQ has settled (the first 8192 samples are discarded).
    fn rms_at(cfg: PtekConfig, freq_hz: f64) -> f64 {
        let mut eq = EqPtek::default();
        eq.set_sample_rate(SR as f32);

        let warmup = 8192usize;
        let measured = 24_000usize;
        let mut acc = 0.0;
        for i in 0..(warmup + measured) {
            let x = (2.0 * std::f64::consts::PI * freq_hz * i as f64 / SR).sin() * 0.1;
            let out = eq.process_with_config(StereoFrame::new(x as f32, x as f32), &cfg);
            if i >= warmup {
                acc += (out.left as f64) * (out.left as f64);
            }
        }
        (acc / measured as f64).sqrt()
    }

    fn flat_rms(freq_hz: f64) -> f64 {
        rms_at(PtekConfig::default(), freq_hz)
    }

    #[test]
    fn flat_setting_passes_signal_unchanged() {
        for f in [30.0, 100.0, 1000.0, 10_000.0] {
            let flat = flat_rms(f);
            let expected = 0.1 / 2.0_f64.sqrt();
            assert!(
                (flat - expected).abs() < 1.0e-4,
                "flat EQ altered {f} Hz: {flat} vs {expected}"
            );
        }
    }

    #[test]
    fn all_four_ptek_circuits_are_audible() {
        // LOW BOOST at 60 Hz.
        let low_boost = PtekConfig {
            low_boost: 10.0,
            low_freq_idx: 3,
            ..PtekConfig::default()
        };
        assert!(
            rms_at(low_boost, 30.0) > flat_rms(30.0) * 2.0,
            "low boost did not lift the low band"
        );

        // LOW ATTEN at 60 Hz (corner sits above the boost corner).
        let low_atten = PtekConfig {
            low_atten: 10.0,
            low_freq_idx: 3,
            ..PtekConfig::default()
        };
        assert!(
            rms_at(low_atten, 200.0) < flat_rms(200.0) * 0.7,
            "low attenuation did not cut above the selected corner"
        );

        // HIGH BOOST at 10 kHz.
        let high_boost = PtekConfig {
            high_boost: 10.0,
            high_freq_idx: 4,
            ..PtekConfig::default()
        };
        assert!(
            rms_at(high_boost, 10_000.0) > flat_rms(10_000.0) * 2.5,
            "high boost did not lift the selected band"
        );

        // HIGH ATTEN at 10 kHz.
        let high_atten = PtekConfig {
            high_atten: 10.0,
            atten_freq_idx: 1,
            ..PtekConfig::default()
        };
        assert!(
            rms_at(high_atten, 15_000.0) < flat_rms(15_000.0) * 0.7,
            "high attenuation did not cut the top band"
        );
    }

    #[test]
    fn knob_position_maps_to_the_documented_depth() {
        // Position 10 is the hardware's maximum depth; attenuation reads as a negative gain.
        assert_eq!(PtekBand::LowBoost.knob_to_db(10.0), 13.5);
        assert_eq!(PtekBand::LowAtten.knob_to_db(10.0), -17.5);
        assert_eq!(PtekBand::HighBoost.knob_to_db(10.0), 18.0);
        assert_eq!(PtekBand::HighAtten.knob_to_db(10.0), -16.0);

        // Flat at 0, linear in between, and clamped outside the hardware range.
        for band in [
            PtekBand::LowBoost,
            PtekBand::LowAtten,
            PtekBand::HighBoost,
            PtekBand::HighAtten,
        ] {
            assert_eq!(band.knob_to_db(0.0), 0.0);
            assert!((band.knob_to_db(5.0).abs() - band.max_db() as f32 / 2.0).abs() < 1.0e-6);
            assert_eq!(band.knob_to_db(-1.0), 0.0);
            assert!((band.knob_to_db(99.0).abs() - band.max_db() as f32).abs() < 1.0e-6);
        }
    }

    #[test]
    fn high_boost_selector_moves_the_peak() {
        let at_3k = PtekConfig {
            high_boost: 10.0,
            high_freq_idx: 0,
            ..PtekConfig::default()
        };
        let at_10k = PtekConfig {
            high_boost: 10.0,
            high_freq_idx: 4,
            ..PtekConfig::default()
        };

        assert!(
            rms_at(at_3k, 3000.0) > flat_rms(3000.0) * 2.5,
            "3 kHz position did not boost 3 kHz"
        );
        assert!(
            rms_at(at_10k, 10_000.0) > flat_rms(10_000.0) * 2.5,
            "10 kHz position did not boost 10 kHz"
        );
        // The 3 kHz setting should not produce the 10 kHz setting's boost at 10 kHz.
        assert!(
            rms_at(at_3k, 10_000.0) < rms_at(at_10k, 10_000.0),
            "frequency selector is not reaching the DSP"
        );
    }
}
