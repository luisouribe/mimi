//! Puts dictated text into whichever app is active, and puts the clipboard back afterwards.
//!
//! Pasting is the fastest reliable way to insert text. Mimi saves what was on the clipboard,
//! pastes, then restores it. The dictated text is marked so it doesn't fill Windows'
//! clipboard history or sync to other devices.

use serde::Serialize;
use std::path::Path;
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};
use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, GlobalFree, HANDLE, HWND};
use windows_sys::Win32::Security::{GetTokenInformation, TOKEN_ELEVATION, TOKEN_QUERY, TokenElevation};
use windows_sys::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, EnumClipboardFormats, GetClipboardData, GetClipboardSequenceNumber, OpenClipboard,
    RegisterClipboardFormatW, SetClipboardData,
};
use windows_sys::Win32::System::Memory::{GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock};
use windows_sys::Win32::System::Threading::{
    GetCurrentProcessId, OpenProcess, OpenProcessToken, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
    QueryFullProcessImageNameW,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, INPUT, INPUT_KEYBOARD, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP, SendInput, VIRTUAL_KEY, VK_CONTROL,
    VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

const CF_UNICODETEXT: u32 = 13;
const VK_V: VIRTUAL_KEY = 0x56;
/// Time for the target app to read the clipboard before it's restored.
const RESTORE_DELAY: Duration = Duration::from_millis(400);
const MODIFIER_WAIT: Duration = Duration::from_millis(1_000);
/// A clipboard larger than this isn't copied in memory. It would slow every dictation.
const MAX_SNAPSHOT_BYTES: usize = 256 * 1024 * 1024;

/// Only one paste at a time, so a restore never overwrites the next dictation.
static PASTE_LOCK: Mutex<()> = Mutex::new(());

/// The app that was active when dictation stopped.
#[derive(Debug, Clone, Copy)]
pub struct Target {
    window: HWND,
    process_id: u32,
}

// A window handle is just a number here, and it's only passed back to Windows.
unsafe impl Send for Target {}

impl Target {
    pub fn current() -> Self {
        unsafe {
            let window = GetForegroundWindow();
            let mut process_id = 0u32;
            if !window.is_null() {
                GetWindowThreadProcessId(window, &mut process_id);
            }
            Self { window, process_id }
        }
    }

    pub fn is_mimi(&self) -> bool {
        self.process_id == unsafe { GetCurrentProcessId() }
    }

    pub fn still_active(&self) -> bool {
        unsafe { GetForegroundWindow() == self.window }
    }

    /// A short app name for the history. The window title is never stored.
    pub fn app_name(&self) -> String {
        if self.is_mimi() {
            return "Mimi".to_string();
        }
        process_path(self.process_id)
            .map(|path| friendly_app_name(&path))
            .unwrap_or_else(|| "Unknown app".to_string())
    }

    /// Windows blocks normal apps from typing into windows running as administrator.
    fn runs_as_administrator(&self) -> bool {
        if self.process_id == 0 {
            return false;
        }
        unsafe {
            let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, self.process_id);
            if process.is_null() {
                return true;
            }
            let mut token: HANDLE = std::ptr::null_mut();
            let opened = OpenProcessToken(process, TOKEN_QUERY, &mut token) != 0;
            let mut elevated = !opened;
            if opened {
                let mut info = TOKEN_ELEVATION { TokenIsElevated: 0 };
                let mut size = 0u32;
                if GetTokenInformation(
                    token,
                    TokenElevation,
                    &mut info as *mut _ as *mut _,
                    std::mem::size_of::<TOKEN_ELEVATION>() as u32,
                    &mut size,
                ) != 0
                {
                    elevated = info.TokenIsElevated != 0;
                }
                CloseHandle(token);
            }
            CloseHandle(process);
            elevated
        }
    }
}

fn process_path(process_id: u32) -> Option<String> {
    unsafe {
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, process_id);
        if process.is_null() {
            return None;
        }
        let mut buffer = vec![0u16; 1024];
        let mut length = buffer.len() as u32;
        let ok = QueryFullProcessImageNameW(process, PROCESS_NAME_WIN32, buffer.as_mut_ptr(), &mut length) != 0;
        CloseHandle(process);
        ok.then(|| String::from_utf16_lossy(&buffer[..length as usize]))
    }
}

/// Turns a program's file path into the name people know it by.
pub fn friendly_app_name(path: &str) -> String {
    let file = path.rsplit(['\\', '/']).next().unwrap_or(path);
    let stem = Path::new(file)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or(file)
        .to_lowercase();
    const KNOWN: &[(&str, &str)] = &[
        ("whatsapp", "WhatsApp"),
        ("chrome", "Chrome"),
        ("msedge", "Edge"),
        ("firefox", "Firefox"),
        ("brave", "Brave"),
        ("opera", "Opera"),
        ("winword", "Word"),
        ("excel", "Excel"),
        ("powerpnt", "PowerPoint"),
        ("outlook", "Outlook"),
        ("olk", "Outlook"),
        ("notepad", "Notepad"),
        ("chatgpt", "ChatGPT"),
        ("claude", "Claude"),
        ("code", "VS Code"),
        ("windowsterminal", "Terminal"),
        ("cmd", "Command Prompt"),
        ("powershell", "PowerShell"),
        ("explorer", "File Explorer"),
        ("slack", "Slack"),
        ("discord", "Discord"),
        ("telegram", "Telegram"),
        ("ms-teams", "Teams"),
        ("teams", "Teams"),
        ("photoshop", "Photoshop"),
        ("premiere", "Premiere Pro"),
        ("afterfx", "After Effects"),
    ];
    if let Some((_, name)) = KNOWN.iter().find(|(key, _)| stem == *key || stem.starts_with(&format!("{key}."))) {
        return name.to_string();
    }
    let mut letters = stem.chars();
    match letters.next() {
        Some(first) => first.to_uppercase().chain(letters).collect(),
        None => "Unknown app".to_string(),
    }
}

/// What happened to the dictated text.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PasteOutcome {
    Pasted,
    /// The text couldn't be pasted safely, so it's on the clipboard, ready for Ctrl + V.
    CopiedInstead { reason: String },
    /// The text was dictated inside Mimi's own window, so it only goes to the history.
    StayedInMimi,
}

struct OpenedClipboard;

impl OpenedClipboard {
    /// Another app may be using the clipboard for a moment, so this retries briefly.
    fn open() -> Result<Self, String> {
        for attempt in 0..25 {
            if unsafe { OpenClipboard(std::ptr::null_mut()) } != 0 {
                return Ok(Self);
            }
            thread::sleep(Duration::from_millis(5 + attempt));
        }
        Err("The clipboard is busy.".to_string())
    }
}

impl Drop for OpenedClipboard {
    fn drop(&mut self) {
        unsafe {
            CloseClipboard();
        }
    }
}

/// Formats that hold handles to images or other objects rather than plain memory.
/// Windows rebuilds the common image ones from the bitmap data that is copied.
fn is_plain_memory_format(format: u32) -> bool {
    !matches!(format, 2 | 3 | 9 | 14 | 0x80..=0x83 | 0x8E | 0x200..=0x3FF)
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

fn register_format(name: &str) -> u32 {
    unsafe { RegisterClipboardFormatW(wide(name).as_ptr()) }
}

/// Everything on the clipboard, as copies of its bytes.
fn snapshot_clipboard() -> Result<Vec<(u32, Vec<u8>)>, String> {
    let _clipboard = OpenedClipboard::open()?;
    let mut saved = Vec::new();
    let mut total = 0usize;
    unsafe {
        let mut format = EnumClipboardFormats(0);
        while format != 0 {
            if is_plain_memory_format(format) {
                let handle = GetClipboardData(format);
                if !handle.is_null() {
                    let size = GlobalSize(handle);
                    if size > 0 && total + size <= MAX_SNAPSHOT_BYTES {
                        let source = GlobalLock(handle);
                        if !source.is_null() {
                            saved.push((format, std::slice::from_raw_parts(source as *const u8, size).to_vec()));
                            total += size;
                            GlobalUnlock(handle);
                        }
                    }
                }
            }
            format = EnumClipboardFormats(format);
        }
    }
    Ok(saved)
}

unsafe fn set_clipboard_bytes(format: u32, bytes: &[u8]) -> bool {
    unsafe {
        let memory = GlobalAlloc(GMEM_MOVEABLE, bytes.len().max(1));
        if memory.is_null() {
            return false;
        }
        let target = GlobalLock(memory);
        if target.is_null() {
            GlobalFree(memory);
            return false;
        }
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), target as *mut u8, bytes.len());
        GlobalUnlock(memory);
        if SetClipboardData(format, memory).is_null() {
            GlobalFree(memory);
            return false;
        }
        true
    }
}

/// Asks clipboard-history tools and cloud sync to skip what's on the clipboard now.
unsafe fn mark_private() {
    unsafe {
        set_clipboard_bytes(register_format("ExcludeClipboardContentFromMonitorProcessing"), &[0; 4]);
        set_clipboard_bytes(register_format("CanIncludeInClipboardHistory"), &0u32.to_le_bytes());
        set_clipboard_bytes(register_format("CanUploadToCloudClipboard"), &0u32.to_le_bytes());
    }
}

/// Windows programs expect both line break characters on the clipboard. Older ones ignore a lone one.
fn windows_line_breaks(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\n', "\r\n")
}

#[cfg(test)]
mod line_break_tests {
    use super::windows_line_breaks;

    #[test]
    fn line_breaks_use_the_windows_form_once() {
        assert_eq!(windows_line_breaks("uno\ndos\n\ntres"), "uno\r\ndos\r\n\r\ntres");
        assert_eq!(windows_line_breaks("uno\r\ndos"), "uno\r\ndos");
        assert_eq!(windows_line_breaks("sin saltos"), "sin saltos");
    }
}

fn write_text(text: &str, private: bool) -> Result<(), String> {
    let _clipboard = OpenedClipboard::open()?;
    let bytes: Vec<u8> = wide(&windows_line_breaks(text)).iter().flat_map(|unit| unit.to_le_bytes()).collect();
    unsafe {
        if EmptyClipboard() == 0 {
            return Err("The clipboard couldn't be cleared.".to_string());
        }
        if !set_clipboard_bytes(CF_UNICODETEXT, &bytes) {
            return Err("The text couldn't be placed on the clipboard.".to_string());
        }
        if private {
            mark_private();
        }
    }
    Ok(())
}

fn restore_clipboard(saved: &[(u32, Vec<u8>)]) -> Result<(), String> {
    let _clipboard = OpenedClipboard::open()?;
    unsafe {
        if EmptyClipboard() == 0 {
            return Err("The clipboard couldn't be restored.".to_string());
        }
        for (format, bytes) in saved {
            set_clipboard_bytes(*format, bytes);
        }
        // It was already in the history when first copied, so it shouldn't appear twice.
        mark_private();
    }
    Ok(())
}

/// Puts text on the clipboard for the Copy buttons. This behaves like a normal copy.
pub fn copy_to_clipboard(text: &str) -> Result<(), String> {
    let _guard = PASTE_LOCK.lock().map_err(|_| "The clipboard is locked.".to_string())?;
    write_text(text, false)
}

fn key_is_down(key: VIRTUAL_KEY) -> bool {
    unsafe { GetAsyncKeyState(key as i32) < 0 }
}

fn keyboard_input(key: VIRTUAL_KEY, flags: KEYBD_EVENT_FLAGS) -> INPUT {
    let mut input: INPUT = unsafe { std::mem::zeroed() };
    input.r#type = INPUT_KEYBOARD;
    input.Anonymous.ki.wVk = key;
    input.Anonymous.ki.dwFlags = flags;
    input
}

/// Sends Ctrl + V. Keys still held from the shortcut are given a moment to come up first,
/// because Shift or Alt held down would turn this into a different command.
fn send_paste_keys() -> Result<(), String> {
    let started = Instant::now();
    while started.elapsed() < MODIFIER_WAIT
        && [VK_CONTROL, VK_SHIFT, VK_MENU, VK_LWIN, VK_RWIN].iter().any(|key| key_is_down(*key))
    {
        thread::sleep(Duration::from_millis(5));
    }
    if [VK_SHIFT, VK_MENU, VK_LWIN, VK_RWIN].iter().any(|key| key_is_down(*key)) {
        return Err("A modifier key is still held down.".to_string());
    }
    let ctrl_held = key_is_down(VK_CONTROL);
    let mut inputs = Vec::with_capacity(4);
    if !ctrl_held {
        inputs.push(keyboard_input(VK_CONTROL, 0));
    }
    inputs.push(keyboard_input(VK_V, 0));
    inputs.push(keyboard_input(VK_V, KEYEVENTF_KEYUP));
    if !ctrl_held {
        inputs.push(keyboard_input(VK_CONTROL, KEYEVENTF_KEYUP));
    }
    let sent = unsafe { SendInput(inputs.len() as u32, inputs.as_ptr(), std::mem::size_of::<INPUT>() as i32) };
    if sent as usize != inputs.len() {
        return Err(format!("Windows blocked the paste ({}).", unsafe { GetLastError() }));
    }
    Ok(())
}

fn copied_instead(text: &str, reason: &str) -> Result<PasteOutcome, String> {
    write_text(text, false)?;
    Ok(PasteOutcome::CopiedInstead { reason: reason.to_string() })
}

/// Pastes into the app that was active when dictation stopped.
/// The text is never pasted into a different app than the one the person was using.
pub fn deliver(text: &str, target: Target) -> Result<PasteOutcome, String> {
    if target.is_mimi() {
        return Ok(PasteOutcome::StayedInMimi);
    }
    let _guard = PASTE_LOCK.lock().map_err(|_| "The clipboard is locked.".to_string())?;
    if !target.still_active() {
        return copied_instead(text, "You switched apps while Mimi was typing.");
    }
    if target.runs_as_administrator() {
        return copied_instead(text, "Windows doesn't let Mimi type into this administrator window.");
    }

    let saved = snapshot_clipboard()?;
    write_text(text, true)?;
    let written = unsafe { GetClipboardSequenceNumber() };

    if let Err(reason) = send_paste_keys() {
        return copied_instead(text, &reason);
    }

    thread::sleep(RESTORE_DELAY);
    // If the clipboard changed meanwhile, someone copied something new, which must be kept.
    if unsafe { GetClipboardSequenceNumber() } == written {
        restore_clipboard(&saved)?;
    }
    Ok(PasteOutcome::Pasted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_apps_get_their_familiar_names() {
        for (path, name) in [
            (r"C:\Program Files\Google\Chrome\Application\chrome.exe", "Chrome"),
            (r"C:\Program Files\Microsoft Office\root\Office16\WINWORD.EXE", "Word"),
            (r"C:\Program Files\WindowsApps\5319275A\WhatsApp.Root.exe", "WhatsApp"),
            (r"C:\Windows\System32\notepad.exe", "Notepad"),
        ] {
            assert_eq!(friendly_app_name(path), name, "{path}");
        }
    }

    #[test]
    fn unknown_apps_use_their_file_name() {
        assert_eq!(friendly_app_name(r"C:\Tools\obsidian.exe"), "Obsidian");
        assert_eq!(friendly_app_name("thing"), "Thing");
    }

    #[test]
    fn names_that_only_start_like_a_known_app_are_not_confused() {
        assert_eq!(friendly_app_name(r"C:\Tools\codeblocks.exe"), "Codeblocks");
    }

    #[test]
    fn image_handles_are_skipped_but_text_and_bitmap_data_are_kept() {
        assert!(is_plain_memory_format(13)); // Unicode text
        assert!(is_plain_memory_format(8)); // Bitmap data
        assert!(is_plain_memory_format(0xC123)); // Registered formats, such as web content
        assert!(!is_plain_memory_format(2)); // Bitmap handle
        assert!(!is_plain_memory_format(14)); // Metafile handle
        assert!(!is_plain_memory_format(0x210)); // Private handle
    }
}
