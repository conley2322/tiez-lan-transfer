//! The roster the window shows, the pairing code, and the verify word.
//!
//! A pairing code is a URL, not a short PIN: an attacker with LAN access
//! could try millions of 6-digit codes against an open port, but cannot
//! guess a 128-bit token and a 256-bit key. The cost is a long string, so
//! both sides also confirm a two-word phrase computed from exactly what the
//! code contained — the same trick LocalSend uses to catch a spoofed code.

use tauri::{AppHandle, Emitter, Manager};

use sha2::{Digest, Sha256};

use crate::services::file_transfer::handlers::is_format_char;
use crate::services::file_transfer::sealed::{self, LanKey};

use super::models::{PeerHandle, PeerIdentity, PeerRosterState, PeerView};

/// Display text a peer claims about itself. `char::is_control()` alone is
/// not enough: U+202E is a formatting mark, invisible, and would render
/// "录音\u{202E}gpj.exe" as "录音exe.jpg" in every list we show.
pub fn clean_text(value: &str, max: usize) -> String {
    value
        .chars()
        .filter(|ch| !ch.is_control() && !is_format_char(*ch))
        .take(max)
        .collect::<String>()
        .trim()
        .to_string()
}

const ADJECTIVES: [&str; 32] = [
    "赤色", "金色", "青色", "紫色", "银色", "暖橙", "冰蓝", "墨绿",
    "雪白", "桃粉", "靛蓝", "琥珀", "石灰", "柳绿", "杏黄", "暮紫",
    "晨曦", "夜蓝", "深海", "松绿", "麦金", "樱粉", "竹青", "霞红",
    "雾灰", "山黛", "海浪", "星火", "月白", "雷鸣", "风信", "雨清",
];

const NOUNS: [&str; 32] = [
    "猎户", "白泽", "青龙", "朱雀", "玄武", "麒麟", "貔貅", "山猫",
    "海豹", "雪鸮", "游隼", "赤狐", "灰鹤", "锦鲤", "熊猫", "考拉",
    "河狸", "麋鹿", "羚羊", "海豚", "松鸦", "云雀", "刺猬", "蜜獾",
    "貂熊", "猞猁", "燕鸥", "角马", "水獭", "沙丘", "星轨", "云杉",
];

/// The two-machine phrase. Sorted inputs, so neither side's order matters.
pub fn verify_word(ours: (&str, &LanKey), theirs: (&str, &LanKey)) -> String {
    let first: (String, Vec<u8>);
    let second: (String, Vec<u8>);
    if ours.0 <= theirs.0 {
        first = (ours.0.to_string(), ours.1.to_vec());
        second = (theirs.0.to_string(), theirs.1.to_vec());
    } else {
        first = (theirs.0.to_string(), theirs.1.to_vec());
        second = (ours.0.to_string(), ours.1.to_vec());
    }
    let mut hasher = Sha256::new();
    hasher.update(b"tiez-pair-v1");
    hasher.update(first.0.as_bytes());
    hasher.update(&first.1);
    hasher.update(second.0.as_bytes());
    hasher.update(&second.1);
    let digest = hasher.finalize();
    let adjective = ADJECTIVES[digest[0] as usize % ADJECTIVES.len()];
    let noun = NOUNS[digest[1] as usize % NOUNS.len()];
    format!("{adjective}-{noun}")
}

/// What a machine pastes or scans to pair. The fragment is stripped by
/// browsers, but this is read by our own window, so plain query is fine and
/// survives every "copy text" path.
pub fn build_pairing_code(identity: &PeerIdentity, ip: &str, port: u16) -> String {
    format!(
        "tiez://pair?v=1&id={}&name={}&nick={}&plat={}&ip={}&port={}&auth={}&key={}",
        identity.device_id,
        urlencoding::encode(&identity.device_name),
        urlencoding::encode(&identity.nickname),
        identity.platform,
        ip,
        port,
        identity.auth_token,
        sealed::encode_key(&identity.key)
    )
}

/// The other half of a pairing: everything needed to push files to that
/// machine, and to prove it is the one we just showed a code for.
#[derive(Debug, Clone, PartialEq)]
pub struct PairTarget {
    pub device_id: String,
    pub device_name: String,
    pub nickname: String,
    pub platform: String,
    pub ip: String,
    pub port: u16,
    pub auth_token: String,
    pub key: LanKey,
}

fn query_pairs(text: &str) -> Vec<(String, String)> {
    text.split('&')
        .filter_map(|part| part.split_once('='))
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect()
}

pub fn parse_pairing_code(code: &str) -> Result<PairTarget, String> {
    let trimmed = code.trim();
    let query = trimmed
        .strip_prefix("tiez://pair?")
        .or_else(|| trimmed.strip_prefix("TIEZ://PAIR?"))
        .ok_or_else(|| "这不是 TieZ 的配对码".to_string())?;
    let params = query_pairs(query);
    let get = |key: &str| -> Option<String> {
        params
            .iter()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value.clone())
    };
    let device_id = get("id").unwrap_or_default();
    if device_id.len() < 8 || !device_id.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err("配对码里的设备编号不完整".to_string());
    }
    let ip = get("ip").unwrap_or_default();
    if ip.is_empty() || ip.len() > 128 || !ip.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | ':' | '-' | '_')) {
        return Err("配对码里的地址无效".to_string());
    }
    let port: u16 = get("port")
        .and_then(|text| text.parse().ok())
        .filter(|port| *port > 0)
        .ok_or_else(|| "配对码里的端口无效".to_string())?;
    let auth_token = get("auth").unwrap_or_default();
    if auth_token.len() < 16 {
        return Err("配对码里的密钥不完整".to_string());
    }
    let key = sealed::decode_key(&get("key").unwrap_or_default())
        .ok_or_else(|| "配对码里的传输密钥无效".to_string())?;
    let decode_field = |key_name: &str, max: usize| -> String {
        let raw = get(key_name).unwrap_or_default();
        clean_text(
            &urlencoding::decode(&raw).map(|v| v.into_owned()).unwrap_or(raw),
            max,
        )
    };
    Ok(PairTarget {
        device_id,
        device_name: decode_field("name", 80),
        nickname: decode_field("nick", 40),
        platform: decode_field("plat", 20),
        ip,
        port,
        auth_token,
        key,
    })
}

/// The roster as the windows see it: ⭐ first, then in pairing order, so every
/// list in the app agrees on which machine the user cares about most.
pub fn views(app: &AppHandle) -> Vec<PeerView> {
    let state = app.state::<PeerRosterState>();
    let mut peers: Vec<PeerHandle> = state.0.read().unwrap().values().cloned().collect();
    peers.sort_by(|a, b| {
        a.favorite
            .cmp(&b.favorite)
            .reverse()
            .then(a.created_at.cmp(&b.created_at))
            .then_with(|| a.device_id.cmp(&b.device_id))
    });
    peers.iter().map(PeerHandle::view).collect()
}

/// Beacon freshness: a peer that stopped announcing is shown as offline, but
/// stays in the roster because its address is still worth trying.
pub const ONLINE_WINDOW_MS: i64 = 12_000;

/// The rows of the screen-corner drop box: 「仅收藏用户」 leaves out everything
/// without a star. The order is `views`' own, so a starred machine is the first
/// thing the user drags onto either way.
pub fn corner_views(app: &AppHandle, favorites_only: bool) -> Vec<PeerView> {
    let peers = views(app);
    if !favorites_only {
        return peers;
    }
    peers.into_iter().filter(|peer| peer.favorite).collect()
}

pub fn is_online(peer: &PeerHandle, now_ms: i64) -> bool {
    peer.last_seen > 0 && now_ms - peer.last_seen <= ONLINE_WINDOW_MS
}

pub fn notify_peers_changed(app: &AppHandle) {
    let _ = app.emit("peers-changed", views(app));
    // The drop box lists a subset of the same roster and has to re-read it: a
    // star added in 设置 is a new row in the corner, and removing a machine is
    // one fewer place to drop a file.
    crate::app::corner_box::roster_changed(app);
    // On Windows the same roster is a right-click menu, and the only way it
    // stays true to the list on this screen is to be rewritten with it.
    #[cfg(windows)]
    if let Err(error) = crate::services::peer_transfer::win_menu::sync(app) {
        eprintln!(">>> [WINMENU] {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target() -> PairTarget {
        PairTarget {
            device_id: "0123456789abcdef".to_string(),
            device_name: "Conley 的 Mac".to_string(),
            nickname: String::new(),
            platform: "macos".to_string(),
            ip: "192.168.1.20".to_string(),
            port: 28462,
            auth_token: "pt_0123456789abcdef".to_string(),
            key: [9u8; 32],
        }
    }

    fn code() -> String {
        let identity = PeerIdentity {
            device_id: "0123456789abcdef".to_string(),
            device_name: "Conley 的 Mac".to_string(),
            nickname: "书房".to_string(),
            platform: "macos".to_string(),
            avatar_color: String::new(),
            auth_token: "pt_0123456789abcdef".to_string(),
            key: [9u8; 32],
        };
        build_pairing_code(&identity, "192.168.1.20", 28462)
    }

    #[test]
    fn a_pairing_code_round_trips() {
        let parsed = parse_pairing_code(&code()).expect("parse");
        assert_eq!(parsed.device_id, "0123456789abcdef");
        assert_eq!(parsed.device_name, "Conley 的 Mac");
        assert_eq!(parsed.nickname, "书房");
        assert_eq!(parsed.ip, "192.168.1.20");
        assert_eq!(parsed.port, 28462);
        assert_eq!(parsed.auth_token, "pt_0123456789abcdef");
        assert_eq!(parsed.key, [9u8; 32]);
    }

    #[test]
    fn surrounding_spaces_are_forgiven() {
        let parsed = parse_pairing_code(&format!("  {}\n", code())).expect("parse");
        assert_eq!(parsed.port, 28462);
    }

    #[test]
    fn junk_is_rejected() {
        for case in [
            "",
            "http://192.168.1.5:28462/?auth=x#k=y",
            "tiez://pair?v=1&id=zzzzzzzz&ip=1.2.3.4&port=1&auth=0123456789abcdef&key=a",
            "tiez://pair?v=1&id=0123456789abcdef&ip=../../etc&port=1&auth=0123456789abcdef&key=a",
            "tiez://pair?v=1&id=0123456789abcdef&ip=1.2.3.4&port=0&auth=0123456789abcdef&key=a",
            "tiez://pair?v=1&id=0123456789abcdef&ip=1.2.3.4&port=9&auth=short&key=a",
            "tiez://pair?v=1&id=0123456789abcdef&ip=1.2.3.4&port=9&auth=0123456789abcdef&key=nope!",
        ] {
            assert!(parse_pairing_code(case).is_err(), "accepted {case:?}");
        }
    }

    #[test]
    fn invisible_marks_never_survive_a_pairing_code() {
        let identity = PeerIdentity {
            device_id: "0123456789abcdef".to_string(),
            device_name: "ab\u{202e}cd".to_string(),
            nickname: "\u{2066}\u{200b} 书房 \u{feff}".to_string(),
            platform: "mac\u{7}os".to_string(),
            avatar_color: String::new(),
            auth_token: "pt_0123456789abcdef".to_string(),
            key: [9u8; 32],
        };
        let parsed = parse_pairing_code(&build_pairing_code(&identity, "192.168.1.20", 28462))
            .expect("parse");
        assert_eq!(parsed.device_name, "abcd");
        assert_eq!(parsed.nickname, "书房");
        assert_eq!(parsed.platform, "macos");
    }

    #[test]
    fn verify_word_ignores_which_side_computes_it() {
        let ours = ("0123456789abcdef", &[1u8; 32] as &LanKey);
        let theirs = ("fedcba9876543210", &[2u8; 32] as &LanKey);
        assert_eq!(verify_word(ours, theirs), verify_word(theirs, ours));
        assert!(verify_word(ours, theirs).contains('-'));
        // A different key on either side is a different phrase.
        assert_ne!(
            verify_word(ours, theirs),
            verify_word(("0123456789abcdef", &[3u8; 32]), theirs)
        );
    }

    #[test]
    fn beacons_only_mark_a_peer_online_for_a_short_window() {
        let handle = PeerHandle {
            device_id: "x".to_string(),
            device_name: "x".to_string(),
            nickname: String::new(),
            platform: "macos".to_string(),
            ip: "1.2.3.4".to_string(),
            port: 1,
            auth_token: "a".to_string(),
            key: [0u8; 32],
            trusted: true,
            pair_source: "pair".to_string(),
            avatar_color: String::new(),
            created_at: 0,
            last_seen: 100_000,
            favorite: false,
            name_locked: false,
        };
        assert!(is_online(&handle, 100_000 + ONLINE_WINDOW_MS));
        assert!(!is_online(&handle, 100_000 + ONLINE_WINDOW_MS + 1));
        let never = PeerHandle {
            last_seen: 0,
            ..handle
        };
        assert!(!is_online(&never, 1_000_000));
    }
}
