pub mod autostart;
pub mod commands;
pub mod platform;
pub mod poller;
pub mod probe;
pub mod state;
pub mod sync;
pub mod tray;
pub mod windows;

use tauri::{Manager, RunEvent};

use crate::state::Core;

pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| windows::open_full(app)))
        .invoke_handler(tauri::generate_handler![
            commands::get_state,
            commands::set_battery_profile,
            commands::set_thermal_mode,
            commands::get_history,
            commands::get_health_log,
            commands::set_autostart,
            commands::dismiss_optimizer_warning,
            commands::open_full_window,
            commands::window_ready,
            commands::fit_flyout,
            commands::get_device_info,
        ])
        .setup(|app| {
            let (core, rx) = Core::new();
            app.manage(core);
            app.manage(windows::WindowsState::default());
            tray::build(app.handle())?;
            // Starts in Idle: tray only, no WebView.
            poller::spawn(app.handle().clone(), rx);
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|_app, event| {
        // Tray app: stay alive when the last window closes.
        if let RunEvent::ExitRequested { code: None, api, .. } = event {
            api.prevent_exit();
        }
    });
}
