//! Battery profiles (Home / Campus / Storage) and detection of the active one from the BIOS value.

use serde::{Deserialize, Serialize};

use crate::dell::ChargeCfg;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BatteryProfile {
    Home,
    Campus,
    Storage,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Profiles {
    pub home: ChargeCfg,
    pub campus: ChargeCfg,
    pub storage: ChargeCfg,
}

impl Default for Profiles {
    fn default() -> Self {
        Self {
            home: ChargeCfg::Custom { start: 75, stop: 80 },
            campus: ChargeCfg::Standard,
            storage: ChargeCfg::Custom { start: 50, stop: 60 },
        }
    }
}

impl Profiles {
    pub fn get(&self, p: BatteryProfile) -> ChargeCfg {
        match p {
            BatteryProfile::Home => self.home,
            BatteryProfile::Campus => self.campus,
            BatteryProfile::Storage => self.storage,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ActiveProfile {
    Known { profile: BatteryProfile },
    Other { cfg: ChargeCfg },
}

pub fn detect(cfg: ChargeCfg, profiles: &Profiles) -> ActiveProfile {
    [BatteryProfile::Home, BatteryProfile::Campus, BatteryProfile::Storage]
        .into_iter()
        .find(|&p| profiles.get(p) == cfg)
        .map_or(ActiveProfile::Other { cfg }, |profile| ActiveProfile::Known { profile })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_spec() {
        let p = Profiles::default();
        assert_eq!(p.home, ChargeCfg::Custom { start: 75, stop: 80 });
        assert_eq!(p.campus, ChargeCfg::Standard);
        assert_eq!(p.storage, ChargeCfg::Custom { start: 50, stop: 60 });
    }

    #[test]
    fn get_returns_profile_cfg() {
        assert_eq!(Profiles::default().get(BatteryProfile::Storage), ChargeCfg::Custom { start: 50, stop: 60 });
    }

    #[test]
    fn detects_known() {
        assert_eq!(
            detect(ChargeCfg::Standard, &Profiles::default()),
            ActiveProfile::Known { profile: BatteryProfile::Campus }
        );
        assert_eq!(
            detect(ChargeCfg::Custom { start: 75, stop: 80 }, &Profiles::default()),
            ActiveProfile::Known { profile: BatteryProfile::Home }
        );
    }

    #[test]
    fn detects_other() {
        let cfg = ChargeCfg::Custom { start: 60, stop: 90 };
        assert_eq!(detect(cfg, &Profiles::default()), ActiveProfile::Other { cfg });
    }

    #[test]
    fn serializes_for_ui() {
        let json = serde_json::to_string(&ActiveProfile::Known { profile: BatteryProfile::Home }).unwrap();
        assert_eq!(json, r#"{"kind":"known","profile":"home"}"#);
        let json = serde_json::to_string(&ActiveProfile::Other { cfg: ChargeCfg::Custom { start: 60, stop: 90 } }).unwrap();
        assert_eq!(json, r#"{"kind":"other","cfg":{"kind":"Custom","start":60,"stop":90}}"#);
    }
}
