use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;

use myprecision_core::dell::{CctkRunner, DellError};

use super::CREATE_NO_WINDOW;

pub struct ExeCctkRunner {
    path: PathBuf,
}

impl ExeCctkRunner {
    pub const DEFAULT_PATH: &str = r"C:\Program Files (x86)\Dell\Command Configure\X86_64\cctk.exe";

    /// `None` when Dell Command | Configure is not installed.
    pub fn locate() -> Option<Self> {
        let path = PathBuf::from(Self::DEFAULT_PATH);
        path.is_file().then_some(Self { path })
    }
}

impl CctkRunner for ExeCctkRunner {
    fn run(&self, args: &[&str]) -> Result<(i32, String), DellError> {
        let out = Command::new(&self.path)
            .args(args)
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|e| match e.kind() {
                std::io::ErrorKind::NotFound => DellError::NotInstalled,
                _ => DellError::Failed { code: -1, message: e.to_string() },
            })?;
        let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
        text.push_str(&String::from_utf8_lossy(&out.stderr));
        Ok((out.status.code().unwrap_or(-1), text))
    }
}
