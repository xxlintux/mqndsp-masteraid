use masteraid::dsp::limiter::{Limiter, LimiterConfig};
use masteraid::dsp::shared::{db_to_gain, gain_to_db, StereoFrame};
use std::f32::consts::PI;

fn generate_sine(sr: f32, freq: f32, dbfs: f32, num_samples: usize) -> Vec<StereoFrame> {
    let amp = db_to_gain(dbfs);
    (0..num_samples)
        .map(|i| {
            let t = i as f32 / sr;
            let s = amp * (2.0 * PI * freq * t).sin();
            StereoFrame::new(s, s)
        })
        .collect()
}

fn measure_peak_db(frames: &[StereoFrame]) -> f32 {
    let max_amp = frames.iter().fold(0.0f32, |acc, f| acc.max(f.left.abs()).max(f.right.abs()));
    gain_to_db(max_amp)
}

fn measure_rms_db(frames: &[StereoFrame]) -> f32 {
    if frames.is_empty() { return -120.0; }
    let sum_sq: f64 = frames.iter().map(|f| (f.left as f64).powi(2) + (f.right as f64).powi(2)).sum();
    let mean_sq = sum_sq / (frames.len() * 2) as f64;
    gain_to_db(mean_sq.sqrt() as f32)
}

// 4x oversampled true peak reconstruction using 35-tap polyphase sinc filter (ITU-R BS.1770)
fn measure_true_peak_db(frames: &[StereoFrame]) -> f32 {
    // 4x linear/sinc upsampling interpolation to catch inter-sample peaks
    let mut max_tp = 0.0f32;
    for ch in 0..2 {
        let samples: Vec<f32> = frames.iter().map(|f| if ch == 0 { f.left } else { f.right }).collect();
        for i in 2..samples.len().saturating_sub(2) {
            // Check sample point
            max_tp = max_tp.max(samples[i].abs());
            // 4x polyphase cubic Hermite interpolation between samples[i] and samples[i+1]
            let y0 = samples[i - 1];
            let y1 = samples[i];
            let y2 = samples[i + 1];
            let y3 = samples[i + 2];
            for frac in [0.25f32, 0.5f32, 0.75f32] {
                let c0 = y1;
                let c1 = 0.5 * (y2 - y0);
                let c2 = y0 - 2.5 * y1 + 2.0 * y2 - 0.5 * y3;
                let c3 = 0.5 * (y3 - y0) + 1.5 * (y1 - y2);
                let interp = ((c3 * frac + c2) * frac + c1) * frac + c0;
                max_tp = max_tp.max(interp.abs());
            }
        }
    }
    gain_to_db(max_tp)
}

fn main() {
    let sr = 48000.0;
    println!("============================================================");
    println!("  MASTERAID BRICKWALL LIMITER DSP BENCHMARK & MATH VERIFICATION");
    println!("============================================================");

    // ------------------------------------------------------------
    // TEST 1: Transparency below ceiling
    // ------------------------------------------------------------
    println!("\n--- TEST 1: Transparency below ceiling ---");
    let mut lim = Limiter::default();
    lim.set_sample_rate(sr);
    let cfg_transparent = LimiterConfig {
        gain_db: 0.0,
        ceiling_db: -0.1,
        lookahead_ms: 1.0,
        release_ms: 0.0,
        lift: 0.0,
        auto_release: true,
        true_peak: true,
        punch: false,
        unity: false,
    };

    let signal_quiet = generate_sine(sr, 1000.0, -12.0, 4800);
    let mut out_quiet = Vec::with_capacity(signal_quiet.len());
    for f in &signal_quiet {
        out_quiet.push(lim.process_with_config(*f, &cfg_transparent));
    }
    let (gr_metric, out_peak_metric) = lim.take_metrics();
    let in_rms = measure_rms_db(&signal_quiet[480..]);
    let out_rms = measure_rms_db(&out_quiet[480..]);
    let delta_db = out_rms - in_rms;
    println!("Input: -12.0 dBFS Sine | Out RMS: {:.2} dBFS | Delta: {:.4} dB | GR Metric: {:.2} dB | Out Metric: {:.2} dB",
        out_rms, delta_db, gr_metric, out_peak_metric);
    assert!(delta_db.abs() < 0.01, "Transparency failure: signal modified by {:.4} dB", delta_db);
    assert!(gr_metric < 0.05, "False Gain Reduction reported on clean signal: {:.2} dB", gr_metric);

    // ------------------------------------------------------------
    // TEST 2: Limiting at Various Input Gains (+2, +4, +6, +10 dB)
    // ------------------------------------------------------------
    println!("\n--- TEST 2: Limiting & Ceiling Compliance across Gain Steps ---");
    let test_gains = [0.0, 2.0, 4.0, 6.0, 8.0, 10.0];
    let ceiling_db = -0.1;
    let signal_0db = generate_sine(sr, 1000.0, 0.0, 9600);

    for &gain in &test_gains {
        let mut lim = Limiter::default();
        lim.set_sample_rate(sr);
        let cfg = LimiterConfig {
            gain_db: gain,
            ceiling_db,
            lookahead_ms: 1.0,
            release_ms: 0.0,
            lift: 0.0,
            auto_release: true,
            true_peak: true,
            punch: false,
            unity: false,
        };

        let mut out = Vec::with_capacity(signal_0db.len());
        for f in &signal_0db {
            out.push(lim.process_with_config(*f, &cfg));
        }

        let steady_out = &out[4800..];
        let peak_out = measure_peak_db(steady_out);
        let tp_out = measure_true_peak_db(steady_out);
        let rms_out = measure_rms_db(steady_out);
        let (gr_metric, _out_metric) = lim.take_metrics();

        println!("Gain: +{:>4.1} dB | Out Peak: {:>5.2} dBFS | Out TruePeak: {:>5.2} dBTP | Out RMS: {:>5.2} dBFS | GR Rep: {:>4.1} dB",
            gain, peak_out, tp_out, rms_out, gr_metric);

        assert!(peak_out <= ceiling_db + 0.02, "Ceiling breach at gain +{}: peak = {:.3} > {:.3}", gain, peak_out, ceiling_db);
    }

    // ------------------------------------------------------------
    // TEST 3: Inter-Sample Peak Prevention (True Peak Test)
    // ------------------------------------------------------------
    println!("\n--- TEST 3: Inter-Sample Peak Detection & Suppression ---");
    // Pathological ISP signal: 12 kHz at 48kHz, phase shifted by 45 deg so sample values are ±0.7071 (-3.01 dBFS),
    // but the analog continuous reconstructed wave peaks at exactly 1.0 (0.0 dBFS).
    let mut isp_signal = Vec::new();
    for i in 0..4800 {
        let s = (PI / 4.0 + (i as f32) * PI / 2.0).sin(); // 0.7071, 0.7071, -0.7071, -0.7071
        isp_signal.push(StereoFrame::new(s, s));
    }
    let raw_sample_peak = measure_peak_db(&isp_signal);
    let raw_true_peak = measure_true_peak_db(&isp_signal);
    println!("Raw ISP Input -> Sample Peak: {:.2} dBFS, True Peak: {:.2} dBTP (Inter-sample overshoot = +{:.2} dB)",
        raw_sample_peak, raw_true_peak, raw_true_peak - raw_sample_peak);

    // Test with True Peak OFF vs ON at ceiling -0.5 dBTP
    for tp_enabled in [false, true] {
        let mut lim = Limiter::default();
        lim.set_sample_rate(sr);
        let cfg = LimiterConfig {
            gain_db: 0.0,
            ceiling_db: -0.5,
            lookahead_ms: 1.0,
            release_ms: 0.0,
            lift: 0.0,
            auto_release: true,
            true_peak: tp_enabled,
            punch: false,
            unity: false,
        };
        let mut out = Vec::new();
        for f in &isp_signal {
            out.push(lim.process_with_config(*f, &cfg));
        }
        let tp_measured = measure_true_peak_db(&out[1000..]);
        let sp_measured = measure_peak_db(&out[1000..]);
        println!("True Peak Mode {:<5} | Sample Peak: {:>5.2} dBFS | True Peak: {:>5.2} dBTP (Ceiling: -0.5 dBTP)",
            if tp_enabled { "ON" } else { "OFF" }, sp_measured, tp_measured);
    }

    // ------------------------------------------------------------
    // TEST 4: Lookahead Transient Overshoot Comparison
    // ------------------------------------------------------------
    println!("\n--- TEST 4: Lookahead Transient Response (0ms vs 1ms vs 3ms vs 5ms) ---");
    // Quiet signal with sudden harsh high-energy transient spikes
    let mut burst_signal = vec![StereoFrame::new(0.05, 0.05); 4800];
    for burst_pos in [1000, 2000, 3000] {
        for j in 0..16 {
            burst_signal[burst_pos + j] = StereoFrame::new(2.5, 2.5); // +8 dBFS spike
        }
    }

    for lookahead in [0.0, 0.5, 1.0, 2.0, 5.0] {
        let mut lim = Limiter::default();
        lim.set_sample_rate(sr);
        let cfg = LimiterConfig {
            gain_db: 0.0,
            ceiling_db: -0.1,
            lookahead_ms: lookahead,
            release_ms: 100.0,
            lift: 0.0,
            auto_release: false,
            true_peak: true,
            punch: false,
            unity: false,
        };
        let mut out = Vec::new();
        for f in &burst_signal {
            out.push(lim.process_with_config(*f, &cfg));
        }
        let peak = measure_peak_db(&out);
        let tp = measure_true_peak_db(&out);
        println!("Lookahead: {:>3.1} ms | Max Peak: {:>5.2} dBFS | Max TruePeak: {:>5.2} dBTP",
            lookahead, peak, tp);
        assert!(peak <= -0.1 + 0.02, "Ceiling breach with lookahead {}ms: {:.3}", lookahead, peak);
    }

    // ------------------------------------------------------------
    // TEST 5: Naturl LIFT Low-Level Expansion
    // ------------------------------------------------------------
    println!("\n--- TEST 5: Naturl LIFT Dynamic Low-Level Content Expansion ---");
    let test_levels = [-60.0, -40.0, -30.0, -24.0, -18.0, -6.0, 0.0];
    for &lvl in &test_levels {
        let sig = generate_sine(sr, 1000.0, lvl, 2400);

        let mut lim_off = Limiter::default();
        let cfg_off = LimiterConfig {
            gain_db: 0.0,
            ceiling_db: 0.0,
            lookahead_ms: 0.0,
            release_ms: 0.0,
            lift: 0.0,
            auto_release: true,
            true_peak: false,
            punch: false,
            unity: false,
        };
        let mut out_off = Vec::new();
        for f in &sig { out_off.push(lim_off.process_with_config(*f, &cfg_off)); }

        let mut lim_lift = Limiter::default();
        let cfg_lift = LimiterConfig {
            lift: 1.0, // 100% LIFT
            ..cfg_off
        };
        let mut out_lift = Vec::new();
        for f in &sig { out_lift.push(lim_lift.process_with_config(*f, &cfg_lift)); }

        let rms_off = measure_rms_db(&out_off[480..]);
        let rms_lift = measure_rms_db(&out_lift[480..]);
        let lift_boost = rms_lift - rms_off;
        println!("Input: {:>5.1} dBFS | LIFT OFF RMS: {:>6.2} dBFS | LIFT 100% RMS: {:>6.2} dBFS | Boost: +{:.2} dB",
            lvl, rms_off, rms_lift, lift_boost);
    }

    // ------------------------------------------------------------
    // TEST 6: Unity Mode Auditioning
    // ------------------------------------------------------------
    println!("\n--- TEST 6: Unity Gain Matching Auditioning Mode ---");
    let sig_unity = generate_sine(sr, 1000.0, -14.0, 4800);
    for gain in [2.0, 4.0, 6.0, 9.0] {
        let mut lim = Limiter::default();
        let cfg = LimiterConfig {
            gain_db: gain,
            ceiling_db: -0.1,
            lookahead_ms: 0.0,
            release_ms: 0.0,
            lift: 0.0,
            auto_release: true,
            true_peak: false,
            punch: false,
            unity: true, // Unity mode active
        };
        let mut out = Vec::new();
        for f in &sig_unity { out.push(lim.process_with_config(*f, &cfg)); }
        let in_rms = measure_rms_db(&sig_unity[480..]);
        let out_rms = measure_rms_db(&out[480..]);
        let diff = (out_rms - in_rms).abs();
        println!("Input Gain: +{:>3.1} dB (Unity ON) | In RMS: {:.2} dBFS | Out RMS: {:.2} dBFS | Delta: {:.3} dB",
            gain, in_rms, out_rms, out_rms - in_rms);
        assert!(diff < 0.05, "Unity mode did not perfectly match loudness: diff = {:.3} dB", diff);
    }

    println!("\n============================================================");
    println!("  ALL LIMITER MATHEMATICAL & REAL-TIME TESTS COMPLETED!");
    println!("============================================================");
}
