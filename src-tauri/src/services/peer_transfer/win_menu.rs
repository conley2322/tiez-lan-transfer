//! Windows has no corner drop box, so the Explorer submenu written here is the
//! one way a file leaves this machine.
//!
//! Two halves that have to stay in agreement. `sync` rebuilds the submenu from
//! the roster — one item per paired device, names only, because avatars belong
//! to the settings screen — and `handle_args` performs the send when Explorer
//! launches a menu command. Explorer launches it *once per selected file*, so a
//! five-file selection arrives as five process starts and is merged back into a
//! single batch before it reaches the transfer queue.

use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use tauri::{AppHandle, Manager};
use tauri_plugin_notification::NotificationExt;

use crate::database::DbState;
use crate::infrastructure::repository::settings_repo::SettingsRepository;

use super::{client, roster};

/// 设置 → 右键菜单. It starts on: on Windows this is the only send path, so a
/// switch that began off would hide the feature behind a checkbox.
pub const SETTING_KEY: &str = "win_context_menu_enabled";

/// `"<exe>" --win-send "<deviceId>" "%1"` — Explorer fills in `%1`.
const SEND_FLAG: &str = "--win-send";

/// Long enough for Explorer's per-file launches to land, short enough that the
/// progress card still feels like it answered the click.
const COLLECT_WINDOW: Duration = Duration::from_millis(400);

/// One submenu row. `key` is the registry key name and the id handed back on the
/// command line, so it never comes from a label a user typed.
struct MenuItem {
    key: String,
    label: String,
}

struct Job {
    device_id: String,
    paths: Vec<String>,
}

fn setting(app: &AppHandle, key: &str) -> Option<String> {
    let state = app.try_state::<DbState>()?;
    state.settings_repo.get(key).ok().flatten()
}

/// Off only when the user said so; a missing row is the default, not "disabled".
pub fn is_enabled(app: &AppHandle) -> bool {
    setting(app, SETTING_KEY).as_deref() != Some("false")
}

pub fn set_enabled(app: &AppHandle, enabled: bool) -> Result<(), String> {
    let value = if enabled { "true" } else { "false" };
    let state = app
        .try_state::<DbState>()
        .ok_or_else(|| "设置存储不可用".to_string())?;
    state
        .settings_repo
        .set(SETTING_KEY, value)
        .map_err(|error| format!("写入设置失败：{error}"))?;
    sync(app)
}

/// The submenu title follows 语言, because it is the one string this machine
/// writes for itself; the rows underneath are the peer names the user chose.
fn outer_title(app: &AppHandle) -> &'static str {
    match setting(app, "app.language").as_deref() {
        Some("en") => "Send with TieZ",
        Some("tw") => "傳送至 TieZ",
        _ => "发送给 TieZ",
    }
}

/// Our own device ids are 16 hex characters. The id goes into a registry key
/// name and into a command line Explorer will hand to `CreateProcess`, so
/// anything else is refused rather than quoted and hoped for.
fn is_device_key(id: &str) -> bool {
    id.len() >= 8 && id.len() <= 64 && id.chars().all(|c| c.is_ascii_hexdigit())
}

fn menu_items(app: &AppHandle) -> Vec<MenuItem> {
    roster::views(app)
        .into_iter()
        // Same rule as the corner box: only machines this one has paired with.
        // A discovered neighbour stays on the 设备与用户 page, but a row in
        // Explorer's menu that fails when clicked is worse than no row.
        .filter(|peer| peer.trusted)
        .filter_map(|peer| {
            if !is_device_key(&peer.device_id) {
                return None;
            }
            let name = if peer.nickname.is_empty() {
                peer.device_name
            } else {
                peer.nickname
            };
            let label = roster::clean_text(&name, 40);
            if label.is_empty() {
                return None;
            }
            // `&` is a menu accelerator in a shell label; doubled it reads back
            // as the character itself.
            Some(MenuItem {
                key: peer.device_id,
                label: label.replace('&', "&&"),
            })
        })
        .collect()
}

/// What the menu holds right now, read back from the registry instead of from
/// the roster — the settings page shows this so 「开」 means 「写进去了」.
pub fn entries() -> Vec<String> {
    shell::entries().unwrap_or_default()
}

/// The last submenu this process wrote, so a beacon that only refreshed an
/// address does not rewrite the registry every few seconds.
static LAST_SYNCED: Mutex<String> = Mutex::new(String::new());

/// Rebuild the submenu when — and only when — it would no longer match. Called
/// at startup, whenever the roster changes, and whenever a setting it reads
/// changes.
pub fn sync(app: &AppHandle) -> Result<(), String> {
    let exe = match std::env::current_exe() {
        Ok(path) => path.to_string_lossy().into_owned(),
        Err(error) => return Err(format!("找不到本机程序路径：{error}")),
    };
    let title = outer_title(app);
    let items = if is_enabled(app) {
        menu_items(app)
    } else {
        vec![]
    };
    let signature = shell::signature(&exe, title, &items);
    let unchanged = LAST_SYNCED
        .lock()
        .map(|last| *last == signature)
        .unwrap_or(false);
    if unchanged {
        return Ok(());
    }
    shell::write(&exe, title, &items)?;
    if let Ok(mut last) = LAST_SYNCED.lock() {
        *last = signature;
    }
    Ok(())
}

/// Start on Windows: put the menu in place, and run the send if this start was
/// itself a right-click (the app was not running when the user clicked).
pub fn startup(app: &AppHandle) {
    if let Err(error) = sync(app) {
        eprintln!(">>> [WINMENU] {error}");
    }
    let args: Vec<String> = std::env::args().collect();
    handle_args(app, &args);
}

/// True when these arguments were a send, so the caller knows not to open the
/// settings window for what is really a file leaving the machine.
pub fn handle_args(app: &AppHandle, args: &[String]) -> bool {
    let Some(job) = parse_args(args) else {
        return false;
    };
    enqueue(app, job);
    true
}

fn parse_args(args: &[String]) -> Option<Job> {
    let at = args.iter().position(|arg| arg == SEND_FLAG)?;
    let device_id = args.get(at + 1)?.clone();
    if !is_device_key(&device_id) {
        return None;
    }
    let paths: Vec<String> = args
        .iter()
        .skip(at + 2)
        .map(|path| path.trim().to_string())
        .filter(|path| !path.is_empty())
        .collect();
    if paths.is_empty() {
        return None;
    }
    Some(Job { device_id, paths })
}

static APP: OnceLock<AppHandle> = OnceLock::new();
static SENDER: OnceLock<Mutex<Option<Sender<Job>>>> = OnceLock::new();

fn enqueue(app: &AppHandle, job: Job) {
    let _ = APP.set(app.clone());
    let slot = SENDER.get_or_init(|| Mutex::new(None));
    let mut guard = match slot.lock() {
        Ok(guard) => guard,
        Err(_) => return,
    };
    if guard.is_none() {
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || collect(rx));
        *guard = Some(tx);
    }
    let Some(tx) = guard.as_ref() else {
        return;
    };
    if tx.send(job).is_err() {
        eprintln!(">>> [WINMENU] 发送队列线程已退出");
    }
}

/// One thread, alive for as long as the app: it holds a batch open while more
/// files from the same click keep arriving, then hands it to the queue.
fn collect(rx: Receiver<Job>) {
    loop {
        let Ok(first) = rx.recv() else { return };
        let mut batch = first;
        loop {
            match rx.recv_timeout(COLLECT_WINDOW) {
                Ok(next) if next.device_id == batch.device_id => {
                    for path in next.paths {
                        if !batch.paths.contains(&path) {
                            batch.paths.push(path);
                        }
                    }
                }
                Ok(next) => {
                    flush(batch);
                    batch = next;
                }
                Err(_) => {
                    flush(batch);
                    break;
                }
            }
        }
    }
}

fn flush(job: Job) {
    let Some(app) = APP.get() else {
        return;
    };
    if let Err(error) = client::send(app, vec![job.device_id], job.paths, None) {
        // Nothing was on screen to receive this click, so the only honest way to
        // say it did not start is a notification.
        eprintln!(">>> [WINMENU] {error}");
        let _ = app
            .notification()
            .builder()
            .title("发送失败")
            .body(error)
            .show();
    }
}

/// The registry itself. Two locations: one for files, one for folders, each with
/// the same verb key and the same per-device rows under it.
mod shell {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    use super::MenuItem;

    /// Key name, not shown to the user; the display text is the default value.
    const VERB_KEY: &str = "TieZSend";

    const PLACES: [&str; 2] = [
        r"Software\Classes\*\shell",
        r"Software\Classes\Directory\shell",
    ];

    /// Rows would otherwise look unchanged after a rename, and the next write
    /// would be skipped as "already in sync".
    pub fn signature(exe: &str, title: &str, items: &[MenuItem]) -> String {
        let mut text = format!("{exe}\u{1}{title}");
        for item in items {
            text.push_str(&format!("\u{1}{}\u{1}{}", item.key, item.label));
        }
        text
    }

    pub fn write(exe: &str, title: &str, items: &[MenuItem]) -> Result<(), String> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        for place in PLACES {
            let path = format!("{place}\\{VERB_KEY}");
            // Rebuild rather than patch: a device that left the roster has to
            // leave the menu too, and only a deleted key guarantees that.
            let _ = hkcu.delete_subkey_all(&path);
            if items.is_empty() {
                continue;
            }
            let (verb, _) = hkcu
                .create_subkey(&path)
                .map_err(|error| format!("写入右键菜单失败：{error}"))?;
            write_default(&verb, title)?;
            verb.set_value("Icon", &format!("\"{exe}\",0"))
                .map_err(|error| format!("写入右键菜单图标失败：{error}"))?;
            let rows = verb
                .create_subkey("shell")
                .map_err(|error| format!("写入右键子菜单失败：{error}"))?
                .0;
            for item in items {
                let entry = rows
                    .create_subkey(&item.key)
                    .map_err(|error| format!("写入 {} 失败：{error}", item.key))?
                    .0;
                write_default(&entry, &item.label)?;
                entry
                    .set_value("Icon", &format!("\"{exe}\",0"))
                    .map_err(|error| format!("写入右键菜单图标失败：{error}"))?;
                let command = entry
                    .create_subkey("command")
                    .map_err(|error| format!("写入 {} 的命令失败：{error}", item.key))?;
                let line = format!("\"{exe}\" {} \"{}\" \"%1\"", super::SEND_FLAG, item.key);
                command
                    .0
                    .set_value("", &line)
                    .map_err(|error| format!("写入 {} 的命令失败：{error}", item.key))?;
            }
        }
        notify_explorer();
        Ok(())
    }

    fn write_default(key: &RegKey, text: &str) -> Result<(), String> {
        key.set_value("", &text.to_string())
            .map_err(|error| format!("写入右键菜单名称失败：{error}"))
    }

    /// Labels under the files location. `Ok` even when empty: an empty submenu
    /// is a real state the page should be able to show.
    pub fn entries() -> Result<Vec<String>, String> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let path = format!("{}\\{VERB_KEY}\\shell", PLACES[0]);
        let rows = match hkcu.open_subkey(&path) {
            Ok(rows) => rows,
            // Nothing written yet is not an error; it is an empty submenu.
            Err(_) => return Ok(vec![]),
        };
        let mut labels = vec![];
        for name in rows.enum_keys().flatten() {
            let Ok(entry) = rows.open_subkey(&name) else {
                continue;
            };
            let label: String = entry.get_value("").unwrap_or_default();
            // Undo the menu-accelerator escape, so the page prints the name the
            // user typed and not the bytes Explorer was given.
            let label = label.replace("&&", "&");
            if !label.is_empty() {
                labels.push(label);
            }
        }
        labels.sort();
        Ok(labels)
    }

    /// Explorer caches the verb lists it read from the registry. This is the
    /// documented way to say the shell associations changed; without it a rename
    /// shows up in the menu only after Explorer restarts.
    fn notify_explorer() {
        use windows::Win32::UI::Shell::{SHChangeNotify, SHCNE_ASSOCCHANGED, SHCNF_IDLIST};
        unsafe { SHChangeNotify(SHCNE_ASSOCCHANGED, SHCNF_IDLIST, None, None) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(tail: &[&str]) -> Vec<String> {
        std::iter::once(r"C:\Programs\TieZ.exe".to_string())
            .chain(tail.iter().map(|arg| arg.to_string()))
            .collect()
    }

    #[test]
    fn a_menu_launch_is_read_back_as_one_peer_and_its_paths() {
        let job = parse_args(&args(&[
            "--win-send",
            "0123456789abcdef",
            r"C:\Users\Conley\Desktop\报告.pdf",
        ]))
        .expect("the command line Explorer builds");
        assert_eq!(job.device_id, "0123456789abcdef");
        assert_eq!(
            job.paths,
            vec![r"C:\Users\Conley\Desktop\报告.pdf".to_string()]
        );
    }

    #[test]
    fn a_login_start_is_not_a_send() {
        assert!(parse_args(&args(&["--minimized"])).is_none());
        assert!(parse_args(&args(&[])).is_none());
    }

    /// The id becomes a registry key name and part of a command line, so a value
    /// that is not one of ours must never reach either.
    #[test]
    fn only_our_own_device_ids_are_accepted() {
        let long = "0".repeat(65);
        for bad in [
            "",
            "0123",
            long.as_str(),
            r#"0123456789ab" \"C:\windows\system32\calc.exe\""#,
            "0123456789abcg",
            r"0123\456789abcdef",
        ] {
            assert!(
                !is_device_key(bad),
                "a registry key name may not be built from {bad:?}"
            );
        }
        assert!(is_device_key("0123456789abcdef"));
        assert!(parse_args(&args(&["--win-send", r#"0123456789ab" calc"#, "x"])).is_none());
    }

    #[test]
    fn an_empty_path_list_is_not_a_send() {
        assert!(parse_args(&args(&["--win-send", "0123456789abcdef"])).is_none());
    }

    #[test]
    fn the_signature_moves_when_a_peer_is_renamed() {
        let items = |label: &str| {
            vec![MenuItem {
                key: "0123456789abcdef".to_string(),
                label: label.to_string(),
            }]
        };
        let exe = r"C:\Programs\TieZ.exe";
        assert_eq!(
            shell::signature(exe, "发送给 TieZ", &items("书房")),
            shell::signature(exe, "发送给 TieZ", &items("书房")),
        );
        assert_ne!(
            shell::signature(exe, "发送给 TieZ", &items("书房")),
            shell::signature(exe, "发送给 TieZ", &items("卧室")),
        );
        assert_ne!(
            shell::signature(exe, "发送给 TieZ", &items("书房")),
            shell::signature(exe, "傳送至 TieZ", &items("书房")),
        );
    }
}
