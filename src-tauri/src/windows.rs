//! Flyout and full-window lifecycle. No window exists while the app sits in the tray, so no WebView either.

use std::sync::Mutex;

use chrono::Local;
use myprecision_core::tray_icon::ToggleGuard;
use tauri::window::{Effect, EffectsBuilder};
use tauri::{AppHandle, Manager, PhysicalPosition, WebviewUrl, WebviewWindow, WebviewWindowBuilder, WindowEvent};

use crate::state::{Core, PollMode};

pub const FLYOUT: &str = "flyout";
pub const MAIN: &str = "main";

const FLYOUT_W: f64 = 360.0;
const FLYOUT_H: f64 = 520.0;
const FLYOUT_MARGIN: f64 = 12.0;

#[derive(Default)]
pub struct WindowsState {
    guard: Mutex<ToggleGuard>,
    /// Physical centre of the tray icon at the last click: picks the monitor for the flyout.
    tray_point: Mutex<Option<(f64, f64)>>,
}

pub fn set_tray_point(app: &AppHandle, x: f64, y: f64) {
    *app.state::<WindowsState>().tray_point.lock().unwrap() = Some((x, y));
}

pub fn toggle_flyout(app: &AppHandle) {
    if let Some(w) = app.get_webview_window(FLYOUT) {
        let _ = w.close();
        return;
    }
    // A click that arrives right after the flyout hid on blur is the same click that blurred it.
    if !app.state::<WindowsState>().guard.lock().unwrap().should_open(Local::now().timestamp_millis()) {
        return;
    }
    let built = WebviewWindowBuilder::new(app, FLYOUT, WebviewUrl::App("index.html".into()))
        .title("MyPrecision")
        .inner_size(FLYOUT_W, FLYOUT_H)
        .resizable(false)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .transparent(true)
        .shadow(true)
        .effects(EffectsBuilder::new().effect(Effect::Acrylic).build())
        .visible(false)
        .build();
    let Ok(w) = built else { return };
    place_flyout(app, &w);
    let _ = w.show();
    let _ = w.set_focus();
    watch(app, &w);
    let handle = app.clone();
    w.on_window_event(move |e| {
        if let WindowEvent::Focused(false) = e {
            handle.state::<WindowsState>().guard.lock().unwrap().on_hidden(Local::now().timestamp_millis());
            if let Some(w) = handle.get_webview_window(FLYOUT) {
                let _ = w.close();
            }
        }
    });
}

/// Bottom-right of the work area of the monitor that holds the tray icon.
fn place_flyout(app: &AppHandle, w: &WebviewWindow) {
    let point = *app.state::<WindowsState>().tray_point.lock().unwrap();
    let monitor = match point {
        Some((x, y)) => app.monitor_from_point(x, y).ok().flatten(),
        None => None,
    }
    .or_else(|| app.primary_monitor().ok().flatten());
    let Some(m) = monitor else { return };
    let scale = m.scale_factor();
    let area = m.work_area();
    let x = f64::from(area.position.x) + f64::from(area.size.width) - (FLYOUT_W + FLYOUT_MARGIN) * scale;
    let y = f64::from(area.position.y) + f64::from(area.size.height) - (FLYOUT_H + FLYOUT_MARGIN) * scale;
    let _ = w.set_position(PhysicalPosition::new(x.round() as i32, y.round() as i32));
}

pub fn open_full(app: &AppHandle) {
    if let Some(f) = app.get_webview_window(FLYOUT) {
        let _ = f.close();
    }
    if let Some(w) = app.get_webview_window(MAIN) {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
        return;
    }
    let built = WebviewWindowBuilder::new(app, MAIN, WebviewUrl::App("index.html".into()))
        .title("MyPrecision")
        .inner_size(960.0, 640.0)
        .min_inner_size(760.0, 520.0)
        .effects(EffectsBuilder::new().effect(Effect::Mica).build())
        .center()
        .build();
    if let Ok(w) = built {
        let _ = w.set_focus();
        watch(app, &w);
    }
}

/// Live sampling while any window is open; tray-only polling once the last one is gone.
fn watch(app: &AppHandle, w: &WebviewWindow) {
    app.state::<Core>().set_poll_mode(PollMode::Active);
    let handle = app.clone();
    let label = w.label().to_owned();
    w.on_window_event(move |e| {
        if let WindowEvent::Destroyed = e
            && handle.webview_windows().keys().all(|l| *l == label)
        {
            handle.state::<Core>().set_poll_mode(PollMode::Idle);
        }
    });
}
