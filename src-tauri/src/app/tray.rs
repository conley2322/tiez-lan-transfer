//! The menu-bar icon — the only entry point a background transfer agent has.
//!
//! The tray used to carry 「打开窗口」 for the clipboard panel. What it opens now
//! are the settings and the transfer log, plus a switch that refuses inbound
//! transfers without unpairing anything.

use tauri::menu::{CheckMenuItem, Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager};

use crate::global_state::{receiving_paused, set_receiving_paused};

/// The item whose check mark follows [`PAUSE_RECEIVING`](crate::global_state),
/// kept because the settings window can flip that switch too.
pub struct TrayMenu {
    pause: CheckMenuItem<tauri::Wry>,
}

/// Build the icon and its menu. Always visible: with no window at launch, an
/// app the user cannot reach is an app they can only force-quit.
pub fn setup(app: &tauri::App) {
    let settings_i = MenuItem::with_id(app, "settings", "设置…", true, None::<&str>).unwrap();
    let transfers_i = MenuItem::with_id(app, "transfers", "传输记录…", true, None::<&str>).unwrap();
    let pause_i = CheckMenuItem::with_id(
        app,
        "pause",
        "暂停接收",
        true,
        receiving_paused(),
        None::<&str>,
    )
    .unwrap();
    let quit_i = MenuItem::with_id(app, "quit", "退出", true, None::<&str>).unwrap();
    let menu = Menu::with_items(app, &[&settings_i, &transfers_i, &pause_i, &quit_i]).unwrap();
    let icon =
        tauri::image::Image::from_bytes(include_bytes!("../../icons/tray-icon.png")).unwrap();

    let mut builder = TrayIconBuilder::with_id("main_tray")
        .icon(icon)
        .tooltip("TieZ 局域网传输")
        .show_menu_on_left_click(false)
        .menu(&menu)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "settings" => {
                let _ = crate::app::floating_windows::open_settings(app, None);
            }
            "transfers" => {
                let _ = crate::app::floating_windows::open_settings(app, Some("transfers"));
            }
            "pause" => {
                set_receiving_paused(!receiving_paused());
                sync_pause_state(app);
            }
            "quit" => crate::app::light_mode::request_app_exit(app),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let _ = crate::app::floating_windows::open_settings(tray.app_handle(), None);
            }
        });

    #[cfg(target_os = "macos")]
    {
        builder = builder.icon_as_template(true);
    }

    let tray = builder.build(app).expect("Failed to build tray");
    let _ = tray.set_visible(true);
    app.manage(tray);
    app.manage(TrayMenu { pause: pause_i });
}

/// Push [`PAUSE_RECEIVING`](crate::global_state) onto the menu and the windows.
fn sync_pause_state(app: &AppHandle) {
    let paused = receiving_paused();
    let _ = app.emit("receiving-pause-changed", paused);
    if let Some(menu) = app.try_state::<TrayMenu>() {
        let _ = menu.pause.set_checked(paused);
    }
}

#[tauri::command]
pub fn set_pause_receiving(app: AppHandle, paused: bool) {
    set_receiving_paused(paused);
    sync_pause_state(&app);
}

#[tauri::command]
pub fn get_pause_receiving() -> bool {
    receiving_paused()
}
