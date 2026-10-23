use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::{Mutex, RwLock};
use std::time::Instant;

use crate::services::file_transfer::sealed::LanKey;

/// UDP beacon port. The transfer server keeps its own TCP port; beacons are
/// how a peer's current address is learned without re-pairing after a lease
/// change.
pub const PEER_DISCOVERY_PORT: u16 = 28463;
/// Site-local multicast group; "tiez" in hex, so packet captures read as ours.
pub const PEER_MULTICAST_GROUP: &str = "ff15::7469:657a";
pub const BEACON_INTERVAL: std::time::Duration = std::time::Duration::from_secs(3);
/// How long a beacon from an unknown device stays in "nearby".
pub const DISCOVERED_TTL: std::time::Duration = std::time::Duration::from_secs(15);
/// How long either side of a pairing exchange waits for the other.
pub const PAIR_REQUEST_TTL: std::time::Duration = std::time::Duration::from_secs(5 * 60);
/// A nearby connect is clicked, not typed: the asker is watching a spinner, so
/// the card waits a minute rather than a pasted code's five.
pub const CONNECT_REQUEST_TTL: std::time::Duration = std::time::Duration::from_secs(60);

/// A peer's rotating secrets, kept in one encrypted column.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerCredentials {
    pub port: u16,
    pub auth: String,
    pub key: String,
}

impl PeerCredentials {
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    pub fn from_json(json: &str) -> Option<(u16, String, LanKey)> {
        let parsed: PeerCredentials = serde_json::from_str(json).ok()?;
        let key = crate::services::file_transfer::sealed::decode_key(&parsed.key)?;
        Some((parsed.port, parsed.auth, key))
    }
}

/// A paired desktop, ready to push files to.
#[derive(Debug, Clone)]
pub struct PeerHandle {
    pub device_id: String,
    pub device_name: String,
    pub nickname: String,
    pub platform: String,
    pub ip: String,
    pub port: u16,
    pub auth_token: String,
    pub key: LanKey,
    pub trusted: bool,
    /// "pair" (mutual code exchange); reserved for later sources.
    pub pair_source: String,
    pub avatar_color: String,
    pub created_at: i64,
    pub last_seen: i64,
    /// ⭐ in the settings list: pinned to the top and the only machines the
    /// corner box shows when 「仅收藏用户」 is on.
    pub favorite: bool,
    /// True once the user renamed this peer here. Until then `device_name` is
    /// whatever the peer last announced, and a beacon or handshake may replace
    /// it — that is how a machine that changed its own name stays current
    /// everywhere without anyone re-pairing.
    pub name_locked: bool,
}

/// What the window may see: everything except the transfer secrets.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PeerView {
    pub device_id: String,
    pub device_name: String,
    pub nickname: String,
    pub platform: String,
    pub ip: String,
    pub port: u16,
    pub trusted: bool,
    pub avatar_color: String,
    pub last_seen: i64,
    pub favorite: bool,
    /// A beacon in the last few seconds means the machine is awake and on.
    pub online: bool,
}

pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_millis() as i64)
        .unwrap_or(0)
}

impl PeerHandle {
    pub fn view(&self) -> PeerView {
        PeerView {
            device_id: self.device_id.clone(),
            device_name: self.device_name.clone(),
            nickname: self.nickname.clone(),
            platform: self.platform.clone(),
            ip: self.ip.clone(),
            port: self.port,
            trusted: self.trusted,
            avatar_color: self.avatar_color.clone(),
            online: super::roster::is_online(self, now_ms()),
            last_seen: self.last_seen,
            favorite: self.favorite,
        }
    }

    pub fn display_name(&self) -> String {
        let nickname = self.nickname.trim();
        if !nickname.is_empty() {
            return nickname.to_string();
        }
        let device = self.device_name.trim();
        if !device.is_empty() {
            return device.to_string();
        }
        self.device_id.clone()
    }
}

pub struct PeerRosterState(pub RwLock<HashMap<String, PeerHandle>>);

/// This machine's durable peer identity: unlike the phone session, it never
/// rotates, so a pairing survives reboots and port changes.
#[derive(Debug, Clone)]
pub struct PeerIdentity {
    pub device_id: String,
    pub device_name: String,
    pub nickname: String,
    pub platform: String,
    /// This machine's own avatar colour, for its own settings page. Peers pick
    /// their own colours locally (`update_peer_profile`); nobody is told what
    /// anyone else looks like.
    pub avatar_color: String,
    pub auth_token: String,
    pub key: LanKey,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PeerIdentityView {
    pub device_id: String,
    pub device_name: String,
    pub nickname: String,
    pub platform: String,
    pub avatar_color: String,
}

impl PeerIdentity {
    pub fn view(&self) -> PeerIdentityView {
        PeerIdentityView {
            device_id: self.device_id.clone(),
            device_name: self.device_name.clone(),
            nickname: self.nickname.clone(),
            platform: self.platform.clone(),
            avatar_color: self.avatar_color.clone(),
        }
    }

    pub fn display_name(&self) -> String {
        let nickname = self.nickname.trim();
        if !nickname.is_empty() {
            return nickname.to_string();
        }
        self.device_name.clone()
    }
}

pub struct PeerIdentityState(pub RwLock<PeerIdentity>);

/// The body of `/p/handshake`, sealed with the receiver's peer key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerHandshake {
    /// "request" (pair me) | "accept" | "reject".
    pub status: String,
    /// Identifies the in-flight request so an answer reaches the right side.
    #[serde(default)]
    pub group: String,
    pub device_id: String,
    pub device_name: String,
    #[serde(default)]
    pub nickname: String,
    pub platform: String,
    pub port: u16,
    pub auth: String,
    /// base64url of the sender's 32-byte peer key.
    pub key: String,
}

/// The unauthenticated ask behind clicking a nearby avatar. It carries only
/// what a beacon already carries, plus the asker's public key so the answer
/// (which includes this machine's pairing code) can be sealed back to it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectAsk {
    pub group: String,
    pub device_id: String,
    pub device_name: String,
    #[serde(default)]
    pub nickname: String,
    pub platform: String,
    pub port: u16,
    /// base64url of the asker's 32-byte peer key.
    pub key: String,
}

/// The sealed answer to a `ConnectAsk`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectReply {
    /// "accept" (carries `code`) | "reject" | "already" | "timeout".
    pub status: String,
    #[serde(default)]
    pub code: String,
}

/// One file in a group a peer wants to send, before it has sent anything.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InviteFile {
    pub name: String,
    pub size: u64,
}

/// The body of `/p/invite`: "I am about to push these files to you" — asked
/// only when this machine is set to confirm before receiving.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInvite {
    pub group: String,
    pub device_id: String,
    pub device_name: String,
    #[serde(default)]
    pub nickname: String,
    pub platform: String,
    /// The sender's peer key, so the answer can be sealed back to it.
    pub key: String,
    pub files: Vec<InviteFile>,
    pub total_size: u64,
}

/// The receiver's answer, sealed with the key the invite carried.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InviteReply {
    pub group: String,
    /// "accept" | "reject".
    pub decision: String,
}

/// A pairing exchange waiting on the other side.
#[derive(Debug, Clone)]
pub struct PendingPairing {
    pub peer_device_id: String,
    pub peer_name: String,
    pub peer_nickname: String,
    pub peer_platform: String,
    pub peer_ip: String,
    pub peer_port: u16,
    pub peer_auth: String,
    pub peer_key: String,
    /// The initiator's group id, echoed back by accept/reject.
    pub group: String,
    /// "in" = the other machine asked us; "out" = we asked and await its answer.
    pub direction: &'static str,
    pub verify_word: String,
    pub expires_at: Instant,
}

pub struct PairingState(pub Mutex<HashMap<String, PendingPairing>>);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PairRequestView {
    pub device_id: String,
    pub device_name: String,
    pub nickname: String,
    pub platform: String,
    pub ip: String,
    pub verify_word: String,
    /// Seconds left before the request is dropped.
    pub expires_in: i64,
    /// "pair" (a sealed code handshake) or "connect" (someone nearby clicked
    /// this machine's avatar). A connect has no verify word until the other
    /// side's code comes back, so the card explains itself with `kind`.
    pub kind: String,
}

impl PendingPairing {
    fn remaining(&self) -> i64 {
        self.expires_at
            .saturating_duration_since(Instant::now())
            .as_secs() as i64
    }

    pub fn view(&self) -> PairRequestView {
        PairRequestView {
            device_id: self.peer_device_id.clone(),
            device_name: self.peer_name.clone(),
            nickname: self.peer_nickname.clone(),
            platform: self.peer_platform.clone(),
            ip: self.peer_ip.clone(),
            verify_word: self.verify_word.clone(),
            expires_in: self.remaining(),
            kind: "pair".to_string(),
        }
    }

    pub fn is_expired(&self) -> bool {
        Instant::now() >= self.expires_at
    }
}

/// An inbound group waiting for the user to slide to accept. The sender's
/// request is parked on the `responder` side of the channel while it waits, so
/// this is deliberately not `Clone` — there is exactly one answer to give.
#[derive(Debug)]
pub struct PendingReceive {
    pub group: String,
    pub device_id: String,
    pub device_name: String,
    pub platform: String,
    pub files: Vec<InviteFile>,
    pub total_size: u64,
    pub expires_at: Instant,
    pub responder: Option<tokio::sync::oneshot::Sender<bool>>,
}

/// What the window may see of a pending receive, plus the countdown it shows.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReceiveRequestView {
    pub group: String,
    pub device_id: String,
    pub device_name: String,
    pub platform: String,
    pub files: Vec<InviteFile>,
    pub total_size: u64,
    pub expires_in: i64,
}

impl PendingReceive {
    fn remaining(&self) -> i64 {
        self.expires_at
            .saturating_duration_since(Instant::now())
            .as_secs() as i64
    }

    pub fn view(&self) -> ReceiveRequestView {
        ReceiveRequestView {
            group: self.group.clone(),
            device_id: self.device_id.clone(),
            device_name: self.device_name.clone(),
            platform: self.platform.clone(),
            files: self.files.clone(),
            total_size: self.total_size,
            expires_in: self.remaining(),
        }
    }

    pub fn is_expired(&self) -> bool {
        Instant::now() >= self.expires_at
    }
}

pub struct ReceiveRequestsState(pub Mutex<HashMap<String, PendingReceive>>);

/// A machine that beaconed but is not paired yet.
#[derive(Debug, Clone)]
pub struct DiscoveredPeer {
    pub device_id: String,
    pub device_name: String,
    pub platform: String,
    pub ip: String,
    pub port: u16,
    pub seen_at: Instant,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveredView {
    pub device_id: String,
    pub device_name: String,
    pub platform: String,
    pub ip: String,
    pub port: u16,
}

pub struct DiscoveredState(pub Mutex<HashMap<String, DiscoveredPeer>>);

/// A nearby machine that clicked this one's avatar. Nothing secret has crossed
/// the wire yet: accepting is what lets this machine hand back its pairing
/// code, and the sealed handshake that follows completes the pairing.
#[derive(Debug)]
pub struct PendingConnect {
    pub group: String,
    pub device_id: String,
    pub device_name: String,
    pub nickname: String,
    pub platform: String,
    pub ip: String,
    pub port: u16,
    /// The asker's public key, so the pairing code can be sealed back to it.
    pub peer_key: String,
    pub expires_at: Instant,
    pub responder: Option<tokio::sync::oneshot::Sender<bool>>,
}

impl PendingConnect {
    fn remaining(&self) -> i64 {
        self.expires_at
            .saturating_duration_since(Instant::now())
            .as_secs() as i64
    }

    pub fn view(&self) -> PairRequestView {
        PairRequestView {
            device_id: self.device_id.clone(),
            device_name: self.device_name.clone(),
            nickname: self.nickname.clone(),
            platform: self.platform.clone(),
            ip: self.ip.clone(),
            verify_word: String::new(),
            expires_in: self.remaining(),
            kind: "connect".to_string(),
        }
    }

    pub fn is_expired(&self) -> bool {
        Instant::now() >= self.expires_at
    }
}

pub struct ConnectRequestsState(pub Mutex<HashMap<String, PendingConnect>>);

/// Device ids whose user accepted a nearby connect, and when. The sealed
/// handshake that follows arrives within seconds; a minute is enough to finish
/// it without raising a second prompt, and short enough not to approve a
/// different machine that happens to reuse the id much later.
pub struct ApprovedConnectsState(pub Mutex<HashMap<String, Instant>>);

/// What the discovery thread can honestly report about itself: whether either
/// channel bound, and whether traffic has actually crossed each one since the
/// app started. 连接自检 reads this instead of guessing from the settings.
#[derive(Debug, Default, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryHealth {
    pub v4_bound: bool,
    pub v6_bound: bool,
    pub beacons_sent: u64,
    pub beacons_in: u64,
    pub last_error: String,
}

pub struct DiscoveryHealthState(pub Mutex<DiscoveryHealth>);

/// One file's progress, either direction, as the corner/progress windows need.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PeerTransferEvent {
    pub group: String,
    /// "out" = we send, "in" = we receive.
    pub direction: String,
    pub peer_device_id: String,
    pub peer_name: String,
    pub file_name: String,
    pub upload_id: String,
    pub sent_bytes: u64,
    pub total_bytes: u64,
    /// "preparing" | "active" | "done" | "failed".
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub batch_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub batch_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub batch_index: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub batch_total: Option<usize>,
    pub timestamp: i64,
}

/// A group of files aimed at one or more peers; the unit the progress box and
/// retry act on.
#[derive(Debug, Clone)]
pub struct PeerTransferJob {
    pub group: String,
    pub peer_ids: Vec<String>,
    pub file_paths: Vec<String>,
    pub batch_name: Option<String>,
}

pub struct PeerTransferRegistryState(pub Mutex<PeerTransferRegistry>);

#[derive(Default)]
pub struct PeerTransferRegistry {
    /// group -> (job, cancel flag) for everything still running.
    pub active: HashMap<String, (PeerTransferJob, std::sync::Arc<AtomicBool>)>,
    /// Failed groups kept so "重试" can replay them without the UI resending paths.
    pub retryable: HashMap<String, PeerTransferJob>,
}

impl PeerTransferRegistry {
    pub fn register(&mut self, job: PeerTransferJob) -> std::sync::Arc<AtomicBool> {
        let cancel = std::sync::Arc::new(AtomicBool::new(false));
        let group = job.group.clone();
        self.retryable.remove(&group);
        self.active.insert(group, (job, cancel.clone()));
        cancel
    }

    pub fn finished(&mut self, group: &str, failed: bool) {
        if let Some((job, _)) = self.active.remove(group) {
            if failed {
                self.retryable.insert(group.to_string(), job);
            }
        }
    }

    pub fn take_retry(&mut self, group: &str) -> Option<PeerTransferJob> {
        self.retryable.remove(group)
    }

    pub fn cancel(&mut self, group: &str) -> bool {
        match self.active.get(group) {
            Some((_, cancel)) => {
                cancel.store(true, std::sync::atomic::Ordering::Relaxed);
                true
            }
            None => false,
        }
    }
}
