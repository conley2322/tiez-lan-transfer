pub mod apps;
#[cfg(windows)]
pub mod drag_drop;
#[cfg(windows)]
pub mod elevated_autostart;
#[cfg(windows)]
pub mod single_instance;
#[cfg(windows)]
pub mod webview_keys;
#[cfg(windows)]
pub mod native_drag;
pub mod win_clipboard;
pub mod window_tracker;
