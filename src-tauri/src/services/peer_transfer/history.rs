//! What the 传输记录 page reads: one row per file that finished, sent or
//! received.
//!
//! The same `PeerTransferEvent` the progress float consumes is written here when
//! it reaches a final status, so the log cannot disagree with what the user just
//! watched. Only `done` and `failed` land in the table — an `active` row would
//! need updating afterwards, and the float is the live view anyway.

use tauri::{AppHandle, Manager};

use crate::database::DbState;
use crate::infrastructure::repository::transfer_log_repo::{
    SqliteTransferLogRepository, TransferRow,
};

use super::models::PeerTransferEvent;

fn repository(app: &AppHandle) -> Option<SqliteTransferLogRepository> {
    app.try_state::<DbState>()
        .map(|state| SqliteTransferLogRepository::new(state.conn.clone()))
}

/// The row an event deserves, or nothing while it is still moving.
fn row_for(event: &PeerTransferEvent) -> Option<TransferRow> {
    if event.status != "done" && event.status != "failed" {
        return None;
    }
    Some(TransferRow {
        id: 0,
        group_id: event.group.clone(),
        direction: event.direction.clone(),
        peer_device_id: event.peer_device_id.clone(),
        peer_name: event.peer_name.clone(),
        // A whole batch can fail before any file started; the page shows that
        // row without a file name rather than dropping the only clue.
        file_name: event.file_name.clone(),
        total_size: event.total_bytes as i64,
        status: event.status.clone(),
        error: event.error.clone().unwrap_or_default(),
        created_at: event.timestamp,
    })
}

/// Called from both progress funnels — outbound in `client.rs`, inbound in
/// `notify_inbound` — so the log and the float see the same outcome.
pub fn record(app: &AppHandle, event: &PeerTransferEvent) {
    let Some(row) = row_for(event) else {
        return;
    };
    let Some(repository) = repository(app) else {
        return;
    };
    if let Err(error) = repository.record(&row) {
        eprintln!(">>> [PEER] failed to write the transfer log: {error}");
    }
}

/// How many rows the page asks for; a bounded default keeps a cold open cheap.
const DEFAULT_HISTORY_LIMIT: i64 = 300;

#[tauri::command]
pub fn list_peer_transfers(app_handle: AppHandle, limit: Option<i64>) -> Vec<TransferRow> {
    let Some(repository) = repository(&app_handle) else {
        return Vec::new();
    };
    let limit = limit.unwrap_or(DEFAULT_HISTORY_LIMIT).clamp(1, 1_000);
    repository.list(limit).unwrap_or_default()
}

#[tauri::command]
pub fn clear_peer_transfers(app_handle: AppHandle) -> Result<usize, String> {
    let repository = repository(&app_handle).ok_or("传输记录存储还没准备好")?;
    repository.clear().map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(status: &str) -> PeerTransferEvent {
        PeerTransferEvent {
            group: "g1".to_string(),
            direction: "out".to_string(),
            peer_device_id: "peer-a".to_string(),
            peer_name: "书房".to_string(),
            file_name: "report.pdf".to_string(),
            upload_id: "u1".to_string(),
            sent_bytes: 10,
            total_bytes: 10,
            status: status.to_string(),
            error: (status == "failed").then(|| "对方不在线".to_string()),
            batch_id: None,
            batch_name: None,
            batch_index: None,
            batch_total: None,
            timestamp: 7,
        }
    }

    #[test]
    fn only_a_finished_file_becomes_a_row() {
        assert!(row_for(&event("active")).is_none());
        assert_eq!(row_for(&event("done")).expect("done").status, "done");
        let failed = row_for(&event("failed")).expect("failed");
        assert_eq!(failed.error, "对方不在线");
    }

    #[test]
    fn a_row_carries_who_it_went_to_and_how_much() {
        let row = row_for(&event("done")).expect("row");
        assert_eq!(row.group_id, "g1");
        assert_eq!(row.direction, "out");
        assert_eq!(row.peer_name, "书房");
        assert_eq!(row.file_name, "report.pdf");
        assert_eq!(row.total_size, 10);
        assert_eq!(row.created_at, 7);
    }

    #[test]
    fn rows_written_by_the_repository_read_back_newest_first() {
        let conn = std::sync::Arc::new(std::sync::Mutex::new(
            rusqlite::Connection::open_in_memory().expect("open database"),
        ));
        crate::infrastructure::repository::migrations::run_migrations(&conn.lock().unwrap())
            .expect("run migrations");
        let repository = SqliteTransferLogRepository::new(conn);
        for (id, status) in [("done", "done"), ("failed", "failed")] {
            let mut event = event(status);
            event.group = id.to_string();
            let row = row_for(&event).expect("row");
            repository.record(&row).expect("record");
        }
        let rows = repository.list(10).expect("list");
        assert_eq!(rows.len(), 2);
        // Both rows share `timestamp: 7`, so the tie breaks on the rowid.
        assert_eq!(rows[0].status, "failed");
    }
}
