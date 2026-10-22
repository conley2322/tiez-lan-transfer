use crate::database::DbState;
use crate::global_state::{LAST_APP_SET_HASH, LAST_APP_SET_TIMESTAMP};
use crate::infrastructure::repository::settings_repo::SettingsRepository;
use crate::services::sync_notifications::{self, NoticeReceiver, PublishRoute};
use crate::{error, info};
use rumqttc::{AsyncClient, Event, Incoming, MqttOptions, QoS, Transport};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;
use tokio::time::sleep;

#[derive(Clone, Debug)]
struct MqttConfig {
    host: String,
    port: u16,
    username: Option<String>,
    password: Option<String>,
    topic: String,
    client_id: String,
    protocol: String,
    ssl: bool,
    ws_path: String,
    #[allow(dead_code)]
    tls_insecure: bool,
}

// Global MQTT client state
static MQTT_RUNNING: AtomicBool = AtomicBool::new(false);
static MQTT_CONNECTED: AtomicBool = AtomicBool::new(false);
static MQTT_RECONNECT_ATTEMPTS: AtomicU32 = AtomicU32::new(0);
/// A connection that lasted this long resets the reconnect backoff.
const STABLE_CONNECTION: Duration = Duration::from_secs(60);
/// rumqttc's default is 10 KB, which dropped long texts and tore down the
/// connection on every oversized (possibly retained) message.
const MAX_MQTT_PACKET: usize = 1024 * 1024;
static MQTT_TASK_ACTIVE: AtomicBool = AtomicBool::new(false);
static MQTT_GENERATION: AtomicU64 = AtomicU64::new(0);

struct NoticePublisher {
    client: AsyncClient,
    topic: String,
    generation: u64,
}

fn notice_publisher() -> &'static Mutex<Option<NoticePublisher>> {
    static PUBLISHER: OnceLock<Mutex<Option<NoticePublisher>>> = OnceLock::new();
    PUBLISHER.get_or_init(|| Mutex::new(None))
}

fn restart_signal() -> &'static tokio::sync::Notify {
    static SIGNAL: OnceLock<tokio::sync::Notify> = OnceLock::new();
    SIGNAL.get_or_init(tokio::sync::Notify::new)
}

fn clear_notice_publisher() {
    if let Ok(mut publisher) = notice_publisher().lock() {
        *publisher = None;
    }
}

fn default_client_id(device_id: &str) -> String {
    use sha2::{Digest, Sha256};
    let digest = format!("{:x}", Sha256::digest(device_id.as_bytes()));
    format!("tiez_{}", &digest[..16])
}

fn reconnect_delay(attempt: u32) -> Duration {
    // Clamp before exponentiation so a long offline period cannot overflow.
    Duration::from_secs((5u64 << attempt.saturating_sub(1).min(4)).min(60))
}

async fn wait_or_restart(delay: Duration, generation: u64) {
    if MQTT_GENERATION.load(Ordering::Relaxed) != generation {
        return;
    }
    tokio::select! {
        _ = restart_signal().notified() => {},
        _ = sleep(delay) => {},
    }
}

pub fn publish_webdav_changed(scope: &str, device_id: &str) {
    let Ok(payload) = sync_notifications::encode_notice(scope, device_id) else {
        return;
    };
    let Ok(publisher) = notice_publisher().lock() else {
        return;
    };
    if let Some(publisher) = publisher.as_ref() {
        if MQTT_CONNECTED.load(Ordering::Relaxed)
            && publisher.generation == MQTT_GENERATION.load(Ordering::Relaxed)
            && publisher
                .client
                .try_publish(&publisher.topic, QoS::AtLeastOnce, false, payload)
                .is_err()
        {
            info!(">>> [SYNC] MQTT update hint not queued; WebDAV polling will retry");
        }
    }
}

pub fn recover_mqtt_connection(app: AppHandle) {
    if get_mqtt_config(&app).is_some() {
        restart_mqtt_client(app);
    }
}

struct MqttTaskGuard;

impl Drop for MqttTaskGuard {
    fn drop(&mut self) {
        clear_notice_publisher();
        MQTT_CONNECTED.store(false, Ordering::Relaxed);
        MQTT_TASK_ACTIVE.store(false, Ordering::Relaxed);
    }
}

pub fn get_mqtt_status() -> bool {
    MQTT_CONNECTED.load(Ordering::Relaxed)
}

pub fn get_mqtt_running() -> bool {
    MQTT_RUNNING.load(Ordering::Relaxed)
}

// Force restart MQTT client by setting running flag to false
pub fn restart_mqtt_client(app: AppHandle) {
    info!(">>> [MQTT] Restart requested.");
    MQTT_RUNNING.store(false, Ordering::Relaxed);
    MQTT_CONNECTED.store(false, Ordering::Relaxed);
    MQTT_RECONNECT_ATTEMPTS.store(0, Ordering::Relaxed);
    MQTT_GENERATION.fetch_add(1, Ordering::Relaxed);
    clear_notice_publisher();
    restart_signal().notify_one();

    // 如果之前达到最大重连次数导致任务退出，重新启动任务
    if !MQTT_TASK_ACTIVE.load(Ordering::Relaxed) {
        start_mqtt_client(app);
    }
}

// Function to read config from DB
fn get_mqtt_config(app: &AppHandle) -> Option<MqttConfig> {
    let db_state = match app.try_state::<DbState>() {
        Some(s) => s,
        None => return None,
    };

    let enabled_str = db_state
        .settings_repo
        .get("mqtt_enabled")
        .ok()
        .flatten()
        .unwrap_or("false".to_string());
    if enabled_str != "true" {
        return None;
    }

    let host = db_state
        .settings_repo
        .get("mqtt_server")
        .ok()
        .flatten()
        .filter(|s| !s.is_empty());

    if host.is_none() || host.as_ref().unwrap().is_empty() {
        return None;
    }
    let host = host.unwrap();

    let port_str = db_state.settings_repo.get("mqtt_port").ok().flatten();
    let port = port_str.and_then(|p| p.parse::<u16>().ok()).unwrap_or(443);

    let password = db_state
        .settings_repo
        .get("mqtt_password")
        .ok()
        .flatten()
        .filter(|s| !s.is_empty());

    // Topic logic (using shortened ID)
    let stored_anon_id = db_state.settings_repo.get("app.anon_id").ok().flatten();
    let anon_id = stored_anon_id
        .as_deref()
        .and_then(crate::app::system::normalize_anon_id)
        .unwrap_or_else(
            || crate::app::system::build_anon_id(&crate::app::system::get_machine_id()),
        );
    if stored_anon_id
        .as_deref()
        .map(|value| value.trim() != anon_id)
        .unwrap_or(true)
    {
        let _ = db_state.settings_repo.set("app.anon_id", &anon_id);
    }
    let db_topic = db_state
        .settings_repo
        .get("mqtt_topic")
        .ok()
        .flatten()
        .filter(|s| !s.is_empty());
    // The topic is the only thing keeping strangers on a shared broker out, so
    // a new install gets an unguessable one (128 random bits) instead of the
    // first 8 hex digits of the anonymous id. Saved topics are kept: the phone
    // was paired with them.
    let topic = db_topic.unwrap_or_else(|| {
        let default_topic = format!("tiez/tiez_{}", uuid::Uuid::new_v4().simple());
        let _ = db_state.settings_repo.set("mqtt_topic", &default_topic);
        default_topic
    });

    let custom_client_id = db_state
        .settings_repo
        .get("mqtt_client_id")
        .ok()
        .flatten()
        .filter(|s| !s.is_empty());
    // Shared topics must not give different computers the same broker session ID.
    let client_id = custom_client_id.unwrap_or_else(|| default_client_id(&anon_id));

    let username = db_state
        .settings_repo
        .get("mqtt_username")
        .ok()
        .flatten()
        .filter(|s| !s.is_empty());

    let protocol = db_state
        .settings_repo
        .get("mqtt_protocol")
        .ok()
        .flatten()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| {
            if port == 8883 || port == 443 {
                "mqtts://".to_string()
            } else {
                "mqtt://".to_string()
            }
        });

    // Prefer the explicit protocol selection. `mqtt_ssl` is a legacy flag and
    // should not force TLS when the user selected mqtt:// or ws://.
    let legacy_ssl = db_state
        .settings_repo
        .get("mqtt_ssl")
        .ok()
        .flatten()
        .map(|s| s == "true");
    let ssl = if protocol.starts_with("mqtt://") || protocol.starts_with("ws://") {
        false
    } else if protocol.starts_with("mqtts://") || protocol.starts_with("wss://") {
        true
    } else {
        legacy_ssl.unwrap_or(port == 8883 || port == 443)
    };

    let ws_path = db_state
        .settings_repo
        .get("mqtt_ws_path")
        .ok()
        .flatten()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "/mqtt".to_string());

    let tls_insecure = db_state
        .settings_repo
        .get("mqtt_tls_insecure")
        .ok()
        .flatten()
        .map(|s| s == "true")
        .unwrap_or(false);

    Some(MqttConfig {
        host,
        port,
        username,
        password,
        topic,
        client_id,
        protocol,
        ssl,
        ws_path,
        tls_insecure,
    })
}

pub fn start_mqtt_client(app: AppHandle) {
    if MQTT_TASK_ACTIVE.swap(true, Ordering::Relaxed) {
        return;
    }

    tauri::async_runtime::spawn(async move {
        let _guard = MqttTaskGuard;
        let mut notice_receiver = NoticeReceiver::default();
        // Loop forever if enabled, using exponential backoff inside.

        loop {
            clear_notice_publisher();
            MQTT_CONNECTED.store(false, Ordering::Relaxed);
            let generation = MQTT_GENERATION.load(Ordering::Relaxed);
            let config = get_mqtt_config(&app);

            if let Some(cfg) = config {
                if !MQTT_RUNNING.load(Ordering::Relaxed) {
                    info!(">>> [MQTT] Enabling MQTT client");
                    MQTT_RUNNING.store(true, Ordering::Relaxed);
                    let _ = app.emit("mqtt-status", "connecting");
                }

                // Extract host from config (strip scheme, path, and optional port)
                let host_clean = cfg
                    .host
                    .trim_start_matches("wss://")
                    .trim_start_matches("ws://")
                    .trim_start_matches("mqtt://")
                    .trim_start_matches("mqtts://")
                    .split('/')
                    .next()
                    .unwrap_or(&cfg.host)
                    .to_string();
                let host_clean = if host_clean.starts_with('[') {
                    if let Some(idx) = host_clean.rfind("]:") {
                        let (h, p) = host_clean.split_at(idx + 1);
                        if p.trim_start_matches(':')
                            .chars()
                            .all(|c| c.is_ascii_digit())
                        {
                            h.to_string()
                        } else {
                            host_clean
                        }
                    } else {
                        host_clean
                    }
                } else if let Some((h, p)) = host_clean.rsplit_once(':') {
                    if p.chars().all(|c| c.is_ascii_digit()) {
                        h.to_string()
                    } else {
                        host_clean
                    }
                } else {
                    host_clean
                };

                let use_wss = cfg.protocol.starts_with("wss://");
                let use_tls = cfg.protocol.starts_with("mqtts://") || (cfg.ssl && !use_wss);
                let use_ws = cfg.protocol.starts_with("ws://");
                let ws_path = if cfg.ws_path.starts_with('/') {
                    cfg.ws_path.clone()
                } else {
                    format!("/{}", cfg.ws_path)
                };

                info!(
                    ">>> [MQTT] Connecting to '{}:{}' (Protocol: {}, ID: {})",
                    host_clean, cfg.port, cfg.protocol, cfg.client_id
                );

                // Build mqtt options. For websockets we must store a full URL in broker_addr.
                let mut mqttoptions = if use_wss {
                    let url = format!("wss://{}:{}{}", host_clean, cfg.port, ws_path);
                    MqttOptions::new(cfg.client_id.clone(), url, cfg.port)
                } else if use_ws {
                    let url = format!("ws://{}:{}{}", host_clean, cfg.port, ws_path);
                    MqttOptions::new(cfg.client_id.clone(), url, cfg.port)
                } else {
                    MqttOptions::new(cfg.client_id.clone(), host_clean.clone(), cfg.port)
                };

                // Build rustls config manually so MQTT TLS/WSS does not depend on
                // potentially problematic system certificate stores.
                if use_wss || use_tls {
                    let mut root_store = rustls::RootCertStore::empty();
                    root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

                    let mut config = rustls::ClientConfig::builder()
                        .with_root_certificates(root_store)
                        .with_no_client_auth();

                    if cfg.tls_insecure {
                        #[derive(Debug)]
                        struct NoVerifier;

                        impl rustls::client::danger::ServerCertVerifier for NoVerifier {
                            fn verify_server_cert(
                                &self,
                                _end_entity: &rustls::pki_types::CertificateDer<'_>,
                                _intermediates: &[rustls::pki_types::CertificateDer<'_>],
                                _server_name: &rustls::pki_types::ServerName<'_>,
                                _ocsp_response: &[u8],
                                _now: rustls::pki_types::UnixTime,
                            ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error>
                            {
                                Ok(rustls::client::danger::ServerCertVerified::assertion())
                            }

                            fn verify_tls12_signature(
                                &self,
                                _message: &[u8],
                                _cert: &rustls::pki_types::CertificateDer<'_>,
                                _dss: &rustls::DigitallySignedStruct,
                            ) -> Result<
                                rustls::client::danger::HandshakeSignatureValid,
                                rustls::Error,
                            > {
                                Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
                            }

                            fn verify_tls13_signature(
                                &self,
                                _message: &[u8],
                                _cert: &rustls::pki_types::CertificateDer<'_>,
                                _dss: &rustls::DigitallySignedStruct,
                            ) -> Result<
                                rustls::client::danger::HandshakeSignatureValid,
                                rustls::Error,
                            > {
                                Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
                            }

                            fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
                                vec![
                                    rustls::SignatureScheme::RSA_PKCS1_SHA1,
                                    rustls::SignatureScheme::ECDSA_SHA1_Legacy,
                                    rustls::SignatureScheme::RSA_PKCS1_SHA256,
                                    rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
                                    rustls::SignatureScheme::RSA_PKCS1_SHA384,
                                    rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
                                    rustls::SignatureScheme::RSA_PKCS1_SHA512,
                                    rustls::SignatureScheme::ECDSA_NISTP521_SHA512,
                                    rustls::SignatureScheme::RSA_PSS_SHA256,
                                    rustls::SignatureScheme::RSA_PSS_SHA384,
                                    rustls::SignatureScheme::RSA_PSS_SHA512,
                                    rustls::SignatureScheme::ED25519,
                                    rustls::SignatureScheme::ED448,
                                ]
                            }
                        }

                        config
                            .dangerous()
                            .set_certificate_verifier(std::sync::Arc::new(NoVerifier));
                    }

                    if use_wss {
                        mqttoptions.set_transport(Transport::wss_with_config(config.into()));
                    } else {
                        mqttoptions.set_transport(Transport::tls_with_config(config.into()));
                    }
                } else if use_ws {
                    mqttoptions.set_transport(Transport::ws());
                }

                mqttoptions.set_keep_alive(Duration::from_secs(30));

                if let Some(u) = cfg.username.clone() {
                    let p = cfg.password.clone().unwrap_or_default();
                    mqttoptions.set_credentials(u, p);
                }

                // Initialize client
                mqttoptions.set_max_packet_size(MAX_MQTT_PACKET, MAX_MQTT_PACKET);
                let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);

                let mut connected = false;
                let connecting = tokio::time::timeout(Duration::from_secs(30), async {
                    loop {
                        match eventloop.poll().await {
                            Ok(Event::Incoming(Incoming::ConnAck(_))) => {
                                info!(">>> [MQTT] Connection Established.");
                                MQTT_CONNECTED.store(true, Ordering::Relaxed);
                                // Attempts are reset only once a connection has
                                // proved stable (see the event-loop error below).
                                let _ = app.emit("mqtt-status", "connected");
                                return Ok::<(), rumqttc::ConnectionError>(());
                            }
                            Ok(_) => continue,
                            Err(e) => return Err(e),
                        }
                    }
                });
                let connect_result = tokio::select! {
                    biased;
                    _ = restart_signal().notified() => continue,
                    result = connecting => result,
                };
                if MQTT_GENERATION.load(Ordering::Relaxed) != generation {
                    continue;
                }

                match connect_result {
                    Ok(Ok(())) => connected = true,
                    Ok(Err(e)) => {
                        error!(">>> [MQTT] Connection error: {}", e);
                    }
                    Err(_) => {
                        error!(">>> [MQTT] Connection timeout.");
                    }
                }

                if !connected {
                    let current_attempts =
                        MQTT_RECONNECT_ATTEMPTS.fetch_add(1, Ordering::Relaxed) + 1;
                    MQTT_RUNNING.store(false, Ordering::Relaxed);
                    MQTT_CONNECTED.store(false, Ordering::Relaxed);
                    let _ = app.emit("mqtt-status", "disconnected");

                    // Cap backoff at 60 seconds
                    let delay = reconnect_delay(current_attempts);
                    info!(">>> [MQTT] Retrying in {}s (Attempt {})...", delay.as_secs(), current_attempts);
                    wait_or_restart(delay, generation).await;
                    continue;
                }

                let sub_topic = format!("{}/#", cfg.topic);
                if let Err(e) = client.subscribe(sub_topic.clone(), QoS::AtLeastOnce).await {
                    error!(">>> [MQTT] Subscribe failed: {:?}", e);
                    wait_or_restart(Duration::from_secs(2), generation).await;
                    continue;
                }
                info!(">>> [MQTT] Subscribed to {}", sub_topic);

                let notice_topic = sync_notifications::notification_topic(&cfg.topic);
                if client.try_subscribe(&notice_topic, QoS::AtLeastOnce).is_err() {
                    info!(">>> [SYNC] Update hint subscription not queued; using WebDAV polling");
                }
                if let Ok(mut publisher) = notice_publisher().lock() {
                    *publisher = Some(NoticePublisher {
                        client: client.clone(), topic: notice_topic.clone(), generation,
                    });
                }
                // Catch changes published while this computer was disconnected.
                crate::services::cloud_sync::request_cloud_sync(app.clone());
                let connected_at = Instant::now();

                loop {
                    // Restart/stop requested: drop this connection and let outer loop reconnect.
                    if !MQTT_RUNNING.load(Ordering::Relaxed)
                        || MQTT_GENERATION.load(Ordering::Relaxed) != generation {
                        info!(">>> [MQTT] Stop/restart requested. Leaving event loop.");
                        MQTT_CONNECTED.store(false, Ordering::Relaxed);
                        let _ = app.emit("mqtt-status", "disconnected");
                        let _ = client.try_disconnect();
                        break;
                    }

                    let poll_result = tokio::select! {
                        biased;
                        _ = restart_signal().notified() => break,
                        result = tokio::time::timeout(Duration::from_secs(5), eventloop.poll()) => result,
                    };
                    match poll_result {
                        Ok(event_result) => match event_result {
                            Ok(Event::Incoming(notification)) => match notification {
                                Incoming::Publish(publish) => {
                                    match sync_notifications::route_publish(&publish.topic, &notice_topic) {
                                        PublishRoute::Ignore => continue,
                                        PublishRoute::SyncNotice => {
                                            if let Some((scope, device_id)) = crate::services::cloud_sync::notification_identity(&app) {
                                                if notice_receiver.accept(&publish.payload, publish.retain, &scope, &device_id) {
                                                    crate::services::cloud_sync::request_cloud_sync(app.clone());
                                                }
                                            }
                                            continue;
                                        }
                                        PublishRoute::Clipboard => {},
                                    }
                                    // The broker redelivers a retained message on every
                                    // (re)connect; apply each one once.
                                    if publish.retain && !first_delivery_of_retained(&app, &publish.payload) {
                                        continue;
                                    }
                                    if let Ok(payload_str) = std::str::from_utf8(&publish.payload) {
                                        let opened;
                                        let payload_str = match open_e2e_payload(&app, payload_str) {
                                            Some(E2eOpened::Plain) => payload_str,
                                            Some(E2eOpened::Decrypted(text)) => {
                                                opened = text;
                                                opened.as_str()
                                            }
                                            None => continue,
                                        };
                                        let payload_trimmed = payload_str.trim();
                                        let final_content = if let Ok(json_val) =
                                            serde_json::from_str::<serde_json::Value>(
                                                payload_trimmed,
                                            ) {
                                            let content_fields =
                                                ["msg", "content", "body", "text", "message"];
                                            let mut found_content = None;
                                            for field in content_fields {
                                                if let Some(val) = json_val.get(field) {
                                                    if let Some(s) = val.as_str() {
                                                        found_content = Some(s.to_string());
                                                        break;
                                                    } else if val.is_number() || val.is_boolean() {
                                                        found_content = Some(val.to_string());
                                                        break;
                                                    }
                                                }
                                            }
                                            found_content
                                                .unwrap_or_else(|| payload_trimmed.to_string())
                                        } else {
                                            payload_trimmed.to_string()
                                        };

                                        apply_incoming_text(&app, final_content.clone());
                                        notify_incoming_message(&app, &final_content);
                                    }
                                }
                                Incoming::SubAck(ack) => {
                                    if ack.return_codes.iter().any(|code| matches!(code, rumqttc::SubscribeReasonCode::Failure)) {
                                        info!(">>> [MQTT] A subscription was refused; WebDAV polling remains available");
                                    }
                                }
                                Incoming::ConnAck(_) => {
                                    let _ = app.emit("mqtt-status", "connected");
                                }
                                _ => {}
                            },
                            Ok(Event::Outgoing(_)) => {}
                            Err(e) => {
                                error!(">>> [MQTT] Event loop error: {:?}", e);
                                MQTT_RUNNING.store(false, Ordering::Relaxed);
                                MQTT_CONNECTED.store(false, Ordering::Relaxed);
                                let _ = app.emit("mqtt-status", "disconnected");
                                // Back off before reconnecting. A connection that
                                // keeps dropping right after connecting (another
                                // client with this id, a broker kick, a packet it
                                // rejects) must not turn into a tight loop.
                                let attempts = if connected_at.elapsed() >= STABLE_CONNECTION {
                                    MQTT_RECONNECT_ATTEMPTS.store(1, Ordering::Relaxed);
                                    1
                                } else {
                                    MQTT_RECONNECT_ATTEMPTS.fetch_add(1, Ordering::Relaxed) + 1
                                };
                                let delay = reconnect_delay(attempts);
                                info!(">>> [MQTT] Reconnecting in {}s (Attempt {})...", delay.as_secs(), attempts);
                                wait_or_restart(delay, generation).await;
                                break;
                            }
                        },
                        Err(_) => {
                            // Timeout: check stop/restart flag and whether still enabled
                            if !MQTT_RUNNING.load(Ordering::Relaxed) {
                                info!(">>> [MQTT] Stop/restart requested during idle poll.");
                                MQTT_CONNECTED.store(false, Ordering::Relaxed);
                                let _ = app.emit("mqtt-status", "disconnected");
                                let _ = client.try_disconnect();
                                break;
                            }
                            if get_mqtt_config(&app).is_none() {
                                info!(">>> [MQTT] Disabled. Stopping task.");
                                MQTT_RUNNING.store(false, Ordering::Relaxed);
                                MQTT_CONNECTED.store(false, Ordering::Relaxed);
                                let _ = app.emit("mqtt-status", "disconnected");
                                return;
                            }
                        }
                    }
                }
            } else if MQTT_RUNNING.load(Ordering::Relaxed) {
                info!(">>> [MQTT] Configuration invalid or disabled.");
                MQTT_RUNNING.store(false, Ordering::Relaxed);
                let _ = app.emit("mqtt-status", "disconnected");
            }

            // Reconnect promptly after an explicit stop/restart signal.
            if !MQTT_RUNNING.load(Ordering::Relaxed) && get_mqtt_config(&app).is_some() {
                continue;
            }

            wait_or_restart(Duration::from_secs(5), generation).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[test]
    fn shared_topics_do_not_share_default_broker_session_ids() {
        assert_ne!(default_client_id("device-a"), default_client_id("device-b"));
        assert_eq!(default_client_id("device-a"), default_client_id("device-a"));
        assert!(default_client_id("device-a").len() <= 23);
    }

    #[test]
    fn reconnect_backoff_is_bounded_even_after_a_long_outage() {
        assert_eq!(reconnect_delay(1), Duration::from_secs(5));
        assert_eq!(reconnect_delay(2), Duration::from_secs(10));
        assert_eq!(reconnect_delay(4), Duration::from_secs(40));
        assert_eq!(reconnect_delay(5), Duration::from_secs(60));
        assert_eq!(reconnect_delay(u32::MAX), Duration::from_secs(60));
    }

    #[tokio::test]
    async fn restart_interrupts_the_backoff_and_stale_connections_do_not_wait() {
        let generation = MQTT_GENERATION.load(Ordering::Relaxed);
        let restart = async {
            tokio::task::yield_now().await;
            restart_signal().notify_one();
        };
        tokio::time::timeout(Duration::from_secs(1), async {
            tokio::join!(
                wait_or_restart(Duration::from_secs(60), generation),
                restart
            );
        })
        .await
        .unwrap();
        tokio::time::timeout(
            Duration::from_secs(1),
            wait_or_restart(Duration::from_secs(60), generation.wrapping_sub(1)),
        )
        .await
        .unwrap();
    }

    async fn read_packet(stream: &mut tokio::net::TcpStream) -> (u8, Vec<u8>) {
        let header = stream.read_u8().await.unwrap();
        let mut length = 0usize;
        let mut multiplier = 1;
        loop {
            let byte = stream.read_u8().await.unwrap();
            length += (byte as usize & 127) * multiplier;
            if byte & 128 == 0 {
                break;
            }
            multiplier *= 128;
            assert!(multiplier <= 128usize.pow(3));
        }
        assert!(length < 4096);
        let mut body = vec![0; length];
        stream.read_exact(&mut body).await.unwrap();
        (header, body)
    }

    #[tokio::test]
    async fn webdav_hint_is_published_as_non_retained_metadata_on_a_real_mqtt_connection() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let (release_broker, wait_for_ack) = tokio::sync::oneshot::channel();
        let broker = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            assert_eq!(read_packet(&mut stream).await.0 >> 4, 1); // CONNECT
            stream.write_all(&[0x20, 2, 0, 0]).await.unwrap(); // CONNACK
            let (header, body) = read_packet(&mut stream).await;
            assert_eq!(header, 0x32); // QoS 1, not retained, not a duplicate
            let topic_len = u16::from_be_bytes([body[0], body[1]]) as usize;
            let topic = String::from_utf8(body[2..2 + topic_len].to_vec()).unwrap();
            let id_offset = 2 + topic_len;
            let payload = body[id_offset + 2..].to_vec();
            stream
                .write_all(&[0x40, 2, body[id_offset], body[id_offset + 1]])
                .await
                .unwrap();
            // Keep the socket open until the client consumes PUBACK. Closing it
            // here would test an abrupt disconnect instead of successful delivery.
            wait_for_ack.await.unwrap();
            (topic, payload)
        });

        let options = MqttOptions::new(default_client_id("device-a"), "127.0.0.1", port);
        let (client, mut eventloop) = AsyncClient::new(options, 10);
        let topic = sync_notifications::notification_topic("tiez/shared");
        let scope =
            sync_notifications::webdav_scope("https://example.test/dav", "user", "tiez-sync")
                .unwrap();
        let result = tokio::time::timeout(Duration::from_secs(5), async {
            assert!(matches!(
                eventloop.poll().await.unwrap(),
                Event::Incoming(Incoming::ConnAck(_))
            ));
            let _cleanup = MqttTaskGuard;
            MQTT_CONNECTED.store(true, Ordering::Relaxed);
            *notice_publisher().lock().unwrap() = Some(NoticePublisher {
                client,
                topic: topic.clone(),
                generation: MQTT_GENERATION.load(Ordering::Relaxed),
            });
            publish_webdav_changed(&scope, "device-a");
            loop {
                if matches!(
                    eventloop.poll().await.unwrap(),
                    Event::Incoming(Incoming::PubAck(_))
                ) {
                    break;
                }
            }
            release_broker.send(()).unwrap();
            broker.await.unwrap()
        })
        .await
        .unwrap();
        assert_eq!(result.0, topic);
        assert_eq!(
            sync_notifications::route_publish(&result.0, &topic),
            PublishRoute::SyncNotice
        );
        let notice: serde_json::Value = serde_json::from_slice(&result.1).unwrap();
        assert_eq!(notice["scope"], scope);
        assert_eq!(notice["device_id"], "device-a");
        assert_eq!(notice.as_object().unwrap().len(), 5);
        let mut receiver = NoticeReceiver::default();
        assert!(receiver.accept(&result.1, false, &scope, "device-b"));
        assert!(!receiver.accept(&result.1, false, &scope, "device-b"));
    }
}

/// System notification for an incoming message. Sent from here rather than by
/// the WebView so it still arrives while light mode has destroyed the UI.
fn notify_incoming_message(app: &AppHandle, content: &str) {
    let Some(db) = app.try_state::<DbState>() else {
        return;
    };
    let enabled = db
        .settings_repo
        .get("mqtt_notification_enabled")
        .ok()
        .flatten()
        .map(|value| value != "false")
        .unwrap_or(true);
    if !enabled {
        return;
    }

    let message = content.split_whitespace().collect::<Vec<_>>().join(" ");
    if message.is_empty() {
        return;
    }

    // The same text often arrives twice in a row (retained + live, resends).
    static LAST_NOTIFIED: OnceLock<Mutex<Option<(String, Instant)>>> = OnceLock::new();
    {
        let mut last = LAST_NOTIFIED
            .get_or_init(|| Mutex::new(None))
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some((previous, at)) = last.as_ref() {
            if *previous == message && at.elapsed() < Duration::from_secs(10) {
                return;
            }
        }
        *last = Some((message.clone(), Instant::now()));
    }

    // Same wording as mqtt_notification_title in src/locales.ts.
    let title = match db.settings_repo.get("app.language").ok().flatten().as_deref() {
        Some("en") => "New MQTT Message",
        Some("tw") => "收到新訊息",
        _ => "收到新消息",
    };
    let body = if message.chars().count() <= 64 {
        message
    } else {
        format!("{}...", message.chars().take(61).collect::<String>())
    };
    if let Err(err) = app.notification().builder().title(title).body(body).show() {
        error!(">>> [MQTT] Notification failed: {}", err);
    }
}

/// Incoming texts go through one worker so they reach the clipboard in the
/// order they arrived (a blocking task each could finish out of order).
fn apply_incoming_text(app: &AppHandle, text: String) {
    static WORKER: OnceLock<Mutex<std::sync::mpsc::Sender<(AppHandle, String)>>> = OnceLock::new();
    let sender = WORKER.get_or_init(|| {
        let (tx, rx) = std::sync::mpsc::channel::<(AppHandle, String)>();
        std::thread::spawn(move || {
            for (app, text) in rx {
                write_incoming_text(&app, text);
            }
        });
        Mutex::new(tx)
    });
    let _ = sender
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .send((app.clone(), text));
}

fn write_incoming_text(app: &AppHandle, text: String) {
    let normalized = text.trim().replace("\r\n", "\n");
    let mut hasher = DefaultHasher::new();
    normalized.hash(&mut hasher);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    LAST_APP_SET_HASH.store(hasher.finish(), Ordering::SeqCst);
    LAST_APP_SET_TIMESTAMP.store(now, Ordering::SeqCst);
    if let Ok(mut clipboard) = arboard::Clipboard::new() {
        for _ in 0..3 {
            if clipboard.set_text(text.clone()).is_ok() {
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    crate::services::clipboard::process_new_entry(
        app,
        crate::services::clipboard::ClipboardData::Text(text),
        Some("mqtt".to_string()),
    );
}

/// Remembers the last retained payload applied (across restarts), so a stale
/// retained message can't overwrite the clipboard again after every wake or
/// settings change, while a new one sent during downtime still gets through.
fn first_delivery_of_retained(app: &AppHandle, payload: &[u8]) -> bool {
    use sha2::{Digest, Sha256};
    let Some(db) = app.try_state::<DbState>() else {
        return false;
    };
    let digest = format!("{:x}", Sha256::digest(payload));
    let key = "mqtt_last_retained_sha256";
    if db.settings_repo.get(key).ok().flatten().as_deref() == Some(digest.as_str()) {
        return false;
    }
    let _ = db.settings_repo.set(key, &digest);
    true
}

enum E2eOpened {
    Plain,
    Decrypted(String),
}

/// With an end-to-end key set, only fresh messages sealed with it get
/// through (strangers on the broker can neither read nor inject). Without
/// one, sealed messages are dropped: there is nothing to open them with.
fn open_e2e_payload(app: &AppHandle, payload: &str) -> Option<E2eOpened> {
    use crate::services::e2e;
    match e2e::load_keys(app) {
        Some(keys) => {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as i64;
            match keys.open_mqtt_text(payload, now) {
                Ok(text) => Some(E2eOpened::Decrypted(text)),
                Err(reason) => {
                    info!(">>> [MQTT] Dropped a message: end-to-end check failed ({:?})", reason);
                    None
                }
            }
        }
        None if e2e::is_sealed_mqtt_text(payload) => {
            info!(">>> [MQTT] Dropped an encrypted message: no end-to-end key set");
            None
        }
        None => Some(E2eOpened::Plain),
    }
}
