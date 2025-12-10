//! System tray icon module
//!
//! Provides a tray icon with right-click quit menu.

use std::mem::size_of;
use std::ptr::{null, null_mut};
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT};
use windows_sys::Win32::UI::Shell::{
    NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW, Shell_NotifyIconW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, DestroyMenu, GetCursorPos, LoadIconW, SetForegroundWindow,
    TrackPopupMenu, IDI_APPLICATION, MF_STRING, TPM_BOTTOMALIGN, TPM_LEFTALIGN, WM_RBUTTONUP,
    WM_USER,
};

pub const WM_TRAYICON: u32 = WM_USER + 1;
pub const IDM_QUIT: usize = 1001;

/// Encodes a string as a null-terminated wide string
fn wide_string(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Adds the system tray icon
pub fn add_tray_icon(hwnd: HWND) -> bool {
    unsafe {
        let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
        nid.cbSize = size_of::<NOTIFYICONDATAW>() as u32;
        nid.hWnd = hwnd;
        nid.uID = 1;
        nid.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
        nid.uCallbackMessage = WM_TRAYICON;
        nid.hIcon = LoadIconW(null_mut(), IDI_APPLICATION);

        let tip = "Taskbar Hider - Right-click to quit";
        let tip_wide: Vec<u16> = tip.encode_utf16().chain(std::iter::once(0)).collect();
        let copy_len = tip_wide.len().min(128);
        nid.szTip[..copy_len].copy_from_slice(&tip_wide[..copy_len]);

        Shell_NotifyIconW(NIM_ADD, &nid) != 0
    }
}

/// Removes the system tray icon
pub fn remove_tray_icon(hwnd: HWND) {
    unsafe {
        let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
        nid.cbSize = size_of::<NOTIFYICONDATAW>() as u32;
        nid.hWnd = hwnd;
        nid.uID = 1;
        Shell_NotifyIconW(NIM_DELETE, &nid);
    }
}

/// Shows the context menu on right-click
pub fn show_context_menu(hwnd: HWND) {
    unsafe {
        let menu = CreatePopupMenu();
        if menu.is_null() {
            return;
        }
        let quit_text = wide_string("Quit");
        AppendMenuW(menu, MF_STRING, IDM_QUIT, quit_text.as_ptr());

        let mut pt = POINT { x: 0, y: 0 };
        GetCursorPos(&mut pt);

        SetForegroundWindow(hwnd);
        TrackPopupMenu(menu, TPM_BOTTOMALIGN | TPM_LEFTALIGN, pt.x, pt.y, 0, hwnd, null());
        DestroyMenu(menu);
    }
}

/// Handle tray icon messages in window proc
pub fn handle_tray_message(lparam: LPARAM, hwnd: HWND) -> Option<LRESULT> {
    let message = (lparam & 0xFFFF) as u32;
    match message {
        WM_RBUTTONUP => {
            show_context_menu(hwnd);
            Some(0)
        }
        _ => None,
    }
}
