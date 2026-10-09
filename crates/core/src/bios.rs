//! A curated set of everyday BIOS settings the app may read and change through cctk.
//!
//! Only what is listed here can be written, and only with the values listed here: security,
//! boot order, TPM, passwords and virtualization stay out on purpose.

use serde::Serialize;

use crate::dell::{Cctk, CctkRunner, DellError};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Group {
    Keyboard,
    Power,
    Devices,
    Startup,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Opt {
    pub value: &'static str,
    pub label: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Kind {
    /// `Enabled` / `Disabled`.
    Toggle,
    Choice { choices: &'static [Opt] },
    Number { min: u32, max: u32 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Def {
    pub key: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    pub group: Group,
    #[serde(flatten)]
    pub kind: Kind,
    /// Shown only while this setting is not `Disabled`.
    pub parent: Option<&'static str>,
}

const fn c(value: &'static str, label: &'static str) -> Opt {
    Opt { value, label }
}

const TIMEOUTS: &[Opt] = &[
    c("5s", "5 seconds"),
    c("10s", "10 seconds"),
    c("15s", "15 seconds"),
    c("30s", "30 seconds"),
    c("1m", "1 minute"),
    c("5m", "5 minutes"),
    c("15m", "15 minutes"),
    c("Never", "Never"),
];

const fn def(
    key: &'static str,
    label: &'static str,
    description: &'static str,
    group: Group,
    kind: Kind,
) -> Def {
    Def { key, label, description, group, kind, parent: None }
}

const fn child(mut d: Def, parent: &'static str) -> Def {
    d.parent = Some(parent);
    d
}

use Group::*;
use Kind::*;

/// Values are the ones cctk 5.2 reports as supported on the Precision 5560 (BIOS 1.47).
pub const SETTINGS: &[Def] = &[
    def(
        "KeyboardIllumination",
        "Keyboard backlight",
        "Brightness of the keyboard backlight",
        Keyboard,
        Choice { choices: &[c("Disabled", "Off"), c("Dim", "Dim"), c("Bright", "Bright")] },
    ),
    child(
        def(
            "KbdBacklightTimeoutAc",
            "Backlight timeout when plugged in",
            "How long the backlight stays on after the last key press",
            Keyboard,
            Choice { choices: TIMEOUTS },
        ),
        "KeyboardIllumination",
    ),
    child(
        def(
            "KbdBacklightTimeoutBatt",
            "Backlight timeout on battery",
            "How long the backlight stays on after the last key press",
            Keyboard,
            Choice { choices: TIMEOUTS },
        ),
        "KeyboardIllumination",
    ),
    def("FnLock", "Fn Lock", "Lets Fn + Esc switch the top row between F-keys and media keys", Keyboard, Toggle),
    child(
        def(
            "FnLockMode",
            "Top row default",
            "What the F1–F12 keys do without holding Fn",
            Keyboard,
            Choice { choices: &[c("DisableStandard", "F1–F12"), c("EnableSecondary", "Media keys")] },
        ),
        "FnLock",
    ),
    def("NumLock", "Num Lock at startup", "Turn Num Lock on when the laptop starts", Keyboard, Toggle),
    def(
        "TurboMode",
        "Intel Turbo Boost",
        "Lets the processor run above its base frequency",
        Power,
        Toggle,
    ),
    def("WakeOnAc", "Power on when plugged in", "Turn on when the charger is connected", Power, Toggle),
    def("WakeOnDock", "Power on when docked", "Turn on when a dock is connected", Power, Toggle),
    def(
        "WakeOnLan",
        "Wake on LAN",
        "Let a network packet turn the laptop on",
        Power,
        Choice { choices: &[c("Disabled", "Off"), c("LanOnly", "On")] },
    ),
    def("PowerOnLidOpen", "Power on when the lid opens", "Turn on from off when you open the lid", Power, Toggle),
    def(
        "AutoOn",
        "Scheduled power on",
        "Turn the laptop on at a set time",
        Power,
        Choice { choices: &[c("Disabled", "Off"), c("Everyday", "Every day"), c("Weekdays", "Weekdays")] },
    ),
    child(def("AutoOnHr", "Power-on hour", "0–23", Power, Number { min: 0, max: 23 }), "AutoOn"),
    child(def("AutoOnMn", "Power-on minute", "0–59", Power, Number { min: 0, max: 59 }), "AutoOn"),
    def("Camera", "Camera", "Built-in camera", Devices, Toggle),
    def("Microphone", "Microphone", "Built-in microphone", Devices, Toggle),
    def("InternalSpeaker", "Speakers", "Built-in speakers", Devices, Toggle),
    def("FingerprintReader", "Fingerprint reader", "Fingerprint reader in the power button", Devices, Toggle),
    def("SdCard", "SD card reader", "Built-in SD card reader", Devices, Toggle),
    def("WirelessLan", "Wi-Fi", "Wireless network adapter", Devices, Toggle),
    def("BluetoothDevice", "Bluetooth", "Bluetooth adapter", Devices, Toggle),
    def(
        "WlanAutoSense",
        "Turn off Wi-Fi on wired network",
        "Disable Wi-Fi while a network cable is connected",
        Devices,
        Toggle,
    ),
    def(
        "Fastboot",
        "Fast boot",
        "How much hardware is checked at startup",
        Startup,
        Choice { choices: &[c("Minimal", "Minimal"), c("Thorough", "Thorough"), c("Auto", "Auto")] },
    ),
    def("FullScreenLogo", "Full-screen logo", "Show the Dell logo across the whole screen at startup", Startup, Toggle),
    def(
        "ExtPostTime",
        "Extra startup delay",
        "Extra time to press F2 or F12 at startup",
        Startup,
        Choice { choices: &[c("0s", "None"), c("5s", "5 seconds"), c("10s", "10 seconds")] },
    ),
    def(
        "SignOfLifeByKbdBacklight",
        "Backlight flash at power on",
        "Light the keyboard as soon as the power button is pressed",
        Startup,
        Toggle,
    ),
];

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Setting {
    #[serde(flatten)]
    pub def: Def,
    /// What the BIOS reports now; `None` when it did not report this setting.
    pub value: Option<String>,
}

fn find(key: &str) -> Option<&'static Def> {
    SETTINGS.iter().find(|d| d.key == key)
}

/// Rejects anything outside the whitelist before cctk ever runs.
pub fn validate(key: &str, value: &str) -> Result<(), DellError> {
    let not_allowed = || DellError::NotAllowed { key: key.to_string(), value: value.to_string() };
    let def = find(key).ok_or_else(not_allowed)?;
    let ok = match def.kind {
        Toggle => matches!(value, "Enabled" | "Disabled"),
        Choice { choices } => choices.iter().any(|c| c.value == value),
        Number { min, max } => value.parse::<u32>().is_ok_and(|n| (min..=max).contains(&n)) && !value.starts_with('+'),
    };
    if ok { Ok(()) } else { Err(not_allowed()) }
}

/// Pairs each setting with its value in cctk's `key=value` output.
pub fn parse(output: &str) -> Vec<Setting> {
    SETTINGS
        .iter()
        .map(|def| Setting {
            def: *def,
            value: output
                .lines()
                .filter_map(|l| l.split_once('='))
                .find(|(k, _)| k.trim() == def.key)
                .map(|(_, v)| v.trim().to_string()),
        })
        .collect()
}

/// Every setting in one cctk process (a spawn costs seconds).
pub fn read<R: CctkRunner>(cctk: &Cctk<R>) -> Result<Vec<Setting>, DellError> {
    let keys: Vec<&str> = SETTINGS.iter().map(|d| d.key).collect();
    cctk.read_keys(&keys).map(|out| parse(&out))
}

/// Writes one whitelisted value; exit code 0 means the BIOS accepted it.
pub fn write<R: CctkRunner>(cctk: &Cctk<R>, key: &str, value: &str) -> Result<(), DellError> {
    validate(key, value)?;
    cctk.write_key(key, value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct Fake {
        reply: (i32, &'static str),
        calls: Mutex<Vec<Vec<String>>>,
    }

    impl CctkRunner for Fake {
        fn run(&self, args: &[&str]) -> Result<(i32, String), DellError> {
            self.calls.lock().unwrap().push(args.iter().map(|a| a.to_string()).collect());
            Ok((self.reply.0, self.reply.1.to_string()))
        }
    }

    fn cctk(code: i32, out: &'static str) -> Cctk<Fake> {
        Cctk::new(Fake { reply: (code, out), calls: Mutex::default() })
    }

    fn calls(c: &Cctk<Fake>) -> Vec<Vec<String>> {
        c.runner().calls.lock().unwrap().clone()
    }

    #[test]
    fn serializes_flat_for_the_ui() {
        let s = parse("FnLockMode=DisableStandard
AutoOnHr=7");
        let json = |k: &str| serde_json::to_value(s.iter().find(|x| x.def.key == k).unwrap()).unwrap();
        assert_eq!(
            json("FnLockMode"),
            serde_json::json!({
                "key": "FnLockMode",
                "label": "Top row default",
                "description": "What the F1–F12 keys do without holding Fn",
                "group": "Keyboard",
                "kind": "choice",
                "choices": [
                    { "value": "DisableStandard", "label": "F1–F12" },
                    { "value": "EnableSecondary", "label": "Media keys" }
                ],
                "parent": "FnLock",
                "value": "DisableStandard"
            })
        );
        assert_eq!(json("AutoOnHr")["kind"], "number");
        assert_eq!(json("AutoOnHr")["max"], 23);
        assert_eq!(json("Camera")["kind"], "toggle");
        assert_eq!(json("Camera")["value"], serde_json::Value::Null);
    }

    #[test]
    fn keys_are_unique_and_parents_exist() {
        for (i, d) in SETTINGS.iter().enumerate() {
            assert!(SETTINGS[..i].iter().all(|o| o.key != d.key), "duplicate {}", d.key);
            if let Some(p) = d.parent {
                assert!(find(p).is_some(), "{} has unknown parent {p}", d.key);
            }
        }
    }

    #[test]
    fn nothing_security_related_is_writable() {
        for key in ["SecureBoot", "TpmSecurity", "BootOrder", "Virtualization", "AllowBiosDowngrade", "SetupPwd"] {
            assert!(validate(key, "Disabled").is_err(), "{key} must not be writable");
        }
    }

    #[test]
    fn validates_values_per_kind() {
        assert!(validate("Camera", "Enabled").is_ok());
        assert!(validate("Camera", "Disabled").is_ok());
        assert!(validate("Camera", "Off").is_err());
        assert!(validate("KeyboardIllumination", "Bright").is_ok());
        assert!(validate("KeyboardIllumination", "Auto").is_err());
        assert!(validate("AutoOnHr", "0").is_ok());
        assert!(validate("AutoOnHr", "23").is_ok());
        assert!(validate("AutoOnHr", "24").is_err());
        assert!(validate("AutoOnHr", "+5").is_err());
        assert!(validate("AutoOnHr", "x").is_err());
    }

    #[test]
    fn values_cannot_smuggle_extra_arguments() {
        assert!(validate("Camera", "Enabled --SecureBoot=Disabled").is_err());
        assert!(validate("KeyboardIllumination", "Bright\n--TpmSecurity=Disabled").is_err());
    }

    #[test]
    fn parse_pairs_values_and_leaves_missing_ones_empty() {
        let got = parse("KeyboardIllumination=Bright\r\nCamera = Disabled\r\nSecureBoot=Enabled\r\n");
        let value = |k: &str| got.iter().find(|s| s.def.key == k).unwrap().value.clone();
        assert_eq!(value("KeyboardIllumination").as_deref(), Some("Bright"));
        assert_eq!(value("Camera").as_deref(), Some("Disabled"));
        assert_eq!(value("Microphone"), None);
        assert_eq!(got.len(), SETTINGS.len());
    }

    #[test]
    fn read_is_one_call_with_every_key() {
        let c = cctk(0, "Camera=Enabled");
        read(&c).unwrap();
        let expected: Vec<String> = SETTINGS.iter().map(|d| format!("--{}", d.key)).collect();
        assert_eq!(calls(&c), vec![expected]);
    }

    #[test]
    fn write_is_one_exact_call() {
        let c = cctk(0, "");
        write(&c, "KbdBacklightTimeoutAc", "1m").unwrap();
        assert_eq!(calls(&c), vec![vec!["--KbdBacklightTimeoutAc=1m".to_string()]]);
    }

    #[test]
    fn invalid_write_never_runs_cctk() {
        let c = cctk(0, "");
        let err = write(&c, "SecureBoot", "Disabled").unwrap_err();
        assert!(matches!(err, DellError::NotAllowed { .. }));
        assert!(calls(&c).is_empty());
    }

    #[test]
    fn rejected_write_reports_bios_message() {
        let c = cctk(58, "Setup password is required\r\n");
        let err = write(&c, "Camera", "Disabled").unwrap_err();
        assert_eq!(err, DellError::Failed { code: 58, message: "Setup password is required".into() });
    }
}
