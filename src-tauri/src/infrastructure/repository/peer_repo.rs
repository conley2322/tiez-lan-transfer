use rusqlite::{params, Connection, Result};

use crate::infrastructure::encryption;

/// One paired desktop in the LAN roster. `credentials` is stored encrypted
/// (JSON: port, auth token, hex transfer key) because it rotates when the
/// peer restarts its server, unlike name/nickname.
#[derive(Debug, Clone)]
pub struct StoredPeer {
    pub device_id: String,
    pub device_name: String,
    pub nickname: String,
    pub platform: String,
    pub last_ip: String,
    pub trusted: bool,
    pub pair_source: String,
    pub avatar_color: String,
    pub credentials: String,
    pub created_at: i64,
    pub last_seen: i64,
    /// ⭐ — set by the user, and the only thing in this row that a peer never
    /// learns about itself.
    pub favorite: bool,
}

pub struct SqlitePeerRepository {
    conn: std::sync::Arc<std::sync::Mutex<Connection>>,
}

const COLUMNS: &str = "device_id, device_name, nickname, platform, last_ip, trusted, pair_source, avatar_color, credentials, created_at, last_seen, favorite";

fn row_to_peer(row: &rusqlite::Row<'_>) -> Result<StoredPeer> {
    Ok(StoredPeer {
        device_id: row.get(0)?,
        device_name: row.get(1)?,
        nickname: row.get(2)?,
        platform: row.get(3)?,
        last_ip: row.get(4)?,
        trusted: row.get::<_, i64>(5)? != 0,
        pair_source: row.get(6)?,
        avatar_color: row.get(7)?,
        credentials: row.get(8)?,
        created_at: row.get(9)?,
        last_seen: row.get(10)?,
        favorite: row.get::<_, i64>(11)? != 0,
    })
}

impl SqlitePeerRepository {
    pub fn new(conn: std::sync::Arc<std::sync::Mutex<Connection>>) -> Self {
        Self { conn }
    }

    fn decrypt_credentials(stored: &str) -> Result<String> {
        if stored.is_empty() {
            return Ok(String::new());
        }
        // Portable builds deliberately store plaintext; anything else is an
        // envelope and must not be handed out unreadable.
        if !encryption::is_encrypted_value(stored) {
            return Ok(stored.to_string());
        }
        encryption::decrypt_stored_value(stored)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e))))
    }

    fn encrypt_credentials(plain: &str) -> Result<String> {
        if plain.is_empty() {
            return Ok(String::new());
        }
        encryption::encrypt_for_storage(plain)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e))))
    }

    /// Insert or update a paired device. `favorite` is written on insert only:
    /// a re-pair carries nothing about how *this* machine labels the peer, so
    /// the ON CONFLICT list deliberately leaves it alone and `set_favorite` is
    /// the only update path.
    pub fn upsert(&self, peer: &StoredPeer) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let credentials = Self::encrypt_credentials(&peer.credentials)?;
        conn.execute(
            "INSERT INTO lan_peers
                (device_id, device_name, nickname, platform, last_ip, trusted,
                 pair_source, avatar_color, credentials, created_at, last_seen, favorite)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
             ON CONFLICT(device_id) DO UPDATE SET
                 device_name = excluded.device_name,
                 nickname = excluded.nickname,
                 platform = excluded.platform,
                 last_ip = excluded.last_ip,
                 trusted = excluded.trusted,
                 pair_source = excluded.pair_source,
                 avatar_color = excluded.avatar_color,
                 credentials = excluded.credentials,
                 created_at = excluded.created_at,
                 last_seen = excluded.last_seen",
            params![
                peer.device_id,
                peer.device_name,
                peer.nickname,
                peer.platform,
                peer.last_ip,
                peer.trusted as i64,
                peer.pair_source,
                peer.avatar_color,
                credentials,
                peer.created_at,
                peer.last_seen,
                peer.favorite as i64
            ],
        )?;
        Ok(())
    }

    pub fn list(&self) -> Result<Vec<StoredPeer>> {
        let conn = self.conn.lock().unwrap();
        let sql = format!("SELECT {} FROM lan_peers ORDER BY created_at ASC", COLUMNS);
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map([], row_to_peer)?;
        rows.map(|row| row.and_then(Self::decrypt_row_credentials))
            .collect()
    }

    pub fn get(&self, device_id: &str) -> Result<Option<StoredPeer>> {
        let conn = self.conn.lock().unwrap();
        let sql = format!("SELECT {} FROM lan_peers WHERE device_id = ?1", COLUMNS);
        let mut stmt = conn.prepare(&sql)?;
        let mut rows = stmt.query_map(params![device_id], row_to_peer)?;
        match rows.next() {
            Some(row) => row.and_then(Self::decrypt_row_credentials).map(Some),
            None => Ok(None),
        }
    }

    fn decrypt_row_credentials(peer: StoredPeer) -> Result<StoredPeer> {
        let credentials = Self::decrypt_credentials(&peer.credentials)?;
        Ok(StoredPeer { credentials, ..peer })
    }

    pub fn remove(&self, device_id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM lan_peers WHERE device_id = ?1", params![device_id])?;
        Ok(())
    }

    /// UDP beacons only ever refresh the address and the seen time; they must
    /// not blank the credentials a pairing exchange delivered.
    pub fn update_seen(&self, device_id: &str, ip: &str, seen_ms: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE lan_peers SET last_ip = ?2, last_seen = ?3 WHERE device_id = ?1",
            params![device_id, ip, seen_ms],
        )?;
        Ok(())
    }

    /// Settings edits for the user-visible fields.
    pub fn update_profile(&self, device_id: &str, nickname: &str, avatar_color: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE lan_peers SET nickname = ?2, avatar_color = ?3 WHERE device_id = ?1",
            params![device_id, nickname, avatar_color],
        )?;
        Ok(())
    }

    pub fn set_favorite(&self, device_id: &str, favorite: bool) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE lan_peers SET favorite = ?2 WHERE device_id = ?1",
            params![device_id, favorite as i64],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::repository::migrations::run_migrations;

    fn repository() -> (SqlitePeerRepository, std::sync::Arc<std::sync::Mutex<Connection>>) {
        let conn = std::sync::Arc::new(std::sync::Mutex::new(
            Connection::open_in_memory().expect("open database"),
        ));
        {
            let guard = conn.lock().unwrap();
            run_migrations(&guard).expect("run migrations");
        }
        (SqlitePeerRepository::new(conn.clone()), conn)
    }

    fn peer(id: &str) -> StoredPeer {
        StoredPeer {
            device_id: id.to_string(),
            device_name: "Mac".to_string(),
            nickname: "书房 Mac".to_string(),
            platform: "macos".to_string(),
            last_ip: "192.168.1.10".to_string(),
            trusted: true,
            pair_source: "qr".to_string(),
            avatar_color: "#0A84FF".to_string(),
            credentials: r#"{"port":28462,"auth":"t","key":"ab"}"#.to_string(),
            created_at: 1,
            last_seen: 0,
            favorite: false,
        }
    }

    #[test]
    fn round_trips_a_peer() {
        let (repo, _conn) = repository();
        repo.upsert(&peer("abc")).expect("insert");
        let listed = repo.list().expect("list");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].nickname, "书房 Mac");
        assert!(listed[0].trusted);
        // The credentials blob survives the at-rest cipher.
        let plain: String = {
            let conn = _conn.lock().unwrap();
            conn.query_row("SELECT credentials FROM lan_peers", [], |r| r.get(0))
                .unwrap()
        };
        assert!(!plain.contains("28462"), "stored form must not be plaintext");
        let got = repo.get("abc").expect("get").expect("present");
        assert_eq!(got.credentials, peer("abc").credentials);
        assert!(repo.get("missing").expect("get").is_none());
        repo.remove("abc").expect("remove");
        assert!(repo.list().expect("list").is_empty());
    }

    #[test]
    fn update_seen_keeps_credentials_and_trust() {
        let (repo, _conn) = repository();
        repo.upsert(&peer("abc")).expect("insert");
        repo.update_seen("abc", "10.0.0.5", 99).expect("seen");
        let got = repo.get("abc").expect("get").expect("present");
        assert_eq!(got.last_ip, "10.0.0.5");
        assert_eq!(got.last_seen, 99);
        assert!(got.trusted);
        assert!(!got.credentials.is_empty());
    }

    #[test]
    fn upsert_replaces_the_same_device() {
        let (repo, _conn) = repository();
        repo.upsert(&peer("abc")).expect("first");
        let mut rotated = peer("abc");
        rotated.credentials = r#"{"port":28463,"auth":"t2","key":"cd"}"#.to_string();
        rotated.created_at = 7;
        repo.upsert(&rotated).expect("rotate");
        let listed = repo.list().expect("list");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].created_at, 7);
        assert!(listed[0].credentials.contains("28463"));
    }

    #[test]
    fn a_repair_keeps_the_star_the_user_gave_it() {
        let (repo, _conn) = repository();
        repo.upsert(&peer("abc")).expect("insert");
        repo.set_favorite("abc", true).expect("star");

        // The handshake that arrives on a re-pair knows nothing about the ⭐.
        let mut again = peer("abc");
        again.favorite = false;
        again.nickname = "客厅".to_string();
        repo.upsert(&again).expect("re-pair");

        let listed = repo.list().expect("list");
        assert!(listed[0].favorite, "a fresh handshake must not clear the star");
        assert_eq!(listed[0].nickname, "客厅");

        repo.set_favorite("abc", false).expect("unstar");
        assert!(!repo.get("abc").expect("get").expect("present").favorite);
    }
}
