//! Confirm-before-receiving, on the machine files are heading for.
//!
//! By default files land without asking — this is a tool for one person's own
//! computers. With 「接收前询问」 on, a peer's `/p/invite` is held open here
//! (up to the 接收超时 setting) while the user slides to accept, and the
//! answer travels back on that same request. Nothing streams before the answer
//! is given, so a refused transfer never touches the disk and the sender gets a
//! truthful "对方没有接受".
//!
//! The pending request also lives in `ReceiveRequestsState`: a window created
//! after the event was emitted can still pick up what is on screen with
//! `list_receive_requests`.

use std::collections::HashMap;
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;
use tokio::sync::oneshot;

use crate::database::DbState;
use crate::infrastructure::repository::settings_repo::SettingsRepository;

use super::models::{
    InviteFile, PeerTransferEvent, PendingReceive, ReceiveRequestView, ReceiveRequestsState,
};
use super::roster;

/// How many confirmations may be open in total before we stop asking; each one
/// holds the peer's request open, so this is what keeps a chatty or hostile
/// machine from tying up the server. Reaching it is a refusal, not a queue: the
/// user can only answer one slide at a time anyway.
pub const MAX_PENDING_RECEIVES: usize = 8;

/// The settings key; anything but "true" means receive without asking.
pub const CONFIRM_SETTING: &str = "peer_confirm_receive";
/// How long the user gets to decide before the transfer is refused, when the
/// setting row is missing (a database from before the setting existed).
pub const DECIDE_TIMEOUT: Duration = Duration::from_secs(60);
/// The 接收超时 setting, in seconds.
pub const TIMEOUT_SETTING: &str = "peer_receive_timeout";
/// The range the setting is allowed to move in. The sender waits
/// `MAX_DECIDE_TIMEOUT` plus room for the answer, so an unbounded timeout here
/// would leave its invite request hanging on its own network timeout instead of
/// a real "对方没有接受".
pub const MIN_DECIDE_TIMEOUT: u64 = 15;
pub const MAX_DECIDE_TIMEOUT: u64 = 120;
/// The 接收完成通知 setting; anything but an explicit "false" keeps it on.
pub const NOTIFY_SETTING: &str = "peer_notify_received";

/// The window the user gets, read fresh on every request so a change in 设置
/// applies to the next transfer rather than at next launch.
pub fn decide_timeout(app: &AppHandle) -> Duration {
    Duration::from_secs(clamp_seconds(app_setting(app, TIMEOUT_SETTING)))
}

/// The setting is free text in the database: anything unparseable falls back to
/// the default, and whatever the user typed is kept inside the range the sender
/// is willing to wait for.
fn clamp_seconds(text: Option<String>) -> u64 {
    let requested = text
        .and_then(|text| text.trim().parse::<u64>().ok())
        .unwrap_or(DECIDE_TIMEOUT.as_secs());
    requested.clamp(MIN_DECIDE_TIMEOUT, MAX_DECIDE_TIMEOUT)
}

pub fn notify_received_enabled(app: &AppHandle) -> bool {
    app_setting(app, NOTIFY_SETTING).as_deref() != Some("false")
}

fn app_setting(app: &AppHandle, key: &str) -> Option<String> {
    app.try_state::<DbState>()
        .and_then(|state| state.settings_repo.get(key).ok().flatten())
}

pub fn confirm_required(app: &AppHandle) -> bool {
    app_setting(app, CONFIRM_SETTING).is_some_and(|value| value == "true")
}

/// 「已接收文件 · 来自XX」. One toast per batch, not per file: a folder of 200
/// files is one delivery. `notify_inbound` knows both the index and the total,
/// so it is the place that can tell the last file of a batch from the rest.
pub fn announce_inbound(app: &AppHandle, event: &PeerTransferEvent) {
    if event.status != "done" || !notify_received_enabled(app) {
        return;
    }
    let last = match (event.batch_index, event.batch_total) {
        (Some(index), Some(total)) => index + 1 >= total,
        (None, None) => true,
        _ => false,
    };
    if !last {
        return;
    }
    let count = event.batch_total.unwrap_or(1);
    let body = if count > 1 {
        format!("{} 个文件 · 来自 {}", count, event.peer_name)
    } else {
        format!("{} · 来自 {}", event.file_name, event.peer_name)
    };
    let _ = app
        .notification()
        .builder()
        .title("已接收文件")
        .body(body)
        .show();
}

/// Show the request and wait for the slide. `false` on timeout, on a window
/// that never answered, and when the pending list itself is unavailable.
pub async fn await_decision(
    app: &AppHandle,
    pending: PendingReceive,
    decision: oneshot::Receiver<bool>,
) -> bool {
    let group = pending.group.clone();
    let view = pending.view();
    let timeout = decide_timeout(app);
    let Some(state) = app.try_state::<ReceiveRequestsState>() else {
        return false;
    };
    {
        let mut guard = match state.0.lock() {
            Ok(guard) => guard,
            Err(_) => return false,
        };
        prune(&mut guard);
        if guard.len() >= MAX_PENDING_RECEIVES && !guard.contains_key(&group) {
            return false;
        }
        // The same group twice means the sender retried an invite whose answer
        // it never received; the newer request is the one to show.
        guard.insert(group.clone(), pending);
    }
    crate::app::peer_floats::on_receive_request(app);
    let _ = app.emit("peer-receive-request", view);

    let accepted = match tokio::time::timeout(timeout, decision).await {
        Ok(Ok(accept)) => accept,
        _ => false,
    };

    if let Ok(mut guard) = state.0.lock() {
        guard.remove(&group);
    }
    let _ = app.emit(
        "peer-receive-decided",
        serde_json::json!({ "group": group, "accepted": accepted }),
    );
    accepted
}

fn prune(guard: &mut HashMap<String, PendingReceive>) {
    guard.retain(|_, pending| !pending.is_expired());
}

/// What the receive-confirmation window shows when it opens mid-transfer.
#[tauri::command]
pub fn list_receive_requests(app_handle: AppHandle) -> Vec<ReceiveRequestView> {
    let Some(state) = app_handle.try_state::<ReceiveRequestsState>() else {
        return Vec::new();
    };
    let mut views = match state.0.lock() {
        Ok(mut guard) => {
            prune(&mut guard);
            guard.values().map(PendingReceive::view).collect::<Vec<_>>()
        }
        Err(_) => return Vec::new(),
    };
    views.sort_by(|a, b| a.group.cmp(&b.group));
    views
}

/// The slide-to-accept control's call. `false` when the request already timed
/// out, which is also how a second click on a stale window behaves.
#[tauri::command]
pub fn respond_receive_request(app_handle: AppHandle, group: String, accept: bool) -> bool {
    let Some(state) = app_handle.try_state::<ReceiveRequestsState>() else {
        return false;
    };
    let pending = match state.0.lock() {
        Ok(mut guard) => guard.remove(group.trim()),
        Err(_) => None,
    };
    match pending.and_then(|pending| pending.responder) {
        Some(responder) => responder.send(accept).is_ok(),
        None => false,
    }
}

/// A file list short enough to display and honest about its size, built from
/// whatever a peer claimed it was about to send.
pub fn invite_files(files: &[InviteFile]) -> Vec<InviteFile> {
    files
        .iter()
        .take(2_000)
        .map(|file| InviteFile {
            name: roster::clean_text(&file.name, 240),
            size: file.size,
        })
        .filter(|file| !file.name.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(name: &str, size: u64) -> InviteFile {
        InviteFile {
            name: name.to_string(),
            size,
        }
    }

    #[test]
    fn an_invited_file_name_is_cleaned_and_stays_readable() {
        let files = invite_files(&[
            file("报告\u{202e}gpj.pdf", 1),
            file("   ", 2),
            file("正常.mp4", 3),
        ]);
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].name, "报告gpj.pdf");
        assert_eq!(files[1].name, "正常.mp4");
        assert_eq!(files[1].size, 3);
    }

    #[test]
    fn a_receive_timeout_stays_in_the_range_the_sender_waits_for() {
        assert_eq!(clamp_seconds(Some("60".to_string())), DECIDE_TIMEOUT.as_secs());
        assert_eq!(clamp_seconds(Some(" 90 ".to_string())), 90);
        assert_eq!(clamp_seconds(Some("1".to_string())), MIN_DECIDE_TIMEOUT);
        assert_eq!(clamp_seconds(Some("3600".to_string())), MAX_DECIDE_TIMEOUT);
        // A missing or broken row is not a reason to hang a peer's request.
        assert_eq!(clamp_seconds(None), DECIDE_TIMEOUT.as_secs());
        assert_eq!(clamp_seconds(Some("很快".to_string())), DECIDE_TIMEOUT.as_secs());
    }

    #[test]
    fn a_countdown_never_goes_negative() {
        let (responder, _decision) = oneshot::channel();
        let pending = PendingReceive {
            group: "g".to_string(),
            device_id: "0123456789abcdef".to_string(),
            device_name: "Mac".to_string(),
            platform: "macos".to_string(),
            files: vec![file("a.txt", 1)],
            total_size: 1,
            expires_at: std::time::Instant::now() - Duration::from_secs(5),
            responder: Some(responder),
        };
        assert!(pending.is_expired());
        assert_eq!(pending.view().expires_in, 0);
    }
}
