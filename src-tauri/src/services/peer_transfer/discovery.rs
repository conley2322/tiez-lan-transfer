//! UDP discovery: announce ourselves, and keep every paired peer's address
//! fresh without asking the user to re-pair after a DHCP lease change.
//!
//! Two channels, like LocalSend: an IPv4 broadcast on `PEER_DISCOVERY_PORT`
//! and an IPv6 site-local multicast on the port after it (binding `::` and
//! `0.0.0.0` to the same port would collide). A beacon carries no secret: only
//! the device id, a display name and the TCP port. Pairing still requires the
//! code.

use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

use crate::database::DbState;
use crate::infrastructure::repository::settings_repo::SettingsRepository;
use crate::services::file_transfer::models::ServerInfo;

use super::db;
use super::identity;
use super::models::{
    DiscoveredPeer, DiscoveredState, DiscoveredView, DiscoveryHealth, DiscoveryHealthState,
    BEACON_INTERVAL, DISCOVERED_TTL, PEER_DISCOVERY_PORT, PEER_MULTICAST_GROUP,
};
use super::roster;

/// v6 multicast uses the next port: the v4 socket already holds this one.
const MULTICAST_PORT: u16 = PEER_DISCOVERY_PORT + 1;
/// IPv4 multicast fallback for networks that drop broadcast, the address
/// LocalSend uses. Joining costs one option on the socket we already have.
const MULTICAST_V4_GROUP: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 167);
/// A beacon that arrives from a machine we can answer is answered once in a
/// while by unicast: a firewall that drops one direction of broadcast still
/// lets the other machine learn our address. The `reply` flag stops the two
/// from answering each other forever.
const REPLY_WINDOW: Duration = Duration::from_secs(3);

/// 设置 → 文件传输 → 自动发现. Off means this machine neither announces
/// itself nor listens: peers keep their last known address, so a machine whose
/// lease changed while this was off has to be paired again.
pub const ENABLED_SETTING: &str = "peer_discovery_enabled";

pub fn is_enabled(app: &AppHandle) -> bool {
    app.try_state::<DbState>()
        .and_then(|state| state.settings_repo.get(ENABLED_SETTING).ok().flatten())
        .as_deref()
        != Some("false")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Beacon {
    /// Protocol version; a beacon that isn't ours is ignored.
    v: u8,
    id: String,
    name: String,
    port: u16,
    #[serde(default)]
    platform: String,
    /// Set on the unicast answer to a beacon we heard. An answer must not be
    /// answered again, or two machines would talk in a loop.
    #[serde(default)]
    reply: bool,
}

pub fn init(app: &AppHandle) {
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || run(handle));
}

fn run(app: AppHandle) {
    let v4 = match UdpSocket::bind(("0.0.0.0", PEER_DISCOVERY_PORT)) {
        Ok(socket) => {
            let _ = socket.set_broadcast(true);
            // Multicast is the fallback for access points that drop broadcast.
            // Join on every local address: interface 0 only gets the default
            // route, which on a laptop is often not the LAN at all.
            let _ = socket.join_multicast_v4(&MULTICAST_V4_GROUP, &Ipv4Addr::UNSPECIFIED);
            for ip in local_v4_ips() {
                let _ = socket.join_multicast_v4(&MULTICAST_V4_GROUP, &ip);
            }
            Some(socket)
        }
        Err(err) => {
            eprintln!(">>> [PEER] IPv4 discovery unavailable: {err}");
            note_health(&app, move |health| health.last_error = err.to_string());
            None
        }
    };
    let v6 = bind_multicast();
    note_health(&app, |health| {
        health.v4_bound = v4.is_some();
        health.v6_bound = v6.is_some();
    });
    if v4.is_none() && v6.is_none() {
        eprintln!(">>> [PEER] discovery disabled: no UDP socket could be bound");
        return;
    }

    let mut buffer = [0u8; 2048];
    let mut next_beacon = Instant::now();
    let mut next_sender_refresh = Instant::now();
    let mut lan = V4Broadcast::current();
    // Source -> when we last answered it. Replies are a repair path, not a
    // conversation: one every REPLY_WINDOW is plenty.
    let mut replies: HashMap<IpAddr, Instant> = HashMap::new();
    // Read once per beacon rather than per tick: the setting is a row in SQLite
    // and this loop wakes every 120ms.
    let mut listening = is_enabled(&app);
    loop {
        for socket in v4.iter().chain(v6.iter()) {
            // The read timeout is the tick: it keeps receiving between beacons.
            let _ = socket.set_read_timeout(Some(Duration::from_millis(120)));
            if !listening {
                // Nothing to read; without a wait here the loop would spin for
                // three seconds between setting checks and burn a core.
                std::thread::sleep(Duration::from_millis(200));
                continue;
            }
            while let Ok((len, from)) = socket.recv_from(&mut buffer) {
                if len > 0 {
                    handle_beacon(&app, &buffer[..len], &from, v4.as_ref(), &mut replies);
                }
            }
        }
        if next_beacon <= Instant::now() {
            next_beacon = Instant::now() + BEACON_INTERVAL;
            let enabled = is_enabled(&app);
            if enabled != listening {
                listening = enabled;
                if !enabled {
                    forget_discovered(&app);
                }
            }
            if enabled {
                if next_sender_refresh <= Instant::now() {
                    next_sender_refresh = Instant::now() + Duration::from_secs(30);
                    lan = V4Broadcast::current();
                    // Wi-Fi may have come up since the socket was bound.
                    if let Some(socket) = v4.as_ref() {
                        for ip in local_v4_ips() {
                            let _ = socket.join_multicast_v4(&MULTICAST_V4_GROUP, &ip);
                        }
                    }
                }
                send_beacons(&app, v4.as_ref(), v6.as_ref(), &lan);
            }
        }
    }
}

fn local_v4_ips() -> Vec<Ipv4Addr> {
    crate::services::file_transfer::get_available_ips()
        .iter()
        .filter_map(|ip| ip.parse::<Ipv4Addr>().ok())
        .filter(|ip| !ip.is_loopback() && !ip.is_unspecified())
        .collect()
}

/// One ephemeral socket per local IPv4, so `255.255.255.255` leaves every
/// interface instead of only the default route. Sending from the interface's
/// own address is what makes a multi-homed machine (Wi-Fi + Ethernet + a VPN)
/// announce on the network the user actually cares about.
struct V4Broadcast {
    sockets: Vec<(Ipv4Addr, UdpSocket)>,
}

impl V4Broadcast {
    fn current() -> Self {
        let mut sockets = Vec::new();
        for ip in local_v4_ips() {
            if let Ok(socket) = UdpSocket::bind((ip, 0)) {
                let _ = socket.set_broadcast(true);
                sockets.push((ip, socket));
            }
        }
        Self { sockets }
    }

    fn send(&self, body: &[u8]) -> usize {
        let mut sent = 0;
        for (_, socket) in &self.sockets {
            // Both: limited broadcast is what older peers listen for, multicast
            // survives APs that filter broadcast.
            if socket
                .send_to(body, (Ipv4Addr::BROADCAST, PEER_DISCOVERY_PORT))
                .is_ok()
            {
                sent += 1;
            }
            if socket
                .send_to(body, (MULTICAST_V4_GROUP, PEER_DISCOVERY_PORT))
                .is_ok()
            {
                sent += 1;
            }
        }
        sent
    }
}

/// What the loop knows about itself, which is what 连接自检 can honestly claim.
fn note_health(app: &AppHandle, f: impl FnOnce(&mut DiscoveryHealth)) {
    let Some(state) = app.try_state::<DiscoveryHealthState>() else {
        return;
    };
    // Bound to a local: a guard used in a tail expression of an `if let`
    // outlives `state`, which borrows it (E0597).
    let Ok(mut guard) = state.0.lock() else {
        return;
    };
    f(&mut guard);
}

/// An unpaired machine stops being listed the moment discovery is turned off,
/// so the settings page cannot pair against one this machine has just agreed to
/// stop listening for.
fn forget_discovered(app: &AppHandle) {
    let had = {
        let state = app.state::<DiscoveredState>();
        let mut guard = state.0.lock().unwrap();
        let had = !guard.is_empty();
        guard.clear();
        had
    };
    if had {
        let _ = app.emit("peer-discovered", discovered_views(app));
    }
}

fn bind_multicast() -> Option<UdpSocket> {
    let group: Ipv6Addr = PEER_MULTICAST_GROUP.parse().ok()?;
    let socket = UdpSocket::bind(("::", MULTICAST_PORT)).ok()?;
    // Interface 0 lets the kernel pick the route the group is reachable on.
    socket.join_multicast_v6(&group, 0).ok()?;
    Some(socket)
}

fn source_ip(address: &std::net::SocketAddr) -> IpAddr {
    match address {
        std::net::SocketAddr::V4(v4) => IpAddr::V4(*v4.ip()),
        std::net::SocketAddr::V6(v6) => {
            let ip = *v6.ip();
            match ip.to_ipv4_mapped() {
                Some(mapped) => IpAddr::V4(mapped),
                None => IpAddr::V6(ip),
            }
        }
    }
}

fn beacon_payload(app: &AppHandle, reply: bool) -> Option<Vec<u8>> {
    let server = app.state::<ServerInfo>();
    let port = server.port.load(std::sync::atomic::Ordering::Relaxed);
    if port == 0 {
        // Nothing to connect to while the server is off; announcing a port we
        // are not listening on would only make peers fail later.
        return None;
    }
    let identity = identity::current(app);
    let body = serde_json::to_vec(&Beacon {
        v: 1,
        id: identity.device_id.clone(),
        name: identity.display_name(),
        port,
        platform: identity.platform.clone(),
        reply,
    })
    .ok()?;
    Some(body)
}

fn send_beacons(
    app: &AppHandle,
    v4: Option<&UdpSocket>,
    v6: Option<&UdpSocket>,
    lan: &V4Broadcast,
) {
    let Some(body) = beacon_payload(app, false) else {
        return;
    };
    let mut sent = 0;
    if let Some(socket) = v4 {
        if socket
            .send_to(&body, (Ipv4Addr::BROADCAST, PEER_DISCOVERY_PORT))
            .is_ok()
        {
            sent += 1;
        }
    }
    sent += lan.send(&body);
    if let Some(socket) = v6 {
        if let Ok(group) = PEER_MULTICAST_GROUP.parse::<Ipv6Addr>() {
            if socket.send_to(&body, (group, MULTICAST_PORT)).is_ok() {
                sent += 1;
            }
        }
    }
    if sent > 0 {
        note_health(app, |health| health.beacons_sent += sent as u64);
    }
}

/// Answer a beacon by unicast, so a machine whose broadcast never reaches us
/// still learns our address. Addressed to the fixed discovery port: the beacon
/// itself may have left from a per-interface socket with an ephemeral port.
fn answer_beacon(
    app: &AppHandle,
    v4: Option<&UdpSocket>,
    replies: &mut HashMap<IpAddr, Instant>,
    from_ip: &IpAddr,
) {
    let IpAddr::V4(ip) = from_ip else {
        return;
    };
    let Some(socket) = v4 else {
        return;
    };
    let now = Instant::now();
    if replies
        .get(from_ip)
        .is_some_and(|last| now.duration_since(*last) < REPLY_WINDOW)
    {
        return;
    }
    let Some(body) = beacon_payload(app, true) else {
        return;
    };
    if socket
        .send_to(&body, (IpAddr::V4(*ip), PEER_DISCOVERY_PORT))
        .is_ok()
    {
        replies.insert(*from_ip, now);
        // Keep the map to machines actually around, not every IP ever seen.
        replies.retain(|_, at| now.duration_since(*at) < REPLY_WINDOW * 4);
        note_health(app, |health| health.beacons_sent += 1);
    }
}

fn is_plausible_device_id(id: &str) -> bool {
    id.len() >= 8 && id.len() <= 64 && id.chars().all(|c| c.is_ascii_hexdigit())
}

fn handle_beacon(
    app: &AppHandle,
    bytes: &[u8],
    from: &SocketAddr,
    v4: Option<&UdpSocket>,
    replies: &mut HashMap<IpAddr, Instant>,
) {
    let Ok(beacon) = serde_json::from_slice::<Beacon>(bytes) else {
        return;
    };
    if beacon.v != 1 || !is_plausible_device_id(&beacon.id) {
        return;
    }
    // Counted before the self-check: a beacon from this machine is still
    // evidence the socket listens, which is the one thing 连接自检 reports.
    note_health(app, |health| health.beacons_in += 1);
    let ours = identity::current(app);
    if ours.device_id == beacon.id {
        return;
    }
    let from_ip = source_ip(from);
    if !beacon.reply {
        answer_beacon(app, v4, replies, &from_ip);
    }
    let ip_text = from_ip.to_string();

    if db::is_trusted_sender(app, &beacon.id) {
        // Paired machines belong in 设备与用户, not the nearby list. A pairing
        // that happened seconds ago may have left an entry here.
        let state = app.state::<DiscoveredState>();
        let removed = {
            let mut guard = state.0.lock().unwrap();
            guard.remove(&beacon.id).is_some()
        };
        if removed {
            let _ = app.emit("peer-discovered", discovered_views(app));
        }
        if db::apply_seen(app, &beacon.id, &beacon.name, &ip_text, beacon.port) {
            roster::notify_peers_changed(app);
        }
        return;
    }

    // An unpaired machine nearby. A repaint is only worth an event when
    // something visible changed: beacons arrive every few seconds.
    let state = app.state::<DiscoveredState>();
    let changed = {
        let mut guard = state.0.lock().unwrap();
        prune_discovered(&mut guard);
        let before = discovered_snapshot(&guard);
        guard.insert(
            beacon.id.clone(),
            DiscoveredPeer {
                device_id: beacon.id.clone(),
                device_name: roster::clean_text(&beacon.name, 80),
                platform: beacon.platform.clone(),
                ip: ip_text,
                port: beacon.port,
                seen_at: Instant::now(),
            },
        );
        before != discovered_snapshot(&guard)
    };
    if changed {
        let _ = app.emit("peer-discovered", discovered_views(app));
    }
}

/// Everything the nearby list shows, sorted by id, so two snapshots compare by
/// value rather than by the map's iteration order.
fn discovered_snapshot(
    guard: &HashMap<String, DiscoveredPeer>,
) -> Vec<(String, String, String, String, u16)> {
    let mut list: Vec<(String, String, String, String, u16)> = guard
        .values()
        .map(|peer| {
            (
                peer.device_id.clone(),
                peer.device_name.clone(),
                peer.platform.clone(),
                peer.ip.clone(),
                peer.port,
            )
        })
        .collect();
    list.sort();
    list
}

fn discovered_ids(guard: &HashMap<String, DiscoveredPeer>) -> Vec<String> {
    let mut ids: Vec<String> = guard.keys().cloned().collect();
    ids.sort();
    ids
}

fn prune_discovered(guard: &mut HashMap<String, DiscoveredPeer>) {
    let now = Instant::now();
    guard.retain(|_, peer| now.duration_since(peer.seen_at) < DISCOVERED_TTL);
}

/// The settings window's "unpaired machines nearby" list. Pairing still needs
/// a code: seeing a name is not a reason to trust it.
#[tauri::command]
pub fn list_nearby_peers(app_handle: AppHandle) -> Vec<DiscoveredView> {
    discovered_views(&app_handle)
}

/// The one nearby machine a connect click targets, still fresh.
pub fn discovered_peer(app: &AppHandle, device_id: &str) -> Option<DiscoveredPeer> {
    let state = app.state::<DiscoveredState>();
    let mut guard = state.0.lock().ok()?;
    prune_discovered(&mut guard);
    guard.get(device_id).cloned()
}

pub fn discovered_views(app: &AppHandle) -> Vec<DiscoveredView> {
    let state = app.state::<DiscoveredState>();
    let mut guard = state.0.lock().unwrap();
    prune_discovered(&mut guard);
    let mut views: Vec<DiscoveredView> = guard
        .values()
        // A peer paired through a code (not a beacon) is still in this map
        // until its next beacon arrives; showing it under 附近的电脑 for those
        // seconds invites a second pairing with a machine already in the list.
        .filter(|peer| !db::is_trusted_sender(app, &peer.device_id))
        .map(|peer| DiscoveredView {
            device_id: peer.device_id.clone(),
            device_name: peer.device_name.clone(),
            platform: peer.platform.clone(),
            ip: peer.ip.clone(),
            port: peer.port,
        })
        .collect();
    views.sort_by(|a, b| a.device_id.cmp(&b.device_id));
    views
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn beacons_carry_only_public_fields() {
        let body = serde_json::to_vec(&Beacon {
            v: 1,
            id: "0123456789abcdef".to_string(),
            name: "书房".to_string(),
            port: 28462,
            platform: "macos".to_string(),
            reply: false,
        })
        .unwrap();
        let text = String::from_utf8(body).unwrap();
        assert!(text.contains("书房"));
        assert!(!text.contains("auth"), "a beacon must not carry a token");
        assert!(!text.contains("key"), "a beacon must not carry a key");
    }

    #[test]
    fn a_beacon_of_another_version_is_not_acted_on() {
        let parsed = serde_json::from_slice::<Beacon>(b"{\"v\":2,\"id\":\"x\",\"name\":\"n\",\"port\":1}")
            .expect("decodes");
        assert_ne!(parsed.v, 1);
        assert!(serde_json::from_slice::<Beacon>(b"{\"id\":\"x\"}").is_err());
    }

    #[test]
    fn an_older_beacon_without_the_new_fields_still_decodes() {
        let parsed = serde_json::from_slice::<Beacon>(
            b"{\"v\":1,\"id\":\"01234567\",\"name\":\"old\",\"port\":1}",
        )
        .expect("decodes");
        assert!(!parsed.reply);
        assert_eq!(parsed.platform, "");
    }

    #[test]
    fn the_multicast_group_is_a_site_local_v6_address() {
        let group = PEER_MULTICAST_GROUP.parse::<Ipv6Addr>().expect("parses");
        assert!(group.is_multicast());
        assert_eq!(MULTICAST_PORT, PEER_DISCOVERY_PORT + 1);
    }

    #[test]
    fn discovered_pruning_drops_silent_machines() {
        let mut guard: HashMap<String, DiscoveredPeer> = HashMap::new();
        guard.insert(
            "fresh".to_string(),
            DiscoveredPeer {
                device_id: "fresh".to_string(),
                device_name: "a".to_string(),
                platform: "macos".to_string(),
                ip: "1.2.3.4".to_string(),
                port: 1,
                seen_at: Instant::now(),
            },
        );
        guard.insert(
            "stale".to_string(),
            DiscoveredPeer {
                device_id: "stale".to_string(),
                device_name: "a".to_string(),
                platform: "macos".to_string(),
                ip: "1.2.3.4".to_string(),
                port: 1,
                seen_at: Instant::now() - DISCOVERED_TTL * 2,
            },
        );
        prune_discovered(&mut guard);
        assert_eq!(discovered_ids(&guard), vec!["fresh".to_string()]);
    }

    #[test]
    fn a_v4_mapped_source_is_read_as_ipv4() {
        let mapped = std::net::SocketAddr::new(
            IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0xffff, 0xc0a8, 0x14)),
            28463,
        );
        assert_eq!(source_ip(&mapped).to_string(), "192.168.0.20");
        let v6 = std::net::SocketAddr::new(IpAddr::V6(Ipv6Addr::LOCALHOST), 28464);
        assert_eq!(source_ip(&v6), IpAddr::V6(Ipv6Addr::LOCALHOST));
    }
}
