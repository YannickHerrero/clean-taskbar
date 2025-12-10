//! Keyboard hook module
//!
//! Installs a low-level keyboard hook to track Windows key state.

use std::ptr::null_mut;
use std::sync::atomic::{AtomicPtr, Ordering};
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{VK_LWIN, VK_RWIN};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, PostMessageW, SetWindowsHookExW, UnhookWindowsHookEx,
    KBDLLHOOKSTRUCT, WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP, WM_USER,
};

pub const WM_WINKEY_DOWN: u32 = WM_USER + 100;
pub const WM_WINKEY_UP: u32 = WM_USER + 101;

static HOOK_HANDLE: AtomicPtr<std::ffi::c_void> = AtomicPtr::new(null_mut());
static NOTIFY_HWND: AtomicPtr<std::ffi::c_void> = AtomicPtr::new(null_mut());

/// Low-level keyboard hook callback
unsafe extern "system" fn keyboard_hook_proc(
    code: i32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if code >= 0 {
        let kbd = &*(lparam as *const KBDLLHOOKSTRUCT);
        let vk = kbd.vkCode as u16;

        if vk == VK_LWIN || vk == VK_RWIN {
            let hwnd = NOTIFY_HWND.load(Ordering::SeqCst) as HWND;
            let msg = match wparam as u32 {
                WM_KEYDOWN | WM_SYSKEYDOWN => Some(WM_WINKEY_DOWN),
                WM_KEYUP | WM_SYSKEYUP => Some(WM_WINKEY_UP),
                _ => None,
            };

            if let Some(m) = msg {
                PostMessageW(hwnd, m, 0, 0);
            }
        }
    }

    CallNextHookEx(null_mut(), code, wparam, lparam)
}

/// Install the keyboard hook
pub fn install(notify_hwnd: HWND) -> Result<(), &'static str> {
    unsafe {
        NOTIFY_HWND.store(notify_hwnd as *mut _, Ordering::SeqCst);

        let hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_hook_proc), null_mut(), 0);
        if hook.is_null() {
            return Err("Failed to install keyboard hook");
        }

        HOOK_HANDLE.store(hook, Ordering::SeqCst);
        Ok(())
    }
}

/// Uninstall the keyboard hook
pub fn uninstall() {
    unsafe {
        let hook = HOOK_HANDLE.swap(null_mut(), Ordering::SeqCst);
        if !hook.is_null() {
            UnhookWindowsHookEx(hook);
        }
    }
}
