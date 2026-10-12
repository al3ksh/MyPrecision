//! The Windows file picker, for choosing an app to set a GPU for.

use windows::Win32::Foundation::HWND;
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE, CoCreateInstance, CoInitializeEx,
    CoTaskMemFree, CoUninitialize,
};
use windows::Win32::UI::Shell::Common::COMDLG_FILTERSPEC;
use windows::Win32::UI::Shell::{FileOpenDialog, IFileOpenDialog, SIGDN_FILESYSPATH};
use windows::core::w;

/// The `.exe` the user picks, owned by the window `owner`; `None` when cancelled.
pub fn pick_exe(owner: Option<isize>) -> Option<String> {
    // The dialog needs a single-threaded apartment, which the async runtime's threads aren't.
    std::thread::spawn(move || {
        // SAFETY: COM is initialised and torn down on this thread, which owns every interface used.
        unsafe {
            CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE).ok().ok()?;
            let path = show(owner);
            CoUninitialize();
            path
        }
    })
    .join()
    .ok()
    .flatten()
}

/// SAFETY: call on a thread with COM initialised as single-threaded.
unsafe fn show(owner: Option<isize>) -> Option<String> {
    unsafe {
        let dialog: IFileOpenDialog = CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER).ok()?;
        let types = [COMDLG_FILTERSPEC { pszName: w!("Apps"), pszSpec: w!("*.exe") }];
        dialog.SetFileTypes(&types).ok()?;
        dialog.SetTitle(w!("Choose an app")).ok()?;
        // Cancelling is an error too.
        dialog.Show(owner.map(|h| HWND(h as *mut _))).ok()?;
        let name = dialog.GetResult().ok()?.GetDisplayName(SIGDN_FILESYSPATH).ok()?;
        let path = name.to_string().ok();
        CoTaskMemFree(Some(name.0 as *const _));
        path
    }
}
