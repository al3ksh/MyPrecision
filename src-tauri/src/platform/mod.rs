//! Windows-specific readers: cctk process, WMI (battery + Dell Command | Monitor), CPU, RAM, system.

mod cctk_exec;
mod cpu_times;
mod mem;
mod system;
mod wmi_battery;
mod wmi_dcim;

pub use cctk_exec::ExeCctkRunner;
pub use cpu_times::cpu_times;
pub use mem::mem;
pub use system::{is_elevated, optimizer_running};

use wmi::WMIConnection;

/// `CREATE_NO_WINDOW`: child console processes must not flash a window.
pub(crate) const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// WMI connections are `!Send`: create and use this on the poller thread only.
pub struct WmiReaders {
    root_wmi: WMIConnection,
    dcim: Option<WMIConnection>,
}

impl WmiReaders {
    pub fn new() -> anyhow::Result<Self> {
        let root_wmi = WMIConnection::with_namespace_path(r"ROOT\WMI")?;
        // Dell Command | Monitor may be missing; sensors then degrade to "unavailable".
        let dcim = WMIConnection::with_namespace_path(r"ROOT\DCIM\SYSMAN").ok();
        Ok(Self { root_wmi, dcim })
    }

    pub fn dcm_available(&self) -> bool {
        self.dcim.is_some()
    }
}
