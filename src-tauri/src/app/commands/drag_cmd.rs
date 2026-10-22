use serde::Deserialize;
use tauri::WebviewWindow;

/// What a native drag carries out of TieZ, and its preview.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeDragRequest {
    /// Files dropped as files (an image or file entry).
    pub files: Vec<String>,
    pub text: Option<String>,
    pub html: Option<String>,
    pub url: Option<String>,
    /// The preview badge as base64 PNG, drawn at a higher scale for Retina.
    pub preview: String,
    /// Badge size in points, and where the pointer holds it.
    pub preview_width: f64,
    pub preview_height: f64,
    pub grab_x: f64,
    pub grab_y: f64,
    /// Pixels per point the preview was drawn at.
    pub scale: f64,
}

/// Starts a system drag for an entry (macOS and Windows); its end arrives as
/// the `native-drag-ended` event. Elsewhere the page keeps its own drag.
#[tauri::command]
pub fn start_native_drag(window: WebviewWindow, request: NativeDragRequest) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        crate::infrastructure::macos_api::native_drag::start(&window, request)
    }
    #[cfg(target_os = "windows")]
    {
        crate::infrastructure::windows_api::native_drag::start(&window, request)
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = (window, request);
        Err("native drag is not available here".to_string())
    }
}
