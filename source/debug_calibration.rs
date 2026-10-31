//! Persistent debug-menu calibration.
//!
//! These values are a one-time hardware/room calibration, not part of a project. Plugin state
//! (`#[persist]` fields and host parameters) is saved per instance, so deleting a plugin from the
//! DAW and inserting a new one starts from the defaults again. This store instead writes a small
//! file into the user's config directory and loads it whenever the plugin is constructed, which is
//! what makes the calibration survive a remove/re-insert cycle.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::OnceLock;

pub const DEFAULT_PEAK_HOLD_MS: u32 = 1500;
/// `0` means "use the measurement engine's own default".
pub const DEFAULT_AUTO_LEARN_S: f32 = 0.0;

const PEAK_HOLD_MIN_MS: u32 = 500;
const PEAK_HOLD_MAX_MS: u32 = 10_000;
const AUTO_LEARN_MAX_S: f32 = 60.0;
const TRIM_LIMIT_DB: f32 = 12.0;

/// Calibration values shared by the GUI and the DSP. Floats are stored as raw bits so the store
/// stays lock-free: the audio thread reads them while the GUI writes them.
#[derive(Debug)]
pub struct DebugCalibration {
    peak_hold_ms: AtomicU32,
    auto_learn_s: AtomicU32,
    trim_fair: AtomicU32,
    trim_vmu: AtomicU32,
}

impl DebugCalibration {
    fn with_defaults() -> Self {
        Self {
            peak_hold_ms: AtomicU32::new(DEFAULT_PEAK_HOLD_MS),
            auto_learn_s: AtomicU32::new(DEFAULT_AUTO_LEARN_S.to_bits()),
            trim_fair: AtomicU32::new(0.0f32.to_bits()),
            trim_vmu: AtomicU32::new(0.0f32.to_bits()),
        }
    }

    /// Read the calibration file, falling back to defaults for anything missing or invalid.
    fn load() -> Self {
        match config_path() {
            Some(path) => Self::load_from(&path),
            None => Self::with_defaults(),
        }
    }

    fn load_from(path: &Path) -> Self {
        let calibration = Self::with_defaults();
        let Ok(text) = fs::read_to_string(path) else {
            return calibration;
        };
        calibration.apply_text(&text);
        calibration
    }

    fn apply_text(&self, text: &str) {
        // Tolerate a UTF-8 BOM if the file was created or edited by a Windows text editor.
        for line in text.trim_start_matches('\u{feff}').lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let Ok(value) = value.trim().parse::<f32>() else {
                continue;
            };
            if !value.is_finite() {
                continue;
            }
            match key.trim() {
                "peak_hold_ms" => self.store_peak_hold_ms(value.round() as i64),
                "auto_learn_s" => self.store_auto_learn_s(value),
                "trim_fair" => self.store_trim_fair(value),
                "trim_vmu" => self.store_trim_vmu(value),
                _ => {}
            }
        }
    }

    pub fn peak_hold_ms(&self) -> u32 {
        self.peak_hold_ms.load(Ordering::Relaxed)
    }

    pub fn auto_learn_s(&self) -> f32 {
        f32::from_bits(self.auto_learn_s.load(Ordering::Relaxed))
    }

    pub fn trim_fair(&self) -> f32 {
        f32::from_bits(self.trim_fair.load(Ordering::Relaxed))
    }

    pub fn trim_vmu(&self) -> f32 {
        f32::from_bits(self.trim_vmu.load(Ordering::Relaxed))
    }

    pub fn set_peak_hold_ms(&self, value: u32) {
        self.store_peak_hold_ms(value as i64);
        self.save();
    }

    pub fn set_auto_learn_s(&self, value: f32) {
        self.store_auto_learn_s(value);
        self.save();
    }

    pub fn set_trim_fair(&self, value: f32) {
        self.store_trim_fair(value);
        self.save();
    }

    pub fn set_trim_vmu(&self, value: f32) {
        self.store_trim_vmu(value);
        self.save();
    }

    /// Reset both threshold trims with a single file write.
    pub fn reset_trims(&self) {
        self.store_trim_fair(0.0);
        self.store_trim_vmu(0.0);
        self.save();
    }

    /// Replace every value with what is on disk now. Called when an editor opens, so a freshly
    /// inserted instance reflects the file even if another process changed it after this process
    /// first loaded the store. A missing file resets to defaults, which is the documented way to
    /// start over.
    pub fn reload(&self) {
        match config_path() {
            Some(path) => self.reload_from(&path),
            None => self.apply_values(&Self::with_defaults()),
        }
    }

    fn reload_from(&self, path: &Path) {
        self.apply_values(&Self::load_from(path));
    }

    fn apply_values(&self, values: &Self) {
        self.peak_hold_ms
            .store(values.peak_hold_ms(), Ordering::Relaxed);
        self.auto_learn_s
            .store(values.auto_learn_s().to_bits(), Ordering::Relaxed);
        self.trim_fair
            .store(values.trim_fair().to_bits(), Ordering::Relaxed);
        self.trim_vmu
            .store(values.trim_vmu().to_bits(), Ordering::Relaxed);
    }

    fn store_peak_hold_ms(&self, value: i64) {
        let clamped = value.clamp(PEAK_HOLD_MIN_MS as i64, PEAK_HOLD_MAX_MS as i64) as u32;
        self.peak_hold_ms.store(clamped, Ordering::Relaxed);
    }

    fn store_auto_learn_s(&self, value: f32) {
        let clamped = if value.is_finite() {
            value.clamp(0.0, AUTO_LEARN_MAX_S)
        } else {
            DEFAULT_AUTO_LEARN_S
        };
        self.auto_learn_s
            .store(clamped.to_bits(), Ordering::Relaxed);
    }

    fn store_trim_fair(&self, value: f32) {
        self.trim_fair
            .store(clamp_trim(value).to_bits(), Ordering::Relaxed);
    }

    fn store_trim_vmu(&self, value: f32) {
        self.trim_vmu
            .store(clamp_trim(value).to_bits(), Ordering::Relaxed);
    }

    fn save(&self) {
        if let Some(path) = config_path() {
            self.save_to(&path);
        }
    }

    fn save_to(&self, path: &Path) {
        if let Some(dir) = path.parent() {
            let _ = fs::create_dir_all(dir);
        }
        let body = format!(
            "# Masteraid debug calibration. Delete this file to reset every value.\n\
             peak_hold_ms={}\n\
             auto_learn_s={}\n\
             trim_fair={}\n\
             trim_vmu={}\n",
            self.peak_hold_ms(),
            self.auto_learn_s(),
            self.trim_fair(),
            self.trim_vmu(),
        );
        // Write through a temporary file so a crash mid-write can never leave a truncated
        // calibration behind.
        let tmp = path.with_extension("tmp");
        if fs::write(&tmp, body).is_ok() && fs::rename(&tmp, path).is_err() {
            let _ = fs::remove_file(&tmp);
        }
    }
}

fn clamp_trim(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(-TRIM_LIMIT_DB, TRIM_LIMIT_DB)
    } else {
        0.0
    }
}

fn config_path() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    let base = std::env::var_os("APPDATA").map(PathBuf::from);

    #[cfg(target_os = "macos")]
    let base = std::env::var_os("HOME")
        .map(PathBuf::from)
        .map(|home| home.join("Library").join("Application Support"));

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")));

    base.map(|base| {
        base.join("MQN")
            .join("Masteraid")
            .join("debug-calibration.txt")
    })
}

/// Process-wide calibration. It is loaded once, the first time any thread asks for it, and every
/// plugin instance in the process shares the same values.
pub fn calibration() -> &'static DebugCalibration {
    static CALIBRATION: OnceLock<DebugCalibration> = OnceLock::new();
    CALIBRATION.get_or_init(DebugCalibration::load)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamps_out_of_range_values() {
        let calibration = DebugCalibration::with_defaults();
        calibration.store_peak_hold_ms(42);
        assert_eq!(calibration.peak_hold_ms(), PEAK_HOLD_MIN_MS);
        calibration.store_peak_hold_ms(999_999);
        assert_eq!(calibration.peak_hold_ms(), PEAK_HOLD_MAX_MS);
        calibration.store_auto_learn_s(999.0);
        assert_eq!(calibration.auto_learn_s(), AUTO_LEARN_MAX_S);
        calibration.store_trim_fair(-99.0);
        assert_eq!(calibration.trim_fair(), -TRIM_LIMIT_DB);
        calibration.store_trim_vmu(99.0);
        assert_eq!(calibration.trim_vmu(), TRIM_LIMIT_DB);
    }

    #[test]
    fn non_finite_values_fall_back() {
        let calibration = DebugCalibration::with_defaults();
        calibration.store_auto_learn_s(f32::NAN);
        assert_eq!(calibration.auto_learn_s(), DEFAULT_AUTO_LEARN_S);
        calibration.store_trim_fair(f32::INFINITY);
        assert_eq!(calibration.trim_fair(), 0.0);
    }

    #[test]
    fn defaults_are_sane() {
        let calibration = DebugCalibration::with_defaults();
        assert_eq!(calibration.peak_hold_ms(), DEFAULT_PEAK_HOLD_MS);
        assert_eq!(calibration.auto_learn_s(), DEFAULT_AUTO_LEARN_S);
        assert_eq!(calibration.trim_fair(), 0.0);
        assert_eq!(calibration.trim_vmu(), 0.0);
    }

    #[test]
    fn parses_the_on_disk_format() {
        let calibration = DebugCalibration::with_defaults();
        calibration.apply_text(
            "\u{feff}# Masteraid debug calibration.\n\
             peak_hold_ms=2400\n\
             auto_learn_s=8.5\n\
             trim_fair=-3.5\n\
             trim_vmu=2\n",
        );
        assert_eq!(calibration.peak_hold_ms(), 2400);
        assert_eq!(calibration.auto_learn_s(), 8.5);
        assert_eq!(calibration.trim_fair(), -3.5);
        assert_eq!(calibration.trim_vmu(), 2.0);
    }

    #[test]
    fn ignores_malformed_lines_and_keeps_defaults() {
        let calibration = DebugCalibration::with_defaults();
        calibration.apply_text(
            "peak_hold_ms=not-a-number\n\
             auto_learn_s\n\
             trim_fair=NaN\n\
             trim_vmu=1e9\n\
             unknown_key=4\n",
        );
        assert_eq!(calibration.peak_hold_ms(), DEFAULT_PEAK_HOLD_MS);
        assert_eq!(calibration.auto_learn_s(), DEFAULT_AUTO_LEARN_S);
        assert_eq!(calibration.trim_fair(), 0.0);
        // Parsed and then clamped to the documented limit.
        assert_eq!(calibration.trim_vmu(), TRIM_LIMIT_DB);
    }

    /// The whole point of the store: a value written by one instance must come back in a
    /// *different* instance, exactly as if the plugin had been removed and re-inserted. This
    /// exercises the real file, not just the parser.
    #[test]
    fn values_survive_a_remove_and_reinsert_cycle() {
        let dir = std::env::temp_dir().join(format!(
            "masteraid-calibration-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let path = dir.join("debug-calibration.txt");

        // First instance: the user calibrates.
        let first = DebugCalibration::with_defaults();
        first.store_peak_hold_ms(2700);
        first.store_auto_learn_s(7.5);
        first.store_trim_fair(-2.5);
        first.store_trim_vmu(3.5);
        first.save_to(&path);
        assert!(path.is_file(), "save_to must create the file");

        // Second instance: the plugin was deleted and inserted again, so it loads from disk.
        let second = DebugCalibration::load_from(&path);
        assert_eq!(second.peak_hold_ms(), 2700);
        assert_eq!(second.auto_learn_s(), 7.5);
        assert_eq!(second.trim_fair(), -2.5);
        assert_eq!(second.trim_vmu(), 3.5);

        // Saving again over an existing file must replace it in place (Windows rename).
        second.store_trim_fair(1.0);
        second.save_to(&path);
        let third = DebugCalibration::load_from(&path);
        assert_eq!(third.trim_fair(), 1.0);
        assert_eq!(third.peak_hold_ms(), 2700);

        // A missing file is not an error; it just means "defaults".
        let missing = DebugCalibration::load_from(&dir.join("nope.txt"));
        assert_eq!(missing.peak_hold_ms(), DEFAULT_PEAK_HOLD_MS);

        // Opening an editor reloads from disk and overwrites whatever this process had cached.
        second.store_peak_hold_ms(9000);
        second.reload_from(&path);
        assert_eq!(second.peak_hold_ms(), 2700);
        assert_eq!(second.trim_fair(), 1.0);

        let _ = fs::remove_dir_all(&dir);
    }
}
