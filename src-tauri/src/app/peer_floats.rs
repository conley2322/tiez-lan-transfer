//! The three cards that appear over the desktop: transfer progress, an
//! incoming pairing request, and a receive confirmation.
//!
//! Same reason the corner box is Rust's job: only the window layer knows where
//! the screen's bottom-right corner is, and these windows must be able to show
//! themselves while the app has no visible window at all. So the services call
//! `show_*` here, this module places and sizes the webview, and the pages inside
//! it read their own data (`list_pair_requests`, `list_receive_requests`,
//! `peer-transfer-progress`) and ask to be hidden again when they are done.
//!
//! Timing lives in the page on purpose. 「完成后约 3 秒自动消失」 is a rule about
//! what the user has finished reading, and the page is the only thing that sees
//! the whole stream; a Rust timer would have to be told the same facts anyway.

use serde::Serialize;
use tauri::{
    AppHandle, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
};

use crate::database::DbState;
use crate::infrastructure::repository::settings_repo::SettingsRepository;

/// The transfer card. Bottom-right unless 设置 moves it.
pub const PROGRESS_LABEL: &str = "progress";
/// 「对方请求配对」 with the verify word.
pub const PAIR_LABEL: &str = "pair";
/// The slide-to-accept card, only while 「接收前询问」 is on.
pub const RECEIVE_LABEL: &str = "receive";

/// 设置 → 文件传输 → 进度窗口.
pub const PROGRESS_SETTING: &str = "peer_progress_float";
/// `bottom-right` | `bottom-left` | `top-right`.
pub const POSITION_SETTING: &str = "peer_progress_position";
/// Seconds a finished card stays on screen.
pub const HOLD_SETTING: &str = "peer_progress_hold";
const DEFAULT_HOLD_SECONDS: u64 = 3;
/// The card is a countdown the user has to be able to read, and a transfer that
/// never finishes must not park a window over their desktop forever.
const MIN_HOLD_SECONDS: u64 = 1;
const MAX_HOLD_SECONDS: u64 = 30;

const EDGE_MARGIN: f64 = 16.0;
const STACK_GAP: f64 = 12.0;

/// Bottom-most first: a card that is already there pushes a newer one up rather
/// than covering it, so nothing can hide behind a transfer nobody has dismissed.
const STACK: [(f64, f64); 3] = [
    (330.0, 168.0), // progress
    (350.0, 300.0), // receive
    (330.0, 232.0), // pair
];

fn labels() -> [&'static str; 3] {
    [PROGRESS_LABEL, RECEIVE_LABEL, PAIR_LABEL]
}

fn size_of(label: &str) -> (f64, f64) {
    match label {
        RECEIVE_LABEL => STACK[1],
        PAIR_LABEL => STACK[2],
        _ => STACK[0],
    }
}

fn setting(app: &AppHandle, key: &str) -> Option<String> {
    let state = app.try_state::<DbState>()?;
    state.settings_repo.get(key).ok().flatten()
}

/// Off means the desktop is left alone: a transfer still runs, still gets
/// checked, and still lands in 传输记录.
pub fn progress_enabled(app: &AppHandle) -> bool {
    setting(app, PROGRESS_SETTING).as_deref() != Some("false")
}

pub fn position(app: &AppHandle) -> &'static str {
    match setting(app, POSITION_SETTING).as_deref() {
        Some("bottom-left") => "bottom-left",
        Some("top-right") => "top-right",
        _ => "bottom-right",
    }
}

fn hold_seconds(app: &AppHandle) -> u64 {
    clamp_hold(setting(app, HOLD_SETTING))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FloatState {
    pub position: &'static str,
    pub hold_seconds: u64,
    pub progress_enabled: bool,
}

/// What a float page reads before it starts counting.
#[tauri::command]
pub fn get_peer_float_state(app: AppHandle) -> FloatState {
    FloatState {
        position: position(&app),
        hold_seconds: hold_seconds(&app),
        progress_enabled: progress_enabled(&app),
    }
}

/// The window every float shares: no chrome, above everything, on every Space,
/// and never the thing that takes the keyboard from whatever the user is doing.
fn build(app: &AppHandle, label: &str, title: &str) -> Result<WebviewWindow, String> {
    if let Some(window) = app.get_webview_window(label) {
        return Ok(window);
    }
    let (width, height) = size_of(label);
    let window = WebviewWindowBuilder::new(
        app,
        label,
        WebviewUrl::App(format!("index.html?window={label}").into()),
    )
    .title(title)
    // Hidden until placed: a card that appears at (0,0) for one frame is a
    // flash in the wrong corner of somebody's desktop.
    .visible(false)
    .decorations(false)
    .transparent(true)
    .always_on_top(true)
    .visible_on_all_workspaces(true)
    .skip_taskbar(true)
    .resizable(false)
    .maximizable(false)
    .minimizable(false)
    .closable(false)
    .shadow(false)
    // Slide-to-accept and 重试 are both mouse; nothing here is typed into.
    .focusable(false)
    .inner_size(width, height)
    .build()
    .map_err(|error| error.to_string())?;

    #[cfg(target_os = "macos")]
    crate::infrastructure::macos_api::window::configure_hud_window(&window);

    Ok(window)
}

/// Move the floats to their corner, stacked from the edge inwards.
fn relayout(app: &AppHandle) {
    let Ok(monitor) = app.primary_monitor() else {
        return;
    };
    let Some(monitor) = monitor else {
        return;
    };
    let area = monitor.work_area();
    let scale = monitor.scale_factor().max(1.0);
    let corner = position(app);
    let left = corner == "bottom-left";
    let top = corner == "top-right";
    let margin = (EDGE_MARGIN * scale).round() as i32;
    let gap = (STACK_GAP * scale).round() as i32;

    // The lowest card sits on the edge; each one above it clears those below.
    let mut offset = 0;
    for label in labels() {
        let Some(window) = app.get_webview_window(label) else {
            continue;
        };
        if !window.is_visible().unwrap_or(false) {
            continue;
        }
        let (width, height) = size_of(label);
        let width_px = (width * scale).round() as i32;
        let height_px = (height * scale).round() as i32;
        let area_right = area.position.x + area.size.width as i32;
        let x = if left {
            area.position.x + margin
        } else {
            area_right - margin - width_px
        };
        let y = if top {
            area.position.y + margin + offset
        } else {
            let area_bottom = area.position.y + area.size.height as i32;
            area_bottom - margin - height_px - offset
        };
        let _ = window.set_size(PhysicalSize::new(width_px as u32, height_px as u32));
        let _ = window.set_position(PhysicalPosition::new(x, y));
        offset += height_px + gap;
    }
}

fn show(app: &AppHandle, label: &str, title: &str) {
    match build(app, label, title) {
        Ok(window) => {
            let _ = window.show();
            relayout(app);
        }
        Err(error) => eprintln!(">>> [FLOAT] {label} did not open: {error}"),
    }
}

/// Already on screen? Then there is nothing to do. The progress card is asked
/// this once per 512 KiB chunk, so it must not read the database every time.
fn already_up(app: &AppHandle, label: &str) -> bool {
    app.get_webview_window(label)
        .and_then(|window| window.is_visible().ok())
        .unwrap_or(false)
}

/// A card leaves when its page says it has been read; the stack closes up.
fn hide(app: &AppHandle, label: &str) {
    if let Some(window) = app.get_webview_window(label) {
        let _ = window.hide();
    }
    relayout(app);
}

/// One call for the three pages: only a float this module opened can be hidden,
/// so a page cannot make the settings window or the drop box vanish.
#[tauri::command]
pub fn hide_peer_float(app: AppHandle, label: String) -> Result<(), String> {
    let label = label.trim();
    if !labels().contains(&label) {
        return Err("不是本模块打开的窗口".to_string());
    }
    hide(&app, label);
    Ok(())
}

/// 设置 just changed the corner, the hold time, or the switch itself. A card
/// already on the desktop would otherwise keep its old position until the next
/// transfer, and one that was just switched off would outlive its setting.
#[tauri::command]
pub fn sync_peer_float_layout(app: AppHandle) {
    if !progress_enabled(&app) {
        hide(&app, PROGRESS_LABEL);
        return;
    }
    relayout(&app);
}

/// Summon the progress card for one event.
///
/// `status` decides whether to ask at all: "active" is emitted once per
/// 512 KiB chunk, and a card that is already showing needs no reminder — while
/// a large file would otherwise read the settings table thousands of times.
/// Every file also produces a "preparing" or "done"/"failed" event, so the card
/// still opens on the first byte and closes after the last.
pub fn on_transfer_progress(app: &AppHandle, status: &str) {
    if status == "active" || already_up(app, PROGRESS_LABEL) || !progress_enabled(app) {
        return;
    }
    show(app, PROGRESS_LABEL, "TieZ 传输进度");
}

pub fn on_pair_request(app: &AppHandle) {
    if already_up(app, PAIR_LABEL) {
        return;
    }
    show(app, PAIR_LABEL, "TieZ 配对请求");
}

pub fn on_receive_request(app: &AppHandle) {
    if already_up(app, RECEIVE_LABEL) {
        return;
    }
    show(app, RECEIVE_LABEL, "TieZ 接收确认");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hold(text: &str) -> Option<String> {
        Some(text.to_string())
    }

    #[test]
    fn a_hold_is_a_number_a_user_can_read() {
        assert_eq!(clamp_hold(hold("3")), 3);
        assert_eq!(clamp_hold(hold(" 10 ")), 10);
        assert_eq!(clamp_hold(hold("0")), MIN_HOLD_SECONDS);
        assert_eq!(clamp_hold(hold("3600")), MAX_HOLD_SECONDS);
        assert_eq!(clamp_hold(None), DEFAULT_HOLD_SECONDS);
        assert_eq!(clamp_hold(hold("很快")), DEFAULT_HOLD_SECONDS);
    }

    #[test]
    fn the_stack_is_the_three_cards_and_nothing_else() {
        assert_eq!(labels().len(), STACK.len());
        assert_eq!(size_of(PROGRESS_LABEL), STACK[0]);
        assert_eq!(size_of(RECEIVE_LABEL), STACK[1]);
        assert_eq!(size_of(PAIR_LABEL), STACK[2]);
        // A card nobody could dismiss would be a window stuck on the desktop.
        assert!(size_of(PAIR_LABEL).1 < 600.0);
    }

    #[test]
    fn only_a_float_this_module_owns_can_be_hidden() {
        // The command's guard, written so a typo in a page cannot hide 设置.
        for label in labels() {
            assert!(labels().contains(&label));
        }
        for not_a_float in ["main", "settings", "corner", ""] {
            assert!(!labels().contains(&not_a_float));
        }
    }
}

fn clamp_hold(text: Option<String>) -> u64 {
    let requested = text
        .and_then(|text| text.trim().parse::<u64>().ok())
        .unwrap_or(DEFAULT_HOLD_SECONDS);
    requested.clamp(MIN_HOLD_SECONDS, MAX_HOLD_SECONDS)
}
