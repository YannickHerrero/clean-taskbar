//! Taskbar Hider - Main Entry Point
//!
//! A minimal Windows utility that hides the taskbar and shows it only when
//! the Windows key is held or the Start menu is active.

#![windows_subsystem = "windows"]

mod hooks;
mod shell;
mod taskbar;
mod tray;

use std::mem::size_of;
use std::ptr::{null, null_mut};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, KillTimer,
    PostQuitMessage, RegisterClassExW, RegisterWindowMessageW, SetTimer, TranslateMessage,
    HWND_MESSAGE, MSG, WNDCLASSEXW, WM_COMMAND, WM_DESTROY, WM_TIMER, WS_OVERLAPPED,
};

// Timing constants
const WIN_KEY_DELAY_MS: u64 = 400;
const TIMER_ID_HIDE_TASKBAR: usize = 1;

// Global state
static TASKBAR_SHOULD_BE_VISIBLE: AtomicBool = AtomicBool::new(false);
static WIN_KEY_HELD: AtomicBool = AtomicBool::new(false);
static SYSTEM_WINDOW_ACTIVE: AtomicBool = AtomicBool::new(false);

static mut TASKBAR_HWND: HWND = null_mut();
static mut MAIN_HWND: HWND = null_mut();
static mut SHELL_HOOK_MSG: u32 = 0;
static mut TASKBAR_CREATED_MSG: u32 = 0;
static mut WIN_KEY_RELEASE_TIME: u64 = 0;

/// Encodes a string as a null-terminated wide string
fn wide_string(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
    }
}

fn run() -> Result<(), &'static str> {
    unsafe {
        let instance = GetModuleHandleW(null());
        if instance.is_null() {
            return Err("Failed to get module handle");
        }

        // Initialize taskbar control
        TASKBAR_HWND = taskbar::init()?;

        // Create main message window
        let class_name = wide_string("TaskbarHiderMain");
        let wc = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            style: 0,
            lpfnWndProc: Some(window_proc),
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

        RegisterClassExW(&wc);

        let window_name = wide_string("TaskbarHider");
        MAIN_HWND = CreateWindowExW(
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

        if MAIN_HWND.is_null() {
            return Err("Failed to create main window");
        }

        // Create shell hook window
        let (_shell_hwnd, shell_msg) = shell::create_shell_hook_window(instance)?;
        SHELL_HOOK_MSG = shell_msg;

        // Register for TaskbarCreated message (Explorer restart detection)
        let taskbar_created = wide_string("TaskbarCreated");
        TASKBAR_CREATED_MSG = RegisterWindowMessageW(taskbar_created.as_ptr());

        // Install keyboard hook
        hooks::install(MAIN_HWND)?;

        // Add tray icon
        if !tray::add_tray_icon(MAIN_HWND) {
            return Err("Failed to add tray icon");
        }

        // Message loop
        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        // Cleanup
        cleanup();

        Ok(())
    }
}

fn cleanup() {
    unsafe {
        hooks::uninstall();
        tray::remove_tray_icon(MAIN_HWND);
        taskbar::cleanup(TASKBAR_HWND);
    }
}

fn update_taskbar_visibility() {
    unsafe {
        let should_show = WIN_KEY_HELD.load(Ordering::SeqCst)
            || SYSTEM_WINDOW_ACTIVE.load(Ordering::SeqCst)
            || is_within_delay_period();

        let currently_visible = TASKBAR_SHOULD_BE_VISIBLE.load(Ordering::SeqCst);

        if should_show && !currently_visible {
            taskbar::show_taskbar(TASKBAR_HWND);
            TASKBAR_SHOULD_BE_VISIBLE.store(true, Ordering::SeqCst);
        } else if !should_show && currently_visible {
            taskbar::hide_taskbar(TASKBAR_HWND);
            TASKBAR_SHOULD_BE_VISIBLE.store(false, Ordering::SeqCst);
        }
    }
}

fn is_within_delay_period() -> bool {
    unsafe {
        if WIN_KEY_RELEASE_TIME == 0 {
            return false;
        }

        let now = get_current_time_ms();
        now < WIN_KEY_RELEASE_TIME + WIN_KEY_DELAY_MS
    }
}

fn get_current_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        // Tray icon messages
        m if m == tray::WM_TRAYICON => {
            if let Some(result) = tray::handle_tray_message(lparam, hwnd) {
                return result;
            }
        }

        // Menu command (Quit)
        WM_COMMAND => {
            if wparam == tray::IDM_QUIT {
                PostQuitMessage(0);
                return 0;
            }
        }

        // Windows key down
        m if m == hooks::WM_WINKEY_DOWN => {
            WIN_KEY_HELD.store(true, Ordering::SeqCst);
            WIN_KEY_RELEASE_TIME = 0;
            update_taskbar_visibility();
            return 0;
        }

        // Windows key up
        m if m == hooks::WM_WINKEY_UP => {
            WIN_KEY_HELD.store(false, Ordering::SeqCst);
            WIN_KEY_RELEASE_TIME = get_current_time_ms();
            SetTimer(hwnd, TIMER_ID_HIDE_TASKBAR, WIN_KEY_DELAY_MS as u32 + 50, None);
            update_taskbar_visibility();
            return 0;
        }

        // Timer for delayed hide
        WM_TIMER => {
            if wparam == TIMER_ID_HIDE_TASKBAR {
                KillTimer(hwnd, TIMER_ID_HIDE_TASKBAR);
                update_taskbar_visibility();
            }
            return 0;
        }

        // Shell hook messages
        m if SHELL_HOOK_MSG != 0 && m == SHELL_HOOK_MSG => {
            let is_system = shell::handle_shell_message(wparam, lparam);
            SYSTEM_WINDOW_ACTIVE.store(is_system, Ordering::SeqCst);
            update_taskbar_visibility();
            return 0;
        }

        // TaskbarCreated - Explorer restarted
        m if TASKBAR_CREATED_MSG != 0 && m == TASKBAR_CREATED_MSG => {
            if let Ok(h) = taskbar::init() {
                TASKBAR_HWND = h;
            }
            tray::add_tray_icon(MAIN_HWND);
            return 0;
        }

        WM_DESTROY => {
            PostQuitMessage(0);
            return 0;
        }

        _ => {}
    }

    DefWindowProcW(hwnd, msg, wparam, lparam)
}
