//! NVMe drive health: the SMART / Health Information log (log page 0x02), the storage
//! device descriptor, and a wear forecast from two readings taken days apart.

use std::collections::BTreeMap;
use std::path::Path;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

/// Fields of the 512-byte NVMe SMART / Health Information log page.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SmartLog {
    /// Bit field; any set bit means the controller reports a problem.
    pub critical_warning: u8,
    pub temp_c: Option<i32>,
    pub available_spare_pct: u8,
    pub spare_threshold_pct: u8,
    /// The vendor's estimate of rated endurance consumed; may exceed 100.
    pub percent_used: u8,
    pub bytes_read: u64,
    pub bytes_written: u64,
    pub power_cycles: u64,
    pub power_on_hours: u64,
    pub unsafe_shutdowns: u64,
    pub media_errors: u64,
}

/// A data unit is 1000 sectors of 512 bytes.
const DATA_UNIT: u128 = 512_000;

pub fn parse_smart_log(log: &[u8]) -> Option<SmartLog> {
    if log.len() < 512 {
        return None;
    }
    let u128_at = |at: usize| u128::from_le_bytes(log[at..at + 16].try_into().unwrap());
    let count = |at: usize| u64::try_from(u128_at(at)).unwrap_or(u64::MAX);
    let bytes = |at: usize| u64::try_from(u128_at(at).saturating_mul(DATA_UNIT)).unwrap_or(u64::MAX);
    let kelvin = u16::from_le_bytes([log[1], log[2]]);
    Some(SmartLog {
        critical_warning: log[0],
        temp_c: (kelvin != 0).then(|| i32::from(kelvin) - 273),
        available_spare_pct: log[3],
        spare_threshold_pct: log[4],
        percent_used: log[5],
        bytes_read: bytes(32),
        bytes_written: bytes(48),
        power_cycles: count(112),
        power_on_hours: count(128),
        unsafe_shutdowns: count(144),
        media_errors: count(160),
    })
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DriveIdentity {
    pub model: Option<String>,
    pub serial: Option<String>,
    pub nvme: bool,
}

/// `STORAGE_BUS_TYPE::BusTypeNvme`.
const BUS_TYPE_NVME: u32 = 17;

/// Parses a `STORAGE_DEVICE_DESCRIPTOR` (the `StorageDeviceProperty` query result).
pub fn parse_device_descriptor(raw: &[u8]) -> DriveIdentity {
    let u32_at = |at: usize| raw.get(at..at + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()));
    let string_at = |field: usize| {
        let offset = u32_at(field)? as usize;
        if offset == 0 || offset >= raw.len() {
            return None;
        }
        let bytes = &raw[offset..];
        let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
        let s = String::from_utf8_lossy(&bytes[..end]).trim().to_string();
        (!s.is_empty()).then_some(s)
    };
    let vendor = string_at(12);
    let product = string_at(16);
    let model = match (vendor, product) {
        (Some(v), Some(p)) if !p.starts_with(&v) => Some(format!("{v} {p}")),
        (_, Some(p)) => Some(p),
        (v, None) => v,
    };
    DriveIdentity { model, serial: string_at(24), nvme: u32_at(28) == Some(BUS_TYPE_NVME) }
}

/// One reading kept to measure the write rate over calendar time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WearReading {
    pub date: NaiveDate,
    pub bytes_written: u64,
    pub percent_used: u8,
}

/// Fewer days than this make the write rate too noisy to extrapolate.
pub const FORECAST_MIN_DAYS: i64 = 7;

/// Years until the drive reaches its rated endurance at the write rate since `first`.
/// Rated endurance is inferred from how much was written per percent used, so it needs
/// at least 1 % used; `None` when either reading cannot support an estimate.
pub fn years_left(first: &WearReading, now: &WearReading) -> Option<f64> {
    let days = (now.date - first.date).num_days();
    if days < FORECAST_MIN_DAYS || now.percent_used == 0 || now.bytes_written <= first.bytes_written {
        return None;
    }
    let per_day = (now.bytes_written - first.bytes_written) as f64 / days as f64;
    let endurance = now.bytes_written as f64 * 100.0 / f64::from(now.percent_used);
    let left = (endurance - now.bytes_written as f64).max(0.0);
    Some(left / per_day / 365.0)
}

/// Returns the first reading kept for the drive `serial`, keeping `now` if there is none.
/// The file maps serial numbers to readings; a broken file starts over.
pub fn first_reading(path: &Path, serial: &str, now: WearReading) -> WearReading {
    let mut store: BTreeMap<String, WearReading> =
        std::fs::read_to_string(path).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
    if let Some(first) = store.get(serial) {
        return *first;
    }
    store.insert(serial.to_string(), now);
    if let Ok(json) = serde_json::to_string_pretty(&store) {
        let _ = crate::config::write_atomic(path, json.as_bytes());
    }
    now
}

#[cfg(test)]
mod tests {
    use super::*;

    fn log() -> Vec<u8> {
        let mut v = vec![0u8; 512];
        v[0] = 0;
        v[1..3].copy_from_slice(&317u16.to_le_bytes()); // 44 °C
        v[3] = 100;
        v[4] = 10;
        v[5] = 3;
        v[32..48].copy_from_slice(&40_000_000u128.to_le_bytes());
        v[48..64].copy_from_slice(&30_000_000u128.to_le_bytes());
        v[112..128].copy_from_slice(&1234u128.to_le_bytes());
        v[128..144].copy_from_slice(&5678u128.to_le_bytes());
        v[144..160].copy_from_slice(&42u128.to_le_bytes());
        v
    }

    #[test]
    fn reads_the_health_log() {
        let s = parse_smart_log(&log()).unwrap();
        assert_eq!(s.temp_c, Some(44));
        assert_eq!(s.available_spare_pct, 100);
        assert_eq!(s.spare_threshold_pct, 10);
        assert_eq!(s.percent_used, 3);
        assert_eq!(s.bytes_read, 20_480_000_000_000);
        assert_eq!(s.bytes_written, 15_360_000_000_000);
        assert_eq!(s.power_cycles, 1234);
        assert_eq!(s.power_on_hours, 5678);
        assert_eq!(s.unsafe_shutdowns, 42);
        assert_eq!(s.media_errors, 0);
        assert_eq!(s.critical_warning, 0);
    }

    #[test]
    fn short_log_and_missing_temperature() {
        assert_eq!(parse_smart_log(&[0; 100]), None);
        assert_eq!(parse_smart_log(&[0; 512]).unwrap().temp_c, None);
    }

    #[test]
    fn huge_counters_saturate() {
        let mut v = log();
        v[48..64].copy_from_slice(&u128::MAX.to_le_bytes());
        assert_eq!(parse_smart_log(&v).unwrap().bytes_written, u64::MAX);
    }

    fn descriptor(vendor: &str, product: &str, serial: &str, bus: u32) -> Vec<u8> {
        let mut v = vec![0u8; 36];
        v[28..32].copy_from_slice(&bus.to_le_bytes());
        for (field, s) in [(12, vendor), (16, product), (24, serial)] {
            if s.is_empty() {
                continue;
            }
            let offset = v.len() as u32;
            v[field..field + 4].copy_from_slice(&offset.to_le_bytes());
            v.extend_from_slice(s.as_bytes());
            v.push(0);
        }
        v
    }

    #[test]
    fn reads_model_serial_and_bus() {
        let id = parse_device_descriptor(&descriptor("", "Micron 2300 NVMe 512GB ", " 2034E1234567 ", 17));
        assert_eq!(
            id,
            DriveIdentity {
                model: Some("Micron 2300 NVMe 512GB".into()),
                serial: Some("2034E1234567".into()),
                nvme: true
            }
        );
        let sata = parse_device_descriptor(&descriptor("ATA", "Samsung SSD 870", "S1", 11));
        assert_eq!(sata.model.as_deref(), Some("ATA Samsung SSD 870"));
        assert!(!sata.nvme);
        assert_eq!(parse_device_descriptor(&[1, 2, 3]), DriveIdentity::default());
    }

    fn reading(date: &str, tb: f64, pct: u8) -> WearReading {
        WearReading { date: date.parse().unwrap(), bytes_written: (tb * 1e12) as u64, percent_used: pct }
    }

    #[test]
    fn forecast_from_write_rate_and_wear() {
        // 10 TB used 5 % → 200 TB rated; 190 TB left at 0.1 TB/day ≈ 5.2 years.
        let years = years_left(&reading("2026-01-01", 9.0, 5), &reading("2026-01-11", 10.0, 5)).unwrap();
        assert!((years - 1900.0 / 365.0).abs() < 0.01, "{years}");
    }

    #[test]
    fn no_forecast_without_enough_data() {
        let first = reading("2026-01-01", 9.0, 5);
        assert_eq!(years_left(&first, &reading("2026-01-05", 10.0, 5)), None, "too few days");
        assert_eq!(years_left(&first, &reading("2026-02-01", 10.0, 0)), None, "no wear reported");
        assert_eq!(years_left(&first, &reading("2026-02-01", 9.0, 5)), None, "nothing written");
    }

    #[test]
    fn worn_out_drive_has_no_years_left() {
        let years = years_left(&reading("2026-01-01", 9.0, 120), &reading("2026-02-01", 10.0, 120)).unwrap();
        assert_eq!(years, 0.0);
    }

    #[test]
    fn first_reading_is_kept_per_drive() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ssd.json");
        let a = reading("2026-01-01", 9.0, 5);
        assert_eq!(first_reading(&path, "A", a), a);
        assert_eq!(first_reading(&path, "A", reading("2026-02-01", 10.0, 6)), a);
        let b = reading("2026-02-01", 1.0, 0);
        assert_eq!(first_reading(&path, "B", b), b);
        assert_eq!(first_reading(&path, "A", b), a);
    }

    #[test]
    fn broken_store_starts_over() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ssd.json");
        std::fs::write(&path, "not json").unwrap();
        let now = reading("2026-02-01", 1.0, 0);
        assert_eq!(first_reading(&path, "A", now), now);
        assert_eq!(first_reading(&path, "A", reading("2026-03-01", 2.0, 1)), now);
    }
}
