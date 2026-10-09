//! Sleep sessions on battery from `powercfg /sleepstudy /xml`: how much each one
//! drained, how much of it the hardware spent in deep sleep, and what kept it awake.

use chrono::{DateTime, Utc};
use quick_xml::events::{BytesStart, Event};
use quick_xml::{Reader, XmlVersion};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SleepSession {
    pub start: DateTime<Utc>,
    pub minutes: u32,
    pub drained_mwh: u32,
    pub full_mwh: u32,
    /// Share of the session the hardware spent in its deepest low-power state.
    pub deep_pct: u8,
    /// What kept the machine awake the longest, when deep sleep fell short.
    pub blocker: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SleepReport {
    /// Newest first.
    pub sessions: Vec<SleepSession>,
}

/// How many sessions the report keeps.
const MAX_SESSIONS: usize = 14;

/// Shorter sessions say little about drain.
const MIN_MINUTES: u64 = 10;
/// Below this share of deep sleep the report names a culprit.
const GOOD_DEEP_PCT: u8 = 95;
/// Sleep study durations are in 100 ns units.
const TICKS_PER_MINUTE: u64 = 600_000_000;

#[derive(Default)]
struct Raw {
    kind: String,
    start: Option<DateTime<Utc>>,
    duration: u64,
    hw_low: u64,
    ac: bool,
    discharge: u32,
    full: u32,
    /// (name, type, active time)
    blockers: Vec<(String, String, u64)>,
}

fn attr(e: &BytesStart, name: &str) -> Option<String> {
    let a = e.try_get_attribute(name).ok()??;
    Some(a.normalized_value(XmlVersion::Implicit1_0).ok()?.into_owned())
}

fn num<T: std::str::FromStr + Default>(e: &BytesStart, name: &str) -> T {
    attr(e, name).and_then(|v| v.parse().ok()).unwrap_or_default()
}

fn scenario(e: &BytesStart) -> Raw {
    Raw {
        kind: attr(e, "Type").unwrap_or_default(),
        start: attr(e, "TimeStamp").and_then(|t| t.parse().ok()),
        duration: num(e, "Duration"),
        hw_low: num(e, "HwLowPowerStateTime"),
        ac: attr(e, "Ac").as_deref() != Some("0"),
        discharge: num(e, "Discharge"),
        full: num(e, "FullChargeCapacity"),
        blockers: Vec::new(),
    }
}

/// `Intel(R) PEG12 - 9A07 (\_SB.PC00.PEG3)` → `Intel(R) PEG12 - 9A07`; activator codes spelled out.
fn blocker_name(name: &str, kind: &str) -> String {
    let name = match name.strip_suffix(')').and_then(|n| n.rsplit_once(" (\\")) {
        Some((base, _)) => base,
        None => name,
    };
    if kind != "Activator" {
        return name.to_string();
    }
    match name {
        "WU" => "Windows Update",
        "NCSI" => "Network connectivity check",
        "BI" => "Background tasks",
        "Audio Active" => "Audio playing",
        other => other,
    }
    .to_string()
}

impl Raw {
    fn session(self) -> Option<SleepSession> {
        if self.kind != "Sleep" || self.ac || self.duration < MIN_MINUTES * TICKS_PER_MINUTE {
            return None;
        }
        let deep_pct = (self.hw_low.saturating_mul(100) / self.duration).min(100) as u8;
        let blocker = (deep_pct < GOOD_DEEP_PCT)
            .then(|| {
                self.blockers
                    .iter()
                    .filter(|(_, kind, _)| kind != "PDC Phase")
                    .max_by_key(|(_, _, active)| *active)
                    .map(|(name, kind, _)| blocker_name(name, kind))
            })
            .flatten();
        Some(SleepSession {
            start: self.start?,
            minutes: (self.duration / TICKS_PER_MINUTE) as u32,
            drained_mwh: self.discharge,
            full_mwh: self.full,
            deep_pct,
            blocker,
        })
    }
}

/// Builds the report from the sleep study XML.
pub fn report(xml: &str) -> SleepReport {
    let mut reader = Reader::from_str(xml.trim_start_matches('\u{feff}'));
    let mut sessions = Vec::new();
    let mut current: Option<Raw> = None;
    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => match e.local_name().as_ref() {
                "ScenarioInstance" => current = Some(scenario(&e)),
                "TopBlocker" => {
                    if let Some(raw) = current.as_mut() {
                        raw.blockers.push((
                            attr(&e, "Name").unwrap_or_default(),
                            attr(&e, "Type").unwrap_or_default(),
                            num(&e, "ActiveTime"),
                        ));
                    }
                }
                _ => {}
            },
            Ok(Event::End(e)) if e.local_name().as_ref() == "ScenarioInstance" => {
                sessions.extend(current.take().and_then(Raw::session));
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }
    sessions.sort_by_key(|s| std::cmp::Reverse(s.start));
    sessions.truncate(MAX_SESSIONS);
    SleepReport { sessions }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 100 ns units per minute.
    const MIN: u64 = 600_000_000;

    struct S<'a> {
        kind: &'a str,
        time: &'a str,
        minutes: u64,
        ac: u8,
        discharge: u32,
        hw_low: u64,
        blockers: &'a [(&'a str, &'a str, u64)],
    }

    fn scenario(s: &S) -> String {
        let blockers: String = s
            .blockers
            .iter()
            .map(|(name, kind, active)| {
                format!(
                    "<TopBlocker Name=\"{name}\" ScenarioActiveTimePercent=\"1\" ActiveTime=\"{active}\" \
                     ActivityLevel=\"low\" Type=\"{kind}\" Id=\"1\"></TopBlocker>"
                )
            })
            .collect();
        format!(
            "<ScenarioInstance\n  Id=\"11\"\n  Duration=\"{}\"\n  Type=\"{}\"\n  TimeStamp=\"{}\"\n  \
             LowPowerStateTime=\"{}\"\n  HwLowPowerStateTime=\"{}\"\n  FullChargeCapacity=\"59228\"\n  Ac=\"{}\"\n  \
             Discharge=\"{}\"\n  EnergyDrain=\"804\"\n  >\n<TopBlockers>{blockers}</TopBlockers></ScenarioInstance>",
            s.minutes * MIN,
            s.kind,
            s.time,
            s.hw_low,
            s.hw_low,
            s.ac,
            s.discharge
        )
    }

    fn doc(scenarios: &[S]) -> String {
        let body: String = scenarios.iter().map(scenario).collect();
        format!(
            "\u{feff}<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<SleepStudy \
             xmlns=\"http://schemas.microsoft.com/sleepstudy/2012\"><ScenarioInstances>\
             <OsStateInstance Type=\"Screen Off\" Duration=\"884810\" Ac=\"0\"></OsStateInstance>\
             {body}</ScenarioInstances></SleepStudy>"
        )
    }

    fn sleep<'a>(time: &'a str, minutes: u64, hw_low: u64, blockers: &'a [(&'a str, &'a str, u64)]) -> S<'a> {
        S { kind: "Sleep", time, minutes, ac: 0, discharge: 450, hw_low, blockers }
    }

    #[test]
    fn battery_sleeps_newest_first() {
        let r = report(&doc(&[
            sleep("2026-10-02T14:58:54Z", 600, 594 * MIN, &[("Intel(R) PEG12 (\\_SB.PC00.PEG3)", "Fx Device", MIN)]),
            sleep("2026-10-05T20:00:00Z", 120, 120 * MIN, &[]),
            // On AC, too short, or not a sleep: dropped.
            S { ac: 1, ..sleep("2026-10-06T20:00:00Z", 120, 0, &[]) },
            sleep("2026-10-07T20:00:00Z", 5, 0, &[]),
            S { kind: "Screen Off", ..sleep("2026-10-08T20:00:00Z", 120, 0, &[]) },
        ]));
        assert_eq!(
            r.sessions,
            vec![
                SleepSession {
                    start: "2026-10-05T20:00:00Z".parse().unwrap(),
                    minutes: 120,
                    drained_mwh: 450,
                    full_mwh: 59228,
                    deep_pct: 100,
                    blocker: None,
                },
                SleepSession {
                    start: "2026-10-02T14:58:54Z".parse().unwrap(),
                    minutes: 600,
                    drained_mwh: 450,
                    full_mwh: 59228,
                    deep_pct: 99,
                    blocker: None,
                },
            ]
        );
    }

    #[test]
    fn names_the_longest_blocker_when_deep_sleep_falls_short() {
        let r = report(&doc(&[sleep(
            "2026-10-05T01:00:00Z",
            100,
            82 * MIN,
            &[
                ("PDC Phase: Network", "PDC Phase", 90 * MIN),
                ("Intel(R) PEG12 - 9A07 (\\_SB.PC00.PEG3)", "Fx Device", 3 * MIN),
                ("WU", "Activator", 15 * MIN),
            ],
        )]));
        assert_eq!(r.sessions[0].deep_pct, 82);
        assert_eq!(r.sessions[0].blocker.as_deref(), Some("Windows Update"));

        let r = report(&doc(&[sleep(
            "2026-10-05T01:00:00Z",
            100,
            50 * MIN,
            &[("Intel(R) PEG12 - 9A07 (\\_SB.PC00.PEG3)", "Fx Device", 30 * MIN), ("BI", "Activator", MIN)],
        )]));
        assert_eq!(r.sessions[0].blocker.as_deref(), Some("Intel(R) PEG12 - 9A07"));
    }

    #[test]
    fn activator_names_are_spelled_out() {
        for (raw, want) in [
            ("WU", "Windows Update"),
            ("NCSI", "Network connectivity check"),
            ("BI", "Background tasks"),
            ("Audio Active", "Audio playing"),
            ("SomethingElse", "SomethingElse"),
        ] {
            let r = report(&doc(&[sleep("2026-10-05T01:00:00Z", 100, 10 * MIN, &[(raw, "Activator", MIN)])]));
            assert_eq!(r.sessions[0].blocker.as_deref(), Some(want), "{raw}");
        }
    }

    #[test]
    fn keeps_at_most_the_newest_sessions() {
        let times: Vec<String> = (1..=20).map(|d| format!("2026-09-{d:02}T01:00:00Z")).collect();
        let scenarios: Vec<S> = times.iter().map(|t| sleep(t, 60, 60 * MIN, &[])).collect();
        let r = report(&doc(&scenarios));
        assert_eq!(r.sessions.len(), MAX_SESSIONS);
        assert_eq!(r.sessions[0].start, "2026-09-20T01:00:00Z".parse::<DateTime<Utc>>().unwrap());
    }

    #[test]
    fn garbage_is_an_empty_report() {
        assert_eq!(report("<not xml"), SleepReport::default());
        assert_eq!(report(""), SleepReport::default());
    }
}
