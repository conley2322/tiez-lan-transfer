use std::sync::Arc;
#[cfg(target_os = "windows")]
use windows::core::PCWSTR;
#[cfg(target_os = "windows")]
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
#[cfg(target_os = "windows")]
use windows::Win32::System::DataExchange::{
    AddClipboardFormatListener, RemoveClipboardFormatListener,
};
#[cfg(target_os = "windows")]
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW,
    GetWindowLongPtrW, RegisterClassW, SetWindowLongPtrW, GWLP_USERDATA, HWND_MESSAGE, MSG,
    WM_CLIPBOARDUPDATE, WNDCLASSW,
};

/// Run captures on a worker thread. The returned closure marks the clipboard
/// as changed: the worker captures as soon as it is free, and changes seen
/// while it is busy collapse into one capture of the latest value.
#[cfg(any(target_os = "macos", target_os = "windows"))]
fn spawn_capture_worker(
    callback: Arc<dyn Fn() + Send + Sync + 'static>,
) -> Arc<dyn Fn() + Send + Sync + 'static> {
    use std::sync::{Condvar, Mutex};
    let pending = Arc::new((Mutex::new(false), Condvar::new()));
    let worker_pending = pending.clone();
    std::thread::spawn(move || loop {
        {
            let (lock, ready) = &*worker_pending;
            let mut changed = lock.lock().unwrap_or_else(|p| p.into_inner());
            while !*changed {
                changed = ready.wait(changed).unwrap_or_else(|p| p.into_inner());
            }
            *changed = false;
        }
        // A panic in one capture must not end clipboard recording.
        if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| callback())).is_err() {
            eprintln!(">>> [CLIPBOARD] capture panicked; continuing");
        }
    });
    Arc::new(move || {
        let (lock, ready) = &*pending;
        *lock.lock().unwrap_or_else(|p| p.into_inner()) = true;
        ready.notify_one();
    })
}

pub fn listen_clipboard(callback: Arc<dyn Fn() + Send + Sync + 'static>) {
    // macOS has no clipboard notification, so changeCount is polled. The
    // clipboard-rs watcher polled every 500 ms and ran the capture on its own
    // polling thread: several quick copies, or any copy made while a capture
    // was busy (e.g. fetching a web page's images), ended up as one entry.
    // Now the poll is fast and captures run on a worker; a change seen while
    // the worker is busy is captured as soon as it is free. Not debounced:
    // a later clipboard value can't be rebuilt from a dropped event, and the
    // monitor drops genuine duplicate events by payload hash.
    #[cfg(target_os = "macos")]
    {
        let notify = spawn_capture_worker(callback);
        std::thread::spawn(move || {
            let mut last = crate::infrastructure::macos_api::clipboard::change_count();
            loop {
                std::thread::sleep(std::time::Duration::from_millis(80));
                let current = crate::infrastructure::macos_api::clipboard::change_count();
                if current != last {
                    last = current;
                    notify();
                }
            }
        });
        return;
    }

    // Windows: WM_CLIPBOARDUPDATE only signals the worker. The capture (with
    // its settle sleeps, up to ~1.4 s for some sources) used to run inside the
    // window procedure and held up the listener's message loop.
    #[cfg(target_os = "windows")]
    let callback = spawn_capture_worker(callback);

    #[cfg(target_os = "windows")]
    {
        std::thread::spawn(move || unsafe {
            let instance = match windows::Win32::System::LibraryLoader::GetModuleHandleW(None) {
                Ok(instance) => instance,
                Err(err) => {
                    eprintln!(">>> [CLIPBOARD] failed to get module handle: {err}");
                    return;
                }
            };
            let class_name: Vec<u16> = "TieZClipboardListener"
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();
            let wnd_class = WNDCLASSW {
                lpfnWndProc: Some(wnd_proc),
                hInstance: instance.into(),
                lpszClassName: PCWSTR(class_name.as_ptr()),
                ..Default::default()
            };
            let _ = RegisterClassW(&wnd_class);

            let hwnd = match CreateWindowExW(
                Default::default(),
                PCWSTR(class_name.as_ptr()),
                PCWSTR::null(),
                Default::default(),
                0,
                0,
                0,
                0,
                Some(HWND_MESSAGE),
                None,
                Some(HINSTANCE(instance.0)),
                None,
            ) {
                Ok(hwnd) => hwnd,
                Err(err) => {
                    eprintln!(">>> [CLIPBOARD] failed to create Windows listener window: {err}");
                    return;
                }
            };

            let callback_ptr = Box::into_raw(Box::new(callback));
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, callback_ptr as isize);
            if let Err(err) = AddClipboardFormatListener(hwnd) {
                eprintln!(">>> [CLIPBOARD] failed to register Windows listener: {err}");
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                drop(Box::from_raw(callback_ptr));
                let _ = DestroyWindow(hwnd);
                return;
            }

            println!(">>> [CLIPBOARD] Windows event-driven listener started");
            let mut msg = MSG::default();
            loop {
                let result = GetMessageW(&mut msg, None, 0, 0);
                if result.0 <= 0 {
                    break;
                }
                DispatchMessageW(&msg);
            }

            let _ = RemoveClipboardFormatListener(hwnd);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            drop(Box::from_raw(callback_ptr));
            let _ = DestroyWindow(hwnd);
        });
        return;
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = callback;
        eprintln!(">>> [CLIPBOARD] listener unsupported on this platform");
    }
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == WM_CLIPBOARDUPDATE {
        let callback_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
        if callback_ptr != 0 {
            let callback = &*(callback_ptr as *const Arc<dyn Fn() + Send + Sync + 'static>);
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| callback()));
        }
        LRESULT(0)
    } else {
        DefWindowProcW(hwnd, msg, wparam, lparam)
    }
}
