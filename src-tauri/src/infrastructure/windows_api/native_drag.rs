//! Dragging an entry out of TieZ through the Windows shell instead of
//! WebView2, which gives other apps nothing they can use for a dragged
//! file:// link: an image or file dropped into Explorer or a chat app did
//! nothing. Files go out the way Explorer hands them over (the shell's own
//! data object), text with its HTML where it has some, under the preview the
//! page drew.

use crate::app::commands::drag_cmd::NativeDragRequest;
use crate::services::clipboard::encode_cf_html;
use base64::Engine;
use serde::Serialize;
use std::mem::ManuallyDrop;
use tauri::{Emitter, Manager};
use windows::core::{w, HSTRING};
use windows::Win32::Foundation::{GlobalFree, COLORREF, E_FAIL, HWND, POINT, RECT, SIZE};
use windows::Win32::Graphics::Gdi::{
    CreateDIBSection, DeleteObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
};
use windows::Win32::System::Com::{
    CoCreateInstance, IDataObject, CLSCTX_INPROC_SERVER, DVASPECT_CONTENT, FORMATETC, STGMEDIUM,
    STGMEDIUM_0, TYMED_HGLOBAL,
};
use windows::Win32::System::DataExchange::RegisterClipboardFormatW;
use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GHND};
use windows::Win32::System::Ole::{IDropSource, DROPEFFECT_COPY, DROPEFFECT_NONE};
use windows::Win32::UI::Shell::Common::ITEMIDLIST;
use windows::Win32::UI::Shell::{
    IDragSourceHelper, ILFree, SHCreateDataObject, SHCreateShellItemArrayFromIDLists, SHDoDragDrop,
    SHParseDisplayName, BHID_DataObject, CLSID_DragDropHelper, SHDRAGIMAGE,
};
use windows::Win32::UI::WindowsAndMessaging::{GetCursorPos, GetWindowRect};

const CF_UNICODETEXT: u16 = 13;

/// Sent to the page when the drag is over (same as on macOS).
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct NativeDragEnded {
    accepted: bool,
    inside: bool,
}

/// The shell's data object for files, as Explorer makes it: CF_HDROP and the
/// shell formats other apps look for.
unsafe fn file_data_object(paths: &[&String]) -> windows::core::Result<IDataObject> {
    let mut pidls: Vec<*mut ITEMIDLIST> = Vec::new();
    for path in paths {
        let mut pidl = std::ptr::null_mut();
        if SHParseDisplayName(&HSTRING::from(path.as_str()), None, &mut pidl, 0, None).is_ok() {
            pidls.push(pidl);
        }
    }
    let result = if pidls.is_empty() {
        Err(E_FAIL.into())
    } else {
        let list: Vec<*const ITEMIDLIST> = pidls.iter().map(|pidl| *pidl as *const _).collect();
        SHCreateShellItemArrayFromIDLists(&list)
            .and_then(|items| items.BindToHandler(None, &BHID_DataObject))
    };
    for pidl in pidls {
        ILFree(Some(pidl));
    }
    result
}

/// Puts `bytes` on `object` as `format`; the object then owns the memory.
unsafe fn set_hglobal(object: &IDataObject, format: u16, bytes: &[u8]) -> windows::core::Result<()> {
    let memory = GlobalAlloc(GHND, bytes.len())?;
    let target = GlobalLock(memory);
    if target.is_null() {
        let _ = GlobalFree(Some(memory));
        return Err(E_FAIL.into());
    }
    std::ptr::copy_nonoverlapping(bytes.as_ptr(), target as *mut u8, bytes.len());
    let _ = GlobalUnlock(memory);
    let format_etc = FORMATETC {
        cfFormat: format,
        ptd: std::ptr::null_mut(),
        dwAspect: DVASPECT_CONTENT.0,
        lindex: -1,
        tymed: TYMED_HGLOBAL.0 as u32,
    };
    let medium = STGMEDIUM {
        tymed: TYMED_HGLOBAL.0 as u32,
        u: STGMEDIUM_0 { hGlobal: memory },
        pUnkForRelease: ManuallyDrop::new(None),
    };
    let result = object.SetData(&format_etc, &medium, true);
    if result.is_err() {
        let _ = GlobalFree(Some(memory));
    }
    result
}

unsafe fn text_data_object(text: &str, html: Option<&str>) -> windows::core::Result<IDataObject> {
    let object: IDataObject = SHCreateDataObject(None, None, None::<&IDataObject>)?;
    let wide: Vec<u8> = text
        .encode_utf16()
        .chain(std::iter::once(0))
        .flat_map(u16::to_le_bytes)
        .collect();
    set_hglobal(&object, CF_UNICODETEXT, &wide)?;
    if let Some(html) = html {
        let format = RegisterClipboardFormatW(w!("HTML Format"));
        if format != 0 {
            let mut bytes = encode_cf_html(html).into_bytes();
            bytes.push(0);
            let _ = set_hglobal(&object, format as u16, &bytes);
        }
    }
    Ok(object)
}

/// Hands the preview to the shell, which draws it under the pointer.
unsafe fn set_drag_image(object: &IDataObject, png: &[u8], grab: POINT) -> Result<(), String> {
    let image = image::load_from_memory(png).map_err(|e| e.to_string())?.to_rgba8();
    let (width, height) = (image.width() as i32, image.height() as i32);
    let info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width,
            biHeight: -height,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut bits = std::ptr::null_mut();
    let bitmap = CreateDIBSection(None, &info, DIB_RGB_COLORS, &mut bits, None, 0).map_err(|e| e.to_string())?;
    // Premultiplied BGRA, as the shell blends it.
    let target = std::slice::from_raw_parts_mut(bits as *mut u8, (width * height * 4) as usize);
    for (dst, src) in target.chunks_exact_mut(4).zip(image.pixels()) {
        let [r, g, b, a] = src.0;
        let premultiply = |c: u8| ((c as u16 * a as u16 + 127) / 255) as u8;
        dst.copy_from_slice(&[premultiply(b), premultiply(g), premultiply(r), a]);
    }
    let helper: IDragSourceHelper =
        CoCreateInstance(&CLSID_DragDropHelper, None, CLSCTX_INPROC_SERVER).map_err(|e| e.to_string())?;
    let drag_image = SHDRAGIMAGE {
        sizeDragImage: SIZE { cx: width, cy: height },
        ptOffset: grab,
        hbmpDragImage: bitmap,
        // No colour key: the alpha channel shapes the preview.
        crColorKey: COLORREF(0xFFFF_FFFF),
    };
    // On success the helper owns the bitmap.
    if let Err(error) = helper.InitializeFromBitmap(&drag_image, object) {
        let _ = DeleteObject(bitmap.into());
        return Err(error.to_string());
    }
    Ok(())
}

unsafe fn run_drag(hwnd: HWND, request: &NativeDragRequest) -> Result<bool, String> {
    let files: Vec<&String> = request
        .files
        .iter()
        .filter(|path| std::path::Path::new(path).exists())
        .collect();
    let object = if !files.is_empty() {
        file_data_object(&files)
    } else if let Some(text) = request.text.as_deref().or(request.url.as_deref()) {
        text_data_object(text, request.html.as_deref())
    } else {
        return Err("nothing to drag".to_string());
    }
    .map_err(|e| e.to_string())?;

    if let Ok(png) = base64::engine::general_purpose::STANDARD.decode(request.preview.as_bytes()) {
        let scale = request.scale.max(1.0);
        let grab = POINT {
            x: (request.grab_x * scale).round() as i32,
            y: (request.grab_y * scale).round() as i32,
        };
        // Without a preview the shell still drags, with its default image.
        if let Err(error) = set_drag_image(&object, &png, grab) {
            eprintln!(">>> [DRAG] preview not set: {error}");
        }
    }

    let effect = SHDoDragDrop(Some(hwnd), &object, None::<&IDropSource>, DROPEFFECT_COPY)
        .map_err(|e| e.to_string())?;
    Ok(effect != DROPEFFECT_NONE)
}

fn pointer_inside(hwnd: HWND) -> bool {
    unsafe {
        let mut point = POINT::default();
        let mut rect = RECT::default();
        GetCursorPos(&mut point).is_ok()
            && GetWindowRect(hwnd, &mut rect).is_ok()
            && point.x >= rect.left
            && point.x < rect.right
            && point.y >= rect.top
            && point.y < rect.bottom
    }
}

/// Starts the drag from the main loop rather than inside the command: the
/// shell's drag runs its own message loop until the drop, which would block
/// WebView2's request handler for that long.
pub fn start(window: &tauri::WebviewWindow, request: NativeDragRequest) -> Result<(), String> {
    let hwnd = HWND(window.hwnd().map_err(|e| e.to_string())?.0);
    let raw = hwnd.0 as isize;
    let app = window.app_handle().clone();
    window
        .run_on_main_thread(move || {
            let hwnd = HWND(raw as *mut core::ffi::c_void);
            let accepted = match unsafe { run_drag(hwnd, &request) } {
                Ok(accepted) => accepted,
                Err(error) => {
                    eprintln!(">>> [DRAG] native drag failed: {error}");
                    false
                }
            };
            let _ = app.emit(
                "native-drag-ended",
                NativeDragEnded { accepted, inside: pointer_inside(hwnd) },
            );
        })
        .map_err(|e| e.to_string())
}
