//! Tauri commands. Errors reach the UI as English sentences.

use chrono::Local;
use myprecision_core::config;
use myprecision_core::dell::{DellError, ThermalMode};
use myprecision_core::history::{HealthEntry, HistorySample};
use myprecision_core::profile::BatteryProfile;
use tauri::{AppHandle, Emitter, State};

use crate::state::{AppState, Core, config_path};

/// After any write attempt — successful or not — re-read the BIOS so the UI shows the truth.
fn after_write<T>(app: &AppHandle, core: &Core, result: Result<T, DellError>) -> Result<AppState, String> {
    let state = core.refresh_bios();
    let _ = app.emit("state-changed", &state);
    result.map(|_| state).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_state(core: State<'_, Core>) -> AppState {
    core.state()
}

#[tauri::command(async)]
pub fn set_battery_profile(app: AppHandle, core: State<'_, Core>, profile: BatteryProfile) -> Result<AppState, String> {
    let Some(cctk) = &core.cctk else {
        return Err(DellError::NotInstalled.to_string());
    };
    let cfg = core.config.lock().unwrap().profiles.get(profile);
    let result = cctk.set_charge_cfg(cfg);
    after_write(&app, &core, result)
}

#[tauri::command(async)]
pub fn set_thermal_mode(app: AppHandle, core: State<'_, Core>, mode: ThermalMode) -> Result<AppState, String> {
    let Some(cctk) = &core.cctk else {
        return Err(DellError::NotInstalled.to_string());
    };
    let result = cctk.set_thermal(mode);
    after_write(&app, &core, result)
}

#[tauri::command]
pub fn get_history(core: State<'_, Core>, minutes: u32) -> Vec<HistorySample> {
    core.history.lock().unwrap().range(minutes, Local::now().timestamp_millis())
}

#[tauri::command]
pub fn get_health_log(core: State<'_, Core>) -> Vec<HealthEntry> {
    core.health.lock().unwrap().entries().to_vec()
}

#[tauri::command]
pub fn set_autostart(enabled: bool) -> Result<bool, String> {
    let _ = enabled;
    Err("Not available yet".into())
}

#[tauri::command]
pub fn dismiss_optimizer_warning(app: AppHandle, core: State<'_, Core>) {
    {
        let mut cfg = core.config.lock().unwrap();
        cfg.optimizer_warning_dismissed = true;
        // Best effort: the flag still holds for this session if the disk write fails.
        let _ = config::save(&config_path(), &cfg);
    }
    let state = core.update(|s| s.optimizer_warning_dismissed = true);
    let _ = app.emit("state-changed", &state);
}

#[tauri::command]
pub fn open_full_window() {}
