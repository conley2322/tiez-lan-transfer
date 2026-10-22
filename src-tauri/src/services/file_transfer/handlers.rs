use axum::{
    body::Body,
    extract::{
        ws::{Message as WsMessage, WebSocket},
        Multipart, Path, Query, Request, State, WebSocketUpgrade,
    },
    http::{header, HeaderMap, HeaderValue, Method, StatusCode},
    middleware::Next,
    response::{Html, IntoResponse, Json, Response},
};
use base64::Engine;
use futures::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{Emitter, Manager};
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt, SeekFrom};
use tokio_util::io::ReaderStream;

use crate::app_state::{SessionHistory, SettingsState};
use crate::database::ClipboardEntry;
use crate::database::DbState;
use crate::infrastructure::repository::clipboard_repo::ClipboardRepository;
use crate::infrastructure::repository::settings_repo::SettingsRepository;

use super::models::*;
use super::sealed;
use super::utils::*;
use super::web_ui::render_index_with_palette;
use super::{
    append_message, append_message_with_batch, register_received_file, register_shared_file,
};

pub(crate) const TRANSFER_CHUNK_SIZE: usize = 512 * 1024;
const MIN_DISK_RESERVE: u64 = 512 * 1024 * 1024;
const MAX_DISK_RESERVE: u64 = 5 * 1024 * 1024 * 1024;

fn ensure_disk_capacity(path: &std::path::Path, required: u64) -> Result<(), &'static str> {
    let available = fs2::available_space(path).map_err(|_| "Unable to inspect free disk space")?;
    let total = fs2::total_space(path).map_err(|_| "Unable to inspect disk capacity")?;
    let reserve = (total / 50).clamp(MIN_DISK_RESERVE, MAX_DISK_RESERVE);
    if available.saturating_sub(reserve) < required {
        Err("Not enough disk space while preserving the system safety reserve")
    } else {
        Ok(())
    }
}

fn safe_file_name(value: &str) -> String {
    let normalized = value.replace('\\', "/");
    let base = normalized.rsplit('/').next().unwrap_or("");
    let cleaned: String = base
        .chars()
        // Invisible direction/format marks can disguise "x\u{202E}gpj.exe" as
        // an image name; they never belong in a file name.
        .filter(|ch| !ch.is_control() && !is_format_char(*ch) && *ch != '/' && *ch != '\\')
        // Characters Windows rejects (':' would even write an NTFS stream).
        .map(|ch| if matches!(ch, '<' | '>' | ':' | '"' | '|' | '?' | '*') { '_' } else { ch })
        .take(240)
        .collect();
    let cleaned = cleaned.trim().trim_matches('.').trim();
    if cleaned.is_empty() {
        return "unnamed-file".to_string();
    }
    // CON, NUL, COM1… are device names on Windows, with or without extension.
    let stem = cleaned.split('.').next().unwrap_or_default().trim_end();
    let reserved = ["CON", "PRN", "AUX", "NUL"].iter().any(|name| stem.eq_ignore_ascii_case(name))
        || matches!(stem.as_bytes(), [a, b, c, digit]
            if (b"COM".eq_ignore_ascii_case(&[*a, *b, *c]) || b"LPT".eq_ignore_ascii_case(&[*a, *b, *c]))
                && digit.is_ascii_digit());
    if reserved {
        format!("_{cleaned}")
    } else {
        cleaned.to_string()
    }
}

pub(crate) fn is_format_char(ch: char) -> bool {
    matches!(
        ch,
        '\u{061C}' | '\u{200B}'..='\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2060}'..='\u{2069}' | '\u{FEFF}'
    )
}

/// The digest a sender promised, lowercase, or `None` when it sent none.
/// Anything that isn't 64 hex chars is not a SHA-256 and is ignored rather
/// than trusted.
fn checksum_of(meta: &ChunkMetadata) -> Option<String> {
    let claimed = meta.sha256.as_deref().map(str::trim).unwrap_or("");
    (claimed.len() == 64 && claimed.chars().all(|c| c.is_ascii_hexdigit()))
        .then(|| claimed.to_lowercase())
}

/// Hex SHA-256 of a finished upload. Off the async thread: a multi-gigabyte
/// file must not stall the server while it is read once more.
async fn digest_file_hex(path: &std::path::Path) -> Option<String> {
    let path = path.to_path_buf();
    tokio::task::spawn_blocking(move || {
        use sha2::Digest;
        let mut file = std::fs::File::open(&path).ok()?;
        let mut hasher = sha2::Sha256::new();
        let mut buffer = vec![0u8; 1024 * 1024];
        loop {
            let read = std::io::Read::read(&mut file, &mut buffer).ok()?;
            if read == 0 {
                break;
            }
            hasher.update(&buffer[..read]);
        }
        Some(format!("{:x}", hasher.finalize()))
    })
    .await
    .ok()
    .flatten()
}

/// Compare secrets without an early exit, so response timing over the LAN
/// doesn't reveal how many leading characters of a guess were right.
pub(crate) fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |diff, (x, y)| diff | (x ^ y)) == 0
}

fn safe_upload_id(value: &str) -> Option<&str> {
    if !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
    {
        Some(value)
    } else {
        None
    }
}

fn safe_identity(value: &str, fallback: &str) -> String {
    let value: String = value
        .chars()
        .filter(|ch| !ch.is_control())
        .take(128)
        .collect();
    if value.trim().is_empty() {
        fallback.to_string()
    } else {
        value
    }
}

const UPLOAD_SESSION_TTL: std::time::Duration = std::time::Duration::from_secs(60 * 60);
const MAX_UPLOAD_SESSIONS: usize = 256;

fn prune_upload_sessions(sessions: &mut std::collections::HashMap<String, std::path::PathBuf>) {
    let stale_keys = sessions
        .iter()
        .filter_map(|(upload_id, path)| {
            let is_stale = std::fs::metadata(path)
                .and_then(|metadata| metadata.modified())
                .ok()
                .and_then(|modified| modified.elapsed().ok())
                .map(|elapsed| elapsed >= UPLOAD_SESSION_TTL)
                .unwrap_or(false);
            is_stale.then(|| upload_id.clone())
        })
        .collect::<Vec<_>>();

    for upload_id in stale_keys {
        if let Some(path) = sessions.remove(&upload_id) {
            let _ = std::fs::remove_file(path);
        }
    }

    while sessions.len() >= MAX_UPLOAD_SESSIONS {
        let Some(upload_id) = sessions.keys().next().cloned() else {
            break;
        };
        if let Some(path) = sessions.remove(&upload_id) {
            let _ = std::fs::remove_file(path);
        }
    }
}

fn validate_chunk(meta: &ChunkMetadata, data_len: usize) -> Result<(), &'static str> {
    if safe_upload_id(&meta.upload_id).is_none() {
        return Err("Invalid upload id");
    }
    let expected_chunks = std::cmp::max(
        1_u64,
        meta.total_size
            .saturating_add(TRANSFER_CHUNK_SIZE as u64 - 1)
            / TRANSFER_CHUNK_SIZE as u64,
    );
    if meta.total_chunks as u64 != expected_chunks || meta.chunk_index >= meta.total_chunks {
        return Err("Invalid chunk metadata");
    }
    let expected_len = if meta.total_size == 0 {
        0_u64
    } else if meta.chunk_index + 1 == meta.total_chunks {
        meta.total_size
            .saturating_sub(meta.chunk_index as u64 * TRANSFER_CHUNK_SIZE as u64)
    } else {
        TRANSFER_CHUNK_SIZE as u64
    };
    if data_len as u64 != expected_len {
        return Err("Invalid chunk size");
    }
    Ok(())
}

fn parse_single_byte_range(value: &str, total_size: u64) -> Option<(u64, u64)> {
    if total_size == 0 {
        return None;
    }

    let value = value.strip_prefix("bytes=")?;
    if value.contains(',') {
        return None;
    }
    let (start, end) = value.split_once('-')?;

    if start.is_empty() {
        let suffix_length = end.parse::<u64>().ok()?;
        if suffix_length == 0 {
            return None;
        }
        let length = suffix_length.min(total_size);
        return Some((total_size - length, total_size - 1));
    }

    let start = start.parse::<u64>().ok()?;
    if start >= total_size {
        return None;
    }
    let end = if end.is_empty() {
        total_size - 1
    } else {
        end.parse::<u64>().ok()?.min(total_size - 1)
    };
    (end >= start).then_some((start, end))
}

fn request_token(request: &Request) -> Option<String> {
    if let Some(value) = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
    {
        return Some(value.to_string());
    }

    if let Some(value) = request
        .uri()
        .query()
        .and_then(|query| query.split('&').find_map(|part| part.strip_prefix("auth=")))
        .and_then(|value| urlencoding::decode(value).ok())
    {
        return Some(value.into_owned());
    }

    request
        .headers()
        .get(header::COOKIE)
        .and_then(|value| value.to_str().ok())
        .and_then(|cookies| {
            cookies.split(';').find_map(|cookie| {
                let (name, value) = cookie.trim().split_once('=')?;
                (name == "tiez_ft_session").then(|| value.to_string())
            })
        })
}

fn apply_security_headers(response: &mut Response) {
    let headers = response.headers_mut();
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("no-referrer"),
    );
    headers.insert("x-frame-options", HeaderValue::from_static("DENY"));
    headers.insert(
        "permissions-policy",
        HeaderValue::from_static("camera=(), microphone=(), geolocation=()"),
    );
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(
            "default-src 'self'; img-src 'self' data: blob:; media-src 'self' blob:; style-src 'self' 'unsafe-inline'; script-src 'self' 'unsafe-inline'; connect-src 'self' ws: wss:; frame-ancestors 'none'; base-uri 'none'; form-action 'self'",
        ),
    );
}

pub async fn apply_download_security_headers(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    apply_security_headers(&mut response);
    // Downloaded files are untrusted: an SVG or HTML file opened in the
    // browser must not run script on the transfer origin (where it could use
    // the session cookie). `sandbox` also gives it an opaque origin.
    response.headers_mut().insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(
            "default-src 'none'; img-src 'self' data: blob:; media-src 'self' blob:; style-src 'unsafe-inline'; sandbox",
        ),
    );
    response
}

pub async fn require_session(
    State(state): State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> Response {
    let expected = state
        .app_handle
        .state::<ServerInfo>()
        .access_token
        .lock()
        .map(|token| token.clone())
        .unwrap_or_default();
    let supplied = request_token(&request);
    let from_query = request
        .uri()
        .query()
        .is_some_and(|query| query.split('&').any(|part| part.starts_with("auth=")));

    let is_preflight = request.method() == Method::OPTIONS;
    let mut response = if is_preflight
        || (!expected.is_empty()
            && supplied
                .as_deref()
                .is_some_and(|token| constant_time_eq(token.as_bytes(), expected.as_bytes())))
    {
        next.run(request).await
    } else {
        (
            StatusCode::UNAUTHORIZED,
            "Invalid or expired transfer session",
        )
            .into_response()
    };

    if from_query && response.status() != StatusCode::UNAUTHORIZED {
        if let Ok(cookie) = HeaderValue::from_str(&format!(
            "tiez_ft_session={}; Path=/; HttpOnly; SameSite=Strict; Max-Age=28800",
            expected
        )) {
            response.headers_mut().append(header::SET_COOKIE, cookie);
        }
    }
    apply_security_headers(&mut response);
    response
}

/// Origins of TieZ's own window, the only page allowed to call the server
/// cross-origin (it shares dropped files through `/share-chunk`).
fn is_app_origin(origin: &str) -> bool {
    matches!(
        origin,
        "tauri://localhost" | "http://tauri.localhost" | "https://tauri.localhost"
    ) || (cfg!(debug_assertions) && origin == "http://localhost:1420")
}

/// Allows cross-origin calls from TieZ's window only, never `*`.
pub async fn app_origin_cors(request: Request, next: Next) -> Response {
    let origin = request
        .headers()
        .get(header::ORIGIN)
        .and_then(|value| value.to_str().ok())
        .filter(|origin| is_app_origin(origin))
        .and_then(|origin| HeaderValue::from_str(origin).ok());
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert(header::VARY, HeaderValue::from_static("Origin"));
    if let Some(origin) = origin {
        headers.insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, origin);
    }
    response
}

fn with_cors(mut response: axum::response::Response) -> axum::response::Response {
    let headers = response.headers_mut();
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_METHODS,
        HeaderValue::from_static("POST, OPTIONS"),
    );
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_HEADERS,
        HeaderValue::from_static("*"),
    );
    response
}

pub async fn index(State(state): State<Arc<AppState>>) -> Html<String> {
    let app_handle = &state.app_handle;
    let settings = app_handle.state::<SettingsState>();
    let db_state = app_handle.state::<DbState>();
    let logo_base64 = get_app_logo_base64(app_handle);
    let theme = {
        let guard = settings.theme.lock().unwrap();
        guard.clone()
    };
    let color_mode = db_state
        .settings_repo
        .get("app.color_mode")
        .ok()
        .flatten()
        .unwrap_or_else(|| "system".to_string());

    let native_palette = db_state
        .settings_repo
        .get("app.native_palette")
        .ok()
        .flatten();
    Html(render_index_with_palette(
        &theme,
        &color_mode,
        &logo_base64,
        native_palette.as_deref(),
    ))
}

fn last_id_param(params: &HashMap<String, String>) -> u64 {
    params
        .get("last_id")
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0)
}

pub async fn poll_messages(
    Query(params): Query<HashMap<String, String>>,
    State(state): State<Arc<AppState>>,
) -> Json<Vec<Message>> {
    Json(collect_messages(&state, last_id_param(&params)))
}

/// Chat messages after `last_id`, with local file paths swapped for download
/// links (`/download/<token>?name=`).
fn collect_messages(state: &AppState, last_id: u64) -> Vec<Message> {
    let chat_state = state.app_handle.state::<ChatState>();

    let msgs_result = {
        match chat_state.0.lock() {
            Ok(msgs) => msgs
                .iter()
                .filter(|m| m.id > last_id)
                .map(|m| {
                    let mut m_clone = m.clone();
                    if matches!(m.msg_type.as_str(), "image" | "video" | "file")
                        && !m.content.starts_with("data:")
                    {
                        if let Some(shared_path) = m.file_path.as_deref() {
                            let path = std::path::Path::new(shared_path);
                            let filename = path.file_name().unwrap_or_default().to_string_lossy();

                            if let Some(token) =
                                register_shared_file(&state.app_handle, shared_path.to_string())
                            {
                                m_clone.content = format!(
                                    "/download/{}?name={}",
                                    token,
                                    urlencoding::encode(&filename)
                                );
                            }
                        } else if !m.content.starts_with("/download/") {
                            let path = std::path::Path::new(&m.content);
                            let filename = path.file_name().unwrap_or_default().to_string_lossy();
                            if let Some(token) =
                                register_shared_file(&state.app_handle, m.content.clone())
                            {
                                m_clone.content = format!(
                                    "/download/{}?name={}",
                                    token,
                                    urlencoding::encode(&filename)
                                );
                            }
                        }
                    }
                    m_clone.file_path = None;
                    m_clone
                })
                .collect::<Vec<Message>>(),
            Err(_) => vec![],
        }
    };

    msgs_result
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<HashMap<String, String>>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    // `?s=1`: every frame is a sealed box (sealed.rs). Without it the socket
    // speaks plain JSON for TieZ apps that predate encryption.
    let sealed_mode = params.get("s").is_some_and(|v| v == "1");
    ws.on_upgrade(move |socket| handle_socket(socket, state, sealed_mode))
}

#[derive(serde::Deserialize)]
#[serde(tag = "type")]
enum WsIncoming {
    #[serde(rename = "identity")]
    Identity {
        device_id: String,
        device_name: String,
    },
    #[serde(rename = "ping")]
    Ping,
    #[serde(other)]
    Unknown,
}

pub async fn handle_socket(socket: WebSocket, state: Arc<AppState>, sealed_mode: bool) {
    let (mut sender, mut receiver) = socket.split();
    let mut rx = state.ws_tx.subscribe();
    let mut shutdown = state.shutdown.clone();
    // Set once the phone says who it is. Removed from the device list after
    // both tasks end, however they end: one is aborted when the other stops.
    let connected_device: Arc<std::sync::Mutex<Option<String>>> = Arc::default();
    let key = state.lan_key;

    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            let frame = if sealed_mode {
                match sealed::seal(&key, "ws", msg.as_bytes()) {
                    Some(boxed) => WsMessage::Binary(boxed.into()),
                    None => break,
                }
            } else {
                WsMessage::Text(msg.into())
            };
            if sender.send(frame).await.is_err() {
                break;
            }
        }
    });

    let state_inner = state.clone();
    let connected = connected_device.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            let text = match (sealed_mode, msg) {
                (true, WsMessage::Binary(boxed)) => sealed::open(&key, "ws", &boxed)
                    .and_then(|plain| String::from_utf8(plain).ok()),
                (false, WsMessage::Text(text)) => Some(text.to_string()),
                _ => None,
            };
            if let Some(text) = text {
                if let Ok(incoming) = serde_json::from_str::<WsIncoming>(&text) {
                    match incoming {
                        WsIncoming::Ping => {
                            let current_device_id = connected.lock().unwrap().clone();
                            if let Some(ref id) = current_device_id {
                                let online_devices = state_inner.app_handle.state::<OnlineDevices>();
                                if let Ok(mut guard) = online_devices.0.lock() {
                                    if let Some(device) = guard.get_mut(id) {
                                        device.last_seen = chrono::Utc::now().timestamp_millis();
                                    }
                                };
                            }
                        }
                        WsIncoming::Unknown => {}
                        WsIncoming::Identity {
                            device_id,
                            device_name,
                        } => {
                            let device_id = safe_identity(&device_id, "mobile");
                            let device_name = safe_identity(&device_name, "手机");
                            *connected.lock().unwrap() = Some(device_id.clone());
                            let online_devices = state_inner.app_handle.state::<OnlineDevices>();
                            {
                                let mut guard = online_devices.0.lock().unwrap();
                                guard.insert(
                                    device_id.clone(),
                                    DeviceInfo {
                                        id: device_id.clone(),
                                        name: device_name,
                                        last_seen: chrono::Utc::now().timestamp_millis(),
                                    },
                                );

                                let devices: Vec<DeviceInfo> = guard.values().cloned().collect();
                                let update = serde_json::json!({
                                    "type": "devices_update",
                                    "devices": devices
                                });
                                let _ = state_inner.ws_tx.send(update.to_string());
                                let _ = state_inner
                                    .app_handle
                                    .emit("online-devices-updated", devices);
                            }
                        }
                    }
                }
            }
        }

    });

    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
        // Transfer turned off: drop phones that are still connected.
        _ = shutdown.changed() => {
            send_task.abort();
            recv_task.abort();
        }
    }

    let left = connected_device.lock().unwrap().take();
    if let Some(id) = left {
        let online_devices = state.app_handle.state::<OnlineDevices>();
        let mut guard = online_devices.0.lock().unwrap();
        guard.remove(&id);
        let devices: Vec<DeviceInfo> = guard.values().cloned().collect();
        let update = serde_json::json!({
            "type": "devices_update",
            "devices": devices
        });
        let _ = state.ws_tx.send(update.to_string());
        let _ = state.app_handle.emit("online-devices-updated", devices);
    }
}

pub async fn handle_text(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ReceiveText>,
) -> axum::response::Response {
    receive_text(state, payload).await
}

async fn receive_text(state: Arc<AppState>, payload: ReceiveText) -> axum::response::Response {
    let db_state = state.app_handle.state::<DbState>();
    update_activity(&state.app_handle);

    if payload.content.len() > 1024 * 1024 {
        return (StatusCode::PAYLOAD_TOO_LARGE, "Text is too large").into_response();
    }
    let sender_id = safe_identity(&payload.sender_id, "mobile");
    let sender_name = safe_identity(&payload.sender_name, "手机");

    append_message(
        &state.app_handle,
        "in",
        "text",
        &payload.content,
        &sender_id,
        &sender_name,
        None,
    );

    let settings = state.app_handle.state::<SettingsState>();
    // The text is already delivered to the chat above; saving it to history is opt-in.
    if !settings.auto_copy_file.load(Ordering::Relaxed) {
        return (StatusCode::OK, "Text received").into_response();
    }
    let session_hist = state.app_handle.state::<SessionHistory>();

    let mut preview = payload.content.clone();
    if preview.chars().count() > 100 {
        preview = preview.chars().take(100).collect();
        preview.push_str("...");
    }

    let id_result = if settings.persistent.load(Ordering::Relaxed) {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;
        let entry = ClipboardEntry {
            id: 0,
            content_type: "text".to_string(),
            content: payload.content.clone(),
            html_content: None,
            source_app: sender_name.to_string(),
            source_app_path: None,
            timestamp,
            created_at: timestamp,
            last_used_at: 0,
            sort_at: timestamp,
            preview: preview.clone(),
            is_pinned: false,
            tags: Vec::new(),
            note: String::new(),
            use_count: 0,
            is_external: false,
            pinned_order: 0,
            file_preview_exists: true,
        };
        db_state.repo.save(&entry, None).map_err(|e| {
            rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                e,
            )))
        })
    } else {
        let id = -(SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_micros() as i64
            / 1000);
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;
        let entry = ClipboardEntry {
            id,
            content_type: "text".to_string(),
            content: payload.content.clone(),
            html_content: None,
            source_app: "File Transfer".to_string(),
            source_app_path: None,
            timestamp,
            created_at: timestamp,
            last_used_at: 0,
            sort_at: timestamp,
            preview: preview.clone(),
            is_pinned: false,
            tags: Vec::new(),
            note: String::new(),
            use_count: 0,
            is_external: false,
            pinned_order: 0,
            file_preview_exists: true,
        };

        if let Ok(mut session) = session_hist.0.lock() {
            session.push_back(entry);
            if session.len() > 500 {
                if let Some(removed) = session.pop_front() {
                    let _ = state.app_handle.emit("clipboard-removed", removed.id);
                }
            }
        }
        Ok(id)
    };

    if let Ok(id) = id_result {
        if id != 0 {
            let _ = state.app_handle.emit("clipboard-changed", id);
            return (StatusCode::OK, "Text received").into_response();
        }
    }
    (StatusCode::INTERNAL_SERVER_ERROR, "Failed to save text").into_response()
}

pub async fn upload(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> axum::response::Response {
    update_activity(&state.app_handle);
    let mut success = false;
    let db_state = state.app_handle.state::<DbState>();

    let mut current_sender_id = "mobile".to_string();
    let mut current_sender_name = "手机".to_string();

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();

        if name == "sender_id" {
            if let Ok(val) = field.text().await {
                current_sender_id = safe_identity(&val, "mobile");
            }
            continue;
        }
        if name == "sender_name" {
            if let Ok(val) = field.text().await {
                current_sender_name = safe_identity(&val, "手机");
            }
            continue;
        }

        if name == "file" {
            let file_name = safe_file_name(field.file_name().unwrap_or("unknown.txt"));
            let content_type = field
                .content_type()
                .unwrap_or("application/octet-stream")
                .to_string();

            let mut save_dir = state
                .app_handle
                .path()
                .download_dir()
                .unwrap_or_else(|_| std::env::temp_dir());
            if let Ok(Some(custom)) = db_state.settings_repo.get("file_transfer_path") {
                if !custom.trim().is_empty() {
                    save_dir = std::path::PathBuf::from(custom);
                }
            }
            if !save_dir.exists() {
                let _ = std::fs::create_dir_all(&save_dir);
            }

            let Ok(target_path) =
                super::utils::reserve_unique_path(&save_dir, "", &file_name)
            else {
                continue;
            };

            if let Ok(mut file) = File::create(&target_path).await {
                let mut stream = field;
                let mut write_success = true;
                while let Some(next) = stream.next().await {
                    // A broken stream (body over the limit, sender gone) is a
                    // failed upload, not a finished one.
                    let Ok(chunk) = next else {
                        write_success = false;
                        break;
                    };
                    if ensure_disk_capacity(&save_dir, chunk.len() as u64).is_err() {
                        write_success = false;
                        break;
                    }
                    if let Err(e) = file.write_all(&chunk).await {
                        eprintln!("Error writing: {}", e);
                        write_success = false;
                        break;
                    }
                }

                if write_success {
                    register_received_file(
                        &state.app_handle,
                        target_path,
                        file_name,
                        content_type,
                        current_sender_id.clone(),
                        current_sender_name.clone(),
                        None,
                        None,
                    )
                    .await;
                    success = true;
                } else {
                    let _ = tokio::fs::remove_file(&target_path).await;
                }
            }
        }
    }

    if success {
        (StatusCode::OK, "Upload successful").into_response()
    } else {
        (StatusCode::INTERNAL_SERVER_ERROR, "Upload failed").into_response()
    }
}

pub async fn upload_chunk(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> axum::response::Response {
    let mut metadata: Option<ChunkMetadata> = None;
    let mut chunk_data: Option<Vec<u8>> = None;

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();

        if name == "metadata" {
            if let Ok(bytes) = field.bytes().await {
                if let Ok(text) = String::from_utf8(bytes.to_vec()) {
                    if let Ok(m) = serde_json::from_str(&text) {
                        metadata = Some(m);
                    }
                }
            }
        } else if name == "data" || name == "file" {
            if let Ok(bytes) = field.bytes().await {
                chunk_data = Some(bytes.to_vec());
            }
        }
    }

    let meta = match metadata {
        Some(m) => m,
        None => return (StatusCode::BAD_REQUEST, "Missing metadata").into_response(),
    };
    let data = match chunk_data {
        Some(d) => d,
        None => return (StatusCode::BAD_REQUEST, "Missing data").into_response(),
    };

    process_upload_chunk(state, meta, data).await
}

#[derive(serde::Deserialize)]
pub struct Base64ChunkPayload {
    metadata: ChunkMetadata,
    data_base64: String,
}

pub async fn upload_chunk_base64(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<Base64ChunkPayload>,
) -> axum::response::Response {
    let data =
        match base64::engine::general_purpose::STANDARD.decode(payload.data_base64.as_bytes()) {
            Ok(data) => data,
            Err(_) => return (StatusCode::BAD_REQUEST, "Invalid base64 chunk").into_response(),
        };
    process_upload_chunk(state, payload.metadata, data).await
}

pub(crate) async fn process_upload_chunk(
    state: Arc<AppState>,
    meta: ChunkMetadata,
    data: Vec<u8>,
) -> axum::response::Response {
    if crate::global_state::receiving_paused() {
        // The upload entry point for every sender, so one check here is what
        // 「暂停接收」 means: refusing a chunk cannot be talked around.
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            "This machine is pausing incoming transfers",
        )
            .into_response();
    }
    update_activity(&state.app_handle);
    if let Err(message) = validate_chunk(&meta, data.len()) {
        return (StatusCode::BAD_REQUEST, message).into_response();
    }
    if upload_already_completed(&meta.upload_id) {
        return (StatusCode::OK, "Upload complete").into_response();
    }
    let safe_name = safe_file_name(&meta.file_name);

    let sessions = state.app_handle.state::<UploadSessions>();
    let temp_path = {
        let mut sessions_map = sessions.0.lock().unwrap();
        prune_upload_sessions(&mut sessions_map);
        sessions_map
            .entry(meta.upload_id.clone())
            .or_insert_with(|| {
                let mut path = state
                    .app_handle
                    .path()
                    .download_dir()
                    .unwrap_or_else(|_| std::env::temp_dir());
                if let Ok(Some(custom)) = state
                    .app_handle
                    .state::<DbState>()
                    .settings_repo
                    .get("file_transfer_path")
                {
                    if !custom.trim().is_empty() {
                        path = std::path::PathBuf::from(custom);
                    }
                }
                if !path.exists() {
                    let _ = std::fs::create_dir_all(&path);
                }
                path.join(format!(".tmp_{}", meta.upload_id))
            })
            .clone()
    };

    // One request per upload at a time: the size check and the append below
    // must not interleave with a concurrent retry of the same chunk.
    let upload_lock = upload_lock(&meta.upload_id);
    let _upload_guard = upload_lock.lock().await;

    let mut options = tokio::fs::OpenOptions::new();
    let mut already_written = false;
    let required_space = if meta.chunk_index == 0 {
        meta.total_size
    } else {
        data.len() as u64
    };
    if ensure_disk_capacity(temp_path.parent().unwrap_or(&temp_path), required_space).is_err() {
        return (
            StatusCode::INSUFFICIENT_STORAGE,
            "Not enough available disk space",
        )
            .into_response();
    }
    options.create(true).write(true);
    if meta.chunk_index == 0 {
        options.truncate(true);
    } else {
        let expected_offset = meta.chunk_index as u64 * TRANSFER_CHUNK_SIZE as u64;
        let current_size = tokio::fs::metadata(&temp_path)
            .await
            .map(|metadata| metadata.len())
            .unwrap_or(0);
        // Written already, but the reply was lost and the phone retried.
        if current_size == expected_offset + data.len() as u64 {
            already_written = true;
        } else if current_size != expected_offset {
            return (StatusCode::CONFLICT, "Chunk received out of order").into_response();
        }
        options.append(true);
    }

    if already_written {
        // Nothing to write; fall through to finalize if this was the last chunk.
    } else if let Ok(mut file) = options.open(&temp_path).await {
        if let Err(e) = file.write_all(&data).await {
            eprintln!("Error writing chunk: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Write failed").into_response();
        }
    } else {
        return (StatusCode::INTERNAL_SERVER_ERROR, "Open failed").into_response();
    }

    if meta.chunk_index == meta.total_chunks - 1 {
        // A desktop sender hashes the whole file before it starts. If the bytes
        // we assembled don't match, the file is garbage — and it must never
        // reach the downloads folder to be opened from there. Phones send no
        // digest, so this check only guards machine-to-machine transfers.
        if let Some(expected) = checksum_of(&meta) {
            let actual = digest_file_hex(&temp_path).await.map(|d| d.to_lowercase());
            if actual.as_deref() != Some(expected.as_str()) {
                let _ = tokio::fs::remove_file(&temp_path).await;
                eprintln!(
                    "Checksum mismatch for {} (wanted {}, got {:?})",
                    meta.file_name, expected, actual
                );
                return (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "Checksum mismatch, file dropped",
                )
                    .into_response();
            }
        }
        let batch = meta.batch_metadata();
        let file_size = Some(meta.total_size);
        let final_path = match super::utils::reserve_unique_path(
            temp_path.parent().unwrap(),
            "",
            &safe_name,
        ) {
            Ok(path) => path,
            Err(e) => {
                eprintln!("Error finalizing file: {}", e);
                let _ = tokio::fs::remove_file(&temp_path).await;
                return (StatusCode::INTERNAL_SERVER_ERROR, "Finalize failed").into_response();
            }
        };

        if let Err(e) = tokio::fs::rename(&temp_path, &final_path).await {
            eprintln!("Error finalizing file: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Finalize failed").into_response();
        }

        {
            let mut sessions_map = sessions.0.lock().unwrap();
            sessions_map.remove(&meta.upload_id);
            remember_completed_upload(&meta.upload_id);
            release_upload_lock(&meta.upload_id);
        }

        let content_type = meta
            .content_type
            .unwrap_or_else(|| "application/octet-stream".to_string());
        let saved_path = final_path.to_string_lossy().to_string();
        register_received_file(
            &state.app_handle,
            final_path,
            safe_name.clone(),
            content_type,
            meta.sender_id.clone(),
            meta.sender_name.clone(),
            batch,
            file_size,
        )
        .await;

        // The received-file window needs the path it landed on; the name alone
        // can't be re-derived because a collision renames it.
        let _ = state.app_handle.emit(
            "file-transfer-received",
            serde_json::json!({
                "uploadId": meta.upload_id,
                "fileName": safe_name,
                "path": saved_path,
                "size": meta.total_size,
                "senderId": meta.sender_id,
                "senderName": meta.sender_name,
                "batchId": meta.batch_id,
                "batchName": meta.batch_name,
                "batchIndex": meta.batch_index,
                "batchTotal": meta.batch_total,
            }),
        );

        return (StatusCode::OK, "Upload complete").into_response();
    }

    let _ = state.app_handle.emit(
        "file-transfer-chunk-progress",
        serde_json::json!({
            "upload_id": meta.upload_id,
            "file_name": safe_name,
            "chunk_index": meta.chunk_index,
            "total_chunks": meta.total_chunks,
            "total_size": meta.total_size,
            "sender_id": meta.sender_id,
            "sender_name": meta.sender_name,
            "batch_id": meta.batch_id,
            "batch_name": meta.batch_name,
            "batch_index": meta.batch_index,
            "batch_total": meta.batch_total,
            "batch_size": meta.batch_size,
        }),
    );

    (StatusCode::OK, "Chunk received").into_response()
}

pub async fn share_chunk(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> axum::response::Response {
    update_activity(&state.app_handle);
    let mut metadata: Option<ChunkMetadata> = None;
    let mut chunk_data: Option<Vec<u8>> = None;

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();

        if name == "metadata" {
            if let Ok(bytes) = field.bytes().await {
                if let Ok(text) = String::from_utf8(bytes.to_vec()) {
                    if let Ok(m) = serde_json::from_str(&text) {
                        metadata = Some(m);
                    }
                }
            }
        } else if name == "data" || name == "file" {
            if let Ok(bytes) = field.bytes().await {
                chunk_data = Some(bytes.to_vec());
            }
        }
    }

    let meta = match metadata {
        Some(m) => m,
        None => return with_cors((StatusCode::BAD_REQUEST, "Missing metadata").into_response()),
    };
    let data = match chunk_data {
        Some(d) => d,
        None => return with_cors((StatusCode::BAD_REQUEST, "Missing data").into_response()),
    };
    if let Err(message) = validate_chunk(&meta, data.len()) {
        return with_cors((StatusCode::BAD_REQUEST, message).into_response());
    }
    if upload_already_completed(&meta.upload_id) {
        return with_cors((StatusCode::OK, "Upload complete").into_response());
    }
    let safe_name = safe_file_name(&meta.file_name);

    let sessions = state.app_handle.state::<UploadSessions>();
    let temp_path = {
        let mut sessions_map = sessions.0.lock().unwrap();
        prune_upload_sessions(&mut sessions_map);
        sessions_map
            .entry(meta.upload_id.clone())
            .or_insert_with(|| {
                let path = std::env::temp_dir().join("tiez_shared_uploads");
                if !path.exists() {
                    let _ = std::fs::create_dir_all(&path);
                }
                path.join(format!(".tmp_share_{}", meta.upload_id))
            })
            .clone()
    };

    // One request per upload at a time: the size check and the append below
    // must not interleave with a concurrent retry of the same chunk.
    let upload_lock = upload_lock(&meta.upload_id);
    let _upload_guard = upload_lock.lock().await;

    let mut options = tokio::fs::OpenOptions::new();
    let mut already_written = false;
    let required_space = if meta.chunk_index == 0 {
        meta.total_size
    } else {
        data.len() as u64
    };
    if ensure_disk_capacity(temp_path.parent().unwrap_or(&temp_path), required_space).is_err() {
        return with_cors(
            (
                StatusCode::INSUFFICIENT_STORAGE,
                "Not enough available disk space",
            )
                .into_response(),
        );
    }
    options.create(true).write(true);
    if meta.chunk_index == 0 {
        options.truncate(true);
    } else {
        let expected_offset = meta.chunk_index as u64 * TRANSFER_CHUNK_SIZE as u64;
        let current_size = tokio::fs::metadata(&temp_path)
            .await
            .map(|metadata| metadata.len())
            .unwrap_or(0);
        // Written already, but the reply was lost and the phone retried.
        if current_size == expected_offset + data.len() as u64 {
            already_written = true;
        } else if current_size != expected_offset {
            return with_cors(
                (StatusCode::CONFLICT, "Chunk received out of order").into_response(),
            );
        }
        options.append(true);
    }

    if already_written {
        // Nothing to write; fall through to finalize if this was the last chunk.
    } else if let Ok(mut file) = options.open(&temp_path).await {
        if let Err(e) = file.write_all(&data).await {
            eprintln!("Error writing shared chunk: {}", e);
            return with_cors((StatusCode::INTERNAL_SERVER_ERROR, "Write failed").into_response());
        }
    } else {
        return with_cors((StatusCode::INTERNAL_SERVER_ERROR, "Open failed").into_response());
    }

    if meta.chunk_index == meta.total_chunks.saturating_sub(1) {
        let batch = meta.batch_metadata();
        let file_size = Some(meta.total_size);
        let final_path = match super::utils::reserve_unique_path(
            temp_path.parent().unwrap(),
            "share_",
            &safe_name,
        ) {
            Ok(path) => path,
            Err(e) => {
                eprintln!("Error finalizing shared file: {}", e);
                let _ = tokio::fs::remove_file(&temp_path).await;
                return with_cors(
                    (StatusCode::INTERNAL_SERVER_ERROR, "Finalize failed").into_response(),
                );
            }
        };

        if let Err(e) = tokio::fs::rename(&temp_path, &final_path).await {
            eprintln!("Error finalizing shared file: {}", e);
            return with_cors(
                (StatusCode::INTERNAL_SERVER_ERROR, "Finalize failed").into_response(),
            );
        }

        {
            let mut sessions_map = sessions.0.lock().unwrap();
            sessions_map.remove(&meta.upload_id);
            remember_completed_upload(&meta.upload_id);
            release_upload_lock(&meta.upload_id);
        }

        let content_type = meta
            .content_type
            .unwrap_or_else(|| "application/octet-stream".to_string());
        let is_image = content_type.starts_with("image/");
        let is_video = content_type.starts_with("video/");
        let file_name_lower = safe_name.to_lowercase();
        let msg_type = if is_image {
            "image"
        } else if is_video
            || [".mp4", ".mov", ".mkv", ".avi", ".wmv", ".flv", ".webm"]
                .iter()
                .any(|ext| file_name_lower.ends_with(ext))
        {
            "video"
        } else {
            "file"
        };

        let shared_path = final_path.to_string_lossy().to_string();
        append_message_with_batch(
            &state.app_handle,
            "out",
            msg_type,
            &shared_path,
            "pc",
            "电脑",
            Some(&shared_path),
            batch.as_ref(),
            file_size,
        );

        return with_cors((StatusCode::OK, "Share upload complete").into_response());
    }

    with_cors((StatusCode::OK, "Shared chunk received").into_response())
}

pub async fn share_chunk_options() -> axum::response::Response {
    with_cors(StatusCode::NO_CONTENT.into_response())
}

#[derive(serde::Serialize)]
pub struct BatchDownloadResponse {
    url: String,
    name: String,
}

pub async fn prepare_batch_download(
    Path(batch_id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> axum::response::Response {
    update_activity(&state.app_handle);
    if batch_id.len() > 96
        || !batch_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return (StatusCode::BAD_REQUEST, "Invalid package id").into_response();
    }

    let (batch_name, files, expected_total) = {
        let chat_state = state.app_handle.state::<ChatState>();
        let messages = match chat_state.0.lock() {
            Ok(messages) => messages,
            Err(_) => {
                return (StatusCode::INTERNAL_SERVER_ERROR, "History unavailable").into_response()
            }
        };
        let matching = messages
            .iter()
            .filter(|message| message.batch_id.as_deref() == Some(batch_id.as_str()))
            .collect::<Vec<_>>();
        let expected_total = matching
            .iter()
            .filter_map(|message| message.batch_total)
            .max()
            .unwrap_or(0);
        let name = matching
            .iter()
            .find_map(|message| message.batch_name.clone())
            .unwrap_or_else(|| "TieZ 文件包".to_string());
        let files = matching
            .iter()
            .filter_map(|message| message.file_path.as_deref())
            .map(std::path::PathBuf::from)
            .filter(|path| path.is_file())
            .collect::<Vec<_>>();
        (name, files, expected_total)
    };

    if expected_total < 2 || files.len() != expected_total {
        return (
            StatusCode::CONFLICT,
            "Package is incomplete or its files are no longer available",
        )
            .into_response();
    }

    let total_size = files.iter().fold(0_u64, |total, path| {
        total.saturating_add(
            std::fs::metadata(path)
                .map(|metadata| metadata.len())
                .unwrap_or(0),
        )
    });
    let mut archive_dir = state
        .app_handle
        .path()
        .app_cache_dir()
        .unwrap_or_else(|_| std::env::temp_dir());
    archive_dir.push("file-transfer-packages");
    if std::fs::create_dir_all(&archive_dir).is_err()
        || ensure_disk_capacity(&archive_dir, total_size).is_err()
    {
        return (
            StatusCode::INSUFFICIENT_STORAGE,
            "Not enough space to prepare this package",
        )
            .into_response();
    }

    let safe_package_name = safe_file_name(&batch_name);
    let download_name = if safe_package_name.to_lowercase().ends_with(".zip") {
        safe_package_name
    } else {
        format!("{safe_package_name}.zip")
    };
    let archive_path = archive_dir.join(format!(
        ".tiez-batch-{batch_id}-{}.zip",
        uuid::Uuid::new_v4()
    ));
    let archive_path_for_job = archive_path.clone();
    let archive_result =
        tokio::task::spawn_blocking(move || create_batch_archive(&archive_path_for_job, &files))
            .await;

    if !matches!(archive_result, Ok(Ok(()))) {
        let _ = std::fs::remove_file(&archive_path);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Unable to create package",
        )
            .into_response();
    }

    let Some(token) = register_shared_file(
        &state.app_handle,
        archive_path.to_string_lossy().to_string(),
    ) else {
        let _ = std::fs::remove_file(&archive_path);
        return (StatusCode::INTERNAL_SERVER_ERROR, "Unable to share package").into_response();
    };

    let cleanup_path = archive_path.clone();
    tokio::spawn(async move {
        // Match the download token lifetime so large packages are not removed
        // while a slow LAN client is still preparing or downloading them.
        tokio::time::sleep(std::time::Duration::from_secs(8 * 60 * 60)).await;
        let _ = tokio::fs::remove_file(cleanup_path).await;
    });

    Json(BatchDownloadResponse {
        url: format!(
            "/download/{}?name={}",
            token,
            urlencoding::encode(&download_name)
        ),
        name: download_name,
    })
    .into_response()
}

/// A file's own modification time for its zip entry, so extracted files keep
/// their dates. Zip stores local time without a zone. None outside zip's
/// 1980-2107 range; the entry then gets the time of packing.
fn zip_modified_time(path: &std::path::Path) -> Option<zip::DateTime> {
    use chrono::{Datelike, Timelike};
    let modified = std::fs::metadata(path).ok()?.modified().ok()?;
    let local: chrono::DateTime<chrono::Local> = modified.into();
    zip::DateTime::from_date_and_time(
        u16::try_from(local.year()).ok()?,
        local.month() as u8,
        local.day() as u8,
        local.hour() as u8,
        local.minute() as u8,
        local.second() as u8,
    )
    .ok()
}

fn create_batch_archive(
    archive_path: &std::path::Path,
    files: &[std::path::PathBuf],
) -> Result<(), String> {
    use std::collections::HashSet;
    use std::io;
    use zip::write::SimpleFileOptions;

    let archive_file = std::fs::File::create(archive_path).map_err(|e| e.to_string())?;
    let mut writer = zip::ZipWriter::new(archive_file);
    let mut used_names = HashSet::new();

    for path in files {
        let original_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("file");
        let safe_name = unique_package_entry_name(&safe_file_name(original_name), &mut used_names);
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let already_compressed = [
            "zip", "rar", "7z", "gz", "bz2", "xz", "jpg", "jpeg", "png", "gif", "webp", "heic",
            "avif", "mp3", "aac", "m4a", "mp4", "mov", "mkv", "webm", "pdf",
        ]
        .contains(&extension.as_str());
        let method = if already_compressed {
            zip::CompressionMethod::Stored
        } else {
            zip::CompressionMethod::Deflated
        };
        let mut options = SimpleFileOptions::default().compression_method(method);
        if let Some(modified) = zip_modified_time(path) {
            options = options.last_modified_time(modified);
        }
        writer
            .start_file(safe_name, options)
            .map_err(|e| e.to_string())?;
        let mut source = std::fs::File::open(path).map_err(|e| e.to_string())?;
        io::copy(&mut source, &mut writer).map_err(|e| e.to_string())?;
    }

    writer.finish().map_err(|e| e.to_string())?;
    Ok(())
}

fn unique_package_entry_name(
    requested: &str,
    used_names: &mut std::collections::HashSet<String>,
) -> String {
    let requested = if requested.trim().is_empty() {
        "file"
    } else {
        requested
    };
    if used_names.insert(requested.to_string()) {
        return requested.to_string();
    }

    let path = std::path::Path::new(requested);
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("file");
    let extension = path.extension().and_then(|value| value.to_str());
    for suffix in 2..=10_000 {
        let candidate = match extension {
            Some(extension) => format!("{stem} ({suffix}).{extension}"),
            None => format!("{stem} ({suffix})"),
        };
        if used_names.insert(candidate.clone()) {
            return candidate;
        }
    }
    format!("{}-{}", uuid::Uuid::new_v4(), requested)
}

pub async fn handle_file_download_proxy(
    Path(token): Path<String>,
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let app_handle = &state.app_handle;
    update_activity(app_handle);
    let shared_state = app_handle.state::<SharedFileState>();

    let file_path = shared_state.0.lock().ok().and_then(|mut guard| {
        guard.retain(|_, entry| !entry.is_expired());
        guard.get(&token).map(|entry| entry.path.clone())
    });

    if let Some(path_str) = file_path {
        let path = std::path::PathBuf::from(&path_str);
        if path.exists() {
            let filename = params
                .get("name")
                .map(|name| safe_file_name(name))
                .filter(|name| !name.trim().is_empty())
                .unwrap_or_else(|| {
                    path.file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string()
                });
            let disposition_filename: String = filename
                .chars()
                .filter(|ch| !ch.is_control())
                .map(|ch| if ch == '"' || ch == '\\' { '_' } else { ch })
                .collect();
            let mime = mime_guess::from_path(&path)
                .first_or_octet_stream()
                .to_string();
            // SVG can carry script, so it is always a download, never inline.
            let is_image = mime.starts_with("image/") && mime != "image/svg+xml";
            let is_video = mime.starts_with("video/");
            let encoded_name = urlencoding::encode(&filename);
            let disposition = if is_image || is_video {
                format!(
                    "inline; filename=\"{}\"; filename*=UTF-8''{}",
                    disposition_filename, encoded_name
                )
            } else {
                format!(
                    "attachment; filename=\"{}\"; filename*=UTF-8''{}",
                    disposition_filename, encoded_name
                )
            };

            if let Ok(mut file) = File::open(&path).await {
                let metadata = match file.metadata().await {
                    Ok(m) => m,
                    Err(_) => {
                        return (StatusCode::INTERNAL_SERVER_ERROR, "Metadata failed")
                            .into_response()
                    }
                };
                let total_size = metadata.len();
                if let Some(range) = headers.get(header::RANGE).and_then(|h| h.to_str().ok()) {
                    let Some((start, end)) = parse_single_byte_range(range, total_size) else {
                        return (
                            StatusCode::RANGE_NOT_SATISFIABLE,
                            [(header::CONTENT_RANGE, format!("bytes */{}", total_size))],
                            Body::empty(),
                        )
                            .into_response();
                    };
                    let content_length = end - start + 1;

                    if file.seek(SeekFrom::Start(start)).await.is_ok() {
                        let stream =
                            ReaderStream::with_capacity(file.take(content_length), 64 * 1024);
                        let body = Body::from_stream(stream);

                        return (
                            StatusCode::PARTIAL_CONTENT,
                            [
                                (header::CONTENT_TYPE, mime),
                                (header::CONTENT_DISPOSITION, disposition),
                                (header::ACCEPT_RANGES, "bytes".to_string()),
                                (
                                    header::CONTENT_RANGE,
                                    format!("bytes {}-{}/{}", start, end, total_size),
                                ),
                                (header::CONTENT_LENGTH, content_length.to_string()),
                            ],
                            body,
                        )
                            .into_response();
                    }
                }

                let stream = ReaderStream::with_capacity(file, 64 * 1024);
                let body = Body::from_stream(stream);

                return (
                    StatusCode::OK,
                    [
                        (header::CONTENT_TYPE, mime),
                        (header::CONTENT_DISPOSITION, disposition),
                        (header::ACCEPT_RANGES, "bytes".to_string()),
                        (header::CONTENT_LENGTH, total_size.to_string()),
                    ],
                    body,
                )
                    .into_response();
            }
        }
    }

    (StatusCode::NOT_FOUND, "File not found").into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_retried_last_chunk_finds_its_upload_done() {
        assert!(!upload_already_completed("upload-retry-test"));
        remember_completed_upload("upload-retry-test");
        assert!(upload_already_completed("upload-retry-test"));
    }

    #[test]
    fn only_the_app_window_may_call_cross_origin() {
        assert!(is_app_origin("tauri://localhost"));
        assert!(is_app_origin("http://tauri.localhost"));
        assert!(!is_app_origin("http://evil.example"));
        assert!(!is_app_origin("null"));
        assert!(!is_app_origin("tauri://localhost.evil"));
    }

    #[test]
    fn file_names_cannot_escape_the_receive_directory() {
        assert_eq!(safe_file_name("../../private/secret.txt"), "secret.txt");
        assert_eq!(safe_file_name(r"..\..\private\secret.txt"), "secret.txt");
        assert_eq!(safe_file_name("../.."), "unnamed-file");
        assert_eq!(safe_file_name("a:b?.txt"), "a_b_.txt");
        assert_eq!(safe_file_name("x\u{202E}gpj.exe"), "xgpj.exe");
        assert_eq!(safe_file_name("con.txt"), "_con.txt");
        assert_eq!(safe_file_name("COM1"), "_COM1");
        assert_eq!(safe_file_name("a€.txt"), "a€.txt");
    }

    #[test]
    fn upload_ids_are_path_safe() {
        assert!(safe_upload_id("mobile_123-abc").is_some());
        assert!(safe_upload_id("../../escape").is_none());
        assert!(safe_upload_id("").is_none());
    }

    #[test]
    fn request_tokens_support_qr_cookie_and_desktop_header() {
        let query_request = Request::builder()
            .uri("/?auth=session-123")
            .body(Body::empty())
            .unwrap();
        assert_eq!(
            request_token(&query_request).as_deref(),
            Some("session-123")
        );

        let cookie_request = Request::builder()
            .header(header::COOKIE, "other=x; tiez_ft_session=session-456")
            .body(Body::empty())
            .unwrap();
        assert_eq!(
            request_token(&cookie_request).as_deref(),
            Some("session-456")
        );

        let header_request = Request::builder()
            .header(header::AUTHORIZATION, "Bearer session-789")
            .body(Body::empty())
            .unwrap();
        assert_eq!(
            request_token(&header_request).as_deref(),
            Some("session-789")
        );
    }

    #[test]
    fn a_promised_checksum_must_actually_be_one() {
        let claimed = |sha256: Option<&str>| {
            checksum_of(&ChunkMetadata {
                upload_id: "u".to_string(),
                chunk_index: 0,
                total_chunks: 1,
                file_name: "f".to_string(),
                sender_id: "s".to_string(),
                sender_name: "s".to_string(),
                total_size: 1,
                content_type: None,
                batch_id: None,
                batch_name: None,
                batch_index: None,
                batch_total: None,
                batch_size: None,
                sha256: sha256.map(str::to_string),
            })
        };
        // A phone sends no digest, so its files are written as they arrive.
        assert_eq!(claimed(None), None);
        assert_eq!(claimed(Some("")), None);
        assert_eq!(claimed(Some("not-hex-at-all")), None);
        assert_eq!(claimed(Some(&"a".repeat(63))), None);
        assert_eq!(claimed(Some(&"A".repeat(64))), Some("a".repeat(64)));
    }

    #[test]
    fn chunk_metadata_must_match_the_real_payload() {
        let metadata = ChunkMetadata {
            upload_id: "upload-1".to_string(),
            chunk_index: 0,
            total_chunks: 1,
            file_name: "small.txt".to_string(),
            sender_id: "mobile".to_string(),
            sender_name: "phone".to_string(),
            total_size: 4,
            content_type: Some("text/plain".to_string()),
            batch_id: None,
            batch_name: None,
            batch_index: None,
            batch_total: None,
            batch_size: None,
            sha256: None,
        };
        assert!(validate_chunk(&metadata, 4).is_ok());
        assert!(validate_chunk(&metadata, 3).is_err());

        let large_size = 25_u64 * 1024 * 1024 * 1024;
        let large_metadata = ChunkMetadata {
            upload_id: "large-upload".to_string(),
            chunk_index: 0,
            total_chunks: (large_size / TRANSFER_CHUNK_SIZE as u64) as usize,
            file_name: "large.bin".to_string(),
            sender_id: "mobile".to_string(),
            sender_name: "phone".to_string(),
            total_size: large_size,
            content_type: Some("application/octet-stream".to_string()),
            batch_id: None,
            batch_name: None,
            batch_index: None,
            batch_total: None,
            batch_size: None,
            sha256: None,
        };
        assert!(validate_chunk(&large_metadata, TRANSFER_CHUNK_SIZE).is_ok());
    }

    #[test]
    fn batch_archives_stream_files_and_preserve_duplicate_names() {
        let root = std::env::temp_dir().join(format!(
            "tiez-batch-test-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        let first_dir = root.join("first");
        let second_dir = root.join("second");
        std::fs::create_dir_all(&first_dir).unwrap();
        std::fs::create_dir_all(&second_dir).unwrap();
        let first = first_dir.join("same.txt");
        let second = second_dir.join("same.txt");
        std::fs::write(&first, b"first").unwrap();
        std::fs::write(&second, b"second").unwrap();
        let archive = root.join("package.zip");

        create_batch_archive(&archive, &[first, second]).unwrap();

        let file = std::fs::File::open(&archive).unwrap();
        let mut zip = zip::ZipArchive::new(file).unwrap();
        assert_eq!(zip.len(), 2);
        assert_eq!(zip.by_index(0).unwrap().name(), "same.txt");
        assert_eq!(zip.by_index(1).unwrap().name(), "same (2).txt");
        // Entries carry the files' own dates, not zip's 1980-01-01 default.
        let current_year = chrono::Datelike::year(&chrono::Local::now()) as u16;
        let stamped = zip.by_index(0).unwrap().last_modified().expect("entry time");
        assert_eq!(stamped.year(), current_year);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn byte_ranges_handle_open_ended_suffix_and_invalid_inputs() {
        assert_eq!(parse_single_byte_range("bytes=10-19", 100), Some((10, 19)));
        assert_eq!(parse_single_byte_range("bytes=90-", 100), Some((90, 99)));
        assert_eq!(parse_single_byte_range("bytes=-10", 100), Some((90, 99)));
        assert_eq!(parse_single_byte_range("bytes=-200", 100), Some((0, 99)));
        assert_eq!(parse_single_byte_range("bytes=90-200", 100), Some((90, 99)));
        assert_eq!(parse_single_byte_range("bytes=20-10", 100), None);
        assert_eq!(parse_single_byte_range("bytes=100-", 100), None);
        assert_eq!(parse_single_byte_range("bytes=0-1,4-5", 100), None);
        assert_eq!(parse_single_byte_range("bytes=0-", 0), None);
    }
}

static UPLOAD_LOCKS: std::sync::LazyLock<
    std::sync::Mutex<HashMap<String, std::sync::Arc<tokio::sync::Mutex<()>>>>,
> = std::sync::LazyLock::new(Default::default);

fn upload_lock(upload_id: &str) -> std::sync::Arc<tokio::sync::Mutex<()>> {
    UPLOAD_LOCKS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .entry(upload_id.to_string())
        .or_default()
        .clone()
}

fn release_upload_lock(upload_id: &str) {
    UPLOAD_LOCKS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .remove(upload_id);
}

/// Uploads finished in the last few minutes. A phone that lost the reply to
/// the last chunk sends it again; answer that it's done instead of saving the
/// file a second time (one chunk) or reporting a failure (several).
static COMPLETED_UPLOADS: std::sync::LazyLock<
    std::sync::Mutex<HashMap<String, std::time::Instant>>,
> = std::sync::LazyLock::new(Default::default);
const COMPLETED_UPLOAD_MEMORY: std::time::Duration = std::time::Duration::from_secs(10 * 60);

fn remember_completed_upload(upload_id: &str) {
    let mut completed = COMPLETED_UPLOADS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    completed.retain(|_, at| at.elapsed() < COMPLETED_UPLOAD_MEMORY);
    completed.insert(upload_id.to_string(), std::time::Instant::now());
}

fn upload_already_completed(upload_id: &str) -> bool {
    COMPLETED_UPLOADS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .get(upload_id)
        .is_some_and(|at| at.elapsed() < COMPLETED_UPLOAD_MEMORY)
}

// ---- Sealed routes: the same operations with encrypted bodies (sealed.rs) ----

fn sealed_body(key: &sealed::LanKey, kind: &str, plain: &[u8]) -> Response {
    match sealed::seal(key, kind, plain) {
        Some(body) => (
            [
                (header::CONTENT_TYPE, "application/octet-stream"),
                (header::CACHE_CONTROL, "no-store"),
            ],
            body,
        )
            .into_response(),
        None => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

fn open_body(key: &sealed::LanKey, kind: &str, body: &[u8]) -> Result<Vec<u8>, Response> {
    sealed::open(key, kind, body)
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "Unreadable sealed body").into_response())
}

pub async fn sealed_poll(
    Query(params): Query<HashMap<String, String>>,
    State(state): State<Arc<AppState>>,
) -> Response {
    let messages = collect_messages(&state, last_id_param(&params));
    let json = serde_json::to_vec(&messages).unwrap_or_else(|_| b"[]".to_vec());
    sealed_body(&state.lan_key, "poll", &json)
}

pub async fn sealed_text(State(state): State<Arc<AppState>>, body: axum::body::Bytes) -> Response {
    let plain = match open_body(&state.lan_key, "text", &body) {
        Ok(plain) => plain,
        Err(response) => return response,
    };
    match serde_json::from_slice::<ReceiveText>(&plain) {
        Ok(payload) => receive_text(state, payload).await,
        Err(_) => (StatusCode::BAD_REQUEST, "Invalid text payload").into_response(),
    }
}

pub async fn sealed_upload_chunk(
    State(state): State<Arc<AppState>>,
    body: axum::body::Bytes,
) -> Response {
    let plain = match open_body(&state.lan_key, "upload", &body) {
        Ok(plain) => plain,
        Err(response) => return response,
    };
    let Some((meta, data)) = sealed::split_upload(&plain) else {
        return (StatusCode::BAD_REQUEST, "Invalid upload payload").into_response();
    };
    match serde_json::from_slice::<ChunkMetadata>(meta) {
        Ok(meta) => process_upload_chunk(state, meta, data.to_vec()).await,
        Err(_) => (StatusCode::BAD_REQUEST, "Invalid chunk metadata").into_response(),
    }
}

/// `prepare_batch_download`'s answer, sealed: its link is all anyone needs
/// to fetch the package.
pub async fn sealed_batch_download(
    Path(batch_id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Response {
    let key = state.lan_key;
    let response = prepare_batch_download(Path(batch_id), State(state)).await;
    if !response.status().is_success() {
        return response;
    }
    match axum::body::to_bytes(response.into_body(), 64 * 1024).await {
        Ok(json) => sealed_body(&key, "batch", &json),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Unable to share package").into_response(),
    }
}

/// The file as `u32 BE length || box` frames of `sealed::DOWNLOAD_CHUNK`
/// plaintext bytes. No Range support and no file name in the URL: the client
/// keeps the name from the (sealed) message that linked here.
pub async fn sealed_download(
    Path(token): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Response {
    update_activity(&state.app_handle);
    let shared_state = state.app_handle.state::<SharedFileState>();
    let file_path = shared_state.0.lock().ok().and_then(|mut guard| {
        guard.retain(|_, entry| !entry.is_expired());
        guard.get(&token).map(|entry| entry.path.clone())
    });
    let Some(path) = file_path else {
        return (StatusCode::NOT_FOUND, "File not found").into_response();
    };
    let Ok(file) = File::open(&path).await else {
        return (StatusCode::NOT_FOUND, "File not found").into_response();
    };
    let total = match file.metadata().await {
        Ok(meta) => meta.len(),
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Metadata failed").into_response(),
    };
    let chunk = sealed::DOWNLOAD_CHUNK as u64;
    let frames = total.div_ceil(chunk).max(1);
    let key = state.lan_key;

    let stream = futures::stream::unfold((file, 0u64), move |(mut file, index)| async move {
        if index >= frames {
            return None;
        }
        let mut buf = vec![0u8; sealed::DOWNLOAD_CHUNK];
        let mut filled = 0;
        while filled < buf.len() {
            match file.read(&mut buf[filled..]).await {
                Ok(0) => break,
                Ok(n) => filled += n,
                Err(err) => return Some((Err(err), (file, frames))),
            }
        }
        buf.truncate(filled);
        let last = index + 1 == frames;
        let Some(frame) = sealed::download_frame(&key, index, last, &buf) else {
            return Some((Err(std::io::Error::other("seal failed")), (file, frames)));
        };
        Some((Ok(axum::body::Bytes::from(frame)), (file, index + 1)))
    });

    (
        [
            (header::CONTENT_TYPE, "application/octet-stream".to_string()),
            (header::CACHE_CONTROL, "no-store".to_string()),
            (header::HeaderName::from_static("x-tiez-size"), total.to_string()),
        ],
        Body::from_stream(stream),
    )
        .into_response()
}
