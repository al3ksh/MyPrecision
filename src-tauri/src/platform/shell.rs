use windows::Win32::System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_DWORD, RegGetValueW};
use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSMICON};
use windows::core::w;

/// Pixel size of a notification-area icon at the system DPI (16 at 100 %, 32 at 200 %).
pub fn tray_icon_size() -> u32 {
    // SAFETY: GetSystemMetrics has no preconditions.
    let px = unsafe { GetSystemMetrics(SM_CXSMICON) };
    u32::try_from(px).ok().filter(|&p| (12..=64).contains(&p)).unwrap_or(16)
}

/// Whether the taskbar uses the light theme; missing value means dark (the Windows 11 default).
pub fn light_taskbar() -> bool {
    let mut value = 0u32;
    let mut len = size_of::<u32>() as u32;
    // SAFETY: `value` and `len` are live and sized for the REG_DWORD that RRF_RT_REG_DWORD restricts to.
    let read = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            w!(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize"),
            w!("SystemUsesLightTheme"),
            RRF_RT_REG_DWORD,
            None,
            Some((&raw mut value).cast()),
            Some(&raw mut len),
        )
    };
    read.is_ok() && value != 0
}
