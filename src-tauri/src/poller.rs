//! Sensor thread. `Active` (a window is open): every sensor each second, streamed as
//! `telemetry`. `Idle` (tray only): battery every 30 s. BIOS modes are re-read every 30 s in both
//! modes on a helper thread; `state-changed` on change.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
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
use crate::sync::LockExt;

const ACTIVE_INTERVAL: Duration = Duration::from_secs(1);
const IDLE_INTERVAL: Duration = Duration::from_secs(30);
/// CPU load is a delta between two samples; after Idle the previous one is stale.
const LOAD_BASELINE: Duration = Duration::from_millis(250);

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
    last_bios: Option<Instant>,
    bios_busy: Arc<AtomicBool>,
}

impl Poller {
    fn new(app: AppHandle) -> Self {
        let wmi = WmiReaders::new().ok();
        let dcm = wmi.as_ref().is_some_and(WmiReaders::dcm_available);
        app.state::<Core>().update(|s| {
            s.availability.wmi = wmi.is_some();
            s.availability.dcm = dcm;
        });
        Self {
            app,
            wmi,
            gpu: GpuReader::new(),
            cpu_prev: platform::cpu_times(),
            last_slow: None,
            last_bios: None,
            bios_busy: Arc::default(),
        }
    }

    fn run(mut self, rx: Receiver<PollMode>) {
        let mut mode = PollMode::Idle;
        loop {
            self.tick(mode);
            let interval = if mode == PollMode::Active { ACTIVE_INTERVAL } else { IDLE_INTERVAL };
            match rx.recv_timeout(interval) {
                Ok(next) => {
                    if next == PollMode::Idle {
                        self.gpu.release();
                        *self.app.state::<Core>().telemetry.lock_ok() = None;
                    } else if mode == PollMode::Idle {
                        self.cpu_prev = platform::cpu_times();
                        std::thread::sleep(LOAD_BASELINE);
                    }
                    mode = next;
                    // A mode switch samples the cheap sensors at once. BIOS modes keep their own
                    // 30 s clock: a cctk read takes ~7 s and would delay a click made right after
                    // opening a window.
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
        if self.last_slow.is_none_or(|t| t.elapsed() >= IDLE_INTERVAL) {
            self.last_slow = Some(Instant::now());
            self.slow_tick(raw.as_ref());
        }
        if self.last_bios.is_none_or(|t| t.elapsed() >= IDLE_INTERVAL) {
            self.last_bios = Some(Instant::now());
            self.spawn_bios_refresh();
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
        core.history.lock_ok().push(HistorySample::from(&telemetry));
        *core.telemetry.lock_ok() = Some(telemetry.clone());
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
            let _ = core.health.lock_ok().record(entry);
        }
        let optimizer_running = platform::optimizer_running();
        core.update(|s| {
            s.battery = battery;
            s.availability.optimizer_running = optimizer_running;
        });
        let after = core.state();
        if after.availability != before.availability {
            let _ = self.app.emit("state-changed", &after);
        }
    }

    /// Re-reads the BIOS modes off the poller thread: the cctk process takes ~7 s, which would
    /// otherwise freeze the 1 s telemetry stream.
    fn spawn_bios_refresh(&self) {
        if self.bios_busy.swap(true, Ordering::AcqRel) {
            return;
        }
        let app = self.app.clone();
        let busy = self.bios_busy.clone();
        std::thread::spawn(move || {
            let core = app.state::<Core>();
            let before = core.state();
            let after = core.refresh_bios();
            crate::tray::refresh(&app, &after, None);
            if after.active_profile != before.active_profile || after.thermal != before.thermal {
                let _ = app.emit("state-changed", &after);
            }
            busy.store(false, Ordering::Release);
        });
    }
}
