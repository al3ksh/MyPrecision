//! Discrete-GPU bookkeeping: which processes hold memory on the NVIDIA adapter, from the
//! `GPU Process Memory` performance counters, and the per-app preference Windows keeps in
//! `HKCU\Software\Microsoft\DirectX\UserGpuPreferences` as `key=value;` strings.

use std::collections::HashMap;

/// `GpuPreference` value meaning "power saving": the integrated GPU.
pub const POWER_SAVING: u32 = 1;

const KEY: &str = "GpuPreference";

/// `pid_<PID>_luid_0x<HIGH>_0x<LOW>_phys_<N>` → (pid, luid).
pub fn parse_instance(name: &str) -> Option<(u32, u64)> {
    let mut parts = name.split('_');
    if parts.next()? != "pid" {
        return None;
    }
    let pid = parts.next()?.parse().ok()?;
    if parts.next()? != "luid" {
        return None;
    }
    let hex = |s: &str| u32::from_str_radix(s.strip_prefix("0x")?, 16).ok();
    let high = hex(parts.next()?)?;
    let low = hex(parts.next()?)?;
    Some((pid, (u64::from(high) << 32) | u64::from(low)))
}

/// Bytes each process holds on `luid`, largest first; processes holding nothing are left out.
pub fn holders<'a>(samples: impl IntoIterator<Item = (&'a str, f64)>, luid: u64) -> Vec<(u32, u64)> {
    let mut by_pid: HashMap<u32, u64> = HashMap::new();
    for (name, bytes) in samples {
        if let Some((pid, l)) = parse_instance(name)
            && l == luid
            && bytes >= 1.0
        {
            *by_pid.entry(pid).or_default() += bytes as u64;
        }
    }
    let mut out: Vec<_> = by_pid.into_iter().collect();
    out.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    out
}

fn pairs(data: &str) -> impl Iterator<Item = (&str, &str)> {
    data.split(';').filter_map(|p| p.split_once('=')).map(|(k, v)| (k.trim(), v.trim()))
}

/// The `GpuPreference` in a preference string.
pub fn preference(data: &str) -> Option<u32> {
    pairs(data).find(|(k, _)| k.eq_ignore_ascii_case(KEY)).and_then(|(_, v)| v.parse().ok())
}

/// `data` with `GpuPreference` set to `pref`, or removed when `None`, other keys kept.
/// `None` when nothing is left, so the value can be deleted.
pub fn with_preference(data: Option<&str>, pref: Option<u32>) -> Option<String> {
    let mut out: String = pairs(data.unwrap_or_default())
        .filter(|(k, _)| !k.eq_ignore_ascii_case(KEY))
        .map(|(k, v)| format!("{k}={v};"))
        .collect();
    if let Some(p) = pref {
        out = format!("{KEY}={p};{out}");
    }
    (!out.is_empty()).then_some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_counter_instances() {
        assert_eq!(parse_instance("pid_31032_luid_0x00000000_0x0001501B_phys_0"), Some((31032, 0x1501b)));
        assert_eq!(parse_instance("pid_4_luid_0x00000001_0x00000002_phys_0"), Some((4, 0x1_0000_0002)));
        assert_eq!(parse_instance("_Total"), None);
        assert_eq!(parse_instance("pid_x_luid_0x0_0x1_phys_0"), None);
    }

    #[test]
    fn sums_memory_per_process_on_one_adapter() {
        let samples = [
            ("pid_10_luid_0x00000000_0x0001501B_phys_0", 7.0e6),
            ("pid_10_luid_0x00000000_0x0001501B_phys_1", 1.0e6),
            ("pid_20_luid_0x00000000_0x0001501B_phys_0", 240.0e6),
            ("pid_30_luid_0x00000000_0x0001501B_phys_0", 0.0),
            ("pid_40_luid_0x00000000_0x00014C36_phys_0", 900.0e6),
        ];
        assert_eq!(holders(samples, 0x1501b), vec![(20, 240_000_000), (10, 8_000_000)]);
    }

    #[test]
    fn reads_the_preference() {
        assert_eq!(preference("GpuPreference=2;"), Some(2));
        assert_eq!(preference("SwapEffectUpgradeEnable=1;GpuPreference=1;"), Some(1));
        assert_eq!(preference("SwapEffectUpgradeEnable=1;"), None);
        assert_eq!(preference(""), None);
    }

    #[test]
    fn sets_the_preference_keeping_other_keys() {
        assert_eq!(with_preference(None, Some(POWER_SAVING)), Some("GpuPreference=1;".into()));
        assert_eq!(
            with_preference(Some("SwapEffectUpgradeEnable=1;GpuPreference=2;"), Some(1)),
            Some("GpuPreference=1;SwapEffectUpgradeEnable=1;".into())
        );
    }

    #[test]
    fn removing_the_preference_keeps_other_keys_or_empties() {
        assert_eq!(with_preference(Some("GpuPreference=1;"), None), None);
        assert_eq!(with_preference(Some("GpuPreference=1;AutoHDREnable=1;"), None), Some("AutoHDREnable=1;".into()));
        assert_eq!(with_preference(None, None), None);
    }
}
