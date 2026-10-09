//! `powercfg` reports that only exist as files: SRUM energy usage and the sleep study.
//! Both need administrator rights.

use std::os::windows::process::CommandExt;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use super::CREATE_NO_WINDOW;

/// Runs `powercfg <args> /output <tmp>.<ext> [tail]`, returns the file's text and deletes it.
fn report(args: &[&str], ext: &str, tail: &[&str]) -> anyhow::Result<String> {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_nanos());
    let path = std::env::temp_dir().join(format!("myprecision-powercfg-{}-{nanos}.{ext}", std::process::id()));
    let out = Command::new("powercfg")
        .args(args)
        .arg("/output")
        .arg(&path)
        .args(tail)
        .creation_flags(CREATE_NO_WINDOW)
        .output()?;
    let text = std::fs::read(&path);
    let _ = std::fs::remove_file(&path);
    if !out.status.success() {
        anyhow::bail!("powercfg failed: {}", String::from_utf8_lossy(&out.stdout).trim());
    }
    Ok(String::from_utf8_lossy(&text?).into_owned())
}

/// The SRUM energy-usage table as CSV.
pub fn srum_csv() -> anyhow::Result<String> {
    report(&["/srumutil"], "csv", &["/csv"])
}

/// The last week of the sleep study as XML.
pub fn sleep_study_xml() -> anyhow::Result<String> {
    report(&["/sleepstudy"], "xml", &["/xml", "/duration", "7"])
}
