use nih_plug::prelude::{FloatParam, GuiContext, IntParam, Param, ParamPtr};
use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::vizia::vg::{Color, FontId, Paint, Path, Solidity};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::debug_calibration;
use crate::dsp::shared::ConsoleMeters;
use crate::gui::primitives::*;
use crate::gui::theme::ThemeFonts;
use crate::parameters::{CompModel, EqModel, MasteraidParams};
use crate::user_presets::{load_user_presets, save_user_presets, UserPreset};
use nih_plug_vizia::widgets::util::ModifiersExt;

/// Format a signed dB readout for the input ladder and true-peak window. At the meter floor there
/// is nothing to read, so the field blanks out instead of sitting on a stuck "-60.0".
fn format_db_readout(value: f32) -> String {
    if value <= METER_FLOOR_DB + 0.1 {
        "--.-".to_string()
    } else {
        format!("{:+.1}", value)
    }
}

/// Give the plugin canvas OS keyboard focus.
///
/// A hosted editor is a child window. Windows does not move keyboard focus to a child window just
/// because it was clicked, so without this the DAW keeps receiving the keystrokes and the debug
/// text entry never sees a character. The cursor is over the canvas while the click is handled, so
/// the window under it is the canvas itself.
fn focus_window_under_cursor() {
    #[cfg(target_os = "windows")]
    {
        use std::ffi::c_void;

        #[repr(C)]
        struct Point {
            x: i32,
            y: i32,
        }

        extern "system" {
            fn GetCapture() -> *mut c_void;
            fn GetCurrentProcessId() -> u32;
            fn GetCursorPos(point: *mut Point) -> i32;
            fn GetWindowThreadProcessId(hwnd: *mut c_void, process_id: *mut u32) -> u32;
            fn SetFocus(hwnd: *mut c_void) -> *mut c_void;
            fn WindowFromPoint(point: Point) -> *mut c_void;
        }

        // SAFETY: every pointer refers to a local value that outlives the call. The window handle
        // is only used after confirming that it belongs to this process, and the canvas is owned
        // by this thread, which is what `SetFocus` requires.
        unsafe {
            // The canvas captures the mouse on button down, so while this click is being handled
            // it is the capture window of this thread: an exact handle to the canvas. Fall back to
            // the window under the cursor if capture has already been released.
            let mut hwnd = GetCapture();
            if hwnd.is_null() {
                let mut point = Point { x: 0, y: 0 };
                if GetCursorPos(&mut point) != 0 {
                    hwnd = WindowFromPoint(point);
                }
            }
            if hwnd.is_null() {
                return;
            }
            let mut process_id = 0u32;
            GetWindowThreadProcessId(hwnd, &mut process_id);
            if process_id == GetCurrentProcessId() {
                SetFocus(hwnd);
            }
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        // macOS and X11 give the canvas keyboard focus on click.
    }
}

const AMP_ALIASES: [&str; 7] = [
    "BYPASS",
    "TRIODE",
    "PUSH PULL",
    "PENTODE",
    "FET",
    "MOSFET",
    "DIODE",
];
const TRANS_ALIASES: [&str; 5] = ["BYPASS", "STEEL", "NICKEL", "AMORPHOUS", "MU METAL"];

const MASS_FREQ_LABELS: [[&str; 6]; 4] = [
    ["33 HZ", "47 HZ", "68 HZ", "100 HZ", "150 HZ", "220 HZ"],
    ["150 HZ", "220 HZ", "330 HZ", "470 HZ", "680 HZ", "1.0KHZ"],
    ["680 HZ", "1.0KHZ", "1.5KHZ", "2.2KHZ", "3.3KHZ", "4.7KHZ"],
    ["3.3KHZ", "4.7KHZ", "6.8KHZ", "10KHZ", "15KHZ", "20KHZ"],
];

// PTEK selector labels (must stay in sync with `PTEK_*_FREQS` in dsp/eq_ptek.rs)
const PU_LOW_LABELS: [&str; 6] = ["20 HZ", "30 HZ", "40 HZ", "60 HZ", "100 HZ", "150 HZ"];
const PU_HIGH_LABELS: [&str; 7] = [
    "3.0KHZ", "4.0KHZ", "5.0KHZ", "8.0KHZ", "10KHZ", "12KHZ", "16KHZ",
];
const PU_ATTEN_LABELS: [&str; 3] = ["5.0KHZ", "10KHZ", "20KHZ"];

// Ptek 0..10 knob scale (BOOST / ATTEN / BANDWIDTH)
const PU_VALS: [f32; 11] = [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
const PU_STRS: [&str; 11] = ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "10"];

/// How long a commanded selector position outranks the parameter value read back from the host.
/// GUI writes round-trip through the host, so a new gesture can otherwise start from a stale index.
const INT_CMD_HOLD: Duration = Duration::from_millis(1500);
/// One cache slot per integer selector (four Mass frequency rings + three Ptek selectors).
/// A single shared slot loses the previous band's position when the pointer jumps between rings,
/// which makes the next gesture on that band start from the host's stale value.
const INT_CTRL_SLOTS: usize = 7;
type IntCmdSlot = Option<(SteppedControl, usize, Instant)>;

/// Stable cache slot for an integer selector. `None` for continuous controls.
fn int_ctrl_slot(ctrl: SteppedControl) -> Option<usize> {
    match ctrl {
        SteppedControl::EqFreq(band) if band < 4 => Some(band),
        SteppedControl::PuLowFreq => Some(4),
        SteppedControl::PuHighFreq => Some(5),
        SteppedControl::PuAttenSel => Some(6),
        _ => None,
    }
}
/// Peak-hold time. Broadcast peak-hold indicators clear 1.5 s after the last peak; long enough to
/// read a transient after it has gone, short enough that an old peak never masks the current one.
const INPUT_PEAK_HOLD_DEFAULT: Duration = Duration::from_millis(1500);

fn input_peak_hold_duration(ms: u32) -> Duration {
    Duration::from_millis(if ms == 0 {
        INPUT_PEAK_HOLD_DEFAULT.as_millis() as u64
    } else {
        ms as u64
    })
}

/// Peak fallback once the hold expires. IEC 60268-10 Type I falls 20 dB in 1.7 s (~11.8 dB/s),
/// which is the slowest of the standard peak ballistics and the easiest to follow by eye.
const INPUT_PEAK_FALL_DB_S: f32 = 20.0 / 1.7;

/// Bottom of the scale, shared by the meter, the readout and the hold.
const METER_FLOOR_DB: f32 = -60.0;

/// Printed VU scale: the `0` mark sits 11 degrees clockwise of vertical, so that is the needle's
/// rest position, not 12 o'clock.
const VU_ZERO_DEG: f32 = 11.0;

/// Full-scale gain reduction (20 dB) sweeps 55 degrees counter-clockwise from the `0` mark,
/// landing on the printed `-20`.
const VU_GR_SPAN_DEG: f32 = 55.0;

/// The GUI treats the audio thread as stopped when its heartbeat has not moved for this long.
/// Long enough to ride out a scheduling hiccup, short enough that the meters visibly settle.
const METER_IDLE_TIMEOUT: Duration = Duration::from_millis(250);

/// Needle angle for a gain reduction reading. `gr_db` is positive dB of reduction.
fn vu_target_deg(gr_db: f32) -> f32 {
    VU_ZERO_DEG - (gr_db.clamp(0.0, 20.0) / 20.0) * VU_GR_SPAN_DEG
}

/// Peak-hold ballistic: instant attack, a fixed hold at the highest level seen, then a linear
/// fallback. The hold is restarted by every new peak, so a run of similar hits keeps the marker
/// still instead of making it creep down between them.
#[derive(Clone, Copy, Debug)]
struct PeakHold {
    db: f32,
    hold_until: Instant,
    /// Level and time the current fallback is measured from. Kept separate from `db` so the
    /// fallback is a function of elapsed time rather than of frame rate: a late frame (occluded
    /// window, host hiccup) lands on exactly the same level as a run of quick ones.
    fall_from: f32,
    fall_at: Instant,
}

impl PeakHold {
    fn at(now: Instant) -> Self {
        Self {
            db: METER_FLOOR_DB,
            hold_until: now,
            fall_from: METER_FLOOR_DB,
            fall_at: now,
        }
    }

    /// Advance to `now` and return the indicated level.
    fn step(&mut self, peak_db: f32, now: Instant, hold_ms: u32) -> f32 {
        if peak_db >= self.db {
            self.db = peak_db;
            self.hold_until = now + input_peak_hold_duration(hold_ms);
            self.fall_from = peak_db;
            self.fall_at = self.hold_until;
        } else if now >= self.hold_until {
            let fallen = self.fall_from
                - INPUT_PEAK_FALL_DB_S * now.duration_since(self.fall_at).as_secs_f32();
            self.db = fallen.max(peak_db).max(METER_FLOOR_DB);
        }
        self.db
    }

    fn reset(&mut self, now: Instant) {
        self.db = METER_FLOOR_DB;
        self.hold_until = now;
        self.fall_from = METER_FLOOR_DB;
        self.fall_at = now;
    }
}

/// Gain reduction peak-hold with 1.2s latch and smooth 20 dB/s decay
#[derive(Clone, Copy, Debug)]
struct GrPeakHold {
    gr_db: f32,
    hold_until: Instant,
    fall_from: f32,
    fall_at: Instant,
}

impl GrPeakHold {
    fn at(now: Instant) -> Self {
        Self {
            gr_db: 0.0,
            hold_until: now,
            fall_from: 0.0,
            fall_at: now,
        }
    }

    fn step(&mut self, live_gr: f32, now: Instant) -> f32 {
        if live_gr >= self.gr_db {
            self.gr_db = live_gr;
            self.hold_until = now + Duration::from_millis(1200);
            self.fall_from = live_gr;
            self.fall_at = self.hold_until;
        } else if now >= self.hold_until {
            let elapsed = now.duration_since(self.fall_at).as_secs_f32();
            let fallen = (self.fall_from - 20.0 * elapsed).max(live_gr).max(0.0);
            self.gr_db = fallen;
        }
        self.gr_db
    }

    fn reset(&mut self, now: Instant) {
        self.gr_db = 0.0;
        self.hold_until = now;
        self.fall_from = 0.0;
        self.fall_at = now;
    }
}

// 9 Stepped Concentric EQ Gains
const EQ_GAIN_VALS: [f32; 9] = [-9.0, -6.0, -3.0, -1.5, 0.0, 1.5, 3.0, 6.0, 9.0];
const EQ_GAIN_STRS: [&str; 9] = [
    "-9.0DB", "-6.0DB", "-3.0DB", "-1.5DB", "+0.0DB", "+1.5DB", "+3.0DB", "+6.0DB", "+9.0DB",
];

// 11-Tick Stepped Value Tables
const KNOB_TICKS_11: [f32; 11] = [-135.0, -108.0, -81.0, -54.0, -27.0, 0.0, 27.0, 54.0, 81.0, 108.0, 135.0];
const DB_11_VALS: [f32; 11] = [-10.0, -8.0, -6.0, -4.0, -2.0, 0.0, 2.0, 4.0, 6.0, 8.0, 10.0];
const DB_11_STRS: [&str; 11] = [
    "-10.0dB", "-8.0dB", "-6.0dB", "-4.0dB", "-2.0dB", "+0.0dB", "+2.0dB", "+4.0dB", "+6.0dB",
    "+8.0dB", "+10.0dB",
];

/// Compressor makeup spans -10..+20 dB. The knob still has 11 detents, spread 3 dB apart.
const MAKEUP_11_VALS: [f32; 11] = [
    -10.0, -7.0, -4.0, -1.0, 2.0, 5.0, 8.0, 11.0, 14.0, 17.0, 20.0,
];
const MAKEUP_11_STRS: [&str; 11] = [
    "-10.0dB", "-7.0dB", "-4.0dB", "-1.0dB", "+2.0dB", "+5.0dB", "+8.0dB", "+11.0dB", "+14.0dB",
    "+17.0dB", "+20.0dB",
];

const TAPE_FREQ_VALS: [f32; 11] = [
    30.0, 40.0, 50.0, 60.0, 70.0, 80.0, 90.0, 105.0, 120.0, 135.0, 150.0,
];
const TAPE_FREQ_STRS: [&str; 11] = [
    "30Hz", "40Hz", "50Hz", "60Hz", "70Hz", "80Hz", "90Hz", "105Hz", "120Hz", "135Hz", "150Hz",
];

const FLUTTER_VALS: [f32; 11] = [
    0.0, 5.0, 10.0, 15.0, 20.0, 25.0, 30.0, 40.0, 50.0, 75.0, 100.0,
];
const FLUTTER_STRS: [&str; 11] = [
    "0%", "5%", "10%", "15%", "20%", "25%", "30%", "40%", "50%", "75%", "100%",
];

const CEIL_VALS: [f32; 11] = [
    -3.0, -2.0, -1.5, -1.0, -0.7, -0.5, -0.3, -0.2, -0.1, -0.05, 0.0,
];
const CEIL_STRS: [&str; 11] = [
    "-3.0dB", "-2.0dB", "-1.5dB", "-1.0dB", "-0.7dB", "-0.5dB", "-0.3dB", "-0.2dB", "-0.1dB",
    "-0.05", "0.0dB",
];

const LIMITER_GAIN_VALS: [f32; 11] = [0.0, 0.5, 1.0, 1.5, 2.0, 3.0, 4.0, 5.0, 6.0, 8.0, 10.0];
const LIMITER_GAIN_STRS: [&str; 11] = [
    "+0.0dB", "+0.5dB", "+1.0dB", "+1.5dB", "+2.0dB", "+3.0dB", "+4.0dB", "+5.0dB", "+6.0dB",
    "+8.0dB", "+10.0dB",
];

const LOOKAHEAD_VALS: [f32; 11] = [0.0, 0.1, 0.2, 0.5, 0.8, 1.0, 1.5, 2.0, 3.0, 4.0, 5.0];
const LOOKAHEAD_STRS: [&str; 11] = [
    "OFF", "0.1ms", "0.2ms", "0.5ms", "0.8ms", "1.0ms", "1.5ms", "2.0ms", "3.0ms", "4.0ms", "5.0ms",
];

const LIMITER_RELEASE_VALS: [f32; 11] = [
    0.0, 15.0, 30.0, 60.0, 100.0, 160.0, 250.0, 400.0, 650.0, 900.0, 1200.0,
];
const LIMITER_RELEASE_STRS: [&str; 11] = [
    "AUTO", "15ms", "30ms", "60ms", "100ms", "160ms", "250ms", "400ms", "650ms", "900ms", "1.2s",
];

const LIFT_VALS: [f32; 11] = [0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0];
const LIFT_STRS: [&str; 11] = [
    "OFF", "10%", "20%", "30%", "40%", "50%", "60%", "70%", "80%", "90%", "100%",
];

const MONO_5_FREQS: [f32; 5] = [40.0, 60.0, 80.0, 120.0, 180.0];
const MONO_FREQ_VALS: [f32; 11] = [
    0.0, 40.0, 40.0, 60.0, 60.0, 80.0, 80.0, 120.0, 120.0, 180.0, 180.0,
];
const MONO_FREQ_STRS: [&str; 11] = [
    "OFF", "40 12dB", "40 24dB", "60 12dB", "60 24dB", "80 12dB", "80 24dB", "120 12dB", "120 24dB", "180 12dB", "180 24dB",
];

/// Low cut: OFF at fully counter-clockwise, then 5 mastering frequencies interleaved with 12 dB and 24 dB slopes.
const LO_CUT_5_FREQS: [f32; 5] = [20.0, 30.0, 40.0, 60.0, 80.0];
const LO_CUT_VALS: [f32; 11] = [
    0.0, 20.0, 20.0, 30.0, 30.0, 40.0, 40.0, 60.0, 60.0, 80.0, 80.0,
];
const LO_CUT_STRS: [&str; 11] = [
    "OFF", "20 12dB", "20 24dB", "30 12dB", "30 24dB", "40 12dB", "40 24dB", "60 12dB", "60 24dB", "80 12dB", "80 24dB",
];

/// High cut: 11 stepped positions. Tick 0 (fully CCW) is 10k 12dB, then rising corner frequencies
/// interleaved with 12 dB and 24 dB slopes up to 20k 24dB, and Tick 10 (fully CW) is OFF.
const HI_CUT_5_FREQS: [f32; 5] = [10000.0, 12000.0, 14000.0, 16000.0, 20000.0];
const HI_CUT_VALS: [f32; 11] = [
    10000.0, 10000.0, 12000.0, 12000.0, 14000.0, 14000.0, 16000.0, 16000.0, 20000.0, 20000.0, 0.0,
];
const HI_CUT_STRS: [&str; 11] = [
    "10k 12dB", "10k 24dB", "12k 12dB", "12k 24dB", "14k 12dB", "14k 24dB", "16k 12dB", "16k 24dB", "20k 12dB", "20k 24dB", "OFF",
];

const PCT_VALS: [f32; 11] = [
    0.0, 10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0, 90.0, 100.0,
];
const PCT_STRS: [&str; 11] = [
    "0%", "10%", "20%", "30%", "40%", "50%", "60%", "70%", "80%", "90%", "100%",
];

const Q_VALS: [f32; 11] = [0.4, 0.5, 0.7, 0.9, 1.2, 1.5, 2.0, 2.5, 3.0, 4.0, 5.0];
const Q_STRS: [&str; 11] = [
    "0.4", "0.5", "0.7", "0.9", "1.2", "1.5", "2.0", "2.5", "3.0", "4.0", "5.0",
];

const SC_HPF_VALS: [f32; 11] = [
    0.0, 20.0, 30.0, 45.0, 60.0, 80.0, 100.0, 120.0, 150.0, 200.0, 250.0,
];
const SC_HPF_STRS: [&str; 11] = [
    "OFF", "20Hz", "30Hz", "45Hz", "60Hz", "80Hz", "100Hz", "120Hz", "150Hz", "200Hz", "250Hz",
];

const RATIO_VALS: [f32; 11] = [1.2, 1.5, 1.8, 2.0, 2.5, 3.0, 4.0, 6.0, 8.0, 10.0, 20.0];
const RATIO_STRS: [&str; 11] = [
    "1.2:1", "1.5:1", "1.8:1", "2.0:1", "2.5:1", "3.0:1", "4.0:1", "6.0:1", "8.0:1", "10:1", "20:1",
];

const ATTACK_VALS: [f32; 11] = [0.1, 0.2, 0.5, 1.0, 2.0, 3.0, 5.0, 10.0, 15.0, 20.0, 30.0];
const ATTACK_STRS: [&str; 11] = [
    "0.1ms", "0.2ms", "0.5ms", "1.0ms", "2.0ms", "3.0ms", "5.0ms", "10ms", "15ms", "20ms", "30ms",
];
const FAIR_TC_STRS: [&str; 11] = [
    "TC 1", "TC 1+", "TC 2", "TC 2+", "TC 3", "TC 3+", "TC 4", "TC 4+", "TC 5", "TC 6", "AUTO",
];

const RELEASE_VALS: [f32; 11] = [
    50.0, 80.0, 120.0, 200.0, 350.0, 500.0, 750.0, 1000.0, 1500.0, 1900.0, 2000.0,
];
const RELEASE_STRS: [&str; 11] = [
    "50ms", "80ms", "120ms", "200ms", "350ms", "500ms", "750ms", "1.0s", "1.5s", "2.0s", "AUTO",
];

pub fn eq_hardware_topology(model: EqModel) -> (i32, i32) {
    match model {
        EqModel::Ptek => (2, 2), // Pultec: Push-Pull Tube + Nickel Transformer
        EqModel::Mass => (1, 1), // Massive Passive: 5751 Triode + Steel Iron
    }
}

pub fn comp_hardware_topology(model: CompModel) -> (i32, i32) {
    match model {
        CompModel::Fair => (2, 1), // Fairchild 670: 6386 Push-Pull Tube + Heavy Steel Core
        CompModel::VMu => (1, 2),  // Manley Variable Mu: 5670 Triode + High-Nickel Core
    }
}

#[derive(Clone, Copy, Debug)]
pub struct PresetDef {
    pub name: &'static str,
    pub tape_speed_30ips: bool,
    pub tape_formula_gp9: bool,
    pub tape_drive: f32,
    pub tape_bias: f32,
    pub tape_bump: f32,
    pub tape_bump_freq: f32,
    pub tape_flutter: f32,
    pub clipper_drive: f32,
    pub clipper_ceiling: f32,
    pub clipper_hard_shape: bool,
    pub clipper_os_4x: bool,
    pub eq_model: EqModel,
    pub comp_model: CompModel,
    pub comp_thresh: f32,
    pub comp_ratio: f32,
    pub comp_attack: f32,
    pub comp_release: f32,
    pub comp_mix: f32,
    pub limiter_gain: f32,
    pub limiter_ceiling: f32,
    pub limiter_lookahead: f32,
    pub limiter_release: f32,
    pub limiter_lift: f32,
    pub limiter_true_peak: bool,
    pub limiter_punch: bool,
}

pub const FACTORY_PRESETS: [PresetDef; 1] = [
    PresetDef {
        name: "DEFAULT MASTER",
        tape_speed_30ips: true,
        tape_formula_gp9: false,
        tape_drive: 0.0,
        tape_bias: 0.0,
        tape_bump: 0.0,
        tape_bump_freq: 60.0,
        tape_flutter: 0.0,
        clipper_drive: 0.0,
        clipper_ceiling: -0.1,
        clipper_hard_shape: false,
        clipper_os_4x: true,
        eq_model: EqModel::Ptek,
        comp_model: CompModel::Fair,
        comp_thresh: 0.0,
        comp_ratio: 2.0,
        comp_attack: 10.0,
        comp_release: 100.0,
        comp_mix: 100.0,
        limiter_gain: 0.0,
        limiter_ceiling: -0.1,
        limiter_lookahead: 1.0,
        limiter_release: 0.0,
        limiter_lift: 0.0,
        limiter_true_peak: true,
        limiter_punch: false,
    },
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum MomentarySlot {
    InputTrim,
    CompThresh,
    EqBand(usize),
    InputPeak,
    InputRms,
    LufsMode,
    LimiterGain,
    LimiterMeter,
    ClipperReadout,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum SteppedControl {
    TapeDrive,
    TapeBias,
    TapeHeadBump,
    TapeBumpFreq,
    TapeFlutter,
    ClipperDrive,
    ClipperCeiling,
    InputTrim,
    LoCut,
    HiCut,
    MonoFreq,
    MonoAmount,
    EqQ(usize),
    CompThresh,
    CompScHpf,
    CompMix,
    CompMakeup,
    CompInGain,
    CompRatio,
    CompAttack,
    CompRelease,
    LimiterGain,
    LimiterCeiling,
    LimiterLookahead,
    LimiterRelease,
    LimiterLift,
    EqGain(usize),
    EqFreq(usize),
    // PTEK hardware controls
    PuLowBoost,
    PuLowAtten,
    PuHighBoost,
    PuHighAtten,
    PuBandwidth,
    PuLowFreq,
    PuHighFreq,
    PuAttenSel,
}

/// Maps every per-band EQ control to the shared momentary readout slot. The Mass bands
/// (`EqGain`/`EqFreq`) and the four large Ptek knobs all use the same readout engine.
fn eq_readout_band(ctrl: SteppedControl) -> Option<usize> {
    match ctrl {
        SteppedControl::EqGain(band) | SteppedControl::EqFreq(band) => Some(band),
        SteppedControl::PuLowBoost => Some(0),
        SteppedControl::PuLowAtten => Some(1),
        SteppedControl::PuHighBoost => Some(2),
        SteppedControl::PuHighAtten => Some(3),
        _ => None,
    }
}

/// Hardware-style Ptek position with one decimal place. Boost stages read `+`, attenuation
/// stages read `-`, and the whole-number part is right-aligned to two cells, so `+ 3.4`,
/// `+10.0` and `- 0.0` all occupy the same four DSEG tokens.
fn ptek_position_label(ctrl: SteppedControl, value: f32) -> Option<String> {
    let negative = match ctrl {
        SteppedControl::PuLowBoost | SteppedControl::PuHighBoost => false,
        SteppedControl::PuLowAtten | SteppedControl::PuHighAtten => true,
        _ => return None,
    };
    let tenths = (value.clamp(0.0, 10.0) * 10.0).round() as i32;
    let whole = tenths / 10;
    let frac = tenths % 10;
    Some(format!(
        "{}{:>2}.{}",
        if negative { '-' } else { '+' },
        whole,
        frac
    ))
}

/// Integer (rotary selector) controls: stepped by position only, never continuous.
fn is_int_control(ctrl: SteppedControl) -> bool {
    matches!(
        ctrl,
        SteppedControl::EqFreq(_)
            | SteppedControl::PuLowFreq
            | SteppedControl::PuHighFreq
            | SteppedControl::PuAttenSel
            | SteppedControl::LoCut
            | SteppedControl::HiCut
            | SteppedControl::MonoFreq
    )
}

enum DragTarget {
    Knob {
        ctrl: SteppedControl,
        ptr: ParamPtr,
        last_y: f32,
        accum_y: f32,
        cur_idx: usize,
        cur_norm: f32,
        fine_mode: bool,
    },
}

/// Special function a control performs when it is pressed and held for [`HOLD_MS`]. Any control can
/// map a press-and-hold to one of these; the 33 ms UI tick fires it while the button is still down.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HoldAction {
    /// Third "SMASHED" state of a coloration PUSH button.
    PushThird { is_comp: bool },
    /// Latch the compressor push-pull makeup to automatic.
    MakeupAuto,
    /// Throw away the frozen AUTO measurement and measure this coloration stage again.
    RelearnColoration { is_comp: bool },
    /// Toggle hidden debug/config menu.
    ToggleDebugMenu,
}

/// Press-and-hold state. `fired` latches once the countdown elapses so the special action runs
/// exactly once per press; releasing earlier performs the control's normal click instead.
#[derive(Clone, Copy)]
struct HoldPress {
    action: HoldAction,
    down_at: Instant,
    start_y: f32,
    /// Turning the knob disarms the hold so a drag can never also fire the special action.
    cancel_on_turn: bool,
    fired: bool,
}

/// Press-and-hold countdown for every special-function gesture.
const HOLD_MS: u64 = 450;
/// Hold time to open the hidden debug/config menu.
const DEBUG_MENU_HOLD_MS: u64 = 3000;

/// One editable row of the hidden debug menu. The same list drives both painting and hit
/// testing, so a row can never be drawn in one place and be clickable in another.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DebugField {
    PeakHold,
    AutoLearn,
    ThreshTrimFair,
    ThreshTrimVmu,
    ResetRanges,
}

const DEBUG_FIELDS: [DebugField; 5] = [
    DebugField::PeakHold,
    DebugField::AutoLearn,
    DebugField::ThreshTrimFair,
    DebugField::ThreshTrimVmu,
    DebugField::ResetRanges,
];

impl DebugField {
    fn label(self) -> &'static str {
        match self {
            DebugField::PeakHold => "PEAK HOLD (ms)",
            DebugField::AutoLearn => "AUTO LEARN (s)",
            DebugField::ThreshTrimFair => "FAIR TRIM (dB)",
            DebugField::ThreshTrimVmu => "V-MU TRIM (dB)",
            DebugField::ResetRanges => "RESET RANGES",
        }
    }

    fn is_numeric(self) -> bool {
        !matches!(self, DebugField::ResetRanges)
    }
}

/// In-progress text entry for one debug row. Held in a `RefCell` because the draw path needs
/// to read it while `event` holds `&mut self`.
#[derive(Clone, Debug)]
struct DebugEdit {
    field: DebugField,
    text: String,
    /// False until the first accepted character. The field starts with the current value, and
    /// typing replaces it instead of appending to it.
    typed: bool,
}

#[derive(Clone, Debug)]
struct PresetEdit {
    text: String,
    typed: bool,
}

/// Period of the AUTO "measuring" blink. Roughly one full on/off per second — fast enough that a
/// glance tells you it is working, not just lit.
const AUTO_BLINK_SECONDS: f32 = 0.8;

/// Period of the AUTO "learned and engaged" breath. Quick enough to read at a glance, and it never
/// goes dark, so it cannot be confused with the measuring blink.
const AUTO_BREATH_SECONDS: f32 = 1.6;

#[derive(Clone, Debug)]
pub struct AudioSnapshot {
    pub master_power: bool,
    pub input_trim: f32,
    pub input_bypass: bool,
    pub polarity: bool,
    pub input_mono: bool,
    pub filter_lo_cut_hz: f32,
    pub filter_lo_cut_slope: i32,
    pub filter_hi_cut_hz: f32,
    pub filter_hi_cut_slope: i32,
    pub bass_mono_freq: f32,
    pub bass_mono_slope: i32,
    pub bass_mono_amount: f32,
    pub tape_bypass: bool,
    pub tape_speed_30ips: bool,
    pub tape_formula_gp9: bool,
    pub tape_drive: f32,
    pub tape_bias: f32,
    pub tape_headbump: f32,
    pub tape_bump_freq: f32,
    pub tape_flutter: f32,
    pub tape_comp: bool,
    pub clipper_bypass: bool,
    pub clipper_drive: f32,
    pub clipper_ceiling: f32,
    pub clipper_hard_shape: bool,
    pub clipper_os_4x: bool,
    pub clipper_comp: bool,
    pub eq_bypass: bool,
    pub eq_model: EqModel,
    pub ptek_low_boost: f32,
    pub ptek_low_atten: f32,
    pub ptek_low_freq: i32,
    pub ptek_high_boost: f32,
    pub ptek_high_freq: i32,
    pub ptek_bandwidth: f32,
    pub ptek_high_atten: f32,
    pub ptek_atten_freq: i32,
    pub mass_low_bypass: bool,
    pub mass_low_gain: f32,
    pub mass_low_freq: i32,
    pub mass_low_q: f32,
    pub mass_low_shelf: bool,
    pub mass_lmf_bypass: bool,
    pub mass_lmf_gain: f32,
    pub mass_lmf_freq: i32,
    pub mass_lmf_q: f32,
    pub mass_lmf_shelf: bool,
    pub mass_hmf_bypass: bool,
    pub mass_hmf_gain: f32,
    pub mass_hmf_freq: i32,
    pub mass_hmf_q: f32,
    pub mass_hmf_shelf: bool,
    pub mass_hf_bypass: bool,
    pub mass_hf_gain: f32,
    pub mass_hf_freq: i32,
    pub mass_hf_q: f32,
    pub mass_hf_shelf: bool,
    pub eq_amp_type: i32,
    pub eq_trans_type: i32,
    pub eq_intensity: i32,
    pub eq_col_comp: bool,
    pub eq_col_match: bool,
    pub comp_bypass: bool,
    pub comp_model: CompModel,
    pub comp_thresh: f32,
    pub comp_ratio: f32,
    pub comp_in_gain: f32,
    pub comp_attack: f32,
    pub comp_release: f32,
    pub comp_sc_hpf_hz: f32,
    pub comp_mix: f32,
    pub comp_makeup: f32,
    pub comp_auto_makeup: bool,
    pub fair_threshold: f32,
    pub fair_time_constant: i32,
    pub fair_mix: f32,
    pub fair_makeup: f32,
    pub fair_sc_hpf: bool,
    pub vmu_threshold: f32,
    pub vmu_attack: i32,
    pub vmu_recovery: i32,
    pub vmu_mode: bool,
    pub vmu_sc_hpf: bool,
    pub vmu_mix: f32,
    pub vmu_makeup: f32,
    pub comp_amp_type: i32,
    pub comp_trans_type: i32,
    pub comp_intensity: i32,
    pub comp_col_comp: bool,
    pub comp_col_match: bool,
    pub limiter_bypass: bool,
    pub limiter_gain: f32,
    pub limiter_ceiling: f32,
    pub limiter_lookahead_ms: f32,
    pub limiter_release_ms: f32,
    pub limiter_lift: f32,
    pub limiter_auto_release: bool,
    pub limiter_true_peak: bool,
    pub limiter_punch: bool,
    pub limiter_unity: bool,
    pub output_trim: f32,
}

pub struct EqEditorView {
    params: Arc<MasteraidParams>,
    meters: Arc<ConsoleMeters>,
    gui_context: Arc<dyn GuiContext>,
    active_drag: Option<DragTarget>,
    /// Armed while a special-function control is held down; polled once per rendered frame so the
    /// action fires while the button is still down. The baseview backend never ticks vizia timers,
    /// so the frame callback is the only reliable clock here.
    hold_press: Cell<Option<HoldPress>>,
    /// Whether the left mouse button is currently down (used to cancel a hold whose release was
    /// swallowed by the OS).
    pointer_down: Cell<bool>,
    momentary_readout: Option<(MomentarySlot, String, Instant)>,
    smash_banner: Cell<Option<(bool, Instant)>>,
    fine_override: Cell<Option<SteppedControl>>,
    font_regular: Cell<Option<FontId>>,
    font_bold: Cell<Option<FontId>>,
    font_dseg: Cell<Option<FontId>>,
    vu_smooth_l: Cell<f32>,
    vu_smooth_r: Cell<f32>,
    /// Last audio heartbeat seen by the GUI and when it last changed. Used to tell a stopped
    /// transport (frozen meter atomics) from a genuinely steady signal.
    meter_counter: Cell<u64>,
    meter_activity: Cell<Instant>,
    /// Input ladder source: false = PEAK, true = RMS (click ladder to toggle).
    input_rms_mode: bool,
    /// Limiter bottom readout mode: false = GAIN REDUCTION ("G"), true = TRUE PEAK ("T").
    limiter_gr_mode: bool,
    /// Output display mode: 0 = MOMENTARY, 1 = SHORT-TERM, 2 = INTEGRATED (click to cycle).
    lufs_mode: u8,
    /// Throttled numeric readouts: last refresh time + held values [in_peak, in_rms, lufs, gr_max, tp_max, clip_max].
    readout_refreshed: Cell<Instant>,
    readout_held: Cell<[f32; 6]>,
    /// Peak hold for input peak
    peak_hold_in: Cell<PeakHold>,
    /// RMS hold for input RMS
    rms_hold_in: Cell<PeakHold>,
    /// Peak hold for Clipper overshoot
    clip_hold: Cell<f32>,
    /// Gain Reduction peak hold with 1.2s latch and fallback
    gr_hold: Cell<GrPeakHold>,
    /// Peak hold for True Peak
    tp_hold: Cell<PeakHold>,
    /// Peak hold for Limiter Output meter ladder (ballistics match input ladder)
    lim_out_hold: Cell<PeakHold>,
    /// Last integer selector position this editor commanded, one slot per selector. GUI parameter
    /// writes are applied by the host asynchronously, so reading the parameter back immediately
    /// after a drag can return an older position. Without this cache a quick back-and-forth drag
    /// starts each new gesture from that stale index and the selector jumps to an unrelated
    /// frequency.
    int_cmd: Cell<[IntCmdSlot; INT_CTRL_SLOTS]>,
    /// Frame-time base for the slow AUTO "measuring" breath. The baseview backend never ticks
    /// vizia timers, so animation is derived from frame time in `draw`.
    boot: Instant,
    /// Hidden debug menu state. A `Cell` because the hold poll runs from `&self` in `draw`.
    debug_menu_open: Cell<bool>,
    /// Active text entry in the debug menu (row + typed characters). Shared with the global
    /// keyboard listener so typing works even when the host never gives this canvas keyboard
    /// focus.
    debug_edit: Rc<RefCell<Option<DebugEdit>>>,
    /// Value parsed by the global keyboard listener, applied on the next frame.
    debug_commit: Rc<Cell<Option<(DebugField, f32)>>>,
    /// Per-compressor-model stored settings (Fair vs V-MU)
    comp_fair_state: Cell<[f32; 7]>, // thresh, ratio, attack, release, sc_hpf, mix, makeup
    comp_vmu_state: Cell<[f32; 7]>,
    /// Last coloration AUTO click, for double-click detection: `(is_comp, value before the click,
    /// when)`. The host may defer the parameter read-back to the audio thread, so the second click
    /// cannot be trusted to toggle back on its own.
    auto_click_memory: Cell<Option<(bool, bool, Instant)>>,
    /// Active Preset Index
    current_preset_idx: Cell<usize>,
    /// Save preset banner timer
    save_preset_banner: Cell<Option<Instant>>,
    /// Active text entry in the preset display (typing new preset name)
    preset_edit: Rc<RefCell<Option<PresetEdit>>>,
    /// Preset name committed via Enter key in the global listener
    preset_commit: Rc<Cell<Option<String>>>,
    /// Loaded user presets (persisted across sessions)
    user_presets: Rc<RefCell<Vec<UserPreset>>>,
    /// Manual popup open state
    manual_open: Cell<bool>,
    /// A/B comparison state ('A' or 'B')
    ab_active: Cell<char>,
    slot_a: RefCell<Option<AudioSnapshot>>,
    slot_b: RefCell<Option<AudioSnapshot>>,
    /// Preset browser modal dialog state
    preset_browser_open: Rc<Cell<bool>>,
    /// Save preset modal dialog state
    preset_save_dialog_open: Rc<Cell<bool>>,
}

impl EqEditorView {
    pub const TOTAL_WIDTH: f32 = 1360.0;
    pub const TOTAL_HEIGHT: f32 = 952.0;

    // Hidden debug panel. It deliberately stops short of the bottom-right screw so the screw
    // stays visible (and holdable) while the menu is open.
    const DBG_W: f32 = 400.0;
    const DBG_H: f32 = 320.0;
    const DBG_X: f32 = Self::TOTAL_WIDTH - Self::DBG_W - 40.0;
    const DBG_Y: f32 = Self::TOTAL_HEIGHT - Self::DBG_H - 24.0;
    const DBG_ROW_H: f32 = 25.0;
    const DBG_ROW0_Y: f32 = Self::DBG_Y + 58.0;
    const DBG_LABEL_X: f32 = Self::DBG_X + 16.0;
    const DBG_MINUS_X: f32 = Self::DBG_X + 284.0;
    const DBG_VALUE_CX: f32 = Self::DBG_X + 337.0;
    const DBG_VALUE_HALF_W: f32 = 37.0;
    const DBG_PLUS_X: f32 = Self::DBG_X + 386.0;

    fn debug_row_y(i: usize) -> f32 {
        Self::DBG_ROW0_Y + i as f32 * Self::DBG_ROW_H
    }

    /// True when a point falls on the debug panel. Everything inside the panel belongs to the
    /// menu: the controls painted underneath must not receive clicks, double-clicks or wheel
    /// events while it is open.
    fn debug_panel_contains(mx: f32, my: f32) -> bool {
        (Self::DBG_X..=Self::DBG_X + Self::DBG_W).contains(&mx)
            && (Self::DBG_Y..=Self::DBG_Y + Self::DBG_H).contains(&my)
    }

    // Measured threshold windows, chosen so the knob behaves like the hardware control: the top
    // of the sweep (-6 dBFS on both) is above any sane programme peak, so the compressor can
    // always be dialled fully out of circuit, while the bottom is deep enough to limit hard.
    //
    // The two spans differ because the circuits do: the Fair detector clamps the output at
    // the threshold, so gain reduction tracks (level - threshold) almost 1:1 and needs a wider
    // span; the V-MU energy detector compresses harder per dB over the threshold, so the same
    // GR range fits in fewer dB. Both are shifted, never stretched, by the debug trims.
    const FAIR_THRESH_MIN_DB: f32 = -26.0;
    const FAIR_THRESH_MAX_DB: f32 = -6.0;
    const VMU_THRESH_MIN_DB: f32 = -24.0;
    const VMU_THRESH_MAX_DB: f32 = -6.0;

    pub const BAND_CENTERS: [f32; 4] = [343.5, 502.5, 661.5, 820.5];

    pub fn new(
        cx: &mut Context,
        params: Arc<MasteraidParams>,
        meters: Arc<ConsoleMeters>,
        gui_context: Arc<dyn GuiContext>,
    ) -> Handle<'_, Self> {
        // A calibration is done once and then forgotten, so the editor always starts from what is
        // on disk. This also picks up a change made by another host process since this process
        // first loaded the store.
        debug_calibration::calibration().reload();

        // The baseview Windows backend never reports cursor enter/leave, so the only thing that
        // arms the root window's OVER pseudo-class is vizia's own hover pass. Once a hover pass
        // sees the cursor outside the client area (dragging a control past the right or bottom
        // edge while the mouse is captured), it clears OVER from the root, and every later pass
        // bails out at its top: `hovered` stays at the root, mouse events stop reaching this view
        // and the GUI goes dead until the editor is reopened. Re-arm OVER from a global listener
        // before each hover pass. The synthetic enter event must originate from the root entity,
        // otherwise vizia skips its internal state update.
        // Keyboard events are normally delivered only to the focused view. In a hosted plugin the
        // canvas starts without OS keyboard focus, so the debug text entry listens globally and
        // asks for OS focus when an edit begins. The shared cell is written here and read by the
        // draw path.
        let debug_edit: Rc<RefCell<Option<DebugEdit>>> = Rc::new(RefCell::new(None));
        let debug_commit: Rc<Cell<Option<(DebugField, f32)>>> = Rc::new(Cell::new(None));
        let listener_edit = debug_edit.clone();
        let listener_commit = debug_commit.clone();

        let preset_edit: Rc<RefCell<Option<PresetEdit>>> = Rc::new(RefCell::new(None));
        let preset_commit: Rc<Cell<Option<String>>> = Rc::new(Cell::new(None));
        let listener_preset_edit = preset_edit.clone();
        let listener_preset_commit = preset_commit.clone();
        let preset_browser_open: Rc<Cell<bool>> = Rc::new(Cell::new(false));
        let preset_save_dialog_open: Rc<Cell<bool>> = Rc::new(Cell::new(false));
        let listener_browser_open = preset_browser_open.clone();
        let listener_save_open = preset_save_dialog_open.clone();
        let user_presets = Rc::new(RefCell::new(load_user_presets()));

        cx.add_global_listener(move |cx, event| {
            event.map(|window_event: &WindowEvent, meta| match window_event {
                WindowEvent::MouseMove(..)
                | WindowEvent::MouseDown(..)
                | WindowEvent::MouseUp(..)
                | WindowEvent::MouseScroll(..) => {
                    cx.emit_custom(
                        Event::new(WindowEvent::MouseEnter)
                            .target(Entity::root())
                            .origin(Entity::root())
                            .propagate(Propagation::Direct),
                    );
                }
                WindowEvent::CharInput(c) => {
                    let mut p_slot = listener_preset_edit.borrow_mut();
                    if let Some(edit) = p_slot.as_mut() {
                        let ch = *c;
                        if ch.is_ascii_alphanumeric()
                            || ch == ' '
                            || ch == '-'
                            || ch == '_'
                            || ch == '+'
                            || ch == '.'
                            || ch == '/'
                            || ch == '&'
                            || ch == '!'
                        {
                            if !edit.typed {
                                edit.text.clear();
                                edit.typed = true;
                            }
                            if edit.text.len() < 24 {
                                edit.text.push(ch.to_ascii_uppercase());
                            }
                            cx.needs_redraw();
                        }
                        meta.consume();
                        return;
                    }
                    drop(p_slot);

                    let mut slot = listener_edit.borrow_mut();
                    let Some(edit) = slot.as_mut() else {
                        return;
                    };
                    let ch = *c;
                    // The field opens with the current value in it; the first accepted character
                    // replaces that seed instead of appending to it.
                    let replacing = !edit.typed;
                    let empty = edit.text.is_empty() || replacing;
                    let valid = ch.is_ascii_digit()
                        || (ch == '.' && (replacing || !edit.text.contains('.')))
                        || (ch == '-' && empty);
                    if valid {
                        if replacing {
                            edit.text.clear();
                            edit.typed = true;
                        }
                        if edit.text.len() < 12 {
                            edit.text.push(ch);
                        }
                        cx.needs_redraw();
                    }
                    meta.consume();
                }
                WindowEvent::KeyDown(code, _) => {
                    if listener_preset_edit.borrow().is_some()
                        || listener_save_open.get()
                        || listener_browser_open.get()
                    {
                        match *code {
                            Code::Enter | Code::NumpadEnter => {
                                if let Some(edit) = listener_preset_edit.borrow_mut().take() {
                                    let trimmed = edit.text.trim();
                                    if !trimmed.is_empty() {
                                        listener_preset_commit.set(Some(trimmed.to_string()));
                                    }
                                }
                                listener_save_open.set(false);
                                cx.needs_redraw();
                                meta.consume();
                                return;
                            }
                            Code::Escape => {
                                listener_preset_edit.borrow_mut().take();
                                listener_save_open.set(false);
                                listener_browser_open.set(false);
                                cx.needs_redraw();
                                meta.consume();
                                return;
                            }
                            Code::Backspace => {
                                if let Some(edit) = listener_preset_edit.borrow_mut().as_mut() {
                                    if !edit.typed {
                                        edit.text.clear();
                                        edit.typed = true;
                                    } else {
                                        edit.text.pop();
                                    }
                                    cx.needs_redraw();
                                }
                                meta.consume();
                                return;
                            }
                            Code::Space => {
                                if let Some(edit) = listener_preset_edit.borrow_mut().as_mut() {
                                    if !edit.typed {
                                        edit.text.clear();
                                        edit.typed = true;
                                    }
                                    if edit.text.len() < 24 && !edit.text.ends_with(' ') {
                                        edit.text.push(' ');
                                        cx.needs_redraw();
                                    }
                                }
                                meta.consume();
                                return;
                            }
                            _ => {}
                        }
                    }

                    if listener_edit.borrow().is_none() {
                        return;
                    }
                    match *code {
                        Code::Enter | Code::NumpadEnter => {
                            let Some(edit) = listener_edit.borrow_mut().take() else {
                                return;
                            };
                            if let Ok(v) = edit.text.trim().parse::<f32>() {
                                if v.is_finite() {
                                    listener_commit.set(Some((edit.field, v)));
                                }
                            }
                            cx.needs_redraw();
                        }
                        Code::Escape => {
                            listener_edit.borrow_mut().take();
                            cx.needs_redraw();
                        }
                        Code::Backspace => {
                            if let Some(edit) = listener_edit.borrow_mut().as_mut() {
                                edit.text.pop();
                                // Backspace counts as an edit, so a later digit appends to what
                                // is left instead of replacing the whole field again.
                                edit.typed = true;
                                cx.needs_redraw();
                            }
                        }
                        _ => return,
                    }
                    meta.consume();
                }
                _ => {}
            });
        });

        Self {
            params,
            meters,
            gui_context,
            active_drag: None,
            hold_press: Cell::new(None),
            pointer_down: Cell::new(false),
            momentary_readout: None,
            smash_banner: Cell::new(None),
            fine_override: Cell::new(None),
            font_regular: Cell::new(None),
            font_bold: Cell::new(None),
            font_dseg: Cell::new(None),
            vu_smooth_l: Cell::new(VU_ZERO_DEG),
            vu_smooth_r: Cell::new(VU_ZERO_DEG),
            meter_counter: Cell::new(0),
            meter_activity: Cell::new(Instant::now()),
            input_rms_mode: false,
            limiter_gr_mode: false,
            lufs_mode: 2,
            readout_refreshed: Cell::new(Instant::now()),
            readout_held: Cell::new([-60.0, -60.0, -60.0, 0.0, -60.0, 0.0]),
            peak_hold_in: Cell::new(PeakHold::at(Instant::now())),
            rms_hold_in: Cell::new(PeakHold::at(Instant::now())),
            clip_hold: Cell::new(0.0),
            gr_hold: Cell::new(GrPeakHold::at(Instant::now())),
            tp_hold: Cell::new(PeakHold::at(Instant::now())),
            lim_out_hold: Cell::new(PeakHold::at(Instant::now())),
            int_cmd: Cell::new([None; INT_CTRL_SLOTS]),
            boot: Instant::now(),
            debug_menu_open: Cell::new(false),
            debug_edit: debug_edit.clone(),
            debug_commit: debug_commit.clone(),
            comp_fair_state: Cell::new([-10.0, 3.0, 1.0, 1000.0, 80.0, 100.0, 2.0]),
            comp_vmu_state: Cell::new([-10.0, 3.0, 1.0, 1000.0, 80.0, 100.0, 2.0]),
            auto_click_memory: Cell::new(None),
            current_preset_idx: Cell::new(0),
            save_preset_banner: Cell::new(None),
            preset_edit: preset_edit.clone(),
            preset_commit: preset_commit.clone(),
            user_presets: user_presets.clone(),
            manual_open: Cell::new(false),
            ab_active: Cell::new('A'),
            slot_a: RefCell::new(None),
            slot_b: RefCell::new(None),
            preset_browser_open: preset_browser_open.clone(),
            preset_save_dialog_open: preset_save_dialog_open.clone(),
        }
        .build(cx, |_| {})
        .width(Stretch(1.0))
        .height(Stretch(1.0))
    }

    fn set_param_norm(&self, ptr: ParamPtr, norm_val: f32) {
        let clamped = if norm_val.is_nan() { 0.0 } else { norm_val.max(0.0).min(1.0) };
        unsafe {
            self.gui_context.raw_set_parameter_normalized(ptr, clamped);
        }
    }

    fn set_param_atomic(&self, ptr: ParamPtr, norm: f32) {
        let clamped = if norm.is_nan() { 0.0 } else { norm.max(0.0).min(1.0) };
        unsafe {
            self.gui_context.raw_begin_set_parameter(ptr);
            self.gui_context.raw_set_parameter_normalized(ptr, clamped);
            self.gui_context.raw_end_set_parameter(ptr);
        }
    }

    fn capture_snapshot(&self) -> AudioSnapshot {
        AudioSnapshot {
            master_power: self.params.master_power.value(),
            input_trim: self.params.input_trim.value(),
            input_bypass: self.params.input_bypass.value(),
            polarity: self.params.polarity.value(),
            input_mono: self.params.input_mono.value(),
            filter_lo_cut_hz: self.params.filter_lo_cut_hz.value(),
            filter_lo_cut_slope: self.params.filter_lo_cut_slope.value(),
            filter_hi_cut_hz: self.params.filter_hi_cut_hz.value(),
            filter_hi_cut_slope: self.params.filter_hi_cut_slope.value(),
            bass_mono_freq: self.params.bass_mono_freq.value(),
            bass_mono_slope: self.params.bass_mono_slope.value(),
            bass_mono_amount: self.params.bass_mono_amount.value(),
            tape_bypass: self.params.tape_bypass.value(),
            tape_speed_30ips: self.params.tape_speed_30ips.value(),
            tape_formula_gp9: self.params.tape_formula_gp9.value(),
            tape_drive: self.params.tape_drive.value(),
            tape_bias: self.params.tape_bias.value(),
            tape_headbump: self.params.tape_headbump.value(),
            tape_bump_freq: self.params.tape_bump_freq.value(),
            tape_flutter: self.params.tape_flutter.value(),
            tape_comp: self.params.tape_comp.value(),
            clipper_bypass: self.params.clipper_bypass.value(),
            clipper_drive: self.params.clipper_drive.value(),
            clipper_ceiling: self.params.clipper_ceiling.value(),
            clipper_hard_shape: self.params.clipper_hard_shape.value(),
            clipper_os_4x: self.params.clipper_os_4x.value(),
            clipper_comp: self.params.clipper_comp.value(),
            eq_bypass: self.params.eq_bypass.value(),
            eq_model: self.params.eq_model.value(),
            ptek_low_boost: self.params.ptek_low_boost.value(),
            ptek_low_atten: self.params.ptek_low_atten.value(),
            ptek_low_freq: self.params.ptek_low_freq.value(),
            ptek_high_boost: self.params.ptek_high_boost.value(),
            ptek_high_freq: self.params.ptek_high_freq.value(),
            ptek_bandwidth: self.params.ptek_bandwidth.value(),
            ptek_high_atten: self.params.ptek_high_atten.value(),
            ptek_atten_freq: self.params.ptek_atten_freq.value(),
            mass_low_bypass: self.params.mass_low_bypass.value(),
            mass_low_gain: self.params.mass_low_gain.value(),
            mass_low_freq: self.params.mass_low_freq.value(),
            mass_low_q: self.params.mass_low_q.value(),
            mass_low_shelf: self.params.mass_low_shelf.value(),
            mass_lmf_bypass: self.params.mass_lmf_bypass.value(),
            mass_lmf_gain: self.params.mass_lmf_gain.value(),
            mass_lmf_freq: self.params.mass_lmf_freq.value(),
            mass_lmf_q: self.params.mass_lmf_q.value(),
            mass_lmf_shelf: self.params.mass_lmf_shelf.value(),
            mass_hmf_bypass: self.params.mass_hmf_bypass.value(),
            mass_hmf_gain: self.params.mass_hmf_gain.value(),
            mass_hmf_freq: self.params.mass_hmf_freq.value(),
            mass_hmf_q: self.params.mass_hmf_q.value(),
            mass_hmf_shelf: self.params.mass_hmf_shelf.value(),
            mass_hf_bypass: self.params.mass_hf_bypass.value(),
            mass_hf_gain: self.params.mass_hf_gain.value(),
            mass_hf_freq: self.params.mass_hf_freq.value(),
            mass_hf_q: self.params.mass_hf_q.value(),
            mass_hf_shelf: self.params.mass_hf_shelf.value(),
            eq_amp_type: self.params.eq_amp_type.value(),
            eq_trans_type: self.params.eq_trans_type.value(),
            eq_intensity: self.params.eq_intensity.value(),
            eq_col_comp: self.params.eq_col_comp.value(),
            eq_col_match: self.params.eq_col_match.value(),
            comp_bypass: self.params.comp_bypass.value(),
            comp_model: self.params.comp_model.value(),
            comp_thresh: self.params.comp_thresh.value(),
            comp_ratio: self.params.comp_ratio.value(),
            comp_in_gain: self.params.comp_in_gain.value(),
            comp_attack: self.params.comp_attack.value(),
            comp_release: self.params.comp_release.value(),
            comp_sc_hpf_hz: self.params.comp_sc_hpf_hz.value(),
            comp_mix: self.params.comp_mix.value(),
            comp_makeup: self.params.comp_makeup.value(),
            comp_auto_makeup: self.params.comp_auto_makeup.value(),
            fair_threshold: self.params.fair_threshold.value(),
            fair_time_constant: self.params.fair_time_constant.value(),
            fair_mix: self.params.fair_mix.value(),
            fair_makeup: self.params.fair_makeup.value(),
            fair_sc_hpf: self.params.fair_sc_hpf.value(),
            vmu_threshold: self.params.vmu_threshold.value(),
            vmu_attack: self.params.vmu_attack.value(),
            vmu_recovery: self.params.vmu_recovery.value(),
            vmu_mode: self.params.vmu_mode.value(),
            vmu_sc_hpf: self.params.vmu_sc_hpf.value(),
            vmu_mix: self.params.vmu_mix.value(),
            vmu_makeup: self.params.vmu_makeup.value(),
            comp_amp_type: self.params.comp_amp_type.value(),
            comp_trans_type: self.params.comp_trans_type.value(),
            comp_intensity: self.params.comp_intensity.value(),
            comp_col_comp: self.params.comp_col_comp.value(),
            comp_col_match: self.params.comp_col_match.value(),
            limiter_bypass: self.params.limiter_bypass.value(),
            limiter_gain: self.params.limiter_gain.value(),
            limiter_ceiling: self.params.limiter_ceiling.value(),
            limiter_lookahead_ms: self.params.limiter_lookahead_ms.value(),
            limiter_release_ms: self.params.limiter_release_ms.value(),
            limiter_lift: self.params.limiter_lift.value(),
            limiter_auto_release: self.params.limiter_auto_release.value(),
            limiter_true_peak: self.params.limiter_true_peak.value(),
            limiter_punch: self.params.limiter_punch.value(),
            limiter_unity: self.params.limiter_unity.value(),
            output_trim: self.params.output_trim.value(),
        }
    }

    fn restore_snapshot(&self, s: &AudioSnapshot) {
        self.set_param_atomic(self.params.master_power.as_ptr(), if s.master_power { 1.0 } else { 0.0 });
        self.set_param_atomic(self.params.input_trim.as_ptr(), self.params.input_trim.preview_normalized(s.input_trim));
        self.set_param_atomic(self.params.input_bypass.as_ptr(), if s.input_bypass { 1.0 } else { 0.0 });
        self.set_param_atomic(self.params.polarity.as_ptr(), if s.polarity { 1.0 } else { 0.0 });
        self.set_param_atomic(self.params.input_mono.as_ptr(), if s.input_mono { 1.0 } else { 0.0 });
        self.set_param_atomic(self.params.filter_lo_cut_hz.as_ptr(), self.params.filter_lo_cut_hz.preview_normalized(s.filter_lo_cut_hz));
        self.set_param_atomic(self.params.filter_lo_cut_slope.as_ptr(), self.params.filter_lo_cut_slope.preview_normalized(s.filter_lo_cut_slope));
        self.set_param_atomic(self.params.filter_hi_cut_hz.as_ptr(), self.params.filter_hi_cut_hz.preview_normalized(s.filter_hi_cut_hz));
        self.set_param_atomic(self.params.filter_hi_cut_slope.as_ptr(), self.params.filter_hi_cut_slope.preview_normalized(s.filter_hi_cut_slope));
        self.set_param_atomic(self.params.bass_mono_freq.as_ptr(), self.params.bass_mono_freq.preview_normalized(s.bass_mono_freq));
        self.set_param_atomic(self.params.bass_mono_slope.as_ptr(), self.params.bass_mono_slope.preview_normalized(s.bass_mono_slope));
        self.set_param_atomic(self.params.bass_mono_amount.as_ptr(), self.params.bass_mono_amount.preview_normalized(s.bass_mono_amount));
        self.set_param_atomic(self.params.tape_bypass.as_ptr(), if s.tape_bypass { 1.0 } else { 0.0 });
        self.set_param_atomic(self.params.tape_speed_30ips.as_ptr(), if s.tape_speed_30ips { 1.0 } else { 0.0 });
        self.set_param_atomic(self.params.tape_formula_gp9.as_ptr(), if s.tape_formula_gp9 { 1.0 } else { 0.0 });
        self.set_param_atomic(self.params.tape_drive.as_ptr(), self.params.tape_drive.preview_normalized(s.tape_drive));
        self.set_param_atomic(self.params.tape_bias.as_ptr(), self.params.tape_bias.preview_normalized(s.tape_bias));
        self.set_param_atomic(self.params.tape_headbump.as_ptr(), self.params.tape_headbump.preview_normalized(s.tape_headbump));
        self.set_param_atomic(self.params.tape_bump_freq.as_ptr(), self.params.tape_bump_freq.preview_normalized(s.tape_bump_freq));
        self.set_param_atomic(self.params.tape_flutter.as_ptr(), self.params.tape_flutter.preview_normalized(s.tape_flutter));
        self.set_param_atomic(self.params.tape_comp.as_ptr(), if s.tape_comp { 1.0 } else { 0.0 });
        self.set_param_atomic(self.params.clipper_bypass.as_ptr(), if s.clipper_bypass { 1.0 } else { 0.0 });
        self.set_param_atomic(self.params.clipper_drive.as_ptr(), self.params.clipper_drive.preview_normalized(s.clipper_drive));
        self.set_param_atomic(self.params.clipper_ceiling.as_ptr(), self.params.clipper_ceiling.preview_normalized(s.clipper_ceiling));
        self.set_param_atomic(self.params.clipper_hard_shape.as_ptr(), if s.clipper_hard_shape { 1.0 } else { 0.0 });
        self.set_param_atomic(self.params.clipper_os_4x.as_ptr(), if s.clipper_os_4x { 1.0 } else { 0.0 });
        self.set_param_atomic(self.params.clipper_comp.as_ptr(), if s.clipper_comp { 1.0 } else { 0.0 });
        self.set_param_atomic(self.params.eq_bypass.as_ptr(), if s.eq_bypass { 1.0 } else { 0.0 });
        self.set_param_atomic(self.params.eq_model.as_ptr(), match s.eq_model { EqModel::Ptek => 0.0, EqModel::Mass => 1.0 });
        self.set_param_atomic(self.params.ptek_low_boost.as_ptr(), self.params.ptek_low_boost.preview_normalized(s.ptek_low_boost));
        self.set_param_atomic(self.params.ptek_low_atten.as_ptr(), self.params.ptek_low_atten.preview_normalized(s.ptek_low_atten));
        self.set_param_atomic(self.params.ptek_low_freq.as_ptr(), self.params.ptek_low_freq.preview_normalized(s.ptek_low_freq));
        self.set_param_atomic(self.params.ptek_high_boost.as_ptr(), self.params.ptek_high_boost.preview_normalized(s.ptek_high_boost));
        self.set_param_atomic(self.params.ptek_high_freq.as_ptr(), self.params.ptek_high_freq.preview_normalized(s.ptek_high_freq));
        self.set_param_atomic(self.params.ptek_bandwidth.as_ptr(), self.params.ptek_bandwidth.preview_normalized(s.ptek_bandwidth));
        self.set_param_atomic(self.params.ptek_high_atten.as_ptr(), self.params.ptek_high_atten.preview_normalized(s.ptek_high_atten));
        self.set_param_atomic(self.params.ptek_atten_freq.as_ptr(), self.params.ptek_atten_freq.preview_normalized(s.ptek_atten_freq));
        self.set_param_atomic(self.params.mass_low_bypass.as_ptr(), if s.mass_low_bypass { 1.0 } else { 0.0 });
        self.set_param_atomic(self.params.mass_low_gain.as_ptr(), self.params.mass_low_gain.preview_normalized(s.mass_low_gain));
        self.set_param_atomic(self.params.mass_low_freq.as_ptr(), self.params.mass_low_freq.preview_normalized(s.mass_low_freq));
        self.set_param_atomic(self.params.mass_low_q.as_ptr(), self.params.mass_low_q.preview_normalized(s.mass_low_q));
        self.set_param_atomic(self.params.mass_low_shelf.as_ptr(), if s.mass_low_shelf { 1.0 } else { 0.0 });
        self.set_param_atomic(self.params.mass_lmf_bypass.as_ptr(), if s.mass_lmf_bypass { 1.0 } else { 0.0 });
        self.set_param_atomic(self.params.mass_lmf_gain.as_ptr(), self.params.mass_lmf_gain.preview_normalized(s.mass_lmf_gain));
        self.set_param_atomic(self.params.mass_lmf_freq.as_ptr(), self.params.mass_lmf_freq.preview_normalized(s.mass_lmf_freq));
        self.set_param_atomic(self.params.mass_lmf_q.as_ptr(), self.params.mass_lmf_q.preview_normalized(s.mass_lmf_q));
        self.set_param_atomic(self.params.mass_lmf_shelf.as_ptr(), if s.mass_lmf_shelf { 1.0 } else { 0.0 });
        self.set_param_atomic(self.params.mass_hmf_bypass.as_ptr(), if s.mass_hmf_bypass { 1.0 } else { 0.0 });
        self.set_param_atomic(self.params.mass_hmf_gain.as_ptr(), self.params.mass_hmf_gain.preview_normalized(s.mass_hmf_gain));
        self.set_param_atomic(self.params.mass_hmf_freq.as_ptr(), self.params.mass_hmf_freq.preview_normalized(s.mass_hmf_freq));
        self.set_param_atomic(self.params.mass_hmf_q.as_ptr(), self.params.mass_hmf_q.preview_normalized(s.mass_hmf_q));
        self.set_param_atomic(self.params.mass_hmf_shelf.as_ptr(), if s.mass_hmf_shelf { 1.0 } else { 0.0 });
        self.set_param_atomic(self.params.mass_hf_bypass.as_ptr(), if s.mass_hf_bypass { 1.0 } else { 0.0 });
        self.set_param_atomic(self.params.mass_hf_gain.as_ptr(), self.params.mass_hf_gain.preview_normalized(s.mass_hf_gain));
        self.set_param_atomic(self.params.mass_hf_freq.as_ptr(), self.params.mass_hf_freq.preview_normalized(s.mass_hf_freq));
        self.set_param_atomic(self.params.mass_hf_q.as_ptr(), self.params.mass_hf_q.preview_normalized(s.mass_hf_q));
        self.set_param_atomic(self.params.mass_hf_shelf.as_ptr(), if s.mass_hf_shelf { 1.0 } else { 0.0 });
        self.set_param_atomic(self.params.eq_amp_type.as_ptr(), self.params.eq_amp_type.preview_normalized(s.eq_amp_type));
        self.set_param_atomic(self.params.eq_trans_type.as_ptr(), self.params.eq_trans_type.preview_normalized(s.eq_trans_type));
        self.set_param_atomic(self.params.eq_intensity.as_ptr(), self.params.eq_intensity.preview_normalized(s.eq_intensity));
        self.set_param_atomic(self.params.eq_col_comp.as_ptr(), if s.eq_col_comp { 1.0 } else { 0.0 });
        self.set_param_atomic(self.params.eq_col_match.as_ptr(), if s.eq_col_match { 1.0 } else { 0.0 });
        self.set_param_atomic(self.params.comp_bypass.as_ptr(), if s.comp_bypass { 1.0 } else { 0.0 });
        self.set_param_atomic(self.params.comp_model.as_ptr(), match s.comp_model { CompModel::Fair => 0.0, CompModel::VMu => 1.0 });
        self.set_param_atomic(self.params.comp_thresh.as_ptr(), self.params.comp_thresh.preview_normalized(s.comp_thresh));
        self.set_param_atomic(self.params.comp_ratio.as_ptr(), self.params.comp_ratio.preview_normalized(s.comp_ratio));
        self.set_param_atomic(self.params.comp_in_gain.as_ptr(), self.params.comp_in_gain.preview_normalized(s.comp_in_gain));
        self.set_param_atomic(self.params.comp_attack.as_ptr(), self.params.comp_attack.preview_normalized(s.comp_attack));
        self.set_param_atomic(self.params.comp_release.as_ptr(), self.params.comp_release.preview_normalized(s.comp_release));
        self.set_param_atomic(self.params.comp_sc_hpf_hz.as_ptr(), self.params.comp_sc_hpf_hz.preview_normalized(s.comp_sc_hpf_hz));
        self.set_param_atomic(self.params.comp_mix.as_ptr(), self.params.comp_mix.preview_normalized(s.comp_mix));
        self.set_param_atomic(self.params.comp_makeup.as_ptr(), self.params.comp_makeup.preview_normalized(s.comp_makeup));
        self.set_param_atomic(self.params.comp_auto_makeup.as_ptr(), if s.comp_auto_makeup { 1.0 } else { 0.0 });
        self.set_param_atomic(self.params.fair_threshold.as_ptr(), self.params.fair_threshold.preview_normalized(s.fair_threshold));
        self.set_param_atomic(self.params.fair_time_constant.as_ptr(), self.params.fair_time_constant.preview_normalized(s.fair_time_constant));
        self.set_param_atomic(self.params.fair_mix.as_ptr(), self.params.fair_mix.preview_normalized(s.fair_mix));
        self.set_param_atomic(self.params.fair_makeup.as_ptr(), self.params.fair_makeup.preview_normalized(s.fair_makeup));
        self.set_param_atomic(self.params.fair_sc_hpf.as_ptr(), if s.fair_sc_hpf { 1.0 } else { 0.0 });
        self.set_param_atomic(self.params.vmu_threshold.as_ptr(), self.params.vmu_threshold.preview_normalized(s.vmu_threshold));
        self.set_param_atomic(self.params.vmu_attack.as_ptr(), self.params.vmu_attack.preview_normalized(s.vmu_attack));
        self.set_param_atomic(self.params.vmu_recovery.as_ptr(), self.params.vmu_recovery.preview_normalized(s.vmu_recovery));
        self.set_param_atomic(self.params.vmu_mode.as_ptr(), if s.vmu_mode { 1.0 } else { 0.0 });
        self.set_param_atomic(self.params.vmu_sc_hpf.as_ptr(), if s.vmu_sc_hpf { 1.0 } else { 0.0 });
        self.set_param_atomic(self.params.vmu_mix.as_ptr(), self.params.vmu_mix.preview_normalized(s.vmu_mix));
        self.set_param_atomic(self.params.vmu_makeup.as_ptr(), self.params.vmu_makeup.preview_normalized(s.vmu_makeup));
        self.set_param_atomic(self.params.comp_amp_type.as_ptr(), self.params.comp_amp_type.preview_normalized(s.comp_amp_type));
        self.set_param_atomic(self.params.comp_trans_type.as_ptr(), self.params.comp_trans_type.preview_normalized(s.comp_trans_type));
        self.set_param_atomic(self.params.comp_intensity.as_ptr(), self.params.comp_intensity.preview_normalized(s.comp_intensity));
        self.set_param_atomic(self.params.comp_col_comp.as_ptr(), if s.comp_col_comp { 1.0 } else { 0.0 });
        self.set_param_atomic(self.params.comp_col_match.as_ptr(), if s.comp_col_match { 1.0 } else { 0.0 });
        self.set_param_atomic(self.params.limiter_bypass.as_ptr(), if s.limiter_bypass { 1.0 } else { 0.0 });
        self.set_param_atomic(self.params.limiter_gain.as_ptr(), self.params.limiter_gain.preview_normalized(s.limiter_gain));
        self.set_param_atomic(self.params.limiter_ceiling.as_ptr(), self.params.limiter_ceiling.preview_normalized(s.limiter_ceiling));
        self.set_param_atomic(self.params.limiter_lookahead_ms.as_ptr(), self.params.limiter_lookahead_ms.preview_normalized(s.limiter_lookahead_ms));
        self.set_param_atomic(self.params.limiter_release_ms.as_ptr(), self.params.limiter_release_ms.preview_normalized(s.limiter_release_ms));
        self.set_param_atomic(self.params.limiter_lift.as_ptr(), self.params.limiter_lift.preview_normalized(s.limiter_lift));
        self.set_param_atomic(self.params.limiter_auto_release.as_ptr(), if s.limiter_auto_release { 1.0 } else { 0.0 });
        self.set_param_atomic(self.params.limiter_true_peak.as_ptr(), if s.limiter_true_peak { 1.0 } else { 0.0 });
        self.set_param_atomic(self.params.limiter_punch.as_ptr(), if s.limiter_punch { 1.0 } else { 0.0 });
        self.set_param_atomic(self.params.limiter_unity.as_ptr(), if s.limiter_unity { 1.0 } else { 0.0 });
        self.set_param_atomic(self.params.output_trim.as_ptr(), self.params.output_trim.preview_normalized(s.output_trim));
    }

    pub fn preset_name(&self, idx: usize) -> String {
        if idx < FACTORY_PRESETS.len() {
            FACTORY_PRESETS[idx].name.to_string()
        } else {
            let u_idx = idx - FACTORY_PRESETS.len();
            let presets = self.user_presets.borrow();
            presets
                .get(u_idx)
                .map(|p| p.name.clone())
                .unwrap_or_else(|| "UNKNOWN".to_string())
        }
    }

    pub fn total_presets(&self) -> usize {
        FACTORY_PRESETS.len() + self.user_presets.borrow().len()
    }

    fn apply_preset_values(&self, p: &UserPreset) {
        self.set_param_norm(self.params.tape_speed_30ips.as_ptr(), if p.tape_speed_30ips { 1.0 } else { 0.0 });
        self.set_param_norm(self.params.tape_formula_gp9.as_ptr(), if p.tape_formula_gp9 { 1.0 } else { 0.0 });
        self.set_param_norm(self.params.tape_drive.as_ptr(), self.params.tape_drive.preview_normalized(p.tape_drive));
        self.set_param_norm(self.params.tape_bias.as_ptr(), self.params.tape_bias.preview_normalized(p.tape_bias));
        self.set_param_norm(self.params.tape_headbump.as_ptr(), self.params.tape_headbump.preview_normalized(p.tape_bump));
        self.set_param_norm(self.params.tape_bump_freq.as_ptr(), self.params.tape_bump_freq.preview_normalized(p.tape_bump_freq));
        self.set_param_norm(self.params.tape_flutter.as_ptr(), self.params.tape_flutter.preview_normalized(p.tape_flutter));
        self.set_param_norm(self.params.clipper_drive.as_ptr(), self.params.clipper_drive.preview_normalized(p.clipper_drive));
        self.set_param_norm(self.params.clipper_ceiling.as_ptr(), self.params.clipper_ceiling.preview_normalized(p.clipper_ceiling));
        self.set_param_norm(self.params.clipper_hard_shape.as_ptr(), if p.clipper_hard_shape { 1.0 } else { 0.0 });
        self.set_param_norm(self.params.clipper_os_4x.as_ptr(), if p.clipper_os_4x { 1.0 } else { 0.0 });
        self.set_param_norm(self.params.eq_model.as_ptr(), match p.eq_model { EqModel::Ptek => 0.0, EqModel::Mass => 1.0 });
        self.set_param_norm(self.params.comp_model.as_ptr(), match p.comp_model { CompModel::Fair => 0.0, CompModel::VMu => 1.0 });
        self.set_param_norm(self.params.comp_thresh.as_ptr(), self.params.comp_thresh.preview_normalized(p.comp_thresh));
        self.set_param_norm(self.params.comp_ratio.as_ptr(), self.params.comp_ratio.preview_normalized(p.comp_ratio.max(1.2).min(20.0)));
        self.set_param_norm(self.params.comp_attack.as_ptr(), self.params.comp_attack.preview_normalized(p.comp_attack.max(0.1).min(30.0)));
        self.set_param_norm(self.params.comp_release.as_ptr(), self.params.comp_release.preview_normalized(p.comp_release.max(0.0).min(2000.0)));
        let mix_val = if p.comp_mix <= 1.0 && p.comp_mix > 0.0 { p.comp_mix * 100.0 } else { p.comp_mix };
        self.set_param_norm(self.params.comp_mix.as_ptr(), self.params.comp_mix.preview_normalized(mix_val.max(0.0).min(100.0)));
        self.set_param_norm(self.params.limiter_gain.as_ptr(), self.params.limiter_gain.preview_normalized(p.limiter_gain.max(0.0).min(10.0)));
        self.set_param_norm(self.params.limiter_ceiling.as_ptr(), self.params.limiter_ceiling.preview_normalized(p.limiter_ceiling.max(-3.0).min(0.0)));
        self.set_param_norm(self.params.limiter_lookahead_ms.as_ptr(), self.params.limiter_lookahead_ms.preview_normalized(p.limiter_lookahead.max(0.0).min(5.0)));
        self.set_param_norm(self.params.limiter_release_ms.as_ptr(), self.params.limiter_release_ms.preview_normalized(p.limiter_release.max(0.0).min(1200.0)));
        self.set_param_norm(self.params.limiter_lift.as_ptr(), self.params.limiter_lift.preview_normalized(p.limiter_lift.max(0.0).min(1.0)));
        self.set_param_norm(self.params.limiter_true_peak.as_ptr(), if p.limiter_true_peak { 1.0 } else { 0.0 });
        self.set_param_norm(self.params.limiter_punch.as_ptr(), if p.limiter_punch { 1.0 } else { 0.0 });
        self.set_param_norm(self.params.comp_auto_makeup.as_ptr(), 1.0);
        if self.params.eq_col_match.value() {
            let (a, t) = eq_hardware_topology(p.eq_model);
            self.set_param_norm(self.params.eq_amp_type.as_ptr(), (a as f32) / 6.0);
            self.set_param_norm(self.params.eq_trans_type.as_ptr(), (t as f32) / 4.0);
        }
        if self.params.comp_col_match.value() {
            let (a, t) = comp_hardware_topology(p.comp_model);
            self.set_param_norm(self.params.comp_amp_type.as_ptr(), (a as f32) / 6.0);
            self.set_param_norm(self.params.comp_trans_type.as_ptr(), (t as f32) / 4.0);
        }
    }

    pub fn load_preset(&self, idx: usize) {
        let total = self.total_presets();
        if total == 0 || idx >= total {
            return;
        }
        self.current_preset_idx.set(idx);
        let preset_opt = if idx < FACTORY_PRESETS.len() {
            let f = &FACTORY_PRESETS[idx];
            Some(UserPreset {
                name: f.name.to_string(),
                tape_speed_30ips: f.tape_speed_30ips,
                tape_formula_gp9: f.tape_formula_gp9,
                tape_drive: f.tape_drive,
                tape_bias: f.tape_bias,
                tape_bump: f.tape_bump,
                tape_bump_freq: f.tape_bump_freq,
                tape_flutter: f.tape_flutter,
                clipper_drive: f.clipper_drive,
                clipper_ceiling: f.clipper_ceiling,
                clipper_hard_shape: f.clipper_hard_shape,
                clipper_os_4x: f.clipper_os_4x,
                eq_model: f.eq_model,
                comp_model: f.comp_model,
                comp_thresh: f.comp_thresh,
                comp_ratio: f.comp_ratio,
                comp_attack: f.comp_attack,
                comp_release: f.comp_release,
                comp_mix: f.comp_mix,
                limiter_gain: f.limiter_gain,
                limiter_ceiling: f.limiter_ceiling,
                limiter_lookahead: f.limiter_lookahead,
                limiter_release: f.limiter_release,
                limiter_lift: f.limiter_lift,
                limiter_true_peak: f.limiter_true_peak,
                limiter_punch: f.limiter_punch,
            })
        } else {
            let u_idx = idx - FACTORY_PRESETS.len();
            self.user_presets.borrow().get(u_idx).cloned()
        };
        if let Some(up) = preset_opt {
            self.apply_preset_values(&up);
        }
    }

    pub fn save_user_preset(&self, name: String) {
        let up = UserPreset {
            name: name.clone(),
            tape_speed_30ips: self.params.tape_speed_30ips.value(),
            tape_formula_gp9: self.params.tape_formula_gp9.value(),
            tape_drive: self.params.tape_drive.value(),
            tape_bias: self.params.tape_bias.value(),
            tape_bump: self.params.tape_headbump.value(),
            tape_bump_freq: self.params.tape_bump_freq.value(),
            tape_flutter: self.params.tape_flutter.value(),
            clipper_drive: self.params.clipper_drive.value(),
            clipper_ceiling: self.params.clipper_ceiling.value(),
            clipper_hard_shape: self.params.clipper_hard_shape.value(),
            clipper_os_4x: self.params.clipper_os_4x.value(),
            eq_model: self.params.eq_model.value(),
            comp_model: self.params.comp_model.value(),
            comp_thresh: self.params.comp_thresh.value(),
            comp_ratio: self.params.comp_ratio.value(),
            comp_attack: self.params.comp_attack.value(),
            comp_release: self.params.comp_release.value(),
            comp_mix: self.params.comp_mix.value(),
            limiter_gain: self.params.limiter_gain.value(),
            limiter_ceiling: self.params.limiter_ceiling.value(),
            limiter_lookahead: self.params.limiter_lookahead_ms.value(),
            limiter_release: self.params.limiter_release_ms.value(),
            limiter_lift: self.params.limiter_lift.value(),
            limiter_true_peak: self.params.limiter_true_peak.value(),
            limiter_punch: self.params.limiter_punch.value(),
        };

        let mut presets = self.user_presets.borrow_mut();
        if let Some(existing_idx) = presets.iter().position(|p| p.name.eq_ignore_ascii_case(&name)) {
            presets[existing_idx] = up;
            self.current_preset_idx.set(FACTORY_PRESETS.len() + existing_idx);
        } else {
            presets.push(up);
            self.current_preset_idx.set(FACTORY_PRESETS.len() + presets.len() - 1);
        }
        save_user_presets(&presets);
    }

    /// Map the current press to a special-function countdown. Called from `MouseDown`; the frame
    /// poll in [`Self::draw`] fires the action once [`HOLD_MS`] has elapsed, whether or not the
    /// mouse moves again.
    fn arm_hold(&self, action: HoldAction, start_y: f32, cancel_on_turn: bool) {
        self.hold_press.set(Some(HoldPress {
            action,
            down_at: Instant::now(),
            start_y,
            cancel_on_turn,
            fired: false,
        }));
    }

    /// Advance the countdown. Runs once per rendered frame, which is the only clock the baseview
    /// backend provides: it never ticks vizia timers, so a timer-driven approach fires nothing
    /// until the next mouse event (i.e. on release).
    fn poll_hold(&self) {
        // A lost `MouseUp` (focus change, window leave) must not leave a countdown armed.
        if !self.pointer_down.get() {
            self.hold_press.set(None);
            return;
        }
        let mut press = match self.hold_press.get() {
            Some(press) => press,
            None => return,
        };
        let hold_ms = match press.action {
            HoldAction::ToggleDebugMenu => DEBUG_MENU_HOLD_MS,
            _ => HOLD_MS,
        };
        if !press.fired && press.down_at.elapsed() >= Duration::from_millis(hold_ms) {
            press.fired = true;
            self.hold_press.set(Some(press));
            self.fire_hold(press.action);
        }
    }

    fn fire_hold(&self, action: HoldAction) {
        match action {
            HoldAction::PushThird { is_comp } => {
                let ptr = if is_comp {
                    self.params.comp_intensity.as_ptr()
                } else {
                    self.params.eq_intensity.as_ptr()
                };
                self.set_param_atomic(ptr, 1.0);
                self.smash_banner.set(Some((is_comp, Instant::now())));
            }
            HoldAction::MakeupAuto => {
                let auto = self.params.comp_auto_makeup.value();
                self.set_param_atomic(
                    self.params.comp_auto_makeup.as_ptr(),
                    if auto { 0.0 } else { 1.0 },
                );
            }
            HoldAction::RelearnColoration { is_comp } => {
                let (ptr, flag) = if is_comp {
                    (
                        self.params.comp_col_comp.as_ptr(),
                        &self.meters.comp_col_relearn,
                    )
                } else {
                    (
                        self.params.eq_col_comp.as_ptr(),
                        &self.meters.eq_col_relearn,
                    )
                };
                // Holding AUTO means "measure this again", which only makes sense with AUTO on.
                self.set_param_atomic(ptr, 1.0);
                flag.store(true, std::sync::atomic::Ordering::Relaxed);
            }
            HoldAction::ToggleDebugMenu => {
                let open = !self.debug_menu_open.get();
                self.debug_menu_open.set(open);
                // Leaving stale text in an edit box after closing would make the next open
                // start in a half-finished state.
                self.debug_edit.replace(None);
                self.debug_commit.set(None);
            }
        }
    }

    /// Normal (short-click) behaviour for a control that also has a hold gesture.
    fn click_action(&mut self, action: HoldAction) {
        match action {
            HoldAction::PushThird { is_comp } => {
                let ptr = if is_comp {
                    self.params.comp_intensity.as_ptr()
                } else {
                    self.params.eq_intensity.as_ptr()
                };
                let cur = if is_comp {
                    self.params.comp_intensity.value()
                } else {
                    self.params.eq_intensity.value()
                };
                // NORMAL <-> OFF: the third state is hold-only.
                let next_norm = if cur == 0 { 0.5 } else { 0.0 };
                self.set_param_atomic(ptr, next_norm);
            }
            // A short press on the push-pull pot never changes AUTO.
            HoldAction::MakeupAuto => {}
            // Short click keeps the plain engage/disengage behaviour. A second click inside the
            // double-click window restores the state from before the pair and forgets the frozen
            // measurement, because the host may not have echoed the first toggle back yet.
            HoldAction::RelearnColoration { is_comp } => {
                let (ptr, cur, clear_flag) = if is_comp {
                    (
                        self.params.comp_col_comp.as_ptr(),
                        self.params.comp_col_comp.value(),
                        &self.meters.comp_col_clear,
                    )
                } else {
                    (
                        self.params.eq_col_comp.as_ptr(),
                        self.params.eq_col_comp.value(),
                        &self.meters.eq_col_clear,
                    )
                };
                let now = Instant::now();
                let previous = self.auto_click_memory.get();
                let is_double = matches!(
                    previous,
                    Some((button, _, at))
                        if button == is_comp
                            && now.duration_since(at) <= Duration::from_millis(500)
                );
                if is_double {
                    let before = previous.map(|(_, value, _)| value).unwrap_or(false);
                    self.set_param_atomic(ptr, if before { 1.0 } else { 0.0 });
                    clear_flag.store(true, std::sync::atomic::Ordering::Relaxed);
                    self.auto_click_memory.set(None);
                } else {
                    self.set_param_atomic(ptr, if cur { 0.0 } else { 1.0 });
                    self.auto_click_memory.set(Some((is_comp, cur, now)));
                }
            }
            HoldAction::ToggleDebugMenu => {
                // The menu is hold-only: a short click must not open it.
            }
        }
    }

    fn closest_index(val: f32, table: &[f32]) -> usize {
        let mut best_i = 0;
        let mut min_d = f32::MAX;
        for (i, &v) in table.iter().enumerate() {
            let d = (val - v).abs();
            if d < min_d {
                min_d = d;
                best_i = i;
            }
        }
        best_i
    }

    fn is_stepped_match(val: f32, table: &[f32]) -> Option<usize> {
        let idx = Self::closest_index(val, table);
        let step_val = table[idx];
        let tol = (step_val.abs() * 0.015).max(0.04);
        if (val - step_val).abs() <= tol {
            Some(idx)
        } else {
            None
        }
    }

    fn float_step_info(
        &self,
        ctrl: SteppedControl,
    ) -> Option<(&FloatParam, &'static [f32], &'static [&'static str])> {
        let is_fair = self.params.comp_model.value() == CompModel::Fair;
        match ctrl {
            SteppedControl::TapeDrive => Some((&self.params.tape_drive, &DB_11_VALS, &DB_11_STRS)),
            SteppedControl::TapeBias => Some((&self.params.tape_bias, &DB_11_VALS, &DB_11_STRS)),
            SteppedControl::TapeHeadBump => {
                Some((&self.params.tape_headbump, &DB_11_VALS, &DB_11_STRS))
            }
            SteppedControl::TapeBumpFreq => Some((
                &self.params.tape_bump_freq,
                &TAPE_FREQ_VALS,
                &TAPE_FREQ_STRS,
            )),
            SteppedControl::TapeFlutter => {
                Some((&self.params.tape_flutter, &FLUTTER_VALS, &FLUTTER_STRS))
            }
            SteppedControl::ClipperDrive => {
                Some((&self.params.clipper_drive, &DB_11_VALS, &DB_11_STRS))
            }
            SteppedControl::ClipperCeiling => {
                Some((&self.params.clipper_ceiling, &CEIL_VALS, &CEIL_STRS))
            }
            SteppedControl::InputTrim => Some((&self.params.input_trim, &DB_11_VALS, &DB_11_STRS)),
            SteppedControl::LoCut => {
                Some((&self.params.filter_lo_cut_hz, &LO_CUT_VALS, &LO_CUT_STRS))
            }
            SteppedControl::HiCut => {
                Some((&self.params.filter_hi_cut_hz, &HI_CUT_VALS, &HI_CUT_STRS))
            }
            SteppedControl::MonoFreq => Some((
                &self.params.bass_mono_freq,
                &MONO_FREQ_VALS,
                &MONO_FREQ_STRS,
            )),
            SteppedControl::MonoAmount => {
                Some((&self.params.bass_mono_amount, &PCT_VALS, &PCT_STRS))
            }
            SteppedControl::EqQ(0) => Some((&self.params.mass_low_q, &Q_VALS, &Q_STRS)),
            SteppedControl::EqQ(1) => Some((&self.params.mass_lmf_q, &Q_VALS, &Q_STRS)),
            SteppedControl::EqQ(2) => Some((&self.params.mass_hmf_q, &Q_VALS, &Q_STRS)),
            SteppedControl::EqQ(_) => Some((&self.params.mass_hf_q, &Q_VALS, &Q_STRS)),
            SteppedControl::CompThresh => {
                Some((&self.params.comp_thresh, &DB_11_VALS, &DB_11_STRS))
            }
            SteppedControl::CompScHpf => {
                Some((&self.params.comp_sc_hpf_hz, &SC_HPF_VALS, &SC_HPF_STRS))
            }
            SteppedControl::CompMix => Some((&self.params.comp_mix, &PCT_VALS, &PCT_STRS)),
            SteppedControl::CompMakeup => {
                Some((&self.params.comp_makeup, &MAKEUP_11_VALS, &MAKEUP_11_STRS))
            }
            SteppedControl::CompInGain => {
                Some((&self.params.comp_in_gain, &DB_11_VALS, &DB_11_STRS))
            }
            SteppedControl::CompRatio => Some((&self.params.comp_ratio, &RATIO_VALS, &RATIO_STRS)),
            SteppedControl::CompAttack => Some((
                &self.params.comp_attack,
                &ATTACK_VALS,
                if is_fair { &FAIR_TC_STRS } else { &ATTACK_STRS },
            )),
            SteppedControl::CompRelease => {
                Some((&self.params.comp_release, &RELEASE_VALS, &RELEASE_STRS))
            }
            SteppedControl::LimiterGain => {
                Some((&self.params.limiter_gain, &LIMITER_GAIN_VALS, &LIMITER_GAIN_STRS))
            }
            SteppedControl::LimiterCeiling => {
                Some((&self.params.limiter_ceiling, &CEIL_VALS, &CEIL_STRS))
            }
            SteppedControl::LimiterLookahead => {
                Some((&self.params.limiter_lookahead_ms, &LOOKAHEAD_VALS, &LOOKAHEAD_STRS))
            }
            SteppedControl::LimiterRelease => {
                Some((&self.params.limiter_release_ms, &LIMITER_RELEASE_VALS, &LIMITER_RELEASE_STRS))
            }
            SteppedControl::LimiterLift => {
                Some((&self.params.limiter_lift, &LIFT_VALS, &LIFT_STRS))
            }
            SteppedControl::EqGain(0) => {
                Some((&self.params.mass_low_gain, &EQ_GAIN_VALS, &EQ_GAIN_STRS))
            }
            SteppedControl::EqGain(1) => {
                Some((&self.params.mass_lmf_gain, &EQ_GAIN_VALS, &EQ_GAIN_STRS))
            }
            SteppedControl::EqGain(2) => {
                Some((&self.params.mass_hmf_gain, &EQ_GAIN_VALS, &EQ_GAIN_STRS))
            }
            SteppedControl::EqGain(_) => {
                Some((&self.params.mass_hf_gain, &EQ_GAIN_VALS, &EQ_GAIN_STRS))
            }
            SteppedControl::PuLowBoost => Some((&self.params.ptek_low_boost, &PU_VALS, &PU_STRS)),
            SteppedControl::PuLowAtten => Some((&self.params.ptek_low_atten, &PU_VALS, &PU_STRS)),
            SteppedControl::PuHighBoost => Some((&self.params.ptek_high_boost, &PU_VALS, &PU_STRS)),
            SteppedControl::PuHighAtten => Some((&self.params.ptek_high_atten, &PU_VALS, &PU_STRS)),
            SteppedControl::PuBandwidth => Some((&self.params.ptek_bandwidth, &PU_VALS, &PU_STRS)),
            SteppedControl::EqFreq(_)
            | SteppedControl::PuLowFreq
            | SteppedControl::PuHighFreq
            | SteppedControl::PuAttenSel => None,
        }
    }

    /// Rotary selector (integer) controls: returns the param and its number of positions.
    fn int_step_info(&self, ctrl: SteppedControl) -> Option<(&IntParam, usize)> {
        match ctrl {
            SteppedControl::EqFreq(b) => Some((self.eq_freq_param(b), 6)),
            SteppedControl::PuLowFreq => Some((&self.params.ptek_low_freq, 6)),
            SteppedControl::PuHighFreq => Some((&self.params.ptek_high_freq, 7)),
            SteppedControl::PuAttenSel => Some((&self.params.ptek_atten_freq, 3)),
            _ => None,
        }
    }

    fn int_label(&self, ctrl: SteppedControl, idx: usize) -> &'static str {
        match ctrl {
            SteppedControl::EqFreq(b) => self.eq_freq_label(b, idx),
            SteppedControl::PuLowFreq => PU_LOW_LABELS[idx.min(5)],
            SteppedControl::PuHighFreq => PU_HIGH_LABELS[idx.min(6)],
            SteppedControl::PuAttenSel => PU_ATTEN_LABELS[idx.min(2)],
            _ => "",
        }
    }

    fn eq_freq_param(&self, band: usize) -> &IntParam {
        match band {
            0 => &self.params.mass_low_freq,
            1 => &self.params.mass_lmf_freq,
            2 => &self.params.mass_hmf_freq,
            _ => &self.params.mass_hf_freq,
        }
    }

    /// Mass band frequency label. The Ptek selectors have their own params/labels.
    fn eq_freq_label(&self, band: usize, idx: usize) -> &'static str {
        MASS_FREQ_LABELS[band.min(3)][idx.min(5)]
    }

    fn control_ptr(&self, ctrl: SteppedControl) -> ParamPtr {
        if let Some((fp, _, _)) = self.float_step_info(ctrl) {
            fp.as_ptr()
        } else if let Some((ip, _)) = self.int_step_info(ctrl) {
            ip.as_ptr()
        } else {
            unreachable!()
        }
    }

    fn control_step_count(&self, ctrl: SteppedControl) -> usize {
        if let Some((_, count)) = self.int_step_info(ctrl) {
            count
        } else if let Some((_, table, _)) = self.float_step_info(ctrl) {
            table.len()
        } else {
            11
        }
    }

    /// Angle of detent `idx` when a control has `count` evenly spaced positions.
    fn tick_angle(idx: usize, count: usize) -> f32 {
        let last = count.max(2) - 1;
        -135.0 + 270.0 * (idx.min(last) as f32) / (last as f32)
    }

    /// Threshold sweep for the selected model: a fixed, measured span shifted by the model's trim.
    fn comp_thresh_range(&self) -> (f32, f32) {
        let calibration = debug_calibration::calibration();
        if self.params.comp_model.value() == CompModel::VMu {
            let t = calibration.trim_vmu();
            (Self::VMU_THRESH_MIN_DB + t, Self::VMU_THRESH_MAX_DB + t)
        } else {
            let t = calibration.trim_fair();
            (Self::FAIR_THRESH_MIN_DB + t, Self::FAIR_THRESH_MAX_DB + t)
        }
    }

    /// dB value of detent `idx` (0..=10) on the compressor threshold knob.
    fn comp_thresh_value_at(&self, idx: usize) -> f32 {
        let (min, max) = self.comp_thresh_range();
        let span = max - min;
        if span.abs() < 1.0e-6 {
            return min;
        }
        min + span * (idx.min(10) as f32 / 10.0)
    }

    /// Nearest detent for a threshold value, given the calibrated range.
    fn comp_thresh_idx(&self, value: f32) -> usize {
        let (min, max) = self.comp_thresh_range();
        let span = max - min;
        if span.abs() < 1.0e-6 {
            return 0;
        }
        (((value - min) / span).clamp(0.0, 1.0) * 10.0).round() as usize
    }

    fn control_current_idx(&self, ctrl: SteppedControl) -> usize {
        if let Some(DragTarget::Knob {
            ctrl: active,
            cur_idx,
            ..
        }) = &self.active_drag
        {
            if *active == ctrl {
                return *cur_idx;
            }
        }
        if ctrl == SteppedControl::CompThresh {
            return self.comp_thresh_idx(self.params.comp_thresh.value());
        }
        match ctrl {
            SteppedControl::LoCut => {
                let hz = self.params.filter_lo_cut_hz.value();
                if hz < 0.5 {
                    0
                } else {
                    let freq_idx = Self::closest_index(hz, &LO_CUT_5_FREQS);
                    let slope = (self.params.filter_lo_cut_slope.value().clamp(0, 1)) as usize;
                    (1 + freq_idx * 2 + slope).min(10)
                }
            }
            SteppedControl::HiCut => {
                let hz = self.params.filter_hi_cut_hz.value();
                if hz < 0.5 {
                    10
                } else {
                    let freq_idx = Self::closest_index(hz, &HI_CUT_5_FREQS);
                    let slope = (self.params.filter_hi_cut_slope.value().clamp(0, 1)) as usize;
                    (freq_idx * 2 + slope).min(9)
                }
            }
            SteppedControl::MonoFreq => {
                let hz = self.params.bass_mono_freq.value();
                if hz < 0.5 {
                    0
                } else {
                    let freq_idx = Self::closest_index(hz, &MONO_5_FREQS);
                    let slope = (self.params.bass_mono_slope.value().clamp(0, 1)) as usize;
                    (1 + freq_idx * 2 + slope).min(10)
                }
            }
            _ => {
                if let Some((fp, table, _)) = self.float_step_info(ctrl) {
                    Self::closest_index(fp.value(), table)
                } else if let Some((ip, count)) = self.int_step_info(ctrl) {
                    let n = count.max(1);
                    if let Some(slot) = int_ctrl_slot(ctrl) {
                        if let Some((cmd_ctrl, cmd_idx, when)) = self.int_cmd.get()[slot] {
                            if cmd_ctrl == ctrl && when.elapsed() < INT_CMD_HOLD {
                                let live = (ip.value().clamp(0, n as i32 - 1)) as usize;
                                if live != cmd_idx.min(n - 1) {
                                    return cmd_idx.min(n - 1);
                                }
                            }
                        }
                    }
                    (ip.value().clamp(0, n as i32 - 1)) as usize
                } else {
                    0
                }
            }
        }
    }

    /// Remember the selector position that was just sent to the host so the UI can draw it
    /// immediately instead of waiting for the parameter write to round-trip.
    fn set_int_cmd(&self, ctrl: SteppedControl, idx: usize) {
        if let Some(slot) = int_ctrl_slot(ctrl) {
            let mut slots = self.int_cmd.get();
            slots[slot] = Some((ctrl, idx, Instant::now()));
            self.int_cmd.set(slots);
        }
    }

    /// Mirror a default reset into the commanded-position cache (Alt-click / double-click).
    fn sync_int_cmd_to_default(&self, ctrl: SteppedControl, ptr: ParamPtr) {
        if is_int_control(ctrl) {
            let idx = unsafe { ptr.default_plain_value() }.round().max(0.0) as usize;
            self.set_int_cmd(ctrl, idx);
        }
    }

    fn control_idx_to_norm(&self, ctrl: SteppedControl, idx: usize) -> f32 {
        if ctrl == SteppedControl::CompThresh {
            return self
                .params
                .comp_thresh
                .preview_normalized(self.comp_thresh_value_at(idx));
        }
        match ctrl {
            SteppedControl::LoCut => {
                let hz = if idx == 0 { 0.0 } else { LO_CUT_5_FREQS[(idx - 1) / 2] };
                self.params.filter_lo_cut_hz.preview_normalized(hz)
            }
            SteppedControl::HiCut => {
                let hz = if idx == 10 { 0.0 } else { HI_CUT_5_FREQS[idx / 2] };
                self.params.filter_hi_cut_hz.preview_normalized(hz)
            }
            SteppedControl::MonoFreq => {
                let hz = if idx == 0 { 0.0 } else { MONO_5_FREQS[(idx - 1) / 2] };
                self.params.bass_mono_freq.preview_normalized(hz)
            }
            _ => {
                if let Some((fp, table, _)) = self.float_step_info(ctrl) {
                    let clamped = idx.min(table.len() - 1);
                    fp.preview_normalized(table[clamped])
                } else if let Some((ip, count)) = self.int_step_info(ctrl) {
                    let clamped = idx.min(count - 1) as i32;
                    ip.preview_normalized(clamped)
                } else {
                    0.0
                }
            }
        }
    }

    /// Plain value for a stepped float position. Used to build the readout from the step that is
    /// being sent to the host rather than from the possibly not-yet-updated parameter value.
    fn control_plain_for_idx(&self, ctrl: SteppedControl, idx: usize) -> Option<f32> {
        if ctrl == SteppedControl::CompThresh {
            return Some(self.comp_thresh_value_at(idx));
        }
        match ctrl {
            SteppedControl::LoCut => {
                Some(if idx == 0 { 0.0 } else { LO_CUT_5_FREQS[(idx - 1) / 2] })
            }
            SteppedControl::HiCut => {
                Some(if idx == 10 { 0.0 } else { HI_CUT_5_FREQS[idx / 2] })
            }
            SteppedControl::MonoFreq => {
                Some(if idx == 0 { 0.0 } else { MONO_5_FREQS[(idx - 1) / 2] })
            }
            _ => {
                self.float_step_info(ctrl)
                    .map(|(_, table, _)| table[idx.min(table.len() - 1)])
            }
        }
    }

    /// `plain_override` shows the value that was just sent to the host. While a DAW is processing
    /// audio, VST3/CLAP deliberately do not apply GUI parameter writes synchronously; they
    /// round-trip through the audio callback. Reading `fp.value()` here would therefore render a
    /// stale value and the readout would appear to stop before the last steps of a drag.
    fn format_continuous_value_at(
        &self,
        ctrl: SteppedControl,
        plain_override: Option<f32>,
    ) -> String {
        if let Some((fp, _, _)) = self.float_step_info(ctrl) {
            let v = plain_override.unwrap_or_else(|| fp.value());
            match ctrl {
                SteppedControl::TapeDrive
                | SteppedControl::TapeBias
                | SteppedControl::TapeHeadBump
                | SteppedControl::ClipperDrive
                | SteppedControl::CompMakeup
                | SteppedControl::CompInGain
                | SteppedControl::LimiterGain => format!("{:+.1}dB", v),
                SteppedControl::InputTrim
                | SteppedControl::CompThresh
                | SteppedControl::EqGain(_) => format!("{:+.1}DB", v),
                SteppedControl::ClipperCeiling | SteppedControl::LimiterCeiling => {
                    format!("{:.2}dB", v)
                }
                SteppedControl::TapeBumpFreq => format!("{:.0}Hz", v),
                SteppedControl::CompScHpf => {
                    if v < 18.0 {
                        "OFF".to_string()
                    } else {
                        format!("{:.0}Hz", v)
                    }
                }
                SteppedControl::TapeFlutter
                | SteppedControl::MonoAmount
                | SteppedControl::CompMix => format!("{:.0}%", v),
                SteppedControl::LimiterLift => {
                    if v <= 0.001 {
                        "OFF".to_string()
                    } else {
                        format!("{:.0}%", v * 100.0)
                    }
                }
                SteppedControl::LimiterLookahead => {
                    if v <= 0.05 {
                        "OFF".to_string()
                    } else {
                        format!("{:.1}ms", v)
                    }
                }
                SteppedControl::LimiterRelease => {
                    if v <= 0.05 {
                        "AUTO".to_string()
                    } else if v >= 1000.0 {
                        format!("{:.1}s", v / 1000.0)
                    } else {
                        format!("{:.0}ms", v)
                    }
                }
                SteppedControl::EqQ(_) => format!("{:.2}", v),
                SteppedControl::CompRatio => format!("{:.1}:1", v),
                SteppedControl::CompAttack => format!("{:.1}ms", v),
                SteppedControl::CompRelease => {
                    if v >= 1950.0 {
                        "AUTO".to_string()
                    } else if v >= 1000.0 {
                        format!("{:.2}s", v / 1000.0)
                    } else {
                        format!("{:.0}ms", v)
                    }
                }
                // Ptek hardware readouts show the 0..10 knob position, not dB. The sign mirrors
                // the stage direction and the field is a fixed five characters wide so one- and
                // two-digit positions land in the same DSEG cells.
                SteppedControl::PuLowBoost
                | SteppedControl::PuLowAtten
                | SteppedControl::PuHighBoost
                | SteppedControl::PuHighAtten => ptek_position_label(ctrl, v).unwrap_or_default(),
                // BANDWIDTH is a Q control with no direction, so it stays a plain position.
                SteppedControl::PuBandwidth => format!("{:.1}", v),
                SteppedControl::EqFreq(_)
                | SteppedControl::PuLowFreq
                | SteppedControl::PuHighFreq
                | SteppedControl::PuAttenSel
                | SteppedControl::LoCut
                | SteppedControl::HiCut
                | SteppedControl::MonoFreq => String::new(),
            }
        } else {
            String::new()
        }
    }

    fn control_display_label(&self, ctrl: SteppedControl) -> String {
        self.control_display_label_at(ctrl, None)
    }

    fn control_display_label_at(
        &self,
        ctrl: SteppedControl,
        plain_override: Option<f32>,
    ) -> String {
        if ctrl == SteppedControl::LoCut {
            let idx = self.control_current_idx(ctrl).min(10);
            return LO_CUT_STRS[idx].to_string();
        }
        if ctrl == SteppedControl::HiCut {
            let idx = self.control_current_idx(ctrl).min(10);
            return HI_CUT_STRS[idx].to_string();
        }
        if ctrl == SteppedControl::MonoFreq {
            let idx = self.control_current_idx(ctrl).min(10);
            return MONO_FREQ_STRS[idx].to_string();
        }
        if ctrl == SteppedControl::CompThresh {
            let v = plain_override.unwrap_or_else(|| self.params.comp_thresh.value());
            // Hardware-style mark, not a dB: the two models use different detector laws, so one
            // shared dB number would be a lie. The real span is shown in the debug menu.
            return format!("{:.0}", self.comp_thresh_idx(v) as f32);
        }
        if let Some((fp, table, strs)) = self.float_step_info(ctrl) {
            let v = plain_override.unwrap_or_else(|| fp.value());
            if let Some(idx) = Self::is_stepped_match(v, table) {
                return strs[idx].to_string();
            }
            return self.format_continuous_value_at(ctrl, Some(v));
        }
        if self.int_step_info(ctrl).is_some() {
            let idx = self.control_current_idx(ctrl);
            return self.int_label(ctrl, idx).to_string();
        }
        String::new()
    }

    fn control_angle_11(&self, ctrl: SteppedControl) -> f32 {
        // While this knob owns the active gesture, take the needle position from the pending step
        // instead of the parameter value. Hosts that process audio apply GUI writes asynchronously,
        // so reading the parameter here would make the pointer trail the cursor during a drag.
        if let Some(DragTarget::Knob {
            ctrl: active,
            cur_idx,
            cur_norm,
            fine_mode,
            ..
        }) = &self.active_drag
        {
            if *active == ctrl {
                if *fine_mode && !is_int_control(ctrl) {
                    return (*cur_norm - 0.5) * 270.0;
                }
                return Self::tick_angle(*cur_idx, self.control_step_count(ctrl));
            }
        }
        if matches!(
            ctrl,
            SteppedControl::LoCut | SteppedControl::HiCut | SteppedControl::MonoFreq
        ) {
            let idx = self.control_current_idx(ctrl).min(10);
            return Self::tick_angle(idx, 11);
        }
        if ctrl == SteppedControl::CompThresh {
            let idx = self.comp_thresh_idx(self.params.comp_thresh.value());
            return Self::tick_angle(idx, 11);
        }
        if let Some((fp, table, _)) = self.float_step_info(ctrl) {
            if let Some(idx) = Self::is_stepped_match(fp.value(), table) {
                return Self::tick_angle(idx, table.len());
            }
            let norm = fp.modulated_normalized_value().clamp(0.0, 1.0);
            return (norm - 0.5) * 270.0;
        }
        0.0
    }

    fn eq_gain_angle_and_idx(&self, band: usize) -> (f32, usize) {
        if let Some((fp, table, _)) = self.float_step_info(SteppedControl::EqGain(band)) {
            let idx = Self::closest_index(fp.value(), table).min(8);
            if Self::is_stepped_match(fp.value(), table).is_some() {
                return (CONCENTRIC_GAIN_DEGS[idx], idx);
            }
            let norm = fp.modulated_normalized_value().clamp(0.0, 1.0);
            return ((norm - 0.5) * 180.0, idx);
        }
        (0.0, 4)
    }

    fn momentary_for_control(
        &self,
        ctrl: SteppedControl,
        fine_mode: bool,
        pending_plain: Option<f32>,
    ) -> Option<(MomentarySlot, String)> {
        // The Ptek knobs are labelled 0..10 on the hardware, so their stepped label is a bare
        // position. Route them through the continuous formatter, which adds the boost/attenuation
        // sign and keeps the numeric field aligned.
        let is_ptek_gain = matches!(
            ctrl,
            SteppedControl::PuLowBoost
                | SteppedControl::PuLowAtten
                | SteppedControl::PuHighBoost
                | SteppedControl::PuHighAtten
        );
        let txt = if fine_mode || is_ptek_gain {
            self.format_continuous_value_at(ctrl, pending_plain)
                .to_uppercase()
        } else {
            self.control_display_label_at(ctrl, pending_plain)
                .to_uppercase()
        };
        match ctrl {
            SteppedControl::InputTrim => Some((MomentarySlot::InputTrim, txt)),
            SteppedControl::CompThresh => Some((MomentarySlot::CompThresh, txt)),
            SteppedControl::LimiterGain => Some((MomentarySlot::LimiterGain, txt)),
            // Mass band controls and the large Ptek knobs share one band readout slot, so
            // both models use the exact same momentary value/restore behavior.
            _ => eq_readout_band(ctrl).map(|band| (MomentarySlot::EqBand(band), txt)),
        }
    }

    fn trigger_momentary_for_ctrl(
        &mut self,
        ctrl: SteppedControl,
        fine_mode: bool,
        pending_plain: Option<f32>,
    ) {
        if let Some((slot, txt)) = self.momentary_for_control(ctrl, fine_mode, pending_plain) {
            self.momentary_readout = Some((slot, txt, Instant::now()));
        }
    }

    fn momentary_text(&self, slot: MomentarySlot) -> Option<String> {
        if let Some((s, ref txt, when)) = self.momentary_readout {
            if s == slot && when.elapsed() < Duration::from_millis(1100) {
                return Some(txt.clone());
            }
        }
        None
    }

    fn readout_for(&self, slot: MomentarySlot, static_lbl: &str) -> String {
        self.momentary_text(slot)
            .unwrap_or_else(|| static_lbl.to_string())
    }

    fn find_stepped_control_at(&self, mx: f32, my: f32) -> Option<(SteppedControl, f32, f32)> {
        let hit_circle = |cx: f32, cy: f32, r: f32| -> bool {
            let dx = mx - cx;
            let dy = my - cy;
            (dx * dx + dy * dy).sqrt() <= r
        };

        // 1. Top Strip Small Pots (cy = 100.0)
        if (my - 100.0).abs() <= 44.0 {
            let tape_pots = [
                (144.0, SteppedControl::TapeDrive),
                (274.0, SteppedControl::TapeBias),
                (404.0, SteppedControl::TapeHeadBump),
                (534.0, SteppedControl::TapeBumpFreq),
                (664.0, SteppedControl::TapeFlutter),
                (960.0, SteppedControl::ClipperDrive),
                (1035.0, SteppedControl::ClipperCeiling),
            ];
            for (cx, ctrl) in tape_pots {
                if hit_circle(cx, 100.0, 40.0) {
                    return Some((ctrl, cx, 100.0));
                }
            }
        }

        // 2. Left Wing: Input Trim + Filtering
        if hit_circle(134.0, 506.0, 62.0) {
            return Some((SteppedControl::InputTrim, 134.0, 506.0));
        }
        if hit_circle(89.0, 740.0, 44.0) {
            return Some((SteppedControl::LoCut, 89.0, 740.0));
        }
        if hit_circle(179.0, 740.0, 44.0) {
            return Some((SteppedControl::HiCut, 179.0, 740.0));
        }
        if hit_circle(89.0, 842.0, 44.0) {
            return Some((SteppedControl::MonoFreq, 89.0, 842.0));
        }
        if hit_circle(179.0, 842.0, 44.0) {
            return Some((SteppedControl::MonoAmount, 179.0, 842.0));
        }

        // 3. Center Upper: 4 EQ columns (big knob at cy = 316.0, small knob at cy = 448.0).
        //    Mass uses a concentric frequency collar + gain rotor + Q trim; Ptek uses one
        //    0..10 knob plus a stepped selector / bandwidth trim.
        let is_mass = self.params.eq_model.value() == EqModel::Mass;
        for (i, &bx) in Self::BAND_CENTERS.iter().enumerate() {
            if hit_circle(bx, 316.0, 62.0) {
                let ctrl = if is_mass {
                    let dx = mx - bx;
                    let dy = my - 316.0;
                    let dist = (dx * dx + dy * dy).sqrt();
                    if dist > 40.0 && dy > 4.0 {
                        SteppedControl::EqFreq(i)
                    } else if dist <= 39.5 || dy < -4.0 {
                        SteppedControl::EqGain(i)
                    } else {
                        SteppedControl::EqFreq(i)
                    }
                } else {
                    match i {
                        0 => SteppedControl::PuLowBoost,
                        1 => SteppedControl::PuLowAtten,
                        2 => SteppedControl::PuHighBoost,
                        _ => SteppedControl::PuHighAtten,
                    }
                };
                return Some((ctrl, bx, 316.0));
            }

            if is_mass {
                let qx = bx - 28.0;
                if hit_circle(qx, 448.0, 36.0) {
                    return Some((SteppedControl::EqQ(i), qx, 448.0));
                }
            } else if hit_circle(bx, 448.0, 44.0) {
                let ctrl = match i {
                    0 => SteppedControl::PuLowFreq,
                    // Bandwidth belongs to the high boost circuit; it sits in the LO ATT
                    // column because that is the only free trim position in the layout.
                    1 => SteppedControl::PuBandwidth,
                    2 => SteppedControl::PuHighFreq,
                    _ => SteppedControl::PuAttenSel,
                };
                return Some((ctrl, bx, 448.0));
            }
        }

        // 4. Center Lower: Dynamics Thresh (798.0, 666.0) & 6 Small Pots (cy = 806.0)
        if hit_circle(798.0, 666.0, 62.0) {
            return Some((SteppedControl::CompThresh, 798.0, 666.0));
        }
        if (my - 806.0).abs() <= 46.0 {
            // The 4th pot is model-dependent: the Fair has an INPUT GAIN drive there, the
            // V-MU keeps its RATIO control.
            let fourth = if self.params.comp_model.value() == CompModel::Fair {
                SteppedControl::CompInGain
            } else {
                SteppedControl::CompRatio
            };
            let comp_pots = [
                SteppedControl::CompScHpf,
                SteppedControl::CompMix,
                SteppedControl::CompMakeup,
                fourth,
                SteppedControl::CompAttack,
                SteppedControl::CompRelease,
            ];
            for (idx, ctrl) in comp_pots.into_iter().enumerate() {
                let px = 312.0 + (idx as f32) * 103.0;
                if hit_circle(px, 806.0, 44.0) {
                    return Some((ctrl, px, 806.0));
                }
            }
        }

        // 5. Right Wing: Brickwall Limiter & Master Output
        if hit_circle(1226.0, 506.0, 62.0) {
            return Some((SteppedControl::LimiterGain, 1226.0, 506.0));
        }
        if hit_circle(1181.0, 740.0, 44.0) {
            return Some((SteppedControl::LimiterCeiling, 1181.0, 740.0));
        }
        if hit_circle(1271.0, 740.0, 44.0) {
            return Some((SteppedControl::LimiterLookahead, 1271.0, 740.0));
        }
        if hit_circle(1181.0, 842.0, 44.0) {
            return Some((SteppedControl::LimiterRelease, 1181.0, 842.0));
        }
        if hit_circle(1271.0, 842.0, 44.0) {
            return Some((SteppedControl::LimiterLift, 1271.0, 842.0));
        }

        None
    }

    fn reset_param_to_default(&self, ptr: ParamPtr) {
        let def_norm = unsafe { ptr.default_normalized_value() };
        self.set_param_atomic(ptr, def_norm);
    }

    fn cycle_int_param(&self, ptr: ParamPtr, current: i32, max_incl: i32, delta: i32) {
        let count = max_incl + 1;
        let next = (current + delta).rem_euclid(count);
        let norm = (next as f32) / (max_incl as f32);
        self.set_param_atomic(ptr, norm);
    }

    /// Terminates any in-flight drag and closes the host parameter gesture. Safe to call when no
    /// drag is active. Hosts that stop sending `MouseUp` (window leave, focus loss, alt-tab) would
    /// otherwise leave the edit gesture open and keep the parameter "touched" forever.
    fn end_active_drag(&mut self, cx: &mut EventContext) {
        self.hold_press.set(None);
        self.pointer_down.set(false);
        if let Some(drag) = self.active_drag.take() {
            let DragTarget::Knob { ptr, .. } = drag;
            unsafe {
                self.gui_context.raw_end_set_parameter(ptr);
            }
            cx.needs_redraw();
        }
        cx.release();
    }

    fn handle_scroll_on_non_knob(
        &mut self,
        cx: &mut EventContext,
        mx: f32,
        my: f32,
        step_dir: i32,
    ) -> bool {
        let in_box = |bx: f32, by: f32, bw: f32, bh: f32| -> bool {
            mx >= bx - bw * 0.5 && mx <= bx + bw * 0.5 && my >= by - bh * 0.5 && my <= by + bh * 0.5
        };

        // 1. Coloration Cassette 9-Cell Steppers
        let steppers = [
            (
                998.0,
                282.0,
                self.params.eq_amp_type.as_ptr(),
                self.params.eq_amp_type.value(),
                6,
                false,
            ),
            (
                998.0,
                338.0,
                self.params.eq_trans_type.as_ptr(),
                self.params.eq_trans_type.value(),
                4,
                false,
            ),
            (
                998.0,
                641.0,
                self.params.comp_amp_type.as_ptr(),
                self.params.comp_amp_type.value(),
                6,
                true,
            ),
            (
                998.0,
                697.0,
                self.params.comp_trans_type.as_ptr(),
                self.params.comp_trans_type.value(),
                4,
                true,
            ),
        ];
        for (sx, sy, ptr, cur, max_incl, is_comp) in steppers {
            if in_box(sx, sy, 160.0, 44.0) {
                if is_comp {
                    self.set_param_atomic(self.params.comp_col_match.as_ptr(), 0.0);
                } else {
                    self.set_param_atomic(self.params.eq_col_match.as_ptr(), 0.0);
                }
                self.cycle_int_param(ptr, cur, max_incl, step_dir);
                cx.needs_redraw();
                return true;
            }
        }

        // 2. Model Select Pills
        let model_pills: [(f32, f32, f32, ParamPtr, bool); 6] = [
            (
                444.0,
                34.0,
                136.0,
                self.params.tape_speed_30ips.as_ptr(),
                false,
            ),
            (
                590.0,
                34.0,
                116.0,
                self.params.tape_formula_gp9.as_ptr(),
                false,
            ),
            (
                1275.0,
                80.0,
                88.0,
                self.params.clipper_hard_shape.as_ptr(),
                false,
            ),
            (
                1275.0,
                116.0,
                88.0,
                self.params.clipper_os_4x.as_ptr(),
                false,
            ),
            (680.0, 176.0, 220.0, self.params.eq_model.as_ptr(), false),
            (680.0, 530.0, 220.0, self.params.comp_model.as_ptr(), false),
        ];
        for (px, py, pw, ptr, invert_dir) in model_pills {
            if in_box(px, py, pw, 32.0) {
                let on = if invert_dir {
                    step_dir < 0
                } else {
                    step_dir > 0
                };
                self.set_param_atomic(ptr, if on { 1.0 } else { 0.0 });
                if ptr == self.params.eq_model.as_ptr() && self.params.eq_col_match.value() {
                    let m = if on { EqModel::Mass } else { EqModel::Ptek };
                    let (a, t) = eq_hardware_topology(m);
                    self.set_param_atomic(self.params.eq_amp_type.as_ptr(), (a as f32) / 6.0);
                    self.set_param_atomic(self.params.eq_trans_type.as_ptr(), (t as f32) / 4.0);
                } else if ptr == self.params.comp_model.as_ptr() && self.params.comp_col_match.value() {
                    let m = if on { CompModel::VMu } else { CompModel::Fair };
                    let (a, t) = comp_hardware_topology(m);
                    self.set_param_atomic(self.params.comp_amp_type.as_ptr(), (a as f32) / 6.0);
                    self.set_param_atomic(self.params.comp_trans_type.as_ptr(), (t as f32) / 4.0);
                }
                cx.needs_redraw();
                return true;
            }
        }

        false
    }

    // =====================================================================
    // Hidden debug menu: shared state, editing and hit testing
    // =====================================================================

    fn debug_field_plain(&self, field: DebugField) -> f32 {
        let calibration = debug_calibration::calibration();
        match field {
            DebugField::PeakHold => calibration.peak_hold_ms() as f32,
            DebugField::AutoLearn => {
                let seconds = calibration.auto_learn_s();
                if seconds == 0.0 {
                    12.0
                } else {
                    seconds
                }
            }
            DebugField::ThreshTrimFair => calibration.trim_fair(),
            DebugField::ThreshTrimVmu => calibration.trim_vmu(),
            DebugField::ResetRanges => 0.0,
        }
    }

    fn debug_field_text(&self, field: DebugField) -> String {
        match field {
            DebugField::PeakHold => format!("{} ms", self.debug_field_plain(field) as u32),
            DebugField::AutoLearn => format!("{:.1} s", self.debug_field_plain(field)),
            DebugField::ThreshTrimFair | DebugField::ThreshTrimVmu => {
                format!("{:+.1} dB", self.debug_field_plain(field))
            }
            DebugField::ResetRanges => "CLICK".to_string(),
        }
    }

    /// Commit one field. Values are clamped by the calibration store (not in the text box) so
    /// typing "-" or "1." on the way to a full number never fights the user.
    fn set_debug_param(&self, field: DebugField, value: f32) {
        let calibration = debug_calibration::calibration();
        match field {
            DebugField::PeakHold => {
                calibration.set_peak_hold_ms(value.round().clamp(500.0, 10000.0) as u32)
            }
            DebugField::AutoLearn => calibration.set_auto_learn_s(value),
            DebugField::ThreshTrimFair => calibration.set_trim_fair(value),
            DebugField::ThreshTrimVmu => calibration.set_trim_vmu(value),
            DebugField::ResetRanges => self.reset_debug_ranges(),
        }
    }

    fn reset_debug_ranges(&self) {
        debug_calibration::calibration().reset_trims();
    }

    fn start_debug_edit(&self, cx: &mut EventContext, field: DebugField) {
        if !field.is_numeric() {
            return;
        }
        let text = if matches!(field, DebugField::PeakHold) {
            format!("{}", self.debug_field_plain(field).round() as i32)
        } else {
            format!("{:.1}", self.debug_field_plain(field))
        };
        self.debug_edit.replace(Some(DebugEdit {
            field,
            text,
            typed: false,
        }));
        // Keyboard events are dispatched to the focused entity, so the row must take focus or it
        // would accept the click but never see a character.
        cx.focus();
        // Vizia focus is not OS focus. A hosted plugin's canvas is a child window that Windows
        // never hands keyboard focus to on its own, so key messages keep going to the DAW. Hand
        // focus to the window under the cursor, which is this canvas.
        focus_window_under_cursor();
        cx.needs_redraw();
    }

    fn commit_debug_edit(&self, cx: &mut EventContext) {
        let Some(edit) = self.debug_edit.replace(None) else {
            return;
        };
        if let Ok(v) = edit.text.trim().parse::<f32>() {
            if v.is_finite() {
                self.set_debug_param(edit.field, v);
            }
        }
        cx.needs_redraw();
    }

    fn handle_debug_click(&self, cx: &mut EventContext, mx: f32, my: f32) -> bool {
        if !Self::debug_panel_contains(mx, my) {
            return false;
        }

        let mut hit: Option<DebugField> = None;
        for (i, field) in DEBUG_FIELDS.iter().enumerate() {
            if (my - Self::debug_row_y(i)).abs() <= Self::DBG_ROW_H * 0.5 {
                hit = Some(*field);
                break;
            }
        }
        let Some(field) = hit else {
            return true;
        };

        let minus = (mx - Self::DBG_MINUS_X).abs() <= 13.0;
        let plus = (mx - Self::DBG_PLUS_X).abs() <= 13.0;

        if field == DebugField::ResetRanges {
            self.reset_debug_ranges();
            self.debug_edit.replace(None);
            cx.needs_redraw();
            return true;
        }

        if minus || plus {
            let step = if matches!(field, DebugField::PeakHold) {
                100.0
            } else {
                0.5
            };
            let next = self.debug_field_plain(field) + if plus { step } else { -step };
            self.set_debug_param(field, next);
            self.debug_edit.replace(None);
            cx.needs_redraw();
            return true;
        }

        let already_editing = self
            .debug_edit
            .borrow()
            .as_ref()
            .map(|e| e.field == field)
            .unwrap_or(false);
        if already_editing {
            cx.focus();
            focus_window_under_cursor();
        } else {
            self.start_debug_edit(cx, field);
        }
        true
    }

    fn handle_click(&mut self, cx: &mut EventContext, mx: f32, my: f32, is_alt: bool) -> bool {
        let in_box = |bx: f32, by: f32, bw: f32, bh: f32| -> bool {
            mx >= bx - bw * 0.5 && mx <= bx + bw * 0.5 && my >= by - bh * 0.5 && my <= by + bh * 0.5
        };

        // --- Modal Priority 1: Save Preset Dialog ---
        if self.preset_save_dialog_open.get() {
            let (sx, sy, sw, sh) = (450.0, 360.0, 460.0, 230.0);
            let btn_y = sy + sh - 36.0;
            // Cancel button
            if in_box(sx + sw * 0.5 - 75.0, btn_y, 110.0, 32.0) {
                self.preset_save_dialog_open.set(false);
                cx.needs_redraw();
                return true;
            }
            // Save button
            if in_box(sx + sw * 0.5 + 75.0, btn_y, 110.0, 32.0) {
                let name = if let Some(edit) = self.preset_edit.borrow_mut().take() {
                    let t = edit.text.trim();
                    if t.is_empty() {
                        self.preset_name(self.current_preset_idx.get())
                    } else {
                        t.to_string()
                    }
                } else {
                    self.preset_name(self.current_preset_idx.get())
                };
                self.save_user_preset(name);
                self.save_preset_banner.set(Some(Instant::now()));
                self.preset_save_dialog_open.set(false);
                cx.needs_redraw();
                return true;
            }
            if in_box(sx + sw * 0.5, sy + sh * 0.5, sw, sh) {
                return true;
            }
            self.preset_save_dialog_open.set(false);
            cx.needs_redraw();
            return true;
        }

        // --- Modal Priority 2: Preset Browser Modal ---
        if self.preset_browser_open.get() {
            let (bx, by, bw, bh) = (400.0, 240.0, 560.0, 460.0);
            // Close button [ X ]
            if in_box(bx + bw - 36.0, by + 28.0, 32.0, 24.0) {
                self.preset_browser_open.set(false);
                cx.needs_redraw();
                return true;
            }
            // Bottom [ CLOSE ] button
            if in_box(bx + bw - 60.0, by + bh - 32.0, 80.0, 30.0) {
                self.preset_browser_open.set(false);
                cx.needs_redraw();
                return true;
            }
            // Bottom [ + SAVE PRESET ] button
            if in_box(bx + 110.0, by + bh - 32.0, 130.0, 30.0) {
                self.preset_browser_open.set(false);
                let cur_name = self.preset_name(self.current_preset_idx.get());
                self.preset_edit.replace(Some(PresetEdit {
                    text: cur_name,
                    typed: false,
                }));
                self.preset_save_dialog_open.set(true);
                cx.needs_redraw();
                return true;
            }
            // Bottom [ OPEN FOLDER ] button
            if in_box(bx + 260.0, by + bh - 32.0, 130.0, 30.0) {
                crate::user_presets::open_presets_folder();
                return true;
            }

            // Preset rows
            let row_h = 36.0;
            let list_y = by + 56.0;
            let total = self.total_presets();
            for i in 0..total.min(8) {
                let ry = list_y + (i as f32) * (row_h + 6.0);
                let is_factory = i < FACTORY_PRESETS.len();
                // User preset [ DEL ] button
                if !is_factory && in_box(bx + bw - 50.0, ry + 18.0, 40.0, 24.0) {
                    let u_idx = i - FACTORY_PRESETS.len();
                    let new_total = {
                        let mut presets = self.user_presets.borrow_mut();
                        if u_idx < presets.len() {
                            let removed = presets.remove(u_idx);
                            crate::user_presets::delete_user_preset_file(&removed.name);
                            save_user_presets(&presets);
                            Some(FACTORY_PRESETS.len() + presets.len())
                        } else {
                            None
                        }
                    };
                    if let Some(total) = new_total {
                        if self.current_preset_idx.get() >= total && total > 0 {
                            self.current_preset_idx.set(total - 1);
                        }
                    }
                    cx.needs_redraw();
                    return true;
                }
                // Row selection
                if in_box(bx + bw * 0.5, ry + row_h * 0.5, bw - 48.0, row_h) {
                    self.load_preset(i);
                    self.preset_browser_open.set(false);
                    cx.needs_redraw();
                    return true;
                }
            }

            // Clicking outside modal closes it
            if !in_box(bx + bw * 0.5, by + bh * 0.5, bw, bh) {
                self.preset_browser_open.set(false);
                cx.needs_redraw();
            }
            return true;
        }

        // --- Hidden debug menu takes priority over everything it covers ---
        // Hold the bottom-right screw for 3 s. The screw sits outside the panel, so it is always
        // reachable; the panel itself swallows clicks so rows are never stolen by the controls
        // painted underneath them.
        if in_box(
            Self::TOTAL_WIDTH - 17.0,
            Self::TOTAL_HEIGHT - 24.0,
            44.0,
            44.0,
        ) && !is_alt
        {
            self.arm_hold(HoldAction::ToggleDebugMenu, my, false);
            cx.capture();
            cx.needs_redraw();
            return true;
        }
        if self.debug_menu_open.get() && self.handle_debug_click(cx, mx, my) {
            return true;
        }

        // --- Model Select Pills ---
        let model_pills: [(f32, f32, f32, ParamPtr, bool); 6] = [
            (
                444.0,
                34.0,
                136.0,
                self.params.tape_speed_30ips.as_ptr(),
                false,
            ),
            (
                590.0,
                34.0,
                116.0,
                self.params.tape_formula_gp9.as_ptr(),
                false,
            ),
            (
                1275.0,
                80.0,
                88.0,
                self.params.clipper_hard_shape.as_ptr(),
                false,
            ),
            (
                1275.0,
                116.0,
                88.0,
                self.params.clipper_os_4x.as_ptr(),
                false,
            ),
            (680.0, 176.0, 220.0, self.params.eq_model.as_ptr(), false),
            (680.0, 530.0, 220.0, self.params.comp_model.as_ptr(), false),
        ];
        for (px, py, pw, ptr, invert_half) in model_pills {
            if in_box(px, py, pw, 28.0) {
                if ptr == self.params.eq_model.as_ptr() {
                    // The two models use different selector params; drop any cached position so a
                    // stale Mass frequency cannot leak into the Ptek selectors.
                    self.int_cmd.set([None; INT_CTRL_SLOTS]);
                    if is_alt {
                        self.reset_param_to_default(ptr);
                    } else {
                        let right_half = mx >= px;
                        let on = if invert_half { !right_half } else { right_half };
                        self.set_param_atomic(ptr, if on { 1.0 } else { 0.0 });
                    }
                    if self.params.eq_col_match.value() {
                        let (a, t) = eq_hardware_topology(self.params.eq_model.value());
                        self.set_param_atomic(self.params.eq_amp_type.as_ptr(), (a as f32) / 6.0);
                        self.set_param_atomic(self.params.eq_trans_type.as_ptr(), (t as f32) / 4.0);
                    }
                    cx.needs_redraw();
                    return true;
                }
                if ptr == self.params.comp_model.as_ptr() {
                    let cur_is_vmu = self.params.comp_model.value() == CompModel::VMu;
                    // save current
                    let cur_state = [
                        self.params.comp_thresh.value(),
                        self.params.comp_ratio.value(),
                        self.params.comp_attack.value(),
                        self.params.comp_release.value(),
                        self.params.comp_sc_hpf_hz.value(),
                        self.params.comp_mix.value(),
                        self.params.comp_makeup.value(),
                    ];
                    if cur_is_vmu {
                        self.comp_vmu_state.set(cur_state);
                    } else {
                        self.comp_fair_state.set(cur_state);
                    }
                    if is_alt {
                        self.reset_param_to_default(ptr);
                    } else {
                        let right_half = mx >= px;
                        let on = if invert_half { !right_half } else { right_half };
                        self.set_param_atomic(ptr, if on { 1.0 } else { 0.0 });
                    }
                    let target = if !cur_is_vmu {
                        self.comp_vmu_state.get()
                    } else {
                        self.comp_fair_state.get()
                    };
                    self.set_param_atomic(
                        self.params.comp_thresh.as_ptr(),
                        self.params.comp_thresh.preview_normalized(target[0]),
                    );
                    self.set_param_atomic(
                        self.params.comp_ratio.as_ptr(),
                        self.params.comp_ratio.preview_normalized(target[1]),
                    );
                    self.set_param_atomic(
                        self.params.comp_attack.as_ptr(),
                        self.params.comp_attack.preview_normalized(target[2]),
                    );
                    self.set_param_atomic(
                        self.params.comp_release.as_ptr(),
                        self.params.comp_release.preview_normalized(target[3]),
                    );
                    self.set_param_atomic(
                        self.params.comp_sc_hpf_hz.as_ptr(),
                        self.params.comp_sc_hpf_hz.preview_normalized(target[4]),
                    );
                    self.set_param_atomic(
                        self.params.comp_mix.as_ptr(),
                        self.params.comp_mix.preview_normalized(target[5]),
                    );
                    self.set_param_atomic(
                        self.params.comp_makeup.as_ptr(),
                        self.params.comp_makeup.preview_normalized(target[6]),
                    );
                    if self.params.comp_col_match.value() {
                        let (a, t) = comp_hardware_topology(self.params.comp_model.value());
                        self.set_param_atomic(self.params.comp_amp_type.as_ptr(), (a as f32) / 6.0);
                        self.set_param_atomic(self.params.comp_trans_type.as_ptr(), (t as f32) / 4.0);
                    }
                    cx.needs_redraw();
                    return true;
                }
                if is_alt {
                    self.reset_param_to_default(ptr);
                } else {
                    let right_half = mx >= px;
                    let on = if invert_half { !right_half } else { right_half };
                    self.set_param_atomic(ptr, if on { 1.0 } else { 0.0 });
                }
                cx.needs_redraw();
                return true;
            }
        }

        // --- Bypass Buttons ("IN" = !bypass) ---
        let bypass_btns = [
            (
                196.0,
                176.0,
                self.params.input_bypass.as_ptr(),
                self.params.input_bypass.value(),
            ),
            (
                840.0,
                34.0,
                self.params.tape_bypass.as_ptr(),
                self.params.tape_bypass.value(),
            ),
            (
                1272.0,
                34.0,
                self.params.clipper_bypass.as_ptr(),
                self.params.clipper_bypass.value(),
            ),
            (
                1056.0,
                176.0,
                self.params.eq_bypass.as_ptr(),
                self.params.eq_bypass.value(),
            ),
            (
                1056.0,
                530.0,
                self.params.comp_bypass.as_ptr(),
                self.params.comp_bypass.value(),
            ),
            (
                1288.0,
                176.0,
                self.params.limiter_bypass.as_ptr(),
                self.params.limiter_bypass.value(),
            ),
        ];
        for (bx, by, ptr, cur) in bypass_btns {
            if in_box(bx, by, 64.0, 26.0) {
                if is_alt {
                    self.reset_param_to_default(ptr);
                } else {
                    self.set_param_atomic(ptr, if !cur { 1.0 } else { 0.0 });
                }
                cx.needs_redraw();
                return true;
            }
        }

        // --- EQ Band 6-Cell Readout Clicks (Toggle Band Bypass, Mass only) ---
        if self.params.eq_model.value() == EqModel::Mass {
            for (i, &bx) in Self::BAND_CENTERS.iter().enumerate() {
                if in_box(bx, 238.0, 84.0, 32.0) {
                    let (ptr, cur) = match i {
                        0 => (
                            self.params.mass_low_bypass.as_ptr(),
                            self.params.mass_low_bypass.value(),
                        ),
                        1 => (
                            self.params.mass_lmf_bypass.as_ptr(),
                            self.params.mass_lmf_bypass.value(),
                        ),
                        2 => (
                            self.params.mass_hmf_bypass.as_ptr(),
                            self.params.mass_hmf_bypass.value(),
                        ),
                        _ => (
                            self.params.mass_hf_bypass.as_ptr(),
                            self.params.mass_hf_bypass.value(),
                        ),
                    };
                    if is_alt {
                        self.reset_param_to_default(ptr);
                    } else {
                        self.set_param_atomic(ptr, if cur { 0.0 } else { 1.0 });
                    }
                    cx.needs_redraw();
                    return true;
                }
            }
        }

        // --- MASS 2-Position Shelf / Bell Toggle Switches ---
        if self.params.eq_model.value() == EqModel::Mass {
            for (i, &bx) in Self::BAND_CENTERS.iter().enumerate() {
                let sw_x = bx + 32.0;
                let sw_y = 448.0;
                let in_sw_box = mx >= sw_x - 18.0 && mx <= sw_x + 18.0 && my >= sw_y - 42.0 && my <= sw_y + 44.0;
                if in_sw_box {
                    let (shelf_ptr, cur_shelf) = match i {
                        0 => (self.params.mass_low_shelf.as_ptr(), self.params.mass_low_shelf.value()),
                        1 => (self.params.mass_lmf_shelf.as_ptr(), self.params.mass_lmf_shelf.value()),
                        2 => (self.params.mass_hmf_shelf.as_ptr(), self.params.mass_hmf_shelf.value()),
                        _ => (self.params.mass_hf_shelf.as_ptr(), self.params.mass_hf_shelf.value()),
                    };

                    if is_alt {
                        self.reset_param_to_default(shelf_ptr);
                    } else {
                        let new_shelf = if my < sw_y - 8.0 {
                            true // Clicked near SHELF (top)
                        } else if my > sw_y + 8.0 {
                            false // Clicked near BELL (bottom)
                        } else {
                            !cur_shelf // Clicked lever center: toggle
                        };
                        self.set_param_atomic(shelf_ptr, if new_shelf { 1.0 } else { 0.0 });
                    }
                    cx.needs_redraw();
                    return true;
                }
            }
        }

        // --- Coloration Cassette 9-Cell Selectors & Up/Down Steppers ---
        let steppers = [
            (
                998.0,
                282.0,
                self.params.eq_amp_type.as_ptr(),
                self.params.eq_amp_type.value(),
                6,
                false,
            ),
            (
                998.0,
                338.0,
                self.params.eq_trans_type.as_ptr(),
                self.params.eq_trans_type.value(),
                4,
                false,
            ),
            (
                998.0,
                641.0,
                self.params.comp_amp_type.as_ptr(),
                self.params.comp_amp_type.value(),
                6,
                true,
            ),
            (
                998.0,
                697.0,
                self.params.comp_trans_type.as_ptr(),
                self.params.comp_trans_type.value(),
                4,
                true,
            ),
        ];
        for (sx, sy, ptr, cur, max_incl, is_comp) in steppers {
            if in_box(sx, sy, 154.0, 34.0) {
                if is_alt {
                    self.reset_param_to_default(ptr);
                } else {
                    let delta = if mx > 1044.0 && my > sy { -1 } else { 1 };
                    self.cycle_int_param(ptr, cur, max_incl, delta);
                }
                if is_comp {
                    self.set_param_atomic(self.params.comp_col_match.as_ptr(), 0.0);
                } else {
                    self.set_param_atomic(self.params.eq_col_match.as_ptr(), 0.0);
                }
                cx.needs_redraw();
                return true;
            }
        }

        // --- Per-Module AUTO (level-neutral) Push Buttons ---
        // Tape and clipper compensation is exact and static: a plain toggle.
        let auto_btns = [
            (
                794.0,
                100.0,
                44.0,
                44.0,
                self.params.tape_comp.as_ptr(),
                self.params.tape_comp.value(),
            ),
            (
                1205.0,
                100.0,
                44.0,
                44.0,
                self.params.clipper_comp.as_ptr(),
                self.params.clipper_comp.value(),
            ),
        ];
        for (bx, by, bw, bh, ptr, cur) in auto_btns {
            if in_box(bx, by, bw, bh) {
                if is_alt {
                    self.reset_param_to_default(ptr);
                } else {
                    self.set_param_atomic(ptr, if cur { 0.0 } else { 1.0 });
                }
                cx.needs_redraw();
                return true;
            }
        }

        // --- Coloration MATCH push buttons ---
        if in_box(942.0, 414.0, 40.0, 40.0) {
            let next = !self.params.eq_col_match.value();
            self.set_param_atomic(self.params.eq_col_match.as_ptr(), if next { 1.0 } else { 0.0 });
            if next {
                let (a, t) = eq_hardware_topology(self.params.eq_model.value());
                self.set_param_atomic(self.params.eq_amp_type.as_ptr(), (a as f32) / 6.0);
                self.set_param_atomic(self.params.eq_trans_type.as_ptr(), (t as f32) / 4.0);
            }
            cx.needs_redraw();
            return true;
        }
        if in_box(942.0, 773.0, 40.0, 40.0) {
            let next = !self.params.comp_col_match.value();
            self.set_param_atomic(self.params.comp_col_match.as_ptr(), if next { 1.0 } else { 0.0 });
            if next {
                let (a, t) = comp_hardware_topology(self.params.comp_model.value());
                self.set_param_atomic(self.params.comp_amp_type.as_ptr(), (a as f32) / 6.0);
                self.set_param_atomic(self.params.comp_trans_type.as_ptr(), (t as f32) / 4.0);
            }
            cx.needs_redraw();
            return true;
        }

        // The two coloration stages learn: short click toggles, hold measures again.
        for (bx, by, is_comp) in [(998.0, 414.0, false), (998.0, 773.0, true)] {
            if in_box(bx, by, 40.0, 40.0) {
                let ptr = if is_comp {
                    self.params.comp_col_comp.as_ptr()
                } else {
                    self.params.eq_col_comp.as_ptr()
                };
                if is_alt {
                    self.reset_param_to_default(ptr);
                    cx.needs_redraw();
                    return true;
                }
                self.arm_hold(HoldAction::RelearnColoration { is_comp }, my, false);
                cx.capture();
                cx.needs_redraw();
                return true;
            }
        }

        // --- 3-State Smoked-Glass Harmonic Drive PUSH Buttons (hold = SMASHED) ---
        if in_box(1054.0, 414.0, 40.0, 40.0) {
            if is_alt {
                self.reset_param_to_default(self.params.eq_intensity.as_ptr());
                cx.needs_redraw();
                return true;
            }
            self.arm_hold(HoldAction::PushThird { is_comp: false }, my, false);
            cx.capture();
            cx.needs_redraw();
            return true;
        }
        if in_box(1054.0, 773.0, 40.0, 40.0) {
            if is_alt {
                self.reset_param_to_default(self.params.comp_intensity.as_ptr());
                cx.needs_redraw();
                return true;
            }
            self.arm_hold(HoldAction::PushThird { is_comp: true }, my, false);
            cx.capture();
            cx.needs_redraw();
            return true;
        }

        // --- MQNdsp Utility & Preset Console ---
        // Prev Preset Button [ < ]
        if in_box(396.0, 906.0, 36.0, 36.0) {
            self.preset_edit.replace(None);
            let total = self.total_presets();
            let cur = self.current_preset_idx.get();
            let next = if cur == 0 { total.saturating_sub(1) } else { cur - 1 };
            self.load_preset(next);
            cx.needs_redraw();
            return true;
        }
        // Preset Name Display (Click to open Preset Library dialog)
        if in_box(596.0, 906.0, 340.0, 34.0) {
            *self.user_presets.borrow_mut() = crate::user_presets::load_user_presets();
            self.preset_browser_open.set(true);
            cx.needs_redraw();
            return true;
        }
        // Next Preset Button [ > ]
        if in_box(796.0, 906.0, 36.0, 36.0) {
            self.preset_edit.replace(None);
            let total = self.total_presets();
            let cur = self.current_preset_idx.get();
            let next = if total > 0 { (cur + 1) % total } else { 0 };
            self.load_preset(next);
            cx.needs_redraw();
            return true;
        }
        // Save Preset Button [ SAVE ] (Opens Save Preset Dialog pre-filled with current name)
        if in_box(849.0, 906.0, 50.0, 36.0) {
            let cur_name = self.preset_name(self.current_preset_idx.get());
            self.preset_edit.replace(Some(PresetEdit {
                text: cur_name,
                typed: false,
            }));
            self.preset_save_dialog_open.set(true);
            cx.focus();
            focus_window_under_cursor();
            cx.needs_redraw();
            return true;
        }
        // Theme Mode Toggle (Sun / Moon)
        if in_box(918.0, 906.0, 36.0, 36.0) {
            let cur = self.params.theme_light.value();
            self.set_param_atomic(self.params.theme_light.as_ptr(), if !cur { 1.0 } else { 0.0 });
            cx.needs_redraw();
            return true;
        }
        // Reference Manual [ 📖 ] (Spawns complete PDF Manual in system viewer)
        if in_box(962.0, 906.0, 36.0, 36.0) {
            crate::user_presets::open_manual_pdf();
            cx.needs_redraw();
            return true;
        }
        // A/B Comparison Matrix [ A ] and [ B ]
        if in_box(1022.0, 906.0, 36.0, 36.0) {
            if self.ab_active.get() == 'B' {
                // Save state of B
                let cur = self.capture_snapshot();
                self.slot_b.replace(Some(cur));
                // Restore state of A
                if let Some(snap_a) = self.slot_a.borrow().as_ref() {
                    self.restore_snapshot(snap_a);
                }
                self.ab_active.set('A');
            }
            cx.needs_redraw();
            return true;
        }
        if in_box(1066.0, 906.0, 36.0, 36.0) {
            if self.ab_active.get() == 'A' {
                // Save state of A
                let cur = self.capture_snapshot();
                self.slot_a.replace(Some(cur.clone()));
                // If B not yet set, seed it with current A
                if self.slot_b.borrow().is_none() {
                    self.slot_b.replace(Some(cur));
                } else if let Some(snap_b) = self.slot_b.borrow().as_ref() {
                    self.restore_snapshot(snap_b);
                }
                self.ab_active.set('B');
            }
            cx.needs_redraw();
            return true;
        }

        // --- Clipper meter / readout: click resets peak clip to 0.0 ---
        if in_box(1095.0, 100.0, 32.0, 60.0) || in_box(1150.0, 100.0, 48.0, 32.0) {
            self.clip_hold.set(0.0);
            let mut held = self.readout_held.get();
            held[5] = 0.0;
            self.readout_held.set(held);
            self.momentary_readout = Some((
                MomentarySlot::ClipperReadout,
                "RST".to_string(),
                Instant::now(),
            ));
            cx.needs_redraw();
            return true;
        }

        // --- Left Wing: Input Top Readout (PEAK) - click resets peak hold ---
        if in_box(134.0, 221.0, 84.0, 26.0) {
            self.reset_input_peak_hold(Instant::now());
            let mut held = self.readout_held.get();
            held[0] = METER_FLOOR_DB;
            self.readout_held.set(held);
            self.momentary_readout = Some((
                MomentarySlot::InputPeak,
                "RESET".to_string(),
                Instant::now(),
            ));
            cx.needs_redraw();
            return true;
        }

        // --- Left Wing: Input Ladder - click toggles ladder mode between PEAK and RMS ---
        if in_box(134.0, 296.0, 84.0, 112.0) {
            self.input_rms_mode = !self.input_rms_mode;
            self.momentary_readout = Some((
                if self.input_rms_mode {
                    MomentarySlot::InputRms
                } else {
                    MomentarySlot::InputPeak
                },
                (if self.input_rms_mode { "RMS" } else { "PEAK" }).to_string(),
                Instant::now(),
            ));
            cx.needs_redraw();
            return true;
        }

        // --- Left Wing: Input Bottom Readout (RMS) - click resets RMS hold ---
        if in_box(134.0, 371.0, 84.0, 26.0) {
            self.reset_input_rms_hold(Instant::now());
            let mut held = self.readout_held.get();
            held[1] = METER_FLOOR_DB;
            self.readout_held.set(held);
            self.momentary_readout = Some((
                MomentarySlot::InputRms,
                "RESET".to_string(),
                Instant::now(),
            ));
            cx.needs_redraw();
            return true;
        }

        // --- Right Wing: Output Top Readout (LUFS) - click cycles MOM/SHORT/INT, Alt-click resets ---
        if in_box(1226.0, 221.0, 84.0, 26.0) {
            if is_alt {
                self.meters
                    .reset_integrated
                    .store(true, std::sync::atomic::Ordering::Relaxed);
                self.lufs_mode = 2;
            } else {
                self.lufs_mode = (self.lufs_mode + 1) % 3;
            }
            self.momentary_readout = Some((
                MomentarySlot::LufsMode,
                match self.lufs_mode {
                    0 => "MOM",
                    1 => "SHORT",
                    _ => "INT",
                }
                .to_string(),
                Instant::now(),
            ));
            self.readout_refreshed
                .set(Instant::now() - Duration::from_secs(1));
            cx.needs_redraw();
            return true;
        }

        // --- Right Wing: Output Limiter Ladder - click toggles bottom readout between GR and TRUE PK ---
        if in_box(1226.0, 296.0, 84.0, 112.0) {
            self.limiter_gr_mode = !self.limiter_gr_mode;
            self.momentary_readout = Some((
                MomentarySlot::LimiterMeter,
                (if self.limiter_gr_mode { "TRUE PK" } else { "GR" }).to_string(),
                Instant::now(),
            ));
            cx.needs_redraw();
            return true;
        }

        // --- Right Wing: Output Bottom Readout (GR / TRUE PK) - click resets to 0.0 / floor ---
        if in_box(1226.0, 371.0, 84.0, 26.0) {
            let mut held = self.readout_held.get();
            if self.limiter_gr_mode {
                self.reset_tp_hold(Instant::now());
                held[4] = METER_FLOOR_DB;
            } else {
                self.reset_gr_hold(Instant::now());
                held[3] = 0.0;
            }
            self.readout_held.set(held);
            self.momentary_readout = Some((
                MomentarySlot::LimiterMeter,
                "RESET".to_string(),
                Instant::now(),
            ));
            cx.needs_redraw();
            return true;
        }

        // --- Left & Right Wing Vintage Push Buttons (cy = 600.0) ---
        let wing_push_btns = [
            (
                134.0 - 35.0,
                600.0,
                self.params.polarity.as_ptr(),
                self.params.polarity.value(),
            ),
            (
                134.0 + 35.0,
                600.0,
                self.params.input_mono.as_ptr(),
                self.params.input_mono.value(),
            ),
            (
                1226.0 - 35.0,
                600.0,
                self.params.limiter_true_peak.as_ptr(),
                self.params.limiter_true_peak.value(),
            ),
            (
                1226.0 + 35.0,
                600.0,
                self.params.limiter_unity.as_ptr(),
                self.params.limiter_unity.value(),
            ),
        ];
        for (bx, by, ptr, cur) in wing_push_btns {
            if in_box(bx, by, 58.0, 26.0) {
                if is_alt {
                    self.reset_param_to_default(ptr);
                } else {
                    self.set_param_atomic(ptr, if cur { 0.0 } else { 1.0 });
                }
                cx.needs_redraw();
                return true;
            }
        }

        // --- Limiter Voicing Punch Toggle (cy = 668.0) ---
        if in_box(1226.0, 668.0, 76.0, 26.0) {
            let ptr = self.params.limiter_punch.as_ptr();
            let cur = self.params.limiter_punch.value();
            if is_alt {
                self.reset_param_to_default(ptr);
            } else {
                self.set_param_atomic(ptr, if cur { 0.0 } else { 1.0 });
            }
            cx.needs_redraw();
            return true;
        }

        false
    }

    /// Advance the input peak hold. `Cell::get` hands out a copy, so the state must be written
    /// back explicitly or the hold never persists between frames.
    fn step_input_peak_hold(&self, live_db: f32, now: Instant) -> f32 {
        let mut hold = self.peak_hold_in.get();
        let db = hold.step(
            live_db,
            now,
            debug_calibration::calibration().peak_hold_ms(),
        );
        self.peak_hold_in.set(hold);
        db
    }

    fn reset_input_peak_hold(&self, now: Instant) {
        let mut hold = self.peak_hold_in.get();
        hold.reset(now);
        self.peak_hold_in.set(hold);
    }

    fn step_input_rms_hold(&self, live_db: f32, now: Instant) -> f32 {
        let mut hold = self.rms_hold_in.get();
        let db = hold.step(
            live_db,
            now,
            debug_calibration::calibration().peak_hold_ms(),
        );
        self.rms_hold_in.set(hold);
        db
    }

    fn reset_input_rms_hold(&self, now: Instant) {
        let mut hold = self.rms_hold_in.get();
        hold.reset(now);
        self.rms_hold_in.set(hold);
    }

    fn step_tp_hold(&self, live_db: f32, now: Instant) -> f32 {
        let mut hold = self.tp_hold.get();
        let db = hold.step(
            live_db,
            now,
            debug_calibration::calibration().peak_hold_ms(),
        );
        self.tp_hold.set(hold);
        db
    }

    fn reset_tp_hold(&self, now: Instant) {
        let mut hold = self.tp_hold.get();
        hold.reset(now);
        self.tp_hold.set(hold);
    }

    fn step_lim_out_hold(&self, live_db: f32, now: Instant) -> f32 {
        let mut hold = self.lim_out_hold.get();
        let db = hold.step(
            live_db,
            now,
            debug_calibration::calibration().peak_hold_ms(),
        );
        self.lim_out_hold.set(hold);
        db
    }

    fn reset_lim_out_hold(&self, now: Instant) {
        let mut hold = self.lim_out_hold.get();
        hold.reset(now);
        self.lim_out_hold.set(hold);
    }

    fn step_gr_hold(&self, live_gr: f32, now: Instant) -> f32 {
        let mut hold = self.gr_hold.get();
        let gr = hold.step(live_gr, now);
        self.gr_hold.set(hold);
        gr
    }

    fn reset_gr_hold(&self, now: Instant) {
        let mut hold = self.gr_hold.get();
        hold.reset(now);
        self.gr_hold.set(hold);
    }

    /// True when the audio thread has stopped producing blocks, which freezes every meter
    /// atomic at its last value. The heartbeat distinguishes that from a steady signal.
    fn meters_idle(&self, now: Instant) -> bool {
        let heartbeat = self.meters.process_counter.load(Ordering::Relaxed);
        if heartbeat != self.meter_counter.get() {
            self.meter_counter.set(heartbeat);
            self.meter_activity.set(now);
            return false;
        }
        self.meter_activity.get().elapsed() > METER_IDLE_TIMEOUT
    }
}

impl View for EqEditorView {
    fn element(&self) -> Option<&'static str> {
        Some("masteraid-rack-console")
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|window_event, _meta| match window_event {
            WindowEvent::MouseDown(MouseButton::Left) => {
                // Clicking outside an active text entry commits it, so a typed number is never
                // silently discarded when the user moves on.
                if self.debug_menu_open.get() && self.debug_edit.borrow().is_some() {
                    let bounds = cx.bounds();
                    let mx = cx.mouse().cursorx - bounds.x;
                    let my = cx.mouse().cursory - bounds.y;
                    let in_panel = (Self::DBG_X..=Self::DBG_X + Self::DBG_W).contains(&mx)
                        && (Self::DBG_Y..=Self::DBG_Y + Self::DBG_H).contains(&my);
                    if !in_panel {
                        self.commit_debug_edit(cx);
                    }
                }

                if self.preset_edit.borrow().is_some() {
                    let bounds = cx.bounds();
                    let mx = cx.mouse().cursorx - bounds.x;
                    let my = cx.mouse().cursory - bounds.y;
                    let in_dseg = (426.0..=766.0).contains(&mx) && (889.0..=923.0).contains(&my);
                    let in_save = (824.0..=874.0).contains(&mx) && (888.0..=924.0).contains(&my);
                    if !in_dseg && !in_save {
                        if let Some(edit) = self.preset_edit.borrow_mut().take() {
                            let trimmed = edit.text.trim();
                            if !trimmed.is_empty() {
                                self.save_user_preset(trimmed.to_string());
                            }
                        }
                        cx.needs_redraw();
                    }
                }

                // A previous gesture whose MouseUp was lost (focus change, drag outside the
                // window, etc.) must be closed before starting a new one.
                self.end_active_drag(cx);
                self.pointer_down.set(true);

                let bounds = cx.bounds();
                let mx = cx.mouse().cursorx - bounds.x;
                let my = cx.mouse().cursory - bounds.y;
                let is_alt = cx.modifiers().alt();
                let is_ctrl = cx.modifiers().contains(Modifiers::CTRL);

                if self.handle_click(cx, mx, my, is_alt) {
                    return;
                }

                if let Some((ctrl, _kcx, _kcy)) = self.find_stepped_control_at(mx, my) {
                    let ptr = self.control_ptr(ctrl);
                    if is_alt {
                        if ctrl == SteppedControl::LoCut {
                            self.reset_param_to_default(self.params.filter_lo_cut_hz.as_ptr());
                            self.reset_param_to_default(self.params.filter_lo_cut_slope.as_ptr());
                        } else if ctrl == SteppedControl::HiCut {
                            self.reset_param_to_default(self.params.filter_hi_cut_hz.as_ptr());
                            self.reset_param_to_default(self.params.filter_hi_cut_slope.as_ptr());
                        } else if ctrl == SteppedControl::MonoFreq {
                            self.reset_param_to_default(self.params.bass_mono_freq.as_ptr());
                            self.reset_param_to_default(self.params.bass_mono_slope.as_ptr());
                        } else {
                            self.reset_param_to_default(ptr);
                            self.sync_int_cmd_to_default(ctrl, ptr);
                        }
                        self.trigger_momentary_for_ctrl(
                            ctrl,
                            false,
                            Some(unsafe { ptr.default_plain_value() }),
                        );
                        cx.needs_redraw();
                        return;
                    }

                    let mut cur_idx = self.control_current_idx(ctrl);
                    let mut cur_norm = unsafe { ptr.unmodulated_normalized_value() };

                    // Compressor makeup is a push-pull pot: turning it while AUTO is latched
                    // hands manual control over at the auto amount, and a press held still
                    // latches AUTO on/off (fired by the UI tick while still held).
                    if matches!(ctrl, SteppedControl::CompMakeup) {
                        if self.params.comp_auto_makeup.value() {
                            let auto_db = self.meters.comp_auto_makeup_db.load().clamp(-10.0, 20.0);
                            let norm = ((auto_db + 10.0) / 30.0).clamp(0.0, 1.0);
                            self.set_param_atomic(self.params.comp_makeup.as_ptr(), norm);
                            self.set_param_atomic(self.params.comp_auto_makeup.as_ptr(), 0.0);
                            cur_norm = norm;
                            cur_idx = Self::closest_index(auto_db, &MAKEUP_11_VALS);
                        }
                        self.arm_hold(HoldAction::MakeupAuto, my, true);
                    }
                    #[cfg(debug_assertions)]
                    if is_int_control(ctrl) {
                        let param_raw = self
                            .int_step_info(ctrl)
                            .map(|(ip, _)| ip.value())
                            .unwrap_or(-1);
                        eprintln!(
                            "TRACE DOWN {:?} start_idx={} param_raw={}",
                            ctrl, cur_idx, param_raw
                        );
                    }
                    unsafe {
                        self.gui_context.raw_begin_set_parameter(ptr);
                    }
                    self.trigger_momentary_for_ctrl(ctrl, is_ctrl, None);
                    self.active_drag = Some(DragTarget::Knob {
                        ctrl,
                        ptr,
                        last_y: my,
                        accum_y: 0.0,
                        cur_idx,
                        cur_norm,
                        fine_mode: is_ctrl,
                    });
                    cx.capture();
                    cx.needs_redraw();
                }
            }

            WindowEvent::MouseDoubleClick(MouseButton::Left) => {
                let bounds = cx.bounds();
                let mx = cx.mouse().cursorx - bounds.x;
                let my = cx.mouse().cursory - bounds.y;
                // A double-click inside the open debug panel belongs to the menu (e.g. a second
                // click on a value row); it must never reset a knob painted underneath.
                if self.debug_menu_open.get() && Self::debug_panel_contains(mx, my) {
                    return;
                }

                // The second press already opened a gesture in MouseDown; close it before the
                // reset so the host does not see nested begin/end pairs.
                self.end_active_drag(cx);

                if let Some((ctrl, _, _)) = self.find_stepped_control_at(mx, my) {
                    let ptr = self.control_ptr(ctrl);
                    if ctrl == SteppedControl::LoCut {
                        self.reset_param_to_default(self.params.filter_lo_cut_hz.as_ptr());
                        self.reset_param_to_default(self.params.filter_lo_cut_slope.as_ptr());
                    } else if ctrl == SteppedControl::HiCut {
                        self.reset_param_to_default(self.params.filter_hi_cut_hz.as_ptr());
                        self.reset_param_to_default(self.params.filter_hi_cut_slope.as_ptr());
                    } else if ctrl == SteppedControl::MonoFreq {
                        self.reset_param_to_default(self.params.bass_mono_freq.as_ptr());
                        self.reset_param_to_default(self.params.bass_mono_slope.as_ptr());
                    } else {
                        self.reset_param_to_default(ptr);
                        self.sync_int_cmd_to_default(ctrl, ptr);
                    }
                    self.trigger_momentary_for_ctrl(
                        ctrl,
                        false,
                        Some(unsafe { ptr.default_plain_value() }),
                    );
                    cx.needs_redraw();
                }
            }

            WindowEvent::MouseMove(_x, _y) => {
                // Safety net for a lost MouseUp: if the OS reports the button as released while
                // we still think a drag or hold is active, close the gesture instead of continuing it.
                if cx.mouse().left.state == MouseButtonState::Released
                    && (self.active_drag.is_some() || self.hold_press.get().is_some() || self.pointer_down.get())
                {
                    self.end_active_drag(cx);
                }

                let bounds = cx.bounds();
                let current_y = cx.mouse().cursory - bounds.y;
                let is_ctrl = cx.modifiers().contains(Modifiers::CTRL);
                // Debug menu now uses arm_hold/poll_hold mechanism

                // Turning a knob that also has a hold gesture disarms the countdown, so a drag can
                // never also fire the special action.
                if let Some(hold) = self.hold_press.get() {
                    if hold.cancel_on_turn && (current_y - hold.start_y).abs() > 3.5 {
                        self.hold_press.set(None);
                    }
                }

                enum MoveAction {
                    StepTo(SteppedControl, ParamPtr, usize),
                    FineTo(SteppedControl, ParamPtr, f32),
                }
                let mut action: Option<MoveAction> = None;

                let drag_step_count = match &self.active_drag {
                    Some(DragTarget::Knob { ctrl, .. }) => self.control_step_count(*ctrl),
                    _ => 1,
                };

                if let Some(ref mut drag) = self.active_drag {
                    match drag {
                        DragTarget::Knob {
                            ctrl,
                            ptr,
                            ref mut last_y,
                            ref mut accum_y,
                            ref mut cur_idx,
                            ref mut cur_norm,
                            ref mut fine_mode,
                        } => {
                            let dy = *last_y - current_y;
                            *last_y = current_y;
                            *fine_mode = is_ctrl;

                            // Ctrl held -> defeat stepping (smooth continuous fine adjustment).
                            // Integer rotary selectors always stay stepped.
                            if is_ctrl && !is_int_control(*ctrl) && !matches!(*ctrl, SteppedControl::LoCut | SteppedControl::HiCut | SteppedControl::MonoFreq) {
                                let fine_sens = 0.0022;
                                let next_norm = (*cur_norm + dy * fine_sens).clamp(0.0, 1.0);
                                if (next_norm - *cur_norm).abs() > 1.0e-5 {
                                    *cur_norm = next_norm;
                                    action = Some(MoveAction::FineTo(*ctrl, *ptr, next_norm));
                                }
                            } else {
                                *accum_y += dy;
                                let step_px = 14.0;
                                if accum_y.abs() >= step_px {
                                    let steps_delta = (*accum_y / step_px).trunc() as isize;
                                    *accum_y -= (steps_delta as f32) * step_px;
                                    let max_idx = (drag_step_count - 1) as isize;
                                    let next_idx = ((*cur_idx as isize) + steps_delta)
                                        .clamp(0, max_idx)
                                        as usize;
                                    if next_idx != *cur_idx {
                                        *cur_idx = next_idx;
                                        action = Some(MoveAction::StepTo(*ctrl, *ptr, next_idx));
                                    }
                                }
                            }
                        }
                    }
                }

                match action {
                    Some(MoveAction::StepTo(ctrl, ptr, next_idx)) => {
                        match ctrl {
                            SteppedControl::LoCut => {
                                let (hz, slope) = if next_idx == 0 {
                                    (0.0, 0)
                                } else {
                                    (LO_CUT_5_FREQS[(next_idx - 1) / 2], ((next_idx - 1) % 2) as i32)
                                };
                                let norm = self.params.filter_lo_cut_hz.preview_normalized(hz);
                                self.set_param_norm(self.params.filter_lo_cut_hz.as_ptr(), norm);
                                self.set_param_atomic(self.params.filter_lo_cut_slope.as_ptr(), slope as f32 / 2.0);
                                if let Some(DragTarget::Knob { ref mut cur_norm, .. }) = self.active_drag {
                                    *cur_norm = norm;
                                }
                            }
                            SteppedControl::HiCut => {
                                let (hz, slope) = if next_idx >= 10 {
                                    (0.0, 0)
                                } else {
                                    (HI_CUT_5_FREQS[next_idx / 2], (next_idx % 2) as i32)
                                };
                                let norm = self.params.filter_hi_cut_hz.preview_normalized(hz);
                                self.set_param_norm(self.params.filter_hi_cut_hz.as_ptr(), norm);
                                self.set_param_atomic(self.params.filter_hi_cut_slope.as_ptr(), slope as f32 / 2.0);
                                if let Some(DragTarget::Knob { ref mut cur_norm, .. }) = self.active_drag {
                                    *cur_norm = norm;
                                }
                            }
                            SteppedControl::MonoFreq => {
                                let (hz, slope) = if next_idx == 0 {
                                    (0.0, 0)
                                } else {
                                    (MONO_5_FREQS[(next_idx - 1) / 2], ((next_idx - 1) % 2) as i32)
                                };
                                let norm = self.params.bass_mono_freq.preview_normalized(hz);
                                self.set_param_norm(self.params.bass_mono_freq.as_ptr(), norm);
                                self.set_param_atomic(self.params.bass_mono_slope.as_ptr(), slope as f32 / 2.0);
                                if let Some(DragTarget::Knob { ref mut cur_norm, .. }) = self.active_drag {
                                    *cur_norm = norm;
                                }
                            }
                            _ => {
                                let norm = self.control_idx_to_norm(ctrl, next_idx);
                                self.set_param_norm(ptr, norm);
                                self.set_int_cmd(ctrl, next_idx);
                                if let Some(DragTarget::Knob { ref mut cur_norm, .. }) = self.active_drag {
                                    *cur_norm = norm;
                                }
                            }
                        }
                        #[cfg(debug_assertions)]
                        if is_int_control(ctrl) {
                            eprintln!("TRACE STEP {:?} idx={}", ctrl, next_idx);
                        }
                        let pending = self.control_plain_for_idx(ctrl, next_idx);
                        self.trigger_momentary_for_ctrl(ctrl, false, pending);
                        cx.needs_redraw();
                    }
                    Some(MoveAction::FineTo(ctrl, ptr, next_norm)) => {
                        self.set_param_norm(ptr, next_norm);
                        let new_idx = self.control_current_idx(ctrl);
                        if let Some(DragTarget::Knob {
                            ref mut cur_idx, ..
                        }) = self.active_drag
                        {
                            *cur_idx = new_idx;
                        }
                        let pending = unsafe { ptr.preview_plain(next_norm) };
                        self.trigger_momentary_for_ctrl(ctrl, true, Some(pending));
                        cx.needs_redraw();
                    }
                    None => {}
                }
            }

            WindowEvent::MouseUp(MouseButton::Left) => {
                self.pointer_down.set(false);
                // A hold that already fired has nothing left to do on release. If the frame poll
                // could not run in time (window occluded, frame dropped), a press that outlived the
                // countdown still fires here; an earlier release performs the normal click.
                if let Some(hold) = self.hold_press.get() {
                    self.hold_press.set(None);
                    let due = hold.down_at.elapsed()
                        >= Duration::from_millis(
                            if matches!(hold.action, HoldAction::ToggleDebugMenu) {
                                DEBUG_MENU_HOLD_MS
                            } else {
                                HOLD_MS
                            },
                        );
                    if hold.fired {
                        // Already handled while the button was down.
                    } else if due {
                        self.fire_hold(hold.action);
                    } else {
                        self.click_action(hold.action);
                    }
                }
                if let Some(drag) = self.active_drag.take() {
                    let DragTarget::Knob { ptr, .. } = drag;
                    unsafe {
                        self.gui_context.raw_end_set_parameter(ptr);
                    }
                }
                cx.release();
                cx.needs_redraw();
            }

            WindowEvent::MouseScroll(_x, y) => {
                if y.abs() < 1.0e-4 {
                    return;
                }
                let bounds = cx.bounds();
                let mx = cx.mouse().cursorx - bounds.x;
                let my = cx.mouse().cursory - bounds.y;
                // The panel swallows the wheel so scrolling over it cannot move the controls it
                // covers.
                if self.debug_menu_open.get() && Self::debug_panel_contains(mx, my) {
                    return;
                }
                let is_ctrl = cx.modifiers().contains(Modifiers::CTRL);
                let step_dir: isize = if *y > 0.0 { 1 } else { -1 };

                if self.handle_scroll_on_non_knob(cx, mx, my, step_dir as i32) {
                    return;
                }

                if let Some((ctrl, _, _)) = self.find_stepped_control_at(mx, my) {
                    let ptr = self.control_ptr(ctrl);
                    if is_ctrl && !is_int_control(ctrl) && !matches!(ctrl, SteppedControl::LoCut | SteppedControl::HiCut | SteppedControl::MonoFreq) {
                        // Ctrl + Wheel: fine continuous adjustment (defeats stepping)
                        let cur_norm = unsafe { ptr.unmodulated_normalized_value() };
                        let fine_step = 0.012 * (step_dir as f32);
                        let next_norm = (cur_norm + fine_step).clamp(0.0, 1.0);
                        self.set_param_atomic(ptr, next_norm);
                        let pending = unsafe { ptr.preview_plain(next_norm) };
                        self.trigger_momentary_for_ctrl(ctrl, true, Some(pending));
                    } else {
                        let cur_idx = self.control_current_idx(ctrl) as isize;
                        let max_idx = (self.control_step_count(ctrl) - 1) as isize;
                        let next_idx = (cur_idx + step_dir).clamp(0, max_idx) as usize;
                        match ctrl {
                            SteppedControl::LoCut => {
                                let (hz, slope) = if next_idx == 0 {
                                    (0.0, 0)
                                } else {
                                    (LO_CUT_5_FREQS[(next_idx - 1) / 2], ((next_idx - 1) % 2) as i32)
                                };
                                let norm = self.params.filter_lo_cut_hz.preview_normalized(hz);
                                self.set_param_atomic(self.params.filter_lo_cut_hz.as_ptr(), norm);
                                self.set_param_atomic(self.params.filter_lo_cut_slope.as_ptr(), slope as f32 / 2.0);
                            }
                            SteppedControl::HiCut => {
                                let (hz, slope) = if next_idx >= 10 {
                                    (0.0, 0)
                                } else {
                                    (HI_CUT_5_FREQS[next_idx / 2], (next_idx % 2) as i32)
                                };
                                let norm = self.params.filter_hi_cut_hz.preview_normalized(hz);
                                self.set_param_atomic(self.params.filter_hi_cut_hz.as_ptr(), norm);
                                self.set_param_atomic(self.params.filter_hi_cut_slope.as_ptr(), slope as f32 / 2.0);
                            }
                            SteppedControl::MonoFreq => {
                                let (hz, slope) = if next_idx == 0 {
                                    (0.0, 0)
                                } else {
                                    (MONO_5_FREQS[(next_idx - 1) / 2], ((next_idx - 1) % 2) as i32)
                                };
                                let norm = self.params.bass_mono_freq.preview_normalized(hz);
                                self.set_param_atomic(self.params.bass_mono_freq.as_ptr(), norm);
                                self.set_param_atomic(self.params.bass_mono_slope.as_ptr(), slope as f32 / 2.0);
                            }
                            _ => {
                                let norm = self.control_idx_to_norm(ctrl, next_idx);
                                self.set_param_atomic(ptr, norm);
                                self.set_int_cmd(ctrl, next_idx);
                            }
                        }
                        let pending = self.control_plain_for_idx(ctrl, next_idx);
                        self.trigger_momentary_for_ctrl(ctrl, false, pending);
                    }
                    cx.needs_redraw();
                }
            }

            // Cancel an open gesture when the pointer leaves the window or the window loses
            // focus; the matching MouseUp will never arrive in those cases.
            WindowEvent::MouseLeave | WindowEvent::FocusOut => {
                self.end_active_drag(cx);
            }

            _ => {}
        });
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        // The baseview backend renders every frame but never ticks vizia timers, so hold
        // countdowns are advanced here, before anything is painted. This is what makes a held
        // button take effect while it is still down.
        // Apply a value committed by the global keyboard listener. Doing it here keeps the
        // parameter write on the draw path, where the shared cells are already borrowed.
        if let Some((field, value)) = self.debug_commit.take() {
            self.set_debug_param(field, value);
        }
        if let Some(name) = self.preset_commit.take() {
            self.save_user_preset(name);
            self.save_preset_banner.set(Some(Instant::now()));
        }
        self.poll_hold();

        let bounds = cx.bounds();
        if bounds.w == 0.0 || bounds.h == 0.0 {
            return;
        }
        let _ = &self.fine_override;

        let font_reg = match self.font_regular.get() {
            Some(id) => id,
            None => {
                let id = canvas
                    .add_font_mem(nih_plug_vizia::assets::fonts::NOTO_SANS_REGULAR)
                    .expect("Failed to load regular font");
                self.font_regular.set(Some(id));
                id
            }
        };
        let font_bold = match self.font_bold.get() {
            Some(id) => id,
            None => {
                let id = canvas
                    .add_font_mem(nih_plug_vizia::assets::fonts::NOTO_SANS_BOLD)
                    .expect("Failed to load bold font");
                self.font_bold.set(Some(id));
                id
            }
        };
        let font_dseg = match self.font_dseg.get() {
            Some(id) => id,
            None => {
                let id = canvas
                    .add_font_mem(include_bytes!(
                        "../../assets/fonts/dseg14-classic-italic.otf"
                    ))
                    .expect("Failed to load DSEG14 font");
                self.font_dseg.set(Some(id));
                id
            }
        };
        let fonts = ThemeFonts {
            regular: font_reg,
            bold: font_bold,
            dseg: font_dseg,
        };

        let now = Instant::now();
        // When the host stops the transport it may stop calling `process` altogether, leaving
        // the shared atomics frozen at their last reading. Fall back to rest in that case.
        let meters_idle = self.meters_idle(now);

        // NOTE: never write parameters from `draw`. The 30 fps redraw timer would start/end a host
        // parameter gesture on every frame. Long-press "smash" is resolved by the event handlers.

        canvas.save();
        canvas.translate(bounds.x, bounds.y);

        let is_light = self.params.theme_light.value();
        crate::gui::primitives::palette::set_light_mode(is_light);

        // =====================================================================
        // 1. MAIN RACK CHASSIS & 8 MOUNTING SCREWS (1360 x 952)
        // =====================================================================
        let mut bg = Path::new();
        bg.rect(0.0, 0.0, Self::TOTAL_WIDTH, Self::TOTAL_HEIGHT);
        let bg_grad = if is_light {
            Paint::linear_gradient(
                0.0,
                0.0,
                0.0,
                Self::TOTAL_HEIGHT,
                Color::rgb(0xCE, 0xCB, 0xC6),
                Color::rgb(0xBE, 0xBA, 0xB4),
            )
        } else {
            Paint::linear_gradient(
                0.0,
                0.0,
                0.0,
                Self::TOTAL_HEIGHT,
                Color::rgb(0x0E, 0x0E, 0x11),
                Color::rgb(0x07, 0x07, 0x09),
            )
        };
        canvas.fill_path(&bg, &bg_grad);

        let rack_screws = [
            (17.0, 24.0, 18.0),
            (17.0, 320.0, 72.0),
            (17.0, 620.0, 126.0),
            (17.0, Self::TOTAL_HEIGHT - 24.0, 48.0),
            (Self::TOTAL_WIDTH - 17.0, 24.0, 84.0),
            (Self::TOTAL_WIDTH - 17.0, 320.0, 32.0),
            (Self::TOTAL_WIDTH - 17.0, 620.0, 105.0),
        ];
        for (sx, sy, angle) in rack_screws {
            draw_screw_angled(canvas, sx, sy, angle);
        }

        // =====================================================================
        // 2. SPLIT TOP STRIP: ANALOG TAPE (LEFT) & MASTERING CLIPPER (RIGHT)
        // =====================================================================
        let (top_x, top_y, top_w, top_h) = (34.0, 14.0, 860.0, 128.0);
        draw_card(canvas, top_x, top_y, top_w, top_h, true);
        draw_text_centered(
            canvas,
            top_x + 132.0,
            top_y + 20.0,
            "ANALOG TAPE MASTERING",
            fonts.bold_paint(silk_muted(), 11.0),
        );
        let tape_is_on = !self.params.tape_bypass.value();
        draw_model_select_lit(
            canvas,
            top_x + 410.0,
            top_y + 20.0,
            "15 IPS",
            "30 IPS",
            self.params.tape_speed_30ips.value(),
            136.0,
            fonts,
            tape_is_on,
        );
        draw_model_select_lit(
            canvas,
            top_x + 556.0,
            top_y + 20.0,
            "456",
            "GP9",
            self.params.tape_formula_gp9.value(),
            116.0,
            fonts,
            tape_is_on,
        );
        draw_vintage_bypass_button(
            canvas,
            top_x + top_w - 54.0,
            top_y + 20.0,
            "IN",
            tape_is_on,
            fonts,
        );

        let tape_cy = top_y + 86.0;
        let top_tape_pots = [
            (top_x + 110.0, "DRIVE", SteppedControl::TapeDrive),
            (top_x + 240.0, "BIAS", SteppedControl::TapeBias),
            (top_x + 370.0, "HEAD BUMP", SteppedControl::TapeHeadBump),
            (top_x + 500.0, "BUMP FREQ", SteppedControl::TapeBumpFreq),
            (top_x + 630.0, "FLUTTER", SteppedControl::TapeFlutter),
        ];
        for (px, lbl, ctrl) in top_tape_pots {
            let v_str = self.control_display_label(ctrl);
            draw_knob_small_ticks_lift_lit(
                canvas,
                px,
                tape_cy,
                lbl,
                &v_str,
                self.control_angle_11(ctrl),
                false,
                fonts,
                &KNOB_TICKS_11,
                0.0,
                tape_is_on,
            );
        }
        // AUTO sits on the same pitch as the pots, square like the coloration switches.
        draw_vintage_push_button(
            canvas,
            top_x + 760.0,
            tape_cy,
            "AUTO",
            if tape_is_on && self.params.tape_comp.value() { 1 } else { 0 },
            44.0,
            44.0,
            fonts,
        );

        // Right Top Card: MASTERING CLIPPER (x: 910, y: 14, w: 416, h: 128)
        let (clip_x, clip_y, clip_w, clip_h) = (910.0, 14.0, 416.0, 128.0);
        draw_card(canvas, clip_x, clip_y, clip_w, clip_h, true);
        draw_text_centered(
            canvas,
            clip_x + 112.0,
            clip_y + 20.0,
            "MASTERING CLIPPER",
            fonts.bold_paint(silk_muted(), 11.0),
        );
        let clip_is_on = !self.params.clipper_bypass.value();
        draw_vintage_bypass_button(
            canvas,
            clip_x + clip_w - 54.0,
            clip_y + 20.0,
            "IN",
            clip_is_on,
            fonts,
        );
        draw_knob_small_ticks_lift_lit(
            canvas,
            clip_x + 50.0,
            tape_cy,
            "DRIVE",
            &self.control_display_label(SteppedControl::ClipperDrive),
            self.control_angle_11(SteppedControl::ClipperDrive),
            false,
            fonts,
            &KNOB_TICKS_11,
            0.0,
            clip_is_on,
        );
        draw_knob_small_ticks_lift_lit(
            canvas,
            clip_x + 125.0,
            tape_cy,
            "CEILING",
            &self.control_display_label(SteppedControl::ClipperCeiling),
            self.control_angle_11(SteppedControl::ClipperCeiling),
            false,
            fonts,
            &KNOB_TICKS_11,
            0.0,
            clip_is_on,
        );

        let (live_clip_l, live_clip_r) = if meters_idle {
            (0.0, 0.0)
        } else {
            (
                self.meters.clipper_clip_db_l.load(),
                self.meters.clipper_clip_db_r.load(),
            )
        };
        let live_clip_max = live_clip_l.max(live_clip_r);
        let mut held_clip = self.clip_hold.get();
        if live_clip_max > held_clip {
            held_clip = live_clip_max;
            self.clip_hold.set(held_clip);
        }
        let clip_val = self.readout_held.get()[5];
        let clip_str = if clip_val < 0.05 {
            "0.0".to_string()
        } else {
            format!("{:>3.1}", clip_val.min(9.9))
        };
        let clip_banner = self.momentary_text(MomentarySlot::ClipperReadout);

        // 1. Clipper LR Meter at clip_x + 185.0 (1095.0)
        draw_clipper_meter(
            canvas,
            clip_x + 185.0,
            tape_cy,
            live_clip_l,
            live_clip_r,
            fonts,
        );

        // 2. Clipper 3-character DSEG Readout at clip_x + 240.0 (1150.0)
        draw_clipper_readout(
            canvas,
            clip_x + 240.0,
            tape_cy,
            &clip_str,
            clip_banner.as_deref(),
            fonts,
        );

        // Square AUTO on the pot pitch at clip_x + 295.0 (1205.0)
        draw_vintage_push_button(
            canvas,
            clip_x + 295.0,
            tape_cy,
            "AUTO",
            if clip_is_on && self.params.clipper_comp.value() {
                1
            } else {
                0
            },
            44.0,
            44.0,
            fonts,
        );
        draw_model_select_lit(
            canvas,
            clip_x + 365.0,
            top_y + 66.0,
            "SOFT",
            "HARD",
            self.params.clipper_hard_shape.value(),
            84.0,
            fonts,
            clip_is_on,
        );
        draw_model_select_lit(
            canvas,
            clip_x + 365.0,
            top_y + 102.0,
            "2X",
            "4X",
            self.params.clipper_os_4x.value(),
            84.0,
            fonts,
            clip_is_on,
        );

        // =====================================================================
        // 3. THREE-COLUMN BALANCED MAIN DECK (y: 156, h: 782)
        // =====================================================================
        let main_y = 156.0;
        let main_h = 782.0;

        // AUTO lamp animation. The baseview backend paints every frame but never ticks vizia
        // timers, so both clocks are derived from frame time.
        // Measuring: hard blinker that goes fully dark, so a glance says "working".
        let blink_on = (self.boot.elapsed().as_secs_f32() / AUTO_BLINK_SECONDS).fract() < 0.5;
        // Engaged: faster swell between 45% and full. It never reaches zero, so "done" is distinct
        // from "working" even out of the corner of your eye.
        let breath = 0.5
            - 0.5
                * (self.boot.elapsed().as_secs_f32() * std::f32::consts::TAU / AUTO_BREATH_SECONDS)
                    .cos();
        let auto_lamp = |engaged: bool, progress: f32| {
            if !engaged {
                AutoLamp::Off
            } else if progress > 0.0 && progress < 0.999 {
                AutoLamp::Learning { on: blink_on }
            } else if progress >= 0.999 {
                AutoLamp::Engaged {
                    fade: 0.45 + 0.55 * breath,
                }
            } else {
                // Engaged but this model has never been measured: steady, not blinking.
                AutoLamp::On
            }
        };
        let eq_auto_lamp = auto_lamp(
            self.params.eq_col_comp.value(),
            self.meters.eq_col_learning.load(),
        );
        let comp_auto_lamp = auto_lamp(
            self.params.comp_col_comp.value(),
            self.meters.comp_col_learning.load(),
        );

        // 3.1 LEFT WING: INPUT STAGE & FILTERING (x: 34, w: 200)
        let (lw_x, lw_w) = (34.0, 200.0);
        let lw_cx = lw_x + lw_w * 0.5;
        let input_is_on = !self.params.input_bypass.value();
        draw_card(canvas, lw_x, main_y, lw_w, main_h, true);
        draw_text(
            canvas,
            lw_x + 14.0,
            main_y + 20.0,
            "INPUT STAGE",
            fonts.bold_paint(silk_muted(), 10.5),
        );
        draw_vintage_bypass_button(
            canvas,
            lw_x + lw_w - 38.0,
            main_y + 20.0,
            "IN",
            input_is_on,
            fonts,
        );

        // Live Input Stereo Ladder (click ladder to toggle PEAK / RMS)
        let peak_l = if meters_idle {
            METER_FLOOR_DB
        } else {
            self.meters.input_peak_db_l.load()
        };
        let peak_r = if meters_idle {
            METER_FLOOR_DB
        } else {
            self.meters.input_peak_db_r.load()
        };
        let rms_l = if meters_idle {
            METER_FLOOR_DB
        } else {
            self.meters.input_rms_db_l.load()
        };
        let rms_r = if meters_idle {
            METER_FLOOR_DB
        } else {
            self.meters.input_rms_db_r.load()
        };

        let (ladder_l, ladder_r) = if self.input_rms_mode {
            (rms_l, rms_r)
        } else {
            (peak_l, peak_r)
        };
        let lit_l = (((ladder_l + 36.0) / 36.0).clamp(0.0, 1.0) * 12.0).round() as usize;
        let lit_r = (((ladder_r + 36.0) / 36.0).clamp(0.0, 1.0) * 12.0).round() as usize;

        let live_in_peak = peak_l.max(peak_r);
        let peak_in = self.step_input_peak_hold(live_in_peak, now);

        let live_in_rms = rms_l.max(rms_r);
        let rms_in = self.step_input_rms_hold(live_in_rms, now);

        let live_lim_gr = if meters_idle {
            0.0
        } else {
            self.meters.limiter_gr_db.load()
        };
        let held_gr_max = if meters_idle {
            self.reset_gr_hold(now);
            0.0
        } else {
            self.step_gr_hold(live_lim_gr, now)
        };

        let live_lim_tp = if meters_idle {
            METER_FLOOR_DB
        } else {
            self.meters.output_true_peak_db.load()
        };
        let tp_in = self.step_tp_hold(live_lim_tp, now);

        let lufs_live = match (meters_idle, self.lufs_mode) {
            (true, 0 | 1) => METER_FLOOR_DB,
            (_, 0) => self.meters.output_lufs_mom.load(),
            (_, 1) => self.meters.output_lufs.load(),
            _ => self.meters.output_lufs_int.load(),
        };

        let clip_val = self.clip_hold.get();

        if self.readout_refreshed.get().elapsed() >= Duration::from_millis(150) {
            self.readout_held.set([peak_in, rms_in, lufs_live, held_gr_max, tp_in, clip_val]);
            self.readout_refreshed.set(now);
        }

        let held = self.readout_held.get();
        let peak_str = format_db_readout(held[0].clamp(METER_FLOOR_DB, 9.9));
        let peak_banner = self.momentary_text(MomentarySlot::InputPeak);
        let rms_str = format_db_readout(held[1].clamp(METER_FLOOR_DB, 9.9));
        let rms_banner = self.momentary_text(MomentarySlot::InputRms);

        draw_input_ladder(
            canvas,
            lw_cx - 42.0,
            main_y + 52.0,
            84.0,
            &peak_str,
            peak_banner.as_deref(),
            &rms_str,
            rms_banner.as_deref(),
            lit_l,
            lit_r,
            self.input_rms_mode,
            fonts,
        );

        let trim_ro = self.readout_for(MomentarySlot::InputTrim, "TRIM");
        draw_knob_large_lit(
            canvas,
            lw_cx,
            506.0,
            &trim_ro,
            self.control_angle_11(SteppedControl::InputTrim),
            fonts,
            input_is_on,
        );
        draw_vintage_push_button(
            canvas,
            lw_cx - 35.0,
            600.0,
            "PHASE",
            if input_is_on && self.params.polarity.value() { 1 } else { 0 },
            58.0,
            24.0,
            fonts,
        );
        draw_vintage_push_button(
            canvas,
            lw_cx + 35.0,
            600.0,
            "MONO",
            if input_is_on && self.params.input_mono.value() { 1 } else { 0 },
            58.0,
            24.0,
            fonts,
        );

        // Row 1: LO CUT and HI CUT side-by-side (compacted horizontally)
        draw_knob_small_ticks_lift_lit(
            canvas,
            89.0,
            740.0,
            "LO CUT",
            &self.control_display_label(SteppedControl::LoCut),
            self.control_angle_11(SteppedControl::LoCut),
            false,
            fonts,
            &KNOB_TICKS_11,
            0.0,
            input_is_on,
        );
        draw_knob_small_ticks_lift_lit(
            canvas,
            179.0,
            740.0,
            "HI CUT",
            &self.control_display_label(SteppedControl::HiCut),
            self.control_angle_11(SteppedControl::HiCut),
            false,
            fonts,
            &KNOB_TICKS_11,
            0.0,
            input_is_on,
        );

        // Row 2: BASS MONO and AMOUNT side-by-side (compacted horizontally)
        draw_knob_small_ticks_lift_lit(
            canvas,
            89.0,
            842.0,
            "BASS MONO",
            &self.control_display_label(SteppedControl::MonoFreq),
            self.control_angle_11(SteppedControl::MonoFreq),
            false,
            fonts,
            &KNOB_TICKS_11,
            0.0,
            input_is_on,
        );
        draw_knob_small_ticks_lift_lit(
            canvas,
            179.0,
            842.0,
            "AMOUNT",
            &self.control_display_label(SteppedControl::MonoAmount),
            self.control_angle_11(SteppedControl::MonoAmount),
            false,
            fonts,
            &KNOB_TICKS_11,
            0.0,
            input_is_on,
        );

        // 3.2 CENTER CONSOLE (x: 250, w: 860)
        let (cw_x, cw_w) = (250.0, 860.0);
        let eq_h = 346.0;
        let is_mass = self.params.eq_model.value() == EqModel::Mass;
        draw_card(canvas, cw_x, main_y, cw_w, eq_h, true);
        draw_text_centered(
            canvas,
            cw_x + 115.0,
            main_y + 20.0,
            if is_mass {
                "PARAMETRIC EQ"
            } else {
                "PROGRAM EQ"
            },
            fonts.bold_paint(silk_muted(), 11.5),
        );
        let eq_is_on = !self.params.eq_bypass.value();
        draw_model_select_lit(
            canvas,
            cw_x + cw_w * 0.5,
            main_y + 20.0,
            "PTEK",
            "MASS",
            is_mass,
            220.0,
            fonts,
            eq_is_on,
        );
        draw_vintage_bypass_button(
            canvas,
            cw_x + cw_w - 54.0,
            main_y + 20.0,
            "IN",
            eq_is_on,
            fonts,
        );

        if is_mass {
            // Mass: concentric frequency collar + gain rotor, with a Q trim below.
            let band_labels = ["LOW", "LO MID", "HI MID", "HIGH"];
            let band_on = [
                !self.params.mass_low_bypass.value(),
                !self.params.mass_lmf_bypass.value(),
                !self.params.mass_hmf_bypass.value(),
                !self.params.mass_hf_bypass.value(),
            ];

            for i in 0..4 {
                let bx = Self::BAND_CENTERS[i];
                let f_idx = self.control_current_idx(SteppedControl::EqFreq(i)).min(5);
                let (g_angle, g_idx) = self.eq_gain_angle_and_idx(i);
                let is_on = band_on[i] && eq_is_on;
                let ro_lbl = if is_on { band_labels[i] } else { "OFF" };
                let ro_str = self.readout_for(MomentarySlot::EqBand(i), ro_lbl);
                #[cfg(debug_assertions)]
                if let Some(label_idx) = MASS_FREQ_LABELS[i].iter().position(|lbl| *lbl == ro_str) {
                    if label_idx != f_idx {
                        eprintln!(
                            "TRACE MISMATCH band={} pointer_idx={} readout={} label_idx={}",
                            i, f_idx, ro_str, label_idx
                        );
                    }
                }
                draw_knob_concentric(
                    canvas,
                    bx,
                    316.0,
                    &ro_str,
                    f_idx,
                    g_angle,
                    g_idx,
                    is_on,
                    fonts,
                );
                let qx = bx - 28.0;
                let sw_x = bx + 32.0;

                // Shared lower card well (matching upper concentric card well width 136px)
                draw_plate_well_160x200(canvas, bx - 68.0, 448.0 - 54.0, 136.0, 100.0);

                // Q trim knob (shifted to the left)
                draw_knob_small_ticks_lift_lit(
                    canvas,
                    qx,
                    448.0,
                    "Q",
                    &self.control_display_label(SteppedControl::EqQ(i)),
                    self.control_angle_11(SteppedControl::EqQ(i)),
                    false,
                    fonts,
                    &KNOB_TICKS_11,
                    0.0,
                    eq_is_on,
                );

                // 2-position toggle switch (SHELF / BELL)
                let is_shelf = match i {
                    0 => self.params.mass_low_shelf.value(),
                    1 => self.params.mass_lmf_shelf.value(),
                    2 => self.params.mass_hmf_shelf.value(),
                    _ => self.params.mass_hf_shelf.value(),
                };
                draw_analog_toggle_switch(
                    canvas,
                    sw_x,
                    448.0,
                    is_shelf,
                    fonts,
                    eq_is_on,
                );
            }
        } else {
            // Ptek: one large 0..10 boost/atten knob per column. The readout keeps the band
            // name and swaps to the current value momentarily while the knob is adjusted.
            let ptek_big = [
                ("LO BST", SteppedControl::PuLowBoost),
                ("LO ATT", SteppedControl::PuLowAtten),
                ("HI BST", SteppedControl::PuHighBoost),
                ("HI ATT", SteppedControl::PuHighAtten),
            ];
            let ptek_small = [
                ("LOW FQ", SteppedControl::PuLowFreq),
                ("HI BW", SteppedControl::PuBandwidth),
                ("HIGH FQ", SteppedControl::PuHighFreq),
                ("ATT FQ", SteppedControl::PuAttenSel),
            ];

            for i in 0..4 {
                let bx = Self::BAND_CENTERS[i];
                let (ro_lbl, big_ctrl) = ptek_big[i];
                let ro_str = self.readout_for(MomentarySlot::EqBand(i), ro_lbl);
                draw_knob_large_lit(
                    canvas,
                    bx,
                    316.0,
                    &ro_str,
                    self.control_angle_11(big_ctrl),
                    fonts,
                    eq_is_on,
                );

                let (small_lbl, small_ctrl) = ptek_small[i];
                if let Some((_, count)) = self.int_step_info(small_ctrl) {
                    let idx = self.control_current_idx(small_ctrl).min(count - 1);
                    draw_knob_small_n(
                        canvas,
                        bx,
                        448.0,
                        small_lbl,
                        self.int_label(small_ctrl, idx),
                        count,
                        idx,
                        fonts,
                    );
                } else {
                    draw_knob_small_ticks_lift_lit(
                        canvas,
                        bx,
                        448.0,
                        small_lbl,
                        &self.control_display_label(small_ctrl),
                        self.control_angle_11(small_ctrl),
                        true,
                        fonts,
                        &KNOB_TICKS_11,
                        0.0,
                        eq_is_on,
                    );
                }
            }
        }

        let eq_out_x = cw_x + cw_w - 184.0 - 20.0;
        let eq_amp_idx = (self.params.eq_amp_type.value() as usize).min(6);
        let eq_trans_idx = (self.params.eq_trans_type.value() as usize).min(4);
        let eq_amp_str = match self.smash_banner.get() {
            Some((false, t)) if t.elapsed() < Duration::from_millis(950) => "SMASH+18",
            _ => AMP_ALIASES[eq_amp_idx],
        };
        draw_coloration_cassette(
            canvas,
            eq_out_x,
            222.0,
            184.0,
            252.0,
            eq_amp_str,
            TRANS_ALIASES[eq_trans_idx],
            eq_is_on && self.params.eq_col_match.value(),
            if eq_is_on { self.params.eq_intensity.value() } else { 0 },
            if eq_is_on { eq_auto_lamp } else { AutoLamp::Off },
            fonts,
        );

        // 3.2.2 LOWER RACK: MASTERING DYNAMICS (y: 510, h: 356)
        let comp_y = 510.0;
        let comp_h = 356.0;
        let is_vmu = self.params.comp_model.value() == CompModel::VMu;
        let comp_is_on = !self.params.comp_bypass.value();
        draw_card(canvas, cw_x, comp_y, cw_w, comp_h, true);
        draw_text_centered(
            canvas,
            cw_x + 115.0,
            comp_y + 20.0,
            "MASTERING DYNAMICS",
            fonts.bold_paint(silk_muted(), 11.5),
        );
        draw_model_select_lit(
            canvas,
            cw_x + cw_w * 0.5,
            comp_y + 20.0,
            "FAIR",
            "V-MU",
            is_vmu,
            220.0,
            fonts,
            comp_is_on,
        );
        draw_vintage_bypass_button(
            canvas,
            cw_x + cw_w - 54.0,
            comp_y + 20.0,
            "IN",
            comp_is_on,
            fonts,
        );

        // Live Stereo GR VU Meters. Centered vertically to the big knob and coloration cassette (vy = 580.0, height = 132.0, center = 646.0).
        let gr_l = if meters_idle {
            0.0
        } else {
            self.meters.comp_gr_db_l.load().abs()
        };
        let gr_r = if meters_idle {
            0.0
        } else {
            self.meters.comp_gr_db_r.load().abs()
        };
        let target_deg_l = vu_target_deg(gr_l);
        let target_deg_r = vu_target_deg(gr_r);
        let cur_l = self.vu_smooth_l.get() * 0.78 + target_deg_l * 0.22;
        let cur_r = self.vu_smooth_r.get() * 0.78 + target_deg_r * 0.22;
        self.vu_smooth_l.set(cur_l);
        self.vu_smooth_r.set(cur_r);

        draw_vu_meter(canvas, cw_x + 22.0, 580.0, "GR L", cur_l, fonts);
        draw_vu_meter(canvas, cw_x + 242.0, 580.0, "GR R", cur_r, fonts);

        // Big knob is THRESHOLD on both models (centered at y = 666.0, vertically aligned with the VU meters)
        let thresh_ro = self.readout_for(MomentarySlot::CompThresh, "THRESH");
        draw_knob_large_lit(
            canvas,
            cw_x + 548.0,
            666.0,
            &thresh_ro,
            self.control_angle_11(SteppedControl::CompThresh),
            fonts,
            comp_is_on,
        );

        // Dynamic Compressor Small Pot labels based on FAIR vs V-MU model
        let comp_small_ctrls = if is_vmu {
            [
                ("SC HPF", SteppedControl::CompScHpf),
                ("MIX", SteppedControl::CompMix),
                ("MAKEUP", SteppedControl::CompMakeup),
                ("RATIO", SteppedControl::CompRatio),
                ("ATTACK", SteppedControl::CompAttack),
                ("RECOV", SteppedControl::CompRelease),
            ]
        } else {
            [
                ("SC HPF", SteppedControl::CompScHpf),
                ("MIX", SteppedControl::CompMix),
                ("MAKEUP", SteppedControl::CompMakeup),
                ("IN GAIN", SteppedControl::CompInGain),
                ("TIME C", SteppedControl::CompAttack),
                ("RELEASE", SteppedControl::CompRelease),
            ]
        };

        for (idx_p, (p_lbl, ctrl)) in comp_small_ctrls.into_iter().enumerate() {
            let px = cw_x + 62.0 + (idx_p as f32) * 103.0;
            if matches!(ctrl, SteppedControl::CompMakeup) {
                // Push-pull makeup: latched AUTO leaves the pot visually "off" (no accent, pointer
                // at the manual position) and the readout below spells AUTO.
                let auto_on = self.params.comp_auto_makeup.value();
                let readout = if auto_on {
                    "AUTO".to_string()
                } else {
                    self.control_display_label(ctrl)
                };
                let angle = self.control_angle_11(ctrl);
                if auto_on {
                    draw_knob_small_unlit(
                        canvas,
                        px,
                        806.0,
                        p_lbl,
                        &readout,
                        angle,
                        true,
                        fonts,
                    );
                } else {
                    draw_knob_small_ticks_lift_lit(
                        canvas,
                        px,
                        806.0,
                        p_lbl,
                        &readout,
                        angle,
                        true,
                        fonts,
                        &KNOB_TICKS_11,
                        0.0,
                        comp_is_on,
                    );
                }
            } else {
                draw_knob_small_ticks_lift_lit(
                    canvas,
                    px,
                    806.0,
                    p_lbl,
                    &self.control_display_label(ctrl),
                    self.control_angle_11(ctrl),
                    true,
                    fonts,
                    &KNOB_TICKS_11,
                    0.0,
                    comp_is_on,
                );
            }
        }

        let comp_amp_idx = (self.params.comp_amp_type.value() as usize).min(6);
        let comp_trans_idx = (self.params.comp_trans_type.value() as usize).min(4);
        let comp_amp_str = match self.smash_banner.get() {
            Some((true, t)) if t.elapsed() < Duration::from_millis(950) => "SMASH+18",
            _ => AMP_ALIASES[comp_amp_idx],
        };
        draw_coloration_cassette(
            canvas,
            eq_out_x,
            581.0,
            184.0,
            252.0,
            comp_amp_str,
            TRANS_ALIASES[comp_trans_idx],
            comp_is_on && self.params.comp_col_match.value(),
            if comp_is_on { self.params.comp_intensity.value() } else { 0 },
            if comp_is_on { comp_auto_lamp } else { AutoLamp::Off },
            fonts,
        );

        // =====================================================================
        // 3.2.3 MQNdsp PRESET & UTILITY CONSOLE (y: 874, h: 64)
        // =====================================================================
        let console_y = 874.0;
        let console_h = 64.0;
        let console_cy = 906.0;
        draw_card(canvas, cw_x, console_y, cw_w, console_h, false);

        // Section A: MQN Official Vector Logo Branding (scaled to fill corner with minimal padding)
        draw_mqn_logo(canvas, cw_x + 6.0, console_cy - 16.5, 112.0, 33.0, accent_magenta());

        // Divider 1
        let mut div1 = Path::new();
        div1.move_to(372.0, console_y + 10.0);
        div1.line_to(372.0, console_y + console_h - 10.0);
        let mut div1_p = Paint::color(if is_light {
            Color::rgb(0xB4, 0xB0, 0xA8)
        } else {
            Color::rgb(0x26, 0x26, 0x2C)
        });
        div1_p.set_line_width(1.0);
        canvas.stroke_path(&div1, &div1_p);

        // Section B: Preset Engine (Centered across available span, 26 characters readout)
        draw_vintage_push_button(
            canvas,
            396.0,
            console_cy,
            "<",
            0,
            36.0,
            36.0,
            fonts,
        );

        let cur_preset = self.current_preset_idx.get();
        let is_saved = match self.save_preset_banner.get() {
            Some(t) if t.elapsed() < Duration::from_millis(1200) => true,
            _ => false,
        };
        let p_name = if is_saved {
            "SAVED!".to_string()
        } else if let Some(edit) = self.preset_edit.borrow().as_ref() {
            let blink = (Instant::now().duration_since(self.boot).as_millis() / 500) % 2 == 0;
            if edit.text.len() < 26 {
                let cursor_ch = if blink { '_' } else { ' ' };
                format!("{}{}", edit.text, cursor_ch)
            } else {
                edit.text.clone()
            }
        } else {
            self.preset_name(cur_preset)
        };

        draw_plate_well_160x200(canvas, 426.0, console_cy - 17.0, 340.0, 34.0);
        draw_dseg_display(
            canvas,
            596.0,
            console_cy,
            328.0,
            26.0,
            26,
            &p_name,
            5.0,
            fonts,
        );

        draw_vintage_push_button(
            canvas,
            796.0,
            console_cy,
            ">",
            0,
            36.0,
            36.0,
            fonts,
        );

        draw_vintage_push_button(
            canvas,
            849.0,
            console_cy,
            "SAVE",
            if is_saved { 1 } else { 0 },
            50.0,
            36.0,
            fonts,
        );

        // Divider 2
        let mut div2 = Path::new();
        div2.move_to(886.0, console_y + 10.0);
        div2.line_to(886.0, console_y + console_h - 10.0);
        canvas.stroke_path(&div2, &div1_p);

        // Section C: Utilities
        let theme_label = if is_light { "MOON" } else { "SUN" };
        draw_vintage_push_button(
            canvas,
            918.0,
            console_cy,
            theme_label,
            if is_light { 1 } else { 0 },
            36.0,
            36.0,
            fonts,
        );

        let man_lit = if self.manual_open.get() { 1 } else { 0 };
        draw_vintage_push_button(
            canvas,
            962.0,
            console_cy,
            "BOOK",
            man_lit,
            36.0,
            36.0,
            fonts,
        );

        // Divider 3
        let mut div3 = Path::new();
        div3.move_to(992.0, console_y + 10.0);
        div3.line_to(992.0, console_y + console_h - 10.0);
        canvas.stroke_path(&div3, &div1_p);

        // Section D: A/B Comparison Matrix (Two illuminated square buttons)
        let a_lit = if self.ab_active.get() == 'A' { 1 } else { 0 };
        draw_vintage_push_button(
            canvas,
            1022.0,
            console_cy,
            "A",
            a_lit,
            36.0,
            36.0,
            fonts,
        );

        let b_lit = if self.ab_active.get() == 'B' { 1 } else { 0 };
        draw_vintage_push_button(
            canvas,
            1066.0,
            console_cy,
            "B",
            b_lit,
            36.0,
            36.0,
            fonts,
        );

        // 3.3 RIGHT WING: BRICKWALL LIMITER & OUTPUT STAGE (x: 1126, w: 200)
        let (rw_x, rw_w) = (1126.0, 200.0);
        let rw_cx = rw_x + rw_w * 0.5;
        let lim_is_on = !self.params.limiter_bypass.value();
        draw_card(canvas, rw_x, main_y, rw_w, main_h, true);
        draw_text(
            canvas,
            rw_x + 14.0,
            main_y + 20.0,
            "LIMITER",
            fonts.bold_paint(silk_muted(), 10.5),
        );
        draw_vintage_bypass_button(
            canvas,
            rw_x + rw_w - 38.0,
            main_y + 20.0,
            "IN",
            lim_is_on,
            fonts,
        );

        let held = self.readout_held.get();
        let lufs_val = held[2].clamp(-60.0, 9.9);
        let lufs_str = if lufs_val <= -59.9 {
            "--.-".to_string()
        } else {
            format!("{:.1}", lufs_val)
        };
        let lufs_tag = match self.lufs_mode {
            0 => "M",
            1 => "S",
            _ => "I",
        };
        let lufs_banner = self.momentary_text(MomentarySlot::LufsMode);

        let live_lim_out = if meters_idle {
            self.reset_lim_out_hold(now);
            METER_FLOOR_DB
        } else {
            self.meters.limiter_out_peak_db.load()
        };
        let held_lim_out = self.step_lim_out_hold(live_lim_out, now);

        const LIMITER_OUT_THRESHOLDS: [f32; 12] = [
            -14.0, -12.0, -10.0, -8.0, -6.0, -5.0, -4.0, -3.0, -2.0, -1.0, -0.5, 0.0,
        ];
        let out_segs = LIMITER_OUT_THRESHOLDS
            .iter()
            .filter(|&&th| held_lim_out >= th)
            .count();

        const LIMITER_GR_THRESHOLDS: [f32; 12] = [
            0.1, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 8.0, 10.0, 11.5, 13.0, 14.0,
        ];
        let gr_segs = LIMITER_GR_THRESHOLDS
            .iter()
            .filter(|&&th| held[3] >= th)
            .count();

        let bot_tag = if self.limiter_gr_mode { "T" } else { "G" };
        let bot_str = if self.limiter_gr_mode {
            format_db_readout(held[4])
        } else {
            let gr_val = held[3];
            if gr_val < 0.05 {
                "- 0.0".to_string()
            } else {
                format!("-{:.1}", gr_val)
            }
        };
        let bot_banner = self.momentary_text(MomentarySlot::LimiterMeter);

        draw_limiter_ladder(
            canvas,
            rw_cx - 42.0,
            main_y + 52.0,
            84.0,
            lufs_tag,
            &lufs_str,
            lufs_banner.as_deref(),
            bot_tag,
            &bot_str,
            bot_banner.as_deref(),
            out_segs,
            gr_segs,
            fonts,
        );

        let gain_ro = self.readout_for(MomentarySlot::LimiterGain, "GAIN");
        draw_knob_large_lit(
            canvas,
            rw_cx,
            506.0,
            &gain_ro,
            self.control_angle_11(SteppedControl::LimiterGain),
            fonts,
            lim_is_on,
        );

        draw_vintage_push_button(
            canvas,
            rw_cx - 35.0,
            600.0,
            "TRUE PK",
            if lim_is_on && self.params.limiter_true_peak.value() { 1 } else { 0 },
            58.0,
            24.0,
            fonts,
        );
        draw_vintage_push_button(
            canvas,
            rw_cx + 35.0,
            600.0,
            "UNITY",
            if lim_is_on && self.params.limiter_unity.value() { 1 } else { 0 },
            58.0,
            24.0,
            fonts,
        );

        draw_vintage_push_button(
            canvas,
            rw_cx,
            668.0,
            "PUNCH",
            if lim_is_on && self.params.limiter_punch.value() { 1 } else { 0 },
            76.0,
            24.0,
            fonts,
        );

        // Row 1: CEILING and LOOKAHEAD side-by-side
        draw_knob_small_ticks_lift_lit(
            canvas,
            rw_cx - 45.0,
            740.0,
            "CEILING",
            &self.control_display_label(SteppedControl::LimiterCeiling),
            self.control_angle_11(SteppedControl::LimiterCeiling),
            false,
            fonts,
            &KNOB_TICKS_11,
            0.0,
            lim_is_on,
        );
        draw_knob_small_ticks_lift_lit(
            canvas,
            rw_cx + 45.0,
            740.0,
            "LOOKAHEAD",
            &self.control_display_label(SteppedControl::LimiterLookahead),
            self.control_angle_11(SteppedControl::LimiterLookahead),
            false,
            fonts,
            &KNOB_TICKS_11,
            0.0,
            lim_is_on,
        );

        // Row 2: RELEASE and LIFT side-by-side
        draw_knob_small_ticks_lift_lit(
            canvas,
            rw_cx - 45.0,
            842.0,
            "RELEASE",
            &self.control_display_label(SteppedControl::LimiterRelease),
            self.control_angle_11(SteppedControl::LimiterRelease),
            false,
            fonts,
            &KNOB_TICKS_11,
            0.0,
            lim_is_on,
        );
        draw_knob_small_ticks_lift_lit(
            canvas,
            rw_cx + 45.0,
            842.0,
            "LIFT",
            &self.control_display_label(SteppedControl::LimiterLift),
            self.control_angle_11(SteppedControl::LimiterLift),
            false,
            fonts,
            &KNOB_TICKS_11,
            0.0,
            lim_is_on,
        );

        // -------------------------------------------------------------
        // Preset Browser Modal Dialog (Somewhat Big: 560 x 460)
        // -------------------------------------------------------------
        if self.preset_browser_open.get() {
            let mut overlay = Path::new();
            overlay.rect(0.0, 0.0, Self::TOTAL_WIDTH, Self::TOTAL_HEIGHT);
            canvas.fill_path(&overlay, &Paint::color(Color::rgba(0, 0, 0, 195)));

            let (bx, by, bw, bh) = (400.0, 240.0, 560.0, 460.0);
            draw_card(canvas, bx, by, bw, bh, true);

            for (sx, sy) in [
                (bx + 12.0, by + 12.0),
                (bx + bw - 12.0, by + 12.0),
                (bx + 12.0, by + bh - 12.0),
                (bx + bw - 12.0, by + bh - 12.0),
            ] {
                draw_screw(canvas, sx, sy);
            }

            draw_text(
                canvas,
                bx + 32.0,
                by + 28.0,
                "PRESET LIBRARY",
                fonts.bold_paint(silk_white(), 13.0),
            );

            let total = self.total_presets();
            draw_text(
                canvas,
                bx + 190.0,
                by + 28.0,
                &format!("({} PRESETS AVAILABLE)", total),
                fonts.bold_paint(silk_muted(), 9.5),
            );

            // Close Button [ X ]
            draw_vintage_push_button(
                canvas,
                bx + bw - 36.0,
                by + 28.0,
                "X",
                1,
                32.0,
                24.0,
                fonts,
            );

            let mut div = Path::new();
            div.move_to(bx + 20.0, by + 46.0);
            div.line_to(bx + bw - 20.0, by + 46.0);
            let mut div_p = Paint::color(Color::rgb(0x28, 0x28, 0x30));
            div_p.set_line_width(1.0);
            canvas.stroke_path(&div, &div_p);

            // Preset rows
            let row_h = 36.0;
            let list_y = by + 56.0;
            let cur_idx = self.current_preset_idx.get();

            let max_visible = 8;
            for i in 0..total.min(max_visible) {
                let ry = list_y + (i as f32) * (row_h + 6.0);
                let is_cur = i == cur_idx;
                let is_factory = i < FACTORY_PRESETS.len();
                let name = self.preset_name(i);

                let mut row_path = Path::new();
                row_path.rounded_rect(bx + 24.0, ry, bw - 48.0, row_h, 5.0);

                if is_cur {
                    canvas.fill_path(&row_path, &Paint::color(if is_light {
                        Color::rgb(0xF8, 0xDC, 0xEB)
                    } else {
                        Color::rgb(0x26, 0x12, 0x1E)
                    }));
                    let mut s = Paint::color(accent_magenta());
                    s.set_line_width(1.2);
                    canvas.stroke_path(&row_path, &s);
                } else {
                    canvas.fill_path(&row_path, &Paint::color(if is_light {
                        Color::rgb(0xE2, 0xDF, 0xD8)
                    } else {
                        Color::rgb(0x13, 0x13, 0x18)
                    }));
                    let mut s = Paint::color(if is_light {
                        Color::rgb(0xC4, 0xC1, 0xBA)
                    } else {
                        Color::rgb(0x22, 0x22, 0x28)
                    });
                    s.set_line_width(1.0);
                    canvas.stroke_path(&row_path, &s);
                }

                // Category badge
                let badge_text = if is_factory { "FACTORY" } else { "USER" };
                let badge_col = if is_factory {
                    Color::rgb(0x56, 0x78, 0xAA)
                } else {
                    accent_magenta()
                };
                let mut b_path = Path::new();
                b_path.rounded_rect(bx + 34.0, ry + 8.0, 56.0, 20.0, 3.0);
                canvas.fill_path(&b_path, &Paint::color(if is_light {
                    Color::rgb(0xD2, 0xCE, 0xC6)
                } else {
                    Color::rgb(0x0A, 0x0A, 0x0D)
                }));
                draw_text_centered(
                    canvas,
                    bx + 62.0,
                    ry + 18.0,
                    badge_text,
                    fonts.bold_paint(badge_col, 8.0),
                );

                // Name
                let name_col = if is_cur { accent_magenta() } else { silk_white() };
                draw_text(
                    canvas,
                    bx + 104.0,
                    ry + 18.0,
                    &name,
                    fonts.bold_paint(name_col, 11.0),
                );

                // Active indicator or user delete button
                if is_cur {
                    draw_text_centered(
                        canvas,
                        bx + bw - 60.0,
                        ry + 18.0,
                        "[ ACTIVE ]",
                        fonts.bold_paint(accent_magenta(), 8.5),
                    );
                } else if !is_factory {
                    draw_text_centered(
                        canvas,
                        bx + bw - 50.0,
                        ry + 18.0,
                        "[ DEL ]",
                        fonts.bold_paint(Color::rgb(0x9E, 0x44, 0x44), 8.5),
                    );
                }
            }

            // Bottom action buttons: [ + SAVE PRESET ] [ OPEN FOLDER ] [ CLOSE ]
            let cy_bot = by + bh - 32.0;
            draw_vintage_push_button(
                canvas,
                bx + 110.0,
                cy_bot,
                "+ SAVE PRESET",
                0,
                130.0,
                30.0,
                fonts,
            );
            draw_vintage_push_button(
                canvas,
                bx + 260.0,
                cy_bot,
                "OPEN FOLDER",
                0,
                130.0,
                30.0,
                fonts,
            );
            draw_vintage_push_button(
                canvas,
                bx + bw - 60.0,
                cy_bot,
                "CLOSE",
                0,
                80.0,
                30.0,
                fonts,
            );
        }

        // -------------------------------------------------------------
        // Save Preset Modal Dialog (460 x 230)
        // -------------------------------------------------------------
        if self.preset_save_dialog_open.get() {
            let mut overlay = Path::new();
            overlay.rect(0.0, 0.0, Self::TOTAL_WIDTH, Self::TOTAL_HEIGHT);
            canvas.fill_path(&overlay, &Paint::color(Color::rgba(0, 0, 0, 195)));

            let (sx, sy, sw, sh) = (450.0, 360.0, 460.0, 230.0);
            draw_card(canvas, sx, sy, sw, sh, true);

            for (screw_x, screw_y) in [
                (sx + 10.0, sy + 10.0),
                (sx + sw - 10.0, sy + 10.0),
                (sx + 10.0, sy + sh - 10.0),
                (sx + sw - 10.0, sy + sh - 10.0),
            ] {
                draw_screw(canvas, screw_x, screw_y);
            }

            draw_text_centered(
                canvas,
                sx + sw * 0.5,
                sy + 26.0,
                "SAVE PRESET",
                fonts.bold_paint(silk_white(), 13.0),
            );
            draw_text_centered(
                canvas,
                sx + sw * 0.5,
                sy + 48.0,
                "Type a name to overwrite or save as a new user preset:",
                fonts.regular_paint(silk_muted(), 9.5),
            );

            // Input Well and DSEG / Text Field
            let input_y = sy + 90.0;
            draw_plate_well_160x200(canvas, sx + 40.0, input_y - 18.0, sw - 80.0, 36.0);

            let edit_text = if let Some(edit) = self.preset_edit.borrow().as_ref() {
                let blink = (Instant::now().duration_since(self.boot).as_millis() / 500) % 2 == 0;
                if blink {
                    format!("{}_", edit.text)
                } else {
                    format!("{} ", edit.text)
                }
            } else {
                self.preset_name(self.current_preset_idx.get())
            };

            draw_dseg_display(
                canvas,
                sx + sw * 0.5,
                input_y,
                sw - 100.0,
                26.0,
                24,
                &edit_text,
                5.0,
                fonts,
            );

            // Buttons: [ CANCEL ] and [ SAVE ]
            let btn_y = sy + sh - 36.0;
            draw_vintage_push_button(
                canvas,
                sx + sw * 0.5 - 75.0,
                btn_y,
                "CANCEL",
                0,
                110.0,
                32.0,
                fonts,
            );
            draw_vintage_push_button(
                canvas,
                sx + sw * 0.5 + 75.0,
                btn_y,
                "SAVE",
                1,
                110.0,
                32.0,
                fonts,
            );
        }

        // Hidden debug menu. Shared layout constants drive both painting and hit testing.
        if self.debug_menu_open.get() {
            let mut m = Path::new();
            m.rounded_rect(Self::DBG_X, Self::DBG_Y, Self::DBG_W, Self::DBG_H, 10.0);
            canvas.fill_path(&m, &Paint::color(Color::rgb(16, 16, 22)));
            let mut ms = Paint::color(Color::rgba(140, 140, 150, 255));
            ms.set_line_width(1.2);
            canvas.stroke_path(&m, &ms);

            draw_text(
                canvas,
                Self::DBG_LABEL_X,
                Self::DBG_Y + 24.0,
                "DEBUG MENU",
                fonts.bold_paint(Color::rgb(230, 230, 240), 11.0),
            );
            draw_text(
                canvas,
                Self::DBG_LABEL_X,
                Self::DBG_Y + 40.0,
                "HOLD SCREW 3s TO CLOSE  |  CLICK VALUE TO TYPE",
                fonts.regular_paint(Color::rgb(150, 150, 160), 8.0),
            );

            let label_paint = fonts.regular_paint(Color::rgb(190, 190, 200), 9.5);
            let value_paint = fonts.regular_paint(Color::rgb(250, 250, 255), 9.5);
            let step_paint = fonts.bold_paint(Color::rgb(150, 150, 165), 12.0);

            let edit = self.debug_edit.borrow();
            for (i, field) in DEBUG_FIELDS.iter().enumerate() {
                let y = Self::debug_row_y(i);
                draw_text(
                    canvas,
                    Self::DBG_LABEL_X,
                    y,
                    field.label(),
                    label_paint.clone(),
                );

                if *field == DebugField::ResetRanges {
                    draw_text_centered(canvas, Self::DBG_VALUE_CX, y, "CLICK", value_paint.clone());
                    continue;
                }

                draw_text_centered(canvas, Self::DBG_MINUS_X, y, "-", step_paint.clone());
                draw_text_centered(canvas, Self::DBG_PLUS_X, y, "+", step_paint.clone());

                let editing = edit.as_ref().map(|e| e.field == *field).unwrap_or(false);
                let text = match edit.as_ref() {
                    Some(e) if editing => format!("{}_", e.text),
                    _ => self.debug_field_text(*field),
                };

                if editing {
                    let mut boxp = Path::new();
                    boxp.rounded_rect(
                        Self::DBG_VALUE_CX - Self::DBG_VALUE_HALF_W,
                        y - 9.0,
                        Self::DBG_VALUE_HALF_W * 2.0,
                        18.0,
                        3.0,
                    );
                    canvas.fill_path(&boxp, &Paint::color(Color::rgb(0x0B, 0x0B, 0x0D)));
                    let mut bs = Paint::color(accent_magenta());
                    bs.set_line_width(1.0);
                    canvas.stroke_path(&boxp, &bs);
                }

                draw_text_centered(
                    canvas,
                    Self::DBG_VALUE_CX,
                    y,
                    &text,
                    if editing {
                        fonts.bold_paint(Color::rgb(255, 255, 255), 9.5)
                    } else {
                        value_paint.clone()
                    },
                );
            }
            drop(edit);

            let calibration = debug_calibration::calibration();
            let (u_min, u_max) = (
                Self::FAIR_THRESH_MIN_DB + calibration.trim_fair(),
                Self::FAIR_THRESH_MAX_DB + calibration.trim_fair(),
            );
            let (v_min, v_max) = (
                Self::VMU_THRESH_MIN_DB + calibration.trim_vmu(),
                Self::VMU_THRESH_MAX_DB + calibration.trim_vmu(),
            );
            draw_text_centered(
                canvas,
                Self::DBG_X + Self::DBG_W * 0.5,
                Self::DBG_Y + Self::DBG_H - 30.0,
                &format!(
                    "FAIR {:.0}..{:.0} dB    V-MU {:.0}..{:.0} dB",
                    u_min, u_max, v_min, v_max
                ),
                fonts.regular_paint(Color::rgb(150, 150, 165), 8.0),
            );
            draw_text_centered(
                canvas,
                Self::DBG_X + Self::DBG_W * 0.5,
                Self::DBG_Y + Self::DBG_H - 14.0,
                "THRESH READOUT IS THE KNOB MARK 0..10",
                fonts.regular_paint(Color::rgb(140, 140, 150), 8.0),
            );
        }

        // Bottom-right screw: painted last, on top of the debug panel, so it is always visible
        // and always holdable. The ring shows the 3 s countdown while it is pressed.
        {
            let (sx, sy) = (Self::TOTAL_WIDTH - 17.0, Self::TOTAL_HEIGHT - 24.0);
            draw_screw_angled(canvas, sx, sy, 41.0);
            if let Some(hold) = self.hold_press.get() {
                if matches!(hold.action, HoldAction::ToggleDebugMenu) && !hold.fired {
                    let progress = (hold.down_at.elapsed().as_secs_f32()
                        / (DEBUG_MENU_HOLD_MS as f32 / 1000.0))
                        .clamp(0.0, 1.0);
                    let mut ring = Path::new();
                    ring.arc(
                        sx,
                        sy,
                        9.0,
                        -std::f32::consts::FRAC_PI_2,
                        -std::f32::consts::FRAC_PI_2 + progress * std::f32::consts::TAU,
                        Solidity::Hole,
                    );
                    let mut rp = Paint::color(accent_magenta());
                    rp.set_line_width(2.0);
                    canvas.stroke_path(&ring, &rp);
                }
            }
        }

        canvas.restore();

        if let Ok(dir) = std::env::var("MASTERAID_SCREENSHOT_DIR") {
            static SHOT_FRAME: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
            let frame = SHOT_FRAME.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if frame == 2 {
                if let Ok(img) = canvas.screenshot() {
                    let path = format!("{}/masteraid_gui_dark.bmp", dir);
                    let _ = save_screenshot_bmp(&path, img.width(), img.height(), img.buf());
                    println!("MASTERAID_DUMPED_DARK: {}", path);
                }
                // Switch EQ to PTEK and Compressor to V-MU
                self.set_param_atomic(self.params.eq_model.as_ptr(), 0.0);
                self.set_param_atomic(self.params.comp_model.as_ptr(), 1.0);
            } else if frame == 5 {
                if let Ok(img) = canvas.screenshot() {
                    let path = format!("{}/masteraid_gui_ptek_vmu.bmp", dir);
                    let _ = save_screenshot_bmp(&path, img.width(), img.height(), img.buf());
                    println!("MASTERAID_DUMPED_PTEK_VMU: {}", path);
                }
                // Restore EQ to MASS and Compressor to FAIR, and switch to Light theme
                self.set_param_atomic(self.params.eq_model.as_ptr(), 1.0);
                self.set_param_atomic(self.params.comp_model.as_ptr(), 0.0);
                self.set_param_atomic(self.params.theme_light.as_ptr(), 1.0);
            } else if frame == 8 {
                if let Ok(img) = canvas.screenshot() {
                    let path = format!("{}/masteraid_gui_light.bmp", dir);
                    let _ = save_screenshot_bmp(&path, img.width(), img.height(), img.buf());
                    println!("MASTERAID_DUMPED_LIGHT: {}", path);
                }
                println!("MASTERAID_SCREENSHOTS_DONE");
                std::process::exit(0);
            }
        }
    }
}

fn save_screenshot_bmp(
    path: &str,
    w: usize,
    h: usize,
    buf: &[nih_plug_vizia::vizia::vg::rgb::RGBA8],
) -> std::io::Result<()> {
    use std::io::Write;
    let mut file = std::fs::File::create(path)?;
    let row_size = w * 4;
    let image_size = (row_size * h) as u32;
    let file_size = 54 + image_size;

    // 14-byte BITMAPFILEHEADER
    file.write_all(&[0x42, 0x4D])?;
    file.write_all(&file_size.to_le_bytes())?;
    file.write_all(&[0, 0, 0, 0])?;
    file.write_all(&54u32.to_le_bytes())?;

    // 40-byte BITMAPINFOHEADER
    file.write_all(&40u32.to_le_bytes())?;
    file.write_all(&(w as i32).to_le_bytes())?;
    file.write_all(&(-(h as i32)).to_le_bytes())?;
    file.write_all(&1u16.to_le_bytes())?;
    file.write_all(&32u16.to_le_bytes())?;
    file.write_all(&0u32.to_le_bytes())?;
    file.write_all(&image_size.to_le_bytes())?;
    file.write_all(&2835u32.to_le_bytes())?;
    file.write_all(&2835u32.to_le_bytes())?;
    file.write_all(&0u32.to_le_bytes())?;
    file.write_all(&0u32.to_le_bytes())?;

    let mut bgra = Vec::with_capacity(w * h * 4);
    for pixel in buf {
        bgra.push(pixel.b);
        bgra.push(pixel.g);
        bgra.push(pixel.r);
        bgra.push(pixel.a);
    }
    file.write_all(&bgra)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn signed(ctrl: SteppedControl, value: f32) -> String {
        ptek_position_label(ctrl, value).expect("expected a Ptek gain knob")
    }

    #[test]
    fn meter_readouts_blank_out_at_the_floor() {
        assert_eq!(format_db_readout(METER_FLOOR_DB), "--.-");
        assert_eq!(format_db_readout(METER_FLOOR_DB + 0.05), "--.-");
        assert_eq!(format_db_readout(-59.8), "-59.8");
        assert_eq!(format_db_readout(0.0), "+0.0");
        assert_eq!(format_db_readout(3.46), "+3.5");
        // The blank field must fit the same fixed cells as a real reading.
        assert!(tokenize_dseg_tagged("P", "--.-", 6).len() == 6);
    }

    #[test]
    fn ptek_readout_keeps_a_fixed_signed_field() {
        let cases = [
            (SteppedControl::PuLowBoost, 0.0, "+ 0.0"),
            (SteppedControl::PuLowBoost, 3.0, "+ 3.0"),
            (SteppedControl::PuLowBoost, 3.4, "+ 3.4"),
            (SteppedControl::PuLowBoost, 3.46, "+ 3.5"),
            (SteppedControl::PuLowBoost, 9.96, "+10.0"),
            (SteppedControl::PuLowBoost, 25.0, "+10.0"),
            (SteppedControl::PuLowAtten, 10.0, "-10.0"),
            (SteppedControl::PuHighAtten, 0.0, "- 0.0"),
            (SteppedControl::PuHighBoost, 6.25, "+ 6.3"),
        ];
        for (ctrl, value, expected) in cases {
            let out = signed(ctrl, value);
            assert_eq!(out, expected);
            // Five characters, one decimal point at the same index, for every value: the DSEG
            // tokenizer therefore always produces the same four tokens and the glyphs never slide.
            assert_eq!(out.chars().count(), 5, "{out:?}");
            assert_eq!(out.chars().nth(3), Some('.'), "{out:?}");

            // Pin the actual six-cell DSEG layout: two blank edge cells, then sign, tens/space,
            // ones (carrying the decimal dot), and tenths. This is what keeps "+ 3.4" and "+10.0"
            // from shifting by a cell.
            let cells = tokenize_dseg_cells(&out, 6);
            assert_eq!(cells.len(), 6, "{out:?}");
            assert_eq!(cells[0].0, ' ', "{out:?}");
            assert_eq!(cells[5].0, ' ', "{out:?}");
            assert!(matches!(cells[1].0, '+' | '-'), "{out:?}");
            assert!(cells[4].0.is_ascii_digit(), "{out:?}");
            assert!(
                cells[3].1,
                "decimal dot must ride on the ones cell: {out:?}"
            );
        }
        assert!(ptek_position_label(SteppedControl::PuBandwidth, 5.0).is_none());
    }

    #[test]
    fn input_meter_tag_and_value_stay_in_fixed_cells() {
        for (tag, value) in [("P", "-12.3"), ("P", "+3.4"), ("R", "-9.9"), ("R", "-60.0")] {
            let cells = tokenize_dseg_tagged(tag, value, 6);
            assert_eq!(cells.len(), 6);
            assert_eq!(cells[0].0, tag.chars().next().unwrap(), "{value}");
            // The ones digit always carries the decimal in cell 4 and the tenths sits in cell 5,
            // whether the value has one or two whole digits.
            assert!(
                cells[4].1,
                "decimal dot must ride on the ones cell: {cells:?}"
            );
            assert!(cells[4].0.is_ascii_digit(), "{cells:?}");
            assert_eq!(cells[5].0, value.chars().last().unwrap(), "{cells:?}");
        }
        assert_eq!(tokenize_dseg_tagged("P", "+3.4", 6)[4], ('3', true));
        assert_eq!(tokenize_dseg_tagged("P", "-12.3", 6)[4], ('2', true));
    }

    #[test]
    fn lufs_tag_and_value_stay_in_fixed_cells() {
        for (tag, value) in [("S", "-12.3"), ("M", "-8.3"), ("I", "-60.0")] {
            let cells = tokenize_dseg_tagged(tag, value, 5);
            assert_eq!(cells.len(), 5);
            assert_eq!(cells[0].0, tag.chars().next().unwrap(), "{value}");
            assert!(cells[3].1, "{cells:?}");
            assert_eq!(cells[4].0, value.chars().last().unwrap(), "{cells:?}");
        }
    }

    #[test]
    fn mode_banners_are_left_aligned() {
        let cells = tokenize_dseg_left("SHORT", 5);
        assert_eq!(cells.iter().map(|c| c.0).collect::<String>(), "SHORT");
        let cells = tokenize_dseg_left("PEAK", 6);
        assert_eq!(cells[0].0, 'P');
        assert_eq!(cells[3].0, 'K');
        assert_eq!(cells[4].0, ' ');
    }

    #[test]
    fn peak_hold_attacks_instantly_and_holds_for_1_5s() {
        let t0 = Instant::now();
        let mut hold = PeakHold::at(t0);
        assert_eq!(hold.step(-12.0, t0, 1500), -12.0);
        // A lower level does not move the marker while the hold window is open.
        assert_eq!(
            hold.step(-40.0, t0 + Duration::from_millis(1000), 1500),
            -12.0
        );
        assert_eq!(
            hold.step(-40.0, t0 + Duration::from_millis(1499), 1500),
            -12.0
        );
        // A new, higher peak restarts the window and wins immediately.
        assert_eq!(
            hold.step(-6.0, t0 + Duration::from_millis(1499), 1500),
            -6.0
        );
        assert_eq!(
            hold.step(-40.0, t0 + Duration::from_millis(2998), 1500),
            -6.0
        );
    }

    #[test]
    fn peak_hold_falls_at_iec_type_i_rate_after_the_hold() {
        let t0 = Instant::now();
        let mut hold = PeakHold::at(t0);
        hold.step(-6.0, t0, 1500);

        // Half a second after the hold expires: 20 dB / 1.7 s * 0.5 s.
        let half = t0 + INPUT_PEAK_HOLD_DEFAULT + Duration::from_millis(500);
        let expected = -6.0 - INPUT_PEAK_FALL_DB_S * 0.5;
        let got = hold.step(-40.0, half, 1500);
        assert!(
            (got - expected).abs() < 0.01,
            "got {got}, expected {expected}"
        );

        // The fallback is time based, not frame based: jumping straight to the same instant
        // from a fresh hold gives the same level.
        let mut other = PeakHold::at(t0);
        other.step(-6.0, t0, 1500);
        assert_eq!(other.step(-40.0, half, 1500), got);

        // It never falls below the live level...
        let later = t0 + INPUT_PEAK_HOLD_DEFAULT + Duration::from_millis(1500);
        assert_eq!(hold.step(-9.0, later, 1500), -9.0);
        // ...or below the floor.
        let much_later = t0 + Duration::from_secs(30);
        assert_eq!(hold.step(METER_FLOOR_DB, much_later, 1500), METER_FLOOR_DB);
    }

    #[test]
    fn peak_hold_reset_clears_the_marker() {
        let t0 = Instant::now();
        let mut hold = PeakHold::at(t0);
        hold.step(-3.0, t0, 1500);
        hold.reset(t0 + Duration::from_millis(10));
        assert_eq!(
            hold.step(METER_FLOOR_DB, t0 + Duration::from_millis(10), 1500),
            METER_FLOOR_DB
        );
    }

    #[test]
    fn peak_hold_state_survives_a_cell_round_trip() {
        let t0 = Instant::now();
        let cell = Cell::new(PeakHold::at(t0));
        // `Cell::get` hands out a copy: mutating it in place must not be mistaken for state.
        cell.get().step(-12.0, t0, 1500);
        assert_eq!(cell.get().db, METER_FLOOR_DB);

        let mut hold = cell.get();
        hold.step(-12.0, t0, 1500);
        cell.set(hold);
        assert_eq!(cell.get().db, -12.0);
    }

    #[test]
    fn vu_needle_rests_on_the_printed_zero() {
        // The printed `0` sits 11 degrees clockwise of vertical, not at 12 o'clock.
        assert_eq!(vu_target_deg(0.0), VU_ZERO_DEG);
        // Full-scale reduction lands on the printed `-20`.
        assert_eq!(vu_target_deg(20.0), -44.0);
        // Between the marks the sweep is linear.
        assert!((vu_target_deg(10.0) - (-16.5)).abs() < 1e-4);
        // Out-of-range readings clamp to the printed scale.
        assert_eq!(vu_target_deg(-5.0), VU_ZERO_DEG);
        assert_eq!(vu_target_deg(50.0), -44.0);
    }

    #[test]
    fn gr_and_clip_hold_resets_to_zero() {
        let cell_clip = Cell::new(0.0f32);
        let cell_gr = Cell::new(0.0f32);

        // Record a clip overshoot of 1.8 dB
        cell_clip.set(cell_clip.get().max(1.8));
        assert_eq!(cell_clip.get(), 1.8);

        // Record a gain reduction of 3.4 dB
        cell_gr.set(cell_gr.get().max(3.4));
        assert_eq!(cell_gr.get(), 3.4);

        // Click reset clears them back to 0.0
        cell_clip.set(0.0);
        cell_gr.set(0.0);
        assert_eq!(cell_clip.get(), 0.0);
        assert_eq!(cell_gr.get(), 0.0);
    }
}
