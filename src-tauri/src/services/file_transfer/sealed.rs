//! Encryption for LAN transfer (docs/LAN-TRANSFER-ENCRYPTION.md).
//!
//! The pairing QR code carries a fresh 32-byte key in its URL fragment, which
//! browsers never send over the network. Every request and response body on
//! the `/s/*` routes and every frame of a `?s=1` WebSocket is a sealed box:
//!
//! ```text
//! box = "TZL1" || nonce (12) || AES-256-GCM ciphertext || tag (16)
//! aad = "tiez-lan-v1/<kind>"
//! ```
//!
//! Downloads are a stream of frames, `u32 BE length || box`, one per
//! `DOWNLOAD_CHUNK` bytes of the file; each frame's AAD names its index and
//! whether it is the last, so frames cannot be dropped, reordered or cut off.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};

use crate::infrastructure::encryption::{aes_gcm_open, aes_gcm_seal, fill_random};

pub const KEY_LEN: usize = 32;
const MAGIC: &[u8; 4] = b"TZL1";
const NONCE_LEN: usize = 12;
const TAG_LEN: usize = 16;
pub const OVERHEAD: usize = MAGIC.len() + NONCE_LEN + TAG_LEN;
/// Plaintext bytes per download frame.
pub const DOWNLOAD_CHUNK: usize = 256 * 1024;

pub type LanKey = [u8; KEY_LEN];

pub fn generate_key() -> Option<LanKey> {
    let mut key = [0u8; KEY_LEN];
    fill_random(&mut key)?;
    Some(key)
}

pub fn encode_key(key: &LanKey) -> String {
    URL_SAFE_NO_PAD.encode(key)
}

/// The pairing code and the peer roster carry keys as this text.
pub fn decode_key(text: &str) -> Option<LanKey> {
    let bytes = URL_SAFE_NO_PAD.decode(text).ok()?;
    bytes.try_into().ok()
}

fn aad(kind: &str) -> Vec<u8> {
    format!("tiez-lan-v1/{kind}").into_bytes()
}

pub fn download_kind(index: u64, last: bool) -> String {
    format!("download/{index}/{}", u8::from(last))
}

pub fn seal(key: &LanKey, kind: &str, plain: &[u8]) -> Option<Vec<u8>> {
    let mut nonce = [0u8; NONCE_LEN];
    fill_random(&mut nonce)?;
    let sealed = aes_gcm_seal(key, &nonce, &aad(kind), plain)?;
    let mut out = Vec::with_capacity(OVERHEAD + plain.len());
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&sealed);
    Some(out)
}

pub fn open(key: &LanKey, kind: &str, sealed: &[u8]) -> Option<Vec<u8>> {
    if sealed.len() < OVERHEAD || &sealed[..MAGIC.len()] != MAGIC {
        return None;
    }
    let (nonce, body) = sealed[MAGIC.len()..].split_at(NONCE_LEN);
    aes_gcm_open(key, nonce, &aad(kind), body)
}

/// One download frame: `u32 BE length || box`.
pub fn download_frame(key: &LanKey, index: u64, last: bool, plain: &[u8]) -> Option<Vec<u8>> {
    let boxed = seal(key, &download_kind(index, last), plain)?;
    let mut frame = Vec::with_capacity(4 + boxed.len());
    frame.extend_from_slice(&(boxed.len() as u32).to_be_bytes());
    frame.extend_from_slice(&boxed);
    Some(frame)
}

/// Upload payload: `u32 BE metadata length || metadata JSON || file bytes`.
pub fn split_upload(plain: &[u8]) -> Option<(&[u8], &[u8])> {
    let len = u32::from_be_bytes(plain.get(..4)?.try_into().ok()?) as usize;
    let rest = &plain[4..];
    (len <= rest.len()).then(|| rest.split_at(len))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key() -> LanKey {
        [7u8; KEY_LEN]
    }

    #[test]
    fn round_trips_and_binds_kind() {
        let boxed = seal(&key(), "text", b"hello").unwrap();
        assert_eq!(&boxed[..4], b"TZL1");
        assert_eq!(boxed.len(), OVERHEAD + 5);
        assert_eq!(open(&key(), "text", &boxed).unwrap(), b"hello");
        assert!(
            open(&key(), "poll", &boxed).is_none(),
            "a box must not open as another kind"
        );
        assert!(open(&[8u8; KEY_LEN], "text", &boxed).is_none(), "wrong key");
    }

    #[test]
    fn rejects_tampering_and_garbage() {
        let mut boxed = seal(&key(), "ws", b"payload").unwrap();
        let last = boxed.len() - 1;
        boxed[last] ^= 1;
        assert!(open(&key(), "ws", &boxed).is_none());
        assert!(open(&key(), "ws", b"TZL1short").is_none());
        assert!(open(&key(), "ws", b"{\"type\":\"ping\"}").is_none());
    }

    #[test]
    fn download_frames_cannot_be_moved() {
        let frame = seal(&key(), &download_kind(3, false), b"chunk").unwrap();
        assert!(open(&key(), &download_kind(3, false), &frame).is_some());
        assert!(open(&key(), &download_kind(4, false), &frame).is_none());
        assert!(open(&key(), &download_kind(3, true), &frame).is_none());
    }

    #[test]
    fn frames_carry_their_length() {
        let frame = download_frame(&key(), 0, true, b"abc").unwrap();
        let len = u32::from_be_bytes(frame[..4].try_into().unwrap()) as usize;
        assert_eq!(len, frame.len() - 4);
        assert_eq!(
            open(&key(), &download_kind(0, true), &frame[4..]).unwrap(),
            b"abc"
        );
    }

    #[test]
    fn splits_upload_payload() {
        let mut plain = 3u32.to_be_bytes().to_vec();
        plain.extend_from_slice(b"{}x");
        plain.extend_from_slice(b"DATA");
        let (meta, data) = split_upload(&plain).unwrap();
        assert_eq!(meta, b"{}x");
        assert_eq!(data, b"DATA");
        assert!(split_upload(&[0, 0, 0, 9, 1]).is_none());
        assert!(split_upload(&[0, 0]).is_none());
    }

    /// Sealed by the TieZ app (tiez-mobile/src/lib/lanSealCore.ts) with key
    /// 0..=31 and nonce a0..=ab; scripts/lan-vectors.test.mts there holds the
    /// vectors going the other way.
    const APP_UPLOAD: &str = "545a4c31a0a1a2a3a4a5a6a7a8a9aaabe6187cae3ee977cf0e0ae6b75813a4fc4a8e2f75f1866040be6d4ef311c02a68bc1222878d1863117de86bbc6816dc9a2f6e282311f2204f6d7c6d37c815daded5d1e04d0a8702697f0463a2a1bab4ceef2789d7a0affd1f9c2a57fe93a06ff9ada809d866c0ebfeec846831e8edcf12060009c1b050b9100dbbb1d0d539ad3eac6bd55de977fd7f523592e4ee9672a107ad481a4dbe75091406d7d5";

    #[test]
    fn opens_what_the_app_seals() {
        let key: LanKey = std::array::from_fn(|i| i as u8);
        let boxed: Vec<u8> = (0..APP_UPLOAD.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&APP_UPLOAD[i..i + 2], 16).unwrap())
            .collect();
        let plain = open(&key, "upload", &boxed).expect("opens");
        let (meta, data) = split_upload(&plain).unwrap();
        let meta: serde_json::Value = serde_json::from_slice(meta).unwrap();
        assert_eq!(meta["file_name"], "手机.txt");
        assert_eq!(data, b"hello");
    }

    #[test]
    fn keys_are_url_safe() {
        let key = generate_key().unwrap();
        let text = encode_key(&key);
        assert_eq!(text.len(), 43);
        assert!(text
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'));
        assert_eq!(decode_key(&text).as_ref(), Some(&key));
        assert!(decode_key("short").is_none());
        assert!(decode_key(&text.repeat(2)).is_none());
        assert!(decode_key("has+plus").is_none());
    }
}
