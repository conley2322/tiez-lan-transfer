//! Starting TieZ normally while it already runs as administrator (the
//! elevated login task). tauri-plugin-single-instance can't open that
//! instance's mutex, so it started a second TieZ; and Windows drops
//! messages from a normal process to an elevated one's windows anyway.
//! Here the normal start finds the elevated instance and posts it a single
//! message that only brings its window forward, and the elevated instance
//! lets just that message through. The object names are the plugin's.

use std::sync::OnceLock;
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{CloseHandle, ERROR_ACCESS_DENIED, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::Threading::{GetCurrentProcessId, OpenMutexW, MUTEX_ALL_ACCESS};
use windows::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass};
use windows::Win32::UI::WindowsAndMessaging::{
    ChangeWindowMessageFilterEx, FindWindowW, GetWindowThreadProcessId, PostMessageW,
    RegisterWindowMessageW, MSGFLT_ALLOW,
};

static APP: OnceLock<tauri::AppHandle> = OnceLock::new();

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

fn show_message() -> u32 {
    static MESSAGE: OnceLock<u32> = OnceLock::new();
    *MESSAGE.get_or_init(|| unsafe { RegisterWindowMessageW(w!("TieZ.ShowWindow")) })
}

/// The plugin's message window of the running instance.
fn find_instance_window(identifier: &str) -> Option<HWND> {
    let class = wide(&format!("{identifier}-sic"));
    let name = wide(&format!("{identifier}-siw"));
    unsafe { FindWindowW(PCWSTR(class.as_ptr()), PCWSTR(name.as_ptr())) }
        .ok()
        .filter(|hwnd| !hwnd.0.is_null())
}

/// Run before Tauri. True when TieZ already runs as administrator: it has
/// been asked to show its window (unless this is a login start), and this
/// process should exit.
pub fn hand_off_to_elevated_instance(identifier: &str, args: &[String]) -> bool {
    let mutex = wide(&format!("{identifier}-sim"));
    match unsafe { OpenMutexW(MUTEX_ALL_ACCESS, false, PCWSTR(mutex.as_ptr())) } {
        // Running, and the plugin can reach it itself.
        Ok(handle) => {
            unsafe {
                let _ = CloseHandle(handle);
            }
            return false;
        }
        // Running, out of reach: an elevated instance.
        Err(error) if error.code() == ERROR_ACCESS_DENIED.to_hresult() => {}
        // Not running.
        Err(_) => return false,
    }
    let login_start = args.iter().any(|arg| arg == "--minimized" || arg == "--autostart");
    if !login_start {
        if let Some(hwnd) = find_instance_window(identifier) {
            unsafe {
                let _ = PostMessageW(Some(hwnd), show_message(), WPARAM(0), LPARAM(0));
            }
        }
    }
    true
}

/// In TieZ running as administrator: let a normal start's show message
/// through. Must run on the main thread, which owns the plugin's window.
pub fn accept_show_from_normal_starts(app: &tauri::AppHandle, identifier: &str) {
    if !crate::app::commands::system_cmd::check_is_admin() {
        return;
    }
    let Some(hwnd) = find_instance_window(identifier) else {
        return;
    };
    let mut pid = 0u32;
    unsafe {
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
    }
    if pid != unsafe { GetCurrentProcessId() } || APP.set(app.clone()).is_err() {
        return;
    }
    unsafe {
        if SetWindowSubclass(hwnd, Some(instance_window_proc), 1, 0).as_bool() {
            let _ = ChangeWindowMessageFilterEx(hwnd, show_message(), MSGFLT_ALLOW, None);
        }
    }
}

unsafe extern "system" fn instance_window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _id: usize,
    _data: usize,
) -> LRESULT {
    if msg == show_message() {
        // Same as the plugin's callback in main.rs.
        if let Some(app) = APP.get() {
            let app = app.clone();
            std::thread::spawn(move || {
                let _ = crate::app::window_manager::focus_clipboard_window(app);
            });
        }
        return LRESULT(0);
    }
    unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
}
