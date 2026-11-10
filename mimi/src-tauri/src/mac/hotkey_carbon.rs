//! Mimi's global shortcuts on Mac, registered with macOS as system hot keys. This needs no
//! permission, and it never sees any key except the two shortcuts.
//!
//! macOS reports each shortcut going down and coming up, which feeds the same `HotkeyLogic`
//! as Windows. Mimi's window catches the new keys while a shortcut is being changed.

use super::*;
use std::ffi::c_void;
use std::sync::{Mutex, OnceLock};

type Ref = *mut c_void;
type EventHandler = unsafe extern "C" fn(Ref, Ref, Ref) -> i32;

#[repr(C)]
struct EventTypeSpec {
    class: u32,
    kind: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct EventHotKeyId {
    signature: u32,
    id: u32,
}

#[link(name = "Carbon", kind = "framework")]
unsafe extern "C" {
    fn GetApplicationEventTarget() -> Ref;
    fn InstallEventHandler(target: Ref, handler: EventHandler, count: u32, types: *const EventTypeSpec, user_data: Ref, out_ref: *mut Ref) -> i32;
    fn RegisterEventHotKey(key: u32, modifiers: u32, id: EventHotKeyId, target: Ref, options: u32, out_ref: *mut Ref) -> i32;
    fn UnregisterEventHotKey(hot_key: Ref) -> i32;
    fn GetEventKind(event: Ref) -> u32;
    fn GetEventParameter(event: Ref, name: u32, desired_type: u32, actual_type: *mut u32, buffer_size: usize, actual_size: *mut usize, buffer: *mut c_void) -> i32;
}

unsafe extern "C" {
    static _dispatch_main_q: c_void;
    fn dispatch_async_f(queue: *const c_void, context: *mut c_void, work: extern "C" fn(*mut c_void));
}

const KEYBOARD_EVENTS: u32 = u32::from_be_bytes(*b"keyb");
const HOT_KEY_PRESSED: u32 = 5;
const HOT_KEY_RELEASED: u32 = 6;
const DIRECT_OBJECT: u32 = u32::from_be_bytes(*b"----");
const HOT_KEY_ID_TYPE: u32 = u32::from_be_bytes(*b"hkid");
const MIMI_SIGNATURE: u32 = u32::from_be_bytes(*b"mimi");
const HOLD_ID: u32 = 1;
const TAP_ID: u32 = 2;

const SHIFT_MASK: u32 = 0x200;
const OPTION_MASK: u32 = 0x800;
const CONTROL_MASK: u32 = 0x1000;

struct Shared {
    logic: Mutex<HotkeyLogic>,
    events: Sender<HotkeyEvent>,
    capture: Mutex<Option<Sender<Option<Pressed>>>>,
}

static SHARED: OnceLock<Shared> = OnceLock::new();
/// The two registered hot keys, kept as numbers because macOS only ever gets them back.
static REGISTERED: Mutex<[usize; 2]> = Mutex::new([0, 0]);
static HANDLER_INSTALLED: OnceLock<()> = OnceLock::new();

/// The Windows code for a key on a Mac keyboard, which is what Settings stores.
fn windows_code(mac_key: u16) -> Option<u32> {
    let code = match mac_key {
        0 => 0x41,   // A
        1 => 0x53,   // S
        2 => 0x44,   // D
        3 => 0x46,   // F
        4 => 0x48,   // H
        5 => 0x47,   // G
        6 => 0x5A,   // Z
        7 => 0x58,   // X
        8 => 0x43,   // C
        9 => 0x56,   // V
        11 => 0x42,  // B
        12 => 0x51,  // Q
        13 => 0x57,  // W
        14 => 0x45,  // E
        15 => 0x52,  // R
        16 => 0x59,  // Y
        17 => 0x54,  // T
        18 => 0x31,  // 1
        19 => 0x32,  // 2
        20 => 0x33,  // 3
        21 => 0x34,  // 4
        22 => 0x36,  // 6
        23 => 0x35,  // 5
        24 => 0xBB,  // =
        25 => 0x39,  // 9
        26 => 0x37,  // 7
        27 => 0xBD,  // -
        28 => 0x38,  // 8
        29 => 0x30,  // 0
        30 => 0xDD,  // ]
        31 => 0x4F,  // O
        32 => 0x55,  // U
        33 => 0xDB,  // [
        34 => 0x49,  // I
        35 => 0x50,  // P
        36 => VK_RETURN,
        37 => 0x4C,  // L
        38 => 0x4A,  // J
        39 => 0xDE,  // '
        40 => 0x4B,  // K
        41 => 0xBA,  // ;
        42 => 0xDC,  // \
        43 => 0xBC,  // ,
        44 => 0xBF,  // /
        45 => 0x4E,  // N
        46 => 0x4D,  // M
        47 => 0xBE,  // .
        48 => VK_TAB,
        49 => VK_SPACE,
        50 => 0xC0,  // `
        51 => VK_BACK,
        53 => VK_ESCAPE,
        65 => 0x6E,  // Number pad .
        67 => 0x6A,  // Number pad *
        69 => 0x6B,  // Number pad +
        75 => 0x6F,  // Number pad /
        78 => 0x6D,  // Number pad -
        82..=89 => 0x60 + (mac_key as u32 - 82), // Number pad 0 to 7
        91 => 0x68,  // Number pad 8
        92 => 0x69,  // Number pad 9
        122 => VK_F1,
        120 => VK_F1 + 1,
        99 => VK_F1 + 2,
        118 => VK_F1 + 3,
        96 => VK_F1 + 4,
        97 => VK_F1 + 5,
        98 => VK_F1 + 6,
        100 => VK_F1 + 7,
        101 => VK_F1 + 8,
        109 => VK_F1 + 9,
        103 => VK_F1 + 10,
        111 => VK_F1 + 11,
        105 => VK_F1 + 12,
        107 => VK_F1 + 13,
        113 => VK_F1 + 14,
        106 => VK_F1 + 15,
        64 => VK_F1 + 16,
        79 => VK_F1 + 17,
        80 => VK_F1 + 18,
        90 => VK_F1 + 19,
        115 => 0x24, // Home
        116 => 0x21, // Page Up
        117 => 0x2E, // Forward delete
        119 => 0x23, // End
        121 => 0x22, // Page Down
        123 => 0x25, // Left
        124 => 0x27, // Right
        125 => 0x28, // Down
        126 => 0x26, // Up
        _ => return None,
    };
    Some(code)
}

fn mac_key_for(windows_key: u32) -> Option<u32> {
    (0u16..128).find(|key| windows_code(*key) == Some(windows_key)).map(u32::from)
}

fn mac_modifiers(shortcut: &Shortcut) -> u32 {
    let mut mask = 0;
    if shortcut.ctrl {
        mask |= CONTROL_MASK;
    }
    if shortcut.alt {
        mask |= OPTION_MASK;
    }
    if shortcut.shift {
        mask |= SHIFT_MASK;
    }
    mask
}

/// What one shortcut going down or up means. Kept apart from macOS so it can be tested.
fn decide(logic: &mut HotkeyLogic, hold: bool, down: bool) -> Option<HotkeyEvent> {
    if logic.capturing {
        if !down {
            return None;
        }
        // These keys belong to macOS's hot key now, so Mimi's window can't see them.
        logic.capturing = false;
        let shortcut = if hold { &logic.hold } else { &logic.tap };
        return Some(HotkeyEvent::Captured(Some(Pressed { modifiers: shortcut.modifiers(), key: shortcut.key })));
    }
    logic.from_screen(hold, down)
}

fn deliver(shared: &Shared, event: HotkeyEvent) {
    match event {
        HotkeyEvent::Captured(pressed) => {
            if let Ok(mut waiting) = shared.capture.lock()
                && let Some(reply) = waiting.take()
            {
                let _ = reply.send(pressed);
            }
        }
        other => {
            let _ = shared.events.send(other);
        }
    }
}

unsafe extern "C" fn on_hot_key(_next: Ref, event: Ref, _user_data: Ref) -> i32 {
    let mut id = EventHotKeyId::default();
    let status = unsafe {
        GetEventParameter(
            event,
            DIRECT_OBJECT,
            HOT_KEY_ID_TYPE,
            std::ptr::null_mut(),
            std::mem::size_of::<EventHotKeyId>(),
            std::ptr::null_mut(),
            (&raw mut id).cast(),
        )
    };
    let Some(shared) = SHARED.get() else { return 0 };
    if status != 0 || id.signature != MIMI_SIGNATURE {
        return 0;
    }
    let down = unsafe { GetEventKind(event) } == HOT_KEY_PRESSED;
    let happened = shared.logic.lock().ok().and_then(|mut logic| decide(&mut logic, id.id == HOLD_ID, down));
    if let Some(happened) = happened {
        crate::diag::note(format!("{} shortcut {} -> {happened:?}", if id.id == HOLD_ID { "hold" } else { "tap" }, if down { "down" } else { "up" }));
        deliver(shared, happened);
    }
    0
}

fn register_one(shortcut: &Shortcut, id: u32, name: &str) -> usize {
    let Some(key) = mac_key_for(shortcut.key) else {
        crate::diag::note(format!("the {name} shortcut uses a key the Mac doesn't have"));
        return 0;
    };
    let mut hot_key: Ref = std::ptr::null_mut();
    let hot_key_id = EventHotKeyId { signature: MIMI_SIGNATURE, id };
    let status = unsafe { RegisterEventHotKey(key, mac_modifiers(shortcut), hot_key_id, GetApplicationEventTarget(), 0, &mut hot_key) };
    crate::diag::note(format!("registered the {name} shortcut {}: status {status}", shortcut.label()));
    hot_key as usize
}

/// Runs on the main thread, where macOS delivers hot keys.
fn register_both(hold: &Shortcut, tap: &Shortcut) {
    HANDLER_INSTALLED.get_or_init(|| {
        let kinds = [
            EventTypeSpec { class: KEYBOARD_EVENTS, kind: HOT_KEY_PRESSED },
            EventTypeSpec { class: KEYBOARD_EVENTS, kind: HOT_KEY_RELEASED },
        ];
        let status = unsafe {
            InstallEventHandler(GetApplicationEventTarget(), on_hot_key, kinds.len() as u32, kinds.as_ptr(), std::ptr::null_mut(), std::ptr::null_mut())
        };
        crate::diag::note(format!("shortcut listener installed: status {status}"));
    });
    let Ok(mut registered) = REGISTERED.lock() else { return };
    for hot_key in registered.iter_mut() {
        if *hot_key != 0 {
            unsafe { UnregisterEventHotKey(*hot_key as Ref) };
            *hot_key = 0;
        }
    }
    registered[0] = register_one(hold, HOLD_ID, "hold");
    registered[1] = register_one(tap, TAP_ID, "tap");
}

extern "C" fn register_on_main(context: *mut c_void) {
    let shortcuts = unsafe { Box::from_raw(context.cast::<(Shortcut, Shortcut)>()) };
    register_both(&shortcuts.0, &shortcuts.1);
}

fn register_later(hold: Shortcut, tap: Shortcut) {
    let context = Box::into_raw(Box::new((hold, tap)));
    unsafe { dispatch_async_f(&raw const _dispatch_main_q, context.cast(), register_on_main) };
}

pub fn install(events: Sender<HotkeyEvent>, hold: Shortcut, tap: Shortcut) -> Result<(), String> {
    SHARED
        .set(Shared { logic: Mutex::new(HotkeyLogic::new(hold.clone(), tap.clone())), events, capture: Mutex::new(None) })
        .map_err(|_| "The shortcut is already installed.".to_string())?;
    register_later(hold, tap);
    Ok(())
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
        logic.configure(hold.clone(), tap.clone());
    }
    register_later(hold, tap);
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

/// Mimi's window saw a shortcut macOS didn't deliver. It's handled exactly the same way.
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

#[cfg(test)]
mod tests {
    use super::*;

    fn mac_logic() -> HotkeyLogic {
        HotkeyLogic::new(Shortcut::hold_default(), Shortcut::tap_default())
    }

    #[test]
    fn holding_records_until_the_shortcut_comes_up() {
        let mut logic = mac_logic();
        assert_eq!(decide(&mut logic, true, true), Some(HotkeyEvent::Start));
        assert_eq!(decide(&mut logic, true, false), Some(HotkeyEvent::Stop));
    }

    #[test]
    fn the_tap_shortcut_starts_on_one_press_and_stops_on_the_next() {
        let mut logic = mac_logic();
        assert_eq!(decide(&mut logic, false, true), Some(HotkeyEvent::Start));
        assert_eq!(decide(&mut logic, false, false), None);
        assert_eq!(decide(&mut logic, false, true), Some(HotkeyEvent::Stop));
        assert_eq!(decide(&mut logic, false, false), None);
    }

    #[test]
    fn either_shortcut_stops_a_recording_from_the_button() {
        let mut logic = mac_logic();
        logic.set_recording(true);
        assert_eq!(decide(&mut logic, true, true), Some(HotkeyEvent::Stop));
    }

    #[test]
    fn pressing_a_current_shortcut_while_changing_one_is_captured() {
        let mut logic = mac_logic();
        logic.start_capture();
        let expected = Pressed { modifiers: Modifiers { ctrl: false, alt: true, shift: false, win: false }, key: VK_SPACE };
        assert_eq!(decide(&mut logic, true, true), Some(HotkeyEvent::Captured(Some(expected))));
        assert_eq!(decide(&mut logic, true, false), None, "letting go afterwards doesn't start anything");
    }

    #[test]
    fn the_default_shortcuts_use_the_right_mac_keys() {
        assert_eq!(mac_key_for(Shortcut::hold_default().key), Some(49));
        assert_eq!(mac_modifiers(&Shortcut::hold_default()), OPTION_MASK);
        assert_eq!(mac_modifiers(&Shortcut::tap_default()), OPTION_MASK | SHIFT_MASK);
    }

    #[test]
    fn keys_translate_both_ways() {
        assert_eq!(windows_code(0), Some(0x41));
        assert_eq!(mac_key_for(0x41), Some(0));
        assert_eq!(mac_key_for(VK_F1 + 7), Some(100));
        assert_eq!(windows_code(200), None);
        assert_eq!(mac_key_for(0xFFFF), None);
    }
}
