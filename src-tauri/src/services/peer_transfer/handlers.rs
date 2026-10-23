//! The routes a peer talks to: `/p/handshake`, `/p/invite` and
//! `/p/upload-chunk`.
//!
//! They live on the same axum server as the phone routes but authenticate
//! separately: a peer presents the durable token from its pairing code, not the
//! ephemeral session token that rotates every time the server starts. Uploads
//! then join the phone pipeline (`process_upload_chunk`), so only the trust
//! check differs — and it differs by requiring the sender to be a paired,
//! trusted device rather than merely a holder of the session token.

use axum::{
    extract::{ConnectInfo, Request, State},
    http::{header, HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::net::SocketAddr;
use std::sync::Arc;

use tauri::{AppHandle, Emitter, Manager};

use crate::services::file_transfer::handlers::{
    constant_time_eq, process_upload_chunk, TRANSFER_CHUNK_SIZE,
};
use crate::services::file_transfer::models::{AppState, ChunkMetadata};
use crate::services::file_transfer::sealed;

use super::client;
use super::db;
use super::discovery;
use super::models::{
    ConnectAsk, ConnectReply, InviteReply, PeerHandshake, PeerIdentity, PeerIdentityState,
    PeerInvite, PendingReceive, CONNECT_REQUEST_TTL,
};
use super::pairing;
use super::receive;
use super::roster;

/// Kind for every handshake frame, in both directions.
const HANDSHAKE_KIND: &str = "handshake";
/// Kind for the nearby-connect ask and its sealed answer.
const CONNECT_KIND: &str = "connect";

/// `None` means the peer feature never finished starting (or its lock broke);
/// every peer route then refuses rather than falling back to a default key,
/// which a caller on the LAN could otherwise guess.
fn our_identity(app: &AppHandle) -> Option<PeerIdentity> {
    let state = app.try_state::<PeerIdentityState>()?;
    state.0.read().ok().map(|guard| guard.clone())
}

fn unavailable() -> Response {
    (StatusCode::SERVICE_UNAVAILABLE, "Peer identity unavailable").into_response()
}

fn bearer(request: &Request) -> Option<&str> {
    let value = request.headers().get(header::AUTHORIZATION)?.to_str().ok()?;
    value.strip_prefix("Bearer ").map(|token| token.trim())
}

/// A peer is who it says it is if it presents this machine's pairing token,
/// which only ever left through that peer's own pairing code.
pub async fn require_peer_session(
    State(state): State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> Response {
    let expected = match our_identity(&state.app_handle) {
        Some(identity) => identity.auth_token,
        None => return unavailable(),
    };
    let authorized = !expected.is_empty()
        && bearer(&request)
            .is_some_and(|token| constant_time_eq(token.as_bytes(), expected.as_bytes()));
    if !authorized {
        return (StatusCode::UNAUTHORIZED, "Invalid pairing token").into_response();
    }
    // 「暂停接收」 from the menu bar: refused here, before a byte or a held
    // invite, so the sender gets a truthful answer instead of a stall.
    if crate::global_state::receiving_paused() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            "This machine is pausing incoming transfers",
        )
            .into_response();
    }
    let mut response = next.run(request).await;
    if let Ok(no_store) = HeaderValue::from_str("no-store") {
        response.headers_mut().insert(header::CACHE_CONTROL, no_store);
    }
    response
}

/// Short, control-character-free text for anything a peer claims about itself.
fn clean_field(value: &str, max: usize) -> String {
    super::roster::clean_text(value, max)
}

fn sealed_reply(their_key: &sealed::LanKey, reply: &PeerHandshake) -> Response {
    sealed_boxed(HANDSHAKE_KIND, their_key, reply)
}

fn reply_to(
    app: &AppHandle,
    ours: &PeerIdentity,
    message: &PeerHandshake,
    their_key: &sealed::LanKey,
    status: &str,
) -> Response {
    sealed_reply(
        their_key,
        &PeerHandshake {
            status: status.to_string(),
            group: message.group.clone(),
            device_id: ours.device_id.clone(),
            device_name: ours.device_name.clone(),
            nickname: ours.nickname.clone(),
            platform: ours.platform.clone(),
            port: pairing::server_port(app),
            auth: ours.auth_token.clone(),
            key: sealed::encode_key(&ours.key),
        },
    )
}

/// Pairing, answered by the machine that showed its code.
///
/// The request arrives sealed with *our* key (it came from our code), so an
/// attacker without the code cannot even produce a readable body. Our answer is
/// sealed with *their* key, which the request carried — that is what proves the
/// two machines hold the same pair of codes, alongside the verify word the user
/// reads on both screens.
pub async fn peer_handshake(
    State(state): State<Arc<AppState>>,
    ConnectInfo(remote): ConnectInfo<SocketAddr>,
    body: axum::body::Bytes,
) -> Response {
    let app = &state.app_handle;
    let Some(our_identity) = our_identity(app) else {
        return unavailable();
    };
    let Some(plain) = sealed::open(&our_identity.key, HANDSHAKE_KIND, &body) else {
        return (StatusCode::BAD_REQUEST, "Unreadable pairing message").into_response();
    };
    let Ok(message) = serde_json::from_slice::<PeerHandshake>(&plain) else {
        return (StatusCode::BAD_REQUEST, "Invalid pairing message").into_response();
    };
    let Some(their_key) = sealed::decode_key(&message.key) else {
        return (StatusCode::BAD_REQUEST, "Invalid pairing key").into_response();
    };
    if message.device_id.len() < 8 || message.device_id.len() > 64 {
        return (StatusCode::BAD_REQUEST, "Invalid device id").into_response();
    }
    // Our own id arriving from elsewhere means the two machines share a machine
    // id (a cloned disk); pairing them would put a copy of ourselves in the roster.
    if message.device_id == our_identity.device_id {
        return (StatusCode::CONFLICT, "这是本机自己的配对码").into_response();
    }

    match message.status.as_str() {
        "request" => {
            if db::is_trusted_sender(app, &message.device_id) {
                return reply_to(app, &our_identity, &message, &their_key, "paired");
            }
            // This machine's user just approved a nearby connect: the sealed
            // handshake is that machine completing what was accepted, so it
            // must not raise a second card.
            if pairing::take_approved_connect(app, &message.device_id) {
                let peer = db::peer_from_handshake(
                    &message.device_id,
                    &clean_field(&message.device_name, 80),
                    &clean_field(&message.nickname, 40),
                    &clean_field(&message.platform, 20),
                    &remote.ip().to_string(),
                    message.port,
                    &message.auth,
                    &message.key,
                );
                if let Some(peer) = peer {
                    if let Err(error) = db::save_peer(app, peer) {
                        eprintln!(">>> [PEER] failed to save connected peer: {error}");
                    }
                }
                roster::notify_peers_changed(app);
                let _ = app.emit(
                    "peer-pair-done",
                    serde_json::json!({
                        "deviceId": message.device_id.clone(),
                        "accepted": true,
                    }),
                );
                return reply_to(app, &our_identity, &message, &their_key, "accept");
            }
            let verify_word = roster::verify_word(
                (&our_identity.device_id, &our_identity.key),
                (&message.device_id, &their_key),
            );
            let announced = PeerHandshake {
                device_name: clean_field(&message.device_name, 80),
                nickname: clean_field(&message.nickname, 40),
                platform: clean_field(&message.platform, 20),
                ..message.clone()
            };
            let pending = pairing::record_inbound(
                app,
                &announced,
                &their_key,
                &remote.ip().to_string(),
                verify_word,
            );
            crate::app::peer_floats::on_pair_request(app);
            let _ = app.emit("peer-pair-request", pending.view());
            reply_to(app, &our_identity, &message, &their_key, "pending")
        }
        "accept" | "reject" => {
            let accept = message.status == "accept";
            if accept {
                if let Some(peer) = db::peer_from_handshake(
                    &message.device_id,
                    &clean_field(&message.device_name, 80),
                    &clean_field(&message.nickname, 40),
                    &clean_field(&message.platform, 20),
                    &remote.ip().to_string(),
                    message.port,
                    &message.auth,
                    &message.key,
                ) {
                    // `save_peer` keeps the nickname, colour and star set *here*:
                    // the handshake only tells us who the other machine is, not
                    // what we call it.
                    if let Err(error) = db::save_peer(app, peer) {
                        eprintln!(">>> [PEER] failed to save accepted peer: {error}");
                    }
                }
            }
            let _ = pairing::resolve_outbound(app, &message.group, accept);
            if accept {
                roster::notify_peers_changed(app);
            }
            let status = message.status.clone();
            reply_to(app, &our_identity, &message, &their_key, &status)
        }
        other => {
            let _ = other;
            (StatusCode::BAD_REQUEST, "Unknown pairing status").into_response()
        }
    }
}

fn sealed_connect_reply(their_key: &sealed::LanKey, reply: &ConnectReply) -> Response {
    sealed_boxed(CONNECT_KIND, their_key, reply)
}

/// A fresh pairing code, sealed only with the asker's key. The IP inside is a
/// best guess; the asker already knows an address that works and keeps it.
fn connect_pairing_code(app: &AppHandle, identity: &PeerIdentity) -> String {
    let ip = crate::services::file_transfer::get_local_ip_addr(app.clone());
    roster::build_pairing_code(identity, &ip, pairing::server_port(app))
}

/// `POST /p/connect`: "I can see you in my nearby list; may we pair?"
///
/// No token, no secret in the ask. The answer carries a pairing code, so it is
/// sealed with the asker's key and only sent after this machine's user slides
/// to accept. That makes clicking an avatar the same trust decision as reading
/// a code aloud — with the code exchanged by the two machines instead.
pub async fn peer_connect(
    State(state): State<Arc<AppState>>,
    ConnectInfo(remote): ConnectInfo<SocketAddr>,
    body: axum::body::Bytes,
) -> Response {
    let app = state.app_handle.clone();
    let Some(our_identity) = our_identity(&app) else {
        return unavailable();
    };
    let Ok(ask) = serde_json::from_slice::<ConnectAsk>(&body) else {
        return (StatusCode::BAD_REQUEST, "Invalid connect request").into_response();
    };
    if ask.device_id.len() < 8
        || ask.device_id.len() > 64
        || !ask.device_id.chars().all(|c| c.is_ascii_hexdigit())
    {
        return (StatusCode::BAD_REQUEST, "Invalid device id").into_response();
    }
    if ask.device_id == our_identity.device_id {
        return (StatusCode::CONFLICT, "这是本机自己").into_response();
    }
    let Some(their_key) = sealed::decode_key(&ask.key) else {
        return (StatusCode::BAD_REQUEST, "Invalid connect key").into_response();
    };
    if crate::global_state::receiving_paused() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            "This machine is pausing incoming transfers",
        )
            .into_response();
    }

    // Already paired here. If the ask carries that peer's stored key it is the
    // same machine, so a one-sided roster can heal without walking to the
    // other screen; a different key is an id collision and gets no code.
    if let Some(peer) = db::send_target(&app, &ask.device_id) {
        let same_machine = sealed::encode_key(&peer.key) == ask.key;
        return sealed_connect_reply(
            &their_key,
            &ConnectReply {
                status: if same_machine { "accept" } else { "already" }.to_string(),
                code: if same_machine {
                    connect_pairing_code(&app, &our_identity)
                } else {
                    String::new()
                },
            },
        );
    }

    if !discovery::is_enabled(&app) {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            "Discovery is turned off",
        )
            .into_response();
    }

    let Some((view, receiver)) =
        pairing::record_connect_ask(&app, &ask, remote.ip().to_string())
    else {
        return (StatusCode::TOO_MANY_REQUESTS, "Too many connect requests").into_response();
    };
    crate::app::peer_floats::on_pair_request(&app);
    let _ = app.emit("peer-pair-request", view);

    // The user has a minute; meanwhile this request is the only channel the
    // asker has, so it stays open and then answers truthfully.
    let decision = tokio::time::timeout(CONNECT_REQUEST_TTL, receiver).await;
    pairing::finish_connect_ask(&app, &ask.device_id);
    let reply = match decision {
        Ok(Ok(true)) => {
            pairing::approve_connect(&app, &ask.device_id);
            ConnectReply {
                status: "accept".to_string(),
                code: connect_pairing_code(&app, &our_identity),
            }
        }
        Ok(Ok(false)) => ConnectReply {
            status: "reject".to_string(),
            code: String::new(),
        },
        _ => ConnectReply {
            status: "timeout".to_string(),
            code: String::new(),
        },
    };
    sealed_connect_reply(&their_key, &reply)
}

/// The inbound window's view of one accepted chunk. `process_upload_chunk`
/// answers 200 for every chunk, so the last chunk is what makes it a completion
/// — the same test the pipeline itself uses to finalize the file.
fn report_inbound(app: &AppHandle, meta: &ChunkMetadata, status: &str, error: Option<String>) {
    let last = meta.chunk_index + 1 >= meta.total_chunks;
    let received = if status == "done" || last {
        meta.total_size
    } else {
        std::cmp::min(
            meta.total_size,
            (meta.chunk_index as u64 + 1) * TRANSFER_CHUNK_SIZE as u64,
        )
    };
    client::notify_inbound(
        app,
        &meta.sender_id,
        // The name is whatever the sender typed into its own settings; this is
        // the window that shows it, so clean it here rather than trust it.
        &clean_field(&meta.sender_name, 80),
        &meta.file_name,
        &meta.upload_id,
        received,
        meta.total_size,
        status,
        error,
        meta.batch_id.clone(),
        meta.batch_name.clone(),
        meta.batch_index,
        meta.batch_total,
    );
}

/// A peer's chunk, sealed with the key from its pairing — then the phone's own
/// upload pipeline, which is where offsets, disk reserve and file names live.
pub async fn peer_upload_chunk(
    State(state): State<Arc<AppState>>,
    body: axum::body::Bytes,
) -> Response {
    // Owned, because the pipeline below takes the shared state.
    let app = state.app_handle.clone();
    let Some(our_identity) = our_identity(&app) else {
        return unavailable();
    };
    let Some(plain) = sealed::open(&our_identity.key, "upload", &body) else {
        return (StatusCode::BAD_REQUEST, "Unreadable sealed body").into_response();
    };
    let Some((meta, data)) = sealed::split_upload(&plain) else {
        return (StatusCode::BAD_REQUEST, "Invalid upload payload").into_response();
    };
    let Ok(meta) = serde_json::from_slice::<ChunkMetadata>(meta) else {
        return (StatusCode::BAD_REQUEST, "Invalid chunk metadata").into_response();
    };
    // The token middleware proved the pairing; this proves the device. A machine
    // that has asked but not been accepted yet is refused until the user says yes.
    if !db::is_trusted_sender(&app, &meta.sender_id) {
        return (StatusCode::FORBIDDEN, "Device is not paired here").into_response();
    }
    let last_chunk = meta.chunk_index + 1 >= meta.total_chunks;
    let response = process_upload_chunk(state, meta.clone(), data.to_vec()).await;
    if response.status().is_success() {
        report_inbound(
            &app,
            &meta,
            if last_chunk { "done" } else { "active" },
            None,
        );
    } else if response.status().is_client_error()
        || response.status() == StatusCode::INSUFFICIENT_STORAGE
    {
        // A refusal is final: the sender stops, so the box here stops too. A 5xx
        // may still be retried, and a premature "failed" would be wrong.
        report_inbound(&app, &meta, "failed", Some("接收失败".to_string()));
    }
    response
}

/// Kind for an invite and its answer.
const INVITE_KIND: &str = "invite";

fn sealed_boxed(kind: &str, their_key: &sealed::LanKey, reply: &impl serde::Serialize) -> Response {
    let Ok(body) = serde_json::to_vec(reply) else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };
    match sealed::seal(their_key, kind, &body) {
        Some(boxed) => (
            [
                (header::CONTENT_TYPE, HeaderValue::from_static("application/octet-stream")),
                (header::CACHE_CONTROL, HeaderValue::from_static("no-store")),
            ],
            boxed,
        )
            .into_response(),
        None => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

/// "May I send you these files?" — asked before a byte moves, and answered by
/// the user when this machine is set to confirm.
///
/// The request stays open for as long as the user takes (up to the timeout in
/// `receive`), because the answer goes back on this same call: the sender has
/// no other way to learn whether it may start.
pub async fn peer_invite(State(state): State<Arc<AppState>>, body: axum::body::Bytes) -> Response {
    let app = state.app_handle.clone();
    let Some(our_identity) = our_identity(&app) else {
        return unavailable();
    };
    let Some(plain) = sealed::open(&our_identity.key, INVITE_KIND, &body) else {
        return (StatusCode::BAD_REQUEST, "Unreadable sealed body").into_response();
    };
    let Ok(invite) = serde_json::from_slice::<PeerInvite>(&plain) else {
        return (StatusCode::BAD_REQUEST, "Invalid invite").into_response();
    };
    if invite.device_id.len() < 8
        || invite.device_id.len() > 64
        || !invite.device_id.chars().all(|c| c.is_ascii_hexdigit())
    {
        return (StatusCode::BAD_REQUEST, "Invalid device id").into_response();
    }
    if invite.device_id == our_identity.device_id {
        return (StatusCode::CONFLICT, "本机不能给自己发送").into_response();
    }
    if !db::is_trusted_sender(&app, &invite.device_id) {
        return (StatusCode::FORBIDDEN, "Device is not paired here").into_response();
    }
    let Some(their_key) = sealed::decode_key(&invite.key) else {
        return (StatusCode::BAD_REQUEST, "Invalid transfer key").into_response();
    };
    let files = receive::invite_files(&invite.files);
    if files.is_empty() || files.len() > 2_000 {
        return (StatusCode::BAD_REQUEST, "Invalid file list").into_response();
    }

    let accepted = if receive::confirm_required(&app) {
        let (responder, decision) = tokio::sync::oneshot::channel();
        let nickname = clean_field(&invite.nickname, 40);
        let device_name = clean_field(&invite.device_name, 80);
        let pending = PendingReceive {
            group: invite.group.clone(),
            device_id: invite.device_id.clone(),
            device_name: if nickname.is_empty() { device_name } else { nickname },
            platform: clean_field(&invite.platform, 20),
            files,
            total_size: invite.total_size,
            expires_at: std::time::Instant::now() + receive::decide_timeout(&app),
            responder: Some(responder),
        };
        receive::await_decision(&app, pending, decision).await
    } else {
        true
    };

    sealed_boxed(
        INVITE_KIND,
        &their_key,
        &InviteReply {
            group: invite.group,
            decision: if accepted { "accept" } else { "reject" }.to_string(),
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn handshake(status: &str) -> PeerHandshake {
        PeerHandshake {
            status: status.to_string(),
            group: "g1".to_string(),
            device_id: "0123456789abcdef".to_string(),
            device_name: "Conley 的 Mac".to_string(),
            nickname: "书房".to_string(),
            platform: "macos".to_string(),
            port: 28462,
            auth: "pt_0123456789abcdef".to_string(),
            key: sealed::encode_key(&[2u8; 32]),
        }
    }

    #[test]
    fn a_handshake_only_unseals_with_the_receiver_key() {
        let our_key = [1u8; 32];
        let body = serde_json::to_vec(&handshake("request")).unwrap();
        let boxed = sealed::seal(&our_key, HANDSHAKE_KIND, &body).unwrap();
        assert_eq!(
            sealed::open(&our_key, HANDSHAKE_KIND, &boxed).as_deref(),
            Some(body.as_slice())
        );
        assert!(sealed::open(&[9u8; 32], HANDSHAKE_KIND, &boxed).is_none());
        // The reply is a different key space: the sender's, not ours.
        assert!(sealed::open(&our_key, HANDSHAKE_KIND, &sealed::seal(&[2u8; 32], HANDSHAKE_KIND, &body).unwrap()).is_none());
    }

    #[test]
    fn claimed_names_cannot_carry_control_characters() {
        assert_eq!(clean_field("书房\u{202e}Mac", 80), "书房Mac");
        assert_eq!(clean_field("  spaced  ", 80), "spaced");
        assert_eq!(clean_field("0123456789", 4), "0123");
    }

    #[test]
    fn a_bearer_header_is_the_only_credential_a_peer_sends() {
        let request = axum::extract::Request::builder()
            .header("authorization", "Bearer pt_x")
            .body(axum::body::Body::empty())
            .unwrap();
        assert_eq!(bearer(&request), Some("pt_x"));
        // No query fallback: a peer never puts its token in a URL a proxy log
        // could keep, and nothing here is loaded by a browser.
        let query = axum::extract::Request::builder()
            .uri("/p/upload-chunk?auth=pt_x")
            .body(axum::body::Body::empty())
            .unwrap();
        assert_eq!(bearer(&query), None);
        let wrong_scheme = axum::extract::Request::builder()
            .header("authorization", "token pt_x")
            .body(axum::body::Body::empty())
            .unwrap();
        assert_eq!(bearer(&wrong_scheme), None);
    }
}
