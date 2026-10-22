//! The 传输记录 table: one row per file whose transfer finished, in either
//! direction.
//!
//! The progress float shows what is happening right now and forgets it a few
//! seconds later; this is what the user asks about afterwards — 那个文件到底过
//! 去了没有. Only finished files are written, because a row for a file still in
//! flight would be a lie by the time anyone reads it.

use rusqlite::{params, Connection, Result};
use serde::Serialize;

/// One outcome. `status` is `done` or `failed`; `id` is filled in on read and
/// ignored on write.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferRow {
    pub id: i64,
    pub group_id: String,
    /// `out` (this machine sent) or `in` (this machine received).
    pub direction: String,
    pub peer_device_id: String,
    pub peer_name: String,
    pub file_name: String,
    pub total_size: i64,
    pub status: String,
    pub error: String,
    pub created_at: i64,
}

pub struct SqliteTransferLogRepository {
    conn: std::sync::Arc<std::sync::Mutex<Connection>>,
}

/// How much history is kept. A row is a few hundred bytes, so this bounds the
/// table at well under a megabyte while covering years of ordinary use.
const MAX_ROWS: i64 = 1_000;

impl SqliteTransferLogRepository {
    pub fn new(conn: std::sync::Arc<std::sync::Mutex<Connection>>) -> Self {
        Self { conn }
    }

    pub fn record(&self, row: &TransferRow) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO peer_transfer_log
                (group_id, direction, peer_device_id, peer_name, file_name,
                 total_size, status, error, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                row.group_id,
                row.direction,
                row.peer_device_id,
                row.peer_name,
                row.file_name,
                row.total_size,
                row.status,
                row.error,
                row.created_at
            ],
        )?;
        // Oldest first: the newest rows are the ones the page shows, so the
        // delete cannot take anything the user is looking at.
        conn.execute(
            "DELETE FROM peer_transfer_log WHERE id NOT IN (
                 SELECT id FROM peer_transfer_log ORDER BY created_at DESC, id DESC LIMIT ?1
             )",
            params![MAX_ROWS],
        )?;
        Ok(())
    }

    /// Newest first, which is the only order the page reads.
    pub fn list(&self, limit: i64) -> Result<Vec<TransferRow>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, group_id, direction, peer_device_id, peer_name, file_name,
                    total_size, status, error, created_at
             FROM peer_transfer_log ORDER BY created_at DESC, id DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit], |row| {
            Ok(TransferRow {
                id: row.get(0)?,
                group_id: row.get(1)?,
                direction: row.get(2)?,
                peer_device_id: row.get(3)?,
                peer_name: row.get(4)?,
                file_name: row.get(5)?,
                total_size: row.get(6)?,
                status: row.get(7)?,
                error: row.get(8)?,
                created_at: row.get(9)?,
            })
        })?;
        rows.collect()
    }

    /// The 清空记录 button. Returns how many rows went away.
    pub fn clear(&self) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        Ok(conn.execute("DELETE FROM peer_transfer_log", [])?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::repository::migrations::run_migrations;

    fn repository() -> SqliteTransferLogRepository {
        let conn = std::sync::Arc::new(std::sync::Mutex::new(
            Connection::open_in_memory().expect("open database"),
        ));
        run_migrations(&conn.lock().unwrap()).expect("run migrations");
        SqliteTransferLogRepository::new(conn.clone())
    }

    fn row(id: i64, status: &str) -> TransferRow {
        TransferRow {
            id: 0,
            group_id: format!("g{status}"),
            direction: "out".to_string(),
            peer_device_id: "peer-a".to_string(),
            peer_name: "书房".to_string(),
            file_name: format!("f{id}.pdf"),
            total_size: 12,
            status: status.to_string(),
            error: if status == "failed" { "对方不在线".to_string() } else { String::new() },
            created_at: id,
        }
    }

    #[test]
    fn records_reads_back_and_clears() {
        let repo = repository();
        repo.record(&row(1, "done")).expect("record");
        repo.record(&row(2, "failed")).expect("record");
        let listed = repo.list(10).expect("list");
        // Newest first.
        assert_eq!(listed[0].file_name, "f2.pdf");
        assert_eq!(listed[0].error, "对方不在线");
        assert_eq!(listed[1].file_name, "f1.pdf");
        assert_eq!(repo.clear().expect("clear"), 2);
        assert!(repo.list(10).expect("list").is_empty());
    }

    #[test]
    fn the_list_stays_bounded() {
        let repo = repository();
        for id in 0..(MAX_ROWS + 25) {
            repo.record(&row(id, "done")).expect("record");
        }
        let listed = repo.list(MAX_ROWS + 100).expect("list");
        assert_eq!(listed.len() as i64, MAX_ROWS);
        // and it kept the newest, not an arbitrary slice
        assert_eq!(listed[0].file_name, format!("f{}.pdf", MAX_ROWS + 24));
    }
}
