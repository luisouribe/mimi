//! The choices Mimi remembers between launches, saved as a small file on this PC.

use crate::hotkey::Shortcut;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const FILE_NAME: &str = "settings.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum ModelChoice {
    #[default]
    Parakeet,
    WhisperTurbo,
    WhisperSmall,
}

/// Mimi's colors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum Appearance {
    #[default]
    Light,
    /// Dark grey, never black.
    Dark,
    /// Follows Windows' light or dark app mode.
    System,
}

/// Settings. Any missing value uses its default, so older files keep working.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub model: ModelChoice,
    /// A language code such as "ja". `None` means Whisper detects the language.
    /// Parakeet always detects it on its own.
    pub language: Option<String>,
    /// Set after a graphics card crash, so Whisper uses the processor instead.
    pub whisper_cpu_only: bool,
    /// Removes filler sounds and stutters, and follows spoken line break commands.
    pub cleanup: bool,
    /// Mimi moves on the home screen. Off shows her as a still image.
    pub animate_mimi: bool,
    /// Mimi starts hidden in the tray when you sign in to Windows.
    pub start_with_windows: bool,
    /// A short chime when Mimi starts and stops listening.
    pub chimes: bool,
    /// Other apps go silent while Mimi listens, then come back.
    pub quiet_other_apps: bool,
    /// Records while held.
    pub hold_shortcut: Shortcut,
    /// One press starts, the next one stops.
    pub tap_shortcut: Shortcut,
    pub appearance: Appearance,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            model: ModelChoice::default(),
            language: None,
            whisper_cpu_only: false,
            cleanup: true,
            animate_mimi: true,
            start_with_windows: true,
            chimes: true,
            quiet_other_apps: true,
            hold_shortcut: Shortcut::hold_default(),
            tap_shortcut: Shortcut::tap_default(),
            appearance: Appearance::Light,
        }
    }
}

fn path_in(folder: &Path) -> PathBuf {
    folder.join(FILE_NAME)
}

/// A missing or damaged file simply means the defaults, so it can never stop Mimi from opening.
pub fn load(folder: &Path) -> Settings {
    std::fs::read_to_string(path_in(folder))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

/// Writes to a temporary file first, so a crash mid-save can't damage the settings.
pub fn save(folder: &Path, settings: &Settings) -> Result<(), String> {
    std::fs::create_dir_all(folder).map_err(|error| error.to_string())?;
    let text = serde_json::to_string_pretty(settings).map_err(|error| error.to_string())?;
    let temporary = folder.join(format!("{FILE_NAME}.tmp"));
    std::fs::write(&temporary, text).map_err(|error| error.to_string())?;
    std::fs::rename(&temporary, path_in(folder)).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temporary_folder(name: &str) -> PathBuf {
        let folder = std::env::temp_dir().join(format!("mimi-settings-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        folder
    }

    #[test]
    fn defaults_are_parakeet_with_automatic_language() {
        let settings = load(&temporary_folder("missing"));
        assert_eq!(settings.model, ModelChoice::Parakeet);
        assert_eq!(settings.language, None);
        assert!(!settings.whisper_cpu_only);
        assert!(settings.cleanup);
        assert!(settings.animate_mimi);
        assert!(settings.start_with_windows);
        assert!(settings.chimes);
        assert!(settings.quiet_other_apps);
        assert_eq!(settings.hold_shortcut.label(), "Ctrl + Space");
        assert_eq!(settings.tap_shortcut.label(), "Ctrl + Shift + Space");
        assert_eq!(settings.appearance, Appearance::Light);
    }

    #[test]
    fn saved_settings_come_back_the_same() {
        let folder = temporary_folder("round-trip");
        let settings = Settings {
            model: ModelChoice::WhisperTurbo,
            language: Some("ja".into()),
            whisper_cpu_only: true,
            cleanup: false,
            animate_mimi: false,
            start_with_windows: false,
            chimes: false,
            quiet_other_apps: false,
            hold_shortcut: Shortcut { ctrl: false, alt: true, shift: false, key: 0x44, key_name: "D".into() },
            tap_shortcut: Shortcut { ctrl: false, alt: false, shift: false, key: 0x77, key_name: "F8".into() },
            appearance: Appearance::Dark,
        };
        save(&folder, &settings).unwrap();
        assert_eq!(load(&folder), settings);
        std::fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn a_damaged_file_falls_back_to_defaults() {
        let folder = temporary_folder("damaged");
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(path_in(&folder), "{ this is not valid").unwrap();
        assert_eq!(load(&folder), Settings::default());
        std::fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn a_file_with_missing_or_unknown_values_still_loads() {
        let folder = temporary_folder("partial");
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(path_in(&folder), r#"{"language":"es","future_option":true}"#).unwrap();
        let settings = load(&folder);
        assert_eq!(settings.language.as_deref(), Some("es"));
        assert_eq!(settings.model, ModelChoice::Parakeet);
        assert!(settings.cleanup, "a file saved before this option existed keeps cleanup on");
        std::fs::remove_dir_all(&folder).unwrap();
    }
}
