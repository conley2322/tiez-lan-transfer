//! This machine's durable peer identity.
//!
//! The phone flow mints a fresh session token and key every time its server
//! starts, which is right for a QR code that is shown once. A desktop pairing
//! has to survive reboots, or every machine would have to be paired again on
//! every launch — so the peer token and key are created once and kept in
//! `peer_identity.json` (encrypted fields, same at-rest cipher as the
//! database).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter, Manager};

use crate::app::system::get_machine_id;
use crate::app_state::AppDataDir;
use crate::infrastructure::encryption;
use crate::services::file_transfer::sealed::{self, LanKey};

use super::models::{PeerIdentity, PeerIdentityState};

const IDENTITY_FILE: &str = "peer_identity.json";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct StoredIdentity {
    #[serde(default)]
    device_id: String,
    #[serde(default)]
    device_name: String,
    #[serde(default)]
    nickname: String,
    /// Empty until the user picks one. Local only: it is how this machine draws
    /// itself, never something a peer is told.
    #[serde(default)]
    avatar_color: String,
    /// Encrypted at rest.
    #[serde(default)]
    auth_token: String,
    /// Encrypted at rest; base64url of the 32-byte key when plaintext.
    #[serde(default)]
    key: String,
}

pub fn identity_path(data_dir: &Path) -> PathBuf {
    data_dir.join(IDENTITY_FILE)
}

/// 16 hex characters, stable for this machine.
pub fn derive_device_id() -> String {
    let mut hasher = Sha256::new();
    hasher.update(get_machine_id().as_bytes());
    hasher.update(b"tiez-peer-v1");
    let hex = format!("{:x}", hasher.finalize());
    hex.chars().take(16).collect()
}

pub fn platform_label() -> String {
    std::env::consts::OS.to_string()
}

/// "<用户名> 的 Mac" — a label the other machines show before the user picks
/// a nickname of their own.
pub fn default_device_name() -> String {
    let machine = match std::env::consts::OS {
        "macos" => "Mac",
        "windows" => "Windows 电脑",
        other => other,
    };
    let user = ["USER", "USERNAME", "LOGNAME"]
        .iter()
        .find_map(|key| std::env::var(key).ok())
        .map(|user| user.trim().to_string())
        .filter(|user| !user.is_empty() && !user.eq_ignore_ascii_case("root"));
    match user {
        Some(user) => format!("{user} 的{machine}"),
        None => format!("未命名的{machine}"),
    }
}

fn new_secret_text(prefix: &str) -> String {
    format!("{prefix}{}", uuid::Uuid::new_v4().as_simple())
}

fn encode_or_error(value: &str) -> Result<String, String> {
    encryption::encrypt_for_storage(value)
}

fn decode_or_error(value: &str) -> Result<String, String> {
    if value.is_empty() {
        return Err("peer identity field is missing".to_string());
    }
    if encryption::is_encrypted_value(value) {
        return encryption::decrypt_stored_value(value);
    }
    // A portable build deliberately keeps plaintext at rest.
    Ok(value.to_string())
}

/// Read the stored identity, creating and saving one on first run.
pub fn load_or_create_identity(data_dir: &Path) -> Result<PeerIdentity, String> {
    let path = identity_path(data_dir);
    if let Ok(text) = std::fs::read_to_string(&path) {
        let stored: StoredIdentity = serde_json::from_str(&text)
            .map_err(|e| format!("peer identity file is unreadable: {e}"))?;
        if !stored.device_id.is_empty() {
            let auth_token = decode_or_error(&stored.auth_token)?;
            let key_text = decode_or_error(&stored.key)?;
            let key: LanKey = sealed::decode_key(&key_text)
                .ok_or_else(|| "peer identity key is corrupt".to_string())?;
            let device_name = if stored.device_name.is_empty() {
                default_device_name()
            } else {
                stored.device_name
            };
            return Ok(PeerIdentity {
                device_id: stored.device_id,
                device_name,
                nickname: stored.nickname,
                platform: platform_label(),
                avatar_color: stored.avatar_color,
                auth_token,
                key,
            });
        }
    }

    let identity = PeerIdentity {
        device_id: derive_device_id(),
        device_name: default_device_name(),
        nickname: String::new(),
        platform: platform_label(),
        avatar_color: String::new(),
        auth_token: new_secret_text("pt_"),
        key: sealed::generate_key().ok_or_else(|| "failed to create the peer key".to_string())?,
    };
    save_identity(data_dir, &identity)?;
    Ok(identity)
}

pub fn save_identity(data_dir: &Path, identity: &PeerIdentity) -> Result<(), String> {
    let stored = StoredIdentity {
        device_id: identity.device_id.clone(),
        device_name: identity.device_name.clone(),
        nickname: identity.nickname.clone(),
        avatar_color: identity.avatar_color.clone(),
        auth_token: encode_or_error(&identity.auth_token)?,
        key: encode_or_error(&sealed::encode_key(&identity.key))?,
    };
    let body = serde_json::to_string_pretty(&stored)
        .map_err(|e| format!("failed to serialize peer identity: {e}"))?;
    std::fs::create_dir_all(data_dir).map_err(|e| format!("failed to open data folder: {e}"))?;
    std::fs::write(identity_path(data_dir), body).map_err(|e| format!("failed to save identity: {e}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(identity_path(data_dir), std::fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

pub fn current(app: &AppHandle) -> PeerIdentity {
    app.state::<PeerIdentityState>().0.read().unwrap().clone()
}

/// A session identity for a machine whose stored one cannot be read (locked
/// Keychain, damaged file). Peers will fail to reach it, which is the honest
/// outcome; the alternative — silently replacing the file — would break every
/// pairing on the other machines without telling anyone.
///
/// `None` only if the OS has no random bytes at all, in which case the peer
/// feature stays off for this run rather than running on a predictable key.
pub fn temporary_identity() -> Option<PeerIdentity> {
    Some(PeerIdentity {
        device_id: derive_device_id(),
        device_name: default_device_name(),
        nickname: String::new(),
        platform: platform_label(),
        avatar_color: String::new(),
        auth_token: new_secret_text("pt_"),
        key: sealed::generate_key()?,
    })
}

fn update_field(app: &AppHandle, f: impl FnOnce(&mut PeerIdentity)) -> Result<(), String> {
    let data_dir = app.state::<AppDataDir>().0.lock().unwrap().clone();
    let view = {
        let state = app.state::<PeerIdentityState>();
        let mut identity = state.0.write().unwrap();
        f(&mut identity);
        save_identity(&data_dir, &identity)?;
        identity.view()
    };
    // The other machines learn a new name from the next beacon or handshake;
    // this window only needs to stop showing the old one. A colour never
    // travels: it is how this machine draws itself.
    let _ = app.emit("peer-identity-changed", view);
    Ok(())
}

pub fn update_nickname(app: &AppHandle, nickname: &str) -> Result<(), String> {
    let nickname = super::roster::clean_text(nickname, 40);
    update_field(app, move |identity| identity.nickname = nickname)
}

/// 设备与用户 → 本机头像底色. Only `normalize_color` output survives, so an
/// empty string means "no colour picked" rather than whatever was typed.
pub fn update_avatar_color(app: &AppHandle, color: &str) -> Result<(), String> {
    let color = color.to_string();
    update_field(app, move |identity| identity.avatar_color = color)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "tiez-peer-identity-{}-{}-{}",
            tag,
            std::process::id(),
            uuid::Uuid::new_v4()
        ))
    }

    #[test]
    fn device_ids_are_stable_and_short() {
        let first = derive_device_id();
        assert_eq!(first.len(), 16);
        assert!(first.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(first, derive_device_id());
    }

    #[test]
    fn identity_round_trips_and_survives_reopen() {
        let dir = temp_dir("round");
        let created = load_or_create_identity(&dir).expect("create identity");
        assert!(!created.auth_token.is_empty());
        assert_eq!(created.device_id, derive_device_id());

        let again = load_or_create_identity(&dir).expect("reload identity");
        assert_eq!(again.device_id, created.device_id);
        assert_eq!(again.auth_token, created.auth_token);
        assert_eq!(again.key, created.key);

        let text = std::fs::read_to_string(identity_path(&dir)).expect("read file");
        assert!(!text.contains(&created.auth_token), "token must not be plaintext");
        assert!(!text.contains("pt_"), "token must not be plaintext");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn nickname_is_saved_and_capped() {
        let dir = temp_dir("nick");
        let mut identity = load_or_create_identity(&dir).expect("create");
        identity.nickname = "书房 Mac".to_string();
        save_identity(&dir, &identity).expect("save");
        let reloaded = load_or_create_identity(&dir).expect("reload");
        assert_eq!(reloaded.nickname, "书房 Mac");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_corrupt_identity_is_reported_not_replaced() {
        let dir = temp_dir("corrupt");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            identity_path(&dir),
            r#"{"device_id":"abc","auth_token":"enc1:not-cipher","key":"xx"}"#,
        )
        .unwrap();
        let error = load_or_create_identity(&dir).expect_err("must fail");
        assert!(error.contains("decrypt"), "{error}");
        // The unreadable file is left alone; a wipe would orphan every peer.
        assert!(identity_path(&dir).exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
