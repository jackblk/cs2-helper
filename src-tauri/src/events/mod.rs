//! Turns GSI state changes into duck/restore decisions. `core` is pure logic;
//! `runtime` owns the engine thread and audio side effects.

pub mod core;
pub mod runtime;

/// The only process whose audio is ever ducked.
pub const TARGET_PROCESS: &str = "cs2.exe";

pub struct Cs2Audio;

#[cfg(windows)]
impl runtime::AudioControl for Cs2Audio {
    fn set_volume(&self, volume: f32) -> Result<bool, String> {
        crate::audio::set_volume_for_process(TARGET_PROCESS, volume)
            .map(|changed| changed > 0)
            .map_err(|e| e.to_string())
    }
    fn get_volume(&self) -> Result<Option<f32>, String> {
        crate::audio::get_volume_for_process(TARGET_PROCESS).map_err(|e| e.to_string())
    }
}

/// Stub so the crate still type-checks off Windows.
#[cfg(not(windows))]
impl runtime::AudioControl for Cs2Audio {
    fn set_volume(&self, _volume: f32) -> Result<bool, String> {
        Ok(false)
    }
    fn get_volume(&self) -> Result<Option<f32>, String> {
        Ok(None)
    }
}
