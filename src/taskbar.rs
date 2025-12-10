//! Taskbar visibility control module
//!
//! Handles finding taskbar windows by class name and controlling their visibility.

use std::thread;
use std::time::Duration;
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::Shell::{ABM_SETSTATE, ABS_AUTOHIDE, APPBARDATA, SHAppBarMessage};
use windows_sys::Win32::UI::WindowsAndMessaging::{FindWindowW, ShowWindow, SW_HIDE, SW_SHOWNOACTIVATE};

/// Encodes a string as a null-terminated wide string
fn wide_string(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Finds the primary taskbar window handle
pub fn find_primary_taskbar() -> Option<HWND> {
    unsafe {
        let class_name = wide_string("Shell_TrayWnd");
        let hwnd = FindWindowW(class_name.as_ptr(), std::ptr::null());
        if hwnd.is_null() {
            None
        } else {
            Some(hwnd)
        }
    }
}

/// Sets the taskbar to auto-hide mode
pub fn set_autohide_mode(hwnd: HWND, enable: bool) {
    unsafe {
        let mut abd: APPBARDATA = std::mem::zeroed();
        abd.cbSize = std::mem::size_of::<APPBARDATA>() as u32;
        abd.hWnd = hwnd;
        abd.lParam = if enable { ABS_AUTOHIDE as isize } else { 0 };
        SHAppBarMessage(ABM_SETSTATE, &mut abd);
    }
}

/// Hides the taskbar window (with retry logic)
pub fn hide_taskbar(hwnd: HWND) {
    unsafe {
        for _ in 0..3 {
            ShowWindow(hwnd, SW_HIDE);
            thread::sleep(Duration::from_millis(50));
        }
    }
}

/// Shows the taskbar window without activating it
pub fn show_taskbar(hwnd: HWND) {
    unsafe {
        ShowWindow(hwnd, SW_SHOWNOACTIVATE);
    }
}

/// Initialize taskbar control - find handles and set auto-hide
pub fn init() -> Result<HWND, &'static str> {
    let hwnd = find_primary_taskbar().ok_or("Failed to find taskbar")?;
    set_autohide_mode(hwnd, true);
    hide_taskbar(hwnd);
    Ok(hwnd)
}

/// Cleanup - restore taskbar visibility
pub fn cleanup(hwnd: HWND) {
    show_taskbar(hwnd);
}
