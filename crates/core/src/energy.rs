//! Per-app battery energy from the SRUM energy-usage table, as exported by
//! `powercfg /srumutil /csv`. Windows estimates each app's share of CPU, display,
//! disk and network energy per (roughly hourly) epoch, in millijoules.

use std::collections::HashMap;

use chrono::{DateTime, NaiveDateTime, TimeDelta, Utc};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppEnergy {
    pub name: String,
    pub mwh: u64,
    /// The part used while the screen was off.
    pub screen_off_mwh: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnergyReport {
    /// Last 24 hours on battery, most energy first.
    pub day: Vec<AppEnergy>,
    /// Last 7 days on battery, most energy first.
    pub week: Vec<AppEnergy>,
    /// All apps in the window, not just the ones listed.
    pub day_total_mwh: u64,
    pub week_total_mwh: u64,
}

/// How many apps each window lists.
const MAX_APPS: usize = 10;

/// A readable name for a SRUM app ID, or `None` for the pseudo entries that aren't apps.
pub fn app_label(id: &str) -> Option<String> {
    let id = id.trim();
    if id.is_empty() || id == "Unknown" || id.starts_with("EMI_") {
        return None;
    }
    if id.starts_with('\\') {
        // `\Device\HarddiskVolume3\...\svchost.exe [utcsvc]`
        let (path, service) = match id.strip_suffix(']').and_then(|s| s.rsplit_once(" [")) {
            Some((path, service)) => (path, Some(service)),
            None => (id, None),
        };
        let file = path.rsplit('\\').next()?;
        let stem = file.rsplit_once('.').map_or(file, |(stem, _)| stem);
        return Some(match service {
            Some(s) => format!("{stem} ({s})"),
            None => stem.to_string(),
        });
    }
    // Packaged apps: `Microsoft.YourPhone_1.0_x64__8wekyb3d8bbwe`.
    if let Some((family, _)) = id.split_once('_') {
        return family.rsplit('.').next().filter(|s| !s.is_empty()).map(str::to_string);
    }
    Some(id.to_string())
}

#[derive(Default)]
struct Totals {
    /// Lowercased name → (display name, mJ, screen-off mJ).
    apps: HashMap<String, (String, u64, u64)>,
}

impl Totals {
    fn add(&mut self, name: &str, mj: u64, screen_off: bool) {
        let e = self.apps.entry(name.to_lowercase()).or_insert_with(|| (name.to_string(), 0, 0));
        e.1 += mj;
        if screen_off {
            e.2 += mj;
        }
    }

    fn finish(self) -> (Vec<AppEnergy>, u64) {
        let mut apps: Vec<AppEnergy> = self
            .apps
            .into_values()
            .map(|(name, mj, off)| AppEnergy { name, mwh: mj / 3600, screen_off_mwh: off / 3600 })
            .collect();
        let total = apps.iter().map(|a| a.mwh).sum();
        apps.sort_by(|a, b| b.mwh.cmp(&a.mwh).then_with(|| a.name.cmp(&b.name)));
        apps.truncate(MAX_APPS);
        (apps, total)
    }
}

/// Builds the report from the CSV, counting only time on battery.
pub fn report(csv: &str, now: DateTime<Utc>) -> EnergyReport {
    let mut lines = csv.lines();
    let Some(header) = lines.next() else {
        return EnergyReport::default();
    };
    let columns: Vec<&str> = header.trim().split(", ").collect();
    let col = |name: &str| columns.iter().position(|c| *c == name);
    let (Some(app), Some(time), Some(battery), Some(screen), Some(energy)) =
        (col("AppId"), col("TimeStamp"), col("OnBattery"), col("ScreenOn"), col("TotalEnergyConsumption"))
    else {
        return EnergyReport::default();
    };

    let (day_start, week_start) = (now - TimeDelta::hours(24), now - TimeDelta::days(7));
    let (mut day, mut week) = (Totals::default(), Totals::default());
    for line in lines {
        // App IDs are paths that may hold ", ", so split from the right.
        let mut fields: Vec<&str> = line.trim().rsplitn(columns.len(), ", ").collect();
        if fields.len() != columns.len() {
            continue;
        }
        fields.reverse();
        if fields[battery] != "TRUE" {
            continue;
        }
        let Some(t) = NaiveDateTime::parse_from_str(fields[time], "%Y-%m-%dT%H:%M:%S%.f").ok().map(|t| t.and_utc())
        else {
            continue;
        };
        if t < week_start {
            continue;
        }
        let (Some(name), Ok(mj)) = (app_label(fields[app]), fields[energy].parse::<u64>()) else {
            continue;
        };
        let screen_off = fields[screen] == "FALSE";
        week.add(&name, mj, screen_off);
        if t >= day_start {
            day.add(&name, mj, screen_off);
        }
    }
    let (day, day_total_mwh) = day.finish();
    let (week, week_total_mwh) = week.finish();
    EnergyReport { day, week, day_total_mwh, week_total_mwh }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEADER: &str = "AppId, UserId, TimeStamp, OnBattery, ScreenOn, BatterySaverActive, LowPowerEpochActive, \
        Foreground, InteractivityState, Container, Committed, TimeInMSec, MeasuredBitmap, EnergyLoss, \
        CPUEnergyConsumption, SocEnergyConsumption, DisplayEnergyConsumption, DiskEnergyConsumption, \
        NetworkEnergyConsumption, MBBEnergyConsumption, NPUEnergyConsumption, OtherEnergyConsumption, \
        EmiEnergyConsumption, CPUEnergyConsumptionWorkOnBehalf, CPUEnergyConsumptionAttributed, \
        TotalEnergyConsumption";

    fn row(app: &str, time: &str, battery: bool, screen: bool, total_mj: u64) -> String {
        let b = |v: bool| if v { "TRUE" } else { "FALSE" };
        format!(
            "{app}, S-1-5-18, {time}, {}, {}, FALSE, FALSE, TRUE, NotUnique, FALSE, TRUE, 3600000, \
             b0000000000, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, {total_mj}",
            b(battery),
            b(screen)
        )
    }

    fn csv(rows: &[String]) -> String {
        format!("{HEADER}\r\n{}\r\n", rows.join("\r\n"))
    }

    fn now() -> DateTime<Utc> {
        "2026-10-09T12:00:00Z".parse().unwrap()
    }

    #[test]
    fn labels() {
        assert_eq!(
            app_label(r"\Device\HarddiskVolume3\Program Files\BraveSoftware\brave.exe").as_deref(),
            Some("brave")
        );
        assert_eq!(
            app_label(r"\Device\HarddiskVolume3\Windows\System32\svchost.exe [utcsvc]").as_deref(),
            Some("svchost (utcsvc)")
        );
        assert_eq!(app_label("Claude_2.19675.0.0_x64__pzs8sxrjxfjjc").as_deref(), Some("Claude"));
        assert_eq!(app_label("Microsoft.YourPhone_1.0_x64__8wekyb3d8bbwe").as_deref(), Some("YourPhone"));
        assert_eq!(app_label("System").as_deref(), Some("System"));
        assert_eq!(app_label("System Interrupts").as_deref(), Some("System Interrupts"));
        assert_eq!(app_label("Unknown"), None);
        assert_eq!(app_label("EMI_RAPL_Package0"), None);
        assert_eq!(app_label(""), None);
    }

    #[test]
    fn sums_battery_energy_per_app_within_each_window() {
        let r = report(
            &csv(&[
                // 1 Wh on battery, screen on, 2 h ago.
                row(r"\Device\HarddiskVolume3\Apps\brave.exe", "2026-10-09T10:00:00.0000", true, true, 3_600_000),
                // Same app, other case and screen off, 3 days ago.
                row(r"\Device\HarddiskVolume3\Apps\Brave.exe", "2026-10-06T10:00:00.0000", true, false, 7_200_000),
                // Two Claude versions merge.
                row("Claude_2.1.0.0_x64__pzs", "2026-10-09T11:00:00.0000", true, true, 1_800_000),
                row("Claude_2.2.0.0_x64__pzs", "2026-10-09T11:00:00.0000", true, false, 3_600_000),
                // Plugged in: ignored.
                row("System", "2026-10-09T11:00:00.0000", false, true, 99_000_000),
                // Older than a week: ignored.
                row("System", "2026-10-01T11:00:00.0000", true, true, 99_000_000),
                // Pseudo entries: ignored.
                row("Unknown", "2026-10-09T11:00:00.0000", true, true, 99_000_000),
            ]),
            now(),
        );
        assert_eq!(
            r.day,
            vec![
                AppEnergy { name: "Claude".into(), mwh: 1500, screen_off_mwh: 1000 },
                AppEnergy { name: "brave".into(), mwh: 1000, screen_off_mwh: 0 },
            ]
        );
        assert_eq!(r.day_total_mwh, 2500);
        assert_eq!(
            r.week,
            vec![
                AppEnergy { name: "brave".into(), mwh: 3000, screen_off_mwh: 2000 },
                AppEnergy { name: "Claude".into(), mwh: 1500, screen_off_mwh: 1000 },
            ]
        );
        assert_eq!(r.week_total_mwh, 4500);
    }

    #[test]
    fn keeps_the_top_apps_but_totals_them_all() {
        let rows: Vec<String> = (1..=12)
            .map(|i| row(&format!(r"\Device\X\app{i:02}.exe"), "2026-10-09T11:00:00.0000", true, true, i * 3600))
            .collect();
        let r = report(&csv(&rows), now());
        assert_eq!(r.day.len(), MAX_APPS);
        assert_eq!(r.day[0].name, "app12");
        assert_eq!(r.day_total_mwh, (1..=12).sum::<u64>());
    }

    #[test]
    fn commas_in_paths_and_bad_lines_are_tolerated() {
        let r = report(
            &csv(&[
                row(r"\Device\X\Tools, Inc\tool.exe", "2026-10-09T11:00:00.0000", true, true, 3600),
                "garbage, line".into(),
                String::new(),
            ]),
            now(),
        );
        assert_eq!(r.day, vec![AppEnergy { name: "tool".into(), mwh: 1, screen_off_mwh: 0 }]);
        assert_eq!(report("", now()), EnergyReport::default());
    }
}
