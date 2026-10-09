use myprecision_core::sensors::CpuTimes;
use windows::Win32::Foundation::FILETIME;
use windows::Win32::System::Threading::GetSystemTimes;

fn ticks(ft: FILETIME) -> u64 {
    (u64::from(ft.dwHighDateTime) << 32) | u64::from(ft.dwLowDateTime)
}

/// System-wide idle/kernel/user times in 100 ns ticks (kernel includes idle).
pub fn cpu_times() -> CpuTimes {
    let (mut idle, mut kernel, mut user) = (FILETIME::default(), FILETIME::default(), FILETIME::default());
    // SAFETY: all three pointers reference live, writable FILETIMEs.
    if unsafe { GetSystemTimes(Some(&mut idle), Some(&mut kernel), Some(&mut user)) }.is_err() {
        return CpuTimes::default();
    }
    CpuTimes { idle: ticks(idle), kernel: ticks(kernel), user: ticks(user) }
}
