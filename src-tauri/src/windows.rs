//! Flyout and full-window lifecycle.
//!
//! Windows are built hidden and shown by `reveal` once their page has painted its first state, so they never
//! flash blank. A dismissed flyout is only hidden, so the next tray click shows it at once; after `FLYOUT_KEEP`
//! unused it is destroyed, and the tray-only app holds no WebView again.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use chrono::Local;
use myprecision_core::tray_icon::{BlurDebounce, ToggleGuard};
use tauri::window::{Effect, EffectsBuilder};
use tauri::{AppHandle, Manager, PhysicalPosition, WebviewUrl, WebviewWindow, WebviewWindowBuilder, WindowEvent};

use crate::state::{Core, PollMode};
use crate::sync::LockExt;

pub const FLYOUT: &str = "flyout";
pub const MAIN: &str = "main";

const FLYOUT_W: f64 = 360.0;
const FLYOUT_H: f64 = 520.0;
const FLYOUT_MARGIN: f64 = 12.0;
/// How long a hidden flyout stays warm for an instant reopen.
const FLYOUT_KEEP: Duration = Duration::from_secs(10 * 60);
/// A page that never reports ready is shown anyway after this long.
const REVEAL_FALLBACK: Duration = Duration::from_millis(1500);

#[derive(Default)]
pub struct WindowsState {
    guard: Mutex<ToggleGuard>,
    /// Physical centre of the tray icon at the last click: picks the monitor for the flyout.
    tray_point: Mutex<Option<(f64, f64)>>,
    /// Bumped on every flyout show: a release timer only fires if no show happened since it was armed.
    flyout_shows: AtomicU64,
}

pub fn set_tray_point(app: &AppHandle, x: f64, y: f64) {
    *app.state::<WindowsState>().tray_point.lock_ok() = Some((x, y));
}

/// Mouse down on the tray icon: the moment an open flyout loses focus.
pub fn tray_pressed(app: &AppHandle) {
    app.state::<WindowsState>().guard.lock_ok().on_pressed(Local::now().timestamp_millis());
}

pub fn toggle_flyout(app: &AppHandle) {
    let existing = app.get_webview_window(FLYOUT);
    if let Some(w) = &existing
        && w.is_visible().unwrap_or(false)
    {
        hide_flyout(app, w);
        return;
    }
    // A click that arrives right after the flyout hid on blur is the same click that blurred it.
    if !app.state::<WindowsState>().guard.lock_ok().should_open(Local::now().timestamp_millis()) {
        return;
    }
    match existing {
        Some(w) => show_flyout(app, &w),
        None => build_flyout(app),
    }
}

/// Builds the flyout hidden; its page calls `window_ready` once painted, which shows it.
fn build_flyout(app: &AppHandle) {
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
    watch(app, &w);
    reveal_fallback(&w);
    let handle = app.clone();
    let blur = Arc::new(Mutex::new(BlurDebounce::default()));
    w.on_window_event(move |e| {
        let WindowEvent::Focused(focused) = *e else { return };
        let blurred_at = Local::now().timestamp_millis();
        blur.lock_ok().on_focus(focused, blurred_at);
        if focused {
            return;
        }
        let (handle, blur) = (handle.clone(), blur.clone());
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(BlurDebounce::SETTLE_MS as u64));
            if !blur.lock_ok().should_hide(Local::now().timestamp_millis()) {
                return;
            }
            let Some(w) = handle.get_webview_window(FLYOUT) else { return };
            if !w.is_visible().unwrap_or(false) {
                return;
            }
            handle.state::<WindowsState>().guard.lock_ok().on_hidden(blurred_at);
            hide_flyout(&handle, &w);
        });
    });
}

fn show_flyout(app: &AppHandle, w: &WebviewWindow) {
    app.state::<WindowsState>().flyout_shows.fetch_add(1, Ordering::SeqCst);
    place_flyout(app, w);
    let _ = w.show();
    let _ = w.set_focus();
    update_poll_mode(app, None);
}

/// Hides the flyout and arms its release: destroyed after `FLYOUT_KEEP` unless shown again first.
fn hide_flyout(app: &AppHandle, w: &WebviewWindow) {
    let _ = w.hide();
    update_poll_mode(app, None);
    let armed_at = app.state::<WindowsState>().flyout_shows.load(Ordering::SeqCst);
    let handle = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(FLYOUT_KEEP);
        if handle.state::<WindowsState>().flyout_shows.load(Ordering::SeqCst) != armed_at {
            return;
        }
        if let Some(w) = handle.get_webview_window(FLYOUT)
            && !w.is_visible().unwrap_or(true)
        {
            let _ = w.destroy();
        }
    });
}

/// Called by a window's page once it has painted its first state.
pub fn reveal(w: &WebviewWindow) {
    if w.is_visible().unwrap_or(false) {
        return;
    }
    let app = w.app_handle();
    if w.label() == FLYOUT {
        show_flyout(app, w);
    } else {
        let _ = w.show();
        let _ = w.set_focus();
    }
}

/// Shows the window even if its page never reports ready, rather than leaving the click without effect.
fn reveal_fallback(w: &WebviewWindow) {
    let w = w.clone();
    std::thread::spawn(move || {
        std::thread::sleep(REVEAL_FALLBACK);
        reveal(&w);
    });
}

/// Bottom-right of the work area of the monitor that holds the tray icon.
fn place_flyout(app: &AppHandle, w: &WebviewWindow) {
    let point = *app.state::<WindowsState>().tray_point.lock_ok();
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
    if let Some(f) = app.get_webview_window(FLYOUT)
        && f.is_visible().unwrap_or(false)
    {
        hide_flyout(app, &f);
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
        .visible(false)
        .build();
    if let Ok(w) = built {
        watch(app, &w);
        reveal_fallback(&w);
    }
}

/// Live sampling while the full window exists or the flyout is on screen; tray-only polling otherwise.
fn update_poll_mode(app: &AppHandle, closing: Option<&str>) {
    let active = app
        .webview_windows()
        .iter()
        .any(|(label, w)| Some(label.as_str()) != closing && (label != FLYOUT || w.is_visible().unwrap_or(false)));
    app.state::<Core>().set_poll_mode(if active { PollMode::Active } else { PollMode::Idle });
}

fn watch(app: &AppHandle, w: &WebviewWindow) {
    update_poll_mode(app, None);
    let handle = app.clone();
    let label = w.label().to_owned();
    w.on_window_event(move |e| {
        if let WindowEvent::Destroyed = e {
            update_poll_mode(&handle, Some(&label));
        }
    });
}
