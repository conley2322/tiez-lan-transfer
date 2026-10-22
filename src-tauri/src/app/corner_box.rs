//! The screen-corner drop box: a capsule parked in a corner that opens into the
//! peer list while a file is being dragged over it.
//!
//! One window, two sizes, because a transparent always-on-top rectangle takes
//! the clicks away from whatever is underneath it — so it is only ever as big
//! as what it shows. The page does the drawing and the hit-testing (which
//! avatar the file is over); this module only decides where the rectangle sits
//! and how tall it is.

use serde::Serialize;
use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
};

use crate::database::DbState;
use crate::infrastructure::repository::settings_repo::SettingsRepository;

pub const CORNER_LABEL: &str = "corner";
/// On by default: on a Mac this is the send entry point, and a quiet capsule in
/// the corner is a much smaller ask than the right-click menu it stands in for.
pub const ENABLED_KEY: &str = "app.corner_box_enabled";
/// `top-right` | `top-left`. The right is the default even though the menu bar
/// is crowded there — that side is where the window close buttons live, so the
/// file being dragged already travels past it.
pub const SIDE_KEY: &str = "app.corner_box_side";
/// 40–100 percent. The page paints its own panel, so this is a number the page
/// reads rather than a window property: `set_opacity` on a transparent WebView
/// would fade the text and the border along with the background.
pub const OPACITY_KEY: &str = "app.corner_box_opacity";
/// When on, the box lists only the machines starred in 设备与用户. The full
/// roster stays in 设置, and the drop target keeps its size either way.
pub const FAVORITES_KEY: &str = "peer_corner_favorites_only";

/// The panel is a HUD the file has to be seen against; below this it is a
/// ghost nobody can aim at.
const MIN_OPACITY: u8 = 40;
const DEFAULT_OPACITY: u8 = 80;

/// The prototype's 34×120 capsule plus a transparent border, so a drag that
/// lands slightly off still enters.
const BAR_WIDTH: f64 = 46.0;
const BAR_HEIGHT: f64 = 132.0;
/// The 190px list panel from the prototype, with no transparent border: in the
/// expanded state the window *is* the panel.
const PANEL_WIDTH: f64 = 190.0;
/// The panel above the rows: 12px padding + the 「松手放到头像上」 line.
const PANEL_HEADER: f64 = 38.0;
/// One `.corner-row` in corner-box.css.
const ROW_HEIGHT: f64 = 40.0;
const PANEL_FOOTER: f64 = 12.0;
const EDGE_MARGIN: f64 = 12.0;
const TOP_MARGIN: f64 = 6.0;
/// `send_files_to_peers` caps a batch at 8 peers; a taller box than the screen
/// helps nobody, and the settings list is the place for the rest.
const MAX_ROWS: usize = 8;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CornerBoxState {
    pub enabled: bool,
    pub side: &'static str,
    pub opacity: u8,
    pub favorites_only: bool,
}

fn setting(app: &AppHandle, key: &str) -> Option<String> {
    let state = app.try_state::<DbState>()?;
    state.settings_repo.get(key).ok().flatten()
}

/// Anything but an explicit "false" keeps the box: a missing row means a
/// database from before this feature, which is not a user turning it off.
pub fn is_enabled(app: &AppHandle) -> bool {
    setting(app, ENABLED_KEY).as_deref() != Some("false")
}

pub fn side(app: &AppHandle) -> &'static str {
    match setting(app, SIDE_KEY).as_deref() {
        Some("top-left") => "top-left",
        _ => "top-right",
    }
}

/// The stored percentage, kept inside the range the slider can produce. A row
/// from a hand-edited database is a reason to clamp, not to crash.
pub fn opacity(app: &AppHandle) -> u8 {
    setting(app, OPACITY_KEY)
        .and_then(|text| text.trim().parse::<u8>().ok())
        .unwrap_or(DEFAULT_OPACITY)
        .clamp(MIN_OPACITY, 100)
}

/// Anything but an explicit "true" lists the whole roster: the box is the
/// default send path, and an empty list is a worse surprise than a long one.
pub fn favorites_only(app: &AppHandle) -> bool {
    setting(app, FAVORITES_KEY).as_deref() == Some("true")
}

fn clamp_opacity(value: u8) -> u8 {
    value.clamp(MIN_OPACITY, 100)
}

fn persist(app: &AppHandle, key: &str, value: &str) -> Result<(), String> {
    let state = app
        .try_state::<DbState>()
        .ok_or("设置存储还没准备好")?;
    state
        .settings_repo
        .set(key, value)
        .map_err(|error| error.to_string())
}

fn size_for(expanded: bool, rows: usize) -> (f64, f64) {
    if !expanded {
        return (BAR_WIDTH, BAR_HEIGHT);
    }
    let rows = rows.clamp(1, MAX_ROWS) as f64;
    (
        PANEL_WIDTH,
        PANEL_HEADER + ROW_HEIGHT * rows + PANEL_FOOTER,
    )
}

/// Move the window so the corner it is drawn against stays put while it grows:
/// the capsule and the list both hang from the same screen corner.
fn place(window: &WebviewWindow, width: f64, height: f64) -> Result<(), String> {
    let app = window.app_handle();
    let monitor = app
        .primary_monitor()
        .map_err(|error| error.to_string())?
        .ok_or("找不到主屏幕")?;
    let area = monitor.work_area();
    let scale = monitor.scale_factor().max(1.0);
    let width_px = (width * scale).round() as i32;
    let height_px = (height * scale).round() as i32;
    let margin_px = (EDGE_MARGIN * scale).round() as i32;
    let top_px = (TOP_MARGIN * scale).round() as i32;

    window
        .set_size(PhysicalSize::new(width_px as u32, height_px as u32))
        .map_err(|error| error.to_string())?;

    let left = side(app) == "top-left";
    let area_right = area.position.x + area.size.width as i32;
    let area_bottom = area.position.y + area.size.height as i32;
    let x = if left {
        area.position.x + margin_px
    } else {
        area_right - margin_px - width_px
    };
    let y = area.position.y + top_px;
    // A short window on a tall screen still has to stay inside it.
    let y = y.min(area_bottom - height_px - margin_px).max(area.position.y);
    window
        .set_position(PhysicalPosition::new(x, y))
        .map_err(|error| error.to_string())
}

/// Build the box if it is not there yet, and make sure it is on screen.
pub fn show(app: &AppHandle) -> Result<(), String> {
    let window = match app.get_webview_window(CORNER_LABEL) {
        Some(window) => window,
        None => WebviewWindowBuilder::new(
            app,
            CORNER_LABEL,
            WebviewUrl::App("index.html?window=corner".into()),
        )
        .title("TieZ 拖拽框")
        // Built hidden, then placed: a window at (0,0) for one frame is a flash
        // in the wrong corner, and this is the first thing the user sees.
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
        // A HUD the user never types into. Without this the box could take the
        // keyboard from Finder mid-drag.
        .focusable(false)
        .inner_size(BAR_WIDTH, BAR_HEIGHT)
        .build()
        .map_err(|error| error.to_string())?,
    };

    #[cfg(target_os = "macos")]
    crate::infrastructure::macos_api::window::configure_corner_drop_window(&window);

    place(&window, BAR_WIDTH, BAR_HEIGHT)?;
    window.show().map_err(|error| error.to_string())?;
    let _ = app.emit_to(CORNER_LABEL, "corner-side-changed", side(app));
    Ok(())
}

pub fn hide(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(CORNER_LABEL) {
        let _ = window.hide();
    }
}

/// Start at boot when the user has not turned it off.
pub fn start_if_enabled(app: &AppHandle) {
    if is_enabled(app) {
        if let Err(error) = show(app) {
            eprintln!(">>> [STARTUP] corner drop box did not open: {error}");
        }
    }
}

#[tauri::command]
pub fn get_corner_box_state(app: AppHandle) -> CornerBoxState {
    CornerBoxState {
        enabled: is_enabled(&app),
        side: side(&app),
        opacity: opacity(&app),
        favorites_only: favorites_only(&app),
    }
}

/// The machine list the box draws: the whole roster, or only the ⭐ in it.
/// Separate from `list_peers` because 设备与用户 shows the rest and the drop
/// target has no room for it.
#[tauri::command]
pub fn list_corner_peers(app_handle: AppHandle) -> Vec<crate::services::peer_transfer::PeerView> {
    crate::services::peer_transfer::roster::corner_views(&app_handle, favorites_only(&app_handle))
}

/// 「仅收藏用户」 flips the same window's list without rebuilding it.
#[tauri::command]
pub fn set_corner_box_favorites_only(app: AppHandle, favorites_only: bool) -> Result<(), String> {
    persist(&app, FAVORITES_KEY, if favorites_only { "true" } else { "false" })?;
    // The box keeps its rows until the page asks again, so a machine that was
    // just un-starred cannot still be a drop target.
    roster_changed(&app);
    Ok(())
}

/// The 不透明度 slider. The page re-reads `get_corner_box_state` on this event.
#[tauri::command]
pub fn set_corner_box_opacity(app: AppHandle, opacity: u8) -> Result<(), String> {
    let opacity = clamp_opacity(opacity);
    persist(&app, OPACITY_KEY, &opacity.to_string())?;
    if let Some(window) = app.get_webview_window(CORNER_LABEL) {
        let _ = window.eval(&format!(
            "document.documentElement.style.setProperty('--corner-opacity', '{opacity}')"
        ));
    }
    let _ = app.emit("corner-opacity-changed", opacity);
    Ok(())
}

/// Called when the roster, or the star on one of its entries, changed.
pub fn roster_changed(app: &AppHandle) {
    if app.get_webview_window(CORNER_LABEL).is_some() {
        let _ = app.emit_to(CORNER_LABEL, "corner-list-changed", ());
    }
}

/// The switch in 设置 → 角落拖拽框. Hidden rather than destroyed: the next
/// "enable" then costs one `show()`, not a fresh WebView.
#[tauri::command]
pub fn set_corner_box_enabled(app: AppHandle, enabled: bool) -> Result<(), String> {
    persist(&app, ENABLED_KEY, if enabled { "true" } else { "false" })?;
    if enabled {
        show(&app)?;
    } else {
        hide(&app);
    }
    app.emit("corner-box-enabled-changed", enabled)
        .map_err(|error| error.to_string())
}

/// 右上角 / 左上角. The window is re-anchored and told which way it now hangs.
#[tauri::command]
pub fn set_corner_box_side(app: AppHandle, side: String) -> Result<(), String> {
    let side = if side == "top-left" { "top-left" } else { "top-right" };
    persist(&app, SIDE_KEY, side)?;
    if let Some(window) = app.get_webview_window(CORNER_LABEL) {
        let size = window.inner_size().map_err(|error| error.to_string())?;
        let scale = window.scale_factor().unwrap_or(1.0).max(1.0);
        // `work_area()` is already physical, so the margins are multiplied here
        // rather than divided back into logical points.
        place(
            &window,
            size.width as f64 / scale,
            size.height as f64 / scale,
        )?;
    }
    let _ = app.emit_to(CORNER_LABEL, "corner-side-changed", side);
    Ok(())
}

/// What the page calls when a drag enters or leaves: the window grows into the
/// list of `rows` peers, then back to the capsule.
#[tauri::command]
pub fn set_corner_box_layout(app: AppHandle, expanded: bool, rows: usize) -> Result<(), String> {
    let Some(window) = app.get_webview_window(CORNER_LABEL) else {
        // Between 「暂停」 and the next enable there is no box to lay out; the
        // page only asks while it is on screen.
        return Ok(());
    };
    let (width, height) = size_for(expanded, rows);
    place(&window, width, height)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_box_grows_downward_from_the_same_corner() {
        // `place` keeps `y` fixed and only the right (or left) edge fixed, so the
        // two sizes share a corner. What the geometry test can pin is the size.
        assert_eq!(size_for(false, 0), (BAR_WIDTH, BAR_HEIGHT));
        assert_eq!(
            size_for(true, 1),
            (PANEL_WIDTH, PANEL_HEADER + ROW_HEIGHT + PANEL_FOOTER)
        );
    }

    #[test]
    fn a_crowded_roster_stops_at_the_send_limit() {
        assert_eq!(size_for(true, MAX_ROWS).1, size_for(true, 30).1);
    }

    #[test]
    fn every_peer_costs_exactly_one_row() {
        assert_eq!(
            size_for(true, 3).1 - size_for(true, 2).1,
            size_for(true, 2).1 - size_for(true, 1).1
        );
    }

    #[test]
    fn an_empty_list_still_gets_a_row_to_put_the_hint_in() {
        assert_eq!(size_for(true, 0), size_for(true, 1));
    }

    #[test]
    fn the_opacity_slider_cannot_paint_a_ghost_or_banish_the_text() {
        assert_eq!(clamp_opacity(80), 80);
        assert_eq!(clamp_opacity(0), MIN_OPACITY);
        assert_eq!(clamp_opacity(39), MIN_OPACITY);
        assert_eq!(clamp_opacity(100), 100);
    }
}
