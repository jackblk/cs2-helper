//! Global hotkeys via tauri-plugin-global-shortcut, which uses Win32
//! `RegisterHotKey`: no keyboard hooks, nothing touches the game process.

use std::sync::Mutex;

use tauri::AppHandle;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

use crate::config::{HotkeyCfg, HotkeysConfig};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Pause,
    Deathmatch,
}

/// Add new hotkeys here.
fn bindings(cfg: &HotkeysConfig) -> [(Action, &HotkeyCfg); 2] {
    [
        (Action::Pause, &cfg.pause),
        (Action::Deathmatch, &cfg.deathmatch),
    ]
}

static ACTIVE: Mutex<Vec<(Shortcut, Action)>> = Mutex::new(Vec::new());

pub fn action_for(shortcut: &Shortcut) -> Option<Action> {
    ACTIVE
        .lock()
        .unwrap()
        .iter()
        .find(|(s, _)| s == shortcut)
        .map(|(_, a)| *a)
}

/// Errors when a key is invalid, duplicated, or taken by another app. After a
/// registration failure nothing stays registered.
pub fn apply(app: &AppHandle, cfg: &HotkeysConfig) -> Result<(), String> {
    let mut next: Vec<(Shortcut, Action, &str)> = Vec::new();
    for (action, h) in bindings(cfg) {
        if !h.enabled {
            continue;
        }
        let shortcut: Shortcut = h
            .key
            .parse()
            .map_err(|e| format!("invalid hotkey \"{}\": {e}", h.key))?;
        if next.iter().any(|(s, _, _)| *s == shortcut) {
            return Err(format!("hotkey \"{}\" is assigned more than once", h.key));
        }
        next.push((shortcut, action, &h.key));
    }

    suspend(app)?;
    let gs = app.global_shortcut();
    for (shortcut, _, key) in &next {
        if let Err(e) = gs.register(*shortcut) {
            let _ = gs.unregister_all();
            return Err(format!("could not register hotkey \"{key}\": {e}"));
        }
    }
    *ACTIVE.lock().unwrap() = next.into_iter().map(|(s, a, _)| (s, a)).collect();
    Ok(())
}

/// Returns the error for `next` even when reverting to `prev` succeeds.
pub fn apply_or_revert(
    app: &AppHandle,
    next: &HotkeysConfig,
    prev: &HotkeysConfig,
) -> Result<(), String> {
    apply(app, next).inspect_err(|_| {
        let _ = apply(app, prev);
    })
}

/// Used while the UI records a new key, so the press reaches the webview
/// instead of triggering its action.
pub fn suspend(app: &AppHandle) -> Result<(), String> {
    ACTIVE.lock().unwrap().clear();
    app.global_shortcut()
        .unregister_all()
        .map_err(|e| format!("unregister hotkeys: {e}"))
}
