//! Native smoke test: boots the real `tiez-app` binary against a throwaway
//! HOME and checks that it starts, creates its database, picks up a saved
//! setting after a restart, records a clipboard change and stays alive.
//!
//! Uses the debug binary cargo builds for this test (`CARGO_BIN_EXE_tiez-app`):
//! debug macOS builds keep their master key in a file, so the run never touches
//! the Keychain or the real data directory. The release bundle is deliberately
//! not used — it reads/writes the login Keychain.
//!
//! Writes to the system pasteboard (restored afterwards, plain text only), so
//! it is opt-in and needs every other TieZ instance quit:
//!   cargo test --test native_system_e2e -- --ignored
//! Set TIEZ_E2E_KEEP_HOME=1 to keep the temp HOME for inspection.
#![cfg(target_os = "macos")]

use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread::sleep;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const BOOT_TIMEOUT: Duration = Duration::from_secs(30);
const COPY_RETRY_INTERVAL: Duration = Duration::from_millis(1500);
const MAX_COPY_ATTEMPTS: u32 = 8;

/// Owns the temp HOME and our own child process. Kills the child by PID (never
/// by name), restores the pasteboard and removes the temp HOME — also when an
/// assertion panics.
struct Fixture {
    child: Option<Child>,
    home: PathBuf,
    saved_clipboard: Option<Vec<u8>>,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.stop();
        if let Some(saved) = self.saved_clipboard.take() {
            let _ = set_clipboard(&saved);
        }
        if std::env::var_os("TIEZ_E2E_KEEP_HOME").is_none() {
            let _ = std::fs::remove_dir_all(&self.home);
        }
    }
}

impl Fixture {
    fn new() -> Self {
        let home = std::env::temp_dir().join(format!("tiez-e2e-{}", unique_suffix()));
        std::fs::create_dir_all(&home).unwrap();
        Fixture { child: None, home, saved_clipboard: None }
    }

    fn log_path(&self) -> PathBuf {
        self.home.join("tiez-app.log")
    }

    fn launch(&mut self) {
        assert!(self.child.is_none(), "app already running");
        let log = OpenOptions::new().create(true).append(true).open(self.log_path()).unwrap();
        let child = Command::new(env!("CARGO_BIN_EXE_tiez-app"))
            .env("HOME", &self.home)
            .stdin(Stdio::null())
            .stdout(log.try_clone().unwrap())
            .stderr(log)
            .spawn()
            .expect("failed to spawn tiez-app");
        self.child = Some(child);
    }

    fn stop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    fn assert_alive(&mut self, stage: &str) {
        let child = self.child.as_mut().expect("app not running");
        if let Some(status) = child.try_wait().expect("try_wait failed") {
            panic!("tiez-app exited during {stage} ({status})\n{}", self.log_tail());
        }
    }

    fn log_tail(&self) -> String {
        let log = std::fs::read_to_string(self.log_path()).unwrap_or_default();
        let lines: Vec<&str> = log.lines().collect();
        let start = lines.len().saturating_sub(40);
        format!("--- last log lines ({}) ---\n{}", self.home.display(), lines[start..].join("\n"))
    }

    /// Polls `check` until it returns Some or `timeout` passes, failing fast
    /// if the app dies.
    fn poll<T>(&mut self, stage: &str, timeout: Duration, mut check: impl FnMut() -> Option<T>) -> Option<T> {
        let deadline = Instant::now() + timeout;
        loop {
            self.assert_alive(stage);
            if let Some(value) = check() {
                return Some(value);
            }
            if Instant::now() >= deadline {
                return None;
            }
            sleep(Duration::from_millis(100));
        }
    }

    fn wait_for<T>(&mut self, stage: &str, timeout: Duration, check: impl FnMut() -> Option<T>) -> T {
        match self.poll(stage, timeout, check) {
            Some(value) => value,
            None => panic!("timed out after {timeout:?} waiting for {stage}\n{}", self.log_tail()),
        }
    }
}

fn unique_suffix() -> String {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    format!("{}-{nanos}", std::process::id())
}

fn other_tiez_running() -> bool {
    Command::new("pgrep")
        .args(["-x", "tiez-app"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn get_clipboard() -> Vec<u8> {
    Command::new("pbpaste").output().expect("pbpaste failed").stdout
}

fn set_clipboard(bytes: &[u8]) -> std::io::Result<()> {
    let mut child = Command::new("pbcopy").stdin(Stdio::piped()).spawn()?;
    child.stdin.take().unwrap().write_all(bytes)?;
    child.wait()?;
    Ok(())
}

fn find_file(root: &Path, name: &str) -> Option<PathBuf> {
    for entry in std::fs::read_dir(root).ok()?.flatten() {
        let path = entry.path();
        if path.file_name().is_some_and(|n| n == name) {
            return Some(path);
        }
        if path.is_dir() && !path.is_symlink() {
            if let Some(found) = find_file(&path, name) {
                return Some(found);
            }
        }
    }
    None
}

fn open_db(db: &Path) -> Option<rusqlite::Connection> {
    rusqlite::Connection::open(db).ok()
}

fn setting(db: &Path, key: &str) -> Option<String> {
    open_db(db)?
        .query_row("SELECT value FROM settings WHERE key = ?1", [key], |r| r.get(0))
        .ok()
}

/// None while the app is still creating the schema.
fn history_count(db: &Path) -> Option<i64> {
    open_db(db)?
        .query_row("SELECT COUNT(*) FROM clipboard_history", [], |r| r.get(0))
        .ok()
}

#[test]
#[ignore = "launches the GUI app and writes the pasteboard; run with --ignored"]
fn native_app_boots_isolated_and_captures_clipboard() {
    assert!(
        !other_tiez_running(),
        "another tiez-app is running; quit it first (single-instance would hand off to it, and it would record the test clipboard into real history)"
    );
    let mut fx = Fixture::new();
    let home = fx.home.clone();

    // 1. First boot creates the database inside the temp HOME, with defaults.
    fx.launch();
    let db = fx.wait_for("database creation", BOOT_TIMEOUT, || find_file(&home, "clipboard.db"));
    let persistent = fx.wait_for("default settings", BOOT_TIMEOUT, || setting(&db, "app.persistent"));
    assert_eq!(persistent, "true", "new installs keep history across restarts");
    // Debug builds never add a login item; the pending flag stays put.
    assert_eq!(
        setting(&db, "app.autostart_pending").as_deref(),
        Some("true"),
        "fresh install should be marked for launch at login"
    );
    fx.assert_alive("first boot idle");
    fx.stop();

    // 2. Restart: the app must reopen the existing database and boot cleanly.
    fx.launch();
    let baseline = fx.wait_for("second boot", BOOT_TIMEOUT, || history_count(&db));

    // 3. A pasteboard change is persisted as exactly one new history row.
    //    The listener thread starts after the schema exists and takes the
    //    current changeCount as its baseline, so a copy made before it is up
    //    is never seen. There is no readiness signal: copy a fresh marker
    //    until one is recorded.
    fx.saved_clipboard = Some(get_clipboard());
    let mut attempts = 0;
    let after = loop {
        attempts += 1;
        let marker = format!("tiez-e2e-marker-{}", unique_suffix());
        set_clipboard(marker.as_bytes()).expect("pbcopy failed");
        if let Some(n) = fx.poll("clipboard capture", COPY_RETRY_INTERVAL, || {
            history_count(&db).filter(|&n| n > baseline)
        }) {
            break n;
        }
        assert!(
            attempts < MAX_COPY_ATTEMPTS,
            "no clipboard change recorded after {attempts} copies\n{}",
            fx.log_tail()
        );
    };
    assert_eq!(after, baseline + 1, "one copy was recorded as {} rows", after - baseline);
    sleep(Duration::from_secs(1)); // late duplicate events would land here
    assert_eq!(history_count(&db), Some(after), "the same copy was recorded again later");

    // 4. Still alive a moment later (no deferred crash in background tasks).
    sleep(Duration::from_secs(2));
    fx.assert_alive("post-capture idle");
}
