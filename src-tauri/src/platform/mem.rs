use myprecision_core::sensors::MemSnapshot;
use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};

pub fn mem() -> Option<MemSnapshot> {
    let mut status = MEMORYSTATUSEX { dwLength: size_of::<MEMORYSTATUSEX>() as u32, ..Default::default() };
    // SAFETY: `status` is a live MEMORYSTATUSEX with dwLength set, as the API requires.
    unsafe { GlobalMemoryStatusEx(&mut status) }.ok()?;
    const MB: u64 = 1024 * 1024;
    Some(MemSnapshot {
        used_mb: ((status.ullTotalPhys - status.ullAvailPhys) / MB) as u32,
        total_mb: (status.ullTotalPhys / MB) as u32,
    })
}
