//! Sensor thread. `Active` (a window is open): every sensor each second, streamed as
//! `telemetry`. `Idle` (tray only): battery and BIOS modes every 30 s, `state-changed` on change.

use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

use chrono::Local;
use myprecision_core::history::{HealthEntry, HistorySample};
use myprecision_core::sensors::{
    BatteryRaw, CpuSnapshot, CpuTimes, Telemetry, battery_from_raw, load_between, parse_dcim,
};
use tauri::{AppHandle, Emitter, Manager};

use crate::platform::{self, GpuReader, WmiReaders};
use crate::state::{Core, PollMode};

const ACTIVE_INTERVAL: Duration = Duration::from_secs(1);
const IDLE_INTERVAL: Duration = Duration::from_secs(30);

pub fn spawn(app: AppHandle, rx: Receiver<PollMode>) {
    std::thread::Builder::new()
        .name("poller".into())
        .spawn(move || Poller::new(app).run(rx))
        .expect("failed to spawn poller thread");
}

struct Poller {
    app: AppHandle,
    // WMI connections are !Send, so they are created on this thread.
    wmi: Option<WmiReaders>,
    gpu: GpuReader,
    cpu_prev: CpuTimes,
    last_slow: Option<Instant>,
}

impl Poller {
    fn new(app: AppHandle) -> Self {
        let wmi = WmiReaders::new().ok();
        let dcm = wmi.as_ref().is_some_and(WmiReaders::dcm_available);
        app.state::<Core>().update(|s| s.availability.dcm = dcm);
        Self { app, wmi, gpu: GpuReader::new(), cpu_prev: platform::cpu_times(), last_slow: None }
    }

    fn run(mut self, rx: Receiver<PollMode>) {
        let mut mode = PollMode::Idle;
        loop {
            self.tick(mode);
            let interval = if mode == PollMode::Active { ACTIVE_INTERVAL } else { IDLE_INTERVAL };
            match rx.recv_timeout(interval) {
                Ok(next) => {
                    mode = next;
                    // A mode switch samples everything at once, BIOS modes included.
                    self.last_slow = None;
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => break,
            }
        }
    }

    fn tick(&mut self, mode: PollMode) {
        let raw = self.wmi.as_ref().and_then(WmiReaders::battery);
        if mode == PollMode::Active {
            self.sample_telemetry(raw.as_ref());
        }
        // BIOS modes cost a cctk process each: every 30 s in both modes.
        if self.last_slow.is_none_or(|t| t.elapsed() >= IDLE_INTERVAL) {
            self.last_slow = Some(Instant::now());
            self.slow_tick(raw.as_ref());
        }
    }

    fn sample_telemetry(&mut self, raw: Option<&BatteryRaw>) {
        let now = platform::cpu_times();
        let load = load_between(self.cpu_prev, now);
        self.cpu_prev = now;
        let dcim = self.wmi.as_ref().and_then(WmiReaders::dcim).map(|rows| parse_dcim(&rows)).unwrap_or_default();
        let gpu = self.gpu.read();
        let telemetry = Telemetry {
            ts_ms: Local::now().timestamp_millis(),
            battery: raw.map(battery_from_raw),
            cpu: CpuSnapshot { load_pct: load, temp_c: dcim.cpu_c },
            gpu,
            fans: dcim.fans,
            mem: platform::mem(),
            dimm_c: dcim.dimm_c,
            skin_c: dcim.skin_c,
        };
        let core = self.app.state::<Core>();
        core.history.lock().unwrap().push(HistorySample::from(&telemetry));
        let state = core.update(|s| s.battery = telemetry.battery.clone());
        crate::tray::refresh(&self.app, &state, telemetry.cpu.temp_c);
        let _ = self.app.emit("telemetry", &telemetry);
    }

    fn slow_tick(&mut self, raw: Option<&BatteryRaw>) {
        let core = self.app.state::<Core>();
        let before = core.state();
        let battery = raw.map(battery_from_raw);
        if let Some(b) = &battery
            && let (Some(full_mwh), Some(design_mwh)) = (b.full_mwh, b.design_mwh)
        {
            let entry = HealthEntry { date: Local::now().date_naive(), full_mwh, design_mwh, cycles: b.cycles };
            let _ = core.health.lock().unwrap().record(entry);
        }
        let optimizer_running = platform::optimizer_running();
        core.update(|s| {
            s.battery = battery;
            s.availability.optimizer_running = optimizer_running;
        });
        let after = core.refresh_bios();
        crate::tray::refresh(&self.app, &after, None);
        if after.active_profile != before.active_profile
            || after.thermal != before.thermal
            || after.availability != before.availability
        {
            let _ = self.app.emit("state-changed", &after);
        }
    }
}

