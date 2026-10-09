//! Tauri commands. Errors reach the UI as English sentences.

use chrono::Local;
use myprecision_core::automation::{self, Automation};
use myprecision_core::bios;
use myprecision_core::config;
use myprecision_core::dell::{DellError, ThermalMode};
use myprecision_core::history::{HealthEntry, HistorySample};
use myprecision_core::profile::BatteryProfile;
use tauri::{AppHandle, Emitter, State};

use crate::state::{AppState, Core, config_path};
use crate::sync::LockExt;

/// After a write attempt the UI must show the truth: a success carries the value the BIOS
/// accepted; a failure leaves both settings uncertain, so re-read them.
fn after_write<T>(
    app: &AppHandle,
    core: &Core,
    result: Result<T, DellError>,
    store: impl FnOnce(&Core, T) -> AppState,
) -> Result<AppState, String> {
    let outcome = match result {
        Ok(read_back) => Ok(store(core, read_back)),
        Err(e) => {
            core.refresh_bios();
            Err(e.to_string())
        }
    };
    let _ = app.emit("state-changed", &core.state());
    outcome
}

#[tauri::command]
pub fn get_state(core: State<'_, Core>) -> AppState {
    core.state()
}

/// Shared by the command and the tray menu.
pub fn apply_battery_profile(app: &AppHandle, core: &Core, profile: BatteryProfile) -> Result<AppState, String> {
    let Some(cctk) = &core.cctk else {
        return Err(DellError::NotInstalled.to_string());
    };
    let cfg = core.config.lock_ok().profiles.get(profile);
    let result = cctk.set_charge_cfg(cfg);
    after_write(app, core, result, Core::set_charge)
}

/// Shared by the command and the tray menu.
pub fn apply_thermal_mode(app: &AppHandle, core: &Core, mode: ThermalMode) -> Result<AppState, String> {
    let Some(cctk) = &core.cctk else {
        return Err(DellError::NotInstalled.to_string());
    };
    let result = cctk.set_thermal(mode);
    after_write(app, core, result, Core::set_thermal)
}

#[tauri::command(async)]
pub fn set_battery_profile(app: AppHandle, core: State<'_, Core>, profile: BatteryProfile) -> Result<AppState, String> {
    apply_battery_profile(&app, &core, profile)
}

#[tauri::command(async)]
pub fn set_thermal_mode(app: AppHandle, core: State<'_, Core>, mode: ThermalMode) -> Result<AppState, String> {
    apply_thermal_mode(&app, &core, mode)
}

#[tauri::command]
pub fn get_history(core: State<'_, Core>, minutes: u32) -> Vec<HistorySample> {
    core.history.lock_ok().range(minutes, Local::now().timestamp_millis())
}

/// The latest sample, so a freshly opened window doesn't wait for the next tick.
#[tauri::command]
pub fn get_telemetry(core: State<'_, Core>) -> Option<myprecision_core::sensors::Telemetry> {
    core.telemetry.lock_ok().clone()
}

#[tauri::command]
pub fn get_health_log(core: State<'_, Core>) -> Vec<HealthEntry> {
    core.health.lock_ok().entries().to_vec()
}

/// Shared by the command and the tray menu. Returns the real state after the attempt.
pub fn apply_autostart(app: &AppHandle, core: &Core, enabled: bool) -> Result<bool, String> {
    let result = if enabled { crate::autostart::enable() } else { crate::autostart::disable() };
    let actual = crate::autostart::is_enabled();
    let state = core.update(|s| s.autostart = actual);
    let _ = app.emit("state-changed", &state);
    result.map(|()| actual)
}

#[tauri::command(async)]
pub fn set_autostart(app: AppHandle, core: State<'_, Core>, enabled: bool) -> Result<bool, String> {
    apply_autostart(&app, &core, enabled)
}

#[tauri::command(async)]
pub fn dismiss_optimizer_warning(app: AppHandle, core: State<'_, Core>) {
    {
        let mut cfg = core.config.lock_ok();
        cfg.optimizer_warning_dismissed = true;
        // Best effort: the flag still holds for this session if the disk write fails.
        let _ = config::save(&config_path(), &cfg);
    }
    let state = core.update(|s| s.optimizer_warning_dismissed = true);
    let _ = app.emit("state-changed", &state);
}

#[tauri::command(async)]
pub fn open_full_window(app: AppHandle) {
    crate::windows::open_full(&app);
}

/// The flyout's content height (CSS px): the window is sized to it.
#[tauri::command(async)]
pub fn fit_flyout(window: tauri::WebviewWindow, height: f64) {
    crate::windows::fit_flyout(&window, height);
}

/// The calling window has painted its first state: show it now, never blank.
#[tauri::command(async)]
pub fn window_ready(window: tauri::WebviewWindow) {
    crate::windows::reveal(&window);
}

/// Model, service tag and BIOS version from SMBIOS; read once, it never changes while running.
#[tauri::command(async)]
pub fn get_device_info() -> myprecision_core::smbios::DeviceInfo {
    static INFO: std::sync::OnceLock<myprecision_core::smbios::DeviceInfo> = std::sync::OnceLock::new();
    INFO.get_or_init(crate::platform::device_info).clone()
}

#[tauri::command]
pub fn get_automation(core: State<'_, Core>) -> Automation {
    core.config.lock_ok().automation.clone()
}

#[tauri::command(async)]
pub fn set_automation(core: State<'_, Core>, rules: Automation) -> Result<Automation, String> {
    automation::validate(&rules)?;
    {
        let mut cfg = core.config.lock_ok();
        let mut next = cfg.clone();
        next.automation = rules.clone();
        config::save(&config_path(), &next).map_err(|e| format!("Could not save settings: {e}"))?;
        *cfg = next;
    }
    let mut state = core.automation.lock_ok();
    state.rules_changed(Local::now().naive_local());
    crate::automation::persist(&state);
    Ok(rules)
}

/// The curated BIOS settings with their current values; one cctk run (seconds).
#[tauri::command(async)]
pub fn get_bios_settings(core: State<'_, Core>) -> Result<Vec<bios::Setting>, String> {
    let Some(cctk) = &core.cctk else {
        return Err(DellError::NotInstalled.to_string());
    };
    bios::read(cctk).map_err(|e| e.to_string())
}

/// Writes one whitelisted setting; the UI confirms with the user before calling this.
#[tauri::command(async)]
pub fn set_bios_setting(core: State<'_, Core>, key: String, value: String) -> Result<(), String> {
    let Some(cctk) = &core.cctk else {
        return Err(DellError::NotInstalled.to_string());
    };
    bios::write(cctk, &key, &value).map_err(|e| e.to_string())
}
