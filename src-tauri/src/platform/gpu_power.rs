//! NVIDIA dGPU power state and identity from the PnP manager. Reading them never wakes
//! the card, unlike NVML, so this gates every NVML call.

use windows::Win32::Devices::DeviceAndDriverInstallation::{
    DIGCF_PRESENT, GUID_DEVCLASS_DISPLAY, SP_DEVINFO_DATA, SetupDiDestroyDeviceInfoList, SetupDiEnumDeviceInfo,
    SetupDiGetClassDevsW, SetupDiGetDevicePropertyW,
};
use windows::Win32::Devices::Properties::{
    DEVPKEY_Device_DeviceDesc, DEVPKEY_Device_HardwareIds, DEVPKEY_Device_PowerData, DEVPROPTYPE,
};
use windows::Win32::Foundation::DEVPROPKEY;
use windows::Win32::System::Power::{CM_POWER_DATA, PowerDeviceD0};
use windows::core::{GUID, PCWSTR};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DevicePower {
    D0,
    Sleeping,
    NotFound,
}

/// The adapter LUID DirectX assigns, which the GPU performance counters are keyed by.
const DEVPKEY_ADAPTER_LUID: DEVPROPKEY =
    DEVPROPKEY { fmtid: GUID::from_u128(0x60b193cb_5276_4d0f_96fc_f173abad3ec6), pid: 2 };

pub struct NvidiaAdapter {
    pub power: DevicePower,
    pub luid: Option<u64>,
    pub name: Option<String>,
}

pub fn nvidia_power_state() -> DevicePower {
    nvidia_adapter().map_or(DevicePower::NotFound, |a| a.power)
}

/// The first NVIDIA display adapter; `None` on machines with integrated graphics only.
pub fn nvidia_adapter() -> Option<NvidiaAdapter> {
    // SAFETY: the device info set is destroyed before returning; every buffer passed
    // to SetupDiGetDevicePropertyW is a live, correctly sized local.
    unsafe {
        let set = SetupDiGetClassDevsW(Some(&GUID_DEVCLASS_DISPLAY), PCWSTR::null(), None, DIGCF_PRESENT).ok()?;
        let mut result = None;
        let mut index = 0;
        loop {
            let mut info = SP_DEVINFO_DATA { cbSize: size_of::<SP_DEVINFO_DATA>() as u32, ..Default::default() };
            if SetupDiEnumDeviceInfo(set, index, &mut info).is_err() {
                break;
            }
            index += 1;

            let mut ty = DEVPROPTYPE::default();
            let mut ids = [0u8; 4096];
            if SetupDiGetDevicePropertyW(set, &info, &DEVPKEY_Device_HardwareIds, &mut ty, Some(&mut ids), None, 0)
                .is_err()
                || !utf16_contains(&ids, "VEN_10DE")
            {
                continue;
            }

            let mut data = CM_POWER_DATA::default();
            let buf = std::slice::from_raw_parts_mut((&raw mut data).cast::<u8>(), size_of::<CM_POWER_DATA>());
            let power =
                match SetupDiGetDevicePropertyW(set, &info, &DEVPKEY_Device_PowerData, &mut ty, Some(buf), None, 0) {
                    Ok(()) if data.PD_MostRecentPowerState == PowerDeviceD0 => DevicePower::D0,
                    Ok(()) => DevicePower::Sleeping,
                    Err(_) => break,
                };
            let mut luid = [0u8; 8];
            let luid = SetupDiGetDevicePropertyW(set, &info, &DEVPKEY_ADAPTER_LUID, &mut ty, Some(&mut luid), None, 0)
                .ok()
                .map(|()| u64::from_le_bytes(luid));
            let mut desc = [0u8; 512];
            let name =
                SetupDiGetDevicePropertyW(set, &info, &DEVPKEY_Device_DeviceDesc, &mut ty, Some(&mut desc), None, 0)
                    .ok()
                    .map(|()| utf16_string(&desc))
                    .filter(|n| !n.is_empty());
            result = Some(NvidiaAdapter { power, luid, name });
            break;
        }
        let _ = SetupDiDestroyDeviceInfoList(set);
        result
    }
}

fn utf16(buf: &[u8]) -> Vec<u16> {
    buf.as_chunks::<2>().0.iter().map(|&c| u16::from_le_bytes(c)).collect()
}

/// A NUL-terminated UTF-16LE string.
fn utf16_string(buf: &[u8]) -> String {
    let wide = utf16(buf);
    let len = wide.iter().position(|&c| c == 0).unwrap_or(wide.len());
    String::from_utf16_lossy(&wide[..len])
}

/// Case-insensitive search in a UTF-16LE REG_MULTI_SZ buffer.
fn utf16_contains(buf: &[u8], needle: &str) -> bool {
    String::from_utf16_lossy(&utf16(buf)).to_ascii_uppercase().contains(needle)
}
