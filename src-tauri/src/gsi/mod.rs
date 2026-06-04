//! CS2 Game State Integration (GSI).
//!
//! CS2 POSTs JSON game state to our local HTTP server (configured by a cfg
//! file we install into the game's cfg directory). This module owns the
//! server, the auth token, payload parsing, and the merged game state.

pub mod install;
pub mod payload;
pub mod server;

use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use payload::GameState;

use rand::distributions::Alphanumeric;
use rand::Rng;
use serde::Serialize;

/// Newtype for the GSI auth token so it can live in Tauri managed state
/// without colliding with other `String`s.
pub struct GsiToken(pub String);

/// State shared between the server thread and Tauri commands.
#[derive(Default)]
pub struct GsiShared {
    pub state: GameState,
    pub last_payload: Option<Instant>,
    pub port: u16,
    pub running: bool,
}

pub type SharedGsi = Arc<Mutex<GsiShared>>;

/// Snapshot for the `gsi_status` command / debug panel.
#[derive(Debug, Clone, Serialize)]
pub struct GsiStatus {
    pub running: bool,
    pub port: u16,
    /// Milliseconds since the last accepted payload; `None` = never.
    pub last_payload_age_ms: Option<u64>,
    /// Where the cfg file goes (None when CS2 install not found).
    pub cfg_path: Option<String>,
    pub cfg_installed: bool,
}

pub fn status(shared: &SharedGsi) -> GsiStatus {
    let g = shared.lock().unwrap();
    let cfg_path = install::cfg_target_path();
    GsiStatus {
        running: g.running,
        port: g.port,
        last_payload_age_ms: g.last_payload.map(|t| t.elapsed().as_millis() as u64),
        cfg_installed: cfg_path.as_deref().map(|p| p.exists()).unwrap_or(false),
        cfg_path: cfg_path.map(|p| p.display().to_string()),
    }
}

/// Load the per-install auth token from `<dir>/gsi_token.txt`, creating a
/// random 32-char alphanumeric one on first run. The same token is written
/// into the game's cfg file and checked on every incoming POST.
pub fn load_or_create_token(dir: &Path) -> Result<String, String> {
    let path = dir.join("gsi_token.txt");
    if let Ok(existing) = fs::read_to_string(&path) {
        let existing = existing.trim().to_string();
        if !existing.is_empty() {
            return Ok(existing);
        }
    }
    let token: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect();
    fs::create_dir_all(dir).map_err(|e| format!("create {}: {e}", dir.display()))?;
    fs::write(&path, &token).map_err(|e| format!("write {}: {e}", path.display()))?;
    Ok(token)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_token_then_reuses_it() {
        let dir = tempfile::tempdir().unwrap();
        let t1 = load_or_create_token(dir.path()).unwrap();
        let t2 = load_or_create_token(dir.path()).unwrap();
        assert_eq!(t1, t2);
        assert_eq!(t1.len(), 32);
        assert!(t1.chars().all(|c| c.is_ascii_alphanumeric()));
    }

    #[test]
    fn distinct_dirs_get_distinct_tokens() {
        let d1 = tempfile::tempdir().unwrap();
        let d2 = tempfile::tempdir().unwrap();
        assert_ne!(
            load_or_create_token(d1.path()).unwrap(),
            load_or_create_token(d2.path()).unwrap()
        );
    }
}
