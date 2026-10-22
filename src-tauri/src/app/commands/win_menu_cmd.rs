//! 设置 → 右键菜单.
//!
//! The two commands exist on every platform because the settings window is one
//! window; only the Windows body has anything to write. On a Mac the page shows
//! the FinderSync situation instead and never asks about this menu, because the
//! corner drop box is how a Mac sends files.

use serde::Serialize;
use tauri::AppHandle;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WinMenuState {
    /// The switch as saved, not as desired.
    pub enabled: bool,
    /// Names currently written into the registry, so 「开」 on screen means
    /// 「菜单里真的有这些项」 rather than means 「数据库里有个 true」.
    pub entries: Vec<String>,
    /// The path Explorer was told to run — it changes when the app is moved.
    pub exe_path: String,
}

fn exe_path() -> String {
    std::env::current_exe()
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_default()
}

#[tauri::command]
pub fn get_win_menu_state(app_handle: AppHandle) -> WinMenuState {
    #[cfg(target_os = "windows")]
    {
        use crate::services::peer_transfer::win_menu;
        return WinMenuState {
            enabled: win_menu::is_enabled(&app_handle),
            entries: win_menu::entries(),
            exe_path: exe_path(),
        };
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = app_handle;
        WinMenuState {
            enabled: false,
            entries: vec![],
            exe_path: exe_path(),
        }
    }
}

#[tauri::command]
pub fn set_win_menu_enabled(app_handle: AppHandle, enabled: bool) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        return crate::services::peer_transfer::win_menu::set_enabled(&app_handle, enabled);
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (app_handle, enabled);
        Err("只有 Windows 有右键菜单".to_string())
    }
}
