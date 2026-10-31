use std::sync::Arc;

use nih_plug::prelude::Buffer;

use crate::parameters::{CompModel, EqModel, MasteraidParams};

use super::clipper::{Clipper, ClipperConfig};
use super::coloration_auto::ColorationAuto;
use super::comp_fair::{CompConfig, CompFair};
use super::comp_vmu::CompVMu;
use super::eq_mass::{EqMass, MassBandConfig};
use super::eq_ptek::{EqPtek, PtekConfig};
use super::io::{FilterSettings, Filtering, FinalOutputStage, InputStage};
use super::limiter::{Limiter, LimiterConfig};
use super::output_stage::OutputStage;
use super::shared::{ConsoleMeters, StereoFrame};
use super::tape::{Tape, TapeConfig};

pub struct MasteringChain {
    input: InputStage,
    filtering: Filtering,
    eq: EqPtek,
    eq_mass: EqMass,
    eq_output: OutputStage,
    eq_col_auto: ColorationAuto,
    compressor: CompFair,
    comp_vmu: CompVMu,
    compressor_output: OutputStage,
    comp_col_auto: ColorationAuto,
    clipper: Clipper,
    tape: Tape,
    limiter: Limiter,
    output: FinalOutputStage,
    sample_rate: f32,
    pub meters: Arc<ConsoleMeters>,
}

impl Default for MasteringChain {
    fn default() -> Self {
        Self {
            input: InputStage::default(),
            filtering: Filtering::default(),
            eq: EqPtek::default(),
            eq_mass: EqMass::default(),
            eq_output: OutputStage::default(),
            eq_col_auto: ColorationAuto::default(),
            compressor: CompFair::default(),
            comp_vmu: CompVMu::default(),
            compressor_output: OutputStage::default(),
            comp_col_auto: ColorationAuto::default(),
            clipper: Clipper::default(),
            tape: Tape::default(),
            limiter: Limiter::default(),
            output: FinalOutputStage::default(),
            sample_rate: 48000.0,
            meters: Arc::new(ConsoleMeters::default()),
        }
    }
}

impl MasteringChain {
    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.input.set_sample_rate(sample_rate);
        self.output.set_sample_rate(sample_rate);
        self.filtering.set_sample_rate(sample_rate);
        self.eq.set_sample_rate(sample_rate);
        self.eq_mass.set_sample_rate(sample_rate);
        self.eq_output.set_sample_rate(sample_rate);
        self.compressor.set_sample_rate(sample_rate);
        self.comp_vmu.set_sample_rate(sample_rate);
        self.compressor_output.set_sample_rate(sample_rate);
        self.tape.set_sample_rate(sample_rate);
        self.clipper.set_sample_rate(sample_rate);
        self.limiter.set_sample_rate(sample_rate);
    }

    pub fn reset(&mut self) {
        self.input.reset();
        self.filtering.reset();
        self.eq.reset();
        self.eq_mass.reset();
        self.eq_output.reset();
        self.compressor.reset();
        self.comp_vmu.reset();
        self.compressor_output.reset();
        self.eq_col_auto.reset_dsp();
        self.comp_col_auto.reset_dsp();
        self.clipper.reset();
        self.tape.reset();
        self.limiter.reset();
        self.output.reset();
    }

    pub fn process(&mut self, buffer: &mut Buffer, params: &MasteraidParams) {
        if !params.master_power.value() {
            return;
        }

        if self
            .meters
            .reset_integrated
            .swap(false, std::sync::atomic::Ordering::Relaxed)
        {
            self.output.reset_integrated();
        }
        let eq_bands = [
            MassBandConfig {
                bypass: params.mass_low_bypass.value(),
                freq_idx: params.mass_low_freq.value() as usize,
                gain_db: params.mass_low_gain.value(),
                q: params.mass_low_q.value(),
                is_shelf: params.mass_low_shelf.value(),
            },
            MassBandConfig {
                bypass: params.mass_lmf_bypass.value(),
                freq_idx: params.mass_lmf_freq.value() as usize,
                gain_db: params.mass_lmf_gain.value(),
                q: params.mass_lmf_q.value(),
                is_shelf: params.mass_lmf_shelf.value(),
            },
            MassBandConfig {
                bypass: params.mass_hmf_bypass.value(),
                freq_idx: params.mass_hmf_freq.value() as usize,
                gain_db: params.mass_hmf_gain.value(),
                q: params.mass_hmf_q.value(),
                is_shelf: params.mass_hmf_shelf.value(),
            },
            MassBandConfig {
                bypass: params.mass_hf_bypass.value(),
                freq_idx: params.mass_hf_freq.value() as usize,
                gain_db: params.mass_hf_gain.value(),
                q: params.mass_hf_q.value(),
                is_shelf: params.mass_hf_shelf.value(),
            },
        ];

        let ptek_cfg = PtekConfig {
            low_boost: params.ptek_low_boost.value(),
            low_atten: params.ptek_low_atten.value(),
            low_freq_idx: params.ptek_low_freq.value().clamp(0, 5) as usize,
            high_boost: params.ptek_high_boost.value(),
            bandwidth: params.ptek_bandwidth.value(),
            high_freq_idx: params.ptek_high_freq.value().clamp(0, 6) as usize,
            high_atten: params.ptek_high_atten.value(),
            atten_freq_idx: params.ptek_atten_freq.value().clamp(0, 2) as usize,
        };

        let comp_cfg = CompConfig {
            threshold_db: params.comp_thresh.value(),
            // INPUT GAIN is the FAIR's drive control and only the Fair panel exposes
            // it. The V-MU has no such pot, so leaving it applied there would silently change
            // its gain staging when the user switches models.
            input_gain_db: if params.comp_model.value() == CompModel::Fair {
                params.comp_in_gain.value()
            } else {
                0.0
            },
            ratio: params.comp_ratio.value(),
            attack_ms: params.comp_attack.value(),
            release_ms: params.comp_release.value(),
            sc_hpf_hz: params.comp_sc_hpf_hz.value(),
            mix_pct: params.comp_mix.value(),
            makeup_db: params.comp_makeup.value(),
            auto_makeup: params.comp_auto_makeup.value(),
        };

        let clip_cfg = ClipperConfig {
            drive_db: params.clipper_drive.value(),
            ceiling_db: params.clipper_ceiling.value(),
            hard_shape: params.clipper_hard_shape.value(),
            oversample_4x: params.clipper_os_4x.value(),
            compensate: params.clipper_comp.value(),
        };

        let tape_cfg = TapeConfig {
            is_30_ips: params.tape_speed_30ips.value(),
            is_gp9: params.tape_formula_gp9.value(),
            drive_db: params.tape_drive.value(),
            bias_db: params.tape_bias.value(),
            head_bump_db: params.tape_headbump.value(),
            bump_freq_hz: params.tape_bump_freq.value(),
            flutter_pct: params.tape_flutter.value(),
            compensate: params.tape_comp.value(),
        };

        let lim_cfg = LimiterConfig {
            gain_db: params.limiter_gain.value(),
            ceiling_db: params.limiter_ceiling.value(),
            lookahead_ms: params.limiter_lookahead_ms.value(),
            release_ms: params.limiter_release_ms.value(),
            lift: params.limiter_lift.value(),
            auto_release: params.limiter_auto_release.value(),
            true_peak: params.limiter_true_peak.value(),
            punch: params.limiter_punch.value(),
            unity: params.limiter_unity.value(),
        };

        let eq_amp = params.eq_amp_type.value();
        let eq_trans = params.eq_trans_type.value();
        let eq_int = params.eq_intensity.value();

        let comp_amp = params.comp_amp_type.value();
        let comp_trans = params.comp_trans_type.value();
        let comp_int = params.comp_intensity.value();

        let eq_auto_on = params.eq_col_comp.value();
        let comp_auto_on = params.comp_col_comp.value();

        self.eq_output.set_compensation(eq_auto_on);
        self.compressor_output.set_compensation(comp_auto_on);

        // A new amplifier / transformer / sample rate selects a different frozen result. The
        // persisted blob is only read here, i.e. when the sensor actually changes, never per
        // block, and it never starts a measurement: model switches reuse what was learned before.
        if !self
            .eq_col_auto
            .key_matches(eq_amp, eq_trans, self.sample_rate)
        {
            if let Ok(saved) = params.eq_col_auto.lock() {
                self.eq_col_auto
                    .configure(eq_amp, eq_trans, self.sample_rate, &saved);
            }
        }
        if !self
            .comp_col_auto
            .key_matches(comp_amp, comp_trans, self.sample_rate)
        {
            if let Ok(saved) = params.comp_col_auto.lock() {
                self.comp_col_auto
                    .configure(comp_amp, comp_trans, self.sample_rate, &saved);
            }
        }

        // Debug menu override for the measurement length, in seconds (0 = default).
        let learn_seconds = crate::debug_calibration::calibration().auto_learn_s() as f64;
        self.eq_col_auto.set_learn_seconds(learn_seconds);
        self.comp_col_auto.set_learn_seconds(learn_seconds);

        self.eq_col_auto.set_enabled(eq_auto_on);
        self.comp_col_auto.set_enabled(comp_auto_on);

        if self
            .meters
            .eq_col_relearn
            .swap(false, std::sync::atomic::Ordering::Relaxed)
        {
            self.eq_col_auto.relearn();
        }
        if self
            .meters
            .comp_col_relearn
            .swap(false, std::sync::atomic::Ordering::Relaxed)
        {
            self.comp_col_auto.relearn();
        }

        if self
            .meters
            .eq_col_clear
            .swap(false, std::sync::atomic::Ordering::Relaxed)
        {
            self.eq_col_auto.clear_learned();
        }
        if self
            .meters
            .comp_col_clear
            .swap(false, std::sync::atomic::Ordering::Relaxed)
        {
            self.comp_col_auto.clear_learned();
        }

        // While a measurement is frozen, the learned residual takes over and the RMS rider stands
        // down. Until then the stage keeps its previous behaviour.
        match self.eq_col_auto.latched_trim_db(eq_int) {
            Some(trim) => {
                self.eq_output.set_auto_level_enabled(false);
                self.eq_output.set_residual_trim_db(trim);
            }
            None => {
                self.eq_output.set_auto_level_enabled(true);
                self.eq_output.set_residual_trim_db(0.0);
            }
        }
        match self.comp_col_auto.latched_trim_db(comp_int) {
            Some(trim) => {
                self.compressor_output.set_auto_level_enabled(false);
                self.compressor_output.set_residual_trim_db(trim);
            }
            None => {
                self.compressor_output.set_auto_level_enabled(true);
                self.compressor_output.set_residual_trim_db(0.0);
            }
        }

        let mut gr_l = 0.0_f32;
        let mut gr_r = 0.0_f32;
        let mut auto_makeup_db = 0.0_f32;

        for mut channel_samples in buffer.iter_samples() {
            if channel_samples.len() < 2 {
                continue;
            }

            let mut samples = channel_samples.iter_mut();
            let left_sample = samples.next().expect("checked stereo channel count");
            let right_sample = samples.next().expect("checked stereo channel count");

            let mut frame = StereoFrame::new(*left_sample, *right_sample);

            // 1. Input Stage (Trim, Polarity, Mono, Peak Metering)
            let in_byp = params.input_bypass.value();
            frame = self.input.process(
                frame,
                if in_byp { 0.0 } else { params.input_trim.value() },
                if in_byp { false } else { params.polarity.value() },
                if in_byp { false } else { params.input_mono.value() },
            );

            // 2. Filtering: Lo Cut -> Hi Cut -> Bass Mono
            if !in_byp {
                frame = self.filtering.process(
                    frame,
                    FilterSettings {
                        lo_cut_hz: params.filter_lo_cut_hz.value(),
                        lo_cut_slope: params.filter_lo_cut_slope.value().clamp(0, 2) as usize,
                        hi_cut_hz: params.filter_hi_cut_hz.value(),
                        hi_cut_slope: params.filter_hi_cut_slope.value().clamp(0, 2) as usize,
                        mono_hz: params.bass_mono_freq.value(),
                        mono_amount_pct: params.bass_mono_amount.value(),
                        mono_slope: params.bass_mono_slope.value().clamp(0, 2) as usize,
                    },
                );
            }

            // 3. Mastering Parametric EQ + Independent EQ Output Stage Coloration
            if !params.eq_bypass.value() {
                frame = match params.eq_model.value() {
                    EqModel::Ptek => self.eq.process_with_config(frame, &ptek_cfg),
                    EqModel::Mass => self.eq_mass.process_with_bands(frame, &eq_bands),
                };
                let coloration_in = frame;
                frame = self
                    .eq_output
                    .process_with_params(frame, eq_amp, eq_trans, eq_int);
                self.eq_col_auto.observe(coloration_in);
            }

            // 4. Mastering Dynamics + Independent Compressor Output Stage Coloration
            if !params.comp_bypass.value() {
                frame = match params.comp_model.value() {
                    CompModel::Fair => {
                        let out = self.compressor.process_with_config(frame, &comp_cfg);
                        gr_l = self.compressor.gr_db_l;
                        gr_r = self.compressor.gr_db_r;
                        auto_makeup_db = self.compressor.auto_makeup_db;
                        out
                    }
                    CompModel::VMu => {
                        let out = self.comp_vmu.process_with_config(frame, &comp_cfg);
                        gr_l = self.comp_vmu.gr_db_l;
                        gr_r = self.comp_vmu.gr_db_r;
                        auto_makeup_db = self.comp_vmu.auto_makeup_db;
                        out
                    }
                };
                let coloration_in = frame;
                frame = self
                    .compressor_output
                    .process_with_params(frame, comp_amp, comp_trans, comp_int);
                self.comp_col_auto.observe(coloration_in);
            }

            // 5. Analog Tape Mastering (ATR-102 / ToTape8)
            if !params.tape_bypass.value() {
                frame = self.tape.process_with_config(frame, &tape_cfg);
            }

            // 6. Mastering Clipper (ClipOnly3 with 4x oversampling)
            if !params.clipper_bypass.value() {
                frame = self.clipper.process_with_config(frame, &clip_cfg);
            }

            // 7. Brickwall Limiter (BlockParty + ButterComp2 + ClipOnly3)
            if !params.limiter_bypass.value() {
                frame = self.limiter.process_with_config(frame, &lim_cfg);
            }

            // 8. Final Master Output Stage & LUFS / True Peak Metering
            frame = self.output.process(frame, params.output_trim.value());

            *left_sample = frame.left;
            *right_sample = frame.right;
        }

        // A finished measurement is frozen into the project state. The lock happens once, at the
        // moment the latch closes, not in the audio loop.
        if self.eq_col_auto.take_dirty() {
            let blob = self.eq_col_auto.snapshot_blob();
            if let Ok(mut slot) = params.eq_col_auto.lock() {
                *slot = blob;
            }
        }
        if self.comp_col_auto.take_dirty() {
            let blob = self.comp_col_auto.snapshot_blob();
            if let Ok(mut slot) = params.comp_col_auto.lock() {
                *slot = blob;
            }
        }

        // Publish real-time metering telemetry to lock-free atomics
        self.meters.input_peak_db_l.store(self.input.peak_db_l);
        self.meters.input_peak_db_r.store(self.input.peak_db_r);
        self.meters.input_rms_db_l.store(self.input.rms_db_l);
        self.meters.input_rms_db_r.store(self.input.rms_db_r);
        self.meters.comp_gr_db_l.store(gr_l);
        self.meters.comp_gr_db_r.store(gr_r);
        self.meters.comp_auto_makeup_db.store(auto_makeup_db);
        self.meters
            .eq_col_learning
            .store(self.eq_col_auto.progress());
        self.meters
            .comp_col_learning
            .store(self.comp_col_auto.progress());
        self.meters.output_lufs.store(self.output.lufs_short);
        self.meters
            .output_lufs_mom
            .store(self.output.lufs_momentary);
        self.meters
            .output_lufs_int
            .store(self.output.lufs_integrated);
        self.meters
            .output_true_peak_db
            .store(self.output.true_peak_db);
        let (clip_l, clip_r, clip_peak) = self.clipper.take_metrics();
        self.meters.clipper_clip_db_l.store(clip_l);
        self.meters.clipper_clip_db_r.store(clip_r);
        self.meters.clipper_clip_db.store(clip_peak.max(clip_l.max(clip_r)));
        let (lim_gr, lim_out_peak) = self.limiter.take_metrics();
        self.meters.limiter_gr_db.store(lim_gr);
        self.meters.limiter_out_peak_db.store(lim_out_peak);

        self.meters
            .process_counter
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_full_airwindows_mastering_chain_stages() {
        let mut chain = MasteringChain::default();
        chain.set_sample_rate(48000.0);

        for i in 0..2048 {
            let t = (i as f32) / 48000.0;
            let l = 0.5 * (2.0 * std::f32::consts::PI * 110.0 * t).sin()
                + 0.25 * (2.0 * std::f32::consts::PI * 2500.0 * t).sin();
            let r = 0.5 * (2.0 * std::f32::consts::PI * 110.0 * t).cos()
                + 0.25 * (2.0 * std::f32::consts::PI * 2500.0 * t).sin();

            let mut frame = StereoFrame::new(l, r);
            frame = chain.input.process(frame, 1.5, false, false);
            frame = chain.filtering.process(
                frame,
                FilterSettings {
                    lo_cut_hz: 30.0,
                    lo_cut_slope: 1,
                    hi_cut_hz: 18000.0,
                    hi_cut_slope: 0,
                    mono_hz: 80.0,
                    mono_amount_pct: 70.0,
                    mono_slope: 1,
                },
            );

            let bands = [
                MassBandConfig {
                    bypass: false,
                    freq_idx: 2,
                    gain_db: 1.5,
                    q: 1.5,
                    is_shelf: true,
                },
                MassBandConfig {
                    bypass: false,
                    freq_idx: 2,
                    gain_db: -2.0,
                    q: 1.5,
                    is_shelf: false,
                },
                MassBandConfig {
                    bypass: false,
                    freq_idx: 4,
                    gain_db: 1.5,
                    q: 1.5,
                    is_shelf: false,
                },
                MassBandConfig {
                    bypass: false,
                    freq_idx: 3,
                    gain_db: 2.0,
                    q: 1.5,
                    is_shelf: true,
                },
            ];
            frame = chain.eq_mass.process_with_bands(frame, &bands);
            frame = chain.eq.process_with_config(frame, &PtekConfig::default());
            // Test State 2 ("Smashed Glass" extreme harmonic drive) on EQ Output Stage
            frame = chain.eq_output.process_with_params(frame, 1, 2, 2);

            let comp_cfg = CompConfig {
                threshold_db: -4.0,
                input_gain_db: 0.0,
                ratio: 3.0,
                attack_ms: 1.0,
                release_ms: 500.0,
                sc_hpf_hz: 80.0,
                mix_pct: 100.0,
                makeup_db: 2.0,
                auto_makeup: false,
            };
            frame = chain.compressor.process_with_config(frame, &comp_cfg);
            frame = chain.comp_vmu.process_with_config(frame, &comp_cfg);
            frame = chain.compressor_output.process_with_params(frame, 3, 1, 1);

            let tape_cfg = TapeConfig {
                is_30_ips: true,
                is_gp9: false,
                drive_db: 2.0,
                bias_db: -2.0,
                head_bump_db: 4.0,
                bump_freq_hz: 80.0,
                flutter_pct: 15.0,
                compensate: true,
            };
            frame = chain.tape.process_with_config(frame, &tape_cfg);

            let clip_cfg = ClipperConfig {
                drive_db: 2.0,
                ceiling_db: -0.3,
                hard_shape: false,
                oversample_4x: true,
                compensate: true,
            };
            frame = chain.clipper.process_with_config(frame, &clip_cfg);

            let lim_cfg = LimiterConfig {
                gain_db: 3.0,
                ceiling_db: -0.1,
                lookahead_ms: 1.0,
                release_ms: 0.0,
                lift: 0.0,
                auto_release: true,
                true_peak: true,
                punch: false,
                unity: false,
            };
            frame = chain.limiter.process_with_config(frame, &lim_cfg);
            frame = chain.output.process(frame, 0.0);

            assert!(frame.left.is_finite() && frame.right.is_finite());
            assert!(frame.left.abs() <= 1.01 && frame.right.abs() <= 1.01);
        }
    }
}
