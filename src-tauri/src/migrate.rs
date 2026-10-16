//! One-time clean-up of a MyPrecision (0.4.0 and earlier) install after the update to MyStation.

use std::os::windows::process::CommandExt;
use std::process::Command;

use mystation_core::migrate::{LEGACY_UNINSTALL_KEY, legacy_uninstaller};

use crate::platform::CREATE_NO_WINDOW;

/// Moves autostart to this exe and removes the old install. Runs before the app state is read,
/// so the autostart switch already shows the moved task. Costs one registry read when there is
/// nothing to do.
pub fn run() {
    // Only an installed copy migrates: a dev or portable build must not take over autostart
    // or uninstall the real app.
    if cfg!(debug_assertions) {
        return;
    }
    let Some(dir) = std::env::current_exe().ok().and_then(|e| e.parent().map(ToOwned::to_owned)) else { return };
    if !dir.join("uninstall.exe").is_file() {
        return;
    }
    let Some(location) = crate::platform::registry::hklm_string(LEGACY_UNINSTALL_KEY, "InstallLocation") else {
        return;
    };
    crate::autostart::migrate_legacy();
    // The old uninstaller only stops processes running from its own folder, and in silent mode
    // it keeps app data, which both versions share in %APPDATA%\MyPrecision anyway.
    if let Some(uninstaller) = legacy_uninstaller(&location, &dir).filter(|p| p.is_file()) {
        let _ = Command::new(uninstaller).arg("/S").creation_flags(CREATE_NO_WINDOW).spawn();
    }
}
