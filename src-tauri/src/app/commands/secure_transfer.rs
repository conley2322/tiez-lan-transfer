//! Password-protected text-history transfer. No settings or attachment extraction.
use crate::{database::DbState, domain::models::ClipboardEntry, infrastructure::{encryption, repository::clipboard_repo::SqliteClipboardRepository}};
use ring::pbkdf2;
use serde::{Deserialize, Serialize};
use std::{io::{Read, Write}, num::NonZeroU32, fs::OpenOptions};

const MAGIC: &[u8; 8] = b"TIEZXFR\x01";
const HEADER: usize = 36; // authenticated magic/version, 16-byte salt, 12-byte nonce
const MAX: usize = 64 * 1024 * 1024;
const MAX_ROWS: usize = 50_000;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Archive { entries: Vec<ClipboardEntry>, excluded: usize }
#[derive(Serialize)]
pub struct TransferReport { transferred: usize, excluded: usize }

fn password_key(password: &str, salt: &[u8]) -> Result<[u8; 32], String> {
    if password.len() < 12 || password.len() > 1024 { return Err("Password must be 12–1024 UTF-8 bytes".into()); }
    let mut key = [0; 32];
    pbkdf2::derive(pbkdf2::PBKDF2_HMAC_SHA256, NonZeroU32::new(600_000).unwrap(), salt, password.as_bytes(), &mut key);
    Ok(key)
}
fn seal(plain: &[u8], password: &str) -> Result<Vec<u8>, String> {
    if plain.len() > MAX { return Err("Archive exceeds 64 MiB".into()); }
    let mut header = [0; HEADER];
    header[..8].copy_from_slice(MAGIC);
    encryption::fill_random(&mut header[8..]).ok_or("Random generator unavailable")?;
    let mut key = password_key(password, &header[8..24])?;
    let body = encryption::aes_gcm_seal(&key, &header[24..], &header, plain);
    key.fill(0);
    let body = body.ok_or("Encryption failed")?;
    let mut result = header.to_vec(); result.extend(body); Ok(result)
}
fn unseal(bytes: &[u8], password: &str) -> Result<Vec<u8>, String> {
    if bytes.len() < HEADER + 16 || bytes.len() > MAX + HEADER + 16 { return Err("Invalid archive size".into()); }
    if &bytes[..8] != MAGIC { return Err("Unsupported archive format or version".into()); }
    let mut key = password_key(password, &bytes[8..24])?;
    let plain = encryption::aes_gcm_open(&key, &bytes[24..HEADER], &bytes[..HEADER], &bytes[HEADER..]);
    key.fill(0);
    plain.ok_or_else(|| "Wrong password or damaged archive".into())
}
fn supported(entry: &ClipboardEntry) -> bool {
    !entry.is_external && matches!(entry.content_type.as_str(), "text" | "code" | "url" | "rich_text")
}
fn snapshot(conn: &rusqlite::Connection) -> Result<Archive, String> {
    let mut stmt = conn.prepare("SELECT content_type, content, preview, html_content, source_app, timestamp, created_at, last_used_at, sort_at, is_pinned, tags, note, use_count, is_external, pinned_order FROM clipboard_history").map_err(|_| "Cannot read history")?;
    let mut rows = stmt.query([]).map_err(|_| "Cannot read history")?;
    let mut archive = Archive { entries: Vec::new(), excluded: 0 };
    let mut size = 0;
    while let Some(row) = rows.next().map_err(|_| "Cannot read history")? {
        let read = || -> rusqlite::Result<ClipboardEntry> { Ok(ClipboardEntry {
            id: 0, content_type: row.get(0)?, content: row.get(1)?, preview: row.get(2)?, html_content: row.get(3)?, source_app: row.get(4)?, timestamp: row.get(5)?, created_at: row.get(6)?, last_used_at: row.get(7)?, sort_at: row.get(8)?, is_pinned: row.get(9)?, tags: serde_json::from_str(&row.get::<_, String>(10)?).map_err(|_| rusqlite::Error::InvalidQuery)?, note: row.get(11)?, use_count: row.get(12)?, is_external: row.get(13)?, pinned_order: row.get(14)?, source_app_path: None, file_preview_exists: false,
        }) };
        let mut entry = read().map_err(|_| "Invalid history record")?;
        // Fail closed even for excluded entries if any stored value is locked.
        entry.content = encryption::decrypt_stored_value(&entry.content)?;
        entry.preview = encryption::decrypt_stored_value(&entry.preview)?;
        entry.html_content = entry.html_content.as_deref().map(encryption::decrypt_stored_value).transpose()?;
        if !supported(&entry) { archive.excluded += 1; continue; }
        size += serde_json::to_vec(&entry).map_err(|_| "Cannot encode record")?.len();
        if size > MAX || archive.entries.len() >= MAX_ROWS { return Err("History exceeds transfer limit (64 MiB / 50,000 entries)".into()); }
        archive.entries.push(entry);
    }
    Ok(archive)
}

#[tauri::command]
pub async fn export_secure_transfer(state: tauri::State<'_, DbState>, path: String, password: String) -> Result<TransferReport, String> {
    let conn = state.conn.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let archive = { let guard = conn.lock().map_err(|_| "Database busy")?; snapshot(&guard)? };
        let plain = serde_json::to_vec(&archive).map_err(|_| "Cannot encode archive")?;
        let bytes = seal(&plain, &password)?;
        let mut options = OpenOptions::new(); options.write(true).create_new(true);
        #[cfg(unix)] { use std::os::unix::fs::OpenOptionsExt; options.mode(0o600); }
        // Exclusive creation: never truncate or overwrite an existing file/symlink.
        let mut file = options.open(&path).map_err(|_| "Cannot create archive; choose a new filename")?;
        file.write_all(&bytes).and_then(|_| file.sync_all()).map_err(|_| "Archive write failed; incomplete encrypted file may remain")?;
        Ok(TransferReport { transferred: archive.entries.len(), excluded: archive.excluded })
    }).await.map_err(|_| "Transfer worker failed")?
}

#[tauri::command]
pub async fn import_secure_transfer(state: tauri::State<'_, DbState>, path: String, password: String, acknowledge_plaintext: bool) -> Result<TransferReport, String> {
    if !acknowledge_plaintext { return Err("Confirm the destination storage warning first".into()); }
    let conn = state.conn.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let file = std::fs::File::open(path).map_err(|_| "Cannot open archive")?;
        if !file.metadata().map_err(|_| "Cannot inspect archive")?.is_file() { return Err("Archive must be a regular file".into()); }
        let mut bytes = Vec::new();
        file.take((MAX + HEADER + 17) as u64).read_to_end(&mut bytes).map_err(|_| "Cannot read archive")?;
        let plain = unseal(&bytes, &password)?;
        let archive: Archive = serde_json::from_slice(&plain).map_err(|_| "Invalid archive contents")?;
        if archive.entries.len() > MAX_ROWS || archive.entries.iter().any(|e| !supported(e)) { return Err("Unsupported archive entries".into()); }
        let repo = SqliteClipboardRepository::new(conn.clone());
        let mut guard = conn.lock().map_err(|_| "Database busy")?;
        let tx = guard.transaction().map_err(|_| "Cannot start import")?;
        for mut entry in archive.entries.iter().cloned() {
            entry.id = 0; entry.source_app_path = None;
            // Preserve literal plaintext, including values beginning with encryption prefixes.
            // Compute receiver protection before writing; failures roll back the whole merge.
            let protected = if crate::database::has_sensitive_tag(&entry.tags) {
                Some((encryption::encrypt_for_storage(&entry.content)?, encryption::encrypt_for_storage(&entry.preview)?, entry.html_content.as_deref().map(encryption::encrypt_for_storage).transpose()?))
            } else { None };
            let id = repo.save_with_conn(&tx, &entry, None, None)?;
            if let Some((content, preview, html)) = protected {
                tx.execute("UPDATE clipboard_history SET content=?1, preview=?2, html_content=?3 WHERE id=?4", rusqlite::params![content, preview, html, id]).map_err(|_| "Cannot protect imported entry")?;
            }
        }
        tx.commit().map_err(|_| "Cannot commit import")?;
        Ok(TransferReport { transferred: archive.entries.len(), excluded: archive.excluded })
    }).await.map_err(|_| "Transfer worker failed")?
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn authenticated_roundtrip() {
        let password = "a long test password";
        let archive = seal(b"private clipboard", password).unwrap();
        assert_eq!(unseal(&archive, password).unwrap(), b"private clipboard");
        assert!(unseal(&archive, "wrong long password").is_err());
        for offset in [8, 24, HEADER, archive.len()-1] { let mut altered = archive.clone(); altered[offset] ^= 1; assert!(unseal(&altered, password).is_err()); }
        let mut version = archive.clone(); version[7] = 2; assert!(unseal(&version, password).is_err());
        assert!(unseal(&archive[..20], password).is_err());
        assert_ne!(archive, seal(b"private clipboard", password).unwrap());
    }
}
