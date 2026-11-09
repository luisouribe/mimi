//! Starting Mimi with Windows, through the same per-user "Run" list that Task Manager's
//! Startup apps page shows. The uninstaller removes the entry, because it uses the same name.

use std::path::Path;
use windows_sys::Win32::Foundation::ERROR_SUCCESS;
use windows_sys::Win32::System::Registry::{
    HKEY_CURRENT_USER, REG_SZ, RRF_RT_REG_SZ, RegDeleteKeyValueW, RegGetValueW, RegSetKeyValueW,
};

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
/// Must match the installer's product name, so uninstalling also removes the entry.
const ENTRY_NAME: &str = "Mimi";
/// Starts Mimi in the tray without opening her window.
pub const HIDDEN_ARGUMENT: &str = "--hidden";

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

fn read(key: &str, name: &str) -> Option<String> {
    let (key, name) = (wide(key), wide(name));
    let mut buffer = vec![0u16; 2048];
    let mut size = (buffer.len() * 2) as u32;
    let status = unsafe {
        RegGetValueW(HKEY_CURRENT_USER, key.as_ptr(), name.as_ptr(), RRF_RT_REG_SZ, std::ptr::null_mut(), buffer.as_mut_ptr().cast(), &mut size)
    };
    (status == ERROR_SUCCESS).then(|| String::from_utf16_lossy(&buffer[..(size as usize / 2).saturating_sub(1)]))
}

fn write(key: &str, name: &str, value: &str) -> Result<(), String> {
    let (key, name, value) = (wide(key), wide(name), wide(value));
    let status = unsafe {
        RegSetKeyValueW(HKEY_CURRENT_USER, key.as_ptr(), name.as_ptr(), REG_SZ, value.as_ptr().cast(), (value.len() * 2) as u32)
    };
    if status == ERROR_SUCCESS { Ok(()) } else { Err(format!("Windows refused to add Mimi to startup (error {status}).")) }
}

fn remove(key: &str, name: &str) -> Result<(), String> {
    if read(key, name).is_none() {
        return Ok(());
    }
    let (key, name) = (wide(key), wide(name));
    let status = unsafe { RegDeleteKeyValueW(HKEY_CURRENT_USER, key.as_ptr(), name.as_ptr()) };
    if status == ERROR_SUCCESS { Ok(()) } else { Err(format!("Windows refused to remove Mimi from startup (error {status}).")) }
}

fn command_for(exe: &Path) -> String {
    format!("\"{}\" {HIDDEN_ARGUMENT}", exe.display())
}

/// Only an installed copy, with its uninstaller beside it, adds itself to startup,
/// so a test build can never take the place of the real Mimi.
fn installed_exe() -> Option<std::path::PathBuf> {
    let exe = std::env::current_exe().ok()?;
    exe.parent()?.join("uninstall.exe").is_file().then_some(exe)
}

fn apply_at(key: &str, name: &str, enabled: bool, exe: Option<&Path>) -> Result<(), String> {
    match (enabled, exe) {
        (true, Some(exe)) => {
            let command = command_for(exe);
            // Rewritten only when different, which also follows Mimi if she's reinstalled elsewhere.
            if read(key, name).as_deref() != Some(command.as_str()) { write(key, name, &command) } else { Ok(()) }
        }
        (true, None) => Ok(()),
        (false, _) => remove(key, name),
    }
}

/// Makes Windows' startup list match the setting.
pub fn apply(enabled: bool) -> Result<(), String> {
    apply_at(RUN_KEY, ENTRY_NAME, enabled, installed_exe().as_deref())
}

/// True when Windows started Mimi at sign-in, so her window should stay closed.
pub fn started_hidden() -> bool {
    std::env::args().skip(1).any(|argument| argument == HIDDEN_ARGUMENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    use windows_sys::Win32::System::Registry::RegDeleteKeyW;

    // A key of its own, so the tests never touch the real startup list.
    const TEST_KEY: &str = r"Software\MimiTests";

    #[test]
    fn turning_on_adds_the_hidden_command_and_turning_off_removes_it() {
        let name = format!("startup-{}", std::process::id());
        let exe = Path::new(r"C:\Users\Someone\AppData\Local\Mimi\mimi.exe");
        apply_at(TEST_KEY, &name, true, Some(exe)).unwrap();
        assert_eq!(read(TEST_KEY, &name).as_deref(), Some(r#""C:\Users\Someone\AppData\Local\Mimi\mimi.exe" --hidden"#));
        apply_at(TEST_KEY, &name, true, Some(exe)).unwrap();
        let moved = Path::new(r"D:\Apps\Mimi\mimi.exe");
        apply_at(TEST_KEY, &name, true, Some(moved)).unwrap();
        assert_eq!(read(TEST_KEY, &name).as_deref(), Some(r#""D:\Apps\Mimi\mimi.exe" --hidden"#));
        apply_at(TEST_KEY, &name, false, Some(moved)).unwrap();
        assert_eq!(read(TEST_KEY, &name), None);
        apply_at(TEST_KEY, &name, false, None).unwrap();
        let key = wide(TEST_KEY);
        unsafe { RegDeleteKeyW(HKEY_CURRENT_USER, key.as_ptr()) };
    }

    #[test]
    fn a_copy_that_isnt_installed_adds_nothing() {
        let name = format!("not-installed-{}", std::process::id());
        apply_at(TEST_KEY, &name, true, None).unwrap();
        assert_eq!(read(TEST_KEY, &name), None);
    }
}
