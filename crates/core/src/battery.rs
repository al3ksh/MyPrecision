//! Battery math: WMI raw values -> UI snapshot.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PowerState {
    Charging,
    Discharging,
    /// On AC, neither charging nor discharging (charge limit reached).
    Bypass,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatteryRaw {
    pub power_online: bool,
    pub charging: bool,
    pub discharging: bool,
    pub charge_rate_mw: u32,
    pub discharge_rate_mw: u32,
    pub remaining_mwh: u32,
    pub voltage_mv: u32,
    pub full_mwh: u32,
    pub design_mwh: u32,
    pub cycles: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatterySnapshot {
    pub percent: Option<f32>,
    pub state: PowerState,
    /// Positive while charging, negative while discharging.
    pub power_w: f32,
    pub remaining_mwh: u32,
    pub full_mwh: Option<u32>,
    pub design_mwh: Option<u32>,
    pub wear_pct: Option<f32>,
    pub cycles: Option<u32>,
    pub voltage_v: f32,
}

fn round1(x: f32) -> f32 {
    (x * 10.0).round() / 10.0
}

fn nonzero(v: u32) -> Option<u32> {
    (v != 0).then_some(v)
}

pub fn battery_from_raw(raw: &BatteryRaw) -> BatterySnapshot {
    let (state, power_w) = if raw.charging {
        (PowerState::Charging, raw.charge_rate_mw as f32 / 1000.0)
    } else if raw.discharging || !raw.power_online {
        (PowerState::Discharging, -(raw.discharge_rate_mw as f32) / 1000.0)
    } else {
        (PowerState::Bypass, 0.0)
    };
    let full = nonzero(raw.full_mwh);
    let design = nonzero(raw.design_mwh);
    BatterySnapshot {
        percent: full.map(|f| round1((raw.remaining_mwh as f32 / f as f32 * 100.0).clamp(0.0, 100.0))),
        state,
        power_w,
        remaining_mwh: raw.remaining_mwh,
        full_mwh: full,
        design_mwh: design,
        wear_pct: full.zip(design).map(|(f, d)| round1((1.0 - f as f32 / d as f32) * 100.0)),
        cycles: nonzero(raw.cycles),
        voltage_v: raw.voltage_mv as f32 / 1000.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw() -> BatteryRaw {
        BatteryRaw {
            power_online: true,
            remaining_mwh: 40000,
            voltage_mv: 12600,
            full_mwh: 58709,
            design_mwh: 67914,
            ..Default::default()
        }
    }

    #[test]
    fn wear_from_real_battery() {
        let s = battery_from_raw(&raw());
        assert_eq!(s.wear_pct, Some(13.6));
        assert_eq!(s.full_mwh, Some(58709));
        assert_eq!(s.design_mwh, Some(67914));
        assert_eq!(s.voltage_v, 12.6);
    }

    #[test]
    fn bypass_when_online_idle() {
        let s = battery_from_raw(&raw());
        assert_eq!(s.state, PowerState::Bypass);
        assert_eq!(s.power_w, 0.0);
    }

    #[test]
    fn charging_power_positive() {
        let s = battery_from_raw(&BatteryRaw { charging: true, charge_rate_mw: 45000, ..raw() });
        assert_eq!(s.state, PowerState::Charging);
        assert_eq!(s.power_w, 45.0);
    }

    #[test]
    fn discharging_power_negative() {
        let s = battery_from_raw(&BatteryRaw { power_online: false, discharging: true, discharge_rate_mw: 12500, ..raw() });
        assert_eq!(s.state, PowerState::Discharging);
        assert_eq!(s.power_w, -12.5);
    }

    #[test]
    fn offline_without_flags_is_discharging() {
        let s = battery_from_raw(&BatteryRaw { power_online: false, ..raw() });
        assert_eq!(s.state, PowerState::Discharging);
    }

    #[test]
    fn percent_rounded() {
        assert_eq!(battery_from_raw(&raw()).percent, Some(68.1)); // 40000 / 58709
    }

    #[test]
    fn percent_clamped() {
        assert_eq!(battery_from_raw(&BatteryRaw { remaining_mwh: 59000, ..raw() }).percent, Some(100.0));
    }

    #[test]
    fn zero_capacities_give_none() {
        let s = battery_from_raw(&BatteryRaw { full_mwh: 0, design_mwh: 0, ..raw() });
        assert_eq!(s.percent, None);
        assert_eq!(s.wear_pct, None);
        assert_eq!(s.full_mwh, None);
        assert_eq!(s.design_mwh, None);
    }

    #[test]
    fn zero_cycles_hidden() {
        assert_eq!(battery_from_raw(&raw()).cycles, None);
        assert_eq!(battery_from_raw(&BatteryRaw { cycles: 12, ..raw() }).cycles, Some(12));
    }
}
