//! Automatic profiles: the rules stored in config.json and a pure engine the poller runs on every
//! tick. The engine reacts only to events (AC plugged or unplugged, a schedule moment passing,
//! a long stretch on AC), so a manual change holds until the next event.

use chrono::{Datelike, Duration, NaiveDateTime, NaiveTime};
use serde::{Deserialize, Serialize};

use crate::dell::ThermalMode;
use crate::profile::BatteryProfile;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ThermalRule {
    pub enabled: bool,
    /// `None`: leave the mode alone.
    pub on_ac: Option<ThermalMode>,
    pub on_battery: Option<ThermalMode>,
}

/// A moment to switch the battery profile, not a range.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleEntry {
    /// 0 = Monday … 6 = Sunday.
    pub days: Vec<u8>,
    /// Local time, `HH:MM`.
    pub time: String,
    pub profile: BatteryProfile,
}

impl ScheduleEntry {
    fn time(&self) -> Option<NaiveTime> {
        NaiveTime::parse_from_str(&self.time, "%H:%M").ok()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ScheduleRule {
    pub enabled: bool,
    pub entries: Vec<ScheduleEntry>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LongAcRule {
    pub enabled: bool,
    pub days: u32,
    pub profile: BatteryProfile,
}

impl Default for LongAcRule {
    fn default() -> Self {
        Self { enabled: false, days: 3, profile: BatteryProfile::Storage }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Automation {
    pub thermal: ThermalRule,
    pub schedule: ScheduleRule,
    pub long_ac: LongAcRule,
    /// A Windows notification for every automatic change.
    pub notify: bool,
}

impl Default for Automation {
    fn default() -> Self {
        Self {
            thermal: ThermalRule::default(),
            schedule: ScheduleRule::default(),
            long_ac: LongAcRule::default(),
            notify: true,
        }
    }
}

pub const LONG_AC_DAYS: std::ops::RangeInclusive<u32> = 1..=60;

/// Rejects rules the UI should never send; the message reaches the user as is.
pub fn validate(rules: &Automation) -> Result<(), String> {
    for e in &rules.schedule.entries {
        if e.days.is_empty() || e.days.iter().any(|&d| d > 6) {
            return Err("Each schedule entry needs at least one day.".into());
        }
        if e.time().is_none() {
            return Err(format!("Invalid time: {}.", e.time));
        }
    }
    if !LONG_AC_DAYS.contains(&rules.long_ac.days) {
        return Err(format!(
            "Days on AC must be between {} and {}.",
            LONG_AC_DAYS.start(),
            LONG_AC_DAYS.end()
        ));
    }
    Ok(())
}

/// What the engine remembers between ticks and across restarts (`automation-state.json`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AutomationState {
    pub on_ac: Option<bool>,
    /// Start of the current continuous AC period.
    pub ac_since: Option<NaiveDateTime>,
    /// The long-AC profile was already applied in this AC period.
    pub long_ac_fired: bool,
    /// The last schedule moment handled; later moments are due.
    pub last_schedule: Option<NaiveDateTime>,
}

impl AutomationState {
    /// After the rules change, only moments from now on count: editing never fires a past entry.
    pub fn rules_changed(&mut self, now: NaiveDateTime) {
        self.last_schedule = Some(now);
    }
}

/// Missing or unreadable state starts fresh: the next tick only records the power source.
pub fn load_state(path: &std::path::Path) -> AutomationState {
    std::fs::read_to_string(path).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

pub fn save_state(path: &std::path::Path, state: &AutomationState) -> std::io::Result<()> {
    crate::config::write_atomic(path, serde_json::to_string_pretty(state)?.as_bytes())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Reason {
    OnAc,
    OnBattery,
    Schedule,
    LongAc,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Thermal { mode: ThermalMode, reason: Reason },
    Battery { profile: BatteryProfile, reason: Reason },
}

/// The latest moment any entry passed at or before `now` (looking back a week) and its profile.
pub fn last_moment(entries: &[ScheduleEntry], now: NaiveDateTime) -> Option<(NaiveDateTime, BatteryProfile)> {
    let mut latest: Option<(NaiveDateTime, BatteryProfile)> = None;
    for e in entries {
        let Some(time) = e.time() else { continue };
        let moment = (0..=7)
            .map(|back| (now.date() - Duration::days(back)).and_time(time))
            .find(|m| *m <= now && e.days.contains(&(m.weekday().num_days_from_monday() as u8)));
        if let Some(m) = moment
            && latest.is_none_or(|(l, _)| m > l)
        {
            latest = Some((m, e.profile));
        }
    }
    latest
}

/// One engine step. Mutates `state`; the caller persists it when it changed.
pub fn evaluate(state: &mut AutomationState, on_ac: bool, now: NaiveDateTime, rules: &Automation) -> Vec<Action> {
    let mut actions = Vec::new();
    // Several events in one tick still mean one write: the highest-priority profile wins.
    let mut battery = None;

    if state.on_ac != Some(on_ac) {
        // The first tick ever has nothing to compare with: not an event.
        let known = state.on_ac.is_some();
        state.on_ac = Some(on_ac);
        if known && rules.thermal.enabled {
            let (mode, reason) = if on_ac {
                (rules.thermal.on_ac, Reason::OnAc)
            } else {
                (rules.thermal.on_battery, Reason::OnBattery)
            };
            if let Some(mode) = mode {
                actions.push(Action::Thermal { mode, reason });
            }
        }
        if on_ac {
            state.ac_since = Some(now);
        } else {
            // Leaving AC ends the storage stretch: back to whatever the schedule says now.
            if state.long_ac_fired && rules.schedule.enabled {
                battery = last_moment(&rules.schedule.entries, now).map(|(_, p)| (p, Reason::Schedule));
            }
            state.ac_since = None;
        }
        state.long_ac_fired = false;
    }

    if on_ac
        && rules.long_ac.enabled
        && !state.long_ac_fired
        && let Some(since) = state.ac_since
        && now - since >= Duration::days(rules.long_ac.days.into())
    {
        state.long_ac_fired = true;
        battery = Some((rules.long_ac.profile, Reason::LongAc));
    }

    match state.last_schedule {
        None => state.last_schedule = Some(now),
        Some(last) => {
            if let Some((moment, profile)) = last_moment(&rules.schedule.entries, now)
                && moment > last
            {
                // A disabled schedule still consumes its moments, so enabling it never fires a stale one.
                state.last_schedule = Some(moment);
                if rules.schedule.enabled && !(on_ac && state.long_ac_fired) {
                    battery = Some((profile, Reason::Schedule));
                }
            }
        }
    }

    if let Some((profile, reason)) = battery {
        actions.push(Action::Battery { profile, reason });
    }
    actions
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    // 2026-10-05 is a Monday.
    fn at(day: u32, h: u32, m: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 10, day).unwrap().and_hms_opt(h, m, 0).unwrap()
    }

    fn entry(days: &[u8], time: &str, profile: BatteryProfile) -> ScheduleEntry {
        ScheduleEntry { days: days.to_vec(), time: time.into(), profile }
    }

    fn thermal(on_ac: Option<ThermalMode>, on_battery: Option<ThermalMode>) -> Automation {
        Automation { thermal: ThermalRule { enabled: true, on_ac, on_battery }, ..Automation::default() }
    }

    fn schedule(entries: Vec<ScheduleEntry>) -> Automation {
        Automation { schedule: ScheduleRule { enabled: true, entries }, ..Automation::default() }
    }

    fn long_ac(mut rules: Automation) -> Automation {
        rules.long_ac.enabled = true;
        rules
    }

    fn seen(on_ac: bool, now: NaiveDateTime) -> AutomationState {
        AutomationState { on_ac: Some(on_ac), ac_since: on_ac.then_some(now), last_schedule: Some(now), ..Default::default() }
    }

    #[test]
    fn first_tick_only_records_the_power_source() {
        let mut s = AutomationState::default();
        let rules = thermal(Some(ThermalMode::UltraPerformance), Some(ThermalMode::Quiet));
        assert_eq!(evaluate(&mut s, true, at(5, 10, 0), &rules), vec![]);
        assert_eq!(s.on_ac, Some(true));
        assert_eq!(s.ac_since, Some(at(5, 10, 0)));
        assert_eq!(s.last_schedule, Some(at(5, 10, 0)));
    }

    #[test]
    fn unplugging_sets_the_battery_mode() {
        let mut s = seen(true, at(5, 9, 0));
        let rules = thermal(Some(ThermalMode::UltraPerformance), Some(ThermalMode::Quiet));
        assert_eq!(
            evaluate(&mut s, false, at(5, 10, 0), &rules),
            vec![Action::Thermal { mode: ThermalMode::Quiet, reason: Reason::OnBattery }]
        );
        assert_eq!(s.ac_since, None);
    }

    #[test]
    fn plugging_in_sets_the_ac_mode_and_starts_the_ac_clock() {
        let mut s = seen(false, at(5, 9, 0));
        let rules = thermal(Some(ThermalMode::UltraPerformance), Some(ThermalMode::Quiet));
        assert_eq!(
            evaluate(&mut s, true, at(5, 10, 0), &rules),
            vec![Action::Thermal { mode: ThermalMode::UltraPerformance, reason: Reason::OnAc }]
        );
        assert_eq!(s.ac_since, Some(at(5, 10, 0)));
    }

    #[test]
    fn dont_change_leaves_the_mode_alone() {
        let mut s = seen(true, at(5, 9, 0));
        let rules = thermal(Some(ThermalMode::UltraPerformance), None);
        assert_eq!(evaluate(&mut s, false, at(5, 10, 0), &rules), vec![]);
    }

    #[test]
    fn disabled_thermal_rule_does_nothing() {
        let mut s = seen(true, at(5, 9, 0));
        let mut rules = thermal(Some(ThermalMode::UltraPerformance), Some(ThermalMode::Quiet));
        rules.thermal.enabled = false;
        assert_eq!(evaluate(&mut s, false, at(5, 10, 0), &rules), vec![]);
    }

    #[test]
    fn steady_power_source_is_not_an_event() {
        let mut s = seen(false, at(5, 9, 0));
        let rules = thermal(Some(ThermalMode::UltraPerformance), Some(ThermalMode::Quiet));
        assert_eq!(evaluate(&mut s, false, at(5, 10, 0), &rules), vec![]);
    }

    #[test]
    fn a_passed_schedule_moment_sets_its_profile_once() {
        let rules = schedule(vec![entry(&[0, 1, 2, 3, 4], "08:00", BatteryProfile::Campus)]);
        let mut s = seen(false, at(5, 7, 59));
        assert_eq!(
            evaluate(&mut s, false, at(5, 8, 0), &rules),
            vec![Action::Battery { profile: BatteryProfile::Campus, reason: Reason::Schedule }]
        );
        assert_eq!(evaluate(&mut s, false, at(5, 8, 1), &rules), vec![]);
    }

    #[test]
    fn schedule_skips_days_not_listed() {
        // 2026-10-10 is a Saturday.
        let rules = schedule(vec![entry(&[0, 1, 2, 3, 4], "08:00", BatteryProfile::Campus)]);
        let mut s = seen(false, at(10, 7, 59));
        assert_eq!(evaluate(&mut s, false, at(10, 8, 0), &rules), vec![]);
    }

    #[test]
    fn schedule_crosses_midnight() {
        let rules = schedule(vec![entry(&[0], "23:59", BatteryProfile::Home)]);
        let mut s = seen(false, at(5, 23, 58));
        assert_eq!(
            evaluate(&mut s, false, at(6, 0, 1), &rules),
            vec![Action::Battery { profile: BatteryProfile::Home, reason: Reason::Schedule }]
        );
    }

    #[test]
    fn after_a_restart_only_the_latest_missed_moment_runs() {
        let rules = schedule(vec![
            entry(&[0, 1, 2, 3, 4, 5, 6], "08:00", BatteryProfile::Campus),
            entry(&[0, 1, 2, 3, 4, 5, 6], "18:00", BatteryProfile::Home),
        ]);
        // Saved Monday evening, back on Wednesday at noon.
        let mut s = AutomationState { on_ac: Some(false), last_schedule: Some(at(5, 20, 0)), ..Default::default() };
        assert_eq!(
            evaluate(&mut s, false, at(7, 12, 0), &rules),
            vec![Action::Battery { profile: BatteryProfile::Campus, reason: Reason::Schedule }]
        );
        assert_eq!(evaluate(&mut s, false, at(7, 12, 1), &rules), vec![]);
    }

    #[test]
    fn editing_the_rules_never_fires_a_past_moment() {
        let rules = schedule(vec![entry(&[0], "08:00", BatteryProfile::Campus)]);
        let mut s = AutomationState { on_ac: Some(false), last_schedule: Some(at(4, 20, 0)), ..Default::default() };
        s.rules_changed(at(5, 10, 0));
        assert_eq!(evaluate(&mut s, false, at(5, 10, 1), &rules), vec![]);
    }

    #[test]
    fn disabled_schedule_does_not_fire_later_on_enable() {
        let mut rules = schedule(vec![entry(&[0], "08:00", BatteryProfile::Campus)]);
        rules.schedule.enabled = false;
        let mut s = seen(false, at(5, 7, 0));
        assert_eq!(evaluate(&mut s, false, at(5, 9, 0), &rules), vec![]);
        rules.schedule.enabled = true;
        assert_eq!(evaluate(&mut s, false, at(5, 9, 1), &rules), vec![]);
    }

    #[test]
    fn long_ac_sets_storage_once_per_ac_period() {
        let rules = long_ac(Automation::default());
        let mut s = seen(true, at(5, 10, 0));
        assert_eq!(evaluate(&mut s, true, at(8, 9, 59), &rules), vec![]);
        assert_eq!(
            evaluate(&mut s, true, at(8, 10, 0), &rules),
            vec![Action::Battery { profile: BatteryProfile::Storage, reason: Reason::LongAc }]
        );
        assert_eq!(evaluate(&mut s, true, at(9, 10, 0), &rules), vec![]);
    }

    #[test]
    fn long_ac_beats_the_schedule_while_on_ac() {
        let rules = long_ac(schedule(vec![entry(&[0, 1, 2, 3, 4, 5, 6], "08:00", BatteryProfile::Home)]));
        let mut s = seen(true, at(5, 7, 0));
        evaluate(&mut s, true, at(8, 7, 0), &rules);
        assert!(s.long_ac_fired);
        assert_eq!(evaluate(&mut s, true, at(8, 8, 0), &rules), vec![]);
    }

    #[test]
    fn unplugging_after_long_ac_restores_the_scheduled_profile() {
        let rules = long_ac(schedule(vec![entry(&[0, 1, 2, 3, 4, 5, 6], "08:00", BatteryProfile::Home)]));
        let mut s = seen(true, at(5, 7, 0));
        evaluate(&mut s, true, at(8, 7, 0), &rules);
        assert_eq!(
            evaluate(&mut s, false, at(8, 12, 0), &rules),
            vec![Action::Battery { profile: BatteryProfile::Home, reason: Reason::Schedule }]
        );
        assert!(!s.long_ac_fired);
    }

    #[test]
    fn a_new_ac_period_restarts_the_long_ac_clock() {
        let rules = long_ac(Automation::default());
        let mut s = seen(true, at(5, 10, 0));
        evaluate(&mut s, false, at(7, 10, 0), &rules);
        evaluate(&mut s, true, at(7, 11, 0), &rules);
        assert_eq!(evaluate(&mut s, true, at(8, 10, 0), &rules), vec![]);
    }

    #[test]
    fn state_survives_a_restart() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("automation-state.json");
        let s = AutomationState { on_ac: Some(true), ac_since: Some(at(5, 10, 0)), long_ac_fired: true, last_schedule: Some(at(6, 8, 0)) };
        save_state(&path, &s).unwrap();
        assert_eq!(load_state(&path), s);
        std::fs::write(&path, "{nope").unwrap();
        assert_eq!(load_state(&path), AutomationState::default());
    }

    #[test]
    fn validate_rejects_bad_entries() {
        assert!(validate(&schedule(vec![entry(&[], "08:00", BatteryProfile::Home)])).is_err());
        assert!(validate(&schedule(vec![entry(&[7], "08:00", BatteryProfile::Home)])).is_err());
        assert!(validate(&schedule(vec![entry(&[0], "8am", BatteryProfile::Home)])).is_err());
        let mut rules = Automation::default();
        rules.long_ac.days = 0;
        assert!(validate(&rules).is_err());
        assert!(validate(&Automation::default()).is_ok());
    }

    #[test]
    fn rules_serialize_for_the_ui() {
        let json = serde_json::to_value(schedule(vec![entry(&[0], "08:00", BatteryProfile::Home)])).unwrap();
        assert_eq!(json["schedule"]["entries"][0]["profile"], "home");
        assert_eq!(json["longAc"]["days"], 3);
        assert_eq!(json["thermal"]["onAc"], serde_json::Value::Null);
    }
}
