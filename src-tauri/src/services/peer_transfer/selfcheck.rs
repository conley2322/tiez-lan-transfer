//! 连接自检: what this machine can prove about itself, in the order a user can
//! act on.
//!
//! Every check reads a fact the running app already knows — the port the server
//! stored, the address the OS reports, the counters the discovery thread keeps,
//! the roster's own freshness window. None of them rebinds a socket or sends a
//! probe, so the page cannot turn a self-check into a port scan of the LAN, and
//! a green tick means "this happened", not "this should work".

use serde::Serialize;
use tauri::{AppHandle, Manager};

use crate::services::file_transfer::models::ServerInfo;

use super::discovery;
use super::models::{DiscoveryHealth, DiscoveryHealthState, PeerHandle, PeerRosterState, PEER_DISCOVERY_PORT};

/// Which fact was tested, whether it holds, and what it actually saw. The label
/// and detail are Chinese on purpose: the two are one sentence about this
/// machine, not locale strings, and a page that had to supply its own name for
/// each key would silently print `server` the moment Rust added a seventh check.
#[derive(Debug, Serialize)]
pub struct Check {
    pub key: &'static str,
    pub ok: bool,
    pub detail: String,
    /// 「传输服务」 — the name the page prints next to the tick.
    pub label: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SelfCheck {
    pub checks: Vec<Check>,
    /// False when the machine is not reachable at all; a self-check that only
    /// found a stopped server is not a broken network.
    pub ready: bool,
}

fn check(key: &'static str, label: &'static str, ok: bool, detail: String) -> Check {
    Check {
        key,
        label,
        ok,
        detail,
    }
}

fn health(app: &AppHandle) -> DiscoveryHealth {
    app.try_state::<DiscoveryHealthState>()
        .and_then(|state| state.0.lock().ok().map(|guard| guard.clone()))
        .unwrap_or_default()
}

/// A server that never started is the reason behind most of the other failures,
/// so it is checked first and named in their details.
pub fn run(app: &AppHandle) -> SelfCheck {
    let port = app
        .try_state::<ServerInfo>()
        .map(|info| info.port.load(std::sync::atomic::Ordering::Relaxed))
        .unwrap_or(0);
    let mut checks = Vec::new();

    checks.push(check(
        "server",
        "传输服务",
        port != 0,
        if port == 0 {
            "传输服务没有运行，其它电脑连不上这台".to_string()
        } else {
            format!("正在监听 :{port}")
        },
    ));

    let ip = if port == 0 {
        String::new()
    } else {
        crate::services::file_transfer::get_local_ip_addr(app.clone())
    };
    let has_ip = !ip.is_empty() && !ip.starts_with("127.");
    checks.push(check(
        "address",
        "本机地址",
        has_ip,
        if has_ip {
            format!("本机地址 {ip}")
        } else if port == 0 {
            "服务未启动，暂时不取本机地址".to_string()
        } else {
            "没找到可用的局域网地址（网络没连上，或只有回环）".to_string()
        },
    ));

    let our_health = health(app);
    let discovery_on = discovery::is_enabled(app);
    checks.push(check(
        "discovery",
        "广播发送",
        !discovery_on || (our_health.v4_bound && our_health.beacons_sent > 0),
        if !discovery_on {
            "已按设置关闭，配对码仍可用".to_string()
        } else if !our_health.v4_bound {
            format!(
                "广播端口 {PEER_DISCOVERY_PORT} 被占用，无法自动发现{}",
                if our_health.last_error.is_empty() {
                    String::new()
                } else {
                    format!("（{}）", our_health.last_error)
                }
            )
        } else if our_health.beacons_sent == 0 {
            "广播在发，但服务还没起来，所以还没真正发出过".to_string()
        } else {
            format!("已发出 {} 次广播", our_health.beacons_sent)
        },
    ));

    checks.push(check(
        "listen",
        "广播接收",
        !discovery_on || our_health.beacons_in > 0,
        if !discovery_on {
            "已关闭，不监听邻居".to_string()
        } else if our_health.beacons_in == 0 {
            "还没收到其它电脑的广播：对方可能没装、没开，或不在同一个局域网".to_string()
        } else {
            format!("已收到 {} 次广播", our_health.beacons_in)
        },
    ));

    let peers: Vec<PeerHandle> = app
        .try_state::<PeerRosterState>()
        .and_then(|state| {
            state
                .0
                .read()
                .ok()
                .map(|roster| roster.values().cloned().collect())
        })
        .unwrap_or_default();
    let online = peers
        .iter()
        .any(|peer| super::roster::is_online(peer, super::models::now_ms()));
    checks.push(check(
        "paired",
        "已配对设备",
        !peers.is_empty(),
        if peers.is_empty() {
            "还没有配对任何电脑：设置 → 设备与用户 → 生成配对码".to_string()
        } else {
            format!("已配对 {} 台", peers.len())
        },
    ));
    checks.push(check(
        "online",
        "在线设备",
        online,
        if online {
            "至少一台邻居正在广播".to_string()
        } else if peers.is_empty() {
            "没有可检测的邻居".to_string()
        } else {
            "配对的电脑都不在线（对方没开机，或发现了但服务关了）".to_string()
        },
    ));

    SelfCheck {
        ready: port != 0 && has_ip,
        checks,
    }
}

#[tauri::command]
pub fn run_network_selfcheck(app_handle: AppHandle) -> SelfCheck {
    run(&app_handle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_check_carries_its_own_name() {
        let item = check(
            "server",
            "传输服务",
            false,
            "正在监听 :28462".to_string(),
        );
        let json = serde_json::to_string(&item).unwrap();
        assert!(json.contains("\"key\":\"server\""));
        assert!(json.contains("\"label\":\"传输服务\""));
        assert!(json.contains("\"ok\":false"));
        assert!(json.contains("28462"));
    }

    #[test]
    fn an_unstarted_discovery_thread_reports_nothing_bound() {
        // The state is only managed once the app is up; outside an app the
        // self-check must still read as "nothing proved", not as a crash.
        let quiet = DiscoveryHealth::default();
        assert!(!quiet.v4_bound);
        assert_eq!(quiet.beacons_sent, 0);
        assert!(quiet.last_error.is_empty());
    }
}
