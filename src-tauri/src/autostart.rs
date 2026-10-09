//! Sign-in autostart through a Task Scheduler task: elevated without a UAC prompt, also on battery.

use std::fs::File;
use std::io::{Read, Write};
use std::os::windows::fs::OpenOptionsExt;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
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

/// Only readers may share the file while we hold it: no rewrite, rename or delete.
const FILE_SHARE_READ: u32 = 0x1;
const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;

/// Writes `bytes` to a new file at `path` and returns a read handle that pins it.
/// The task runs elevated, so its XML must not be swappable between write and import:
/// `create_new` refuses planted files, and the read handle (share READ, no reparse
/// following) fixes both the path and the contents, which are verified through it.
/// schtasks can still open the file for reading.
fn write_exclusive(path: &Path, bytes: &[u8]) -> std::io::Result<File> {
    let mut w = std::fs::OpenOptions::new().write(true).create_new(true).share_mode(0).open(path)?;
    w.write_all(bytes)?;
    drop(w);
    let mut r = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)?;
    let mut back = Vec::with_capacity(bytes.len());
    r.read_to_end(&mut back)?;
    if !r.metadata()?.is_file() || back != bytes {
        return Err(std::io::Error::other("task file changed before import"));
    }
    Ok(r)
}

pub fn enable() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let xml = task_xml(&exe.to_string_lossy(), &user_id());
    // UTF-16LE with a BOM, matching the encoding the XML declares.
    let mut bytes = vec![0xFF, 0xFE];
    bytes.extend(xml.encode_utf16().flat_map(u16::to_le_bytes));
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos());
    let path: PathBuf = std::env::temp_dir().join(format!("myprecision-task-{}-{nanos}.xml", std::process::id()));
    let held = write_exclusive(&path, &bytes).map_err(|e| e.to_string())?;
    let result = schtasks(&["/Create", "/TN", TASK_NAME, "/XML", &path.to_string_lossy(), "/F"]);
    drop(held);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_file_cannot_be_rewritten_while_held() {
        let path = std::env::temp_dir().join(format!("myprecision-test-{}.xml", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let held = write_exclusive(&path, b"<Task/>").unwrap();
        assert!(std::fs::OpenOptions::new().write(true).open(&path).is_err());
        assert!(std::fs::remove_file(&path).is_err());
        // A reader that itself allows only readers (as schtasks may) still gets in.
        let mut reader = std::fs::OpenOptions::new().read(true).share_mode(FILE_SHARE_READ).open(&path).unwrap();
        let mut text = Vec::new();
        reader.read_to_end(&mut text).unwrap();
        assert_eq!(text, b"<Task/>");
        drop(reader);
        drop(held);
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn task_file_is_never_reused() {
        let path = std::env::temp_dir().join(format!("myprecision-test-pre-{}.xml", std::process::id()));
        std::fs::write(&path, b"planted").unwrap();
        assert!(write_exclusive(&path, b"<Task/>").is_err());
        std::fs::remove_file(&path).unwrap();
    }
}
