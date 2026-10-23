//! Watches for a file drag starting anywhere on the system, so the corner box
//! can appear only while it is useful.
//!
//! Tauri's drag-drop event fires only once the pointer is over one of our
//! windows — too late, because the icon has to be there to be aimed at. A
//! global event monitor would be the usual answer, but it needs the
//! Accessibility permission, and AppKit's drag sessions deliver to monitors
//! inconsistently. Polling costs nothing and asks for nothing: the left button
//! held down *and* a file URL on the drag pasteboard is a file drag, no matter
//! which app started it.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use objc2::rc::autoreleasepool;
use objc2_app_kit::{NSEvent, NSPasteboard, NSPasteboardNameDrag};
use tauri::AppHandle;

/// The polling thread outlives every window; a second one would only race the
/// first over the same show/hide calls.
static STARTED: AtomicBool = AtomicBool::new(false);
/// What the last poll saw, so only the two transitions are reported.
static DRAGGING: AtomicBool = AtomicBool::new(false);

const POLL_INTERVAL: Duration = Duration::from_millis(200);

pub fn start(app: AppHandle) {
    if STARTED.swap(true, Ordering::SeqCst) {
        return;
    }
    std::thread::spawn(move || loop {
        std::thread::sleep(POLL_INTERVAL);
        let handle = app.clone();
        // AppKit objects and the window work both belong on the main thread.
        // A failed dispatch only means the app is shutting down.
        let _ = app.run_on_main_thread(move || poll(&handle));
    });
}

fn poll(app: &AppHandle) {
    let dragging = autoreleasepool(|_| {
        // Bit 0 is the primary (left) button.
        if NSEvent::pressedMouseButtons() & 1 == 0 {
            return false;
        }
        // The name is an AppKit global; reading it is the one unsafe step.
        let pasteboard =
            unsafe { NSPasteboard::pasteboardWithName(NSPasteboardNameDrag) };
        let Some(types) = pasteboard.types() else {
            return false;
        };
        types.iter().any(|kind| {
            let kind = kind.to_string();
            kind == "public.file-url" || kind == "NSFilenamesPboardType"
        })
    });

    if dragging == DRAGGING.load(Ordering::SeqCst) {
        return;
    }
    DRAGGING.store(dragging, Ordering::SeqCst);
    if dragging {
        if let Err(error) = crate::app::corner_box::show_for_drag(app) {
            eprintln!(">>> [CORNER] drag icon did not open: {error}");
        }
    } else {
        crate::app::corner_box::end_drag(app);
    }
}
