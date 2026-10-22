//! Optional end-to-end encryption for MQTT messages and WebDAV sync files.
//! The format is specified in docs/E2E-ENCRYPTION.md and shared with the
//! mobile app (tiez-mobile/src/lib/e2e.ts); keep the two in step.

use crate::infrastructure::encryption::{aes_gcm_open, aes_gcm_seal, fill_random};
use base64::Engine;
use sha2::{Digest, Sha256};
use std::collections::VecDeque;
use std::sync::Mutex;

/// Setting that holds the shared key (encrypted at rest, never synced).
pub const E2E_KEY_SETTING: &str = "e2e_key";

const MAGIC: &[u8; 4] = b"TZE1";
const MQTT_PREFIX: &str = "tze1:";
const NONCE_LEN: usize = 12;
const TAG_LEN: usize = 16;
/// MQTT messages further than this from our clock are treated as replays.
const MQTT_MAX_SKEW_MS: i64 = 10 * 60 * 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Mqtt,
    Snapshot,
    Ops,
    Settings,
    Blob,
}

impl Kind {
    fn aad(self) -> &'static [u8] {
        match self {
            Kind::Mqtt => b"tiez-e2e-v1/mqtt",
            Kind::Snapshot => b"tiez-e2e-v1/snapshot",
            Kind::Ops => b"tiez-e2e-v1/ops",
            Kind::Settings => b"tiez-e2e-v1/settings",
            Kind::Blob => b"tiez-e2e-v1/blob",
        }
    }
}

/// Purpose keys derived from the shared key.
#[derive(Clone)]
pub struct E2eKeys {
    mqtt: [u8; 32],
    webdav: [u8; 32],
    blob_name: [u8; 32],
    fingerprint: String,
}

impl std::fmt::Debug for E2eKeys {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Never print key material.
        write!(f, "E2eKeys({})", self.fingerprint)
    }
}

impl E2eKeys {
    pub fn from_master(master: &[u8; 32]) -> Self {
        let digest = Sha256::digest(master);
        let hex = format!("{:02X}{:02X}{:02X}{:02X}", digest[0], digest[1], digest[2], digest[3]);
        Self {
            mqtt: hmac_sha256(master, b"tiez-e2e-v1 mqtt"),
            webdav: hmac_sha256(master, b"tiez-e2e-v1 webdav"),
            blob_name: hmac_sha256(master, b"tiez-e2e-v1 blob-name"),
            fingerprint: format!("{}-{}", &hex[..4], &hex[4..]),
        }
    }

    /// Parse a key as written by `generate_key` (base64url, no padding);
    /// standard base64 and padding are accepted too.
    pub fn parse(text: &str) -> Option<Self> {
        let cleaned: String = text.trim().trim_end_matches('=').chars().filter(|c| !c.is_whitespace()).collect();
        let normalized = cleaned.replace('+', "-").replace('/', "_");
        let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(normalized).ok()?;
        let master: [u8; 32] = bytes.try_into().ok()?;
        Some(Self::from_master(&master))
    }

    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }

    fn key_for(&self, kind: Kind) -> &[u8; 32] {
        if kind == Kind::Mqtt { &self.mqtt } else { &self.webdav }
    }

    pub fn seal(&self, kind: Kind, plaintext: &[u8]) -> Vec<u8> {
        let mut nonce = [0u8; NONCE_LEN];
        fill_random(&mut nonce).expect("system random source");
        self.seal_with_nonce(kind, plaintext, &nonce)
    }

    fn seal_with_nonce(&self, kind: Kind, plaintext: &[u8], nonce: &[u8; NONCE_LEN]) -> Vec<u8> {
        let sealed = aes_gcm_seal(self.key_for(kind), nonce, kind.aad(), plaintext)
            .expect("AES-GCM sealing with a 32-byte key and 12-byte nonce does not fail");
        let mut out = Vec::with_capacity(MAGIC.len() + NONCE_LEN + sealed.len());
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(nonce);
        out.extend_from_slice(&sealed);
        out
    }

    /// Open a box. None for anything that isn't a valid box for this key and
    /// kind (plaintext, another key, tampered, truncated).
    pub fn open(&self, kind: Kind, data: &[u8]) -> Option<Vec<u8>> {
        if !is_sealed(data) || data.len() < MAGIC.len() + NONCE_LEN + TAG_LEN {
            return None;
        }
        let (nonce, sealed) = data[MAGIC.len()..].split_at(NONCE_LEN);
        aes_gcm_open(self.key_for(kind), nonce, kind.aad(), sealed)
    }

    /// Blob file name for content: keyed, so names don't reveal content hashes.
    pub fn blob_name(&self, plaintext: &[u8]) -> String {
        hmac_sha256(&self.blob_name, plaintext)
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect()
    }

    pub fn seal_mqtt_text(&self, json: &str) -> String {
        let sealed = self.seal(Kind::Mqtt, json.as_bytes());
        format!("{}{}", MQTT_PREFIX, base64::engine::general_purpose::STANDARD.encode(sealed))
    }

    /// Decrypt an MQTT payload and check it is fresh and not seen before.
    /// Returns the inner JSON text.
    pub fn open_mqtt_text(&self, payload: &str, now_ms: i64) -> Result<String, MqttRejection> {
        let encoded = payload.trim().strip_prefix(MQTT_PREFIX).ok_or(MqttRejection::NotEncrypted)?;
        let sealed = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .map_err(|_| MqttRejection::Undecryptable)?;
        let plain = self.open(Kind::Mqtt, &sealed).ok_or(MqttRejection::Undecryptable)?;
        let text = String::from_utf8(plain).map_err(|_| MqttRejection::Undecryptable)?;
        let value: serde_json::Value = serde_json::from_str(&text).map_err(|_| MqttRejection::Undecryptable)?;
        let timestamp = value["timestamp"].as_i64().unwrap_or(0);
        if (now_ms - timestamp).abs() > MQTT_MAX_SKEW_MS {
            return Err(MqttRejection::Stale);
        }
        let id = value["id"].as_str().unwrap_or_default();
        if id.is_empty() || !remember_mqtt_id(id) {
            return Err(MqttRejection::Replayed);
        }
        Ok(text)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MqttRejection {
    /// Plaintext while a key is set.
    NotEncrypted,
    /// Sealed with another key, or damaged.
    Undecryptable,
    Stale,
    Replayed,
}

pub fn is_sealed(data: &[u8]) -> bool {
    data.starts_with(MAGIC)
}

pub fn is_sealed_mqtt_text(payload: &str) -> bool {
    payload.trim_start().starts_with(MQTT_PREFIX)
}

/// The configured keys, or None when end-to-end encryption is off.
pub fn load_keys(app: &tauri::AppHandle) -> Option<E2eKeys> {
    use crate::infrastructure::repository::settings_repo::SettingsRepository;
    use tauri::Manager;
    let db = app.try_state::<crate::database::DbState>()?;
    let text = db.settings_repo.get(E2E_KEY_SETTING).ok().flatten()?;
    if text.trim().is_empty() {
        return None;
    }
    E2eKeys::parse(&text)
}

/// Fingerprint of a key typed or pasted in settings; None if it isn't a key.
#[tauri::command]
pub fn e2e_key_fingerprint(key: String) -> Option<String> {
    E2eKeys::parse(&key).map(|keys| keys.fingerprint().to_string())
}

#[tauri::command]
pub fn generate_e2e_key() -> String {
    generate_key()
}

/// A new random key, base64url without padding.
pub fn generate_key() -> String {
    let mut master = [0u8; 32];
    fill_random(&mut master).expect("system random source");
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(master)
}

/// Ids of recently accepted MQTT messages (replay protection).
fn remember_mqtt_id(id: &str) -> bool {
    static SEEN: Mutex<VecDeque<String>> = Mutex::new(VecDeque::new());
    let mut seen = SEEN.lock().unwrap_or_else(|p| p.into_inner());
    if seen.iter().any(|known| known == id) {
        return false;
    }
    if seen.len() >= 512 {
        seen.pop_front();
    }
    seen.push_back(id.to_string());
    true
}

fn hmac_sha256(key: &[u8], message: &[u8]) -> [u8; 32] {
    const BLOCK: usize = 64;
    let mut block_key = [0u8; BLOCK];
    if key.len() > BLOCK {
        block_key[..32].copy_from_slice(&Sha256::digest(key));
    } else {
        block_key[..key.len()].copy_from_slice(key);
    }
    let mut inner = Sha256::new();
    inner.update(block_key.map(|b| b ^ 0x36));
    inner.update(message);
    let inner = inner.finalize();
    let mut outer = Sha256::new();
    outer.update(block_key.map(|b| b ^ 0x5c));
    outer.update(inner);
    outer.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Shared with tiez-mobile/scripts/e2e-vectors.test.ts: both must agree.
    const VECTOR_KEY: &str = "AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8";

    #[test]
    fn hmac_matches_rfc4231_case_2() {
        let mac = hmac_sha256(b"Jefe", b"what do ya want for nothing?");
        let hex: String = mac.iter().map(|b| format!("{:02x}", b)).collect();
        assert_eq!(hex, "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843");
    }

    #[test]
    fn known_vectors_for_cross_platform_checks() {
        let keys = E2eKeys::parse(VECTOR_KEY).expect("vector key");
        assert_eq!(keys.fingerprint(), "630D-CD29");
        let sealed = keys.seal_with_nonce(Kind::Snapshot, b"{\"hello\":1}", &[7u8; NONCE_LEN]);
        let hex: String = sealed.iter().map(|b| format!("{:02x}", b)).collect();
        assert_eq!(hex, include_str!("e2e_vector_snapshot.hex").trim());
        assert_eq!(keys.blob_name(b"hello"), include_str!("e2e_vector_blob_name.hex").trim());
    }

    #[test]
    fn boxes_round_trip_and_reject_other_kinds_keys_and_tampering() {
        let keys = E2eKeys::parse(&generate_key()).unwrap();
        let sealed = keys.seal(Kind::Ops, b"payload");
        assert_eq!(keys.open(Kind::Ops, &sealed).as_deref(), Some(&b"payload"[..]));
        assert!(keys.open(Kind::Snapshot, &sealed).is_none());
        let other = E2eKeys::parse(&generate_key()).unwrap();
        assert!(other.open(Kind::Ops, &sealed).is_none());
        let mut tampered = sealed.clone();
        *tampered.last_mut().unwrap() ^= 1;
        assert!(keys.open(Kind::Ops, &tampered).is_none());
        assert!(keys.open(Kind::Ops, b"{\"plain\":true}").is_none());
    }

    #[test]
    fn mqtt_messages_are_fresh_once() {
        let keys = E2eKeys::parse(&generate_key()).unwrap();
        let now = 1_760_000_000_000;
        let text = keys.seal_mqtt_text(&format!(r#"{{"msg":"hi","timestamp":{now},"id":"a1"}}"#));
        assert!(keys.open_mqtt_text(&text, now + 1_000).unwrap().contains("\"hi\""));
        assert_eq!(keys.open_mqtt_text(&text, now + 1_000), Err(MqttRejection::Replayed));
        let old = keys.seal_mqtt_text(&format!(r#"{{"msg":"hi","timestamp":{now},"id":"a2"}}"#));
        assert_eq!(keys.open_mqtt_text(&old, now + MQTT_MAX_SKEW_MS + 1), Err(MqttRejection::Stale));
        assert_eq!(keys.open_mqtt_text(r#"{"msg":"plain"}"#, now), Err(MqttRejection::NotEncrypted));
    }

    #[test]
    fn keys_parse_in_either_base64_alphabet() {
        let key = generate_key();
        assert_eq!(key.len(), 43);
        let standard = format!("{}=", key.replace('-', "+").replace('_', "/"));
        assert_eq!(E2eKeys::parse(&standard).unwrap().fingerprint(), E2eKeys::parse(&key).unwrap().fingerprint());
        assert!(E2eKeys::parse("too-short").is_none());
    }
}
