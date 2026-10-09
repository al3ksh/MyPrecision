//! Tray icon: battery glyph tinted by profile, tooltip, native menu.

use std::sync::Mutex;

use myprecision_core::dell::{ChargeCfg, ThermalMode};
use myprecision_core::profile::{ActiveProfile, BatteryProfile, Profiles};
use myprecision_core::tray_icon::{icon_color, render_battery_icon};
use tauri::image::Image;
use tauri::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Listener, Manager, Wry};

use crate::commands;
use crate::state::{AppState, Core};
use crate::windows;

const TRAY_ID: &str = "main";
const ICON_SIZE: u32 = 32;

const PROFILES: [(BatteryProfile, &str); 3] = [
    (BatteryProfile::Home, "Home"),
    (BatteryProfile::Campus, "Campus"),
    (BatteryProfile::Storage, "Storage"),
];
const THERMALS: [(ThermalMode, &str); 4] = [
    (ThermalMode::Optimized, "Optimized"),
    (ThermalMode::Cool, "Cool"),
    (ThermalMode::Quiet, "Quiet"),
    (ThermalMode::UltraPerformance, "Ultra Performance"),
];

struct TrayItems {
    profiles: Vec<CheckMenuItem<Wry>>,
    thermals: Vec<CheckMenuItem<Wry>>,
    autostart: CheckMenuItem<Wry>,
}

#[derive(Default)]
struct TrayCache {
    /// Last CPU temperature from the live poller; kept for the tooltip in Idle.
    cpu_c: Option<f32>,
    /// Icon colour and fill last pushed to the shell — skip identical updates.
    icon: Option<([u8; 3], Option<u8>)>,
    tooltip: String,
}

pub struct TrayState {
    items: TrayItems,
    cache: Mutex<TrayCache>,
}

fn charge_label(cfg: ChargeCfg) -> String {
    match cfg {
        ChargeCfg::Custom { start, stop } => format!("{start}–{stop}%"),
        ChargeCfg::Standard => "100%".into(),
        ChargeCfg::Adaptive => "Adaptive".into(),
        ChargeCfg::PrimAcUse => "Primarily AC".into(),
        ChargeCfg::Express => "ExpressCharge".into(),
    }
}

fn profile_item_text(name: &str, profile: BatteryProfile, profiles: &Profiles) -> String {
    format!("{name} {}", charge_label(profiles.get(profile)))
}

fn active_name(active: &ActiveProfile) -> String {
    match active {
        ActiveProfile::Known { profile } => {
            PROFILES.iter().find(|(p, _)| p == profile).map_or("", |(_, n)| n).to_owned()
        }
        ActiveProfile::Other { cfg: ChargeCfg::Custom { start, stop } } => format!("Custom ({start}–{stop}%)"),
        ActiveProfile::Other { cfg } => charge_label(*cfg),
    }
}

/// `MyPrecision — 82% · Home · CPU 54 °C`, missing parts skipped.
fn tooltip(state: &AppState, cpu_c: Option<f32>) -> String {
    let parts: Vec<String> = [
        state.battery.as_ref().and_then(|b| b.percent).map(|p| format!("{}%", p.round())),
        state.active_profile.as_ref().map(active_name),
        cpu_c.map(|c| format!("CPU {} °C", c.round())),
    ]
    .into_iter()
    .flatten()
    .collect();
    if parts.is_empty() { "MyPrecision".into() } else { format!("MyPrecision — {}", parts.join(" · ")) }
}

pub fn build(app: &AppHandle) -> tauri::Result<()> {
    let state = app.state::<Core>().state();
    let profiles = PROFILES
        .iter()
        .map(|(p, name)| {
            let id = format!("profile:{name}");
            CheckMenuItem::with_id(app, id, profile_item_text(name, *p, &state.profiles), true, false, None::<&str>)
        })
        .collect::<tauri::Result<Vec<_>>>()?;
    let thermals = THERMALS
        .iter()
        .map(|(_, name)| CheckMenuItem::with_id(app, format!("thermal:{name}"), *name, true, false, None::<&str>))
        .collect::<tauri::Result<Vec<_>>>()?;
    let thermal_refs: Vec<&dyn tauri::menu::IsMenuItem<Wry>> = thermals.iter().map(|i| i as _).collect();
    let thermal_menu = Submenu::with_items(app, "Thermal mode", true, &thermal_refs)?;
    let open = MenuItem::with_id(app, "open", "Open MyPrecision", true, None::<&str>)?;
    let autostart = CheckMenuItem::with_id(app, "autostart", "Start at sign-in", true, false, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let sep = || PredefinedMenuItem::separator(app);

    let menu = Menu::new(app)?;
    for item in &profiles {
        menu.append(item)?;
    }
    menu.append(&sep()?)?;
    menu.append(&thermal_menu)?;
    menu.append(&sep()?)?;
    menu.append(&open)?;
    menu.append(&autostart)?;
    menu.append(&sep()?)?;
    menu.append(&quit)?;

    app.manage(TrayState {
        items: TrayItems { profiles, thermals, autostart },
        cache: Mutex::new(TrayCache::default()),
    });

    TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("MyPrecision")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(on_menu)
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                position,
                ..
            } = event
            {
                let app = tray.app_handle();
                windows::set_tray_point(app, position.x, position.y);
                windows::toggle_flyout(app);
            }
        })
        .build(app)?;

    refresh(app, &state, None);
    let handle = app.clone();
    app.listen_any("state-changed", move |_| {
        let state = handle.state::<Core>().state();
        refresh(&handle, &state, None);
    });
    Ok(())
}

fn on_menu(app: &AppHandle, event: MenuEvent) {
    let id = event.id().as_ref().to_owned();
    match id.as_str() {
        "open" => windows::open_full(app),
        "quit" => app.exit(0),
        "autostart" => {
            let handle = app.clone();
            std::thread::spawn(move || {
                let core = handle.state::<Core>();
                // Success and failure both emit state-changed, which re-syncs the check mark.
                let _ = commands::apply_autostart(&handle, &core, !core.state().autostart);
            });
        }
        _ => {
            // cctk takes a few hundred ms per call: keep the menu thread free.
            let handle = app.clone();
            std::thread::spawn(move || {
                let core = handle.state::<Core>();
                let result = if let Some(name) = id.strip_prefix("profile:") {
                    PROFILES.iter().find(|(_, n)| *n == name).map(|(p, _)| commands::apply_battery_profile(&handle, &core, *p))
                } else if let Some(name) = id.strip_prefix("thermal:") {
                    THERMALS.iter().find(|(_, n)| *n == name).map(|(m, _)| commands::apply_thermal_mode(&handle, &core, *m))
                } else {
                    None
                };
                // Success already emitted state-changed; a failure still has to undo the menu's own toggle.
                if let Some(Err(_)) = result {
                    let state = core.state();
                    refresh(&handle, &state, None);
                    let _ = handle.emit("state-changed", &state);
                }
            });
        }
    }
}

/// Icon, tooltip and menu checks from `state`. `cpu_c` updates the remembered CPU temperature.
pub fn refresh(app: &AppHandle, state: &AppState, cpu_c: Option<f32>) {
    let Some(tray_state) = app.try_state::<TrayState>() else { return };
    let Some(tray) = app.tray_by_id(TRAY_ID) else { return };
    let mut cache = tray_state.cache.lock().unwrap();
    if cpu_c.is_some() {
        cache.cpu_c = cpu_c;
    }

    let color = icon_color(state.active_profile.as_ref());
    let fill = state.battery.as_ref().and_then(|b| b.percent).map(|p| p.clamp(0.0, 100.0).round() as u8);
    if cache.icon != Some((color, fill)) {
        let rgba = render_battery_icon(color, fill, ICON_SIZE);
        if tray.set_icon(Some(Image::new_owned(rgba, ICON_SIZE, ICON_SIZE))).is_ok() {
            cache.icon = Some((color, fill));
        }
    }
    let text = tooltip(state, cache.cpu_c);
    if cache.tooltip != text {
        let _ = tray.set_tooltip(Some(&text));
        cache.tooltip = text;
    }
    drop(cache);

    let items = &tray_state.items;
    for ((p, name), item) in PROFILES.iter().zip(&items.profiles) {
        let active = state.active_profile == Some(ActiveProfile::Known { profile: *p });
        let _ = item.set_checked(active);
        let _ = item.set_text(profile_item_text(name, *p, &state.profiles));
    }
    for ((m, _), item) in THERMALS.iter().zip(&items.thermals) {
        let _ = item.set_checked(state.thermal == Some(*m));
    }
    let _ = items.autostart.set_checked(state.autostart);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::Availability;
    use myprecision_core::battery::{BatterySnapshot, PowerState};

    fn state(percent: Option<f32>, active: Option<ActiveProfile>) -> AppState {
        AppState {
            charge: None,
            active_profile: active,
            thermal: None,
            profiles: Profiles::default(),
            battery: percent.map(|p| BatterySnapshot {
                percent: Some(p),
                state: PowerState::Discharging,
                power_w: -10.0,
                remaining_mwh: 0,
                full_mwh: None,
                design_mwh: None,
                wear_pct: None,
                cycles: None,
                voltage_v: 0.0,
            }),
            availability: Availability { cctk: true, dcm: true, admin: true, optimizer_running: false },
            autostart: false,
            optimizer_warning_dismissed: false,
        }
    }

    #[test]
    fn tooltip_full() {
        let s = state(Some(82.4), Some(ActiveProfile::Known { profile: BatteryProfile::Home }));
        assert_eq!(tooltip(&s, Some(54.2)), "MyPrecision — 82% · Home · CPU 54 °C");
    }

    #[test]
    fn tooltip_skips_missing_parts() {
        assert_eq!(tooltip(&state(None, None), None), "MyPrecision");
        let other = ActiveProfile::Other { cfg: ChargeCfg::Custom { start: 60, stop: 90 } };
        assert_eq!(tooltip(&state(Some(50.0), Some(other)), None), "MyPrecision — 50% · Custom (60–90%)");
    }

    #[test]
    fn profile_items_follow_config() {
        let p = Profiles::default();
        assert_eq!(profile_item_text("Home", BatteryProfile::Home, &p), "Home 75–80%");
        assert_eq!(profile_item_text("Campus", BatteryProfile::Campus, &p), "Campus 100%");
        assert_eq!(profile_item_text("Storage", BatteryProfile::Storage, &p), "Storage 50–60%");
    }
}
