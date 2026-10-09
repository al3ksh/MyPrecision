//! `config.json` persistence. Loading never fails: bad data falls back to defaults.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::dell::{ChargeCfg, validate_custom};
use crate::profile::Profiles;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Config {
    pub version: u32,
    pub profiles: Profiles,
    pub optimizer_warning_dismissed: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self { version: 1, profiles: Profiles::default(), optimizer_warning_dismissed: false }
    }
}

pub fn load(path: &Path) -> Config {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Config::default();
    };
    let mut cfg = match serde_json::from_str::<Config>(&text) {
        Ok(cfg) => cfg,
        Err(_) => {
            let _ = std::fs::copy(path, path.with_extension("json.bak"));
            return Config::default();
        }
    };
    let defaults = Profiles::default();
    for (slot, fallback) in [
        (&mut cfg.profiles.home, defaults.home),
        (&mut cfg.profiles.campus, defaults.campus),
        (&mut cfg.profiles.storage, defaults.storage),
    ] {
        if let ChargeCfg::Custom { start, stop } = *slot {
            if validate_custom(start, stop).is_err() {
                *slot = fallback;
            }
        }
    }
    cfg
}

pub fn save(path: &Path, cfg: &Config) -> std::io::Result<()> {
    write_atomic(path, serde_json::to_string_pretty(cfg)?.as_bytes())
}

/// Writes `<path>.tmp`, then renames it over `path`, creating the parent directory if needed.
pub(crate) fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_missing_file_gives_default() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(load(&dir.path().join("config.json")), Config::default());
    }

    #[test]
    fn roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        let mut cfg = Config::default();
        cfg.profiles.home = ChargeCfg::Custom { start: 70, stop: 85 };
        cfg.optimizer_warning_dismissed = true;
        save(&path, &cfg).unwrap();
        assert_eq!(load(&path), cfg);
    }

    #[test]
    fn save_creates_missing_dir() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("MyPrecision").join("config.json");
        save(&path, &Config::default()).unwrap();
        assert!(path.exists());
    }

    #[test]
    fn corrupt_json_gives_default_and_backup() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        std::fs::write(&path, "{nope").unwrap();
        assert_eq!(load(&path), Config::default());
        assert_eq!(std::fs::read_to_string(dir.path().join("config.json.bak")).unwrap(), "{nope");
    }

    #[test]
    fn invalid_custom_in_file_falls_back_per_profile() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        std::fs::write(
            &path,
            r#"{"version":1,"profiles":{"home":{"kind":"Custom","start":75,"stop":77},
               "campus":{"kind":"Standard"},"storage":{"kind":"Custom","start":55,"stop":65}}}"#,
        )
        .unwrap();
        let cfg = load(&path);
        assert_eq!(cfg.profiles.home, Profiles::default().home);
        assert_eq!(cfg.profiles.storage, ChargeCfg::Custom { start: 55, stop: 65 });
    }
}
