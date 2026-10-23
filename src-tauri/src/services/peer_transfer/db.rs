//! The roster in memory, kept in step with `lan_peers`.
//!
//! Every send, every inbound trust check and the discovery beacon all read the
//! same map, so it is loaded once at start and written through: a peer that
//! changes its nickname or moves to a new IP is updated in both places.

use tauri::{AppHandle, Manager};

use crate::database::DbState;
use crate::infrastructure::repository::peer_repo::{SqlitePeerRepository, StoredPeer};

use super::identity;
use super::models::{PeerCredentials, PeerHandle, PeerRosterState};

pub fn repository(app: &AppHandle) -> SqlitePeerRepository {
    SqlitePeerRepository::new(app.state::<DbState>().conn.clone())
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_millis() as i64)
        .unwrap_or(0)
}

fn to_stored(peer: &PeerHandle) -> StoredPeer {
    StoredPeer {
        device_id: peer.device_id.clone(),
        device_name: peer.device_name.clone(),
        nickname: peer.nickname.clone(),
        platform: peer.platform.clone(),
        last_ip: peer.ip.clone(),
        trusted: peer.trusted,
        pair_source: peer.pair_source.clone(),
        avatar_color: peer.avatar_color.clone(),
        credentials: PeerCredentials {
            port: peer.port,
            auth: peer.auth_token.clone(),
            key: crate::services::file_transfer::sealed::encode_key(&peer.key),
        }
        .to_json(),
        created_at: peer.created_at,
        last_seen: peer.last_seen,
        favorite: peer.favorite,
        name_locked: peer.name_locked,
    }
}

fn to_handle(peer: StoredPeer) -> Option<PeerHandle> {
    let (port, auth_token, key) = PeerCredentials::from_json(&peer.credentials)?;
    Some(PeerHandle {
        device_id: peer.device_id,
        device_name: peer.device_name,
        nickname: peer.nickname,
        platform: peer.platform,
        ip: peer.last_ip,
        port,
        auth_token,
        key,
        trusted: peer.trusted,
        pair_source: peer.pair_source,
        avatar_color: peer.avatar_color,
        created_at: peer.created_at,
        last_seen: peer.last_seen,
        favorite: peer.favorite,
        name_locked: peer.name_locked,
    })
}

/// Load the roster; a peer whose credentials cannot be decrypted is skipped
/// (and reported) rather than dropping the whole list.
pub fn load_roster(app: &AppHandle) -> Result<usize, String> {
    let stored = repository(app)
        .list()
        .map_err(|e| format!("failed to read paired devices: {e}"))?;
    let mut loaded = 0;
    let mut dropped = Vec::new();
    {
        let state = app.state::<PeerRosterState>();
        let mut roster = state.0.write().unwrap();
        for peer in stored {
            let device_id = peer.device_id.clone();
            match to_handle(peer) {
                Some(handle) => {
                    roster.insert(handle.device_id.clone(), handle);
                    loaded += 1;
                }
                None => dropped.push(device_id),
            }
        }
    }
    if !dropped.is_empty() {
        eprintln!(
            ">>> [PEER] {} paired device(s) have unreadable credentials and stay unusable until re-paired: {}",
            dropped.len(),
            dropped.join(", ")
        );
    }
    Ok(loaded)
}

/// Insert or update a paired device (and its row).
///
/// The handshake says who the other machine is now; the fields the user set
/// locally — a label, colour and ⭐ — are carried over from the roster instead
/// of being reset by whatever the peer announced. Only a label the user
/// actually typed (`name_locked`) wins over the peer's own name: otherwise a
/// machine that renamed itself would keep its old name on every other screen
/// until someone re-paired.
pub fn save_peer(app: &AppHandle, mut peer: PeerHandle) -> Result<(), String> {
    if let Some(existing) = app
        .state::<PeerRosterState>()
        .0
        .read()
        .ok()
        .and_then(|roster| roster.get(&peer.device_id).cloned())
    {
        if existing.name_locked {
            peer.nickname = existing.nickname;
            peer.name_locked = true;
        } else {
            // The announced name is already in `device_name`; the old nickname
            // field must not outrank it in `display_name`.
            peer.nickname = String::new();
            peer.name_locked = false;
        }
        if !existing.avatar_color.is_empty() {
            peer.avatar_color = existing.avatar_color;
        }
        peer.created_at = existing.created_at;
        peer.favorite = existing.favorite;
    }
    repository(app)
        .upsert(&to_stored(&peer))
        .map_err(|e| format!("failed to save paired device: {e}"))?;
    let state = app.state::<PeerRosterState>();
    state.0.write().unwrap().insert(peer.device_id.clone(), peer);
    Ok(())
}

pub fn forget_peer(app: &AppHandle, device_id: &str) -> Result<(), String> {
    repository(app)
        .remove(device_id)
        .map_err(|e| format!("failed to remove paired device: {e}"))?;
    app.state::<PeerRosterState>()
        .0
        .write()
        .unwrap()
        .remove(device_id);
    Ok(())
}

pub fn update_profile(app: &AppHandle, device_id: &str, nickname: &str, avatar_color: &str) -> Result<(), String> {
    let nickname = super::roster::clean_text(nickname, 40);
    let locked = !nickname.is_empty();
    repository(app)
        .update_profile(device_id, &nickname, avatar_color)
        .map_err(|e| format!("failed to update device profile: {e}"))?;
    let state = app.state::<PeerRosterState>();
    if let Some(peer) = state.0.write().unwrap().get_mut(device_id) {
        peer.nickname = nickname;
        peer.avatar_color = avatar_color.to_string();
        peer.name_locked = locked;
    }
    Ok(())
}

/// The ⭐ in the settings list. Local-only: nothing about it travels to the
/// other machine, which is the point — it is how *this* user ranks the roster.
pub fn set_favorite(app: &AppHandle, device_id: &str, favorite: bool) -> Result<(), String> {
    repository(app)
        .set_favorite(device_id, favorite)
        .map_err(|e| format!("failed to update the star: {e}"))?;
    let state = app.state::<PeerRosterState>();
    if let Some(peer) = state.0.write().unwrap().get_mut(device_id) {
        peer.favorite = favorite;
    }
    Ok(())
}

/// A beacon told us a peer lives at a new address — and, unless the user here
/// gave that peer a label, what it currently calls itself. Returns whether any
/// of the listed fields changed, so callers can skip the UI event; `last_seen`
/// alone moves every few seconds and is not a change worth a repaint.
pub fn apply_seen(app: &AppHandle, device_id: &str, name: &str, ip: &str, port: u16) -> bool {
    let announced = super::roster::clean_text(name, 80);
    let (changed, seen_at, stored_name) = {
        let state = app.state::<PeerRosterState>();
        let mut roster = state.0.write().unwrap();
        let Some(peer) = roster.get_mut(device_id) else {
            return false;
        };
        let mut changed = peer.ip != ip || (port != 0 && peer.port != port);
        peer.ip = ip.to_string();
        if port != 0 {
            peer.port = port;
        }
        let mut stored_name = None;
        if !peer.name_locked && !announced.is_empty() && peer.device_name != announced {
            peer.device_name = announced.clone();
            peer.nickname.clear();
            changed = true;
            stored_name = Some(announced);
        }
        peer.last_seen = now_ms();
        (changed, peer.last_seen, stored_name)
    };
    // The connection lock is never taken while the roster is held.
    let _ = repository(app).update_presence(device_id, ip, seen_at, stored_name.as_deref());
    changed
}

/// The address the peer last beaconed from, falling back to the pairing code.
pub fn send_target(app: &AppHandle, device_id: &str) -> Option<PeerHandle> {
    let state = app.state::<PeerRosterState>();
    let peer = state.0.read().unwrap().get(device_id).cloned()?;
    if peer.ip.is_empty() {
        return None;
    }
    Some(peer)
}

pub fn list_peers(app: &AppHandle) -> Vec<PeerHandle> {
    let state = app.state::<PeerRosterState>();
    let mut peers: Vec<PeerHandle> = state.0.read().unwrap().values().cloned().collect();
    peers.sort_by(|a, b| {
        a.created_at
            .cmp(&b.created_at)
            .then_with(|| a.device_id.cmp(&b.device_id))
    });
    peers
}

pub fn is_trusted_sender(app: &AppHandle, device_id: &str) -> bool {
    let state = app.state::<PeerRosterState>();
    let roster = match state.0.read() {
        Ok(roster) => roster,
        Err(_) => return false,
    };
    roster.get(device_id).is_some_and(|peer| peer.trusted)
}

/// A peer record built from a handshake body.
pub fn peer_from_handshake(
    device_id: &str,
    device_name: &str,
    nickname: &str,
    platform: &str,
    ip: &str,
    port: u16,
    auth: &str,
    key_text: &str,
) -> Option<PeerHandle> {
    let key = crate::services::file_transfer::sealed::decode_key(key_text)?;
    // What the other machine calls itself right now: its own nickname if it
    // chose one, its machine name otherwise. Stored as `device_name` so the
    // next beacon can keep it true; `nickname` stays for a local label.
    let announced = if nickname.trim().is_empty() {
        device_name.to_string()
    } else {
        nickname.to_string()
    };
    Some(PeerHandle {
        device_id: device_id.to_string(),
        device_name: announced,
        nickname: String::new(),
        platform: platform.to_string(),
        ip: ip.to_string(),
        port,
        auth_token: auth.to_string(),
        key,
        trusted: true,
        pair_source: "pair".to_string(),
        avatar_color: String::new(),
        created_at: now_ms(),
        last_seen: now_ms(),
        favorite: false,
        name_locked: false,
    })
}

/// The identity this machine offers to peers, for building pairing payloads.
pub fn self_credentials(app: &AppHandle) -> (String, String, String, u16) {
    let identity = identity::current(app);
    let port = app
        .state::<crate::services::file_transfer::models::ServerInfo>()
        .port
        .load(std::sync::atomic::Ordering::Relaxed);
    (identity.device_id, identity.device_name, identity.nickname, port)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stored_and_handle_forms_round_trip() {
        let peer = PeerHandle {
            device_id: "a1b2c3d4e5f60718".to_string(),
            device_name: "Conley 的 Mac".to_string(),
            nickname: "书房".to_string(),
            platform: "macos".to_string(),
            ip: "192.168.1.20".to_string(),
            port: 28462,
            auth_token: "pt_deadbeef".to_string(),
            key: [3u8; 32],
            trusted: true,
            pair_source: "pair".to_string(),
            avatar_color: "#0A84FF".to_string(),
            created_at: 5,
            last_seen: 9,
            favorite: true,
            name_locked: true,
        };
        let stored = to_stored(&peer);
        let back = to_handle(stored).expect("handle");
        assert_eq!(back.device_id, peer.device_id);
        assert_eq!(back.port, peer.port);
        assert_eq!(back.auth_token, peer.auth_token);
        assert_eq!(back.key, peer.key);
        assert_eq!(back.nickname, "书房");
        assert!(back.favorite, "the star rides along with the row");
        assert!(back.name_locked, "a local label survives a round trip");
    }

    #[test]
    fn a_handshake_name_is_what_the_peer_calls_itself() {
        let peer = peer_from_handshake(
            "0123456789abcdef",
            "新名字 的 Windows 电脑",
            "新名字",
            "windows",
            "192.168.1.9",
            28462,
            "pt_x",
            &crate::services::file_transfer::sealed::encode_key(&[7u8; 32]),
        )
        .expect("peer");
        assert_eq!(peer.display_name(), "新名字");
        assert!(
            !peer.name_locked,
            "a fresh pairing follows the announced name until a local label is typed"
        );
        assert!(peer.nickname.is_empty(), "no local label yet");
    }

    #[test]
    fn credentials_without_a_key_are_refused() {
        let stored = StoredPeer {
            device_id: "x".to_string(),
            device_name: "x".to_string(),
            nickname: String::new(),
            platform: String::new(),
            last_ip: String::new(),
            trusted: true,
            pair_source: String::new(),
            avatar_color: String::new(),
            credentials: r#"{"port":1,"auth":"a","key":"nope"}"#.to_string(),
            created_at: 0,
            last_seen: 0,
            favorite: false,
            name_locked: false,
        };
        assert!(to_handle(stored).is_none());
    }
}
