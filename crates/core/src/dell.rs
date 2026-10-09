//! Dell Command | Configure (`cctk.exe`) BIOS settings: domain types, parsing and a serialized runner.

use std::sync::Mutex;

use serde::{Deserialize, Serialize, Serializer};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum ChargeCfg {
    Standard,
    Adaptive,
    PrimAcUse,
    Express,
    Custom { start: u8, stop: u8 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThermalMode {
    Optimized,
    Cool,
    Quiet,
    UltraPerformance,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DellError {
    #[error("Dell Command | Configure (cctk.exe) not found.")]
    NotInstalled,
    #[error("Invalid charge range: {start}–{stop}%.")]
    InvalidRange { start: u8, stop: u8 },
    #[error("Could not read the BIOS setting.")]
    Unparsable(String),
    #[error("BIOS rejected the change (code {code}): {message}")]
    Failed { code: i32, message: String },
    #[error("{key} cannot be set to \"{value}\" from MyPrecision.")]
    NotAllowed { key: String, value: String },
}

impl Serialize for DellError {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

pub fn validate_custom(start: u8, stop: u8) -> Result<(), DellError> {
    let ok = (50..=95).contains(&start) && (55..=100).contains(&stop) && stop >= start.saturating_add(5);
    if ok { Ok(()) } else { Err(DellError::InvalidRange { start, stop }) }
}

/// Returns the trimmed value of the first `key=value` line for `key`.
fn value_for<'a>(output: &'a str, key: &str) -> Result<&'a str, DellError> {
    output
        .lines()
        .filter_map(|l| l.split_once('='))
        .find(|(k, _)| k.trim() == key)
        .map(|(_, v)| v.trim())
        .ok_or_else(|| DellError::Unparsable(output.trim().to_string()))
}

pub fn parse_charge_cfg(output: &str) -> Result<ChargeCfg, DellError> {
    let unparsable = || DellError::Unparsable(output.trim().to_string());
    match value_for(output, "PrimaryBattChargeCfg")? {
        "Standard" => Ok(ChargeCfg::Standard),
        "Adaptive" => Ok(ChargeCfg::Adaptive),
        "PrimAcUse" => Ok(ChargeCfg::PrimAcUse),
        "Express" => Ok(ChargeCfg::Express),
        v => {
            let (start, stop) = v.strip_prefix("Custom:").and_then(|r| r.split_once('-')).ok_or_else(unparsable)?;
            let start = start.trim().parse().map_err(|_| unparsable())?;
            let stop = stop.trim().parse().map_err(|_| unparsable())?;
            Ok(ChargeCfg::Custom { start, stop })
        }
    }
}

pub fn format_charge_cfg(cfg: ChargeCfg) -> String {
    match cfg {
        ChargeCfg::Standard => "Standard".into(),
        ChargeCfg::Adaptive => "Adaptive".into(),
        ChargeCfg::PrimAcUse => "PrimAcUse".into(),
        ChargeCfg::Express => "Express".into(),
        ChargeCfg::Custom { start, stop } => format!("Custom:{start}-{stop}"),
    }
}

fn format_thermal(mode: ThermalMode) -> &'static str {
    match mode {
        ThermalMode::Optimized => "Optimized",
        ThermalMode::Cool => "Cool",
        ThermalMode::Quiet => "Quiet",
        ThermalMode::UltraPerformance => "UltraPerformance",
    }
}

pub fn parse_thermal(output: &str) -> Result<ThermalMode, DellError> {
    match value_for(output, "ThermalManagement")? {
        "Optimized" => Ok(ThermalMode::Optimized),
        "Cool" => Ok(ThermalMode::Cool),
        "Quiet" => Ok(ThermalMode::Quiet),
        "UltraPerformance" => Ok(ThermalMode::UltraPerformance),
        _ => Err(DellError::Unparsable(output.trim().to_string())),
    }
}

pub trait CctkRunner: Send + Sync {
    /// Runs cctk with `args`, returning the exit code and combined stdout/stderr.
    fn run(&self, args: &[&str]) -> Result<(i32, String), DellError>;
}

pub struct Cctk<R: CctkRunner> {
    runner: R,
    lock: Mutex<()>,
}

impl<R: CctkRunner> Cctk<R> {
    pub fn new(runner: R) -> Self {
        Self { runner, lock: Mutex::new(()) }
    }

    #[cfg(test)]
    pub(crate) fn runner(&self) -> &R {
        &self.runner
    }

    fn call(&self, arg: &str) -> Result<String, DellError> {
        self.call_many(&[arg])
    }

    fn call_many(&self, args: &[&str]) -> Result<String, DellError> {
        let (code, out) = self.runner.run(args)?;
        if code == 0 { Ok(out) } else { Err(DellError::Failed { code, message: out.trim().to_string() }) }
    }

    fn guard(&self) -> std::sync::MutexGuard<'_, ()> {
        self.lock.lock().unwrap_or_else(|p| p.into_inner())
    }

    pub fn get_charge_cfg(&self) -> Result<ChargeCfg, DellError> {
        let _g = self.guard();
        parse_charge_cfg(&self.call("--PrimaryBattChargeCfg")?)
    }

    /// Both settings in one cctk process: each spawn costs ~7 s on a Precision 5560.
    pub fn read_bios(&self) -> (Option<ChargeCfg>, Option<ThermalMode>) {
        let _g = self.guard();
        match self.call_many(&["--PrimaryBattChargeCfg", "--ThermalManagement"]) {
            Ok(out) => (parse_charge_cfg(&out).ok(), parse_thermal(&out).ok()),
            Err(_) => (None, None),
        }
    }

    /// Writes `cfg`; exit code 0 means the BIOS accepted it. No read-back: a cctk spawn costs
    /// ~7 s, and the poller re-reads the BIOS every 30 s anyway.
    pub fn set_charge_cfg(&self, cfg: ChargeCfg) -> Result<ChargeCfg, DellError> {
        if let ChargeCfg::Custom { start, stop } = cfg {
            validate_custom(start, stop)?;
        }
        let _g = self.guard();
        self.call(&format!("--PrimaryBattChargeCfg={}", format_charge_cfg(cfg)))?;
        Ok(cfg)
    }

    pub fn get_thermal(&self) -> Result<ThermalMode, DellError> {
        let _g = self.guard();
        parse_thermal(&self.call("--ThermalManagement")?)
    }

    /// Writes `mode`; see [`Self::set_charge_cfg`] for why there is no read-back.
    pub fn set_thermal(&self, mode: ThermalMode) -> Result<ThermalMode, DellError> {
        let _g = self.guard();
        self.call(&format!("--ThermalManagement={}", format_thermal(mode)))?;
        Ok(mode)
    }

    /// Raw `key=value` output for `keys`, in one cctk process. Callers validate keys.
    pub(crate) fn read_keys(&self, keys: &[&str]) -> Result<String, DellError> {
        let args: Vec<String> = keys.iter().map(|k| format!("--{k}")).collect();
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        let _g = self.guard();
        self.call_many(&args)
    }

    /// Writes one setting. Callers validate `key` and `value` against a whitelist first.
    pub(crate) fn write_key(&self, key: &str, value: &str) -> Result<(), DellError> {
        let _g = self.guard();
        self.call(&format!("--{key}={value}")).map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    #[derive(Default)]
    struct FakeRunner {
        replies: Mutex<VecDeque<(i32, String)>>,
        calls: Mutex<Vec<Vec<String>>>,
    }

    impl FakeRunner {
        fn with(replies: &[(i32, &str)]) -> Self {
            Self {
                replies: Mutex::new(replies.iter().map(|(c, s)| (*c, s.to_string())).collect()),
                calls: Mutex::default(),
            }
        }
    }

    impl CctkRunner for FakeRunner {
        fn run(&self, args: &[&str]) -> Result<(i32, String), DellError> {
            self.calls.lock().unwrap().push(args.iter().map(|a| a.to_string()).collect());
            Ok(self.replies.lock().unwrap().pop_front().expect("unexpected cctk call"))
        }
    }

    #[test]
    fn parses_standard() {
        assert_eq!(parse_charge_cfg("PrimaryBattChargeCfg=Standard\r\n").unwrap(), ChargeCfg::Standard);
    }

    #[test]
    fn parses_custom() {
        assert_eq!(
            parse_charge_cfg("PrimaryBattChargeCfg=Custom:75-80").unwrap(),
            ChargeCfg::Custom { start: 75, stop: 80 }
        );
    }

    #[test]
    fn parses_all_simple_variants() {
        assert_eq!(parse_charge_cfg("PrimaryBattChargeCfg=Adaptive").unwrap(), ChargeCfg::Adaptive);
        assert_eq!(parse_charge_cfg("PrimaryBattChargeCfg=PrimAcUse").unwrap(), ChargeCfg::PrimAcUse);
        assert_eq!(parse_charge_cfg("PrimaryBattChargeCfg=Express").unwrap(), ChargeCfg::Express);
    }

    #[test]
    fn parses_thermal() {
        assert_eq!(parse_thermal("ThermalManagement=UltraPerformance").unwrap(), ThermalMode::UltraPerformance);
        assert_eq!(parse_thermal("ThermalManagement=Optimized\r\n").unwrap(), ThermalMode::Optimized);
        assert_eq!(parse_thermal("ThermalManagement=Cool").unwrap(), ThermalMode::Cool);
        assert_eq!(parse_thermal("ThermalManagement=Quiet").unwrap(), ThermalMode::Quiet);
    }

    #[test]
    fn garbage_is_unparsable() {
        assert!(matches!(parse_charge_cfg("Error: something"), Err(DellError::Unparsable(_))));
        assert!(matches!(parse_charge_cfg("PrimaryBattChargeCfg=Custom:abc"), Err(DellError::Unparsable(_))));
        assert!(matches!(parse_thermal("ThermalManagement=Turbo"), Err(DellError::Unparsable(_))));
    }

    #[test]
    fn formats_custom() {
        assert_eq!(format_charge_cfg(ChargeCfg::Custom { start: 50, stop: 60 }), "Custom:50-60");
        assert_eq!(format_charge_cfg(ChargeCfg::Standard), "Standard");
        assert_eq!(format_charge_cfg(ChargeCfg::PrimAcUse), "PrimAcUse");
    }

    #[test]
    fn validates_ranges() {
        assert!(validate_custom(50, 55).is_ok());
        assert!(validate_custom(95, 100).is_ok());
        assert!(validate_custom(49, 60).is_err());
        assert!(validate_custom(90, 94).is_err());
        assert!(validate_custom(75, 79).is_err());
        assert!(validate_custom(96, 100).is_err());
        assert!(validate_custom(60, 101).is_err());
    }

    #[test]
    fn set_charge_is_one_call_with_exact_arg() {
        let cctk = Cctk::new(FakeRunner::with(&[(0, "")]));
        let got = cctk.set_charge_cfg(ChargeCfg::Custom { start: 75, stop: 80 }).unwrap();
        assert_eq!(got, ChargeCfg::Custom { start: 75, stop: 80 });
        assert_eq!(*cctk.runner.calls.lock().unwrap(), vec![vec!["--PrimaryBattChargeCfg=Custom:75-80"]]);
    }

    #[test]
    fn set_thermal_is_one_call_with_exact_arg() {
        let cctk = Cctk::new(FakeRunner::with(&[(0, "")]));
        assert_eq!(cctk.set_thermal(ThermalMode::Quiet).unwrap(), ThermalMode::Quiet);
        assert_eq!(*cctk.runner.calls.lock().unwrap(), vec![vec!["--ThermalManagement=Quiet"]]);
    }

    #[test]
    fn read_bios_is_one_call_for_both_settings() {
        let out = "PrimaryBattChargeCfg=Custom:75-80

ThermalManagement=Optimized



";
        let cctk = Cctk::new(FakeRunner::with(&[(0, out)]));
        let got = cctk.read_bios();
        assert_eq!(got, (Some(ChargeCfg::Custom { start: 75, stop: 80 }), Some(ThermalMode::Optimized)));
        assert_eq!(*cctk.runner.calls.lock().unwrap(), vec![vec!["--PrimaryBattChargeCfg", "--ThermalManagement"]]);
    }

    #[test]
    fn read_bios_failure_yields_neither() {
        let cctk = Cctk::new(FakeRunner::with(&[(95, "Administrator rights required")]));
        assert_eq!(cctk.read_bios(), (None, None));
    }

    #[test]
    fn set_invalid_custom_never_calls_runner() {
        let cctk = Cctk::new(FakeRunner::with(&[]));
        let err = cctk.set_charge_cfg(ChargeCfg::Custom { start: 75, stop: 78 }).unwrap_err();
        assert_eq!(err, DellError::InvalidRange { start: 75, stop: 78 });
        assert!(cctk.runner.calls.lock().unwrap().is_empty());
    }

    #[test]
    fn nonzero_exit_is_failed_with_output() {
        let cctk = Cctk::new(FakeRunner::with(&[(58, "Password is required\r\n")]));
        let err = cctk.set_thermal(ThermalMode::Cool).unwrap_err();
        assert_eq!(err, DellError::Failed { code: 58, message: "Password is required".into() });
    }

    struct SlowRunner {
        active: AtomicUsize,
        max: AtomicUsize,
    }

    impl CctkRunner for SlowRunner {
        fn run(&self, args: &[&str]) -> Result<(i32, String), DellError> {
            let now = self.active.fetch_add(1, Ordering::SeqCst) + 1;
            self.max.fetch_max(now, Ordering::SeqCst);
            std::thread::sleep(Duration::from_millis(50));
            self.active.fetch_sub(1, Ordering::SeqCst);
            let out = if args[0].contains('=') { String::new() } else { "ThermalManagement=Cool".into() };
            Ok((0, out))
        }
    }

    #[test]
    fn concurrent_sets_are_serialized() {
        let cctk = Arc::new(Cctk::new(SlowRunner { active: AtomicUsize::new(0), max: AtomicUsize::new(0) }));
        let handles: Vec<_> = (0..2)
            .map(|_| {
                let c = Arc::clone(&cctk);
                std::thread::spawn(move || c.set_thermal(ThermalMode::Cool).unwrap())
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }
        assert_eq!(cctk.runner.max.load(Ordering::SeqCst), 1);
    }
}
