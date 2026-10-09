//! NVIDIA dGPU power state from the PnP manager. Reading it never wakes the card,
//! unlike NVML, so it gates every NVML call.

use windows::Win32::Devices::DeviceAndDriverInstallation::{
    DIGCF_PRESENT, GUID_DEVCLASS_DISPLAY, SP_DEVINFO_DATA, SetupDiDestroyDeviceInfoList, SetupDiEnumDeviceInfo,
    SetupDiGetClassDevsW, SetupDiGetDevicePropertyW,
};
use windows::Win32::Devices::Properties::{DEVPKEY_Device_HardwareIds, DEVPKEY_Device_PowerData, DEVPROPTYPE};
use windows::Win32::System::Power::{CM_POWER_DATA, PowerDeviceD0};
use windows::core::PCWSTR;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DevicePower {
    D0,
    Sleeping,
    NotFound,
}

pub fn nvidia_power_state() -> DevicePower {
    // SAFETY: the device info set is destroyed before returning; every buffer passed
    // to SetupDiGetDevicePropertyW is a live, correctly sized local.
    unsafe {
        let Ok(set) = SetupDiGetClassDevsW(Some(&GUID_DEVCLASS_DISPLAY), PCWSTR::null(), None, DIGCF_PRESENT) else {
            return DevicePower::NotFound;
        };
        let mut result = DevicePower::NotFound;
        let mut index = 0;
        loop {
            let mut info = SP_DEVINFO_DATA { cbSize: size_of::<SP_DEVINFO_DATA>() as u32, ..Default::default() };
            if SetupDiEnumDeviceInfo(set, index, &mut info).is_err() {
                break;
            }
            index += 1;

            let mut ty = DEVPROPTYPE::default();
            let mut ids = [0u8; 4096];
            if SetupDiGetDevicePropertyW(set, &info, &DEVPKEY_Device_HardwareIds, &mut ty, Some(&mut ids), None, 0).is_err()
                || !utf16_contains(&ids, "VEN_10DE")
            {
                continue;
            }

            let mut data = CM_POWER_DATA::default();
            let buf = std::slice::from_raw_parts_mut((&raw mut data).cast::<u8>(), size_of::<CM_POWER_DATA>());
            result = match SetupDiGetDevicePropertyW(set, &info, &DEVPKEY_Device_PowerData, &mut ty, Some(buf), None, 0) {
                Ok(()) if data.PD_MostRecentPowerState == PowerDeviceD0 => DevicePower::D0,
                Ok(()) => DevicePower::Sleeping,
                Err(_) => DevicePower::NotFound,
            };
            break;
        }
        let _ = SetupDiDestroyDeviceInfoList(set);
        result
    }
}

/// Case-insensitive search in a UTF-16LE REG_MULTI_SZ buffer.
fn utf16_contains(buf: &[u8], needle: &str) -> bool {
    let wide: Vec<u16> = buf.as_chunks::<2>().0.iter().map(|&c| u16::from_le_bytes(c)).collect();
    String::from_utf16_lossy(&wide).to_ascii_uppercase().contains(needle)
}
