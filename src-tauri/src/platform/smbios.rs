use myprecision_core::smbios::{self, DeviceInfo};
use windows::Win32::System::SystemInformation::{FIRMWARE_TABLE_PROVIDER, GetSystemFirmwareTable};

/// `'RSMB'`: the raw SMBIOS provider.
const RSMB: FIRMWARE_TABLE_PROVIDER = FIRMWARE_TABLE_PROVIDER(u32::from_be_bytes(*b"RSMB"));

pub fn device_info() -> DeviceInfo {
    // SAFETY: a null buffer with size 0 only queries the required size.
    let size = unsafe { GetSystemFirmwareTable(RSMB, 0, None) };
    if size == 0 {
        return DeviceInfo::default();
    }
    let mut buf = vec![0u8; size as usize];
    // SAFETY: `buf` is exactly the size the previous call reported.
    let written = unsafe { GetSystemFirmwareTable(RSMB, 0, Some(&mut buf)) };
    buf.truncate(written.min(size) as usize);
    smbios::parse(&buf)
}

#[cfg(test)]
mod tests {
    /// Hardware probe: `cargo test -p myprecision device_info -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn prints_this_machine() {
        println!("{:?}", super::device_info());
    }
}
