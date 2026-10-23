//! The screen-corner drop box: a small icon that appears while a file is being
//! dragged and opens into the peer list when the file is over it.
//!
//! One window, two sizes, because a transparent always-on-top rectangle takes
//! the clicks away from whatever is underneath it — so it is hidden until a
//! drag starts, and only ever as big as what it shows. The page does the drawing
//! and the hit-testing (which avatar the file is over); this module decides
//! where the rectangle sits, how tall it is, and when it is on screen at all.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use serde::Serialize;
use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
};

use crate::database::DbState;
use crate::infrastructure::repository::settings_repo::SettingsRepository;

pub const CORNER_LABEL: &str = "corner";
/// On by default: on a Mac this is the send entry point, and a quiet icon that
/// only appears during a drag is a much smaller ask than the right-click menu
/// it stands in for.
pub const ENABLED_KEY: &str = "app.corner_box_enabled";
/// `follow` | `top-right` | `top-left`. Following the drag is the default: on
/// a large screen, having to travel to a corner while holding a file was the
/// one thing the box got wrong, so it now appears beside the file instead.
pub const SIDE_KEY: &str = "app.corner_box_side";
/// 40–100 percent. The page paints its own panel, so this is a number the page
/// reads rather than a window property: `set_opacity` on a transparent WebView
/// would fade the text and the border along with the background.
pub const OPACITY_KEY: &str = "app.corner_box_opacity";
/// When on, the box lists only the machines starred in 设备与用户. The full
/// roster stays in 设置, and the drop target keeps its size either way.
pub const FAVORITES_KEY: &str = "peer_corner_favorites_only";
/// Shake the mouse during a drag to open the panel without first hovering the
/// icon. On by default: the panel is otherwise unreachable while the native
/// drag preview covers the corner.
pub const SHELF_KEY: &str = "app.corner_shelf_enabled";

/// The panel is a HUD the file has to be seen against; below this it is a
/// ghost nobody can aim at.
const MIN_OPACITY: u8 = 40;
const DEFAULT_OPACITY: u8 = 80;

/// The icon is drawn at 40×40 with a transparent border around it, so a drag
/// that lands slightly off still enters.
const ICON_SIZE: f64 = 48.0;
/// Wide enough for a full device name and a 34px avatar. The native drag
/// preview is drawn by Finder and cannot be made smaller, so the targets have
/// to be bigger than it rather than the other way round.
const PANEL_WIDTH: f64 = 224.0;
/// The panel above the rows: 10px padding + the 14px 「松手放到头像上」 line +
/// the 10px gap under it.
const PANEL_HEADER: f64 = 34.0;
/// One `.corner-row` in corner-box.css.
const ROW_HEIGHT: f64 = 48.0;
const PANEL_FOOTER: f64 = 10.0;
const EDGE_MARGIN: f64 = 12.0;
const TOP_MARGIN: f64 = 6.0;
/// How far the icon sits from the pointer in follow mode. Far enough that
/// showing the box does not put it under the dragged file (which would expand
/// the panel before the user asked), close enough to reach with one flick.
const DRAG_GAP: f64 = 18.0;
/// Where the drag started, in physical desktop pixels. Cleared by the next
/// drag; `None` (a failed pointer read) parks the box in the corner.
static DRAG_ANCHOR: Mutex<Option<(f64, f64)>> = Mutex::new(None);
/// `send_files_to_peers` caps a batch at 8 peers; a taller box than the screen
/// helps nobody, and the settings list is the place for the rest.
const MAX_ROWS: usize = 8;
/// How long the page gets to show the ✓ / ! after a drag ends before Rust
/// hides the window anyway — the page normally asks first, sooner.
const END_NOTICE_MS: u64 = 3000;
/// Bumped for every drag so a fallback scheduled by an earlier drag cannot
/// close the icon of a newer one.
static DRAG_GENERATION: AtomicU64 = AtomicU64::new(0);

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CornerBoxState {
    pub enabled: bool,
    pub side: &'static str,
    pub opacity: u8,
    pub favorites_only: bool,
    pub shelf_enabled: bool,
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
        Some("top-right") => "top-right",
        // Missing means an update from the corner-only version; following the
        // drag is the behaviour those users asked for.
        _ => "follow",
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

/// Anything but an explicit "false" keeps the shake gesture: it is how the
/// panel is revealed while the drag preview covers the icon.
pub fn shelf_enabled(app: &AppHandle) -> bool {
    setting(app, SHELF_KEY).as_deref() != Some("false")
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
        return (ICON_SIZE, ICON_SIZE);
    }
    let rows = rows.clamp(1, MAX_ROWS) as f64;
    (
        PANEL_WIDTH,
        PANEL_HEADER + ROW_HEIGHT * rows + PANEL_FOOTER,
    )
}

/// Move the window so the corner it is drawn against stays put while it grows:
/// the icon and the list both hang from the same screen corner.
fn place(window: &WebviewWindow, width: f64, height: f64) -> Result<(), String> {
    let app = window.app_handle();
    if side(app) == "follow" {
        if let Some(anchor) = DRAG_ANCHOR.lock().ok().and_then(|slot| *slot) {
            return place_at_drag(app, window, anchor, width, height);
        }
    }
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

/// Follow mode: put the box beside the drag that opened it, on the screen the
/// drag started on.
fn place_at_drag(
    app: &AppHandle,
    window: &WebviewWindow,
    anchor: (f64, f64),
    width: f64,
    height: f64,
) -> Result<(), String> {
    let monitor = app
        .monitor_from_point(anchor.0, anchor.1)
        .ok()
        .flatten()
        .or_else(|| app.primary_monitor().ok().flatten())
        .ok_or("找不到屏幕")?;
    let scale = monitor.scale_factor().max(1.0);
    let area = monitor.work_area();
    let width_px = (width * scale).round() as i32;
    let height_px = (height * scale).round() as i32;
    let margin_px = (EDGE_MARGIN * scale).round() as i32;
    let gap_px = (DRAG_GAP * scale).round() as i32;
    let left = area.position.x;
    let top = area.position.y;
    let right = left + area.size.width as i32;
    let bottom = top + area.size.height as i32;

    // The icon decides where the box sits; the expanded panel keeps the same
    // top-left and only gives way to the screen edge, so opening it cannot
    // yank it out from under the pointer that just entered it.
    let (icon_x, icon_y) = drag_origin(
        (anchor.0.round() as i32, anchor.1.round() as i32),
        (ICON_SIZE * scale).round() as i32,
        (left, top, right, bottom),
        gap_px,
    );
    let x = icon_x
        .min(right - margin_px - width_px)
        .max(left + margin_px);
    let y = icon_y
        .min(bottom - margin_px - height_px)
        .max(top + margin_px);

    window
        .set_size(PhysicalSize::new(
            width_px.max(1) as u32,
            height_px.max(1) as u32,
        ))
        .map_err(|error| error.to_string())?;
    window
        .set_position(PhysicalPosition::new(x, y))
        .map_err(|error| error.to_string())
}

/// Top-left for the icon beside a drag point: lower right, flipped to the
/// other side of the pointer when the screen ends.
fn drag_origin(
    anchor: (i32, i32),
    icon_px: i32,
    area: (i32, i32, i32, i32),
    gap_px: i32,
) -> (i32, i32) {
    let (left, top, right, bottom) = area;
    let (ax, ay) = anchor;
    let mut x = ax + gap_px;
    if x + icon_px > right {
        x = ax - gap_px - icon_px;
    }
    let mut y = ay + gap_px;
    if y + icon_px > bottom {
        y = ay - gap_px - icon_px;
    }
    // A screen narrower than the icon: the clamp is a last resort.
    (
        x.clamp(left, (right - icon_px).max(left)),
        y.clamp(top, (bottom - icon_px).max(top)),
    )
}

/// Build the box if it is not there yet. Never shown from here: at boot the
/// page has to be loading while nothing is on screen, and during a drag the
/// show happens one call later.
fn ensure(app: &AppHandle) -> Result<WebviewWindow, String> {
    if let Some(window) = app.get_webview_window(CORNER_LABEL) {
        return Ok(window);
    }
    let window = WebviewWindowBuilder::new(
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
    .inner_size(ICON_SIZE, ICON_SIZE)
    .build()
    .map_err(|error| error.to_string())?;

    #[cfg(target_os = "macos")]
    crate::infrastructure::macos_api::window::configure_corner_drop_window(&window);

    Ok(window)
}

/// Build it (if needed) and park it, hidden, at the collapsed size and corner.
/// Doing this at boot or on enable makes the first drag cost a `show()`, not a
/// fresh WebView that cannot be aimed at until it has loaded.
pub fn prepare(app: &AppHandle) -> Result<(), String> {
    let window = ensure(app)?;
    place(&window, ICON_SIZE, ICON_SIZE)?;
    let _ = app.emit_to(CORNER_LABEL, "corner-side-changed", side(app));
    Ok(())
}

/// The native drag monitor saw a file leave a window. This is the only path
/// that puts the box on screen; the page keeps it a small icon until the file
/// is over it. `anchor` is where the pointer was (physical desktop pixels), so
/// the box opens next to the file rather than in a corner.
pub fn show_for_drag(app: &AppHandle, anchor: Option<(f64, f64)>) -> Result<(), String> {
    if !is_enabled(app) {
        return Ok(());
    }
    DRAG_GENERATION.fetch_add(1, Ordering::SeqCst);
    let window = ensure(app)?;
    if let Ok(mut slot) = DRAG_ANCHOR.lock() {
        *slot = anchor;
    }
    // A previous drag may have ended with the page still expanded.
    place(&window, ICON_SIZE, ICON_SIZE)?;
    window.show().map_err(|error| error.to_string())?;
    let _ = app.emit_to(CORNER_LABEL, "corner-drag-active", true);
    Ok(())
}

/// The drag is over — dropped, cancelled, or the button came up over another
/// app. The page keeps a ✓ / ! icon up for a moment and then hides the window;
/// the timeout is the guard for a page that never heard this event.
pub fn end_drag(app: &AppHandle) {
    let generation = DRAG_GENERATION.load(Ordering::SeqCst);
    let _ = app.emit_to(CORNER_LABEL, "corner-drag-active", false);
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_millis(END_NOTICE_MS)).await;
        if DRAG_GENERATION.load(Ordering::SeqCst) != generation {
            return;
        }
        let hide_handle = handle.clone();
        let _ = handle.run_on_main_thread(move || hide(&hide_handle));
    });
}

pub fn hide(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(CORNER_LABEL) {
        let _ = window.hide();
    }
}

/// Called by the page once its ✓ / ! has been visible long enough.
#[tauri::command]
pub fn hide_corner_box(app: AppHandle) {
    hide(&app);
}

/// Load the page at boot, hidden, when the user has not turned the box off.
pub fn start_if_enabled(app: &AppHandle) {
    if is_enabled(app) {
        if let Err(error) = prepare(app) {
            eprintln!(">>> [STARTUP] corner drop box did not load: {error}");
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
        shelf_enabled: shelf_enabled(&app),
    }
}

/// The native drag monitor saw the mouse wiggle twice during a drag. The page
/// owns the drawing and the size, so this only tells it to expand; it answers
/// with `set_corner_box_layout`, the same call a hover uses. `anchor` is where
/// the wiggle happened, so the shelf pops up under the gesture.
pub fn open_shelf(app: &AppHandle, anchor: Option<(f64, f64)>) -> Result<(), String> {
    if !is_enabled(app) || !shelf_enabled(app) {
        return Ok(());
    }
    let window = ensure(app)?;
    if let Some(anchor) = anchor {
        if let Ok(mut slot) = DRAG_ANCHOR.lock() {
            *slot = Some(anchor);
        }
    }
    // Re-place while still collapsed so the panel opens where the user is,
    // not where the drag began.
    place(&window, ICON_SIZE, ICON_SIZE)?;
    window.show().map_err(|error| error.to_string())?;
    let _ = app.emit_to(CORNER_LABEL, "corner-shake", ());
    Ok(())
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

/// 设置 → 角落拖拽框 → 晃动唤出暂存架.
#[tauri::command]
pub fn set_corner_box_shelf_enabled(app: AppHandle, enabled: bool) -> Result<(), String> {
    persist(&app, SHELF_KEY, if enabled { "true" } else { "false" })?;
    app.emit("corner-shelf-changed", enabled)
        .map_err(|error| error.to_string())
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
        prepare(&app)?;
    } else {
        hide(&app);
    }
    app.emit("corner-box-enabled-changed", enabled)
        .map_err(|error| error.to_string())
}

/// 跟随拖拽 / 右上角 / 左上角. Re-anchors the window and tells it which way
/// the icon now faces.
#[tauri::command]
pub fn set_corner_box_side(app: AppHandle, side: String) -> Result<(), String> {
    let side = match side.as_str() {
        "top-left" => "top-left",
        "top-right" => "top-right",
        _ => "follow",
    };
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
/// list of `rows` peers, then back to the icon.
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
        assert_eq!(size_for(false, 0), (ICON_SIZE, ICON_SIZE));
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

    #[test]
    fn the_box_sits_beside_the_drag_in_follow_mode() {
        let area = (0, 0, 1440, 900);
        assert_eq!(drag_origin((400, 300), 48, area, 18), (418, 318));
    }

    #[test]
    fn a_drag_at_the_edge_flips_the_box_inside() {
        let area = (0, 0, 1000, 800);
        // The right edge: to the left of the pointer.
        assert_eq!(drag_origin((990, 300), 48, area, 18), (924, 318));
        // The bottom edge: above it.
        assert_eq!(drag_origin((400, 790), 48, area, 18), (418, 724));
        // Both at once.
        assert_eq!(drag_origin((990, 790), 48, area, 18), (924, 724));
    }

    #[test]
    fn an_anchor_off_screen_still_gets_an_inside_box() {
        let area = (0, 0, 1000, 800);
        let (x, y) = drag_origin((-50, 900), 48, area, 18);
        assert!((0..=952).contains(&x));
        assert!((0..=752).contains(&y));
    }
}
