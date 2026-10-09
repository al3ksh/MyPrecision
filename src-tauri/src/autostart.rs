//! Sign-in autostart through a Task Scheduler task: elevated without a UAC prompt, also on battery.

use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::{Command, Output};

use myprecision_core::autostart::{TASK_NAME, task_xml};

use crate::platform::CREATE_NO_WINDOW;

fn schtasks(args: &[&str]) -> std::io::Result<Output> {
    Command::new("schtasks.exe").args(args).creation_flags(CREATE_NO_WINDOW).output()
}

fn failure(out: &Output) -> String {
    let msg = String::from_utf8_lossy(&out.stderr).trim().to_owned();
    if msg.is_empty() { "Task Scheduler rejected the change.".into() } else { msg }
}

/// `DOMAIN\user` of the signed-in user.
fn user_id() -> String {
    let user = std::env::var("USERNAME").unwrap_or_default();
    match std::env::var("USERDOMAIN") {
        Ok(domain) if !domain.is_empty() => format!(r"{domain}\{user}"),
        _ => user,
    }
}

pub fn enable() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let xml = task_xml(&exe.to_string_lossy(), &user_id());
    // UTF-16LE with a BOM, matching the encoding the XML declares.
    let mut bytes = vec![0xFF, 0xFE];
    bytes.extend(xml.encode_utf16().flat_map(u16::to_le_bytes));
    let path: PathBuf = std::env::temp_dir().join(format!("myprecision-task-{}.xml", std::process::id()));
    std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
    let result = schtasks(&["/Create", "/TN", TASK_NAME, "/XML", &path.to_string_lossy(), "/F"]);
    let _ = std::fs::remove_file(&path);
    let out = result.map_err(|e| e.to_string())?;
    if out.status.success() { Ok(()) } else { Err(failure(&out)) }
}

pub fn disable() -> Result<(), String> {
    let out = schtasks(&["/Delete", "/TN", TASK_NAME, "/F"]).map_err(|e| e.to_string())?;
    if out.status.success() { Ok(()) } else { Err(failure(&out)) }
}

pub fn is_enabled() -> bool {
    schtasks(&["/Query", "/TN", TASK_NAME]).is_ok_and(|o| o.status.success())
}
