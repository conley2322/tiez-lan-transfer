use crate::global_state::TASKBAR_CREATED_MSG;
use std::sync::atomic::Ordering;
#[cfg(target_os = "windows")]
use tauri::Manager;
#[cfg(target_os = "windows")]
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};

pub const LEGACY_PLACEHOLDER_MACHINE_ID: &str = "MAC-DEVICE-000000";
pub const LEGACY_PLACEHOLDER_ANON_ID: &str = "MAC-DEVICE-000000-0000-0000-0000-000000000000";
pub const LEGACY_ZERO_SUFFIX: &str = "-0000-0000-0000-000000000000";

fn hash_to_short_id(seed: &str) -> String {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    hasher.update(seed.as_bytes());
    let hex = format!("{:x}", hasher.finalize());
    hex.chars().take(8).collect()
}

fn fallback_machine_id() -> String {
    let mut seed = String::new();

    for key in ["COMPUTERNAME", "HOSTNAME", "USER", "USERNAME"] {
        if let Ok(value) = std::env::var(key) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                seed.push_str(trimmed);
                seed.push('|');
            }
        }
    }

    if seed.is_empty() {
        seed.push_str("tiez-device");
    }

    hash_to_short_id(&seed)
}

pub fn get_machine_id() -> String {
    match machine_uid::get() {
        Ok(machine_uid) if !machine_uid.trim().is_empty() => hash_to_short_id(machine_uid.trim()),
        Ok(_) => fallback_machine_id(),
        Err(e) => {
            eprintln!("[WARN] Failed to get machine UID: {}. Using fallback.", e);
            fallback_machine_id()
        }
    }
}

pub fn build_anon_id(machine_id: &str) -> String {
    machine_id.trim().to_string()
}

pub fn is_legacy_placeholder_anon_id(value: &str) -> bool {
    let trimmed = value.trim();
    trimmed == LEGACY_PLACEHOLDER_MACHINE_ID || trimmed == LEGACY_PLACEHOLDER_ANON_ID
}

pub fn normalize_anon_id(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() || is_legacy_placeholder_anon_id(trimmed) {
        return None;
    }

    if let Some(prefix) = trimmed.strip_suffix(LEGACY_ZERO_SUFFIX) {
        let normalized = prefix.trim();
        if normalized.is_empty() || is_legacy_placeholder_anon_id(normalized) {
            return None;
        }
        return Some(normalized.to_string());
    }

    Some(trimmed.to_string())
}

/// Settings key of the id sent with usage pings. Device-local: not synced,
/// kept through 「恢复默认设置」. Not app.anon_id, which is the cloud sync
/// device id.
pub const PING_ID_KEY: &str = "app.ping_id";

pub fn valid_ping_id(value: &str) -> Option<String> {
    let trimmed = value.trim();
    let valid = !trimmed.is_empty()
        && trimmed.len() <= 64
        && trimmed.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    valid.then(|| trimmed.to_string())
}

/// The id sent with usage pings, so each device counts once. A device that
/// pinged before keeps the id its window made then (`window_id`, from WebView
/// storage): changing it would count an update as a new device. New installs
/// use the machine id. Stored in the data folder from then on.
pub fn resolve_ping_id(
    repo: &impl crate::infrastructure::repository::settings_repo::SettingsRepository,
    window_id: Option<&str>,
) -> String {
    if let Some(id) = repo.get(PING_ID_KEY).unwrap_or(None).as_deref().and_then(valid_ping_id) {
        return id;
    }
    let id = window_id
        .and_then(valid_ping_id)
        .unwrap_or_else(|| build_anon_id(&get_machine_id()));
    let _ = repo.set(PING_ID_KEY, &id);
    id
}

pub fn is_same_device_id(id1: &str, id2: &str) -> bool {
    let n1 = normalize_anon_id(id1);
    let n2 = normalize_anon_id(id2);
    n1.is_some() && n1 == n2
}

pub fn same_anon_id(left: &str, right: &str) -> bool {
    match (normalize_anon_id(left), normalize_anon_id(right)) {
        (Some(l), Some(r)) => l == r,
        _ => is_same_device_id(left, right),
    }
}

#[cfg(target_os = "windows")]
const WM_WTSSESSION_CHANGE: u32 = 0x02B1;
#[cfg(target_os = "windows")]
const NOTIFY_FOR_THIS_SESSION: u32 = 0;

#[cfg(target_os = "windows")]
#[link(name = "wtsapi32")]
unsafe extern "system" {
    fn WTSRegisterSessionNotification(hwnd: HWND, flags: u32) -> i32;
}

/// Windows shows the tray icon again when it rebuilds the notification area:
/// after an Explorer restart, and after a Remote Desktop reconnect or unlock.
/// Hide it again if the user chose so. The shell may take a moment, so try a
/// few times. Session handling from #133.
#[cfg(target_os = "windows")]
fn rehide_tray_icon(reason: &'static str, delays_ms: &'static [u64]) {
    let Some(app_handle) = crate::GLOBAL_APP_HANDLE.get() else { return };
    let handle = app_handle.clone();
    std::thread::spawn(move || {
        for delay in delays_ms {
            std::thread::sleep(std::time::Duration::from_millis(*delay));
            let Some(settings) = handle.try_state::<crate::app_state::SettingsState>() else { return };
            if !settings.hide_tray_icon.load(Ordering::Relaxed) {
                return;
            }
            if let Some(tray) = handle.tray_by_id("main_tray") {
                let _ = tray.set_visible(false);
                println!(">>> [TRAY] {reason}: re-hiding tray icon per user setting.");
            }
        }
    });
}

/// Wake-from-sleep, Explorer-restart and session (unlock, Remote Desktop) notifications.
#[cfg(target_os = "windows")]
fn handle_system_message(msg: u32, wparam: WPARAM) {
    use windows::Win32::UI::WindowsAndMessaging::{
        PBT_APMRESUMEAUTOMATIC, PBT_APMRESUMECRITICAL, PBT_APMRESUMESUSPEND,
        WM_POWERBROADCAST,
    };
    if msg == WM_POWERBROADCAST
        && matches!(wparam.0 as u32, PBT_APMRESUMEAUTOMATIC | PBT_APMRESUMECRITICAL | PBT_APMRESUMESUSPEND)
    {
        crate::services::network_recovery::request_wake_recovery();
    }
    let taskbar_msg = TASKBAR_CREATED_MSG.load(Ordering::Relaxed);
    if msg != 0 && msg == taskbar_msg {
        rehide_tray_icon("Explorer restart", &[1500]);
    }
    // WTS_CONSOLE_CONNECT, WTS_REMOTE_CONNECT, WTS_SESSION_LOGON,
    // WTS_SESSION_UNLOCK, WTS_SESSION_REMOTE_CONTROL
    if msg == WM_WTSSESSION_CHANGE && matches!(wparam.0, 0x1 | 0x3 | 0x5 | 0x8 | 0x9) {
        rehide_tray_icon("session change", &[500, 1500, 3000]);
    }
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn system_event_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    handle_system_message(msg, wparam);
    unsafe { windows::Win32::UI::WindowsAndMessaging::DefWindowProcW(hwnd, msg, wparam, lparam) }
}

/// A hidden top-level window that lives as long as the process.
/// WM_POWERBROADCAST and the TaskbarCreated broadcast only reach top-level
/// windows (not message-only ones). They used to be read off the main window,
/// but light mode destroys and recreates that, which lost both — and while it
/// is destroyed (overnight, typically) a resume had no window to arrive at.
#[cfg(target_os = "windows")]
pub fn start_system_event_window() {
    std::thread::spawn(|| unsafe {
        use windows::core::w;
        use windows::Win32::Foundation::HINSTANCE;
        use windows::Win32::System::LibraryLoader::GetModuleHandleW;
        use windows::Win32::UI::WindowsAndMessaging::{
            CreateWindowExW, DispatchMessageW, GetMessageW, RegisterClassW, TranslateMessage,
            MSG, WNDCLASSW, WS_EX_TOOLWINDOW, WS_POPUP,
        };

        let Ok(module) = GetModuleHandleW(None) else {
            eprintln!("[system-events] GetModuleHandleW failed");
            return;
        };
        let instance = HINSTANCE(module.0);
        let class_name = w!("TieZSystemEvents");
        let class = WNDCLASSW {
            lpfnWndProc: Some(system_event_proc),
            hInstance: instance,
            lpszClassName: class_name,
            ..Default::default()
        };
        if RegisterClassW(&class) == 0 {
            eprintln!("[system-events] RegisterClassW failed");
            return;
        }
        // Never shown; WS_EX_TOOLWINDOW keeps it out of Alt+Tab and the taskbar.
        let hwnd = match CreateWindowExW(
            WS_EX_TOOLWINDOW,
            class_name,
            w!("TieZ system events"),
            WS_POPUP,
            0,
            0,
            0,
            0,
            None,
            None,
            Some(instance),
            None,
        ) {
            Ok(hwnd) => hwnd,
            Err(err) => {
                eprintln!("[system-events] CreateWindowExW failed: {err}");
                return;
            }
        };
        // Session changes arrive only for registered windows.
        if WTSRegisterSessionNotification(hwnd, NOTIFY_FOR_THIS_SESSION) == 0 {
            eprintln!("[system-events] WTSRegisterSessionNotification failed");
        }
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::repository::settings_repo::SqliteSettingsRepository;
    use std::sync::{Arc, Mutex};

    fn settings() -> SqliteSettingsRepository {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE settings(key TEXT PRIMARY KEY, value TEXT NOT NULL);")
            .unwrap();
        SqliteSettingsRepository::new(Arc::new(Mutex::new(conn)))
    }

    #[test]
    fn machine_id_derivation_stays_the_same() {
        // Sync and new installs' usage pings identify the device by this; a
        // different result would make every device look new.
        assert_eq!(hash_to_short_id("4C4C4544-0042-3510-8051-B4C04F4E4D32"), "9ac37b3e");
    }

    #[test]
    fn a_device_keeps_the_ping_id_its_window_made() {
        let repo = settings();

        assert_eq!(resolve_ping_id(&repo, Some("k3x9qa")), "k3x9qa");
        // Stored from then on: a cleared WebView storage doesn't change it.
        assert_eq!(resolve_ping_id(&repo, None), "k3x9qa");
        assert_eq!(resolve_ping_id(&repo, Some("other")), "k3x9qa");
    }

    #[test]
    fn a_new_install_pings_with_the_machine_id() {
        let machine = build_anon_id(&get_machine_id());

        assert_eq!(resolve_ping_id(&settings(), None), machine);
        assert_eq!(resolve_ping_id(&settings(), Some("not an id!")), machine);
    }
}
