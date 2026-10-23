//! Watches for a file drag starting anywhere on the system, so the corner box
//! can appear only while it is useful — and for a quick back-and-forth wiggle
//! of the mouse, which summons the expanded panel (the 「暂存架」 gesture).
//!
//! Tauri's drag-drop event fires only once the pointer is over one of our
//! windows — too late, because the icon has to be there to be aimed at. A
//! global event monitor would be the usual answer, but it needs the
//! Accessibility permission, and AppKit's drag sessions deliver to monitors
//! inconsistently. Polling costs nothing and asks for nothing: the left button
//! held down *and* a file URL on the drag pasteboard is a file drag, no matter
//! which app started it.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use objc2::rc::autoreleasepool;
use objc2_app_kit::{NSEvent, NSPasteboard, NSPasteboardNameDrag};
use tauri::AppHandle;

/// The polling thread outlives every window; a second one would only race the
/// first over the same show/hide calls.
static STARTED: AtomicBool = AtomicBool::new(false);
/// What the last poll saw, so only the two transitions are reported.
static DRAGGING: AtomicBool = AtomicBool::new(false);

/// Quick enough to see a wiggle: a shake reverses direction every few tens of
/// milliseconds, and 200ms would turn most of them into a single swing.
const POLL_INTERVAL: Duration = Duration::from_millis(120);
/// Each leg has to travel this far (screen points) to count. Hand tremor while
/// dragging a file is smaller than this.
const MIN_SWING: f64 = 14.0;
/// Two full back-and-forth motions: right→left→right→left.
const REQUIRED_REVERSALS: u32 = 4;
/// The whole gesture has to finish inside this window.
const SHAKE_WINDOW: Duration = Duration::from_millis(1600);
/// A pause longer than this means the user stopped moving, not that they shook.
const SHAKE_IDLE: Duration = Duration::from_millis(650);

#[derive(Debug)]
struct ShakeState {
    last_x: Option<f64>,
    direction: i32,
    leg: f64,
    reversals: u32,
    started: Option<Instant>,
    last_move: Option<Instant>,
    fired: bool,
}

impl ShakeState {
    const fn new() -> Self {
        Self {
            last_x: None,
            direction: 0,
            leg: 0.0,
            reversals: 0,
            started: None,
            last_move: None,
            fired: false,
        }
    }

    fn reset(&mut self) {
        *self = Self::new();
    }

    /// One mouse sample; true exactly once per drag, when the gesture lands.
    fn observe(&mut self, x: f64, now: Instant) -> bool {
        let Some(previous) = self.last_x.replace(x) else {
            return false;
        };
        let dx = x - previous;
        let direction = if dx > 1.0 {
            1
        } else if dx < -1.0 {
            -1
        } else {
            0
        };
        if direction == 0 {
            return false;
        }
        if let Some(moved) = self.last_move {
            if now.duration_since(moved) > SHAKE_IDLE {
                // The gesture broke; start over from this sample.
                self.reset();
                self.last_x = Some(x);
                self.direction = direction;
                self.last_move = Some(now);
                return false;
            }
        }
        self.last_move = Some(now);
        if self.direction == 0 {
            self.direction = direction;
            self.leg = dx.abs();
            return false;
        }
        if direction == self.direction {
            self.leg += dx.abs();
            return false;
        }
        // Direction reversed: the leg that just ended only counts if it was a
        // real swing rather than the jitter of a hand holding a mouse button.
        let swung = self.leg >= MIN_SWING;
        // This sample already travels the first stretch of the new leg.
        self.leg = dx.abs();
        self.direction = direction;
        if !swung {
            self.reversals = 0;
            self.started = None;
            return false;
        }
        self.reversals += 1;
        let started = *self.started.get_or_insert(now);
        if now.duration_since(started) > SHAKE_WINDOW {
            self.reversals = 1;
            self.started = Some(now);
            return false;
        }
        if self.reversals >= REQUIRED_REVERSALS && !self.fired {
            self.fired = true;
            return true;
        }
        false
    }
}

static SHAKE: Mutex<ShakeState> = Mutex::new(ShakeState::new());

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
    let (dragging, mouse_x) = autoreleasepool(|_| {
        // Bit 0 is the primary (left) button.
        if NSEvent::pressedMouseButtons() & 1 == 0 {
            return (false, None);
        }
        // The name is an AppKit global; reading it is the one unsafe step.
        let pasteboard =
            unsafe { NSPasteboard::pasteboardWithName(NSPasteboardNameDrag) };
        let Some(types) = pasteboard.types() else {
            return (false, None);
        };
        let is_file = types.iter().any(|kind| {
            let kind = kind.to_string();
            kind == "public.file-url" || kind == "NSFilenamesPboardType"
        });
        if !is_file {
            return (false, None);
        }
        (true, Some(NSEvent::mouseLocation().x))
    });

    let was_dragging = DRAGGING.load(Ordering::SeqCst);
    if dragging != was_dragging {
        DRAGGING.store(dragging, Ordering::SeqCst);
        if dragging {
            if let Err(error) = crate::app::corner_box::show_for_drag(app) {
                eprintln!(">>> [CORNER] drag icon did not open: {error}");
            }
        } else {
            crate::app::corner_box::end_drag(app);
        }
    }

    let Some(x) = mouse_x else {
        if let Ok(mut shake) = SHAKE.lock() {
            shake.reset();
        }
        return;
    };
    if !dragging {
        return;
    }
    let fire = SHAKE
        .lock()
        .map(|mut shake| shake.observe(x, Instant::now()))
        .unwrap_or(false);
    if fire {
        if let Err(error) = crate::app::corner_box::open_shelf(app) {
            eprintln!(">>> [CORNER] shelf did not open: {error}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Feed a sequence of x positions one poll apart; return whether the
    /// gesture fired at any sample.
    fn feed(state: &mut ShakeState, xs: &[f64], step: Duration) -> bool {
        let start = Instant::now();
        xs.iter()
            .enumerate()
            .any(|(index, x)| state.observe(*x, start + step * index as u32))
    }

    #[test]
    fn a_quick_back_and_forth_opens_the_shelf_once() {
        let mut state = ShakeState::new();
        let step = Duration::from_millis(120);
        // right, left, right, left, each leg ~30px.
        let fired = feed(&mut state, &[0.0, 30.0, 0.0, 30.0, 0.0, 30.0], step);
        assert!(fired, "four real reversals should fire");
        // Further movement in the same drag must not fire (and re-expand).
        assert!(!feed(&mut state, &[0.0, 30.0, 0.0, 30.0], step));
    }

    #[test]
    fn small_jitter_is_not_a_shake() {
        let mut state = ShakeState::new();
        let step = Duration::from_millis(120);
        // 4px legs: a hand dragging a file, not a deliberate wiggle.
        assert!(!feed(
            &mut state,
            &[0.0, 4.0, 0.0, 4.0, 0.0, 4.0, 0.0, 4.0],
            step
        ));
    }

    #[test]
    fn a_slow_swing_does_not_count_as_a_shake() {
        let mut state = ShakeState::new();
        let step = Duration::from_millis(600);
        // Four reversals, but spread over more than the gesture window.
        assert!(!feed(&mut state, &[0.0, 30.0, 0.0, 30.0, 0.0, 30.0], step));
    }

    #[test]
    fn stopping_between_swings_resets_the_gesture() {
        let mut state = ShakeState::new();
        let start = Instant::now();
        let mut time = start;
        for x in [0.0, 30.0, 0.0] {
            state.observe(x, time);
            time += Duration::from_millis(120);
        }
        // A long pause, then two more swings: not the same gesture.
        time += SHAKE_IDLE * 2;
        let mut fired = false;
        for x in [30.0, 0.0, 30.0, 0.0] {
            fired |= state.observe(x, time);
            time += Duration::from_millis(120);
        }
        assert!(!fired);
    }
}
