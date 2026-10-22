//! The windows this app shows — and only after the user asks for one.
//!
//! There is no window at launch (`app.windows` is empty in tauri.conf.json, and
//! the bundle is `LSUIElement`), so everything here is built on demand and kept
//! afterwards: closing one of these hides it instead of destroying its WebView,
//! which makes the next open instant and keeps scroll position and filters.

use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

/// The label of the settings window; the page reads `?window=settings`.
pub const SETTINGS_LABEL: &str = "settings";

/// Show the settings window, on `tab` (`transfers`, `peers`, …) when given.
///
/// A window that already exists cannot change tabs through its query string —
/// it was loaded once — so the tab travels as an event instead.
pub fn open_settings(app: &AppHandle, tab: Option<&str>) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(SETTINGS_LABEL) {
        let _ = window.unminimize();
        show_and_focus(&window);
        if let Some(tab) = tab {
            let _ = app.emit_to(SETTINGS_LABEL, "settings-tab", tab);
        }
        return Ok(());
    }
    let url = match tab {
        Some(tab) => format!("index.html?window=settings&tab={tab}"),
        None => "index.html?window=settings".to_string(),
    };
    let window = WebviewWindowBuilder::new(app, SETTINGS_LABEL, WebviewUrl::App(url.into()))
        .title("TieZ 设置")
        .inner_size(960.0, 640.0)
        .min_inner_size(760.0, 520.0)
        .resizable(true)
        .center()
        .build()
        .map_err(|error| error.to_string())?;
    show_and_focus(&window);
    Ok(())
}

#[tauri::command]
pub fn open_settings_window(app: AppHandle, tab: Option<String>) -> Result<(), String> {
    open_settings(&app, tab.as_deref())
}

/// Whether a window label belongs to this app's own on-demand windows, so
/// closing one means "hide" rather than "quit the last window".
pub fn is_owned_window(label: &str) -> bool {
    label == SETTINGS_LABEL
}

fn show_and_focus(window: &WebviewWindow) {
    let _ = window.show();
    let _ = window.set_focus();
    activate_own_app(window.app_handle());
}

/// A menu-bar agent has no Dock tile and is not the active app when its window
/// appears, so the window would draw but refuse typing. The app itself has to
/// ask for activation before the fields take the keyboard.
fn activate_own_app(app: &AppHandle) {
    #[cfg(target_os = "macos")]
    {
        // Queued onto the main thread: AppKit activation is main-thread-only, and
        // this runs from tray handlers and commands alike.
        let app = app.clone();
        let _ = app.run_on_main_thread(move || {
            crate::infrastructure::macos_api::apps::activate_app_by_pid(std::process::id() as i32);
        });
    }
    #[cfg(not(target_os = "macos"))]
    let _ = app;
}
