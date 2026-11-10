//! Starting Mimi when you sign in to the Mac, through a per-user launch agent.

use std::path::{Path, PathBuf};

const LABEL: &str = "com.mimi.dictation";
/// Starts Mimi in the menu bar without opening her window.
pub const HIDDEN_ARGUMENT: &str = "--hidden";

fn escaped(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

fn agent_for(exe: &Path) -> String {
    let exe = escaped(&exe.display().to_string());
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>{LABEL}</string>
  <key>ProgramArguments</key>
  <array>
    <string>{exe}</string>
    <string>{HIDDEN_ARGUMENT}</string>
  </array>
  <key>RunAtLoad</key>
  <true/>
  <key>LimitLoadToSessionType</key>
  <string>Aqua</string>
</dict>
</plist>
"#
    )
}

fn agent_file() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join("Library/LaunchAgents").join(format!("{LABEL}.plist")))
}

/// Only a copy in Applications adds itself to startup, so a test build can never take the
/// place of the real Mimi.
fn installed_exe() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    exe.starts_with("/Applications").then_some(exe)
}

fn apply_at(file: &Path, enabled: bool, exe: Option<&Path>) -> Result<(), String> {
    let refused = |error: std::io::Error| format!("macOS refused to change Mimi's startup setting ({error}).");
    match (enabled, exe) {
        (true, Some(exe)) => {
            let agent = agent_for(exe);
            // Rewritten only when different, which also follows Mimi if she's moved.
            if std::fs::read_to_string(file).ok().as_deref() == Some(agent.as_str()) {
                return Ok(());
            }
            if let Some(folder) = file.parent() {
                std::fs::create_dir_all(folder).map_err(refused)?;
            }
            std::fs::write(file, agent).map_err(refused)
        }
        (true, None) => Ok(()),
        (false, _) => match std::fs::remove_file(file) {
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => Err(refused(error)),
            _ => Ok(()),
        },
    }
}

/// Makes the Mac's startup list match the setting.
pub fn apply(enabled: bool) -> Result<(), String> {
    let Some(file) = agent_file() else { return Ok(()) };
    apply_at(&file, enabled, installed_exe().as_deref())
}

/// True when the Mac started Mimi at sign-in, so her window should stay closed.
pub fn started_hidden() -> bool {
    std::env::args().skip(1).any(|argument| argument == HIDDEN_ARGUMENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    // A folder of its own, so the tests never touch the real startup list.
    fn test_file(name: &str) -> PathBuf {
        let folder = std::env::temp_dir().join(format!("mimi-autostart-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        folder.join("LaunchAgents").join("agent.plist")
    }

    #[test]
    fn turning_on_adds_the_hidden_command_and_turning_off_removes_it() {
        let file = test_file("round-trip");
        let exe = Path::new("/Applications/Mimi.app/Contents/MacOS/mimi");
        apply_at(&file, true, Some(exe)).unwrap();
        let written = std::fs::read_to_string(&file).unwrap();
        assert!(written.contains("<string>/Applications/Mimi.app/Contents/MacOS/mimi</string>"));
        assert!(written.contains("<string>--hidden</string>"));
        assert!(written.contains("<key>RunAtLoad</key>"));

        let moved = Path::new("/Applications/Tools/Mimi.app/Contents/MacOS/mimi");
        apply_at(&file, true, Some(moved)).unwrap();
        assert!(std::fs::read_to_string(&file).unwrap().contains("/Applications/Tools/"));

        apply_at(&file, false, Some(moved)).unwrap();
        assert!(!file.exists());
        apply_at(&file, false, None).unwrap();
        std::fs::remove_dir_all(file.parent().unwrap().parent().unwrap()).unwrap();
    }

    #[test]
    fn a_copy_that_isnt_installed_adds_nothing() {
        let file = test_file("not-installed");
        apply_at(&file, true, None).unwrap();
        assert!(!file.exists());
    }

    #[test]
    fn special_characters_in_the_path_stay_valid() {
        let agent = agent_for(Path::new("/Applications/A & B/Mimi.app/Contents/MacOS/mimi"));
        assert!(agent.contains("/Applications/A &amp; B/Mimi.app"));
    }
}
