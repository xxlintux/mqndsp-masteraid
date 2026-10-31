//! Offline probe: characterise both compressor models with every other module bypassed, and
//! verify the knob spans the editor calibrates them to.
//!
//! Run with `cargo run --release --example calibrate`.
//!
//! The editor maps the big THRESHOLD knob (mark 0..10) onto a fixed per-model span:
//!   Fairchild -26..-6 dBFS, Vari-Mu -24..-6 dBFS.
//! Both tops sit at -6 dBFS so the compressor can always be dialled out of circuit on normal
//! programme material; the Fairchild span is wider because its detector clamps the output at the
//! threshold (gain reduction tracks the level almost 1:1), while the Vari-Mu energy detector
//! compresses harder per dB over the threshold.

use masteraid::dsp::comp_fair::{CompConfig, CompFair};
use masteraid::dsp::comp_vmu::CompVMu;
use masteraid::dsp::shared::StereoFrame;

const SR: f32 = 48_000.0;

/// Must match `EqEditorView::FAIR_THRESH_*` / `VMU_THRESH_*`.
const FAIR_SPAN: (f32, f32) = (-26.0, -6.0);
const VMU_SPAN: (f32, f32) = (-24.0, -6.0);

fn cfg(threshold_db: f32) -> CompConfig {
    CompConfig {
        threshold_db,
        input_gain_db: 0.0,
        ratio: 4.0,
        attack_ms: 1.0,
        release_ms: 500.0,
        sc_hpf_hz: 0.0,
        mix_pct: 100.0,
        makeup_db: 0.0,
        auto_makeup: false,
    }
}

fn fair_gr(c: &CompConfig, level_db: f32) -> f32 {
    let mut comp = CompFair::default();
    let amp = 10f32.powf(level_db / 20.0);
    let step = 1000.0 * std::f32::consts::TAU / SR;
    let mut phase = 0.0f32;
    for _ in 0..(SR as usize * 2) {
        let x = amp * phase.sin();
        let _ = comp.process_with_config(StereoFrame::new(x, x), c);
        phase += step;
    }
    comp.gr_db_l
}

fn vmu_gr(c: &CompConfig, level_db: f32) -> f32 {
    let mut comp = CompVMu::default();
    let amp = 10f32.powf(level_db / 20.0);
    let step = 1000.0 * std::f32::consts::TAU / SR;
    let mut phase = 0.0f32;
    for _ in 0..(SR as usize * 2) {
        let x = amp * phase.sin();
        let _ = comp.process_with_config(StereoFrame::new(x, x), c);
        phase += step;
    }
    comp.gr_db_l
}

fn report(name: &str, f: fn(&CompConfig, f32) -> f32, span: (f32, f32), levels: [f32; 4]) {
    let (min, max) = span;
    println!();
    println!("=== {name}: GR (dB) per knob mark, span {min:.0}..{max:.0} dBFS ===");
    print!("mark thresh ");
    for l in levels {
        print!("{:>7.0}", l);
    }
    println!("   (input dBFS)");
    for k in 0..=10 {
        let t = min + (max - min) * (k as f32 / 10.0);
        print!("{:>4} {:>6.1} ", k, t);
        for l in levels {
            print!("{:>7.1}", f(&cfg(t), l));
        }
        println!();
    }
}

fn main() {
    let levels = [-24.0f32, -18.0, -12.0, -6.0];
    report("FAIR", fair_gr, FAIR_SPAN, levels);
    report("V-MU", vmu_gr, VMU_SPAN, levels);

    println!();
    println!("=== INPUT GAIN drive, threshold -18 dB, input -18 dBFS ===");
    println!("gain_db  fair  v-mu");
    for g in [-10.0f32, -6.0, -3.0, 0.0, 3.0, 6.0, 10.0] {
        let mut c = cfg(-18.0);
        c.input_gain_db = g;
        println!(
            "{:>7.1}  {:>9.1}  {:>8.1}",
            g,
            fair_gr(&c, -18.0),
            vmu_gr(&c, -18.0)
        );
    }
}
