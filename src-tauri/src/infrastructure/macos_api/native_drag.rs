//! Dragging an entry out of TieZ through AppKit instead of the web view.
//! WebKit gives other apps nothing they can use for a dragged file:// link,
//! so an image or file dropped elsewhere did nothing, and it dims the drag
//! preview so whatever is under it shows through. A native drag carries real
//! files (or the text, HTML and link of other entries) and shows the preview
//! as drawn.

use crate::app::commands::drag_cmd::NativeDragRequest;
use base64::Engine;
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, ProtocolObject};
use objc2::{define_class, msg_send, AllocAnyThread, DefinedClass, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{
    NSApplication, NSDragOperation, NSDraggingContext, NSDraggingItem, NSDraggingSession,
    NSDraggingSource, NSEvent, NSEventModifierFlags, NSEventType, NSImage, NSPasteboardItem,
    NSPasteboardTypeHTML, NSPasteboardTypeString, NSPasteboardTypeURL, NSPasteboardWriting,
    NSWindow,
};
use objc2_foundation::{NSArray, NSData, NSObject, NSObjectProtocol, NSPoint, NSRect, NSSize, NSString, NSURL};
use serde::Serialize;
use std::cell::RefCell;
use tauri::{AppHandle, Emitter, Manager};

/// Sent to the page when the drag is over.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct NativeDragEnded {
    /// Another app took the drop.
    accepted: bool,
    /// Dropped back onto TieZ's own window.
    inside: bool,
}

pub struct DragSourceIvars {
    app: AppHandle,
    window: Retained<NSWindow>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[name = "TieZDragSource"]
    #[thread_kind = MainThreadOnly]
    #[ivars = DragSourceIvars]
    struct DragSource;

    unsafe impl NSObjectProtocol for DragSource {}

    unsafe impl NSDraggingSource for DragSource {
        #[unsafe(method(draggingSession:sourceOperationMaskForDraggingContext:))]
        fn source_operation_mask(
            &self,
            _session: &NSDraggingSession,
            _context: NSDraggingContext,
        ) -> NSDragOperation {
            NSDragOperation::Copy
        }

        #[unsafe(method(draggingSession:endedAtPoint:operation:))]
        fn ended(&self, _session: &NSDraggingSession, point: NSPoint, operation: NSDragOperation) {
            let ivars = self.ivars();
            let frame = ivars.window.frame();
            let inside = point.x >= frame.origin.x
                && point.x < frame.origin.x + frame.size.width
                && point.y >= frame.origin.y
                && point.y < frame.origin.y + frame.size.height;
            let _ = ivars.app.emit(
                "native-drag-ended",
                NativeDragEnded { accepted: operation.0 != 0, inside },
            );
            // Let go of this source once its method has returned.
            let _ = ivars.app.run_on_main_thread(|| {
                ACTIVE_SOURCE.with(|source| source.borrow_mut().take());
            });
        }
    }
);

impl DragSource {
    fn new(mtm: MainThreadMarker, app: AppHandle, window: Retained<NSWindow>) -> Retained<Self> {
        let this = mtm.alloc::<Self>().set_ivars(DragSourceIvars { app, window });
        unsafe { msg_send![super(this), init] }
    }
}

thread_local! {
    /// The source of the drag in progress: AppKit doesn't keep it alive.
    static ACTIVE_SOURCE: RefCell<Option<Retained<DragSource>>> = const { RefCell::new(None) };
}

fn pasteboard_writers(request: &NativeDragRequest) -> Vec<Retained<ProtocolObject<dyn NSPasteboardWriting>>> {
    let files: Vec<_> = request
        .files
        .iter()
        .filter(|path| std::path::Path::new(path).exists())
        .map(|path| ProtocolObject::from_retained(NSURL::fileURLWithPath(&NSString::from_str(path))))
        .collect();
    if !files.is_empty() {
        return files;
    }
    if request.text.is_none() && request.html.is_none() && request.url.is_none() {
        return Vec::new();
    }
    let item = NSPasteboardItem::new();
    unsafe {
        if let Some(text) = &request.text {
            item.setString_forType(&NSString::from_str(text), NSPasteboardTypeString);
        }
        if let Some(html) = &request.html {
            item.setString_forType(&NSString::from_str(html), NSPasteboardTypeHTML);
        }
        if let Some(url) = &request.url {
            item.setString_forType(&NSString::from_str(url), NSPasteboardTypeURL);
        }
    }
    vec![ProtocolObject::from_retained(item)]
}

/// Starts the drag at the pointer. Runs on the main thread (a synchronous
/// command), while the button that started the page's drag is still down.
pub fn start(window: &tauri::WebviewWindow, request: NativeDragRequest) -> Result<(), String> {
    let mtm = MainThreadMarker::new().ok_or("not on the main thread")?;
    let ns_window_ptr = window.ns_window().map_err(|e| e.to_string())? as *mut NSWindow;
    let ns_window = unsafe { Retained::retain(ns_window_ptr) }.ok_or("no window")?;
    let view = ns_window.contentView().ok_or("no content view")?;

    let writers = pasteboard_writers(&request);
    if writers.is_empty() {
        return Err("nothing to drag".to_string());
    }

    let png = base64::engine::general_purpose::STANDARD
        .decode(request.preview.as_bytes())
        .map_err(|e| e.to_string())?;
    let image = NSImage::initWithData(NSImage::alloc(), &NSData::with_bytes(&png))
        .ok_or("unreadable preview")?;
    let size = NSSize::new(request.preview_width, request.preview_height);
    image.setSize(size);

    // AppKit wants the mouse event that starts the drag. The page's press is
    // usually still the current event; otherwise describe one at the pointer.
    let location = ns_window.mouseLocationOutsideOfEventStream();
    let app = NSApplication::sharedApplication(mtm);
    let current = app.currentEvent();
    let event = match current {
        Some(event)
            if matches!(event.r#type(), NSEventType::LeftMouseDown | NSEventType::LeftMouseDragged) =>
        {
            event
        }
        other => NSEvent::mouseEventWithType_location_modifierFlags_timestamp_windowNumber_context_eventNumber_clickCount_pressure(
            NSEventType::LeftMouseDragged,
            location,
            NSEventModifierFlags(0),
            other.map(|event| event.timestamp()).unwrap_or(0.0),
            ns_window.windowNumber(),
            None,
            0,
            1,
            1.0,
        )
        .ok_or("no mouse event")?,
    };

    let point = view.convertPoint_fromView(location, None);
    let y = if view.isFlipped() {
        point.y - request.grab_y
    } else {
        point.y + request.grab_y - size.height
    };
    let frame = NSRect::new(NSPoint::new(point.x - request.grab_x, y), size);

    let items: Vec<Retained<NSDraggingItem>> = writers
        .iter()
        .enumerate()
        .map(|(index, writer)| {
            let item = NSDraggingItem::initWithPasteboardWriter(NSDraggingItem::alloc(), writer);
            // One preview for the lot: the other items ride along unseen.
            let contents: Option<&AnyObject> = (index == 0).then(|| image.as_ref());
            unsafe { item.setDraggingFrame_contents(frame, contents) };
            item
        })
        .collect();

    let source = DragSource::new(mtm, window.app_handle().clone(), ns_window.clone());
    let session = view.beginDraggingSessionWithItems_event_source(
        &NSArray::from_retained_slice(&items),
        &event,
        ProtocolObject::from_ref(&*source),
    );
    session.setAnimatesToStartingPositionsOnCancelOrFail(true);
    ACTIVE_SOURCE.with(|active| *active.borrow_mut() = Some(source));
    Ok(())
}
