//! Shared app core: BIOS access, config, history and the last state shown to the UI.

use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::mpsc::{Receiver, Sender, channel};

use myprecision_core::config::{self, Config};
use myprecision_core::dell::{Cctk, ChargeCfg, ThermalMode};
use myprecision_core::history::{HealthLog, History};
use myprecision_core::profile::{ActiveProfile, Profiles, detect};
use myprecision_core::sensors::BatterySnapshot;
use serde::Serialize;

use crate::platform::{self, ExeCctkRunner};

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Availability {
    pub cctk: bool,
    pub dcm: bool,
    pub admin: bool,
    pub optimizer_running: bool,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AppState {
    pub charge: Option<ChargeCfg>,
    pub active_profile: Option<ActiveProfile>,
    pub thermal: Option<ThermalMode>,
    pub profiles: Profiles,
    pub battery: Option<BatterySnapshot>,
    pub availability: Availability,
    pub autostart: bool,
    pub optimizer_warning_dismissed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PollMode {
    Active,
    Idle,
}

pub struct Core {
    pub cctk: Option<Cctk<ExeCctkRunner>>,
    pub config: Mutex<Config>,
    pub history: Mutex<History>,
    pub health: Mutex<HealthLog>,
    pub snapshot: Mutex<AppState>,
    poller_tx: Sender<PollMode>,
}

/// `%APPDATA%\MyPrecision`.
pub fn data_dir() -> PathBuf {
    let base = std::env::var_os("APPDATA").map_or_else(std::env::temp_dir, PathBuf::from);
    base.join("MyPrecision")
}

pub fn config_path() -> PathBuf {
    data_dir().join("config.json")
}

impl Core {
    /// The receiver goes to the poller thread.
    pub fn new() -> (Self, Receiver<PollMode>) {
        let (poller_tx, rx) = channel();
        let config = config::load(&config_path());
        let cctk = ExeCctkRunner::locate().map(Cctk::new);
        let snapshot = AppState {
            charge: None,
            active_profile: None,
            thermal: None,
            profiles: config.profiles.clone(),
            battery: None,
            availability: Availability {
                cctk: cctk.is_some(),
                dcm: false,
                admin: platform::is_elevated(),
                optimizer_running: false,
            },
            autostart: crate::autostart::is_enabled(),
            optimizer_warning_dismissed: config.optimizer_warning_dismissed,
        };
        let core = Self {
            cctk,
            config: Mutex::new(config),
            history: Mutex::new(History::new()),
            health: Mutex::new(HealthLog::open(&data_dir().join("battery-health.json"))),
            snapshot: Mutex::new(snapshot),
            poller_tx,
        };
        (core, rx)
    }

    pub fn set_poll_mode(&self, m: PollMode) {
        // The poller only stops with the app; a send error then is harmless.
        let _ = self.poller_tx.send(m);
    }

    pub fn state(&self) -> AppState {
        self.snapshot.lock().unwrap().clone()
    }

    pub fn update(&self, f: impl FnOnce(&mut AppState)) -> AppState {
        let mut s = self.snapshot.lock().unwrap();
        f(&mut s);
        s.clone()
    }

    /// Re-read the BIOS settings through cctk into the snapshot.
    pub fn refresh_bios(&self) -> AppState {
        let (charge, thermal) = match &self.cctk {
            Some(cctk) => (cctk.get_charge_cfg().ok(), cctk.get_thermal().ok()),
            None => (None, None),
        };
        let profiles = self.config.lock().unwrap().profiles.clone();
        self.update(|s| {
            s.charge = charge;
            s.active_profile = charge.map(|c| detect(c, &profiles));
            s.thermal = thermal;
            s.profiles = profiles.clone();
        })
    }
}
