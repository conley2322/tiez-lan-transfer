//! Desktop-to-desktop LAN transfer.
//!
//! Tied to the same axum server the phone flow uses (`file_transfer`): a peer
//! authenticates with its own pairing token, uploads through the very same
//! sealed upload pipeline, and announces itself over a UDP beacon so roster
//! addresses stay fresh. Nothing here rotates on restart — a pairing made once
//! keeps working until the user removes it.

pub mod client;
pub mod db;
pub mod discovery;
pub mod handlers;
pub mod history;
pub mod identity;
pub mod models;
pub mod pairing;
pub mod receive;
pub mod roster;
pub mod selfcheck;
#[cfg(windows)]
pub mod win_menu;

pub use models::*;

use std::collections::HashMap;
use std::sync::{Mutex, RwLock};

use tauri::{AppHandle, Manager};

use crate::app_state::AppDataDir;

/// Register the peer state, load this machine's identity and roster, and start
/// announcing. Called once from setup, after the database is open.
///
/// A machine whose identity file cannot be read still starts: it gets an
/// in-memory identity for this session, so a locked Keychain degrades to
/// "pairs don't reach me" rather than "the app won't launch". The file itself is
/// never overwritten.
pub fn init(app: &AppHandle) {
    let data_dir = app
        .try_state::<AppDataDir>()
        .and_then(|dir| dir.0.lock().ok().map(|guard| guard.clone()));
    let identity = match data_dir.as_deref() {
        Some(dir) => match identity::load_or_create_identity(dir) {
            Ok(identity) => Some(identity),
            Err(error) => {
                eprintln!(">>> [PEER] peer identity unavailable this session: {error}");
                identity::temporary_identity()
            }
        },
        None => {
            eprintln!(">>> [PEER] peer identity unavailable: no data folder yet");
            identity::temporary_identity()
        }
    };
    // The pairing windows read these even when the identity is unavailable,
    // so they exist before the early return below.
    app.manage(ConnectRequestsState(Mutex::new(HashMap::new())));
    app.manage(ApprovedConnectsState(Mutex::new(HashMap::new())));
    let Some(identity) = identity else {
        eprintln!(">>> [PEER] device-to-device transfer is off: no random key available");
        return;
    };

    app.manage(PeerIdentityState(RwLock::new(identity)));
    app.manage(PeerRosterState(RwLock::new(HashMap::new())));
    app.manage(PairingState(Mutex::new(HashMap::new())));
    app.manage(DiscoveredState(Mutex::new(HashMap::new())));
    app.manage(PeerTransferRegistryState(Mutex::new(
        PeerTransferRegistry::default(),
    )));
    app.manage(ReceiveRequestsState(Mutex::new(HashMap::new())));
    // Filled in by the discovery thread; 连接自检 reads it instead of probing.
    app.manage(DiscoveryHealthState(Mutex::new(DiscoveryHealth::default())));

    match db::load_roster(app) {
        Ok(loaded) if loaded > 0 => eprintln!(">>> [PEER] {loaded} paired device(s) loaded"),
        Ok(_) => {}
        Err(error) => eprintln!(">>> [PEER] {error}"),
    }
    discovery::init(app);
}
