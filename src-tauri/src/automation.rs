//! Runs the automatic-profile engine on every poller tick and applies what it decides.

use std::path::PathBuf;
use std::sync::Mutex;

use chrono::Local;
use myprecision_core::automation::{Action, Reason, evaluate, save_state};
use myprecision_core::dell::ThermalMode;
use myprecision_core::profile::{ActiveProfile, BatteryProfile};
use tauri::{AppHandle, Manager};
use tauri_plugin_notification::NotificationExt;

use crate::commands;
use crate::state::{Core, data_dir};
use crate::sync::LockExt;

pub fn state_path() -> PathBuf {
    data_dir().join("automation-state.json")
}

/// Persist the engine state; a failed write only costs a repeat after a restart.
pub fn persist(state: &myprecision_core::automation::AutomationState) {
    if let Err(e) = save_state(&state_path(), state) {
        eprintln!("automation: could not save state: {e}");
    }
}

/// One engine step for the current power source. Writes run on a helper thread: cctk takes seconds.
pub fn tick(app: &AppHandle, on_ac: bool) {
    let core = app.state::<Core>();
    let rules = core.config.lock_ok().automation.clone();
    let actions = {
        let mut state = core.automation.lock_ok();
        let before = state.clone();
        let actions = evaluate(&mut state, on_ac, Local::now().naive_local(), &rules);
        if *state != before {
            persist(&state);
        }
        actions
    };
    if actions.is_empty() {
        return;
    }
    let app = app.clone();
    std::thread::spawn(move || {
        // Automatic writes never overlap one another.
        static WRITING: Mutex<()> = Mutex::new(());
        let _guard = WRITING.lock_ok();
        for action in actions {
            apply(&app, action, rules.notify);
        }
    });
}

fn apply(app: &AppHandle, action: Action, notify: bool) {
    let core = app.state::<Core>();
    let current = core.state();
    let (target, reason, result) = match action {
        Action::Thermal { mode, reason } => {
            if current.thermal == Some(mode) {
                return;
            }
            (thermal_name(mode), reason, commands::apply_thermal_mode(app, &core, mode))
        }
        Action::Battery { profile, reason } => {
            if current.active_profile == Some(ActiveProfile::Known { profile }) {
                return;
            }
            (profile_name(profile), reason, commands::apply_battery_profile(app, &core, profile))
        }
    };
    crate::tray::refresh(app, &core.state(), None);
    let why = reason_text(reason, core.config.lock_ok().automation.long_ac.days);
    match result {
        Ok(_) if notify => show(app, &format!("Switched to {target}"), &why),
        Ok(_) => {}
        Err(e) => {
            eprintln!("automation: switching to {target} ({why}) failed: {e}");
            // A failure is always worth a notification: the user expects the change.
            show(app, &format!("Couldn't switch to {target}"), &e);
        }
    }
}

fn show(app: &AppHandle, title: &str, body: &str) {
    if let Err(e) = app.notification().builder().title(title).body(body).show() {
        eprintln!("automation: notification failed: {e}");
    }
}

fn thermal_name(mode: ThermalMode) -> &'static str {
    match mode {
        ThermalMode::Optimized => "Optimized",
        ThermalMode::Cool => "Cool",
        ThermalMode::Quiet => "Quiet",
        ThermalMode::UltraPerformance => "Ultra Performance",
    }
}

fn profile_name(profile: BatteryProfile) -> &'static str {
    match profile {
        BatteryProfile::Home => "Home",
        BatteryProfile::Campus => "Campus",
        BatteryProfile::Storage => "Storage",
    }
}

fn reason_text(reason: Reason, days: u32) -> String {
    match reason {
        Reason::OnAc => "Plugged in.".into(),
        Reason::OnBattery => "On battery.".into(),
        Reason::Schedule => "Scheduled.".into(),
        Reason::LongAc => format!("Plugged in for {days} days."),
    }
}
