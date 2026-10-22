//! Local-at-rest encryption for sensitive clipboard / settings values.
//!
//! Format: `enc1:` + base64(nonce || ciphertext)  (AES-256-GCM)
//! Legacy:
//! - `plain:` + plaintext
//! - `dpapi:` + base64(CryptProtectData output) from older Windows builds
//!
//! Master key is stored in macOS Keychain or Windows DPAPI-protected file.

use base64::Engine;
use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};
use ring::rand::{SecureRandom, SystemRandom};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Current ciphertext prefix (AES-GCM).
pub const ENCRYPT_PREFIX: &str = "enc1:";
/// Legacy fake-encryption prefix from earlier builds.
pub const LEGACY_PLAIN_PREFIX: &str = "plain:";
/// Legacy per-value Windows DPAPI format used before the shared AES key format.
pub const LEGACY_DPAPI_PREFIX: &str = "dpapi:";

#[cfg(all(target_os = "macos", not(debug_assertions)))]
const KEYCHAIN_SERVICE: &str = "com.tiez.clipboard";
#[cfg(all(target_os = "macos", not(debug_assertions)))]
const KEYCHAIN_ACCOUNT: &str = "master-key-v1";
#[cfg(target_os = "windows")]
const DPAPI_KEY_FILE: &str = "master.key.dpapi";
/// Dev / non-OS-keystore fallback (0600 file under app data dir).
#[cfg(any(
    not(any(target_os = "macos", target_os = "windows")),
    all(target_os = "macos", debug_assertions)
))]
const FALLBACK_KEY_FILE: &str = "master.key";
const NONCE_LEN: usize = 12;
const KEY_LEN: usize = 32;

// Option so a failed load (e.g. user denied Keychain access) is cached and we
// never re-prompt within the same session.
static MASTER_KEY: OnceLock<Option<[u8; KEY_LEN]>> = OnceLock::new();
static DATA_DIR: OnceLock<PathBuf> = OnceLock::new();

pub(crate) fn fill_random(buf: &mut [u8]) -> Option<()> {
    SystemRandom::new().fill(buf).ok()
}

/// AES-256-GCM; output is `ciphertext || 16-byte tag`.
pub(crate) fn aes_gcm_seal(key: &[u8], nonce: &[u8], aad: &[u8], plain: &[u8]) -> Option<Vec<u8>> {
    let key = LessSafeKey::new(UnboundKey::new(&AES_256_GCM, key).ok()?);
    let nonce = Nonce::try_assume_unique_for_key(nonce).ok()?;
    let mut buf = plain.to_vec();
    key.seal_in_place_append_tag(nonce, Aad::from(aad), &mut buf).ok()?;
    Some(buf)
}

pub(crate) fn aes_gcm_open(key: &[u8], nonce: &[u8], aad: &[u8], sealed: &[u8]) -> Option<Vec<u8>> {
    let key = LessSafeKey::new(UnboundKey::new(&AES_256_GCM, key).ok()?);
    let nonce = Nonce::try_assume_unique_for_key(nonce).ok()?;
    let mut buf = sealed.to_vec();
    let len = key.open_in_place(nonce, Aad::from(aad), &mut buf).ok()?.len();
    buf.truncate(len);
    Some(buf)
}

/// True if value is protected (modern or legacy wrapper).
pub fn is_encrypted_value(value: &str) -> bool {
    value.starts_with(ENCRYPT_PREFIX)
        || value.starts_with(LEGACY_PLAIN_PREFIX)
        || value.starts_with(LEGACY_DPAPI_PREFIX)
}

/// True if value uses the real AES-GCM format.
pub fn is_modern_ciphertext(value: &str) -> bool {
    value.starts_with(ENCRYPT_PREFIX)
}

/// Initialize encryption with the app data directory (call once at startup).
///
/// On macOS release builds, Keychain access is deferred to a background thread
/// so the auth dialog cannot deadlock the Tauri setup / main run loop.
pub fn init(data_dir: &Path) {
    let _ = DATA_DIR.set(data_dir.to_path_buf());

    // Portable saves are plaintext. Load an existing key only on a decrypt request.
    if cfg!(feature = "portable") {
        return;
    }

    #[cfg(all(target_os = "macos", not(debug_assertions)))]
    {
        // Prewarm off the main thread: SecurityAgent UI needs a free run loop.
        std::thread::spawn(|| {
            let _ = get_master_key();
        });
    }

    #[cfg(not(all(target_os = "macos", not(debug_assertions))))]
    {
        let _ = get_master_key();
    }
}

fn get_master_key() -> Option<&'static [u8; KEY_LEN]> {
    MASTER_KEY
        .get_or_init(|| match load_or_create_master_key() {
            Ok(key) => Some(key),
            Err(e) => {
                eprintln!(">>> [ENCRYPTION] master key unavailable this session: {e}");
                None
            }
        })
        .as_ref()
}

#[allow(dead_code)]
fn data_dir() -> Option<&'static PathBuf> {
    // Tests share one process and run in any order: one that reaches the key
    // before another calls init() cached "no key" for all of them.
    #[cfg(test)]
    return Some(DATA_DIR.get_or_init(|| {
        std::env::temp_dir().join(format!("tiez-test-encryption-{}", std::process::id()))
    }));
    #[cfg(not(test))]
    DATA_DIR.get()
}

fn load_or_create_master_key() -> Result<[u8; KEY_LEN], String> {
    // Debug macOS: never touch Keychain. Dev binaries re-sign every rebuild, so
    // Keychain prompts loop / deadlock the main thread during Tauri setup.
    #[cfg(all(target_os = "macos", debug_assertions))]
    {
        eprintln!(
            ">>> [ENCRYPTION] debug build: using local master.key (Keychain disabled)"
        );
        return load_or_create_file_key(FALLBACK_KEY_FILE);
    }

    #[cfg(all(target_os = "macos", not(debug_assertions)))]
    {
        match load_key_from_keychain() {
            Ok(key) => return Ok(key),
            // Only create a fresh key when the item truly does not exist.
            // Access-denied / user-cancel must NOT overwrite the existing key,
            // otherwise previously encrypted data becomes undecryptable.
            Err(KeychainLoadError::NotFound) => {
                if cfg!(feature = "portable") {
                    return Err("existing master key not found".into());
                }
                // The key stays in the Keychain of the Mac that made it, so
                // a data folder moved to a new Mac brings values nothing here
                // can open. Refusing a new key left every password save
                // failing; as on Windows, start one and the lost values are
                // re-entered. The old ones stay stored, untouched.
                if ensure_no_orphaned_ciphertext().is_err() {
                    eprintln!(
                        ">>> [ENCRYPTION] encrypted data from another Mac; starting a new master key"
                    );
                }
                let key = generate_key();
                store_key_in_keychain(&key)?;
                return Ok(key);
            }
            Err(KeychainLoadError::Other(msg)) => return Err(msg),
        }
    }

    #[cfg(target_os = "windows")]
    {
        let dir = data_dir().ok_or_else(|| "encryption data dir not set".to_string())?;
        let path = dir.join(DPAPI_KEY_FILE);
        if path.exists() {
            match load_key_dpapi(&path) {
                Ok(key) => return Ok(key),
                // The key file came from another Windows install or account
                // (reinstalled system, data folder restored from a backup).
                // Nothing on this machine can open it or what it encrypted,
                // so refusing a new key only left every save failing. Keep
                // the file, start a new key; the lost values are re-entered.
                Err(KeyFileError::FromAnotherInstall) if !cfg!(feature = "portable") => {
                    let kept = keep_unreadable_key_file(&path)?;
                    eprintln!(
                        ">>> [ENCRYPTION] master key is from another Windows install; kept as {} and starting a new one",
                        kept.display()
                    );
                    let key = generate_key();
                    store_key_dpapi(&path, &key)?;
                    return Ok(key);
                }
                Err(error) => return Err(error.to_string()),
            }
        }
        if cfg!(feature = "portable") {
            return Err("existing master key not found".into());
        }
        ensure_no_orphaned_ciphertext()?;
        let key = generate_key();
        store_key_dpapi(&path, &key)?;
        return Ok(key);
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        load_or_create_file_key(FALLBACK_KEY_FILE)
    }
}

fn ensure_no_orphaned_ciphertext() -> Result<(), String> {
    let Some(dir) = data_dir() else { return Ok(()); };
    let path = dir.join("clipboard.db");
    if !path.exists() { return Ok(()); }
    let conn = rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|e| format!("cannot verify existing encrypted database: {e}"))?;
    for (table, fields) in [("settings", vec!["value"]), ("clipboard_history", vec!["content", "preview", "html_content"])] {
        let exists: bool = conn.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1)", [table], |r| r.get(0)).map_err(|e| e.to_string())?;
        if !exists { continue; }
        for field in fields {
            let sql = format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE {field} LIKE 'enc1:%')");
            if conn.query_row(&sql, [], |r| r.get::<_, bool>(0)).map_err(|e| e.to_string())? {
                return Err("existing encrypted data has no accessible master key; restore the original key before saving".into());
            }
        }
    }
    Ok(())
}

#[cfg(any(
    not(any(target_os = "macos", target_os = "windows")),
    all(target_os = "macos", debug_assertions)
))]
fn load_or_create_file_key(filename: &str) -> Result<[u8; KEY_LEN], String> {
    let dir = data_dir().ok_or_else(|| "encryption data dir not set".to_string())?;
    let path = dir.join(filename);
    if path.exists() {
        let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
        if bytes.len() != KEY_LEN {
            return Err("invalid fallback key length".into());
        }
        let mut key = [0u8; KEY_LEN];
        key.copy_from_slice(&bytes);
        return Ok(key);
    }
    if cfg!(feature = "portable") {
        return Err("existing master key not found".into());
    }
    ensure_no_orphaned_ciphertext()?;
    let key = generate_key();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(&path, key).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    }
    Ok(key)
}

fn generate_key() -> [u8; KEY_LEN] {
    let mut key = [0u8; KEY_LEN];
    fill_random(&mut key).expect("system random generator unavailable");
    key
}

#[cfg(all(target_os = "macos", not(debug_assertions)))]
enum KeychainLoadError {
    NotFound,
    Other(String),
}

#[cfg(all(target_os = "macos", not(debug_assertions)))]
fn load_key_from_keychain() -> Result<[u8; KEY_LEN], KeychainLoadError> {
    use security_framework::passwords::get_generic_password;

    // errSecItemNotFound = -25300
    const ERR_SEC_ITEM_NOT_FOUND: i32 = -25300;

    let bytes = get_generic_password(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT).map_err(|e| {
        if e.code() == ERR_SEC_ITEM_NOT_FOUND {
            KeychainLoadError::NotFound
        } else {
            KeychainLoadError::Other(format!("keychain get: {e}"))
        }
    })?;
    if bytes.len() != KEY_LEN {
        return Err(KeychainLoadError::Other(
            "invalid keychain key length".into(),
        ));
    }
    let mut key = [0u8; KEY_LEN];
    key.copy_from_slice(&bytes);
    Ok(key)
}

#[cfg(all(target_os = "macos", not(debug_assertions)))]
fn store_key_in_keychain(key: &[u8; KEY_LEN]) -> Result<(), String> {
    use security_framework::passwords::set_generic_password;
    set_generic_password(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT, key)
        .map_err(|e| format!("keychain set: {e}"))
}

#[cfg(target_os = "windows")]
fn store_key_dpapi(path: &Path, key: &[u8; KEY_LEN]) -> Result<(), String> {
    let protected = dpapi_protect(key)?;
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(path, protected).map_err(|e| e.to_string())
}

/// Renames a key file nothing here can open (never deletes it: it is the only
/// way back to what it encrypted, should the old Windows account return) and
/// returns where it went.
#[cfg(any(target_os = "windows", test))]
fn keep_unreadable_key_file(path: &Path) -> Result<std::path::PathBuf, String> {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "invalid key file name".to_string())?;
    let mut kept = path.with_file_name(format!("{name}.unreadable"));
    let mut n = 1;
    while kept.exists() {
        kept = path.with_file_name(format!("{name}.unreadable.{n}"));
        n += 1;
    }
    std::fs::rename(path, &kept).map_err(|e| format!("cannot set aside unreadable key: {e}"))?;
    Ok(kept)
}

/// NTE_BAD_KEY_STATE, "Key not valid for use in specified state": DPAPI data
/// protected under a different Windows install or user account.
#[cfg(target_os = "windows")]
const NTE_BAD_KEY_STATE: i32 = 0x8009_000B_u32 as i32;

#[cfg(target_os = "windows")]
enum KeyFileError {
    FromAnotherInstall,
    Other(String),
}

#[cfg(target_os = "windows")]
impl std::fmt::Display for KeyFileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FromAnotherInstall => f.write_str("master key belongs to another Windows install or account"),
            Self::Other(message) => f.write_str(message),
        }
    }
}

#[cfg(target_os = "windows")]
fn load_key_dpapi(path: &Path) -> Result<[u8; KEY_LEN], KeyFileError> {
    let protected = std::fs::read(path).map_err(|e| KeyFileError::Other(e.to_string()))?;
    let plain = dpapi_unprotect_checked(&protected).map_err(|error| {
        if error.code().0 == NTE_BAD_KEY_STATE {
            KeyFileError::FromAnotherInstall
        } else {
            KeyFileError::Other(format!("CryptUnprotectData: {error}"))
        }
    })?;
    if plain.len() != KEY_LEN {
        return Err(KeyFileError::Other("invalid DPAPI key length".into()));
    }
    let mut key = [0u8; KEY_LEN];
    key.copy_from_slice(&plain);
    Ok(key)
}

#[cfg(target_os = "windows")]
fn dpapi_protect(data: &[u8]) -> Result<Vec<u8>, String> {
    use windows::Win32::Foundation::{LocalFree, HLOCAL};
    use windows::Win32::Security::Cryptography::{CryptProtectData, CRYPT_INTEGER_BLOB};

    unsafe {
        let mut input = CRYPT_INTEGER_BLOB {
            cbData: data.len() as u32,
            pbData: data.as_ptr() as *mut u8,
        };
        let mut output = CRYPT_INTEGER_BLOB {
            cbData: 0,
            pbData: std::ptr::null_mut(),
        };
        CryptProtectData(
            &mut input,
            windows::core::PCWSTR::null(),
            None,
            None,
            None,
            0,
            &mut output,
        )
        .map_err(|e| format!("CryptProtectData: {e}"))?;

        let slice = std::slice::from_raw_parts(output.pbData, output.cbData as usize);
        let result = slice.to_vec();
        if !output.pbData.is_null() {
            let _ = LocalFree(Some(HLOCAL(output.pbData as *mut _)));
        }
        Ok(result)
    }
}

#[cfg(target_os = "windows")]
fn dpapi_unprotect(data: &[u8]) -> Result<Vec<u8>, String> {
    dpapi_unprotect_checked(data).map_err(|e| format!("CryptUnprotectData: {e}"))
}

#[cfg(target_os = "windows")]
fn dpapi_unprotect_checked(data: &[u8]) -> windows::core::Result<Vec<u8>> {
    use windows::Win32::Foundation::{LocalFree, HLOCAL};
    use windows::Win32::Security::Cryptography::{CryptUnprotectData, CRYPT_INTEGER_BLOB};

    unsafe {
        let mut input = CRYPT_INTEGER_BLOB {
            cbData: data.len() as u32,
            pbData: data.as_ptr() as *mut u8,
        };
        let mut output = CRYPT_INTEGER_BLOB {
            cbData: 0,
            pbData: std::ptr::null_mut(),
        };
        CryptUnprotectData(
            &mut input,
            None,
            None,
            None,
            None,
            0,
            &mut output,
        )
        ?;

        let slice = std::slice::from_raw_parts(output.pbData, output.cbData as usize);
        let result = slice.to_vec();
        if !output.pbData.is_null() {
            let _ = LocalFree(Some(HLOCAL(output.pbData as *mut _)));
        }
        Ok(result)
    }
}

pub fn encrypt_value(plain: &str) -> Option<String> {
    #[cfg(feature = "portable")]
    {
        let _ = plain;
        return None;
    }
    #[cfg(not(feature = "portable"))]
    {
        let key = get_master_key()?;
        let mut nonce_bytes = [0u8; NONCE_LEN];
        fill_random(&mut nonce_bytes)?;
        let ciphertext = aes_gcm_seal(key, &nonce_bytes, &[], plain.as_bytes())?;
        let mut packed = Vec::with_capacity(NONCE_LEN + ciphertext.len());
        packed.extend_from_slice(&nonce_bytes);
        packed.extend_from_slice(&ciphertext);
        let encoded = base64::engine::general_purpose::STANDARD.encode(packed);
        Some(format!("{ENCRYPT_PREFIX}{encoded}"))
    }
}

pub fn decrypt_value(cipher: &str) -> Option<String> {
    decrypt_value_inner(cipher)
}

/// Decode one storage envelope. Modern plaintext is never recursively unwrapped.
pub fn decrypt_stored_value(value: &str) -> Result<String, String> {
    decrypt_value(value).ok_or_else(|| "stored value cannot be decrypted; original data preserved".into())
}

/// Encode literal plaintext for storage; portable builds deliberately save plaintext.
/// Call decrypt_stored_value explicitly before passing an existing storage envelope.
pub fn encrypt_for_storage(value: &str) -> Result<String, String> {
    #[cfg(feature = "portable")]
    { Ok(value.to_string()) }
    #[cfg(not(feature = "portable"))]
    { encrypt_value(value).ok_or_else(|| "encryption unavailable; value was not saved".into()) }
}

fn decrypt_value_inner(cipher: &str) -> Option<String> {
    if let Some(payload) = cipher.strip_prefix(LEGACY_PLAIN_PREFIX) {
        return Some(payload.to_string());
    }

    if let Some(payload) = cipher.strip_prefix(LEGACY_DPAPI_PREFIX) {
        #[cfg(target_os = "windows")]
        {
            let protected = base64::engine::general_purpose::STANDARD
                .decode(payload.as_bytes())
                .ok()?;
            let plain = dpapi_unprotect(&protected).ok()?;
            return String::from_utf8(plain).ok();
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = payload;
            return None;
        }
    }

    if !cipher.starts_with(ENCRYPT_PREFIX) {
        // Unprefixed plaintext (or unknown format)
        return Some(cipher.to_string());
    }

    let plain = decrypt_modern_value_once(cipher)?;
    // Recover old Windows builds that wrapped a DPAPI envelope in AES.
    // Only unwrap an authenticated, locally decryptable DPAPI payload.
    if plain.starts_with(LEGACY_DPAPI_PREFIX) {
        if let Some(legacy) = decrypt_value_inner(&plain) { return Some(legacy); }
    }
    Some(plain)
}

fn decrypt_modern_value_once(cipher: &str) -> Option<String> {
    let payload = cipher.strip_prefix(ENCRYPT_PREFIX)?;
    let packed = base64::engine::general_purpose::STANDARD
        .decode(payload.as_bytes())
        .ok()?;
    if packed.len() <= NONCE_LEN {
        return None;
    }
    let key = get_master_key()?;
    let (nonce_bytes, ciphertext) = packed.split_at(NONCE_LEN);
    let plain = aes_gcm_open(key, nonce_bytes, &[], ciphertext)?;
    String::from_utf8(plain).ok()
}

/// Re-encrypt legacy `plain:` / Windows `dpapi:` blobs when they can be
/// decrypted. Undecryptable ciphertext is intentionally left untouched.
pub fn migrate_legacy_ciphertexts(conn: &rusqlite::Connection) -> Result<usize, String> {
    if cfg!(feature = "portable") {
        return Ok(0);
    }
    backup_before_migration(conn)?;
    let transaction = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    let upgraded = migrate_legacy_in_transaction(&transaction)?;
    transaction.commit().map_err(|e| e.to_string())?;
    Ok(upgraded)
}

// VACUUM INTO takes a consistent SQLite snapshot, including committed WAL data.
// It must run outside the migration transaction. Never overwrite an old backup.
fn backup_before_migration(conn: &rusqlite::Connection) -> Result<(), String> {
    let path: String = conn.query_row("SELECT file FROM pragma_database_list WHERE name = 'main'", [], |row| row.get(0))
        .map_err(|e| e.to_string())?;
    if path.is_empty() { return Ok(()); } // In-memory tests have no durable data.
    let backup = format!("{path}.pre-encryption-v1.bak");
    if std::path::Path::new(&backup).exists() { return Ok(()); }
    conn.execute("VACUUM main INTO ?1", [&backup]).map_err(|e| format!("encryption backup failed: {e}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&backup, std::fs::Permissions::from_mode(0o600))
            .map_err(|e| format!("cannot restrict backup permissions: {e}"))?;
    }
    Ok(())
}

fn migrate_legacy_in_transaction(conn: &rusqlite::Connection) -> Result<usize, String> {
    let mut upgraded = 0usize;

    // Settings: sensitive keys with legacy or plaintext values are rewritten via encrypt.
    {
        let mut stmt = conn
            .prepare("SELECT key, value FROM settings")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|e| e.to_string())?;

        let mut updates: Vec<(String, String)> = Vec::new();
        for row in rows {
            let (key, value) = row.map_err(|e| e.to_string())?;
            if !crate::database::is_sensitive_key(&key) {
                continue;
            }
            let upgraded_value = upgrade_field(&value);
            if upgraded_value != value {
                updates.push((key, upgraded_value));
                continue;
            }
            if is_modern_ciphertext(&value) {
                continue;
            }
            let plain = if is_encrypted_value(&value) {
                let Some(plain) = decrypt_value(&value) else {
                    continue;
                };
                plain
            } else {
                value
            };
            if let Some(enc) = encrypt_value(&plain) {
                updates.push((key, enc));
            }
        }
        for (key, enc) in updates {
            conn.execute(
                "UPDATE settings SET value = ?1 WHERE key = ?2",
                rusqlite::params![enc, key],
            )
            .map_err(|e| e.to_string())?;
            upgraded += 1;
        }
    }

    // Clipboard text fields with legacy plain: prefix
    {
        let mut stmt = conn
            .prepare(
                "SELECT id, content, preview, html_content FROM clipboard_history
                 WHERE content LIKE 'plain:%'
                    OR preview LIKE 'plain:%'
                    OR html_content LIKE 'plain:%'
                    OR content LIKE 'dpapi:%'
                    OR preview LIKE 'dpapi:%'
                    OR html_content LIKE 'dpapi:%'
                    OR content LIKE 'enc1:%'
                    OR preview LIKE 'enc1:%'
                    OR html_content LIKE 'enc1:%'",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            })
            .map_err(|e| e.to_string())?;

        let mut updates: Vec<(i64, String, String, Option<String>)> = Vec::new();
        for row in rows {
            let (id, content, preview, html) = row.map_err(|e| e.to_string())?;
            let new_content = upgrade_field(&content);
            let new_preview = upgrade_field(&preview);
            let new_html = html.as_ref().map(|h| upgrade_field(h));
            if new_content != content
                || new_preview != preview
                || new_html.as_ref() != html.as_ref()
            {
                updates.push((id, new_content, new_preview, new_html));
            }
        }
        for (id, content, preview, html) in updates {
            conn.execute(
                "UPDATE clipboard_history SET content = ?1, preview = ?2, html_content = ?3 WHERE id = ?4",
                rusqlite::params![content, preview, html, id],
            )
            .map_err(|e| e.to_string())?;
            upgraded += 1;
        }
    }

    Ok(upgraded)
}

fn upgrade_field(value: &str) -> String {
    if is_modern_ciphertext(value) {
        return value.to_string();
    }
    if value.starts_with(LEGACY_PLAIN_PREFIX) || value.starts_with(LEGACY_DPAPI_PREFIX) {
        let Some(plain) = decrypt_value(value) else {
            return value.to_string();
        };
        return encrypt_value(&plain).unwrap_or_else(|| value.to_string());
    }
    value.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Only Windows saying "another install or account" sets a key aside; a
    /// damaged file is an ordinary error, so a fresh key never replaces it.
    #[cfg(target_os = "windows")]
    #[test]
    fn a_damaged_key_file_is_not_treated_as_from_another_install() {
        let dir = std::env::temp_dir().join(format!("tiez-damaged-key-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(DPAPI_KEY_FILE);
        std::fs::write(&path, b"not a DPAPI blob").unwrap();
        assert!(matches!(load_key_dpapi(&path), Err(KeyFileError::Other(_))));

        // A key protected by this account still loads.
        let key = [7u8; KEY_LEN];
        store_key_dpapi(&path, &key).unwrap();
        assert!(matches!(load_key_dpapi(&path), Ok(loaded) if loaded == key));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn unreadable_key_files_are_set_aside_never_deleted() {
        let dir = std::env::temp_dir().join(format!("tiez-unreadable-key-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("master.key.dpapi");

        std::fs::write(&path, b"old install 1").unwrap();
        let first = keep_unreadable_key_file(&path).unwrap();
        std::fs::write(&path, b"old install 2").unwrap();
        let second = keep_unreadable_key_file(&path).unwrap();

        assert!(!path.exists());
        assert_eq!(first.file_name().unwrap(), "master.key.dpapi.unreadable");
        assert_eq!(second.file_name().unwrap(), "master.key.dpapi.unreadable.1");
        assert_eq!(std::fs::read(&first).unwrap(), b"old install 1");
        assert_eq!(std::fs::read(&second).unwrap(), b"old install 2");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// What Windows 0.3.3 stored: "dpapi:" + base64(CryptProtectData(text)),
    /// user scope, CRYPTPROTECT_UI_FORBIDDEN, no entropy.
    #[cfg(target_os = "windows")]
    fn v0_3_3_ciphertext(plain: &str) -> String {
        use base64::Engine;
        use windows::Win32::Foundation::{LocalFree, HLOCAL};
        use windows::Win32::Security::Cryptography::{
            CryptProtectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
        };
        unsafe {
            let mut input = CRYPT_INTEGER_BLOB { cbData: plain.len() as u32, pbData: plain.as_ptr() as *mut u8 };
            let mut output = CRYPT_INTEGER_BLOB { cbData: 0, pbData: std::ptr::null_mut() };
            CryptProtectData(
                &mut input,
                windows::core::PCWSTR::null(),
                None,
                None,
                None,
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
            .expect("CryptProtectData");
            let bytes = std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec();
            let _ = LocalFree(Some(HLOCAL(output.pbData as *mut _)));
            format!("dpapi:{}", base64::engine::general_purpose::STANDARD.encode(bytes))
        }
    }

    /// An upgrade from Windows 0.3.3 on the same machine: its DPAPI values
    /// (sensitive settings and records tagged sensitive) must come out readable
    /// and re-encrypted, not as errors.
    #[cfg(target_os = "windows")]
    #[test]
    fn upgrade_reads_and_reencrypts_windows_0_3_3_dpapi_values() {
        let secret = "mqtt-密码-123";
        let legacy = v0_3_3_ciphertext(secret);
        assert_eq!(decrypt_value(&legacy).as_deref(), Some(secret));

        let conn = rusqlite::Connection::open_in_memory().expect("open database");
        conn.execute_batch(
            "CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
             CREATE TABLE clipboard_history (
                 id INTEGER PRIMARY KEY, content TEXT NOT NULL, preview TEXT NOT NULL, html_content TEXT
             );",
        )
        .unwrap();
        conn.execute("INSERT INTO settings VALUES ('mqtt_password', ?1)", [&legacy]).unwrap();
        let record = v0_3_3_ciphertext("银行卡 6222");
        conn.execute("INSERT INTO clipboard_history VALUES (1, ?1, ?1, NULL)", [&record]).unwrap();

        assert_eq!(migrate_legacy_ciphertexts(&conn).expect("migrate"), 2);

        let setting: String = conn
            .query_row("SELECT value FROM settings WHERE key = 'mqtt_password'", [], |r| r.get(0))
            .unwrap();
        assert!(is_modern_ciphertext(&setting), "{setting}");
        assert_eq!(decrypt_value(&setting).as_deref(), Some(secret));
        let (content, preview): (String, String) = conn
            .query_row("SELECT content, preview FROM clipboard_history WHERE id = 1", [], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap();
        assert_eq!(decrypt_value(&content).as_deref(), Some("银行卡 6222"));
        assert_eq!(decrypt_value(&preview).as_deref(), Some("银行卡 6222"));
    }

    #[test]
    fn legacy_plain_decrypts() {
        assert_eq!(decrypt_value("plain:hello").as_deref(), Some("hello"));
        assert!(is_encrypted_value("plain:hello"));
        assert!(!is_modern_ciphertext("plain:hello"));
    }

    #[test]
    fn legacy_dpapi_is_recognized_without_being_modern_ciphertext() {
        assert!(is_encrypted_value("dpapi:AA=="));
        assert!(!is_modern_ciphertext("dpapi:AA=="));
    }

    #[test]
    fn migration_preserves_undecryptable_dpapi_values() {
        let conn = rusqlite::Connection::open_in_memory().expect("open database");
        conn.execute_batch(
            "CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
             CREATE TABLE clipboard_history (
                 id INTEGER PRIMARY KEY,
                 content TEXT NOT NULL,
                 preview TEXT NOT NULL,
                 html_content TEXT
             );
             INSERT INTO settings (key, value)
             VALUES ('mqtt_password', 'dpapi:not-valid-base64');
             INSERT INTO clipboard_history (id, content, preview, html_content)
             VALUES (
                 1,
                 'dpapi:not-valid-base64',
                 'dpapi:not-valid-base64',
                 'dpapi:not-valid-base64'
             );",
        )
        .expect("create legacy rows");

        assert_eq!(migrate_legacy_ciphertexts(&conn).expect("migrate"), 0);
        let setting: String = conn
            .query_row(
                "SELECT value FROM settings WHERE key = 'mqtt_password'",
                [],
                |row| row.get(0),
            )
            .expect("read setting");
        let fields: (String, String, Option<String>) = conn
            .query_row(
                "SELECT content, preview, html_content
                 FROM clipboard_history WHERE id = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("read clipboard row");

        assert_eq!(setting, "dpapi:not-valid-base64");
        assert_eq!(fields.0, "dpapi:not-valid-base64");
        assert_eq!(fields.1, "dpapi:not-valid-base64");
        assert_eq!(fields.2.as_deref(), Some("dpapi:not-valid-base64"));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn legacy_dpapi_round_trip_decrypts() {
        let protected = dpapi_protect(b"old-secret").expect("protect legacy value");
        let encoded = base64::engine::general_purpose::STANDARD.encode(protected);
        let legacy = format!("{LEGACY_DPAPI_PREFIX}{encoded}");
        assert_eq!(decrypt_value(&legacy).as_deref(), Some("old-secret"));
    }

    #[cfg(all(target_os = "windows", not(feature = "portable")))]
    #[test]
    fn modern_ciphertext_can_unwrap_a_legacy_dpapi_value() {
        let test_data_dir = std::env::temp_dir().join(format!(
            "tiez-encryption-test-{}",
            std::process::id()
        ));
        init(&test_data_dir);

        let protected = dpapi_protect(b"old-secret").expect("protect legacy value");
        let encoded = base64::engine::general_purpose::STANDARD.encode(protected);
        let legacy = format!("{LEGACY_DPAPI_PREFIX}{encoded}");
        let modern = encrypt_value(&legacy).expect("wrap legacy value");
        assert_eq!(decrypt_value(&modern).as_deref(), Some("old-secret"));
    }
}
