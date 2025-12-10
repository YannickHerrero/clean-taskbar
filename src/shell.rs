//! Shell hook module
//!
//! Creates a hidden window and registers for shell events to detect
//! when the Start menu or other system windows are active.

use std::mem::size_of;
use std::ptr::{null, null_mut};
use windows_sys::Win32::Foundation::{HINSTANCE, HWND, LPARAM, WPARAM};
use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, GetClassNameW, RegisterClassExW, RegisterWindowMessageW,
    HWND_MESSAGE, WNDCLASSEXW, WS_OVERLAPPED,
};

// Shell hook message codes
pub const HSHELL_WINDOWACTIVATED: u32 = 4;
pub const HSHELL_RUDEAPPACTIVATED: u32 = 0x8004;

// Window classes that should keep taskbar visible
const SYSTEM_WINDOW_CLASSES: &[&str] = &[
    "Windows.UI.Core.CoreWindow",
    "Shell_TrayWnd",
    "Shell_SecondaryTrayWnd",
    "TopLevelWindowForOverflowXamlIsland",
    "XamlExplorerHostIslandWindow",
];

/// Encodes a string as a null-terminated wide string
fn wide_string(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Checks if the given window class should keep the taskbar visible
pub fn is_system_window(hwnd: HWND) -> bool {
    unsafe {
        let mut class_name = [0u16; 256];
        let len = GetClassNameW(hwnd, class_name.as_mut_ptr(), 256);
        if len == 0 {
            return false;
        }

        let class_str = String::from_utf16_lossy(&class_name[..len as usize]);
        SYSTEM_WINDOW_CLASSES.iter().any(|&c| class_str == c)
    }
}

/// Dynamically load and call RegisterShellHookWindow
fn register_shell_hook_window(hwnd: HWND) -> bool {
    unsafe {
        let dll_name = wide_string("user32.dll");
        let user32 = LoadLibraryW(dll_name.as_ptr());
        if user32.is_null() {
            return false;
        }

        let proc = GetProcAddress(user32, b"RegisterShellHookWindow\0".as_ptr());
        if proc.is_none() {
            return false;
        }

        type RegisterShellHookWindowFn = unsafe extern "system" fn(HWND) -> i32;
        let func: RegisterShellHookWindowFn = std::mem::transmute(proc.unwrap());
        func(hwnd) != 0
    }
}

/// Create the shell hook window and register for events
pub fn create_shell_hook_window(instance: HINSTANCE) -> Result<(HWND, u32), &'static str> {
    unsafe {
        let class_name = wide_string("TaskbarHiderShellHook");
        let wc = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            style: 0,
            lpfnWndProc: Some(DefWindowProcW),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: instance,
            hIcon: null_mut(),
            hCursor: null_mut(),
            hbrBackground: null_mut(),
            lpszMenuName: null(),
            lpszClassName: class_name.as_ptr(),
            hIconSm: null_mut(),
        };

        if RegisterClassExW(&wc) == 0 {
            return Err("Failed to register shell hook window class");
        }

        let window_name = wide_string("TaskbarHiderShellHook");
        let hwnd = CreateWindowExW(
            0,
            class_name.as_ptr(),
            window_name.as_ptr(),
            WS_OVERLAPPED,
            0,
            0,
            0,
            0,
            HWND_MESSAGE,
            null_mut(),
            instance,
            null(),
        );

        if hwnd.is_null() {
            return Err("Failed to create shell hook window");
        }

        let shellhook_msg = wide_string("SHELLHOOK");
        let shell_hook_msg = RegisterWindowMessageW(shellhook_msg.as_ptr());
        if shell_hook_msg == 0 {
            return Err("Failed to register shell hook message");
        }

        if !register_shell_hook_window(hwnd) {
            return Err("Failed to register shell hook window");
        }

        Ok((hwnd, shell_hook_msg))
    }
}

/// Handle shell hook messages - returns true if a system window is now active
pub fn handle_shell_message(wparam: WPARAM, lparam: LPARAM) -> bool {
    let code = wparam as u32;

    match code {
        HSHELL_WINDOWACTIVATED | HSHELL_RUDEAPPACTIVATED => {
            let activated_hwnd = lparam as HWND;
            is_system_window(activated_hwnd)
        }
        _ => false,
    }
}
