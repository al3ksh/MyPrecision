pub mod commands;
pub mod platform;
pub mod poller;
pub mod probe;
pub mod state;

use tauri::{Manager, RunEvent};

use crate::state::Core;

pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|_app, _args, _cwd| {}))
        .invoke_handler(tauri::generate_handler![
            commands::get_state,
            commands::set_battery_profile,
            commands::set_thermal_mode,
            commands::get_history,
            commands::get_health_log,
            commands::set_autostart,
            commands::dismiss_optimizer_warning,
            commands::open_full_window,
        ])
        .setup(|app| {
            let (core, rx) = Core::new();
            app.manage(core);
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
