//! Autostart via a Startup-folder shortcut (.lnk), no registry.
#![cfg(windows)]

use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};

use windows::core::{Interface, PCWSTR};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, IPersistFile,
    CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED,
};
use windows::Win32::UI::Shell::{
    FOLDERID_Startup, SHGetKnownFolderPath, IShellLinkW, ShellLink, KF_FLAG_DEFAULT,
};

const SHORTCUT_NAME: &str = "CS2 Helper.lnk";

/// Null-terminated UTF-16. The returned Vec must outlive the `PCWSTR` borrowing
/// it.
fn wide(s: &OsStr) -> Vec<u16> {
    s.encode_wide().chain(std::iter::once(0)).collect()
}

fn startup_dir() -> Result<PathBuf, String> {
    unsafe {
        let pw = SHGetKnownFolderPath(&FOLDERID_Startup, KF_FLAG_DEFAULT, None)
            .map_err(|e| format!("SHGetKnownFolderPath: {e}"))?;
        let s = pw.to_string().map_err(|e| format!("startup path utf16: {e}"));
        CoTaskMemFree(Some(pw.0 as *const _));
        Ok(PathBuf::from(s?))
    }
}

fn shortcut_path() -> Result<PathBuf, String> {
    Ok(startup_dir()?.join(SHORTCUT_NAME))
}

/// Idempotent.
pub fn set_enabled(enabled: bool) -> Result<(), String> {
    let link = shortcut_path()?;
    if enabled {
        let target = std::env::current_exe().map_err(|e| format!("current_exe: {e}"))?;
        // COM on a dedicated short-lived thread so interface pointers never
        // escape it.
        std::thread::spawn(move || create_shortcut(&link, &target))
            .join()
            .map_err(|_| "shortcut thread panicked".to_string())?
    } else if link.exists() {
        std::fs::remove_file(&link).map_err(|e| format!("remove shortcut: {e}"))
    } else {
        Ok(())
    }
}

fn create_shortcut(link_path: &Path, target: &Path) -> Result<(), String> {
    unsafe {
        // Result ignored: this is a fresh thread, so COM is not yet initialized.
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let result = (|| -> windows::core::Result<()> {
            let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)?;
            let target_w = wide(target.as_os_str());
            link.SetPath(PCWSTR(target_w.as_ptr()))?;
            if let Some(dir) = target.parent() {
                let dir_w = wide(dir.as_os_str());
                link.SetWorkingDirectory(PCWSTR(dir_w.as_ptr()))?;
            }
            let persist: IPersistFile = link.cast()?;
            let link_w = wide(link_path.as_os_str());
            persist.Save(PCWSTR(link_w.as_ptr()), true)?;
            Ok(())
        })();
        CoUninitialize();
        result.map_err(|e| format!("create shortcut: {e}"))
    }
}
