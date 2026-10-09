use myprecision_core::sensors::BatteryRaw;
use serde::Deserialize;

use super::WmiReaders;

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct BatteryStatus {
    power_online: bool,
    charging: bool,
    discharging: bool,
    charge_rate: i64,
    discharge_rate: i64,
    remaining_capacity: i64,
    voltage: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct BatteryFullChargedCapacity {
    full_charged_capacity: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct BatteryStaticData {
    designed_capacity: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct BatteryCycleCount {
    cycle_count: i64,
}

fn to_u32(v: i64) -> u32 {
    v.clamp(0, u32::MAX as i64) as u32
}

impl WmiReaders {
    /// First battery's raw values; `None` when no battery is reported. Optional
    /// classes (capacity, design, cycles) fall back to 0, which the core maps to "unknown".
    pub fn battery(&self) -> Option<BatteryRaw> {
        let status = self.first::<BatteryStatus>("SELECT * FROM BatteryStatus")?;
        let full = self.first::<BatteryFullChargedCapacity>("SELECT FullChargedCapacity FROM BatteryFullChargedCapacity");
        let design = self.first::<BatteryStaticData>("SELECT DesignedCapacity FROM BatteryStaticData");
        let cycles = self.first::<BatteryCycleCount>("SELECT CycleCount FROM BatteryCycleCount");
        Some(BatteryRaw {
            power_online: status.power_online,
            charging: status.charging,
            discharging: status.discharging,
            charge_rate_mw: to_u32(status.charge_rate),
            discharge_rate_mw: to_u32(status.discharge_rate),
            remaining_mwh: to_u32(status.remaining_capacity),
            voltage_mv: to_u32(status.voltage),
            full_mwh: full.map_or(0, |f| to_u32(f.full_charged_capacity)),
            design_mwh: design.map_or(0, |d| to_u32(d.designed_capacity)),
            cycles: cycles.map_or(0, |c| to_u32(c.cycle_count)),
        })
    }

    fn first<T: serde::de::DeserializeOwned>(&self, query: &str) -> Option<T> {
        self.root_wmi.raw_query::<T>(query).ok()?.into_iter().next()
    }
}
