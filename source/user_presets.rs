//! User preset storage and on-disk persistence for Masteraid.

use crate::parameters::{CompModel, EqModel};
use std::fs;
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq)]
pub struct UserPreset {
    pub name: String,
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

impl Default for UserPreset {
    fn default() -> Self {
        Self {
            name: "DEFAULT MASTER".to_string(),
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
        }
    }
}

impl UserPreset {
    pub fn to_serialized(&self) -> String {
        format!(
            "PRESET:{}\n\
             tape_speed_30ips={}\n\
             tape_formula_gp9={}\n\
             tape_drive={:.3}\n\
             tape_bias={:.3}\n\
             tape_bump={:.3}\n\
             tape_bump_freq={:.1}\n\
             tape_flutter={:.3}\n\
             clipper_drive={:.3}\n\
             clipper_ceiling={:.3}\n\
             clipper_hard_shape={}\n\
             clipper_os_4x={}\n\
             eq_model={}\n\
             comp_model={}\n\
             comp_thresh={:.3}\n\
             comp_ratio={:.3}\n\
             comp_attack={:.3}\n\
             comp_release={:.3}\n\
             comp_mix={:.3}\n\
             limiter_gain={:.3}\n\
             limiter_ceiling={:.3}\n\
             limiter_lookahead={:.3}\n\
             limiter_release={:.3}\n\
             limiter_lift={:.3}\n\
             limiter_true_peak={}\n\
             limiter_punch={}\n\
             END_PRESET",
            self.name,
            if self.tape_speed_30ips { 1 } else { 0 },
            if self.tape_formula_gp9 { 1 } else { 0 },
            self.tape_drive,
            self.tape_bias,
            self.tape_bump,
            self.tape_bump_freq,
            self.tape_flutter,
            self.clipper_drive,
            self.clipper_ceiling,
            if self.clipper_hard_shape { 1 } else { 0 },
            if self.clipper_os_4x { 1 } else { 0 },
            match self.eq_model {
                EqModel::Ptek => "PTEK",
                EqModel::Mass => "MASS",
            },
            match self.comp_model {
                CompModel::Fair => "FAIR",
                CompModel::VMu => "VMU",
            },
            self.comp_thresh,
            self.comp_ratio,
            self.comp_attack,
            self.comp_release,
            self.comp_mix,
            self.limiter_gain,
            self.limiter_ceiling,
            self.limiter_lookahead,
            self.limiter_release,
            self.limiter_lift,
            if self.limiter_true_peak { 1 } else { 0 },
            if self.limiter_punch { 1 } else { 0 },
        )
    }
}

pub fn parse_presets(text: &str) -> Vec<UserPreset> {
    let mut list = Vec::new();
    let mut cur: Option<UserPreset> = None;

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some(name) = line.strip_prefix("PRESET:") {
            let mut p = UserPreset::default();
            p.name = name.trim().to_string();
            cur = Some(p);
            continue;
        }

        if line == "END_PRESET" {
            if let Some(p) = cur.take() {
                list.push(p);
            }
            continue;
        }

        let Some(p) = cur.as_mut() else {
            continue;
        };

        if let Some((k, v)) = line.split_once('=') {
            let k = k.trim();
            let v = v.trim();
            match k {
                "tape_speed_30ips" => p.tape_speed_30ips = v == "1" || v.eq_ignore_ascii_case("true"),
                "tape_formula_gp9" => p.tape_formula_gp9 = v == "1" || v.eq_ignore_ascii_case("true"),
                "tape_drive" => if let Ok(val) = v.parse() { p.tape_drive = val; },
                "tape_bias" => if let Ok(val) = v.parse() { p.tape_bias = val; },
                "tape_bump" => if let Ok(val) = v.parse() { p.tape_bump = val; },
                "tape_bump_freq" => if let Ok(val) = v.parse() { p.tape_bump_freq = val; },
                "tape_flutter" => if let Ok(val) = v.parse() { p.tape_flutter = val; },
                "clipper_drive" => if let Ok(val) = v.parse() { p.clipper_drive = val; },
                "clipper_ceiling" => if let Ok(val) = v.parse() { p.clipper_ceiling = val; },
                "clipper_hard_shape" => p.clipper_hard_shape = v == "1" || v.eq_ignore_ascii_case("true"),
                "clipper_os_4x" => p.clipper_os_4x = v == "1" || v.eq_ignore_ascii_case("true"),
                "eq_model" => p.eq_model = if v.eq_ignore_ascii_case("MASS") { EqModel::Mass } else { EqModel::Ptek },
                "comp_model" => p.comp_model = if v.eq_ignore_ascii_case("VMU") { CompModel::VMu } else { CompModel::Fair },
                "comp_thresh" => if let Ok(val) = v.parse() { p.comp_thresh = val; },
                "comp_ratio" => if let Ok(val) = v.parse() { p.comp_ratio = val; },
                "comp_attack" => if let Ok(val) = v.parse() { p.comp_attack = val; },
                "comp_release" => if let Ok(val) = v.parse() { p.comp_release = val; },
                "comp_mix" => if let Ok(val) = v.parse() { p.comp_mix = val; },
                "limiter_gain" => if let Ok(val) = v.parse() { p.limiter_gain = val; },
                "limiter_ceiling" => if let Ok(val) = v.parse() { p.limiter_ceiling = val; },
                "limiter_lookahead" => if let Ok(val) = v.parse() { p.limiter_lookahead = val; },
                "limiter_release" => if let Ok(val) = v.parse() { p.limiter_release = val; },
                "limiter_lift" => if let Ok(val) = v.parse() { p.limiter_lift = val; },
                "limiter_true_peak" => p.limiter_true_peak = v == "1" || v.eq_ignore_ascii_case("true"),
                "limiter_punch" => p.limiter_punch = v == "1" || v.eq_ignore_ascii_case("true"),
                _ => {}
            }
        }
    }

    if let Some(p) = cur {
        list.push(p);
    }

    list
}

pub fn sanitize_filename(name: &str) -> String {
    let s: String = name
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect();
    let trimmed = s.trim();
    if trimmed.is_empty() {
        "Preset".to_string()
    } else {
        trimmed.to_string()
    }
}

pub fn presets_dir_path() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        if let Some(userprofile) = std::env::var_os("USERPROFILE") {
            let docs = PathBuf::from(userprofile).join("Documents");
            return Some(docs.join("MQNdsp").join("Masteraid").join("Presets"));
        }
        std::env::var_os("APPDATA")
            .map(|appdata| PathBuf::from(appdata).join("MQNdsp").join("Masteraid").join("Presets"))
    }

    #[cfg(target_os = "macos")]
    {
        std::env::var_os("HOME")
            .map(|home| PathBuf::from(home).join("Documents").join("MQNdsp").join("Masteraid").join("Presets"))
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        std::env::var_os("XDG_DOCUMENTS_DIR")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join("Documents")))
            .map(|docs| docs.join("MQNdsp").join("Masteraid").join("Presets"))
    }
}

fn legacy_presets_file_path() -> Option<PathBuf> {
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
            .join("user-presets.txt")
    })
}

pub fn open_presets_folder() {
    let Some(dir) = presets_dir_path() else { return; };
    let _ = fs::create_dir_all(&dir);

    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("explorer")
            .arg(&dir)
            .spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open")
            .arg(&dir)
            .spawn();
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        let _ = std::process::Command::new("xdg-open")
            .arg(&dir)
            .spawn();
    }
}

pub fn delete_user_preset_file(name: &str) {
    let Some(dir) = presets_dir_path() else { return; };
    let file_path = dir.join(format!("{}.mqnpreset", sanitize_filename(name)));
    let _ = fs::remove_file(file_path);
}

pub fn load_user_presets() -> Vec<UserPreset> {
    let mut presets = Vec::new();
    let mut names_seen = std::collections::HashSet::new();

    let Some(dir) = presets_dir_path() else {
        return presets;
    };
    let _ = fs::create_dir_all(&dir);

    // 1. Scan directory for all *.mqnpreset files
    if let Ok(entries) = fs::read_dir(&dir) {
        let mut entries: Vec<_> = entries.filter_map(|e| e.ok()).collect();
        entries.sort_by_key(|e| e.file_name());
        for entry in entries {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "mqnpreset" {
                        if let Ok(content) = fs::read_to_string(&path) {
                            for p in parse_presets(&content) {
                                if names_seen.insert(p.name.to_lowercase()) {
                                    presets.push(p);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // 2. Also check user-presets.txt in that directory (for consolidated backup/fallback)
    let consolidated_path = dir.join("user-presets.txt");
    if consolidated_path.is_file() {
        if let Ok(content) = fs::read_to_string(&consolidated_path) {
            for p in parse_presets(&content) {
                if names_seen.insert(p.name.to_lowercase()) {
                    presets.push(p);
                }
            }
        }
    }

    // 3. Migration check: if no presets found, check old legacy location
    if presets.is_empty() {
        if let Some(legacy_path) = legacy_presets_file_path() {
            if legacy_path.is_file() {
                if let Ok(content) = fs::read_to_string(&legacy_path) {
                    let migrated = parse_presets(&content);
                    if !migrated.is_empty() {
                        save_user_presets(&migrated);
                        return migrated;
                    }
                }
            }
        }
    }

    presets
}

pub fn save_user_presets(presets: &[UserPreset]) {
    let Some(dir) = presets_dir_path() else {
        return;
    };
    let _ = fs::create_dir_all(&dir);

    // 1. Save each preset to an individual .mqnpreset file for easy sharing
    for p in presets {
        let file_path = dir.join(format!("{}.mqnpreset", sanitize_filename(&p.name)));
        let _ = fs::write(file_path, p.to_serialized());
    }

    // 2. Also maintain the consolidated user-presets.txt file
    let path = dir.join("user-presets.txt");
    let mut text = String::from("# Masteraid User Presets\n\n");
    for p in presets {
        text.push_str(&p.to_serialized());
        text.push_str("\n\n");
    }
    let tmp = path.with_extension("tmp");
    if fs::write(&tmp, text).is_ok() {
        let _ = fs::rename(&tmp, &path);
    }
}

pub fn open_manual_pdf() {
    let base_dir = presets_dir_path()
        .and_then(|d| d.parent().map(|p| p.to_path_buf()))
        .or_else(presets_dir_path);
    let Some(dir) = base_dir else { return; };
    let _ = fs::create_dir_all(&dir);
    let pdf_path = dir.join("Masteraid_User_Manual.pdf");
    const PDF_BYTES: &[u8] = include_bytes!("../assets/Masteraid_User_Manual.pdf");
    let _ = fs::write(&pdf_path, PDF_BYTES);

    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("cmd")
            .args(["/c", "start", "", &pdf_path.to_string_lossy()])
            .spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open")
            .arg(&pdf_path)
            .spawn();
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        let _ = std::process::Command::new("xdg-open")
            .arg(&pdf_path)
            .spawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_preset_round_trip() {
        let p = UserPreset {
            name: "MY CUSTOM MASTER".to_string(),
            tape_speed_30ips: false,
            tape_formula_gp9: true,
            tape_drive: 2.3,
            tape_bias: -0.4,
            tape_bump: 1.5,
            tape_bump_freq: 70.0,
            tape_flutter: 0.2,
            clipper_drive: 1.2,
            clipper_ceiling: -0.25,
            clipper_hard_shape: true,
            clipper_os_4x: false,
            eq_model: EqModel::Mass,
            comp_model: CompModel::VMu,
            comp_thresh: 3.5,
            comp_ratio: 3.0,
            comp_attack: 15.0,
            comp_release: 250.0,
            comp_mix: 0.8,
            limiter_gain: 4.2,
            limiter_ceiling: -0.3,
            limiter_lookahead: 0.5,
            limiter_release: 120.0,
            limiter_lift: 0.15,
            limiter_true_peak: true,
            limiter_punch: false,
        };

        let serialized = p.to_serialized();
        let parsed = parse_presets(&serialized);
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0], p);
    }
}
