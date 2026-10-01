//! Global hotkeys (the `[hotkeys.*]` tables; pause defaults to F10).
//!
//! Uses tauri-plugin-global-shortcut, which calls Win32 `RegisterHotKey`: no
//! keyboard hooks, nothing touches the game process. This module keeps the
//! registered shortcuts in sync with the config and maps a press back to its
//! action; `lib.rs` performs the action.

use std::sync::Mutex;

use tauri::AppHandle;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

use crate::config::{HotkeyCfg, HotkeysConfig};

/// What a hotkey does when pressed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Toggle pause (same as the tray Pause/Resume).
    Pause,
}

/// Every configurable hotkey with its action. Add new hotkeys here.
fn bindings(cfg: &HotkeysConfig) -> [(Action, &HotkeyCfg); 1] {
    [(Action::Pause, &cfg.pause)]
}

/// The currently registered shortcuts, for mapping a press to its action.
static ACTIVE: Mutex<Vec<(Shortcut, Action)>> = Mutex::new(Vec::new());

/// The action bound to `shortcut`, if it is one of ours.
pub fn action_for(shortcut: &Shortcut) -> Option<Action> {
    ACTIVE
        .lock()
        .unwrap()
        .iter()
        .find(|(s, _)| s == shortcut)
        .map(|(_, a)| *a)
}

/// Make the registered shortcuts match `cfg`: validate every enabled key
/// (parseable, no duplicates), unregister everything, register the new set.
/// Errors when a key is invalid, duplicated, or already taken by another app
/// (on a registration failure nothing stays registered).
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

/// Apply `next`; if that fails, re-apply `prev` so the user keeps working
/// hotkeys. Returns the error for `next`.
pub fn apply_or_revert(
    app: &AppHandle,
    next: &HotkeysConfig,
    prev: &HotkeysConfig,
) -> Result<(), String> {
    apply(app, next).inspect_err(|_| {
        let _ = apply(app, prev);
    })
}

/// Release every hotkey (e.g. while the UI records a new one, so the press
/// reaches the webview instead of triggering its action).
pub fn suspend(app: &AppHandle) -> Result<(), String> {
    ACTIVE.lock().unwrap().clear();
    app.global_shortcut()
        .unregister_all()
        .map_err(|e| format!("unregister hotkeys: {e}"))
}
