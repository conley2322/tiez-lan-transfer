//! Play the bundled clipboard sounds through cached NSSound instances for low
//! latency. Same WAV data as Windows (see `bundled_sound`).

use std::sync::Mutex;

use objc2::rc::{autoreleasepool, Retained};
use objc2::AllocAnyThread;
use objc2_app_kit::NSSound;
use objc2_foundation::NSData;

use crate::infrastructure::bundled_sound::{COPY_SOUND, PASTE_SOUND};

struct CachedSound(Option<Retained<NSSound>>);

// SAFETY: the NSSound is only touched while its mutex is held. Before, every
// thread shared AppKit's single named instance without any lock.
unsafe impl Send for CachedSound {}

// Process-wide rather than per thread: paste sounds play on a short-lived
// thread, and a thread-local unnamed NSSound would be freed mid-playback.
static COPY_CACHE: Mutex<CachedSound> = Mutex::new(CachedSound(None));
static PASTE_CACHE: Mutex<CachedSound> = Mutex::new(CachedSound(None));

fn slot_for(kind: &str) -> (&'static Mutex<CachedSound>, &'static [u8]) {
    match kind {
        "paste" => (&PASTE_CACHE, PASTE_SOUND),
        _ => (&COPY_CACHE, COPY_SOUND),
    }
}

fn with_sound<R>(kind: &str, f: impl FnOnce(&NSSound) -> R) -> Option<R> {
    let (cache, wav) = slot_for(kind);
    let mut slot = cache.lock().unwrap_or_else(|e| e.into_inner());
    autoreleasepool(|_| {
        if slot.0.is_none() {
            slot.0 = NSSound::initWithData(NSSound::alloc(), &NSData::with_bytes(wav));
        }
        slot.0.as_deref().map(f)
    })
}

pub fn play_clipboard_sound(kind: &str, volume: f64) {
    let vol = volume.clamp(0.0, 1.0);
    if vol <= 0.0 {
        return;
    }

    with_sound(kind, |sound| {
        sound.setVolume(vol as f32);
        let _ = sound.stop();
        sound.setCurrentTime(0.0);
        sound.play()
    });
}

/// Decode both sounds at startup so the first copy feedback is not delayed.
pub fn preload_clipboard_sounds() {
    with_sound("copy", |_| ());
    with_sound("paste", |_| ());
}
