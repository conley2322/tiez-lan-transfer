//! Upgrade regression tests: a database written by an older release must come
//! through the current startup path (init_db + legacy cipher migration) with
//! every record intact, readable and searchable.
//!
//! Old schemas are rebuilt from each release's own migrations.rs and default
//! settings, kept verbatim under tests/fixtures/legacy_migrations/ (taken from
//! git), so no real user data is involved. Old macOS builds stored sensitive
//! values as plaintext, which is what the samples below contain.

use crate::database;
use crate::infrastructure::encryption;
use crate::infrastructure::repository::clipboard_repo::{ClipboardRepository, SqliteClipboardRepository};
use crate::infrastructure::repository::settings_repo::{SettingsRepository, SqliteSettingsRepository};
use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, Once};

#[path = "../tests/fixtures/legacy_migrations/v0_3_1.rs"]
mod v0_3_1;
#[path = "../tests/fixtures/legacy_migrations/v0_3_2.rs"]
mod v0_3_2;
#[path = "../tests/fixtures/legacy_migrations/v0_3_3.rs"]
mod v0_3_3;
#[path = "../tests/fixtures/legacy_migrations/v0_3_4.rs"]
mod v0_3_4;
#[path = "../tests/fixtures/legacy_migrations/v1_0_0_beta_1.rs"]
mod v1_0_0_beta_1;

type Migrate = fn(&Connection) -> rusqlite::Result<()>;

const RELEASES: &[(&str, Migrate, &str)] = &[
    ("0.3.1", v0_3_1::run_migrations, include_str!("../tests/fixtures/legacy_migrations/v0_3_1_settings.sql")),
    ("0.3.2", v0_3_2::run_migrations, include_str!("../tests/fixtures/legacy_migrations/v0_3_2_settings.sql")),
    ("0.3.3", v0_3_3::run_migrations, include_str!("../tests/fixtures/legacy_migrations/v0_3_3_settings.sql")),
    ("0.3.4", v0_3_4::run_migrations, include_str!("../tests/fixtures/legacy_migrations/v0_3_4_settings.sql")),
    ("1.0.0-beta.1", v1_0_0_beta_1::run_migrations, include_str!("../tests/fixtures/legacy_migrations/v1_0_0_beta_1_settings.sql")),
];

struct Sample {
    id: i64,
    content_type: &'static str,
    content: &'static str,
    html: Option<&'static str>,
    pinned: bool,
    pinned_order: i64,
    tags: &'static [&'static str],
    timestamp: i64,
}

const SAMPLES: &[Sample] = &[
    Sample { id: 1, content_type: "text", content: "项目周会：周五 15:00 确认需求范围", html: None, pinned: false, pinned_order: 0, tags: &[], timestamp: 1_760_000_001_000 },
    Sample { id: 2, content_type: "text", content: "常用回复模板：收到，稍后处理。", html: None, pinned: true, pinned_order: 3, tags: &["工作"], timestamp: 1_760_000_002_000 },
    Sample { id: 3, content_type: "rich_text", content: "加粗的标题", html: Some("<p><b>加粗的标题</b></p>"), pinned: false, pinned_order: 0, tags: &[], timestamp: 1_760_000_003_000 },
    Sample { id: 4, content_type: "text", content: "账号 admin 密码 Hunter2!", html: None, pinned: false, pinned_order: 0, tags: &["密码"], timestamp: 1_760_000_004_000 },
    Sample { id: 5, content_type: "image", content: "/Users/me/Library/Application Support/com.tiez/attachments/a.png", html: None, pinned: false, pinned_order: 0, tags: &[], timestamp: 1_760_000_005_000 },
];

/// Settings a user had customised in the old release, plus plaintext secrets.
const USER_SETTINGS: &[(&str, &str)] = &[
    ("app.hotkey", "Ctrl+Shift+Z"),
    ("app.persistent", "true"),
    ("app.persistent_limit", "2000"),
    ("mqtt_password", "mqtt-secret"),
    ("cloud_sync_webdav_password", "dav-secret"),
];

/// encryption::init is process-wide (OnceLock); give it one throwaway dir.
/// Debug builds keep the master key in a file there, never in the Keychain.
fn init_encryption() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let dir = std::env::temp_dir().join(format!("tiez-upgrade-tests-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        encryption::init(&dir);
    });
}

fn temp_db(release: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tiez-upgrade-{}-{}-{}", release, std::process::id(), rand_suffix()));
    std::fs::create_dir_all(&dir).unwrap();
    dir.join("clipboard.db")
}

fn rand_suffix() -> u128 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
}

/// Builds the database the given release would have left on disk.
fn write_legacy_db(path: &Path, migrate: Migrate, settings_sql: &str) {
    let conn = Connection::open(path).unwrap();
    migrate(&conn).unwrap();
    conn.execute_batch(settings_sql).unwrap();
    for (key, value) in USER_SETTINGS {
        conn.execute("INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)", [key, value]).unwrap();
    }
    for s in SAMPLES {
        let tags_json = serde_json::to_string(s.tags).unwrap();
        conn.execute(
            "INSERT INTO clipboard_history (id, content_type, content, source_app, timestamp, preview, is_pinned, tags, use_count, pinned_order, html_content)
             VALUES (?1, ?2, ?3, 'Notes', ?4, ?3, ?5, ?6, 0, ?7, ?8)",
            rusqlite::params![s.id, s.content_type, s.content, s.timestamp, s.pinned as i32, tags_json, s.pinned_order, s.html],
        )
        .unwrap();
        for tag in s.tags {
            conn.execute("INSERT INTO entry_tags (entry_id, tag) VALUES (?1, ?2)", rusqlite::params![s.id, tag]).unwrap();
        }
    }
}

/// The value a release's seed SQL gives `key`, if it seeds it at all.
fn legacy_default(settings_sql: &str, key: &str) -> Option<String> {
    let needle = format!("VALUES ('{key}', '");
    let start = settings_sql.find(&needle)? + needle.len();
    let end = settings_sql[start..].find('\'')?;
    Some(settings_sql[start..start + end].to_string())
}

/// The current app's startup path for an existing database (see app/setup.rs).
fn upgrade(path: &Path) -> Connection {
    let conn = database::init_db(&path.to_string_lossy()).unwrap();
    encryption::migrate_legacy_ciphertexts(&conn).unwrap();
    conn
}

fn latest_schema_version() -> i64 {
    let conn = Connection::open_in_memory().unwrap();
    crate::infrastructure::repository::migrations::run_migrations(&conn).unwrap();
    conn.query_row("SELECT MAX(version) FROM schema_migrations", [], |r| r.get(0)).unwrap()
}

fn check_upgrade(release: &str, migrate: Migrate, settings_sql: &str) {
    init_encryption();
    let path = temp_db(release);
    write_legacy_db(&path, migrate, settings_sql);

    let conn = upgrade(&path);
    let ctx = |what: &str| format!("upgrade from {release}: {what}");

    // Schema is current.
    let version: i64 = conn.query_row("SELECT MAX(version) FROM schema_migrations", [], |r| r.get(0)).unwrap();
    assert_eq!(version, latest_schema_version(), "{}", ctx("schema version"));

    // Every record is intact and readable through the repository.
    let conn = Arc::new(Mutex::new(conn));
    let repo = SqliteClipboardRepository::new(conn.clone());
    assert_eq!(repo.get_count().unwrap(), SAMPLES.len() as i64, "{}", ctx("record count"));
    for s in SAMPLES {
        let entry = repo.get_entry_by_id(s.id).unwrap().unwrap_or_else(|| panic!("{}", ctx(&format!("record {} missing", s.id))));
        assert_eq!(entry.content, s.content, "{}", ctx(&format!("record {} content", s.id)));
        assert_eq!(entry.html_content.as_deref(), s.html, "{}", ctx(&format!("record {} html", s.id)));
        assert_eq!(entry.is_pinned, s.pinned, "{}", ctx(&format!("record {} pinned", s.id)));
        let mut tags = entry.tags.clone();
        tags.sort();
        assert_eq!(tags, s.tags.iter().map(|t| t.to_string()).collect::<Vec<_>>(), "{}", ctx(&format!("record {} tags", s.id)));
        assert_eq!(entry.note, "", "{}", ctx(&format!("record {} note default", s.id)));
        assert_eq!(entry.sort_at, s.timestamp, "{}", ctx(&format!("record {} sort_at backfill", s.id)));
        assert_eq!(entry.created_at, s.timestamp, "{}", ctx(&format!("record {} created_at backfill", s.id)));
    }

    // Pinned order survives, and pinned records still list first.
    let pinned_order: i64 = conn.lock().unwrap()
        .query_row("SELECT pinned_order FROM clipboard_history WHERE id = 2", [], |r| r.get(0)).unwrap();
    assert_eq!(pinned_order, 3, "{}", ctx("pinned_order"));
    let first = repo.get_history(10, 0, None).unwrap();
    assert_eq!(first.first().map(|e| e.id), Some(2), "{}", ctx("pinned record lists first"));

    // Search by text and by tag.
    let hits: Vec<i64> = repo.search("周会", 50, false, false).unwrap().iter().map(|e| e.id).collect();
    assert!(hits.contains(&1), "{}", ctx(&format!("text search, got {hits:?}")));
    let hits: Vec<i64> = repo.search("工作", 50, true, false).unwrap().iter().map(|e| e.id).collect();
    assert_eq!(hits, vec![2], "{}", ctx("tag search"));

    // A sensitive plaintext record can be encrypted at rest (what the startup
    // alignment pass does) and still reads back.
    {
        let guard = conn.lock().unwrap();
        repo.encrypt_entry_with_conn(&guard, 4).unwrap();
        let raw: String = guard.query_row("SELECT content FROM clipboard_history WHERE id = 4", [], |r| r.get(0)).unwrap();
        assert!(encryption::is_modern_ciphertext(&raw), "{}", ctx("sensitive record encrypted at rest"));
    }
    assert_eq!(repo.get_entry_by_id(4).unwrap().unwrap().content, SAMPLES[3].content, "{}", ctx("sensitive record readable"));

    // Customised settings kept; plaintext secrets now encrypted but readable;
    // new defaults added.
    let settings = SqliteSettingsRepository::new(conn.clone());
    for (key, value) in USER_SETTINGS {
        assert_eq!(settings.get(key).unwrap().as_deref(), Some(*value), "{}", ctx(&format!("setting {key}")));
    }
    for key in ["mqtt_password", "cloud_sync_webdav_password"] {
        let raw: String = conn.lock().unwrap()
            .query_row("SELECT value FROM settings WHERE key = ?1", [key], |r| r.get(0)).unwrap();
        assert!(encryption::is_modern_ciphertext(&raw), "{}", ctx(&format!("{key} encrypted at rest")));
    }
    assert!(settings.get("app.arrow_key_selection").unwrap().is_some(), "{}", ctx("new defaults seeded"));

    // Decided policy: upgraders keep the defaults they were installed with,
    // even where new installs get a different one (old port 12345 stays).
    for key in ["file_server_port", "cloud_sync_snapshot_interval_min"] {
        if let Some(old) = legacy_default(settings_sql, key) {
            assert_eq!(settings.get(key).unwrap().as_deref(), Some(old.as_str()), "{}", ctx(&format!("{key} keeps the old default")));
        }
    }
    for (key, _, existing) in database::CHANGED_DEFAULTS {
        if USER_SETTINGS.iter().any(|(k, _)| k == key) {
            continue;
        }
        // Saved (seeded by that release) or never stored: the old behaviour either way.
        let expected = legacy_default(settings_sql, key).unwrap_or_else(|| existing.to_string());
        assert_eq!(settings.get(key).unwrap(), Some(expected), "{}", ctx(&format!("{key} keeps the old default")));
    }
    assert_eq!(settings.get(database::AUTOSTART_PENDING_KEY).unwrap(), None, "{}", ctx("login item left alone"));

    // A backup was taken before rewriting secrets.
    let backup = PathBuf::from(format!("{}.pre-encryption-v1.bak", path.display()));
    assert!(backup.exists(), "{}", ctx("pre-migration backup"));

    // Starting again is a no-op.
    drop(settings);
    drop(repo);
    drop(conn);
    let conn = upgrade(&path);
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM clipboard_history", [], |r| r.get(0)).unwrap();
    assert_eq!(count, SAMPLES.len() as i64, "{}", ctx("second startup keeps records"));

    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}

#[test]
fn new_install_gets_new_defaults_and_launches_at_login() {
    let path = temp_db("fresh");
    let conn = upgrade(&path);
    let get = |key: &str| -> Option<String> {
        conn.query_row("SELECT value FROM settings WHERE key = ?1", [key], |r| r.get(0)).ok()
    };
    for (key, new_install, _) in database::CHANGED_DEFAULTS {
        assert_eq!(get(key).as_deref(), Some(*new_install), "{key}");
    }
    assert_eq!(get(database::AUTOSTART_PENDING_KEY).as_deref(), Some("true"));
    assert_eq!(get("file_server_port").as_deref(), Some("28462"));
    assert_eq!(get("cloud_sync_snapshot_interval_min").as_deref(), Some("720"));
    // Restarting is not a new install.
    drop(conn);
    let conn = upgrade(&path);
    conn.execute("DELETE FROM settings WHERE key = ?1", [database::AUTOSTART_PENDING_KEY]).unwrap();
    drop(conn);
    let conn = upgrade(&path);
    let pending: Option<String> = conn
        .query_row("SELECT value FROM settings WHERE key = ?1", [database::AUTOSTART_PENDING_KEY], |r| r.get(0))
        .ok();
    assert_eq!(pending, None, "autostart is only turned on once");
    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}

#[test]
fn upgrades_from_every_release_keep_records_settings_and_search() {
    for (release, migrate, settings_sql) in RELEASES {
        check_upgrade(release, *migrate, settings_sql);
    }
}
