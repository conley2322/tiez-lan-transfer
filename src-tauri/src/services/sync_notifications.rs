//! Metadata-only WebDAV hints. They never enter the clipboard message path.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::VecDeque;

const NOTICE_NAMESPACE: &str = "tiez-sync/";
const MAX_NOTICE_BYTES: usize = 1024;
const RECENT_NOTICE_LIMIT: usize = 256;

pub fn notification_topic(mqtt_topic: &str) -> String {
    let root = mqtt_topic.trim().trim_end_matches('/');
    format!(
        "{NOTICE_NAMESPACE}v1/{:x}/changed",
        Sha256::digest(root.as_bytes())
    )
}

pub fn webdav_scope(url: &str, username: &str, base_path: &str) -> Option<String> {
    let mut url = reqwest::Url::parse(url.trim()).ok()?;
    if !matches!(url.scheme(), "http" | "https") {
        return None;
    }
    url.set_fragment(None);
    // Credentials are not part of the endpoint and never leave this process.
    let _ = url.set_username("");
    let _ = url.set_password(None);
    let target = format!(
        "{}\n{}\n{}",
        url.as_str().trim_end_matches('/'),
        username.trim(),
        base_path.trim().trim_matches('/')
    );
    Some(format!("{:x}", Sha256::digest(target.as_bytes())))
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SyncChangedNotice {
    v: u8,
    kind: String,
    scope: String,
    device_id: String,
    event_id: String,
}

pub fn encode_notice(scope: &str, device_id: &str) -> Result<Vec<u8>, serde_json::Error> {
    serde_json::to_vec(&SyncChangedNotice {
        v: 1,
        kind: "webdav_changed".into(),
        scope: scope.into(),
        device_id: device_id.into(),
        event_id: uuid::Uuid::new_v4().to_string(),
    })
}

#[derive(Debug, PartialEq, Eq)]
pub enum PublishRoute {
    Clipboard,
    SyncNotice,
    Ignore,
}

pub fn route_publish(topic: &str, own_notice_topic: &str) -> PublishRoute {
    if topic == own_notice_topic {
        PublishRoute::SyncNotice
    } else if topic.starts_with(NOTICE_NAMESPACE) {
        // Malformed and future notification versions must not become copied text.
        PublishRoute::Ignore
    } else {
        PublishRoute::Clipboard
    }
}

#[derive(Default)]
pub struct NoticeReceiver {
    recent: VecDeque<String>,
}

impl NoticeReceiver {
    pub fn accept(
        &mut self,
        payload: &[u8],
        retained: bool,
        scope: &str,
        own_device: &str,
    ) -> bool {
        if retained || payload.len() > MAX_NOTICE_BYTES {
            return false;
        }
        let Ok(notice) = serde_json::from_slice::<SyncChangedNotice>(payload) else {
            return false;
        };
        if notice.v != 1
            || notice.kind != "webdav_changed"
            || notice.scope != scope
            || notice.device_id == own_device
            || notice.device_id.is_empty()
            || notice.device_id.len() > 128
            || uuid::Uuid::parse_str(&notice.event_id).is_err()
            || self.recent.contains(&notice.event_id)
        {
            return false;
        }
        self.recent.push_back(notice.event_id);
        if self.recent.len() > RECENT_NOTICE_LIMIT {
            self.recent.pop_front();
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scope() -> String {
        webdav_scope("https://example.test/dav/", "user", "tiez-sync").unwrap()
    }

    #[test]
    fn channel_is_shared_but_outside_the_legacy_clipboard_subscription() {
        let topic = notification_topic("tiez/shared");
        assert_eq!(topic, notification_topic(" tiez/shared/ "));
        assert!(!topic.starts_with("tiez/shared/"));
        assert_ne!(topic, notification_topic("tiez/other"));
        assert_eq!(route_publish(&topic, &topic), PublishRoute::SyncNotice);
        assert_eq!(
            route_publish("tiez/shared/sms", &topic),
            PublishRoute::Clipboard
        );
        assert_eq!(
            route_publish("tiez-sync/v2/future", &topic),
            PublishRoute::Ignore
        );
        assert_eq!(
            route_publish(&notification_topic("other"), &topic),
            PublishRoute::Ignore
        );
    }

    #[test]
    fn scope_normalizes_the_endpoint_and_separates_accounts_and_folders() {
        assert_eq!(
            Some(scope()),
            webdav_scope(" HTTPS://EXAMPLE.TEST:443/dav ", "user", "/tiez-sync/")
        );
        assert_ne!(
            Some(scope()),
            webdav_scope("https://example.test/dav", "other", "tiez-sync")
        );
        assert_ne!(
            Some(scope()),
            webdav_scope("https://example.test/dav", "user", "other")
        );
        assert!(webdav_scope("file:///tmp/dav", "user", "tiez-sync").is_none());
    }

    #[test]
    fn peers_accept_once_and_ignore_self_echo_wrong_scope_and_retained_hints() {
        let mut receiver = NoticeReceiver::default();
        let payload = encode_notice(&scope(), "device-a").unwrap();
        assert!(!receiver.accept(&payload, false, &scope(), "device-a"));
        assert!(!receiver.accept(&payload, true, &scope(), "device-b"));
        assert!(!receiver.accept(&payload, false, "other", "device-b"));
        assert!(receiver.accept(&payload, false, &scope(), "device-b"));
        assert!(!receiver.accept(&payload, false, &scope(), "device-b"));
        assert!(receiver.accept(
            &encode_notice(&scope(), "device-a").unwrap(),
            false,
            &scope(),
            "device-b"
        ));
    }

    #[test]
    fn malformed_or_content_bearing_notices_are_rejected() {
        let mut receiver = NoticeReceiver::default();
        let payload = encode_notice(&scope(), "device-a").unwrap();
        let mut json: serde_json::Value = serde_json::from_slice(&payload).unwrap();
        json["content"] = "must not be copied".into();
        assert!(!receiver.accept(
            &serde_json::to_vec(&json).unwrap(),
            false,
            &scope(),
            "device-b"
        ));
        json.as_object_mut().unwrap().remove("content");
        json["v"] = 2.into();
        assert!(!receiver.accept(
            &serde_json::to_vec(&json).unwrap(),
            false,
            &scope(),
            "device-b"
        ));
        assert!(!receiver.accept(b"invalid json", false, &scope(), "device-b"));
        assert!(!receiver.accept(
            &vec![b' '; MAX_NOTICE_BYTES + 1],
            false,
            &scope(),
            "device-b"
        ));
        assert!(!String::from_utf8(payload).unwrap().contains("example.test"));
    }

    #[test]
    fn duplicate_cache_stays_bounded() {
        let mut receiver = NoticeReceiver::default();
        for _ in 0..RECENT_NOTICE_LIMIT + 10 {
            assert!(receiver.accept(
                &encode_notice(&scope(), "device-a").unwrap(),
                false,
                &scope(),
                "device-b"
            ));
        }
        assert_eq!(receiver.recent.len(), RECENT_NOTICE_LIMIT);
    }
}
