//! Pairing: a code out, a handshake back, both sides in the roster.
//!
//! Neither machine is a server for the other in any special sense — a
//! handshake is a sealed POST to the other's `/p/handshake`, answered with the
//! same key it arrived with. The machine that shows the code answers
//! "pending"; its user then accepts, and the accept carries this machine's own
//! credentials back, so the pairing ends mutual with one confirmation each way.
//! A code that was pasted twice only costs one more confirmation.

use std::collections::HashMap;
use std::time::Duration;

use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use tauri::{AppHandle, Emitter, Manager};

use crate::services::file_transfer::sealed::{self, LanKey};
use crate::services::file_transfer::ServerInfo;

use super::db;
use super::identity;
use super::models::{
    PendingPairing, PairRequestView, PeerHandshake, PeerIdentity, PairingState, PAIR_REQUEST_TTL,
};
use super::roster;

/// Both sides need a live server: a pairing carries ports, and the answer has
/// somewhere to come back to.
async fn ensure_server(app: &AppHandle) -> Result<(), String> {
    let port = server_port(app);
    if port != 0 {
        return Ok(());
    }
    crate::services::file_transfer::toggle_file_server(app.clone(), true, None)
        .await
        .map_err(|error| format!("传输服务启动失败：{error}"))?;
    // The command returns before the listener is stored; give it a moment.
    for _ in 0..30 {
        if server_port(app) != 0 {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    Err("传输服务没能启动".to_string())
}

pub fn server_port(app: &AppHandle) -> u16 {
    app.state::<ServerInfo>()
        .port
        .load(std::sync::atomic::Ordering::Relaxed)
}

fn self_handshake(app: &AppHandle, status: &str, group: &str) -> PeerHandshake {
    let identity = identity::current(app);
    PeerHandshake {
        status: status.to_string(),
        group: group.to_string(),
        device_id: identity.device_id.clone(),
        device_name: identity.device_name.clone(),
        nickname: identity.nickname.clone(),
        platform: identity.platform.clone(),
        port: server_port(app),
        auth: identity.auth_token.clone(),
        key: sealed::encode_key(&identity.key),
    }
}

/// POST a handshake and read the sealed answer.
async fn exchange(
    ip: &str,
    port: u16,
    auth: &str,
    key: &LanKey,
    message: &PeerHandshake,
) -> Result<PeerHandshake, String> {
    let body = serde_json::to_vec(message)
        .map_err(|error| format!("配对请求无法编码：{error}"))?;
    let boxed = sealed::seal(key, "handshake", &body).ok_or("配对请求加密失败")?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|error| format!("配对请求无法发出：{error}"))?;
    let response = client
        .post(format!("http://{ip}:{port}/p/handshake"))
        .header(AUTHORIZATION, format!("Bearer {auth}"))
        .header(CONTENT_TYPE, "application/octet-stream")
        .body(boxed)
        .send()
        .await
        .map_err(|error| format!("连不上 {ip}:{port}（{error}）"))?;
    let status = response.status();
    if status == reqwest::StatusCode::UNAUTHORIZED {
        return Err("这个配对码的密钥已经失效，请让对方重新生成".to_string());
    }
    if !status.is_success() {
        return Err(format!("对方拒绝了配对（{status}）"));
    }
    let bytes = response
        .bytes()
        .await
        .map_err(|error| format!("读不到对方的回复：{error}"))?;
    let plain = sealed::open(key, "handshake", &bytes).ok_or("对方的回复无法解读")?;
    serde_json::from_slice(&plain).map_err(|error| format!("对方的回复无法识别：{error}"))
}

/// The pairing map, or `None` if its lock broke.
///
/// The guard is always bound to a local here rather than used in a tail
/// expression: a temporary guard dropped after the `State` it borrows is an
/// E0597, and `State` borrows `AppHandle`, which outlives neither.
fn with_pairing<R>(app: &AppHandle, f: impl FnOnce(&mut HashMap<String, PendingPairing>) -> R) -> Option<R> {
    let state = app.state::<PairingState>();
    let mut guard = state.0.lock().ok()?;
    Some(f(&mut guard))
}

fn prune_pending(app: &AppHandle) {
    let _ = with_pairing(app, |pending| {
        pending.retain(|_, pending| !pending.is_expired())
    });
}

/// Our view of a machine we asked to pair with, or that asked us.
pub fn pending_for(app: &AppHandle, device_id: &str, direction: &str) -> Option<PendingPairing> {
    with_pairing(app, |pending| {
        pending
            .values()
            .find(|entry| entry.peer_device_id == device_id && entry.direction == direction)
            .cloned()
    })
    .flatten()
}

pub fn insert_pending(app: &AppHandle, pending: PendingPairing) {
    let _ = with_pairing(app, |pending_map| {
        pending_map.retain(|_, existing| !existing.is_expired());
        pending_map.insert(pending.group.clone(), pending);
    });
}

pub fn remove_pending(app: &AppHandle, group: &str) -> Option<PendingPairing> {
    with_pairing(app, |pending| pending.remove(group)).flatten()
}

/// A request the other machine sent us.
pub fn record_inbound(
    app: &AppHandle,
    message: &PeerHandshake,
    their_key: &LanKey,
    ip: &str,
    verify_word: String,
) -> PendingPairing {
    let pending = PendingPairing {
        peer_device_id: message.device_id.clone(),
        peer_name: message.device_name.clone(),
        peer_nickname: message.nickname.clone(),
        peer_platform: message.platform.clone(),
        peer_ip: ip.to_string(),
        peer_port: message.port,
        peer_auth: message.auth.clone(),
        peer_key: sealed::encode_key(their_key),
        group: message.group.clone(),
        direction: "in",
        verify_word,
        expires_at: std::time::Instant::now() + PAIR_REQUEST_TTL,
    };
    let state = app.state::<PairingState>();
    // Their first request may have timed out waiting for our answer and been
    // sent again under a new group; only the newest one is worth showing.
    let mut guard = match state.0.lock() {
        Ok(guard) => guard,
        Err(_) => return pending,
    };
    guard.retain(|_, existing| {
        !existing.is_expired()
            && !(existing.direction == "in" && existing.peer_device_id == pending.peer_device_id)
    });
    guard.insert(pending.group.clone(), pending.clone());
    pending
}

#[tauri::command]
pub async fn get_peer_pairing_code(app_handle: AppHandle) -> Result<String, String> {
    ensure_server(&app_handle).await?;
    let ip = crate::services::file_transfer::get_local_ip_addr(app_handle.clone());
    let port = server_port(&app_handle);
    let identity = identity::current(&app_handle);
    Ok(roster::build_pairing_code(&identity, &ip, port))
}

/// What pasting a code produced: either the pairing is already mutual, or the
/// other machine is waiting for its user to confirm.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PairResult {
    /// "paired" | "pending".
    pub status: String,
    pub device_id: String,
    pub device_name: String,
    pub verify_word: String,
    /// Seconds left on the pending request; 0 once paired.
    pub expires_in: i64,
}

/// Paste the other machine's code: this asks it to pair.
#[tauri::command]
pub async fn pair_with_code(app_handle: AppHandle, code: String) -> Result<PairResult, String> {
    let target = roster::parse_pairing_code(&code)?;
    let ours = identity::current(&app_handle);
    if target.device_id == ours.device_id {
        return Err("这是本机自己的配对码".to_string());
    }
    ensure_server(&app_handle).await?;

    let group = uuid::Uuid::new_v4().as_simple().to_string();
    let message = self_handshake(&app_handle, "request", &group);
    let verify_word = roster::verify_word(
        (&ours.device_id, &ours.key),
        (&target.device_id, &target.key),
    );
    let reply = exchange(
        &target.ip,
        target.port,
        &target.auth_token,
        &target.key,
        &message,
    )
    .await?;

    match reply.status.as_str() {
        // They already have us paired: take their credentials and stop there.
        "paired" | "accept" => {
            let key = sealed::decode_key(&reply.key).ok_or("对方的密钥无法识别")?;
            let peer = db::peer_from_handshake(
                &reply.device_id,
                &reply.device_name,
                &reply.nickname,
                &reply.platform,
                &target.ip,
                reply.port,
                &reply.auth,
                &sealed::encode_key(&key),
            )
            .ok_or("对方的配对信息不完整")?;
            // A re-pair keeps the name this machine gave the peer; `save_peer`
            // is what carries the local fields over.
            db::save_peer(&app_handle, peer)?;
            remove_pending(&app_handle, &group);
            roster::notify_peers_changed(&app_handle);
            let _ = app_handle.emit(
                "peer-pair-done",
                serde_json::json!({ "deviceId": reply.device_id, "nickname": reply.nickname }),
            );
            Ok(PairResult {
                status: "paired".to_string(),
                device_id: reply.device_id,
                device_name: reply.device_name,
                verify_word,
                expires_in: 0,
            })
        }
        "pending" => {
            let pending = PendingPairing {
                peer_device_id: target.device_id.clone(),
                peer_name: if target.nickname.is_empty() {
                    target.device_name.clone()
                } else {
                    target.nickname.clone()
                },
                peer_nickname: target.nickname.clone(),
                peer_platform: target.platform.clone(),
                peer_ip: target.ip.clone(),
                peer_port: target.port,
                peer_auth: target.auth_token.clone(),
                peer_key: sealed::encode_key(&target.key),
                group,
                direction: "out",
                verify_word: verify_word.clone(),
                expires_at: std::time::Instant::now() + PAIR_REQUEST_TTL,
            };
            let view = pending.view();
            insert_pending(&app_handle, pending);
            Ok(PairResult {
                status: "pending".to_string(),
                device_id: view.device_id,
                device_name: view.device_name,
                verify_word,
                expires_in: view.expires_in,
            })
        }
        "reject" => Err("对方拒绝了这个配对请求".to_string()),
        other => Err(format!("对方返回了无法识别的状态（{other}）")),
    }
}

/// Everything the window needs to show: both directions.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PairingViews {
    pub inbound: Vec<PairRequestView>,
    pub outbound: Vec<PairRequestView>,
}

#[tauri::command]
pub fn list_pair_requests(app_handle: AppHandle) -> PairingViews {
    prune_pending(&app_handle);
    let state = app_handle.state::<PairingState>();
    let guard = match state.0.lock() {
        Ok(guard) => guard,
        Err(_) => return PairingViews { inbound: vec![], outbound: vec![] },
    };
    let mut inbound: Vec<PairRequestView> = guard
        .values()
        .filter(|pending| pending.direction == "in")
        .map(PendingPairing::view)
        .collect();
    let mut outbound: Vec<PairRequestView> = guard
        .values()
        .filter(|pending| pending.direction == "out")
        .map(PendingPairing::view)
        .collect();
    inbound.sort_by(|a, b| a.device_id.cmp(&b.device_id));
    outbound.sort_by(|a, b| a.device_id.cmp(&b.device_id));
    PairingViews { inbound, outbound }
}

/// The user's answer to an inbound request. Accepting stores the peer and
/// sends our own credentials to it, which completes the pairing on that side.
#[tauri::command]
pub async fn respond_pair_request(
    app_handle: AppHandle,
    device_id: String,
    accept: bool,
) -> Result<(), String> {
    let Some(pending) = pending_for(&app_handle, &device_id, "in") else {
        return Err("这个配对请求已经不在了".to_string());
    };
    if accept {
        let key = sealed::decode_key(&pending.peer_key).ok_or("对方的密钥无法识别")?;
        let peer = db::peer_from_handshake(
            &pending.peer_device_id,
            &pending.peer_name,
            &pending.peer_nickname,
            &pending.peer_platform,
            &pending.peer_ip,
            pending.peer_port,
            &pending.peer_auth,
            &sealed::encode_key(&key),
        )
        .ok_or("对方的配对信息不完整")?;
        db::save_peer(&app_handle, peer)?;
        remove_pending(&app_handle, &pending.group);
        roster::notify_peers_changed(&app_handle);
        let _ = app_handle.emit(
            "peer-pair-done",
            serde_json::json!({ "deviceId": pending.peer_device_id.clone(), "accepted": true }),
        );
        let message = self_handshake(&app_handle, "accept", &pending.group);
        if let Err(error) = exchange(
            &pending.peer_ip,
            pending.peer_port,
            &pending.peer_auth,
            &key,
            &message,
        )
        .await
        {
            // They simply do not know yet; the next handshake from them answers
            // "paired", so a lost reply is not a lost pairing.
            eprintln!(">>> [PEER] accept reply did not reach {}: {error}", pending.peer_device_id);
        }
        Ok(())
    } else {
        remove_pending(&app_handle, &pending.group);
        let _ = app_handle.emit(
            "peer-pair-rejected",
            serde_json::json!({ "deviceId": pending.peer_device_id.clone() }),
        );
        let message = self_handshake(&app_handle, "reject", &pending.group);
        if let Some(key) = sealed::decode_key(&pending.peer_key) {
            let _ = exchange(
                &pending.peer_ip,
                pending.peer_port,
                &pending.peer_auth,
                &key,
                &message,
            )
            .await;
        }
        Ok(())
    }
}

/// Answer a peer's request after the window was closed and reopened.
pub fn inbound_views(app: &AppHandle) -> Vec<PairRequestView> {
    prune_pending(app);
    let state = app.state::<PairingState>();
    let Ok(guard) = state.0.lock() else {
        return Vec::new();
    };
    guard
        .values()
        .filter(|pending| pending.direction == "in")
        .map(PendingPairing::view)
        .collect()
}

/// Called by the handshake handler when the answer reaches a request we sent.
pub fn resolve_outbound(app: &AppHandle, group: &str, accept: bool) -> Option<PendingPairing> {
    remove_pending(app, group).map(|pending| {
        if accept {
            let _ = app.emit(
                "peer-pair-done",
                serde_json::json!({ "deviceId": pending.peer_device_id.clone() }),
            );
        } else {
            let _ = app.emit(
                "peer-pair-rejected",
                serde_json::json!({ "deviceId": pending.peer_device_id.clone() }),
            );
        }
        pending
    })
}

#[tauri::command]
pub fn get_peer_identity(app_handle: AppHandle) -> super::models::PeerIdentityView {
    identity::current(&app_handle).view()
}

#[tauri::command]
pub fn set_peer_nickname(app_handle: AppHandle, nickname: String) -> Result<(), String> {
    identity::update_nickname(&app_handle, &nickname)
}

/// 本机头像底色 in 设备与用户. Filtered through `normalize_color` so the
/// identity file can only ever hold something a webview can paint with.
#[tauri::command]
pub fn set_self_avatar_color(app_handle: AppHandle, color: String) -> Result<(), String> {
    identity::update_avatar_color(&app_handle, &normalize_color(&color))
}

#[tauri::command]
pub fn list_peers(app_handle: AppHandle) -> Vec<super::models::PeerView> {
    roster::views(&app_handle)
}

#[tauri::command]
pub fn remove_peer(app_handle: AppHandle, device_id: String) -> Result<(), String> {
    // A machine can only be removed by someone who can already reach it, and
    // the roster is local: no network call, just forget it.
    db::forget_peer(&app_handle, &device_id)?;
    roster::notify_peers_changed(&app_handle);
    Ok(())
}

#[tauri::command]
pub fn update_peer_profile(
    app_handle: AppHandle,
    device_id: String,
    nickname: String,
    avatar_color: String,
) -> Result<(), String> {
    let color = normalize_color(&avatar_color);
    db::update_profile(&app_handle, &device_id, &nickname, &color)?;
    roster::notify_peers_changed(&app_handle);
    Ok(())
}

/// ⭐ in 设备与用户. Local to this machine: the peer is never told how its own
/// user ranks it, and a re-pair keeps the star (see `db::save_peer`).
#[tauri::command]
pub fn set_peer_favorite(
    app_handle: AppHandle,
    device_id: String,
    favorite: bool,
) -> Result<(), String> {
    db::set_favorite(&app_handle, &device_id, favorite)?;
    roster::notify_peers_changed(&app_handle);
    Ok(())
}

/// Only a plain `#rgb`/`#rrggbb` reaches the window as a CSS colour.
fn normalize_color(value: &str) -> String {
    let trimmed = value.trim().to_lowercase();
    let hex = trimmed.strip_prefix('#').unwrap_or(&trimmed);
    let valid = (hex.len() == 3 || hex.len() == 6) && hex.chars().all(|c| c.is_ascii_hexdigit());
    if valid {
        format!("#{hex}")
    } else {
        String::new()
    }
}

/// The identity a handshake reply should carry.
pub fn reply_identity(app: &AppHandle) -> PeerIdentity {
    identity::current(app)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colors_are_the_only_ones_a_webview_can_use() {
        assert_eq!(normalize_color("#0A84FF"), "#0a84ff");
        assert_eq!(normalize_color("fff"), "#fff");
        assert_eq!(normalize_color("red"), "");
        assert_eq!(normalize_color("#12345"), "");
        assert_eq!(normalize_color("#00g000"), "");
        assert_eq!(normalize_color(""), "");
    }

    #[test]
    fn a_handshake_json_round_trips() {
        let message = PeerHandshake {
            status: "request".to_string(),
            group: "g1".to_string(),
            device_id: "0123456789abcdef".to_string(),
            device_name: "Mac".to_string(),
            nickname: "书房".to_string(),
            platform: "macos".to_string(),
            port: 28462,
            auth: "pt_x".to_string(),
            key: sealed::encode_key(&[1u8; 32]),
        };
        let json = serde_json::to_vec(&message).unwrap();
        let back: PeerHandshake = serde_json::from_slice(&json).unwrap();
        assert_eq!(back.device_id, message.device_id);
        assert_eq!(back.nickname, "书房");
        assert_eq!(sealed::decode_key(&back.key).as_ref(), Some(&[1u8; 32]));
    }
}
