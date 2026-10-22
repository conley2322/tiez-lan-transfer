//! Disable WebView2 browser accelerator keys (Ctrl+F find, F1 help, reload, zoom, …)
//! so they cannot fight TieZ / OS global hotkeys while the window is focused.

use tauri::{AppHandle, Manager, WebviewWindow};

/// Apply to every existing webview window.
pub fn disable_browser_accelerator_keys_for_app(app: &AppHandle) {
    for (_label, window) in app.webview_windows() {
        disable_browser_accelerator_keys_for_window(&window);
    }
}

pub fn disable_browser_accelerator_keys_for_window(window: &WebviewWindow) {
    let label = window.label().to_string();
    let label_for_err = label.clone();
    if let Err(error) = window.with_webview(move |platform_webview| {
        // SAFETY: WebView2 COM calls must run on the webview thread; `with_webview` guarantees that.
        unsafe {
            use webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2Settings3;
            use windows_core::Interface;

            let controller = platform_webview.controller();
            let Ok(core_webview) = controller.CoreWebView2() else {
                eprintln!("[webview-keys] CoreWebView2 unavailable for window '{label}'");
                return;
            };
            let Ok(settings) = core_webview.Settings() else {
                eprintln!("[webview-keys] Settings unavailable for window '{label}'");
                return;
            };
            let Ok(settings3) = settings.cast::<ICoreWebView2Settings3>() else {
                eprintln!(
                    "[webview-keys] ICoreWebView2Settings3 unavailable for window '{label}'"
                );
                return;
            };
            if let Err(error) = settings3.SetAreBrowserAcceleratorKeysEnabled(false) {
                eprintln!(
                    "[webview-keys] SetAreBrowserAcceleratorKeysEnabled(false) failed for '{label}': {error}"
                );
                return;
            }
            eprintln!("[webview-keys] browser accelerator keys disabled for '{label}'");
        }
    }) {
        eprintln!("[webview-keys] with_webview failed for '{label_for_err}': {error}");
    }
}
