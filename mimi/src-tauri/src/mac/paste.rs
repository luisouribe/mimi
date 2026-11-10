//! Puts dictated text into whichever app is active, and puts the clipboard back afterwards.
//!
//! Mimi saves what was on the clipboard, pastes with Cmd + V, then restores it. The dictated
//! text is marked as temporary, so clipboard history apps skip it.

use objc2::rc::{Retained, autoreleasepool};
use objc2::runtime::ProtocolObject;
use objc2_app_kit::{
    NSPasteboard, NSPasteboardItem, NSPasteboardTypeString, NSPasteboardWriting, NSRunningApplication, NSWorkspace,
};
use objc2_foundation::{NSArray, NSData, NSString};
use serde::Serialize;
use std::ffi::c_void;
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};

type Ref = *mut c_void;

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGEventSourceCreate(state: i32) -> Ref;
    fn CGEventSourceFlagsState(state: i32) -> u64;
    fn CGEventCreateKeyboardEvent(source: Ref, key: u16, down: bool) -> Ref;
    fn CGEventSetFlags(event: Ref, flags: u64);
    fn CGEventPost(tap: u32, event: Ref);
}

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFRelease(item: Ref);
}

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn AXIsProcessTrusted() -> bool;
}

const COMBINED_SESSION_STATE: i32 = 0;
const HID_SYSTEM_STATE: i32 = 1;
const SESSION_TAP: u32 = 1;
const KEY_V: u16 = 9;
const COMMAND_FLAG: u64 = 0x10_0000;
/// Shift, Control, Option and Command.
const ANY_MODIFIER: u64 = 0x2_0000 | 0x4_0000 | 0x8_0000 | COMMAND_FLAG;
/// Clipboard history apps skip anything carrying this type.
const TEMPORARY_TYPE: &str = "org.nspasteboard.TransientType";

/// Time for the target app to read the clipboard before it's restored.
const RESTORE_DELAY: Duration = Duration::from_millis(400);
const MODIFIER_WAIT: Duration = Duration::from_millis(1_000);
/// A clipboard larger than this isn't copied in memory. It would slow every dictation.
const MAX_SNAPSHOT_BYTES: usize = 256 * 1024 * 1024;

/// Only one paste at a time, so a restore never overwrites the next dictation.
static PASTE_LOCK: Mutex<()> = Mutex::new(());

fn front_process_id() -> i32 {
    autoreleasepool(|_| {
        NSWorkspace::sharedWorkspace().frontmostApplication().map_or(0, |app| app.processIdentifier())
    })
}

/// The app that was active when dictation stopped.
#[derive(Debug, Clone, Copy)]
pub struct Target {
    process_id: i32,
}

impl Target {
    pub fn current() -> Self {
        Self { process_id: front_process_id() }
    }

    pub fn is_mimi(&self) -> bool {
        self.process_id == std::process::id() as i32
    }

    pub fn still_active(&self) -> bool {
        front_process_id() == self.process_id
    }

    /// A short app name for the history. The window title is never stored.
    pub fn app_name(&self) -> String {
        if self.is_mimi() {
            return "Mimi".to_string();
        }
        autoreleasepool(|_| {
            NSRunningApplication::runningApplicationWithProcessIdentifier(self.process_id)
                .and_then(|app| app.localizedName())
                .map_or_else(|| "Unknown app".to_string(), |name| name.to_string())
        })
    }
}

/// What happened to the dictated text.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PasteOutcome {
    Pasted,
    /// The text couldn't be pasted safely, so it's on the clipboard, ready for Cmd + V.
    CopiedInstead { reason: String },
    /// The text was dictated inside Mimi's own window, so it only goes to the history.
    StayedInMimi,
}

type SavedItem = Vec<(Retained<NSString>, Retained<NSData>)>;

fn temporary_type() -> Retained<NSString> {
    NSString::from_str(TEMPORARY_TYPE)
}

fn replace_contents(board: &NSPasteboard, items: Vec<Retained<NSPasteboardItem>>) -> Result<(), String> {
    board.clearContents();
    if items.is_empty() {
        return Ok(());
    }
    let writers: Vec<Retained<ProtocolObject<dyn NSPasteboardWriting>>> =
        items.into_iter().map(ProtocolObject::from_retained).collect();
    if board.writeObjects(&NSArray::from_retained_slice(&writers)) {
        Ok(())
    } else {
        Err("The text couldn't be placed on the clipboard.".to_string())
    }
}

/// Everything on the clipboard, as copies of its data.
fn snapshot(board: &NSPasteboard) -> Vec<SavedItem> {
    let mut saved = Vec::new();
    let mut total = 0usize;
    let Some(items) = board.pasteboardItems() else { return saved };
    for item in items.iter() {
        let mut parts: SavedItem = Vec::new();
        for kind in item.types().iter() {
            if let Some(data) = item.dataForType(&kind)
                && total + data.len() <= MAX_SNAPSHOT_BYTES
            {
                total += data.len();
                parts.push((kind.clone(), data));
            }
        }
        if !parts.is_empty() {
            saved.push(parts);
        }
    }
    saved
}

fn write_text(board: &NSPasteboard, text: &str, temporary: bool) -> Result<(), String> {
    let item = NSPasteboardItem::new();
    item.setString_forType(&NSString::from_str(text), unsafe { NSPasteboardTypeString });
    if temporary {
        item.setData_forType(&NSData::new(), &temporary_type());
    }
    replace_contents(board, vec![item])
}

fn restore(board: &NSPasteboard, saved: &[SavedItem]) -> Result<(), String> {
    let items: Vec<Retained<NSPasteboardItem>> = saved
        .iter()
        .map(|parts| {
            let item = NSPasteboardItem::new();
            for (kind, data) in parts {
                item.setData_forType(data, kind);
            }
            // It was already in clipboard history when first copied, so it shouldn't appear twice.
            item.setData_forType(&NSData::new(), &temporary_type());
            item
        })
        .collect();
    replace_contents(board, items)
}

/// Puts text on the clipboard for the Copy buttons. This behaves like a normal copy.
pub fn copy_to_clipboard(text: &str) -> Result<(), String> {
    let _guard = PASTE_LOCK.lock().map_err(|_| "The clipboard is locked.".to_string())?;
    autoreleasepool(|_| write_text(&NSPasteboard::generalPasteboard(), text, false))
}

fn modifier_held() -> bool {
    unsafe { CGEventSourceFlagsState(HID_SYSTEM_STATE) & ANY_MODIFIER != 0 }
}

/// Sends Cmd + V. Keys still held from the shortcut are given a moment to come up first.
fn send_paste_keys() -> Result<(), String> {
    let started = Instant::now();
    while started.elapsed() < MODIFIER_WAIT && modifier_held() {
        thread::sleep(Duration::from_millis(5));
    }
    if modifier_held() {
        return Err("A modifier key is still held down.".to_string());
    }
    unsafe {
        let source = CGEventSourceCreate(COMBINED_SESSION_STATE);
        let press = CGEventCreateKeyboardEvent(source, KEY_V, true);
        let release = CGEventCreateKeyboardEvent(source, KEY_V, false);
        let created = !press.is_null() && !release.is_null();
        if created {
            CGEventSetFlags(press, COMMAND_FLAG);
            CGEventSetFlags(release, COMMAND_FLAG);
            CGEventPost(SESSION_TAP, press);
            CGEventPost(SESSION_TAP, release);
        }
        for object in [press, release, source] {
            if !object.is_null() {
                CFRelease(object);
            }
        }
        if created { Ok(()) } else { Err("macOS blocked the paste.".to_string()) }
    }
}

fn copied_instead(board: &NSPasteboard, text: &str, reason: &str) -> Result<PasteOutcome, String> {
    write_text(board, text, false)?;
    Ok(PasteOutcome::CopiedInstead { reason: reason.to_string() })
}

/// Pastes into the app that was active when dictation stopped.
/// The text is never pasted into a different app than the one the person was using.
pub fn deliver(text: &str, target: Target) -> Result<PasteOutcome, String> {
    if target.is_mimi() {
        return Ok(PasteOutcome::StayedInMimi);
    }
    let _guard = PASTE_LOCK.lock().map_err(|_| "The clipboard is locked.".to_string())?;
    autoreleasepool(|_| {
        let board = NSPasteboard::generalPasteboard();
        if !target.still_active() {
            return copied_instead(&board, text, "You switched apps while Mimi was typing.");
        }
        if !unsafe { AXIsProcessTrusted() } {
            return copied_instead(&board, text, "Mimi needs the Accessibility permission to paste.");
        }

        let saved = snapshot(&board);
        write_text(&board, text, true)?;
        let written = board.changeCount();

        if let Err(reason) = send_paste_keys() {
            return copied_instead(&board, text, &reason);
        }

        thread::sleep(RESTORE_DELAY);
        // If the clipboard changed meanwhile, someone copied something new, which must be kept.
        if board.changeCount() == written {
            restore(&board, &saved)?;
        }
        Ok(PasteOutcome::Pasted)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // These use a private pasteboard, so the tests never touch the real clipboard.
    fn private_board() -> Retained<NSPasteboard> {
        NSPasteboard::pasteboardWithUniqueName()
    }

    fn discard(board: &NSPasteboard) {
        let _: () = unsafe { objc2::msg_send![board, releaseGlobally] };
    }

    fn text_on(board: &NSPasteboard) -> Option<String> {
        board.stringForType(unsafe { NSPasteboardTypeString }).map(|text| text.to_string())
    }

    fn has_temporary_mark(board: &NSPasteboard) -> bool {
        board.dataForType(&temporary_type()).is_some()
    }

    #[test]
    fn dictated_text_is_marked_temporary_and_a_normal_copy_is_not() {
        let board = private_board();
        write_text(&board, "uno\ndos ñandú", true).unwrap();
        assert_eq!(text_on(&board).as_deref(), Some("uno\ndos ñandú"));
        assert!(has_temporary_mark(&board));
        write_text(&board, "normal", false).unwrap();
        assert!(!has_temporary_mark(&board));
        discard(&board);
    }

    #[test]
    fn the_clipboard_comes_back_with_every_type_it_had() {
        let board = private_board();
        let custom = NSString::from_str("com.mimi.test-type");
        let item = NSPasteboardItem::new();
        item.setString_forType(&NSString::from_str("lo que había"), unsafe { NSPasteboardTypeString });
        item.setData_forType(&NSData::with_bytes(&[1, 2, 3]), &custom);
        replace_contents(&board, vec![item]).unwrap();

        let saved = snapshot(&board);
        write_text(&board, "dictado", true).unwrap();
        assert_eq!(text_on(&board).as_deref(), Some("dictado"));
        restore(&board, &saved).unwrap();

        assert_eq!(text_on(&board).as_deref(), Some("lo que había"));
        assert_eq!(board.dataForType(&custom).unwrap().to_vec(), vec![1, 2, 3]);
        assert!(has_temporary_mark(&board));
        discard(&board);
    }

    #[test]
    fn an_empty_clipboard_stays_empty() {
        let board = private_board();
        write_text(&board, "algo", true).unwrap();
        restore(&board, &[]).unwrap();
        assert_eq!(text_on(&board), None);
        discard(&board);
    }

    #[test]
    fn someone_copying_during_the_paste_is_visible_in_the_change_count() {
        let board = private_board();
        write_text(&board, "dictado", true).unwrap();
        let written = board.changeCount();
        write_text(&board, "copiado por la persona", false).unwrap();
        assert_ne!(board.changeCount(), written);
        discard(&board);
    }
}
