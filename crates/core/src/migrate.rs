//! Clean-up after the rename from MyPrecision: 0.4.0 and earlier installed under the old product
//! name, so an update to MyStation lands in a new folder and leaves the old install behind.

use std::path::{Path, PathBuf};

/// Uninstall key the MyPrecision installer wrote (HKLM, per-machine install).
pub const LEGACY_UNINSTALL_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall\MyPrecision";

/// The old uninstaller to run, from the registry's `InstallLocation` (written quoted) and the
/// folder this exe runs from. `None` when nothing usable is recorded or it is our own folder.
pub fn legacy_uninstaller(install_location: &str, current_dir: &Path) -> Option<PathBuf> {
    let trim = |s: &str| s.trim_end_matches(['\\', '/']).to_owned();
    let dir = trim(install_location.trim().trim_matches('"'));
    let ours = current_dir.to_str().is_some_and(|c| trim(c).eq_ignore_ascii_case(&dir));
    (!dir.is_empty() && Path::new(&dir).is_absolute() && !ours).then(|| Path::new(&dir).join("uninstall.exe"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn points_at_the_old_uninstaller() {
        let current = Path::new(r"C:\Program Files\MyStation");
        assert_eq!(
            legacy_uninstaller(r#""C:\Program Files\MyPrecision""#, current),
            Some(PathBuf::from(r"C:\Program Files\MyPrecision\uninstall.exe"))
        );
        assert_eq!(
            legacy_uninstaller(r"D:\Apps\MyPrecision", current),
            Some(PathBuf::from(r"D:\Apps\MyPrecision\uninstall.exe"))
        );
    }

    #[test]
    fn never_targets_its_own_folder() {
        let current = Path::new(r"C:\Program Files\MyPrecision");
        assert_eq!(legacy_uninstaller(r#""C:\Program Files\MyPrecision""#, current), None);
        assert_eq!(legacy_uninstaller(r"c:\program files\myprecision\", current), None);
    }

    #[test]
    fn ignores_empty_or_relative_locations() {
        let current = Path::new(r"C:\Program Files\MyStation");
        assert_eq!(legacy_uninstaller("", current), None);
        assert_eq!(legacy_uninstaller(r#""""#, current), None);
        assert_eq!(legacy_uninstaller("MyPrecision", current), None);
    }
}
