use crate::domain::models::ClipboardEntry;
use crate::services::encryption_queue::EncryptionQueue;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;

pub struct SettingsState {
    pub deduplicate: AtomicBool,
    pub persistent: AtomicBool,
    pub file_server_auto_close: AtomicBool,
    pub theme: Mutex<String>,
    pub capture_files: AtomicBool,
    pub capture_rich_text: AtomicBool,
    pub rich_text_snapshot_preview: AtomicBool,
    pub auto_copy_file: AtomicBool,
    pub silent_start: AtomicBool,
    pub delete_after_paste: AtomicBool,
    pub privacy_protection: AtomicBool,
    pub privacy_protection_kinds: Mutex<Vec<String>>,
    pub privacy_protection_custom_rules: Mutex<Vec<String>>,
    pub cleanup_rules: Mutex<String>,
    pub app_cleanup_policies: Mutex<String>,
    pub sequential_mode: AtomicBool,
    pub sequential_paste_hotkey: Mutex<String>,
    /// Pastes as plain text: the latest item, or the selected one while the
    /// window is open (#148). Empty: off.
    pub plain_paste_hotkey: Mutex<String>,
    pub search_hotkey: Mutex<String>,
    pub quick_paste_modifier: Mutex<String>,
    pub sound_enabled: AtomicBool,
    pub paste_sound_enabled: AtomicBool,
    pub hide_tray_icon: AtomicBool,
    pub hide_dock_icon: AtomicBool,
    pub edge_docking: AtomicBool,
    pub follow_mouse: AtomicBool,
    pub arrow_key_selection: AtomicBool,
    /// Idle WebView release (#154): destroy main WebView after idle, keep listeners.
    pub light_mode_enabled: AtomicBool,
    /// Idle minutes before entering light mode (min 1).
    pub light_mode_idle_minutes: AtomicU64,
    pub main_hotkey: Mutex<String>,
    pub monitors: Mutex<Vec<tauri::Monitor>>,
    pub sound_volume: Mutex<f64>,
}

impl SettingsState {
    /// Takes every value from `fresh` (「恢复默认设置」). No `..` below: a new
    /// field won't compile until it's handled here. Monitors are runtime state.
    pub fn take_values_from(&self, fresh: SettingsState) {
        let SettingsState {
            deduplicate,
            persistent,
            file_server_auto_close,
            theme,
            capture_files,
            capture_rich_text,
            rich_text_snapshot_preview,
            auto_copy_file,
            silent_start,
            delete_after_paste,
            privacy_protection,
            privacy_protection_kinds,
            privacy_protection_custom_rules,
            cleanup_rules,
            app_cleanup_policies,
            sequential_mode,
            sequential_paste_hotkey,
            plain_paste_hotkey,
            search_hotkey,
            quick_paste_modifier,
            sound_enabled,
            paste_sound_enabled,
            hide_tray_icon,
            hide_dock_icon,
            edge_docking,
            follow_mouse,
            arrow_key_selection,
            light_mode_enabled,
            light_mode_idle_minutes,
            main_hotkey,
            monitors: _,
            sound_volume,
        } = fresh;
        self.deduplicate.store(deduplicate.into_inner(), Ordering::Relaxed);
        self.persistent.store(persistent.into_inner(), Ordering::Relaxed);
        self.file_server_auto_close.store(file_server_auto_close.into_inner(), Ordering::Relaxed);
        *self.theme.lock().unwrap() = theme.into_inner().unwrap();
        self.capture_files.store(capture_files.into_inner(), Ordering::Relaxed);
        self.capture_rich_text.store(capture_rich_text.into_inner(), Ordering::Relaxed);
        self.rich_text_snapshot_preview.store(rich_text_snapshot_preview.into_inner(), Ordering::Relaxed);
        self.auto_copy_file.store(auto_copy_file.into_inner(), Ordering::Relaxed);
        self.silent_start.store(silent_start.into_inner(), Ordering::Relaxed);
        self.delete_after_paste.store(delete_after_paste.into_inner(), Ordering::Relaxed);
        self.privacy_protection.store(privacy_protection.into_inner(), Ordering::Relaxed);
        *self.privacy_protection_kinds.lock().unwrap() = privacy_protection_kinds.into_inner().unwrap();
        *self.privacy_protection_custom_rules.lock().unwrap() = privacy_protection_custom_rules.into_inner().unwrap();
        *self.cleanup_rules.lock().unwrap() = cleanup_rules.into_inner().unwrap();
        *self.app_cleanup_policies.lock().unwrap() = app_cleanup_policies.into_inner().unwrap();
        self.sequential_mode.store(sequential_mode.into_inner(), Ordering::Relaxed);
        *self.sequential_paste_hotkey.lock().unwrap() = sequential_paste_hotkey.into_inner().unwrap();
        *self.plain_paste_hotkey.lock().unwrap() = plain_paste_hotkey.into_inner().unwrap();
        *self.search_hotkey.lock().unwrap() = search_hotkey.into_inner().unwrap();
        *self.quick_paste_modifier.lock().unwrap() = quick_paste_modifier.into_inner().unwrap();
        self.sound_enabled.store(sound_enabled.into_inner(), Ordering::Relaxed);
        self.paste_sound_enabled.store(paste_sound_enabled.into_inner(), Ordering::Relaxed);
        self.hide_tray_icon.store(hide_tray_icon.into_inner(), Ordering::Relaxed);
        self.hide_dock_icon.store(hide_dock_icon.into_inner(), Ordering::Relaxed);
        self.edge_docking.store(edge_docking.into_inner(), Ordering::Relaxed);
        self.follow_mouse.store(follow_mouse.into_inner(), Ordering::Relaxed);
        self.arrow_key_selection.store(arrow_key_selection.into_inner(), Ordering::Relaxed);
        self.light_mode_enabled.store(light_mode_enabled.into_inner(), Ordering::Relaxed);
        self.light_mode_idle_minutes.store(light_mode_idle_minutes.into_inner(), Ordering::Relaxed);
        *self.main_hotkey.lock().unwrap() = main_hotkey.into_inner().unwrap();
        *self.sound_volume.lock().unwrap() = sound_volume.into_inner().unwrap();
    }
}

#[derive(Default)]
pub struct PasteQueueState {
    pub items: VecDeque<i64>,
    pub last_action_was_paste: bool,
    pub last_pasted_content: Option<String>,
    pub last_pasted_fingerprint: Option<String>,
    pub last_paste_timestamp_ms: u64,
}

#[derive(Default)]
pub struct PasteQueue(pub Mutex<PasteQueueState>);

pub struct SessionHistory(pub Mutex<VecDeque<ClipboardEntry>>);

pub struct AppDataDir(pub Mutex<std::path::PathBuf>);

pub struct EncryptionQueueState(pub EncryptionQueue);

#[cfg(test)]
mod tests {
    use crate::app::setup::{load_settings, settings_state_from};
    use crate::infrastructure::repository::settings_repo::{SettingsRepository, SqliteSettingsRepository};
    use std::sync::atomic::Ordering;
    use std::sync::{Arc, Mutex};

    #[test]
    fn a_settings_reset_reaches_the_running_state() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE settings(key TEXT PRIMARY KEY, value TEXT NOT NULL);")
            .unwrap();
        let repo = SqliteSettingsRepository::new(Arc::new(Mutex::new(conn)));
        let defaults = settings_state_from(&load_settings(&repo));
        repo.set("app.delete_after_paste", "true").unwrap();
        repo.set("app.cleanup_rules", "secret => ").unwrap();
        repo.set("app.hotkey", "Ctrl+Shift+K").unwrap();
        let running = settings_state_from(&load_settings(&repo));
        assert!(running.delete_after_paste.load(Ordering::Relaxed));

        repo.clear().unwrap();
        running.take_values_from(settings_state_from(&load_settings(&repo)));

        assert!(!running.delete_after_paste.load(Ordering::Relaxed));
        assert_eq!(*running.cleanup_rules.lock().unwrap(), *defaults.cleanup_rules.lock().unwrap());
        assert_eq!(*running.main_hotkey.lock().unwrap(), *defaults.main_hotkey.lock().unwrap());
    }
}
