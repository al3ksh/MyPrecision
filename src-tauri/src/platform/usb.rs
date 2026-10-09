//! Present USB devices from the PnP manager: name, last arrival, removability and power state.

use myprecision_core::usb::RawUsbDevice;
use windows::Win32::Devices::DeviceAndDriverInstallation::{
    CM_DEVCAP_REMOVABLE, DIGCF_ALLCLASSES, DIGCF_PRESENT, HDEVINFO, SP_DEVINFO_DATA, SetupDiDestroyDeviceInfoList,
    SetupDiEnumDeviceInfo, SetupDiGetClassDevsW, SetupDiGetDevicePropertyW,
};
use windows::Win32::Devices::Properties::{
    DEVPKEY_Device_BusReportedDeviceDesc, DEVPKEY_Device_Capabilities, DEVPKEY_Device_DeviceDesc,
    DEVPKEY_Device_FriendlyName, DEVPKEY_Device_InstanceId, DEVPKEY_Device_LastArrivalDate, DEVPKEY_Device_PowerData,
    DEVPROPTYPE,
};
use windows::Win32::Foundation::DEVPROPKEY;
use windows::Win32::System::Power::{CM_POWER_DATA, PowerDeviceD0};
use windows::core::w;

use super::gpu_power::utf16_string;

/// FILETIME of the Unix epoch, in 100 ns units.
const UNIX_EPOCH_FILETIME: u64 = 116_444_736_000_000_000;

/// Reads one property into `buf`; `false` when the device doesn't have it.
///
/// # Safety
/// `set` and `info` must be a live device info set and one of its elements.
unsafe fn property(set: HDEVINFO, info: &SP_DEVINFO_DATA, key: &DEVPROPKEY, buf: &mut [u8]) -> bool {
    let mut ty = DEVPROPTYPE::default();
    unsafe { SetupDiGetDevicePropertyW(set, info, key, &mut ty, Some(buf), None, 0) }.is_ok()
}

/// # Safety
/// As for [`property`].
unsafe fn string(set: HDEVINFO, info: &SP_DEVINFO_DATA, key: &DEVPROPKEY) -> Option<String> {
    let mut buf = [0u8; 1024];
    unsafe { property(set, info, key, &mut buf) }.then(|| utf16_string(&buf)).filter(|s| !s.trim().is_empty())
}

/// # Safety
/// As for [`property`].
unsafe fn device(set: HDEVINFO, info: &SP_DEVINFO_DATA) -> Option<RawUsbDevice> {
    unsafe {
        let id = string(set, info, &DEVPKEY_Device_InstanceId)?;
        let upper = id.to_ascii_uppercase();
        // Root hubs belong to the controller; `&MI_` entries are interfaces of a composite device.
        if upper.starts_with(r"USB\ROOT_HUB") || upper.contains("&MI_") {
            return None;
        }
        let name = [DEVPKEY_Device_BusReportedDeviceDesc, DEVPKEY_Device_FriendlyName, DEVPKEY_Device_DeviceDesc]
            .iter()
            .find_map(|key| string(set, info, key))?;

        let mut filetime = [0u8; 8];
        let arrived_ms = property(set, info, &DEVPKEY_Device_LastArrivalDate, &mut filetime)
            .then(|| u64::from_le_bytes(filetime))
            .filter(|ft| *ft > UNIX_EPOCH_FILETIME)
            .map(|ft| ((ft - UNIX_EPOCH_FILETIME) / 10_000) as i64);

        let mut caps = [0u8; 4];
        let removable = property(set, info, &DEVPKEY_Device_Capabilities, &mut caps)
            && u32::from_le_bytes(caps) & CM_DEVCAP_REMOVABLE.0 != 0;

        let mut data = CM_POWER_DATA::default();
        let buf = std::slice::from_raw_parts_mut((&raw mut data).cast::<u8>(), size_of::<CM_POWER_DATA>());
        let suspended =
            property(set, info, &DEVPKEY_Device_PowerData, buf) && data.PD_MostRecentPowerState != PowerDeviceD0;

        Some(RawUsbDevice { id, name: name.trim().to_string(), arrived_ms, removable, suspended })
    }
}

/// Every present device the USB bus enumerated.
pub fn devices() -> Vec<RawUsbDevice> {
    let mut out = Vec::new();
    // SAFETY: the device info set is destroyed before returning and only used while live.
    unsafe {
        let Ok(set) = SetupDiGetClassDevsW(None, w!("USB"), None, DIGCF_PRESENT | DIGCF_ALLCLASSES) else {
            return out;
        };
        let mut index = 0;
        loop {
            let mut info = SP_DEVINFO_DATA { cbSize: size_of::<SP_DEVINFO_DATA>() as u32, ..Default::default() };
            if SetupDiEnumDeviceInfo(set, index, &mut info).is_err() {
                break;
            }
            index += 1;
            out.extend(device(set, &info));
        }
        let _ = SetupDiDestroyDeviceInfoList(set);
    }
    out
}
