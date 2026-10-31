use masteraid::dsp::clipper::{Clipper, ClipperConfig};
use masteraid::dsp::output_stage::OutputStage;
use masteraid::dsp::shared::{gain_to_db, StereoFrame};
use masteraid::dsp::tape::{Tape, TapeConfig};

fn pink_noise(n: usize) -> Vec<f32> {
    // Deterministic white noise -> simple pink filter, normalized to -18 dBFS RMS.
    let mut state = 0x12345678_u32;
    let mut b = [0.0_f64; 7];
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        state = state.wrapping_mul(1664525).wrapping_add(1013904223);
        let white = ((state >> 8) as f64 / 16777216.0) * 2.0 - 1.0;
        b[0] = 0.99886 * b[0] + white * 0.0555179;
        b[1] = 0.99332 * b[1] + white * 0.0750759;
        b[2] = 0.96900 * b[2] + white * 0.1538520;
        b[3] = 0.86650 * b[3] + white * 0.3104856;
        b[4] = 0.55000 * b[4] + white * 0.5329522;
        b[5] = -0.7616 * b[5] - white * 0.0168980;
        let s = b[0] + b[1] + b[2] + b[3] + b[4] + b[5] + b[6] + white * 0.5362;
        b[6] = white * 0.115926;
        out.push(s as f32);
    }
    let rms =
        (out.iter().map(|x| (*x as f64) * (*x as f64)).sum::<f64>() / out.len() as f64).sqrt();
    let scale = (0.125_f64) / rms; // -18 dBFS
    for x in &mut out {
        *x = (*x as f64 * scale) as f32;
    }
    out
}

fn measure<F: FnMut(StereoFrame) -> StereoFrame>(input: &[f32], mut proc: F) -> (f64, f64) {
    let skip = 48000;
    let mut in_acc = 0.0;
    let mut out_acc = 0.0;
    let mut count = 0usize;
    for (i, &x) in input.iter().enumerate() {
        let y = proc(StereoFrame::new(x, x));
        if i >= skip {
            in_acc += (x as f64) * (x as f64);
            out_acc += (y.left as f64) * (y.left as f64);
            count += 1;
        }
    }
    (
        gain_to_db((in_acc / count as f64).sqrt() as f32) as f64,
        gain_to_db((out_acc / count as f64).sqrt() as f32) as f64,
    )
}

fn main() {
    let n = 48000 * 4;
    let input = pink_noise(n);

    println!("=== TAPE (dB in / out / delta) ===");
    let base = TapeConfig {
        is_30_ips: true,
        is_gp9: false,
        drive_db: 0.0,
        bias_db: -2.0,
        head_bump_db: 0.0,
        bump_freq_hz: 80.0,
        flutter_pct: 0.0,
        compensate: false,
    };
    let cases = [
        (
            "drive 0",
            TapeConfig {
                drive_db: 0.0,
                ..base
            },
        ),
        (
            "drive +2",
            TapeConfig {
                drive_db: 2.0,
                ..base
            },
        ),
        (
            "drive +6",
            TapeConfig {
                drive_db: 6.0,
                ..base
            },
        ),
        (
            "drive +10",
            TapeConfig {
                drive_db: 10.0,
                ..base
            },
        ),
        (
            "gp9 +2",
            TapeConfig {
                is_gp9: true,
                drive_db: 2.0,
                ..base
            },
        ),
        (
            "gp9 +10",
            TapeConfig {
                is_gp9: true,
                drive_db: 10.0,
                ..base
            },
        ),
        (
            "bump 0",
            TapeConfig {
                head_bump_db: 0.0,
                ..base
            },
        ),
        (
            "bump +10",
            TapeConfig {
                head_bump_db: 10.0,
                ..base
            },
        ),
        (
            "bias +10",
            TapeConfig {
                bias_db: 10.0,
                ..base
            },
        ),
        (
            "bias -10",
            TapeConfig {
                bias_db: -10.0,
                ..base
            },
        ),
    ];
    for (label, cfg) in cases {
        let mut raw = Tape::default();
        raw.set_sample_rate(48000.0);
        let (_, o_raw) = measure(&input, |f| raw.process_with_config(f, &cfg));

        let mut comp = Tape::default();
        comp.set_sample_rate(48000.0);
        let (i, o_comp) = measure(&input, |f| {
            comp.process_with_config(
                f,
                &TapeConfig {
                    compensate: true,
                    ..cfg
                },
            )
        });
        let raw_delta = o_raw - i;
        let comp_delta = o_comp - i;
        println!("{label:12} in {i:7.2} raw {raw_delta:6.2} comp {comp_delta:6.2}");
    }

    println!("=== COLORATION AMP (trans bypass) ===");
    let amp_names = [
        "", "Triode", "PushPull", "Pentode", "FET", "MOSFET", "Diode",
    ];
    for amp in 1..=6 {
        for intensity in 0..=2 {
            let mut raw = OutputStage::default();
            raw.set_sample_rate(48000.0);
            raw.set_compensation(false);
            let (_, o_raw) = measure(&input, |f| raw.process_with_params(f, amp, 0, intensity));

            let mut comp = OutputStage::default();
            comp.set_sample_rate(48000.0);
            comp.set_compensation(true);
            let (i, o_comp) = measure(&input, |f| comp.process_with_params(f, amp, 0, intensity));
            println!(
                "{:10} int {} raw {:6.2} comp {:6.2}",
                amp_names[amp as usize],
                intensity,
                o_raw - i,
                o_comp - i
            );
        }
    }
    println!("=== COLORATION TRANSFORMER (amp bypass) ===");
    let trans_names = ["", "Steel", "Nickel", "Amorphous", "MuMetal"];
    for trans in 1..=4 {
        for intensity in 0..=2 {
            let mut raw = OutputStage::default();
            raw.set_sample_rate(48000.0);
            raw.set_compensation(false);
            let (_, o_raw) = measure(&input, |f| raw.process_with_params(f, 0, trans, intensity));

            let mut comp = OutputStage::default();
            comp.set_sample_rate(48000.0);
            comp.set_compensation(true);
            let (i, o_comp) = measure(&input, |f| comp.process_with_params(f, 0, trans, intensity));
            println!(
                "{:10} int {} raw {:6.2} comp {:6.2}",
                trans_names[trans as usize],
                intensity,
                o_raw - i,
                o_comp - i
            );
        }
    }

    println!("=== CLIPPER ===");
    for drive in [-6.0, 0.0, 2.0, 6.0, 10.0] {
        for hard in [false, true] {
            let mut raw = Clipper::default();
            let raw_cfg = ClipperConfig {
                drive_db: drive,
                ceiling_db: -0.3,
                hard_shape: hard,
                oversample_4x: true,
                compensate: false,
            };
            let (_, o_raw) = measure(&input, |f| raw.process_with_config(f, &raw_cfg));

            let mut comp = Clipper::default();
            let comp_cfg = ClipperConfig {
                compensate: true,
                ..raw_cfg
            };
            let (i, o_comp) = measure(&input, |f| comp.process_with_config(f, &comp_cfg));
            println!(
                "drive {drive:5.1} hard {hard:5} raw {:6.2} comp {:6.2}",
                o_raw - i,
                o_comp - i
            );
        }
    }
}
