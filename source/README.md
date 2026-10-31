# Masteraid Source Structure

The source tree uses Rust-style module names. DSP processors are file-based unless a model grows enough internal detail to justify its own folder.

## Plugin Shell

- `lib.rs` - crate root and CLAP/VST3 exports.
- `plugin.rs` - main `nih-plug` plugin implementation.
- `parameters.rs` - automatable parameters and model selectors.

## DSP

- `dsp/chain.rs` - fixed serial mastering chain order: `Input -> Filters -> EQ -> Dynamics -> Tape -> Clipper -> Limiter -> Output`.
- `dsp/io.rs` - input trim, polarity, `LO CUT` / `HI CUT` filters (12/24 dB/oct), bass monoizer, final output trim, and input/output metering.
- `dsp/eq_ptek.rs` - `PTEK` passive/Pultec-style program EQ processor with interactive low-end boost/atten curve trick.
- `dsp/eq_mass.rs` - `MASS` Massive Passive-style 4-band proportional-Q LC parametric EQ processor.
- `dsp/comp_fair.rs` - `FAIR` Fairchild 670 variable-mu compressor with 6 Time Constants and ButterComp2 dynamic release recovery.
- `dsp/comp_vmu.rs` - `V-MU` modern Vari-Mu compressor with sidechain HPF, continuous attack/release, and parallel blend.
- `dsp/output_stage.rs` - shared 7 amplifier topologies and 5 transformer core coloration DSP used independently by EQ and compressor.
- `dsp/tape.rs` - ATR-102 tape mastering processor upgraded to `ToTape8` (Dubly dynamic compander encode/decode, golden-ratio bias slew, resonant head bump biquads, infrasonic servo).
- `dsp/clipper.rs` - `ClipOnly3` nonlinear peak control with 4x linear-phase oversampling and noise-masked dithered edge transitions.
- `dsp/limiter.rs` - true-peak brickwall limiter (BlockParty + ButterComp2 + ClipOnly3 + Smooth Punch + Lift).
- `dsp/shared.rs` - reusable math, filters, smoothing, metering helpers, and utility types.

## GUI

- `gui/editor.rs` - main editor view and interaction/readout logic.
- `gui/primitives` - shared, reusable drawing primitives:
  - `palette.rs` - accent/silk colour tokens.
  - `text.rs` - left/centre text helpers.
  - `card.rs` - panel cards and recessed plate wells.
  - `dseg.rs` - 14-segment readout tokenizers and glass display.
  - `model_select.rs` - two-position model selector.
  - `buttons.rs` - bypass buttons, backlit push buttons, segmented selectors.
  - `cassette.rs` - coloration cassette (amplifier/transformer + harmonic drive).
  - `knob.rs` - concentric, large, and small stepped pots.
  - `meters.rs` - VU meter, input ladder, LUFS/true-peak display.
  - `power.rs` - master power strip.
  - `screw.rs` - panel screws.
- `gui/theme.rs` - colors, typography, layout constants, and hardware-style visual tokens.
