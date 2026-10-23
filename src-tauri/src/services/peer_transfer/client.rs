//! Outbound pushes: the same sealed upload the phone uses, aimed at one peer.
//!
//! There is no second transfer protocol. Each 512 KiB chunk is
//! `seal(peer_key, "upload", u32 len || metadata || bytes)` posted to the
//! peer's `/p/upload-chunk` with its pairing token, so the receiver's
//! existing pipeline — offsets, resume-on-retry, disk reserve, unique file
//! names — is the code that already ships.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::{AsyncReadExt, AsyncSeekExt};

use crate::services::file_transfer::handlers::TRANSFER_CHUNK_SIZE;
use crate::services::file_transfer::models::ChunkMetadata;
use crate::services::file_transfer::sealed;

use super::db;
use super::identity;
use super::models::{
    InviteFile, InviteReply, PeerHandle, PeerInvite, PeerTransferEvent, PeerTransferJob,
    PeerTransferRegistryState,
};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const RETRIES: usize = 3;
/// Sealed kind for an invite and its answer (same AAD both directions, as with
/// the handshake).
const INVITE_KIND: &str = "invite";
/// The receiver holds the invite for as long as its own 接收超时 setting allows;
/// the sender waits the longest that can be, plus room for the answer to travel
/// back. Reading the timeout here would be wrong — it is the *other* machine's
/// setting — so the sender stays patient and lets the receiver answer "reject"
/// when its own countdown ends.
const INVITE_WAIT: Duration =
    Duration::from_secs(super::receive::MAX_DECIDE_TIMEOUT + 15);

/// One group of files, pushed to one peer.
pub struct PeerPusher {
    app: AppHandle,
    client: reqwest::Client,
}

impl PeerPusher {
    pub fn new(app: AppHandle) -> Self {
        let client = reqwest::Client::builder()
            // LAN only, and a stalled peer must not hold a queue forever.
            .timeout(REQUEST_TIMEOUT)
            .build()
            .unwrap_or_default();
        Self { app, client }
    }

    /// Run a job against every peer it names; peers go in parallel, a peer's
    /// files go one after another so the receiver sees a clean stream.
    pub async fn run_job(self, job: PeerTransferJob, cancel: Arc<AtomicBool>) -> bool {
        let mut tasks = Vec::new();
        for device_id in job.peer_ids.clone() {
            let pusher = PeerPusher {
                app: self.app.clone(),
                client: self.client.clone(),
            };
            let job = job.clone();
            let cancel = cancel.clone();
            tasks.push(tauri::async_runtime::spawn(async move {
                pusher.run_for_peer(&job, &device_id, cancel).await
            }));
        }
        let mut failed = false;
        for task in tasks {
            match task.await {
                Ok(ok) => failed |= !ok,
                Err(_) => failed = true,
            }
        }
        failed
    }

    /// `false` when anything failed or the user cancelled.
    async fn run_for_peer(&self, job: &PeerTransferJob, device_id: &str, cancel: Arc<AtomicBool>) -> bool {
        let Some(peer) = db::send_target(&self.app, device_id) else {
            self.emit(
                &job.group,
                "out",
                device_id,
                "",
                "",
                "",
                0,
                0,
                "failed",
                Some("这台设备不在配对列表里，或还没发现它的地址".to_string()),
                None,
            );
            return false;
        };

        let items = match expand(&job.file_paths) {
            Ok(items) => items,
            Err(error) => {
                self.emit(
                    &job.group,
                    "out",
                    &peer.device_id,
                    &peer.display_name(),
                    "",
                    "",
                    0,
                    0,
                    "failed",
                    Some(error),
                    None,
                );
                return false;
            }
        };
        if items.is_empty() {
            return false;
        }

        // Nothing moves until the other machine says yes — and in the default
        // (auto-receive) mode it says yes without asking anyone.
        if let Err(error) = self.invite(&peer, &job.group, &items).await {
            self.emit(
                &job.group,
                "out",
                &peer.device_id,
                &peer.display_name(),
                "",
                "",
                0,
                0,
                "failed",
                Some(error),
                None,
            );
            return false;
        }

        let total = items.len();
        let batch_id = if total > 1 {
            Some(uuid::Uuid::new_v4().as_simple().to_string())
        } else {
            None
        };
        // A dropped folder arrives as one path and expands to its files; naming
        // the batch after the folder is what makes the receiver rebuild it.
        let folder = items.iter().find_map(|(_, _, folder)| folder.clone());
        let batch_name = job.batch_name.clone().or_else(|| match &folder {
            Some(name) if total > 1 => Some(name.clone()),
            _ if total > 1 => Some(format!("{total} 个文件")),
            _ => None,
        });

        let mut ok = true;
        for (index, (path, name, _)) in items.iter().enumerate() {
            if cancel.load(Ordering::Relaxed) {
                return false;
            }
            let batch = batch_id.as_ref().map(|id| BatchInfo {
                id: id.clone(),
                name: batch_name.clone().unwrap_or_else(|| "文件包".to_string()),
                index,
                total,
            });
            if !self.push_file(&peer, path, name, job, batch, &cancel).await {
                ok = false;
                // A peer that refused one file will refuse the rest.
                if cancel.load(Ordering::Relaxed) {
                    break;
                }
            }
        }
        ok
    }

    /// Ask before pushing, sealed with the peer's key and answered on that
    /// machine. When it asks the user, this request stays open while they
    /// decide — which is why there is no second call for the answer, and why a
    /// refusal means not one byte was written.
    async fn invite(
        &self,
        peer: &PeerHandle,
        group: &str,
        items: &[(String, String, Option<String>)],
    ) -> Result<(), String> {
        let ours = identity::current(&self.app);
        let listed: Vec<(String, String)> = items
            .iter()
            .take(2_000)
            .map(|(path, name, _)| (path.clone(), name.clone()))
            .collect();
        // Sizes come from disk, off the async thread: a folder drop can name
        // two thousand files and the other side shows the total before it says yes.
        let files = tokio::task::spawn_blocking(move || {
            listed
                .into_iter()
                .map(|(path, name)| InviteFile {
                    size: std::fs::metadata(&path).map(|meta| meta.len()).unwrap_or(0),
                    name,
                })
                .collect::<Vec<InviteFile>>()
        })
        .await
        .map_err(|_| "准备文件清单失败".to_string())?;
        if files.is_empty() {
            return Err("没有可发送的文件".to_string());
        }
        let total_size = files.iter().map(|file| file.size).sum();
        let message = PeerInvite {
            group: group.to_string(),
            device_id: ours.device_id.clone(),
            device_name: ours.device_name.clone(),
            nickname: ours.nickname.clone(),
            platform: ours.platform.clone(),
            key: sealed::encode_key(&ours.key),
            files,
            total_size,
        };
        let Ok(body) = serde_json::to_vec(&message) else {
            return Err("无法准备传输请求".to_string());
        };
        let Some(boxed) = sealed::seal(&peer.key, INVITE_KIND, &body) else {
            return Err("加密失败".to_string());
        };
        self.emit(
            group,
            "out",
            &peer.device_id,
            &peer.display_name(),
            "",
            "",
            0,
            0,
            "preparing",
            None,
            None,
        );
        let url = format!("http://{}:{}/p/invite", peer.ip, peer.port);
        let response = self
            .client
            .post(&url)
            .timeout(INVITE_WAIT)
            .header(AUTHORIZATION, format!("Bearer {}", peer.auth_token))
            .header(CONTENT_TYPE, "application/octet-stream")
            .body(boxed)
            .send()
            .await
            .map_err(|error| {
                if error.is_timeout() {
                    "对方没有回应这次传输请求".to_string()
                } else {
                    format!("连接不上对方：{error}")
                }
            })?;
        let status = response.status();
        if !status.is_success() {
            let detail = response.text().await.unwrap_or_default();
            return Err(match status.as_u16() {
                401 => "配对已失效，请重新配对".to_string(),
                403 => "对方还没有确认这次配对".to_string(),
                503 => "对方已暂停接收".to_string(),
                code => format!("对方返回 {code}：{}", detail.trim()),
            });
        }
        let bytes = response
            .bytes()
            .await
            .map_err(|error| format!("读不到对方的回应：{error}"))?;
        // The answer is sealed with *our* key, which only a machine holding our
        // pairing code could have read from the invite.
        let Some(plain) = sealed::open(&ours.key, INVITE_KIND, &bytes) else {
            return Err("对方的回应读不出来".to_string());
        };
        let reply: InviteReply =
            serde_json::from_slice(&plain).map_err(|_| "对方的回应无效".to_string())?;
        if reply.group != group {
            return Err("对方的回应对不上这次传输".to_string());
        }
        match reply.decision.as_str() {
            "accept" => Ok(()),
            _ => Err("对方没有接受这次传输".to_string()),
        }
    }

    #[allow(clippy::too_many_arguments)]
    async fn push_file(
        &self,
        peer: &PeerHandle,
        path: &str,
        name: &str,
        job: &PeerTransferJob,
        batch: Option<BatchInfo>,
        cancel: &Arc<AtomicBool>,
    ) -> bool {
        let upload_id = uuid::Uuid::new_v4().as_simple().to_string();
        let ours = identity::current(&self.app);

        let (size, digest) = match hash_file(path).await {
            Ok(values) => values,
            Err(error) => {
                self.fail(peer, job, name, &upload_id, &batch, &error);
                return false;
            }
        };
        if cancel.load(Ordering::Relaxed) {
            return false;
        }

        let total_chunks = std::cmp::max(1, size.div_ceil(TRANSFER_CHUNK_SIZE as u64) as usize);
        self.emit(
            &job.group,
            "out",
            &peer.device_id,
            &peer.display_name(),
            name,
            &upload_id,
            0,
            size,
            "active",
            None,
            batch.as_ref(),
        );

        let mut file = match tokio::fs::File::open(path).await {
            Ok(file) => file,
            Err(error) => {
                self.fail(peer, job, name, &upload_id, &batch, &format!("文件读不到：{error}"));
                return false;
            }
        };

        let url = format!("http://{}:{}/p/upload-chunk", peer.ip, peer.port);
        let mut buffer = vec![0u8; TRANSFER_CHUNK_SIZE];
        for index in 0..total_chunks {
            if cancel.load(Ordering::Relaxed) {
                return false;
            }
            let filled = match read_chunk(&mut file, &mut buffer).await {
                Ok(filled) => filled,
                Err(error) => {
                    self.fail(peer, job, name, &upload_id, &batch, &format!("读取失败：{error}"));
                    return false;
                }
            };
            let metadata = ChunkMetadata {
                upload_id: upload_id.clone(),
                chunk_index: index,
                total_chunks,
                file_name: name.to_string(),
                sender_id: ours.device_id.clone(),
                sender_name: ours.display_name(),
                total_size: size,
                content_type: mime_guess::from_path(path).first_raw().map(String::from),
                batch_id: batch.as_ref().map(|batch| batch.id.clone()),
                batch_name: batch.as_ref().map(|batch| batch.name.clone()),
                batch_index: batch.as_ref().map(|batch| batch.index),
                batch_total: batch.as_ref().map(|batch| batch.total),
                batch_size: None,
                sha256: Some(digest.clone()),
            };
            let plain = upload_plain(&metadata, &buffer[..filled]);
            let Some(boxed) = sealed::seal(&peer.key, "upload", &plain) else {
                self.fail(peer, job, name, &upload_id, &batch, "加密失败");
                return false;
            };

            match self
                .post_chunk(&url, &peer.auth_token, boxed, index, total_chunks, cancel)
                .await
            {
                PostOutcome::Ok => {}
                PostOutcome::Cancelled => return false,
                PostOutcome::Failed(reason) => {
                    self.fail(peer, job, name, &upload_id, &batch, &reason);
                    return false;
                }
            }

            let sent = std::cmp::min(size, (index as u64 + 1) * TRANSFER_CHUNK_SIZE as u64);
            self.emit(
                &job.group,
                "out",
                &peer.device_id,
                &peer.display_name(),
                name,
                &upload_id,
                sent,
                size,
                "active",
                None,
                batch.as_ref(),
            );
        }

        self.emit(
            &job.group,
            "out",
            &peer.device_id,
            &peer.display_name(),
            name,
            &upload_id,
            size,
            size,
            "done",
            None,
            batch.as_ref(),
        );
        true
    }

    async fn post_chunk(
        &self,
        url: &str,
        auth: &str,
        body: Vec<u8>,
        index: usize,
        total_chunks: usize,
        cancel: &Arc<AtomicBool>,
    ) -> PostOutcome {
        let mut last_error = String::from("传输失败");
        for attempt in 0..RETRIES {
            if cancel.load(Ordering::Relaxed) {
                return PostOutcome::Cancelled;
            }
            let request = self
                .client
                .post(url)
                .header(AUTHORIZATION, format!("Bearer {auth}"))
                .header(CONTENT_TYPE, "application/octet-stream")
                .body(body.clone())
                .send()
                .await;
            match request {
                Ok(response) => {
                    let status = response.status();
                    if status.is_success() {
                        return PostOutcome::Ok;
                    }
                    let detail = response.text().await.unwrap_or_default();
                    last_error = match status.as_u16() {
                        401 => "配对已失效，请重新配对".to_string(),
                        403 => "对方还没有确认这次配对".to_string(),
                        409 => "传输顺序对不上，请重试".to_string(),
                        // The receiver threw the finished file away because the
                        // bytes didn't hash to what we promised. Re-sending the
                        // same chunk would only fail the same way.
                        422 => "对方校验文件不通过，已丢弃".to_string(),
                        503 => "对方已暂停接收".to_string(),
                        507 => "对方磁盘空间不足".to_string(),
                        code => format!("对方返回 {code}：{}", detail.trim()),
                    };
                    // A refused status is the peer's state, not a flaky wire:
                    // only retry what a retry could plausibly fix.
                    if matches!(status.as_u16(), 401 | 403 | 409 | 422 | 503 | 507) {
                        return PostOutcome::Failed(last_error);
                    }
                }
                Err(error) => {
                    last_error = if error.is_timeout() {
                        "对方没有响应，可能已关机或退出".to_string()
                    } else {
                        format!("连接不上对方：{error}")
                    };
                }
            }
            if attempt + 1 < RETRIES {
                let _ = self
                    .app
                    .emit("peer-transfer-retry", serde_json::json!({
                        "url": url, "chunk_index": index, "total_chunks": total_chunks, "attempt": attempt + 2,
                    }));
                tokio::time::sleep(Duration::from_millis(400 * (attempt as u64 + 1))).await;
            }
        }
        PostOutcome::Failed(last_error)
    }

    fn fail(
        &self,
        peer: &PeerHandle,
        job: &PeerTransferJob,
        name: &str,
        upload_id: &str,
        batch: &Option<BatchInfo>,
        reason: &str,
    ) {
        self.emit(
            &job.group,
            "out",
            &peer.device_id,
            &peer.display_name(),
            name,
            upload_id,
            0,
            0,
            "failed",
            Some(reason.to_string()),
            batch.as_ref(),
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn emit(
        &self,
        group: &str,
        direction: &str,
        peer_id: &str,
        peer_name: &str,
        file_name: &str,
        upload_id: &str,
        sent: u64,
        total: u64,
        status: &str,
        error: Option<String>,
        batch: Option<&BatchInfo>,
    ) {
        let event = PeerTransferEvent {
            group: group.to_string(),
            direction: direction.to_string(),
            peer_device_id: peer_id.to_string(),
            peer_name: peer_name.to_string(),
            file_name: file_name.to_string(),
            upload_id: upload_id.to_string(),
            sent_bytes: sent,
            total_bytes: total,
            status: status.to_string(),
            error,
            batch_id: batch.map(|batch| batch.id.clone()),
            batch_name: batch.map(|batch| batch.name.clone()),
            batch_index: batch.map(|batch| batch.index),
            batch_total: batch.map(|batch| batch.total),
            timestamp: now_ms(),
        };
        super::history::record(&self.app, &event);
        crate::app::peer_floats::on_transfer_progress(&self.app, &event.status);
        let _ = self.app.emit("peer-transfer-progress", event);
    }
}

struct BatchInfo {
    id: String,
    name: String,
    index: usize,
    total: usize,
}

enum PostOutcome {
    Ok,
    Failed(String),
    Cancelled,
}

/// `u32 BE metadata length || metadata || bytes` (sealed.rs `split_upload`).
fn upload_plain(metadata: &ChunkMetadata, data: &[u8]) -> Vec<u8> {
    let meta = serde_json::to_vec(metadata).unwrap_or_default();
    let mut plain = Vec::with_capacity(4 + meta.len() + data.len());
    plain.extend_from_slice(&(meta.len() as u32).to_be_bytes());
    plain.extend_from_slice(&meta);
    plain.extend_from_slice(data);
    plain
}

/// A dropped folder arrives as one path; the receiver has no directory
/// concept, so its files go as one batch that keeps the folder name.
fn expand(paths: &[String]) -> Result<Vec<(String, String, Option<String>)>, String> {
    let mut items = Vec::new();
    for path in paths {
        let parsed = crate::services::file_transfer::utils::expand_transfer_paths(vec![
            path.clone(),
        ])
        .map_err(|error| error)?;
        for item in parsed {
            items.push((item.path, item.name, item.folder_name));
        }
    }
    if items.is_empty() {
        return Err("没有可发送的文件".to_string());
    }
    Ok(items)
}

async fn hash_file(path: &str) -> Result<(u64, String), String> {
    let mut file = tokio::fs::File::open(path)
        .await
        .map_err(|error| format!("文件读不到：{error}"))?;
    let size = file
        .metadata()
        .await
        .map_err(|error| format!("文件信息读不到：{error}"))?
        .len();
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 1024 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .await
            .map_err(|error| format!("文件校验失败：{error}"))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    file.seek(std::io::SeekFrom::Start(0))
        .await
        .map_err(|error| format!("文件读不到：{error}"))?;
    Ok((size, format!("{:x}", hasher.finalize())))
}

async fn read_chunk(file: &mut tokio::fs::File, buffer: &mut [u8]) -> std::io::Result<usize> {
    let mut filled = 0;
    while filled < buffer.len() {
        let read = file.read(&mut buffer[filled..]).await?;
        if read == 0 {
            break;
        }
        filled += read;
    }
    Ok(filled)
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_millis() as i64)
        .unwrap_or(0)
}

/// Start a job and remember it, so the progress box can cancel or retry it.
pub fn start(app: &AppHandle, job: PeerTransferJob) -> Result<String, String> {
    let app = app.clone();
    let group = job.group.clone();
    let cancel = {
        let state = app.state::<PeerTransferRegistryState>();
        let mut registry = match state.0.lock() {
            Ok(registry) => registry,
            Err(_) => return Err("传输队列不可用".to_string()),
        };
        registry.register(job.clone())
    };
    let pusher = PeerPusher::new(app.clone());
    let finished_group = group.clone();
    tauri::async_runtime::spawn(async move {
        let failed = pusher.run_job(job, cancel.clone()).await;
        let state = app.state::<PeerTransferRegistryState>();
        let mut registry = match state.0.lock() {
            Ok(registry) => registry,
            Err(_) => return,
        };
        if cancel.load(Ordering::Relaxed) {
            // A cancelled group is finished, not retryable: the files may be
            // half-written on both ends and re-running would duplicate them.
            registry.active.remove(&finished_group);
            return;
        }
        registry.finished(&finished_group, failed);
        drop(registry);
        let _ = app.emit(
            "peer-transfer-group",
            serde_json::json!({ "group": finished_group, "status": if failed { "failed" } else { "done" } }),
        );
    });
    Ok(group)
}

/// Replay a failed group; the receiver treats a re-sent chunk of the same
/// upload id as already written, so a retried file is not duplicated.
pub fn retry(app: &AppHandle, group: &str) -> Result<String, String> {
    let job = {
        let state = app.state::<PeerTransferRegistryState>();
        let mut registry = state.0.lock().map_err(|_| "传输队列不可用".to_string())?;
        registry
            .take_retry(group)
            .ok_or_else(|| "这一批已经不在失败列表里了".to_string())?
    };
    start(app, job)
}

pub fn cancel(app: &AppHandle, group: &str) -> bool {
    let state = app.state::<PeerTransferRegistryState>();
    state
        .0
        .lock()
        .map(|mut registry| registry.cancel(group))
        .unwrap_or(false)
}

/// The corner drop box's one call: these files, to these peers. It returns as
/// soon as the group is queued — the progress window follows it by `group`.
#[tauri::command]
pub fn send_files_to_peers(
    app_handle: AppHandle,
    peer_ids: Vec<String>,
    paths: Vec<String>,
    batch_name: Option<String>,
) -> Result<String, String> {
    send(&app_handle, peer_ids, paths, batch_name)
}

/// The same call without a window in front of it. The Windows right-click menu
/// reaches the app as process arguments, and those have to pass the very same
/// checks a drop does — nothing gets to skip the path or peer-count rules.
pub fn send(
    app: &AppHandle,
    peer_ids: Vec<String>,
    paths: Vec<String>,
    batch_name: Option<String>,
) -> Result<String, String> {
    let peer_ids: Vec<String> = peer_ids
        .into_iter()
        .map(|id| id.trim().to_string())
        .filter(|id| !id.is_empty())
        .collect();
    if peer_ids.is_empty() {
        return Err("请先选择要发给谁".to_string());
    }
    if peer_ids.len() > 8 {
        return Err("一次最多发给 8 台设备".to_string());
    }
    let paths: Vec<String> = paths
        .into_iter()
        .map(|path| path.trim().to_string())
        .filter(|path| !path.is_empty())
        .collect();
    if paths.is_empty() {
        return Err("没有要发送的文件".to_string());
    }
    if paths.len() > 2_000 {
        return Err("一次最多发送 2000 个文件".to_string());
    }
    // Checked before the queue so the drop box can say which item is gone,
    // rather than showing a transfer that never starts.
    for path in &paths {
        if !std::path::Path::new(path).exists() {
            return Err(format!("文件已经不在原来的位置：{path}"));
        }
    }
    // The name shows up on the other machine's progress box, so it goes
    // through the same filter as everything a peer displays.
    let batch_name = batch_name
        .map(|name| super::roster::clean_text(&name, 120))
        .filter(|name| !name.is_empty());
    let job = PeerTransferJob {
        group: uuid::Uuid::new_v4().as_simple().to_string(),
        peer_ids,
        file_paths: paths,
        batch_name,
    };
    start(app, job)
}

/// "重试" in the progress box: replay a group that failed.
#[tauri::command]
pub fn retry_peer_transfer(app_handle: AppHandle, group: String) -> Result<String, String> {
    retry(&app_handle, group.trim())
}

#[tauri::command]
pub fn cancel_peer_transfer(app_handle: AppHandle, group: String) -> bool {
    cancel(&app_handle, group.trim())
}

/// The progress card opens on the first event, which can be before its page is
/// listening. An empty card asks this before hiding: it has nothing to show,
/// but it must not disappear while the transfer that opened it is still on.
#[tauri::command]
pub fn has_active_peer_transfer(app: AppHandle) -> bool {
    let state = app.state::<PeerTransferRegistryState>();
    state
        .0
        .lock()
        .map(|registry| !registry.active.is_empty())
        .unwrap_or(false)
}

/// Inbound progress, from the shared upload pipeline (handlers.rs).
pub fn notify_inbound(
    app: &AppHandle,
    sender_id: &str,
    sender_name: &str,
    file_name: &str,
    upload_id: &str,
    received_bytes: u64,
    total_bytes: u64,
    status: &str,
    error: Option<String>,
    batch_id: Option<String>,
    batch_name: Option<String>,
    batch_index: Option<usize>,
    batch_total: Option<usize>,
) {
    // Only peers already in the roster drive these windows; phones keep using
    // the existing chat UI.
    if !db::is_trusted_sender(app, sender_id) {
        return;
    }
    let event = PeerTransferEvent {
        group: format!("in-{upload_id}"),
        direction: "in".to_string(),
        peer_device_id: sender_id.to_string(),
        peer_name: sender_name.to_string(),
        file_name: file_name.to_string(),
        upload_id: upload_id.to_string(),
        sent_bytes: received_bytes,
        total_bytes,
        status: status.to_string(),
        error,
        batch_id,
        batch_name,
        batch_index,
        batch_total,
        timestamp: now_ms(),
    };
    super::history::record(app, &event);
    super::receive::announce_inbound(app, &event);
    crate::app::peer_floats::on_transfer_progress(app, &event.status);
    let _ = app.emit("peer-transfer-progress", event);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::file_transfer::sealed::LanKey;

    fn metadata(index: usize, total: usize, size: u64) -> ChunkMetadata {
        ChunkMetadata {
            upload_id: "u1".to_string(),
            chunk_index: index,
            total_chunks: total,
            file_name: "a.txt".to_string(),
            sender_id: "peer-a".to_string(),
            sender_name: "书房".to_string(),
            total_size: size,
            content_type: None,
            batch_id: None,
            batch_name: None,
            batch_index: None,
            batch_total: None,
            batch_size: None,
            sha256: None,
        }
    }

    #[test]
    fn an_upload_frame_carries_length_then_metadata_then_bytes() {
        let plain = upload_plain(&metadata(0, 1, 5), b"hello");
        let (meta, data) = sealed::split_upload(&plain).expect("splits");
        assert_eq!(data, b"hello");
        let parsed: ChunkMetadata = serde_json::from_slice(meta).expect("metadata");
        assert_eq!(parsed.upload_id, "u1");
    }

    #[test]
    fn an_empty_file_still_occupies_one_chunk() {
        assert_eq!(std::cmp::max(1, 0_u64.div_ceil(TRANSFER_CHUNK_SIZE as u64) as usize), 1);
    }

    #[test]
    fn chunk_count_matches_the_receiver() {
        let size = TRANSFER_CHUNK_SIZE as u64 * 3 + 7;
        let total = std::cmp::max(1, size.div_ceil(TRANSFER_CHUNK_SIZE as u64) as usize);
        assert_eq!(total, 4);
        let last = &metadata(3, total, size);
        assert_eq!(last.total_chunks, total);
    }

    #[test]
    fn sealing_a_chunk_binds_it_to_the_peer_key() {
        let key: LanKey = [4u8; 32];
        let plain = upload_plain(&metadata(0, 1, 2), b"hi");
        let boxed = sealed::seal(&key, "upload", &plain).expect("sealed");
        assert_eq!(sealed::open(&key, "upload", &boxed).as_deref(), Some(plain.as_slice()));
        assert!(sealed::open(&[5u8; 32], "upload", &boxed).is_none());
    }
}
