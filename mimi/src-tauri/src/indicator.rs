//! The small recording indicator. It floats above everything, never takes focus and lets
//! clicks pass through, so typing and clicking in the active app are never interrupted.

use tauri::{AppHandle, Manager, WebviewWindow};
use windows_sys::Win32::Foundation::{HWND, RECT};
use windows_sys::Win32::Graphics::Gdi::{GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromWindow};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GWL_EXSTYLE, GetForegroundWindow, GetWindowLongPtrW, GetWindowRect, HWND_TOPMOST, SW_HIDE, SW_SHOWNOACTIVATE,
    SWP_NOACTIVATE, SWP_NOSIZE, SetWindowLongPtrW, SetWindowPos, ShowWindow, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    WS_EX_TRANSPARENT,
};

const LABEL: &str = "indicator";
const GAP_ABOVE_TASKBAR: i32 = 18;

fn window_handle(window: &WebviewWindow) -> Option<HWND> {
    window.hwnd().ok().map(|handle| handle.0 as HWND)
}

/// Sets the window styles that keep it out of the way. Runs once at startup.
pub fn prepare(app: &AppHandle) {
    let Some(window) = app.get_webview_window(LABEL) else { return };
    let Some(handle) = window_handle(&window) else { return };
    unsafe {
        let style = GetWindowLongPtrW(handle, GWL_EXSTYLE);
        let extra = (WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW | WS_EX_TRANSPARENT) as isize;
        SetWindowLongPtrW(handle, GWL_EXSTYLE, style | extra);
    }
}

/// Shows it at the bottom center of the screen the person is working on, without taking focus.
pub fn show(app: &AppHandle) {
    let Some(window) = app.get_webview_window(LABEL) else { return };
    let Some(handle) = window_handle(&window) else { return };
    unsafe {
        let mut info: MONITORINFO = std::mem::zeroed();
        info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        let monitor = MonitorFromWindow(GetForegroundWindow(), MONITOR_DEFAULTTONEAREST);
        let mut own: RECT = std::mem::zeroed();
        if GetMonitorInfoW(monitor, &mut info) != 0 && GetWindowRect(handle, &mut own) != 0 {
            let width = own.right - own.left;
            let height = own.bottom - own.top;
            let work = info.rcWork;
            let x = work.left + (work.right - work.left - width) / 2;
            let y = work.bottom - height - GAP_ABOVE_TASKBAR;
            SetWindowPos(handle, HWND_TOPMOST, x, y, 0, 0, SWP_NOSIZE | SWP_NOACTIVATE);
        }
        ShowWindow(handle, SW_SHOWNOACTIVATE);
    }
}

pub fn hide(app: &AppHandle) {
    let Some(window) = app.get_webview_window(LABEL) else { return };
    let Some(handle) = window_handle(&window) else { return };
    unsafe {
        ShowWindow(handle, SW_HIDE);
    }
}
