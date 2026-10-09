//! Which apps hold the discrete GPU, from the `GPU Process Memory` performance counters,
//! and the per-app GPU preference Windows keeps for each executable.

use myprecision_core::gpu;
use windows::Win32::Foundation::{CloseHandle, ERROR_SUCCESS};
use windows::Win32::System::Performance::{
    PDH_FMT_COUNTERVALUE_ITEM_W, PDH_FMT_DOUBLE, PDH_HCOUNTER, PDH_HQUERY, PDH_MORE_DATA, PdhAddEnglishCounterW,
    PdhCloseQuery, PdhCollectQueryData, PdhGetFormattedCounterArrayW, PdhOpenQueryW,
};
use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_READ, REG_SZ, RRF_RT_REG_SZ, RegCloseKey, RegDeleteKeyValueW, RegEnumValueW,
    RegGetValueW, RegOpenKeyExW, RegSetKeyValueW,
};
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
};
use windows::core::{HSTRING, PWSTR, w};

const PREFS_KEY: windows::core::PCWSTR = w!(r"Software\Microsoft\DirectX\UserGpuPreferences");

/// `(pid, bytes)` of every process holding memory on the adapter `luid`, largest first.
pub fn holders(luid: u64) -> Vec<(u32, u64)> {
    let items = counter_items(w!(r"\GPU Process Memory(*)\Total Committed")).unwrap_or_default();
    gpu::holders(items.iter().map(|(n, v)| (n.as_str(), *v)), luid)
}

fn counter_items(path: windows::core::PCWSTR) -> Option<Vec<(String, f64)>> {
    let mut query = PDH_HQUERY::default();
    // SAFETY: the query is closed once below; every buffer is live and sized as PDH reports.
    unsafe {
        if PdhOpenQueryW(None, 0, &mut query) != ERROR_SUCCESS.0 {
            return None;
        }
        let items = (|| {
            let mut counter = PDH_HCOUNTER::default();
            if PdhAddEnglishCounterW(query, path, 0, &mut counter) != ERROR_SUCCESS.0
                || PdhCollectQueryData(query) != ERROR_SUCCESS.0
            {
                return None;
            }
            let (mut size, mut count) = (0u32, 0u32);
            if PdhGetFormattedCounterArrayW(counter, PDH_FMT_DOUBLE, &mut size, &mut count, None) != PDH_MORE_DATA {
                return Some(Vec::new());
            }
            // The buffer holds the item array followed by the instance names it points into.
            let item_size = size_of::<PDH_FMT_COUNTERVALUE_ITEM_W>();
            let mut buf = vec![PDH_FMT_COUNTERVALUE_ITEM_W::default(); (size as usize).div_ceil(item_size)];
            if PdhGetFormattedCounterArrayW(counter, PDH_FMT_DOUBLE, &mut size, &mut count, Some(buf.as_mut_ptr()))
                != ERROR_SUCCESS.0
            {
                return None;
            }
            Some(
                buf[..count as usize]
                    .iter()
                    .filter(|i| i.FmtValue.CStatus == 0)
                    .filter_map(|i| Some((i.szName.to_string().ok()?, i.FmtValue.Anonymous.doubleValue)))
                    .collect(),
            )
        })();
        let _ = PdhCloseQuery(query);
        items
    }
}

/// Full executable path of a running process.
pub fn process_path(pid: u32) -> Option<String> {
    // SAFETY: the handle is closed once; `buf` is live and its length is passed alongside.
    unsafe {
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        let read = QueryFullProcessImageNameW(process, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut len);
        let _ = CloseHandle(process);
        read.ok()?;
        Some(String::from_utf16_lossy(&buf[..len as usize]))
    }
}

/// The preference string stored for `exe`, if any.
pub fn preference(exe: &str) -> Option<String> {
    let name = HSTRING::from(exe);
    let mut buf = [0u16; 512];
    let mut len = size_of_val(&buf) as u32;
    // SAFETY: `buf` and `len` are live; RRF_RT_REG_SZ restricts the type to a string.
    let read = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            PREFS_KEY,
            &name,
            RRF_RT_REG_SZ,
            None,
            Some(buf.as_mut_ptr().cast()),
            Some(&mut len),
        )
    };
    (read == ERROR_SUCCESS).then(|| utf16_string(&buf))
}

/// Every executable with a stored preference string.
pub fn preferences() -> Vec<(String, String)> {
    let mut key = HKEY::default();
    let mut out = Vec::new();
    // SAFETY: the key is closed once; every buffer is live and its length is passed alongside.
    unsafe {
        if RegOpenKeyExW(HKEY_CURRENT_USER, PREFS_KEY, None, KEY_READ, &mut key) != ERROR_SUCCESS {
            return out;
        }
        for index in 0.. {
            let mut name = [0u16; 1024];
            let mut name_len = name.len() as u32;
            let mut data = [0u16; 512];
            let mut data_len = size_of_val(&data) as u32;
            let mut ty = 0u32;
            let status = RegEnumValueW(
                key,
                index,
                Some(PWSTR(name.as_mut_ptr())),
                &mut name_len,
                None,
                Some(&mut ty),
                Some(data.as_mut_ptr().cast()),
                Some(&mut data_len),
            );
            if status != ERROR_SUCCESS {
                break;
            }
            if ty == REG_SZ.0 {
                out.push((String::from_utf16_lossy(&name[..name_len as usize]), utf16_string(&data)));
            }
        }
        let _ = RegCloseKey(key);
    }
    out
}

/// Stores `data` for `exe`, or deletes the value when `None`.
pub fn set_preference(exe: &str, data: Option<&str>) -> windows::core::Result<()> {
    let name = HSTRING::from(exe);
    // SAFETY: `name` and `wide` outlive the calls; the byte count includes the terminating NUL.
    let status = unsafe {
        match data {
            Some(data) => {
                let wide: Vec<u16> = data.encode_utf16().chain([0]).collect();
                RegSetKeyValueW(
                    HKEY_CURRENT_USER,
                    PREFS_KEY,
                    &name,
                    REG_SZ.0,
                    Some(wide.as_ptr().cast()),
                    (wide.len() * 2) as u32,
                )
            }
            None => match RegDeleteKeyValueW(HKEY_CURRENT_USER, PREFS_KEY, &name) {
                windows::Win32::Foundation::ERROR_FILE_NOT_FOUND => ERROR_SUCCESS,
                s => s,
            },
        }
    };
    status.ok()
}

fn utf16_string(buf: &[u16]) -> String {
    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..len])
}

#[cfg(test)]
mod tests {
    /// Hardware probe: `cargo test -p myprecision gpu_apps -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn prints_holders() {
        let adapter = super::super::gpu_power::nvidia_adapter();
        println!("{:?}", adapter.as_ref().map(|a| (a.power, a.luid, &a.name)));
        if let Some(luid) = adapter.and_then(|a| a.luid) {
            for (pid, bytes) in super::holders(luid) {
                println!("{pid} {bytes} {:?}", super::process_path(pid));
            }
        }
        println!("{:#?}", super::preferences());
    }
}
