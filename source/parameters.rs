use std::sync::{Arc, Mutex};

use nih_plug::prelude::*;
use nih_plug_vizia::ViziaState;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Enum)]
pub enum EqModel {
    Ptek,
    Mass,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Enum)]
pub enum CompModel {
    Fair,
    VMu,
}

#[derive(Params)]
pub struct MasteraidParams {
    #[persist = "editor-state"]
    pub editor_state: Arc<ViziaState>,

    // ==========================================
    // Master Chain Routing, Input & Filtering
    // ==========================================
    #[id = "master_power"]
    pub master_power: BoolParam,

    #[id = "input_trim"]
    pub input_trim: FloatParam,

    #[id = "input_bypass"]
    pub input_bypass: BoolParam,

    #[id = "polarity"]
    pub polarity: BoolParam,

    #[id = "input_mono"]
    pub input_mono: BoolParam,

    #[id = "filter_lo_cut_hz"]
    pub filter_lo_cut_hz: FloatParam,

    #[id = "filter_lo_cut_slope"]
    pub filter_lo_cut_slope: IntParam,

    #[id = "filter_hi_cut_hz"]
    pub filter_hi_cut_hz: FloatParam,

    #[id = "filter_hi_cut_slope"]
    pub filter_hi_cut_slope: IntParam,

    #[id = "bass_mono_freq"]
    pub bass_mono_freq: FloatParam,

    #[id = "bass_mono_slope"]
    pub bass_mono_slope: IntParam,

    #[id = "bass_mono_amount"]
    pub bass_mono_amount: FloatParam,

    #[id = "eq_bypass"]
    pub eq_bypass: BoolParam,

    #[id = "eq_model"]
    pub eq_model: EnumParam<EqModel>,

    #[id = "comp_bypass"]
    pub comp_bypass: BoolParam,

    #[id = "comp_model"]
    pub comp_model: EnumParam<CompModel>,

    // ==========================================
    // Unified Mastering Dynamics Parameters
    // ==========================================
    #[id = "comp_thresh"]
    pub comp_thresh: FloatParam,

    #[id = "comp_ratio"]
    pub comp_ratio: FloatParam,

    /// Fair-style input drive. On the real 670 the INPUT GAIN attenuator sits ahead of the
    /// amplifier stage that does the compressing, so it changes how hard the tube is hit without
    /// moving the threshold. Separate from `comp_makeup`, which trims the output.
    #[id = "comp_in_gain"]
    pub comp_in_gain: FloatParam,

    #[id = "comp_attack"]
    pub comp_attack: FloatParam,

    #[id = "comp_release"]
    pub comp_release: FloatParam,

    #[id = "comp_sc_hpf_hz"]
    pub comp_sc_hpf_hz: FloatParam,

    #[id = "comp_mix"]
    pub comp_mix: FloatParam,

    #[id = "comp_makeup"]
    pub comp_makeup: FloatParam,

    /// Program-dependent auto makeup (push-and-hold on the makeup knob). DSP-side so it keeps
    /// working with the editor closed and persists with the project.
    #[id = "comp_auto_makeup"]
    pub comp_auto_makeup: BoolParam,

    // ==========================================
    // FAIR ("comp_fair") Legacy Parameters
    // ==========================================
    #[id = "fair_threshold"]
    pub fair_threshold: FloatParam,

    #[id = "fair_time_constant"]
    pub fair_time_constant: IntParam, // 0..5 -> [1, 2, 3, 4, 5, 6]

    #[id = "fair_mix"]
    pub fair_mix: FloatParam, // 0..100 %

    #[id = "fair_makeup"]
    pub fair_makeup: FloatParam, // 0..12 dB

    #[id = "fair_sc_hpf"]
    pub fair_sc_hpf: BoolParam,

    // ==========================================
    // V-MU ("comp_vmu") Legacy Parameters
    // ==========================================
    #[id = "vmu_threshold"]
    pub vmu_threshold: FloatParam,

    #[id = "vmu_attack"]
    pub vmu_attack: IntParam, // 0..4 -> [Fast, M-Fast, Med, M-Slow, Slow]

    #[id = "vmu_recovery"]
    pub vmu_recovery: IntParam, // 0..3 -> [Fast, Med, Slow, Auto]

    #[id = "vmu_mode"]
    pub vmu_mode: BoolParam, // false = Compress, true = Limit

    #[id = "vmu_sc_hpf"]
    pub vmu_sc_hpf: BoolParam, // false = Flat, true = 100 Hz HPF

    #[id = "vmu_mix"]
    pub vmu_mix: FloatParam, // 0..100 %

    #[id = "vmu_makeup"]
    pub vmu_makeup: FloatParam, // 0..12 dB

    // ==========================================
    // EQ & Compressor Output Stage Coloration
    // (0=Bypass, 1=Triode, 2=Push-Pull, 3=Pentode, 4=FET, 5=MOSFET, 6=Diode)
    // (0=Bypass, 1=Steel, 2=Nickel, 3=Amorphous, 4=Mu-Metal)
    // (0=Subtle/Off, 1=Normal/On, 2=Smashed Glass/Extreme)
    // ==========================================
    #[id = "eq_amp_type"]
    pub eq_amp_type: IntParam,

    #[id = "eq_trans_type"]
    pub eq_trans_type: IntParam,

    #[id = "eq_intensity"]
    pub eq_intensity: IntParam,

    /// Level-neutral coloration (AUTO switch on the EQ cassette).
    #[id = "eq_col_comp"]
    pub eq_col_comp: BoolParam,

    /// Hardware-topology match mode: locks amplifier & transformer to active EQ model defaults.
    #[id = "eq_col_match"]
    pub eq_col_match: BoolParam,

    /// Frozen AUTO learn result for the EQ coloration, serialised with the project so the
    /// measurement survives a reload. Written by the audio thread only when a learn pass finishes.
    #[persist = "eq-col-auto"]
    pub eq_col_auto: Arc<Mutex<Vec<f32>>>,

    #[id = "comp_amp_type"]
    pub comp_amp_type: IntParam,

    #[id = "comp_trans_type"]
    pub comp_trans_type: IntParam,

    #[id = "comp_intensity"]
    pub comp_intensity: IntParam,

    /// Level-neutral coloration (AUTO switch on the compressor cassette).
    #[id = "comp_col_comp"]
    pub comp_col_comp: BoolParam,

    /// Hardware-topology match mode: locks amplifier & transformer to active Comp model defaults.
    #[id = "comp_col_match"]
    pub comp_col_match: BoolParam,

    /// Global interactive tooltips toggle.
    #[id = "tooltips_enabled"]
    pub tooltips_enabled: BoolParam,

    /// UI Theme Mode: false = Dark (Hardware Rack), true = Light (Studio Aluminum)
    #[id = "theme_light"]
    pub theme_light: BoolParam,

    /// Frozen AUTO learn result for the compressor coloration (see `eq_col_auto`).
    #[persist = "comp-col-auto"]
    pub comp_col_auto: Arc<Mutex<Vec<f32>>>,

    // ==========================================
    // Mastering Clipper Parameters
    // ==========================================
    #[id = "clipper_bypass"]
    pub clipper_bypass: BoolParam,

    #[id = "clipper_drive"]
    pub clipper_drive: FloatParam,

    #[id = "clipper_ceiling"]
    pub clipper_ceiling: FloatParam,

    #[id = "clipper_hard_shape"]
    pub clipper_hard_shape: BoolParam,

    #[id = "clipper_os_4x"]
    pub clipper_os_4x: BoolParam,

    /// Keep the clipper level-neutral: Drive sets clip density only (AUTO switch).
    #[id = "clipper_comp"]
    pub clipper_comp: BoolParam,

    // ==========================================
    // Analog Tape Mastering Parameters (ATR-102)
    // ==========================================
    #[id = "tape_bypass"]
    pub tape_bypass: BoolParam,

    #[id = "tape_speed_30ips"]
    pub tape_speed_30ips: BoolParam,

    #[id = "tape_formula_gp9"]
    pub tape_formula_gp9: BoolParam,

    #[id = "tape_drive"]
    pub tape_drive: FloatParam,

    #[id = "tape_bias"]
    pub tape_bias: FloatParam,

    #[id = "tape_headbump"]
    pub tape_headbump: FloatParam,

    #[id = "tape_bump_freq"]
    pub tape_bump_freq: FloatParam,

    #[id = "tape_flutter"]
    pub tape_flutter: FloatParam,

    /// Keep the tape level-neutral: Drive sets saturation only (AUTO switch).
    #[id = "tape_comp"]
    pub tape_comp: BoolParam,

    // ==========================================
    // Brickwall Limiter & Final Output Parameters
    // ==========================================
    #[id = "limiter_bypass"]
    pub limiter_bypass: BoolParam,

    #[id = "limiter_gain"]
    pub limiter_gain: FloatParam,

    #[id = "limiter_ceiling"]
    pub limiter_ceiling: FloatParam,

    #[id = "limiter_lookahead_ms"]
    pub limiter_lookahead_ms: FloatParam,

    #[id = "limiter_release_ms"]
    pub limiter_release_ms: FloatParam,

    #[id = "limiter_lift"]
    pub limiter_lift: FloatParam,

    #[id = "limiter_auto_release"]
    pub limiter_auto_release: BoolParam,

    #[id = "limiter_true_peak"]
    pub limiter_true_peak: BoolParam,

    #[id = "limiter_punch"]
    pub limiter_punch: BoolParam,

    #[id = "limiter_unity"]
    pub limiter_unity: BoolParam,

    #[id = "output_trim"]
    pub output_trim: FloatParam,

    // ==========================================
    // PTEK Parameters
    // ==========================================
    #[id = "ptek_low_boost"]
    pub ptek_low_boost: FloatParam,

    #[id = "ptek_low_atten"]
    pub ptek_low_atten: FloatParam,

    #[id = "ptek_low_freq"]
    pub ptek_low_freq: IntParam, // 0..5 -> [20, 30, 40, 60, 100, 150 Hz] (hardware 20/30/60/100 + 40/150 extras)

    #[id = "ptek_high_boost"]
    pub ptek_high_boost: FloatParam,

    #[id = "ptek_high_freq"]
    pub ptek_high_freq: IntParam, // 0..6 -> [3k, 4k, 5k, 8k, 10k, 12k, 16k Hz] (EQP-1A boost frequencies)

    #[id = "ptek_bandwidth"]
    pub ptek_bandwidth: FloatParam, // 0..10

    #[id = "ptek_high_atten"]
    pub ptek_high_atten: FloatParam,

    #[id = "ptek_atten_freq"]
    pub ptek_atten_freq: IntParam, // 0..2 -> [5k, 10k, 20k Hz]

    // ==========================================
    // MASS 4-Band Mastering EQ Parameters
    // ==========================================
    // Band 1: LOW
    #[id = "mass_low_bypass"]
    pub mass_low_bypass: BoolParam,

    #[id = "mass_low_gain"]
    pub mass_low_gain: FloatParam, // -9 dB .. +9 dB

    #[id = "mass_low_freq"]
    pub mass_low_freq: IntParam, // 0..5 -> [33, 47, 68, 100, 150, 220 Hz]

    #[id = "mass_low_q"]
    pub mass_low_q: FloatParam, // 0.4 .. 5.0

    #[id = "mass_low_shelf"]
    pub mass_low_shelf: BoolParam,

    // Band 2: LOW MID
    #[id = "mass_lmf_bypass"]
    pub mass_lmf_bypass: BoolParam,

    #[id = "mass_lmf_gain"]
    pub mass_lmf_gain: FloatParam,

    #[id = "mass_lmf_freq"]
    pub mass_lmf_freq: IntParam, // 0..5 -> [150, 220, 330, 470, 680, 1k Hz]

    #[id = "mass_lmf_q"]
    pub mass_lmf_q: FloatParam, // 0.4 .. 5.0

    #[id = "mass_lmf_shelf"]
    pub mass_lmf_shelf: BoolParam,

    // Band 3: HIGH MID
    #[id = "mass_hmf_bypass"]
    pub mass_hmf_bypass: BoolParam,

    #[id = "mass_hmf_gain"]
    pub mass_hmf_gain: FloatParam,

    #[id = "mass_hmf_freq"]
    pub mass_hmf_freq: IntParam, // 0..5 -> [680, 1k, 1.5k, 2.2k, 3.3k, 4.7k Hz]

    #[id = "mass_hmf_q"]
    pub mass_hmf_q: FloatParam, // 0.4 .. 5.0

    #[id = "mass_hmf_shelf"]
    pub mass_hmf_shelf: BoolParam,

    // Band 4: HIGH
    #[id = "mass_hf_bypass"]
    pub mass_hf_bypass: BoolParam,

    #[id = "mass_hf_gain"]
    pub mass_hf_gain: FloatParam,

    #[id = "mass_hf_freq"]
    pub mass_hf_freq: IntParam, // 0..5 -> [3.3k, 4.7k, 6.8k, 10k, 15k, 20k Hz]

    #[id = "mass_hf_q"]
    pub mass_hf_q: FloatParam, // 0.4 .. 5.0

    #[id = "mass_hf_shelf"]
    pub mass_hf_shelf: BoolParam,
}

impl Default for MasteraidParams {
    fn default() -> Self {
        Self {
            editor_state: ViziaState::new(|| (1360, 952)),

            master_power: BoolParam::new("Master Power", true),

            input_trim: FloatParam::new(
                "Input Trim",
                0.0,
                FloatRange::Linear {
                    min: -10.0,
                    max: 10.0,
                },
            )
            .with_unit(" dB"),

            input_bypass: BoolParam::new("Input Stage Bypass", false),

            polarity: BoolParam::new("Polarity", false),
            input_mono: BoolParam::new("Input Mono", false),

            filter_lo_cut_hz: FloatParam::new(
                "Lo Cut Frequency",
                0.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 300.0,
                },
            )
            .with_unit(" Hz"),

            // 0 = 12 dB/oct, 1 = 24 dB/oct, 2 = 36 dB/oct.
            filter_lo_cut_slope: IntParam::new(
                "Lo Cut Slope",
                0,
                IntRange::Linear { min: 0, max: 2 },
            ),

            filter_hi_cut_hz: FloatParam::new(
                "Hi Cut Frequency",
                0.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 20000.0,
                },
            )
            .with_unit(" Hz"),

            filter_hi_cut_slope: IntParam::new(
                "Hi Cut Slope",
                0,
                IntRange::Linear { min: 0, max: 2 },
            ),

            bass_mono_freq: FloatParam::new(
                "Bass Mono Frequency",
                0.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 220.0,
                },
            )
            .with_unit(" Hz"),

            bass_mono_slope: IntParam::new(
                "Bass Mono Slope",
                0,
                IntRange::Linear { min: 0, max: 2 },
            ),

            bass_mono_amount: FloatParam::new(
                "Bass Mono Amount",
                70.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 100.0,
                },
            )
            .with_unit(" %"),

            eq_bypass: BoolParam::new("EQ Bypass", false),
            eq_model: EnumParam::new("EQ Model", EqModel::Mass),

            comp_bypass: BoolParam::new("Compressor Bypass", false),
            comp_model: EnumParam::new("Compressor Model", CompModel::Fair),

            // Unified Mastering Dynamics Defaults (matches masteraid_console.svg)
            comp_thresh: FloatParam::new(
                "Comp Threshold",
                0.0,
                // Wide enough to cover any calibrated knob range set from the debug menu.
                FloatRange::Linear {
                    min: -60.0,
                    max: 20.0,
                },
            )
            .with_unit(" dB"),
            comp_ratio: FloatParam::new(
                "Comp Ratio",
                2.0,
                FloatRange::Skewed {
                    min: 1.2,
                    max: 20.0,
                    factor: FloatRange::skew_factor(-1.2),
                },
            ),
            comp_in_gain: FloatParam::new(
                "Comp Input Gain",
                0.0,
                FloatRange::Linear {
                    min: -10.0,
                    max: 10.0,
                },
            )
            .with_unit(" dB")
            .with_step_size(0.1),
            comp_attack: FloatParam::new(
                "Comp Attack",
                1.0,
                FloatRange::Skewed {
                    min: 0.1,
                    max: 30.0,
                    factor: FloatRange::skew_factor(-1.2),
                },
            )
            .with_unit(" ms"),
            comp_release: FloatParam::new(
                "Comp Release",
                1000.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 2000.0,
                },
            )
            .with_unit(" ms"),
            comp_sc_hpf_hz: FloatParam::new(
                "Comp SC HPF",
                80.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 250.0,
                },
            )
            .with_unit(" Hz"),
            comp_mix: FloatParam::new(
                "Comp Mix",
                100.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 100.0,
                },
            )
            .with_unit(" %"),
            comp_makeup: FloatParam::new(
                "Comp Makeup",
                0.0,
                FloatRange::Linear {
                    min: -10.0,
                    max: 20.0,
                },
            )
            .with_unit(" dB"),
            comp_auto_makeup: BoolParam::new("Comp Auto Makeup", true),

            // Clipper Defaults
            clipper_bypass: BoolParam::new("Clipper Bypass", false),
            clipper_drive: FloatParam::new(
                "Clipper Drive",
                0.0,
                FloatRange::Linear {
                    min: -10.0,
                    max: 10.0,
                },
            )
            .with_unit(" dB"),
            clipper_ceiling: FloatParam::new(
                "Clipper Ceiling",
                -0.1,
                FloatRange::Linear {
                    min: -3.0,
                    max: 0.0,
                },
            )
            .with_unit(" dB"),
            clipper_hard_shape: BoolParam::new("Clipper Hard Shape", false),
            clipper_os_4x: BoolParam::new("Clipper 4x Oversampling", true),
            clipper_comp: BoolParam::new("Clipper Auto Gain", false),

            // Tape Defaults (Neutral: 30 IPS, GP9, Drive 0.0dB, Bias 0.0dB, HeadBump 0.0dB, 60Hz, Flutter 0.0%)
            tape_bypass: BoolParam::new("Tape Bypass", false),
            tape_speed_30ips: BoolParam::new("Tape 30 IPS", true),
            tape_formula_gp9: BoolParam::new("Tape Formula GP9", true),
            tape_drive: FloatParam::new(
                "Tape Drive",
                0.0,
                FloatRange::Linear {
                    min: -10.0,
                    max: 10.0,
                },
            )
            .with_unit(" dB"),
            tape_bias: FloatParam::new(
                "Tape Bias",
                0.0,
                FloatRange::Linear {
                    min: -10.0,
                    max: 10.0,
                },
            )
            .with_unit(" dB"),
            tape_headbump: FloatParam::new(
                "Tape Head Bump",
                0.0,
                FloatRange::Linear {
                    min: -10.0,
                    max: 10.0,
                },
            )
            .with_unit(" dB"),
            tape_bump_freq: FloatParam::new(
                "Tape Bump Freq",
                60.0,
                FloatRange::Linear {
                    min: 30.0,
                    max: 150.0,
                },
            )
            .with_unit(" Hz"),
            tape_flutter: FloatParam::new(
                "Tape Flutter",
                0.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 100.0,
                },
            )
            .with_unit(" %"),
            tape_comp: BoolParam::new("Tape Auto Gain", false),

            // Brickwall Limiter & Final Output Defaults
            limiter_bypass: BoolParam::new("Limiter Bypass", false),
            limiter_gain: FloatParam::new(
                "Limiter Gain",
                0.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 10.0,
                },
            )
            .with_unit(" dB"),
            limiter_ceiling: FloatParam::new(
                "Limiter Ceiling",
                -0.1,
                FloatRange::Linear {
                    min: -3.0,
                    max: 0.0,
                },
            )
            .with_unit(" dB"),
            limiter_lookahead_ms: FloatParam::new(
                "Limiter Lookahead",
                1.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 5.0,
                },
            )
            .with_unit(" ms"),
            limiter_release_ms: FloatParam::new(
                "Limiter Release",
                0.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 1200.0,
                },
            )
            .with_unit(" ms"),
            limiter_lift: FloatParam::new(
                "Limiter Lift",
                0.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 1.0,
                },
            ),
            limiter_auto_release: BoolParam::new("Limiter Auto Release", true),
            limiter_true_peak: BoolParam::new("Limiter True Peak", true),
            limiter_punch: BoolParam::new("Limiter Punch Mode", false),
            limiter_unity: BoolParam::new("Limiter Unity", false),

            output_trim: FloatParam::new(
                "Output Trim",
                0.0,
                FloatRange::Linear {
                    min: -10.0,
                    max: 10.0,
                },
            )
            .with_unit(" dB"),

            // PTEK: all knobs are stepped 0..10 on the GUI; flat by default
            ptek_low_boost: FloatParam::new(
                "Ptek Low Boost",
                0.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 10.0,
                },
            ),
            ptek_low_atten: FloatParam::new(
                "Ptek Low Atten",
                0.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 10.0,
                },
            ),
            ptek_low_freq: IntParam::new("Ptek Low Freq", 3, IntRange::Linear { min: 0, max: 5 }),
            ptek_high_boost: FloatParam::new(
                "Ptek High Boost",
                0.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 10.0,
                },
            ),
            ptek_high_freq: IntParam::new("Ptek High Freq", 4, IntRange::Linear { min: 0, max: 6 }),
            ptek_bandwidth: FloatParam::new(
                "Ptek Bandwidth",
                5.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 10.0,
                },
            ),
            ptek_high_atten: FloatParam::new(
                "Ptek High Atten",
                0.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 10.0,
                },
            ),
            ptek_atten_freq: IntParam::new(
                "Ptek Atten Freq",
                1,
                IntRange::Linear { min: 0, max: 2 },
            ),

            // Mass 4-Band EQ Defaults (Neutral: 0.0dB gains across all bands)
            mass_low_bypass: BoolParam::new("Mass Low Bypass", false),
            mass_low_gain: FloatParam::new(
                "Mass Low Gain",
                0.0,
                FloatRange::Linear {
                    min: -9.0,
                    max: 9.0,
                },
            )
            .with_unit(" dB"),
            mass_low_freq: IntParam::new("Mass Low Freq", 2, IntRange::Linear { min: 0, max: 5 }),
            mass_low_q: FloatParam::new(
                "Mass Low Q",
                1.5,
                FloatRange::Skewed {
                    min: 0.4,
                    max: 5.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            ),
            mass_low_shelf: BoolParam::new("Mass Low Shelf", true),

            mass_lmf_bypass: BoolParam::new("Mass Low-Mid Bypass", false),
            mass_lmf_gain: FloatParam::new(
                "Mass Low-Mid Gain",
                0.0,
                FloatRange::Linear {
                    min: -9.0,
                    max: 9.0,
                },
            )
            .with_unit(" dB"),
            mass_lmf_freq: IntParam::new(
                "Mass Low-Mid Freq",
                2,
                IntRange::Linear { min: 0, max: 5 },
            ),
            mass_lmf_q: FloatParam::new(
                "Mass Low-Mid Q",
                1.5,
                FloatRange::Skewed {
                    min: 0.4,
                    max: 5.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            ),
            mass_lmf_shelf: BoolParam::new("Mass Low-Mid Shelf", false),

            mass_hmf_bypass: BoolParam::new("Mass High-Mid Bypass", false),
            mass_hmf_gain: FloatParam::new(
                "Mass High-Mid Gain",
                0.0,
                FloatRange::Linear {
                    min: -9.0,
                    max: 9.0,
                },
            )
            .with_unit(" dB"),
            mass_hmf_freq: IntParam::new(
                "Mass High-Mid Freq",
                4,
                IntRange::Linear { min: 0, max: 5 },
            ),
            mass_hmf_q: FloatParam::new(
                "Mass High-Mid Q",
                1.5,
                FloatRange::Skewed {
                    min: 0.4,
                    max: 5.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            ),
            mass_hmf_shelf: BoolParam::new("Mass High-Mid Shelf", false),

            mass_hf_bypass: BoolParam::new("Mass High Bypass", false),
            mass_hf_gain: FloatParam::new(
                "Mass High Gain",
                0.0,
                FloatRange::Linear {
                    min: -9.0,
                    max: 9.0,
                },
            )
            .with_unit(" dB"),
            mass_hf_freq: IntParam::new("Mass High Freq", 3, IntRange::Linear { min: 0, max: 5 }),
            mass_hf_q: FloatParam::new(
                "Mass High Q",
                1.5,
                FloatRange::Skewed {
                    min: 0.4,
                    max: 5.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            ),
            mass_hf_shelf: BoolParam::new("Mass High Shelf", true),

            // FAIR Legacy Defaults
            fair_threshold: FloatParam::new(
                "Fair Threshold",
                5.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 10.0,
                },
            ),
            fair_time_constant: IntParam::new(
                "Fair Time Constant",
                1,
                IntRange::Linear { min: 0, max: 5 },
            ),
            fair_mix: FloatParam::new(
                "Fair Mix",
                100.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 100.0,
                },
            )
            .with_unit(" %"),
            fair_makeup: FloatParam::new(
                "Fair Makeup",
                0.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 12.0,
                },
            )
            .with_unit(" dB"),
            fair_sc_hpf: BoolParam::new("Fair SC HPF", false),

            // V-MU Legacy Defaults
            vmu_threshold: FloatParam::new(
                "V-MU Threshold",
                5.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 10.0,
                },
            ),
            vmu_attack: IntParam::new("V-MU Attack", 2, IntRange::Linear { min: 0, max: 4 }),
            vmu_recovery: IntParam::new("V-MU Recovery", 1, IntRange::Linear { min: 0, max: 3 }),
            vmu_mode: BoolParam::new("V-MU Mode (Limit)", false),
            vmu_sc_hpf: BoolParam::new("V-MU SC HPF", true),
            vmu_mix: FloatParam::new(
                "V-MU Mix",
                100.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 100.0,
                },
            )
            .with_unit(" %"),
            vmu_makeup: FloatParam::new(
                "V-MU Makeup",
                0.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 12.0,
                },
            )
            .with_unit(" dB"),

            // EQ & Compressor Output Stage Coloration Defaults (Neutral: Bypass 0)
            eq_amp_type: IntParam::new("EQ Amp Type", 0, IntRange::Linear { min: 0, max: 6 }),
            eq_trans_type: IntParam::new(
                "EQ Transformer Type",
                0,
                IntRange::Linear { min: 0, max: 4 },
            ),
            eq_intensity: IntParam::new(
                "EQ Harmonic Drive",
                0,
                IntRange::Linear { min: 0, max: 2 },
            ),
            eq_col_comp: BoolParam::new("EQ Coloration Auto Gain", false),
            eq_col_match: BoolParam::new("EQ Coloration Hardware Match", false),
            eq_col_auto: Arc::new(Mutex::new(Vec::new())),

            // Comp defaults to Bypass (0)
            comp_amp_type: IntParam::new("Comp Amp Type", 0, IntRange::Linear { min: 0, max: 6 }),
            comp_trans_type: IntParam::new(
                "Comp Transformer Type",
                0,
                IntRange::Linear { min: 0, max: 4 },
            ),
            comp_intensity: IntParam::new(
                "Comp Harmonic Drive",
                0,
                IntRange::Linear { min: 0, max: 2 },
            ),
            comp_col_comp: BoolParam::new("Comp Coloration Auto Gain", false),
            comp_col_match: BoolParam::new("Comp Coloration Hardware Match", false),
            comp_col_auto: Arc::new(Mutex::new(Vec::new())),
            tooltips_enabled: BoolParam::new("Tooltips Enabled", true),
            theme_light: BoolParam::new("Light Theme", false),
        }
    }
}
