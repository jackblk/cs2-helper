//! CS2 Game State Integration: CS2 POSTs JSON game state to our local HTTP
//! server, configured by a cfg file we install into the game's cfg directory.

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

/// Newtype so the token can be Tauri managed state without colliding with
/// other `String`s.
pub struct GsiToken(pub String);

#[derive(Default)]
pub struct GsiShared {
    pub state: GameState,
    pub last_payload: Option<Instant>,
    pub port: u16,
    pub running: bool,
}

pub type SharedGsi = Arc<Mutex<GsiShared>>;

#[derive(Debug, Clone, Serialize)]
pub struct GsiStatus {
    pub running: bool,
    pub port: u16,
    pub last_payload_age_ms: Option<u64>,
    pub fresh: bool,
    /// `None` when the CS2 install is not found.
    pub cfg_path: Option<String>,
    pub cfg_installed: bool,
}

/// Payload age below which CS2 counts as running. Above the cfg's 10 s
/// heartbeat, so an idle menu never reads as "waiting".
const FRESH_MS: u64 = 15_000;

fn payload_age_ms(g: &GsiShared) -> Option<u64> {
    g.last_payload.map(|t| t.elapsed().as_millis() as u64)
}

pub fn is_fresh(shared: &SharedGsi) -> bool {
    payload_age_ms(&shared.lock().unwrap()).is_some_and(|ms| ms < FRESH_MS)
}

pub fn status(shared: &SharedGsi) -> GsiStatus {
    // Before locking: it reads the registry and the disk, and the lock would
    // hold up the GSI server.
    let cfg_path = install::cfg_target_path();
    let g = shared.lock().unwrap();
    let age = payload_age_ms(&g);
    GsiStatus {
        running: g.running,
        port: g.port,
        last_payload_age_ms: age,
        fresh: age.is_some_and(|ms| ms < FRESH_MS),
        cfg_installed: cfg_path.as_deref().is_some_and(Path::exists),
        cfg_path: cfg_path.map(|p| p.display().to_string()),
    }
}

const TOKEN_FILE_NAME: &str = "gsi_token.txt";
const TOKEN_LEN: usize = 32;

/// Per-install random token, created on first run. It is written into the
/// game's cfg file and checked on every incoming POST.
pub fn load_or_create_token(dir: &Path) -> Result<String, String> {
    let path = dir.join(TOKEN_FILE_NAME);
    if let Ok(existing) = fs::read_to_string(&path) {
        let existing = existing.trim().to_string();
        if !existing.is_empty() {
            return Ok(existing);
        }
    }
    let token: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(TOKEN_LEN)
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
        assert_eq!(t1.len(), TOKEN_LEN);
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
