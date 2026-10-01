//! Per-process session volume via WASAPI, never the system master volume.
//!
//! All COM work runs on a freshly spawned MTA thread, so it never depends on
//! the apartment state of whichever thread Tauri calls from. Calls are
//! infrequent (game events), so the per-call spawn is negligible.

#![cfg(windows)]

use serde::Serialize;
use windows::core::Interface;
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::Media::Audio::{
    eRender, IAudioSessionControl2, IAudioSessionManager2, IMMDeviceCollection,
    IMMDeviceEnumerator, ISimpleAudioVolume, MMDeviceEnumerator, DEVICE_STATE_ACTIVE,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_MULTITHREADED,
};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
    PROCESS_QUERY_LIMITED_INFORMATION,
};

/// PID of the system-sounds session, which has no owning process.
const SYSTEM_SOUNDS_PID: u32 = 0;

#[derive(Debug, Clone, Serialize)]
pub struct AudioSessionInfo {
    pub pid: u32,
    pub process_name: String,
    pub volume: f32,
    pub muted: bool,
}

/// COM interface pointers must never escape the thread: `f` may only return
/// plain `Send` data.
fn with_com<T, F>(f: F) -> windows::core::Result<T>
where
    F: FnOnce() -> windows::core::Result<T> + Send + 'static,
    T: Send + 'static,
{
    std::thread::spawn(move || unsafe {
        // S_FALSE (already initialized) still counts as success.
        let init_ok = CoInitializeEx(None, COINIT_MULTITHREADED).is_ok();
        let result = f();
        if init_ok {
            CoUninitialize();
        }
        result
    })
    .join()
    .expect("audio COM thread panicked")
}

/// Executable base name, e.g. `cs2.exe`.
unsafe fn process_name(pid: u32) -> Option<String> {
    if pid == SYSTEM_SOUNDS_PID {
        return None;
    }
    let handle: HANDLE = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
    let mut buf = [0u16; 260];
    let mut len = buf.len() as u32;
    let result = QueryFullProcessImageNameW(
        handle,
        PROCESS_NAME_WIN32,
        windows::core::PWSTR(buf.as_mut_ptr()),
        &mut len,
    );
    let _ = CloseHandle(handle);
    result.ok()?;
    let full = String::from_utf16_lossy(&buf[..len as usize]);
    Some(
        full.rsplit(['\\', '/'])
            .next()
            .unwrap_or(&full)
            .to_string(),
    )
}

/// Visits every session on every active render endpoint. `visit` returns
/// `true` to stop early.
unsafe fn for_each_session<F>(mut visit: F) -> windows::core::Result<()>
where
    F: FnMut(u32, &IAudioSessionControl2, &ISimpleAudioVolume) -> bool,
{
    let enumerator: IMMDeviceEnumerator =
        CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
    let devices: IMMDeviceCollection =
        enumerator.EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE)?;

    for i in 0..devices.GetCount()? {
        let device = devices.Item(i)?;
        let manager: IAudioSessionManager2 = match device.Activate(CLSCTX_ALL, None) {
            Ok(m) => m,
            Err(_) => continue,
        };
        let sessions = manager.GetSessionEnumerator()?;
        for s in 0..sessions.GetCount()? {
            let control = match sessions.GetSession(s) {
                Ok(c) => c,
                Err(_) => continue,
            };
            let control2: IAudioSessionControl2 = match control.cast() {
                Ok(c) => c,
                Err(_) => continue,
            };
            let simple: ISimpleAudioVolume = match control.cast() {
                Ok(v) => v,
                Err(_) => continue,
            };
            let pid = control2.GetProcessId().unwrap_or(0);
            if visit(pid, &control2, &simple) {
                return Ok(());
            }
        }
    }
    Ok(())
}

pub fn list_sessions() -> windows::core::Result<Vec<AudioSessionInfo>> {
    with_com(|| unsafe {
        let mut out = Vec::new();
        for_each_session(|pid, _control2, simple| {
            let name = process_name(pid).unwrap_or_else(|| "<system>".to_string());
            let volume = simple.GetMasterVolume().unwrap_or(-1.0);
            let muted = simple.GetMute().map(|b| b.as_bool()).unwrap_or(false);
            out.push(AudioSessionInfo {
                pid,
                process_name: name,
                volume,
                muted,
            });
            false
        })?;
        Ok(out)
    })
}

/// Sets every session whose process name matches (case-insensitive). Returns
/// the number of sessions changed.
pub fn set_volume_for_process(process: &str, volume: f32) -> windows::core::Result<usize> {
    let target = process.to_owned();
    let level = volume.clamp(0.0, 1.0);
    with_com(move || unsafe {
        let mut changed = 0usize;
        for_each_session(|pid, _control2, simple| {
            if let Some(name) = process_name(pid) {
                if name.eq_ignore_ascii_case(&target) {
                    if simple.SetMasterVolume(level, std::ptr::null()).is_ok() {
                        changed += 1;
                    }
                }
            }
            false
        })?;
        Ok(changed)
    })
}

/// First matching session (case-insensitive); `None` if there is none.
pub fn get_volume_for_process(process: &str) -> windows::core::Result<Option<f32>> {
    let target = process.to_owned();
    with_com(move || unsafe {
        let mut found = None;
        for_each_session(|pid, _control2, simple| {
            if let Some(name) = process_name(pid) {
                if name.eq_ignore_ascii_case(&target) {
                    found = simple.GetMasterVolume().ok();
                    return true;
                }
            }
            false
        })?;
        Ok(found)
    })
}
