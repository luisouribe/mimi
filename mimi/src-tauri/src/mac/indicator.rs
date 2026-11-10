//! The small recording indicator. It floats above everything, never takes focus and lets
//! clicks pass through, so typing and clicking in the active app are never interrupted.

use objc2::MainThreadMarker;
use objc2_app_kit::{NSScreen, NSWindow, NSWindowCollectionBehavior};
use objc2_foundation::NSPoint;
use tauri::{AppHandle, Manager};

const LABEL: &str = "indicator";
const GAP_ABOVE_DOCK: f64 = 18.0;
/// NSStatusWindowLevel: above normal windows, the Dock included.
const FLOATING_LEVEL: isize = 25;

/// Mac windows can only be changed on the main thread.
fn on_indicator(app: &AppHandle, work: impl FnOnce(&NSWindow, MainThreadMarker) + Send + 'static) {
    let Some(window) = app.get_webview_window(LABEL) else { return };
    let _ = app.run_on_main_thread(move || {
        let Some(main_thread) = MainThreadMarker::new() else { return };
        let Ok(pointer) = window.ns_window() else { return };
        // Tauri keeps this window alive for as long as `window` exists.
        let native = unsafe { &*(pointer as *const NSWindow) };
        work(native, main_thread);
    });
}

/// Sets what keeps it out of the way. Runs once at startup.
pub fn prepare(app: &AppHandle) {
    on_indicator(app, |window, _| {
        window.setLevel(FLOATING_LEVEL);
        window.setIgnoresMouseEvents(true);
        window.setHasShadow(false);
        // On every desktop, and above apps in full screen.
        window.setCollectionBehavior(
            NSWindowCollectionBehavior::CanJoinAllSpaces
                | NSWindowCollectionBehavior::Stationary
                | NSWindowCollectionBehavior::IgnoresCycle
                | NSWindowCollectionBehavior::FullScreenAuxiliary,
        );
    });
}

/// Shows it at the bottom center of the screen the person is working on, without taking focus.
pub fn show(app: &AppHandle) {
    on_indicator(app, |window, main_thread| {
        if let Some(screen) = NSScreen::mainScreen(main_thread) {
            let work = screen.visibleFrame();
            let own = window.frame();
            window.setFrameOrigin(NSPoint::new(
                work.origin.x + (work.size.width - own.size.width) / 2.0,
                work.origin.y + GAP_ABOVE_DOCK,
            ));
        }
        window.orderFrontRegardless();
    });
}

pub fn hide(app: &AppHandle) {
    on_indicator(app, |window, _| window.orderOut(None));
}
