//! `myprecision.exe --probe <file>`: dump every reading as JSON for on-device verification.

use std::path::Path;
use std::time::Duration;

use myprecision_core::dell::Cctk;
use myprecision_core::sensors::{battery_from_raw, load_between, parse_dcim};
use serde_json::{Value, json};

use crate::platform::{self, ExeCctkRunner, WmiReaders};

fn result_json<T: serde::Serialize, E: ToString>(r: Result<T, E>) -> Value {
    match r {
        Ok(v) => json!(v),
        Err(e) => json!({ "error": e.to_string() }),
    }
}

pub fn run_probe(out: &Path) -> anyhow::Result<()> {
    let (charge_cfg, thermal) = match ExeCctkRunner::locate() {
        Some(runner) => {
            let cctk = Cctk::new(runner);
            (result_json(cctk.get_charge_cfg()), result_json(cctk.get_thermal()))
        }
        None => {
            let missing = json!({ "error": "cctk.exe not found" });
            (missing.clone(), missing)
        }
    };

    let wmi = WmiReaders::new()?;
    let battery_raw = wmi.battery();
    let dcim_rows = wmi.dcim();

    let before = platform::cpu_times();
    std::thread::sleep(Duration::from_millis(500));
    let cpu_load = load_between(before, platform::cpu_times());

    let report = json!({
        "chargeCfg": charge_cfg,
        "thermal": thermal,
        "batteryRaw": battery_raw,
        "battery": battery_raw.as_ref().map(battery_from_raw),
        "dcmAvailable": wmi.dcm_available(),
        "dcimRows": dcim_rows,
        "dcim": dcim_rows.as_deref().map(parse_dcim),
        "cpuLoad": cpu_load,
        "mem": platform::mem(),
        "elevated": platform::is_elevated(),
        "optimizerRunning": platform::optimizer_running(),
    });
    if let Some(dir) = out.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(out, serde_json::to_string_pretty(&report)?)?;
    Ok(())
}
