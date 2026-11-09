//! Mimi's two global shortcuts, which work in every app, Mimi's own window included.
//!
//! - Hold to talk (Ctrl + Space unless changed): records while the keys are held.
//! - Tap to start and stop (Ctrl + Shift + Space unless changed): one press starts, the next stops.
//!
//! This needs a low-level keyboard hook, because ordinary Windows hotkeys can't report when a
//! key is released. The hook only looks for these two combinations, or for the new keys while
//! you change a shortcut in Settings. It never stores or sends keystrokes anywhere, and every
//! other key passes straight through.

use serde::{Deserialize, Serialize};
use std::sync::mpsc::Sender;
use std::thread;
use std::time::{Duration, Instant};

/// Windows repeats a held key, Ctrl and Shift included, at least every second. A modifier not
/// heard from for longer than this was let go while its release went missing, as across Win + L.
const MODIFIER_FORGOTTEN_AFTER: Duration = Duration::from_millis(1_600);

const VK_BACK: u32 = 0x08;
const VK_TAB: u32 = 0x09;
const VK_RETURN: u32 = 0x0D;
const VK_SHIFT: u32 = 0x10;
const VK_CONTROL: u32 = 0x11;
const VK_MENU: u32 = 0x12;
const VK_PAUSE: u32 = 0x13;
const VK_ESCAPE: u32 = 0x1B;
const VK_SPACE: u32 = 0x20;
const VK_LWIN: u32 = 0x5B;
const VK_RWIN: u32 = 0x5C;
const VK_F1: u32 = 0x70;
const VK_F4: u32 = 0x73;
const VK_F24: u32 = 0x87;
const VK_LSHIFT: u32 = 0xA0;
const VK_RSHIFT: u32 = 0xA1;
const VK_LCONTROL: u32 = 0xA2;
const VK_RCONTROL: u32 = 0xA3;
const VK_LMENU: u32 = 0xA4;
const VK_RMENU: u32 = 0xA5;

/// The keys held down together with the main key.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Modifiers {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    #[serde(default)]
    pub win: bool,
}

/// One shortcut, as saved in Settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Shortcut {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    /// The Windows code of the main key.
    pub key: u32,
    /// How the main key is written on this keyboard, such as "Space" or "F8".
    pub key_name: String,
}

impl Shortcut {
    pub fn hold_default() -> Self {
        Self { ctrl: true, alt: false, shift: false, key: VK_SPACE, key_name: "Space".into() }
    }

    pub fn tap_default() -> Self {
        Self { ctrl: true, alt: false, shift: true, key: VK_SPACE, key_name: "Space".into() }
    }

    fn modifiers(&self) -> Modifiers {
        Modifiers { ctrl: self.ctrl, alt: self.alt, shift: self.shift, win: false }
    }

    /// "Ctrl + Shift + Space".
    pub fn label(&self) -> String {
        let mut parts: Vec<&str> = Vec::new();
        if self.ctrl {
            parts.push("Ctrl");
        }
        if self.alt {
            parts.push("Alt");
        }
        if self.shift {
            parts.push("Shift");
        }
        parts.push(&self.key_name);
        parts.join(" + ")
    }

    fn same_keys(&self, other: &Shortcut) -> bool {
        self.modifiers() == other.modifiers() && self.key == other.key
    }

    /// True when these exact keys are down: Ctrl + Shift + Space doesn't count as Ctrl + Space.
    fn matches(&self, held: Modifiers, key: u32) -> bool {
        key == self.key && held == self.modifiers()
    }

    fn needs(&self, modifier: u32) -> bool {
        match modifier {
            VK_CONTROL | VK_LCONTROL | VK_RCONTROL => self.ctrl,
            VK_MENU | VK_LMENU | VK_RMENU => self.alt,
            VK_SHIFT | VK_LSHIFT | VK_RSHIFT => self.shift,
            _ => false,
        }
    }
}

/// Keys pressed while choosing a new shortcut.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pressed {
    pub modifiers: Modifiers,
    pub key: u32,
}

fn is_modifier(key: u32) -> bool {
    matches!(
        key,
        VK_SHIFT | VK_LSHIFT | VK_RSHIFT | VK_CONTROL | VK_LCONTROL | VK_RCONTROL | VK_MENU | VK_LMENU | VK_RMENU | VK_LWIN | VK_RWIN
    )
}

/// Letters, numbers, symbols and Space: keys that type text.
fn types_text(key: u32) -> bool {
    matches!(key, VK_SPACE | 0x30..=0x39 | 0x41..=0x5A | 0x60..=0x6F | 0xBA..=0xC0 | 0xDB..=0xDF | 0xE2)
}

/// Keys used to move around or edit while typing.
fn edits_text(key: u32) -> bool {
    matches!(key, VK_BACK | VK_TAB | VK_RETURN | VK_ESCAPE | 0x21..=0x28 | 0x2E)
}

/// Why a combination can't be a shortcut, in words for the person. `None` means it's fine.
pub fn problem_with(pressed: &Pressed, name: &str) -> Option<String> {
    let Modifiers { ctrl, alt, shift, win } = pressed.modifiers;
    let key = pressed.key;
    if win {
        return Some("Windows keeps the Windows key for its own shortcuts. Please choose one without it.".into());
    }
    if !ctrl && !alt && !shift && (types_text(key) || edits_text(key)) {
        return Some(format!("{name} on its own is needed for typing. Add Ctrl, Alt or Shift, or choose a key like F8."));
    }
    if shift && !ctrl && !alt && (types_text(key) || edits_text(key)) {
        return Some(format!("Shift + {name} is needed for typing. Add Ctrl or Alt to it."));
    }
    if ctrl && alt && types_text(key) && key != VK_SPACE {
        return Some(
            "Ctrl + Alt is the same as AltGr on Spanish keyboards, which types @, # and other symbols. Please choose another.".into(),
        );
    }
    let windows_uses = (alt && !ctrl && (key == VK_TAB || key == VK_F4 || key == VK_ESCAPE)) || (ctrl && key == VK_ESCAPE);
    if windows_uses {
        return Some("Windows uses that one to switch or close windows. Please choose another.".into());
    }
    if ctrl && !alt && !shift && matches!(key, 0x41 | 0x43 | 0x56 | 0x58 | 0x59 | 0x5A) {
        return Some(format!("Ctrl + {name} is used in every app to copy, paste or undo. Please choose another."));
    }
    None
}

/// What the shortcuts ask for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotkeyEvent {
    Start,
    Stop,
    /// The keys pressed while choosing a new shortcut, or `None` if Esc cancelled.
    Captured(Option<Pressed>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Hold,
    Tap,
}

/// Follows the keys and decides what each one means. It has no Windows code, so every
/// situation can be tested without touching a real keyboard.
#[derive(Debug)]
pub struct HotkeyLogic {
    held: Modifiers,
    hold: Shortcut,
    tap: Shortcut,
    /// The main key of a shortcut that's still down. Its repeats and its release are hidden
    /// from other apps, even if the modifier keys come up first. A stray Space release would
    /// otherwise press whatever button has focus, Mimi's own "Stop" button included.
    trigger_down: Option<u32>,
    recording: Option<Mode>,
    capturing: bool,
    /// When each modifier was last reported down: Ctrl, Alt, Shift, Windows.
    modifier_seen: [Option<Instant>; 4],
}

impl Default for HotkeyLogic {
    fn default() -> Self {
        Self::new(Shortcut::hold_default(), Shortcut::tap_default())
    }
}

/// The result of one key event.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Decision {
    /// True hides this key from other apps, so a space isn't typed.
    pub swallow: bool,
    pub event: Option<HotkeyEvent>,
}

impl HotkeyLogic {
    pub fn new(hold: Shortcut, tap: Shortcut) -> Self {
        Self {
            held: Modifiers::default(),
            hold,
            tap,
            trigger_down: None,
            recording: None,
            capturing: false,
            modifier_seen: [None; 4],
        }
    }

    pub fn configure(&mut self, hold: Shortcut, tap: Shortcut) {
        self.hold = hold;
        self.tap = tap;
    }

    /// Mimi follows the modifier keys herself, from the key presses and releases she sees.
    /// Asking Windows instead gave wrong answers while Mimi's own window was in front, so
    /// Ctrl + Space looked like a plain Space there.
    fn set_modifier(&mut self, key: u32, down: bool) {
        let slot = match key {
            VK_CONTROL | VK_LCONTROL | VK_RCONTROL => {
                self.held.ctrl = down;
                0
            }
            VK_MENU | VK_LMENU | VK_RMENU => {
                self.held.alt = down;
                1
            }
            VK_SHIFT | VK_LSHIFT | VK_RSHIFT => {
                self.held.shift = down;
                2
            }
            VK_LWIN | VK_RWIN => {
                self.held.win = down;
                3
            }
            _ => return,
        };
        self.modifier_seen[slot] = down.then(Instant::now);
    }

    /// Lets go of modifiers whose release went missing. A modifier still held keeps repeating,
    /// so it's never forgotten while it's really down.
    pub fn forget_stale_modifiers(&mut self, now: Instant) {
        let stale = |seen: Option<Instant>| seen.is_none_or(|at| now.saturating_duration_since(at) > MODIFIER_FORGOTTEN_AFTER);
        if stale(self.modifier_seen[0]) {
            self.held.ctrl = false;
        }
        if stale(self.modifier_seen[1]) {
            self.held.alt = false;
        }
        if stale(self.modifier_seen[2]) {
            self.held.shift = false;
        }
        if stale(self.modifier_seen[3]) {
            self.held.win = false;
        }
    }

    pub fn held(&self) -> Modifiers {
        self.held
    }

    pub fn key_down(&mut self, key: u32) -> Decision {
        if is_modifier(key) {
            self.set_modifier(key, true);
            return Decision::default();
        }
        if self.trigger_down == Some(key) {
            // Windows repeats a held key. Keep hiding it from other apps.
            return Decision { swallow: true, event: None };
        }
        if self.capturing {
            self.capturing = false;
            self.trigger_down = Some(key);
            let cancelled = key == VK_ESCAPE && self.held == Modifiers::default();
            let pressed = (!cancelled).then_some(Pressed { modifiers: self.held, key });
            return Decision { swallow: true, event: Some(HotkeyEvent::Captured(pressed)) };
        }
        let mode = if self.hold.matches(self.held, key) {
            Mode::Hold
        } else if self.tap.matches(self.held, key) {
            Mode::Tap
        } else {
            return Decision::default();
        };
        self.trigger_down = Some(key);
        // Either shortcut stops a recording, whichever one started it.
        let event = if self.recording.take().is_some() {
            HotkeyEvent::Stop
        } else {
            self.recording = Some(mode);
            HotkeyEvent::Start
        };
        Decision { swallow: true, event: Some(event) }
    }

    pub fn key_up(&mut self, key: u32) -> Decision {
        if is_modifier(key) {
            self.set_modifier(key, false);
            // Letting go of Ctrl ends a hold too, even while Space is still down.
            if self.recording == Some(Mode::Hold) && self.trigger_down.is_some() && self.hold.needs(key) {
                self.recording = None;
                return Decision { swallow: false, event: Some(HotkeyEvent::Stop) };
            }
            return Decision::default();
        }
        if self.trigger_down == Some(key) {
            self.trigger_down = None;
            let event = (self.recording == Some(Mode::Hold)).then(|| {
                self.recording = None;
                HotkeyEvent::Stop
            });
            return Decision { swallow: true, event };
        }
        Decision::default()
    }

    /// The app may start, finish or fail on its own, so it tells this logic the true state.
    /// A recording started from Mimi's button stops with either shortcut.
    pub fn set_recording(&mut self, recording: bool) {
        self.recording = match (recording, self.recording) {
            (false, _) => None,
            (true, None) => Some(Mode::Tap),
            (true, current) => current,
        };
    }

    /// A shortcut seen by Mimi's own window instead of the keyboard hook. `hold` says which
    /// shortcut, and `down` whether its keys went down or came up.
    pub fn from_screen(&mut self, hold: bool, down: bool) -> Option<HotkeyEvent> {
        if !down {
            return (hold && self.recording == Some(Mode::Hold)).then(|| {
                self.recording = None;
                HotkeyEvent::Stop
            });
        }
        if self.recording.take().is_some() {
            Some(HotkeyEvent::Stop)
        } else {
            self.recording = Some(if hold { Mode::Hold } else { Mode::Tap });
            Some(HotkeyEvent::Start)
        }
    }

    /// The next key pressed becomes a new shortcut instead of doing anything.
    pub fn start_capture(&mut self) {
        self.capturing = true;
    }

    pub fn cancel_capture(&mut self) {
        self.capturing = false;
    }

}

#[cfg(windows)]
mod windows_hook {
    use super::*;
    use std::sync::{Mutex, OnceLock};
    use windows_sys::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        GetAsyncKeyState, GetKeyNameTextW, INPUT, INPUT_KEYBOARD, KEYEVENTF_KEYUP, MAPVK_VK_TO_VSC, MapVirtualKeyW, SendInput,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, DispatchMessageW, GetMessageW, HC_ACTION, KBDLLHOOKSTRUCT, LLKHF_INJECTED, MSG,
        SetWindowsHookExW, TranslateMessage, WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
    };

    /// A key code no keyboard has, used to stop Alt from opening an app's menu bar.
    const VK_UNASSIGNED: u16 = 0xE8;

    struct Shared {
        logic: Mutex<HotkeyLogic>,
        events: Sender<HotkeyEvent>,
        capture: Mutex<Option<Sender<Option<Pressed>>>>,
    }

    static SHARED: OnceLock<Shared> = OnceLock::new();

    fn key_held(key: u32) -> bool {
        unsafe { GetAsyncKeyState(key as i32) < 0 }
    }

    fn mimi_in_front() -> bool {
        use windows_sys::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};
        unsafe {
            let mut process = 0u32;
            GetWindowThreadProcessId(GetForegroundWindow(), &mut process);
            process == std::process::id()
        }
    }

    fn held_now() -> Modifiers {
        Modifiers {
            ctrl: key_held(VK_CONTROL),
            alt: key_held(VK_MENU),
            shift: key_held(VK_SHIFT),
            win: key_held(VK_LWIN) || key_held(VK_RWIN),
        }
    }

    /// When a shortcut uses Alt, Windows would see Alt pressed and released on its own and
    /// open the menu bar in apps like Notepad. A key between them makes it an Alt combination.
    fn hide_lone_alt() {
        let mut inputs: [INPUT; 2] = unsafe { std::mem::zeroed() };
        for (input, flags) in inputs.iter_mut().zip([0, KEYEVENTF_KEYUP]) {
            input.r#type = INPUT_KEYBOARD;
            input.Anonymous.ki.wVk = VK_UNASSIGNED;
            input.Anonymous.ki.dwFlags = flags;
        }
        unsafe {
            SendInput(2, inputs.as_ptr(), std::mem::size_of::<INPUT>() as i32);
        }
    }

    /// Windows calls this for every key press on the PC, so it must return almost instantly.
    /// It reads only the key code, and never keeps it.
    unsafe extern "system" fn on_key(code: i32, message: WPARAM, data: LPARAM) -> LRESULT {
        if code == HC_ACTION as i32
            && let Some(shared) = SHARED.get()
        {
            let info = unsafe { &*(data as *const KBDLLHOOKSTRUCT) };
            // Ignore keys sent by programs, including Mimi's own paste.
            if info.flags & LLKHF_INJECTED == 0
                && let Ok(mut logic) = shared.logic.lock()
            {
                let began = std::time::Instant::now();
                let key_down = matches!(message as u32, WM_KEYDOWN | WM_SYSKEYDOWN);
                if !is_modifier(info.vkCode) {
                    logic.forget_stale_modifiers(began);
                }
                let shortcut_key = info.vkCode == logic.hold.key || info.vkCode == logic.tap.key;
                // For the diagnostics log: whether Windows agrees about the modifiers, and
                // whether this key arrived while Mimi's own window was in front.
                if shortcut_key && logic.held() != Modifiers::default() {
                    let windows_says = held_now();
                    if windows_says != logic.held() {
                        crate::diag::note(format!("Windows reports modifiers {windows_says:?}, Mimi saw {:?}", logic.held()));
                    }
                }
                if shortcut_key && mimi_in_front() {
                    crate::diag::note(format!(
                        "key {:#04x} {} reached the hook while Mimi is in front, held {:?}",
                        info.vkCode,
                        if key_down { "down" } else { "up" },
                        logic.held()
                    ));
                }
                // Only shortcut keys are noted, and plain typing never is.
                let worth_noting = if is_modifier(info.vkCode) {
                    logic.trigger_down.is_some() || logic.recording.is_some()
                } else {
                    (info.vkCode == logic.hold.key || info.vkCode == logic.tap.key)
                        && (logic.trigger_down.is_some() || logic.held != Modifiers::default() || logic.capturing)
                };
                let before = format!("held {:?} recording {:?}", logic.held, logic.recording);
                let decision = match message as u32 {
                    WM_KEYDOWN | WM_SYSKEYDOWN => logic.key_down(info.vkCode),
                    WM_KEYUP | WM_SYSKEYUP => logic.key_up(info.vkCode),
                    _ => Decision::default(),
                };
                let alt_held = logic.held.alt;
                drop(logic);
                if worth_noting || decision.event.is_some() {
                    crate::diag::note(format!(
                        "key {:#04x} {} {before} -> swallow {} event {:?} ({} µs)",
                        info.vkCode,
                        if key_down { "down" } else { "up" },
                        decision.swallow,
                        decision.event,
                        began.elapsed().as_micros()
                    ));
                }
                if decision.swallow && key_down && alt_held {
                    hide_lone_alt();
                }
                match decision.event {
                    Some(HotkeyEvent::Captured(pressed)) => {
                        if let Ok(mut waiting) = shared.capture.lock()
                            && let Some(reply) = waiting.take()
                        {
                            let _ = reply.send(pressed);
                        }
                    }
                    Some(event) => {
                        let _ = shared.events.send(event);
                    }
                    None => {}
                }
                if decision.swallow {
                    return 1;
                }
            }
        }
        unsafe { CallNextHookEx(std::ptr::null_mut(), code, message, data) }
    }

    pub fn install(events: Sender<HotkeyEvent>, hold: Shortcut, tap: Shortcut) -> Result<(), String> {
        SHARED
            .set(Shared { logic: Mutex::new(HotkeyLogic::new(hold, tap)), events, capture: Mutex::new(None) })
            .map_err(|_| "The shortcut is already installed.".to_string())?;
        let (ready, answer) = std::sync::mpsc::channel();
        thread::Builder::new()
            .name("mimi-hotkey".into())
            .spawn(move || unsafe {
                let hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(on_key), std::ptr::null_mut(), 0);
                let _ = ready.send(!hook.is_null());
                if hook.is_null() {
                    return;
                }
                // The hook only runs while this thread is waiting for messages.
                let mut message: MSG = std::mem::zeroed();
                while GetMessageW(&mut message, std::ptr::null_mut(), 0, 0) > 0 {
                    TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
            })
            .map_err(|error| error.to_string())?;
        match answer.recv_timeout(Duration::from_secs(5)) {
            Ok(true) => Ok(()),
            _ => Err("Mimi couldn't set up her keyboard shortcuts.".to_string()),
        }
    }

    pub fn set_recording(recording: bool) {
        if let Some(shared) = SHARED.get()
            && let Ok(mut logic) = shared.logic.lock()
        {
            logic.set_recording(recording);
        }
    }

    pub fn configure(hold: Shortcut, tap: Shortcut) {
        if let Some(shared) = SHARED.get()
            && let Ok(mut logic) = shared.logic.lock()
        {
            logic.configure(hold, tap);
        }
    }

    /// Waits for the next key combination. `None` means Esc, a cancel, or no key in time.
    pub fn capture(limit: Duration) -> Result<Option<Pressed>, String> {
        let shared = SHARED.get().ok_or("Mimi's shortcuts aren't running.")?;
        let (reply, answer) = std::sync::mpsc::channel();
        *shared.capture.lock().map_err(|_| "Mimi hit an internal error.")? = Some(reply);
        shared.logic.lock().map_err(|_| "Mimi hit an internal error.")?.start_capture();
        let pressed = answer.recv_timeout(limit).ok().flatten();
        cancel_capture();
        Ok(pressed)
    }

    /// Mimi's window saw a shortcut the hook missed. It's handled exactly like one from the hook.
    pub fn from_screen(hold: bool, down: bool) {
        if let Some(shared) = SHARED.get() {
            let event = shared.logic.lock().ok().and_then(|mut logic| logic.from_screen(hold, down));
            crate::diag::note(format!("screen saw the {} shortcut {} -> {event:?}", if hold { "hold" } else { "tap" }, if down { "down" } else { "up" }));
            if let Some(event) = event {
                let _ = shared.events.send(event);
            }
        }
    }

    /// Mimi's window saw the new keys while a shortcut was being changed.
    pub fn captured_from_screen(pressed: Pressed) {
        if let Some(shared) = SHARED.get() {
            crate::diag::note(format!("screen captured {pressed:?}"));
            if let Ok(mut logic) = shared.logic.lock() {
                logic.cancel_capture();
            }
            if let Ok(mut waiting) = shared.capture.lock()
                && let Some(reply) = waiting.take()
            {
                let _ = reply.send(Some(pressed));
            }
        }
    }

    pub fn cancel_capture() {
        if let Some(shared) = SHARED.get() {
            if let Ok(mut logic) = shared.logic.lock() {
                logic.cancel_capture();
            }
            if let Ok(mut waiting) = shared.capture.lock()
                && let Some(reply) = waiting.take()
            {
                let _ = reply.send(None);
            }
        }
    }

    /// How a key is written on this keyboard: "Space", "F8", "Ñ".
    pub fn key_name(key: u32) -> String {
        if let Some(name) = super::common_key_name(key) {
            return name;
        }
        unsafe {
            let scan = MapVirtualKeyW(key, MAPVK_VK_TO_VSC);
            let mut text = [0u16; 64];
            let length = GetKeyNameTextW((scan << 16) as i32, text.as_mut_ptr(), text.len() as i32);
            if length > 0 {
                return String::from_utf16_lossy(&text[..length as usize]);
            }
        }
        format!("Key {key}")
    }
}

/// Names for keys whose Windows name is unclear, such as arrows and the number pad.
fn common_key_name(key: u32) -> Option<String> {
    let name = match key {
        VK_SPACE => "Space".to_string(),
        VK_RETURN => "Enter".to_string(),
        VK_TAB => "Tab".to_string(),
        VK_BACK => "Backspace".to_string(),
        VK_ESCAPE => "Esc".to_string(),
        VK_PAUSE => "Pause".to_string(),
        0x21 => "Page Up".to_string(),
        0x22 => "Page Down".to_string(),
        0x23 => "End".to_string(),
        0x24 => "Home".to_string(),
        0x25 => "Left".to_string(),
        0x26 => "Up".to_string(),
        0x27 => "Right".to_string(),
        0x28 => "Down".to_string(),
        0x2D => "Insert".to_string(),
        0x2E => "Delete".to_string(),
        0x30..=0x39 | 0x41..=0x5A => char::from_u32(key)?.to_string(),
        0x60..=0x69 => format!("Num {}", key - 0x60),
        0x6A => "Num *".to_string(),
        0x6B => "Num +".to_string(),
        0x6D => "Num -".to_string(),
        0x6E => "Num .".to_string(),
        0x6F => "Num /".to_string(),
        VK_F1..=VK_F24 => format!("F{}", key - VK_F1 + 1),
        0x91 => "Scroll Lock".to_string(),
        _ => return None,
    };
    Some(name)
}

/// Turns pressed keys into a shortcut, or explains why they can't be one.
pub fn shortcut_from(pressed: &Pressed, other: &Shortcut, other_name: &str) -> Result<Shortcut, String> {
    #[cfg(windows)]
    let name = windows_hook::key_name(pressed.key);
    #[cfg(not(windows))]
    let name = common_key_name(pressed.key).unwrap_or_else(|| format!("Key {}", pressed.key));
    if let Some(problem) = problem_with(pressed, &name) {
        return Err(problem);
    }
    let shortcut = Shortcut {
        ctrl: pressed.modifiers.ctrl,
        alt: pressed.modifiers.alt,
        shift: pressed.modifiers.shift,
        key: pressed.key,
        key_name: name,
    };
    if shortcut.same_keys(other) {
        return Err(format!("{} is already your {other_name} shortcut.", shortcut.label()));
    }
    Ok(shortcut)
}

#[cfg(windows)]
pub use windows_hook::{cancel_capture, capture, captured_from_screen, configure, from_screen, install, set_recording};

#[cfg(test)]
mod tests {
    use super::*;

    const CTRL: u32 = VK_LCONTROL;
    const SHIFT: u32 = VK_LSHIFT;

    fn logic() -> HotkeyLogic {
        HotkeyLogic::default()
    }

    fn start() -> Decision {
        Decision { swallow: true, event: Some(HotkeyEvent::Start) }
    }

    fn stop() -> Decision {
        Decision { swallow: true, event: Some(HotkeyEvent::Stop) }
    }

    fn hidden() -> Decision {
        Decision { swallow: true, event: None }
    }

    fn pressed(ctrl: bool, alt: bool, shift: bool, key: u32) -> Pressed {
        Pressed { modifiers: Modifiers { ctrl, alt, shift, win: false }, key }
    }

    #[test]
    fn holding_records_until_space_is_released() {
        let mut logic = logic();
        logic.key_down(CTRL);
        assert_eq!(logic.key_down(VK_SPACE), start());
        assert_eq!(logic.key_up(VK_SPACE), stop());
    }

    #[test]
    fn a_quick_hold_still_stops_on_release() {
        // Holding is only for holding now: a tap of Ctrl + Space records just that instant.
        let mut logic = logic();
        logic.key_down(CTRL);
        assert_eq!(logic.key_down(VK_SPACE), start());
        assert_eq!(logic.key_up(VK_SPACE), stop());
        assert_eq!(logic.key_up(CTRL), Decision::default());
    }

    #[test]
    fn releasing_ctrl_first_stops_and_the_space_release_stays_hidden() {
        // This was the bug inside Mimi's window: the visible Space release pressed her
        // focused "Stop" button.
        let mut logic = logic();
        logic.key_down(CTRL);
        assert_eq!(logic.key_down(VK_SPACE), start());
        assert_eq!(logic.key_up(CTRL), Decision { swallow: false, event: Some(HotkeyEvent::Stop) });
        assert_eq!(logic.key_up(VK_SPACE), hidden());
    }

    #[test]
    fn tapping_the_tap_shortcut_starts_and_the_next_tap_stops() {
        let mut logic = logic();
        logic.key_down(CTRL);
        logic.key_down(SHIFT);
        assert_eq!(logic.key_down(VK_SPACE), start());
        assert_eq!(logic.key_up(VK_SPACE), hidden());
        assert_eq!(logic.key_up(SHIFT), Decision::default());
        assert_eq!(logic.key_up(CTRL), Decision::default());

        logic.key_down(CTRL);
        logic.key_down(SHIFT);
        assert_eq!(logic.key_down(VK_SPACE), stop());
        assert_eq!(logic.key_up(SHIFT), Decision::default());
        // Shift came up first. Space's release is still hidden.
        assert_eq!(logic.key_up(VK_SPACE), hidden());
        assert_eq!(logic.key_up(CTRL), Decision::default());
    }

    #[test]
    fn the_hold_shortcut_stops_a_tap_recording_without_starting_another() {
        let mut logic = logic();
        logic.key_down(CTRL);
        logic.key_down(SHIFT);
        logic.key_down(VK_SPACE);
        logic.key_up(VK_SPACE);
        logic.key_up(SHIFT);
        assert_eq!(logic.key_down(VK_SPACE), stop());
        assert_eq!(logic.key_up(VK_SPACE), hidden());
    }

    #[test]
    fn a_recording_from_mimis_button_stops_with_either_shortcut() {
        let mut logic = logic();
        logic.set_recording(true);
        logic.key_down(CTRL);
        assert_eq!(logic.key_down(VK_SPACE), stop());
        assert_eq!(logic.key_up(VK_SPACE), hidden());
    }

    #[test]
    fn a_modifier_whose_release_went_missing_is_forgotten() {
        let mut logic = logic();
        logic.key_down(CTRL);
        // No release arrives, and no repeat either, for two seconds.
        logic.forget_stale_modifiers(Instant::now() + Duration::from_secs(2));
        assert_eq!(logic.key_down(VK_SPACE), Decision::default());
    }

    #[test]
    fn a_modifier_still_repeating_is_kept() {
        let mut logic = logic();
        logic.key_down(CTRL);
        logic.key_down(CTRL); // Windows repeats a held Ctrl.
        logic.forget_stale_modifiers(Instant::now());
        assert_eq!(logic.key_down(VK_SPACE), start());
    }

    #[test]
    fn shortcuts_seen_by_mimis_window_work_like_the_hook() {
        let mut logic = logic();
        assert_eq!(logic.from_screen(true, true), Some(HotkeyEvent::Start));
        assert_eq!(logic.from_screen(true, false), Some(HotkeyEvent::Stop));
        assert_eq!(logic.from_screen(false, true), Some(HotkeyEvent::Start));
        assert_eq!(logic.from_screen(false, false), None);
        assert_eq!(logic.from_screen(false, true), Some(HotkeyEvent::Stop));
        // A tap recording also stops with the hold shortcut, without starting another.
        logic.from_screen(false, true);
        assert_eq!(logic.from_screen(true, true), Some(HotkeyEvent::Stop));
        assert_eq!(logic.from_screen(true, false), None);
    }

    #[test]
    fn repeated_space_from_holding_is_hidden_and_does_nothing() {
        let mut logic = logic();
        logic.key_down(CTRL);
        logic.key_down(VK_SPACE);
        for _ in 0..3 {
            assert_eq!(logic.key_down(VK_SPACE), hidden());
        }
    }

    #[test]
    fn normal_typing_passes_through() {
        let mut logic = logic();
        assert_eq!(logic.key_down(VK_SPACE), Decision::default());
        assert_eq!(logic.key_up(VK_SPACE), Decision::default());
        assert_eq!(logic.key_down(0x41), Decision::default());
        assert_eq!(logic.key_up(0x41), Decision::default());
    }

    #[test]
    fn other_ctrl_shortcuts_pass_through() {
        let mut logic = logic();
        logic.key_down(CTRL);
        assert_eq!(logic.key_down(0x43), Decision::default()); // Ctrl + C
        assert_eq!(logic.key_up(0x43), Decision::default());
        assert_eq!(logic.key_up(CTRL), Decision::default());
    }

    #[test]
    fn altgr_space_on_spanish_keyboards_is_not_a_shortcut() {
        let mut logic = logic();
        logic.key_down(VK_LCONTROL);
        logic.key_down(VK_RMENU);
        assert_eq!(logic.key_down(VK_SPACE), Decision::default());
    }

    #[test]
    fn ctrl_win_space_is_left_alone() {
        let mut logic = logic();
        logic.key_down(CTRL);
        logic.key_down(VK_LWIN);
        assert_eq!(logic.key_down(VK_SPACE), Decision::default());
    }

    #[test]
    fn typing_a_space_after_the_shortcut_is_a_normal_space() {
        let mut logic = logic();
        logic.key_down(CTRL);
        logic.key_down(VK_SPACE);
        logic.key_up(VK_SPACE);
        logic.key_up(CTRL);
        assert_eq!(logic.key_down(VK_SPACE), Decision::default());
        assert_eq!(logic.key_up(VK_SPACE), Decision::default());
    }

    #[test]
    fn right_ctrl_works_too() {
        let mut logic = logic();
        logic.key_down(VK_RCONTROL);
        assert_eq!(logic.key_down(VK_SPACE).event, Some(HotkeyEvent::Start));
    }

    #[test]
    fn a_failed_recording_lets_the_next_press_start_again() {
        let mut logic = logic();
        logic.key_down(CTRL);
        logic.key_down(SHIFT);
        logic.key_down(VK_SPACE);
        logic.key_up(VK_SPACE);
        logic.set_recording(false);
        assert_eq!(logic.key_down(VK_SPACE).event, Some(HotkeyEvent::Start));
    }

    #[test]
    fn changed_shortcuts_take_over_and_the_old_ones_are_freed() {
        let mut logic = logic();
        let f8 = Shortcut { ctrl: false, alt: false, shift: false, key: VK_F1 + 7, key_name: "F8".into() };
        let alt_d = Shortcut { ctrl: false, alt: true, shift: false, key: 0x44, key_name: "D".into() };
        logic.configure(f8, alt_d);
        logic.key_down(CTRL);
        assert_eq!(logic.key_down(VK_SPACE), Decision::default());
        logic.key_up(VK_SPACE);
        logic.key_up(CTRL);
        assert_eq!(logic.key_down(VK_F1 + 7), start());
        assert_eq!(logic.key_up(VK_F1 + 7), stop());
        logic.key_down(VK_LMENU);
        assert_eq!(logic.key_down(0x44), start());
        assert_eq!(logic.key_up(0x44), hidden());
        assert_eq!(logic.key_down(0x44), stop());
    }

    #[test]
    fn choosing_a_shortcut_takes_the_next_combination_and_hides_it() {
        let mut logic = logic();
        logic.start_capture();
        logic.key_down(CTRL);
        logic.key_down(VK_LMENU);
        let decision = logic.key_down(0x4D);
        assert_eq!(decision, Decision { swallow: true, event: Some(HotkeyEvent::Captured(Some(pressed(true, true, false, 0x4D)))) });
        assert_eq!(logic.key_up(0x4D), hidden());
        // Capturing ends after one combination, and the shortcuts work again.
        logic.key_up(VK_LMENU);
        assert_eq!(logic.key_down(VK_SPACE), start());
    }

    #[test]
    fn esc_cancels_choosing_a_shortcut() {
        let mut logic = logic();
        logic.start_capture();
        assert_eq!(logic.key_down(VK_ESCAPE).event, Some(HotkeyEvent::Captured(None)));
    }

    #[test]
    fn shortcuts_that_would_get_in_the_way_are_refused() {
        let refused = [
            pressed(false, false, false, 0x41), // A
            pressed(false, false, false, VK_SPACE),
            pressed(false, false, true, 0x41), // Shift + A types a capital
            pressed(true, true, false, 0x51),  // Ctrl + Alt + Q is AltGr + Q, which types @
            pressed(false, true, false, VK_TAB),
            pressed(false, true, false, VK_F4),
            pressed(true, false, false, 0x56), // Ctrl + V
            pressed(true, false, false, VK_ESCAPE),
        ];
        for keys in refused {
            assert!(problem_with(&keys, "X").is_some(), "{keys:?} should be refused");
        }
        let win = Pressed { modifiers: Modifiers { ctrl: false, alt: false, shift: false, win: true }, key: 0x44 };
        assert!(problem_with(&win, "D").is_some());
    }

    #[test]
    fn sensible_shortcuts_are_accepted() {
        let accepted = [
            pressed(true, false, false, VK_SPACE),
            pressed(true, false, true, VK_SPACE),
            pressed(false, false, false, VK_F1 + 7), // F8
            pressed(false, true, false, 0x44),        // Alt + D
            pressed(true, true, false, VK_SPACE),
            pressed(true, false, true, 0x56), // Ctrl + Shift + V
            pressed(false, false, false, VK_PAUSE),
        ];
        for keys in accepted {
            assert_eq!(problem_with(&keys, "X"), None, "{keys:?} should be accepted");
        }
    }

    #[test]
    fn both_shortcuts_must_be_different() {
        let hold = Shortcut::hold_default();
        let result = shortcut_from(&pressed(true, false, false, VK_SPACE), &hold, "hold to talk");
        assert_eq!(result.unwrap_err(), "Ctrl + Space is already your hold to talk shortcut.");
        let tap = shortcut_from(&pressed(false, true, false, 0x44), &hold, "hold to talk").unwrap();
        assert_eq!(tap.label(), "Alt + D");
    }
}
