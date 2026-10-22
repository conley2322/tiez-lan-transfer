//! Windows elevated (admin) logon autostart via Task Scheduler.
//! Registry Run cannot elevate; a Highest-privilege ONLOGON task can.

#![cfg(target_os = "windows")]

use std::os::windows::process::CommandExt;
use std::process::Command;
use std::time::Duration;

const TASK_NAME: &str = r"TieZ\ElevatedAutostart";
const CREATE_NO_WINDOW: u32 = 0x0800_0000;
/// Passed by the task, so an instance it started never hands off to it again.
pub const TASK_ARG: &str = "--elevated-autostart";

fn exe_path() -> Result<String, String> {
    Ok(std::env::current_exe()
        .map_err(|e| e.to_string())?
        .to_string_lossy()
        .to_string())
}

fn task_command_line() -> Result<String, String> {
    let exe = exe_path()?;
    Ok(format!("\"{exe}\" --minimized"))
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// The task as XML. `schtasks /Create /SC ONLOGON` keeps Task Scheduler's
/// defaults, which broke "start as administrator": no start on battery (a
/// laptop unplugged at logon never got an elevated TieZ, so users opened a
/// normal one), killed on unplugging or after 72 hours, below-normal priority.
fn task_xml(exe: &str, user: &str) -> String {
    let exe = xml_escape(exe);
    let user = xml_escape(user);
    format!(
        r#"<?xml version="1.0" encoding="UTF-16"?>
<Task version="1.2" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <Triggers>
    <LogonTrigger>
      <Enabled>true</Enabled>
      <UserId>{user}</UserId>
    </LogonTrigger>
  </Triggers>
  <Principals>
    <Principal id="Author">
      <UserId>{user}</UserId>
      <LogonType>InteractiveToken</LogonType>
      <RunLevel>HighestAvailable</RunLevel>
    </Principal>
  </Principals>
  <Settings>
    <MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>
    <DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>
    <StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>
    <AllowHardTerminate>true</AllowHardTerminate>
    <StartWhenAvailable>false</StartWhenAvailable>
    <RunOnlyIfNetworkAvailable>false</RunOnlyIfNetworkAvailable>
    <IdleSettings>
      <StopOnIdleEnd>false</StopOnIdleEnd>
      <RestartOnIdle>false</RestartOnIdle>
    </IdleSettings>
    <AllowStartOnDemand>true</AllowStartOnDemand>
    <Enabled>true</Enabled>
    <Hidden>false</Hidden>
    <RunOnlyIfIdle>false</RunOnlyIfIdle>
    <WakeToRun>false</WakeToRun>
    <ExecutionTimeLimit>PT0S</ExecutionTimeLimit>
    <Priority>4</Priority>
  </Settings>
  <Actions Context="Author">
    <Exec>
      <Command>{exe}</Command>
      <Arguments>--minimized {TASK_ARG}</Arguments>
    </Exec>
  </Actions>
</Task>
"#
    )
}

fn current_user() -> String {
    let name = std::env::var("USERNAME").unwrap_or_default();
    match std::env::var("USERDOMAIN") {
        Ok(domain) if !domain.is_empty() => format!("{domain}\\{name}"),
        _ => name,
    }
}

/// The installed task was made by this version (battery and time limits off).
pub fn is_task_current() -> bool {
    let Ok(output) = Command::new("schtasks")
        .args(["/Query", "/TN", TASK_NAME, "/XML"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
    else {
        return false;
    };
    if !output.status.success() {
        return false;
    }
    // The XML can come back as UTF-16 or in the console code page.
    let wide: Vec<u16> = output
        .stdout
        .chunks_exact(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .collect();
    [String::from_utf16_lossy(&wide), String::from_utf8_lossy(&output.stdout).into_owned()]
        .iter()
        .any(|xml| xml.contains(TASK_ARG))
}

/// Tasks made by older versions keep their bad defaults until rewritten,
/// which needs admin rights: done whenever TieZ runs elevated.
pub fn refresh_outdated_task() {
    if is_elevated_autostart_installed() && !is_task_current() {
        if let Err(error) = install_elevated_autostart_task() {
            eprintln!(">>> [ELEVATED-AUTOSTART] refreshing the task failed: {error}");
        }
    }
}

/// An admin whose TieZ is running with the limited token (UAC split token):
/// the task can start an elevated one without a prompt.
fn can_elevate_through_task() -> bool {
    use std::ffi::c_void;
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::Security::{
        GetTokenInformation, TokenElevationType, TokenElevationTypeLimited, TOKEN_ELEVATION_TYPE,
        TOKEN_QUERY,
    };
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
    unsafe {
        let mut token = HANDLE::default();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).is_err() {
            return false;
        }
        let mut elevation_type = TOKEN_ELEVATION_TYPE::default();
        let mut length = 0;
        let ok = GetTokenInformation(
            token,
            TokenElevationType,
            Some(&mut elevation_type as *mut _ as *mut c_void),
            std::mem::size_of::<TOKEN_ELEVATION_TYPE>() as u32,
            &mut length,
        )
        .is_ok();
        let _ = CloseHandle(token);
        ok && elevation_type == TokenElevationTypeLimited
    }
}

/// A login started TieZ without admin rights although "start as
/// administrator" is on (something else launched it first, or the old task
/// didn't run): start the task instead and let this instance exit. Only with
/// a task of this version, so the elevated start can't be blocked on battery,
/// and never from an instance the task started.
pub fn hand_off_to_task(args: &[String]) -> bool {
    if args.iter().any(|a| a == TASK_ARG) || !args.iter().any(|a| a == "--minimized") {
        return false;
    }
    if !can_elevate_through_task() || !is_task_current() {
        return false;
    }
    Command::new("schtasks")
        .args(["/Run", "/TN", TASK_NAME])
        .creation_flags(CREATE_NO_WINDOW)
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

pub fn is_elevated_autostart_installed() -> bool {
    Command::new("schtasks")
        .args(["/Query", "/TN", TASK_NAME])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub fn install_elevated_autostart_task() -> Result<(), String> {
    let xml = task_xml(&exe_path()?, &current_user());
    // schtasks wants the file in the encoding the declaration names.
    let mut bytes = vec![0xFF, 0xFE];
    bytes.extend(xml.encode_utf16().flat_map(u16::to_le_bytes));
    let xml_path = std::env::temp_dir().join(format!("tiez-autostart-{}.xml", std::process::id()));
    std::fs::write(&xml_path, bytes).map_err(|e| format!("writing the task failed: {e}"))?;
    let status = Command::new("schtasks")
        .args(["/Create", "/TN", TASK_NAME, "/XML"])
        .arg(&xml_path)
        .arg("/F")
        .creation_flags(CREATE_NO_WINDOW)
        .status();
    let _ = std::fs::remove_file(&xml_path);
    let status = status.map_err(|e| format!("schtasks create failed: {e}"))?;
    if !status.success() {
        return Err(
            "Failed to create elevated autostart task (administrator rights required)".into(),
        );
    }
    clear_registry_run_entry()?;
    Ok(())
}

pub fn uninstall_elevated_autostart_task() -> Result<(), String> {
    if !is_elevated_autostart_installed() {
        return Ok(());
    }
    let status = Command::new("schtasks")
        .args(["/Delete", "/TN", TASK_NAME, "/F"])
        .creation_flags(CREATE_NO_WINDOW)
        .status()
        .map_err(|e| format!("schtasks delete failed: {e}"))?;
    if !status.success() {
        return Err("Failed to remove elevated autostart task".into());
    }
    Ok(())
}

pub fn clear_registry_run_entry() -> Result<(), String> {
    use winreg::enums::*;
    use winreg::RegKey;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = hkcu
        .open_subkey_with_flags(
            "Software\\Microsoft\\Windows\\CurrentVersion\\Run",
            KEY_WRITE | KEY_READ,
        )
        .map_err(|e| e.to_string())?;
    let _ = key.delete_value("TieZ");
    let _ = key.delete_value("tie-z");
    Ok(())
}

pub fn write_registry_run_entry() -> Result<(), String> {
    use winreg::enums::*;
    use winreg::RegKey;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = hkcu
        .open_subkey_with_flags(
            "Software\\Microsoft\\Windows\\CurrentVersion\\Run",
            KEY_WRITE | KEY_READ,
        )
        .map_err(|e| e.to_string())?;
    let cmd = task_command_line()?;
    key.set_value("TieZ", &cmd).map_err(|e| e.to_string())
}

pub fn registry_run_present() -> bool {
    use winreg::enums::*;
    use winreg::RegKey;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let Ok(key) = hkcu.open_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Run") else {
        return false;
    };
    key.get_value::<String, _>("TieZ").is_ok() || key.get_value::<String, _>("tie-z").is_ok()
}

/// Relaunch this exe elevated to install/uninstall the task (one UAC prompt).
pub fn run_elevated_helper(install: bool) -> Result<(), String> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{CloseHandle, WAIT_OBJECT_0, WAIT_TIMEOUT};
    use windows::Win32::System::Threading::WaitForSingleObject;
    use windows::Win32::UI::Shell::{ShellExecuteExW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW};
    use windows::Win32::UI::WindowsAndMessaging::SW_HIDE;

    let exe = exe_path()?;
    let arg = if install {
        "--install-elevated-autostart"
    } else {
        "--uninstall-elevated-autostart"
    };

    let exe_wide: Vec<u16> = OsStr::new(&exe)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let args_wide: Vec<u16> = OsStr::new(arg)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let runas: Vec<u16> = OsStr::new("runas")
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    unsafe {
        let mut info = SHELLEXECUTEINFOW {
            cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
            fMask: SEE_MASK_NOCLOSEPROCESS,
            lpVerb: PCWSTR::from_raw(runas.as_ptr()),
            lpFile: PCWSTR::from_raw(exe_wide.as_ptr()),
            lpParameters: PCWSTR::from_raw(args_wide.as_ptr()),
            nShow: SW_HIDE.0 as i32,
            ..Default::default()
        };

        if ShellExecuteExW(&mut info).is_err() || info.hProcess.is_invalid() {
            return Err("Administrator approval was cancelled or elevation failed".into());
        }

        let wait = WaitForSingleObject(info.hProcess, 60_000);
        let _ = CloseHandle(info.hProcess);
        if wait == WAIT_TIMEOUT {
            return Err("Timed out waiting for elevated autostart helper".into());
        }
        if wait != WAIT_OBJECT_0 {
            return Err("Elevated autostart helper did not finish cleanly".into());
        }
    }

    for _ in 0..20 {
        if is_elevated_autostart_installed() == install {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    if install {
        Err("Elevated autostart task was not created".into())
    } else if is_elevated_autostart_installed() {
        Err("Elevated autostart task was not removed".into())
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_runs_on_battery_without_time_limit() {
        let xml = task_xml(r"C:\Program Files\R&D <TieZ>\tiez-app.exe", r"PC\User");
        assert!(xml.contains("<DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>"));
        assert!(xml.contains("<StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>"));
        assert!(xml.contains("<ExecutionTimeLimit>PT0S</ExecutionTimeLimit>"));
        assert!(xml.contains("<RunLevel>HighestAvailable</RunLevel>"));
        assert!(xml.contains(r"<Command>C:\Program Files\R&amp;D &lt;TieZ&gt;\tiez-app.exe</Command>"));
        assert!(xml.contains(&format!("<Arguments>--minimized {TASK_ARG}</Arguments>")));
        assert!(xml.contains(r"<UserId>PC\User</UserId>"));
    }

    #[test]
    fn only_a_login_start_hands_off() {
        let args = |list: &[&str]| list.iter().map(|a| a.to_string()).collect::<Vec<_>>();
        // A manual start or one the task made never hands off.
        assert!(!hand_off_to_task(&args(&["tiez-app.exe"])));
        assert!(!hand_off_to_task(&args(&["tiez-app.exe", "--minimized", TASK_ARG])));
    }
}
