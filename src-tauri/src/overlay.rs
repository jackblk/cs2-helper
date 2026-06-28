//! The in-game overlay window (label `overlay`): a transparent, click-through,
//! always-on-top window that renders the bomb defuse timer. Created only while
//! `OverlayConfig::enabled`; destroyed when disabled. Position/size derive from
//! the saved geometry (logical pixels), defaulting to bottom-center.

use tauri::{
    Emitter, LogicalPosition, LogicalSize, Manager, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
};

use crate::config::OverlayConfig;

pub const OVERLAY_LABEL: &str = "overlay";
/// Base logical size at scale 1.0; the badge fills it.
const BASE_W: f64 = 240.0;
const BASE_H: f64 = 96.0;

fn logical_size(scale: f32) -> (f64, f64) {
    let s = scale as f64;
    (BASE_W * s, BASE_H * s)
}

/// Margin (logical px) from the screen edge for the default position.
const EDGE_MARGIN: f64 = 24.0;

/// Place + size the overlay from config: saved pos, or bottom-center fallback.
fn position(win: &WebviewWindow, cfg: &OverlayConfig) -> tauri::Result<()> {
    let (w, h) = logical_size(cfg.scale);
    let (x, y) = match (cfg.pos_x, cfg.pos_y) {
        (Some(x), Some(y)) => (x, y),
        _ => match win.primary_monitor()? {
            Some(mon) => {
                let sf = mon.scale_factor();
                let mon_w = mon.size().width as f64 / sf;
                let mon_h = mon.size().height as f64 / sf;
                (
                    ((mon_w - w) / 2.0).max(0.0),
                    (mon_h - h - EDGE_MARGIN).max(0.0),
                )
            }
            None => (100.0, 100.0),
        },
    };
    win.set_size(LogicalSize::new(w, h))?;
    win.set_position(LogicalPosition::new(x, y))?;
    Ok(())
}

fn create(app: &tauri::AppHandle, cfg: &OverlayConfig) -> tauri::Result<()> {
    if app.get_webview_window(OVERLAY_LABEL).is_some() {
        return Ok(());
    }
    let (w, h) = logical_size(cfg.scale);
    let win = WebviewWindowBuilder::new(app, OVERLAY_LABEL, WebviewUrl::App("index.html".into()))
        .title("CS2 Helper Overlay")
        .inner_size(w, h)
        .transparent(true)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .focusable(false)
        .shadow(false)
        .resizable(false)
        .visible(true)
        .build()?;
    win.set_ignore_cursor_events(true)?;
    position(&win, cfg)?;
    Ok(())
}

/// Make the window match `cfg`: create / destroy / reposition, then broadcast
/// the new config so a running overlay updates its timer/scale immediately.
pub fn reconcile(app: &tauri::AppHandle, cfg: &OverlayConfig) {
    match (cfg.enabled, app.get_webview_window(OVERLAY_LABEL)) {
        (true, None) => {
            let _ = create(app, cfg);
        }
        (false, Some(win)) => {
            let _ = win.close();
        }
        (true, Some(win)) => {
            let _ = win.show();
            let _ = win.set_always_on_top(true);
            let _ = position(&win, cfg);
        }
        (false, None) => {}
    }
    let _ = app.emit("overlay:config", cfg);
}

/// Watchdog repair (runs ~1/s on the main thread): keep an enabled overlay
/// present, shown, and topmost. The OS can hide the window or drop it behind a
/// fullscreen game WITHOUT destroying it, so `reconcile`'s create path (which
/// only fires when the window is gone) never sees those cases. Re-assert the
/// volatile properties every tick; recreate only if the window was actually
/// torn down. Deliberately does NOT reposition: that would fight the user mid-
/// drag during overlay edit mode.
pub fn ensure_alive(app: &tauri::AppHandle, cfg: &OverlayConfig) {
    if !cfg.enabled {
        return;
    }
    match app.get_webview_window(OVERLAY_LABEL) {
        None => {
            let _ = create(app, cfg);
        }
        Some(win) => {
            if !win.is_visible().unwrap_or(true) {
                let _ = win.show();
            }
            // Cheap and focus-safe (set_always_on_top uses SWP_NOACTIVATE):
            // re-claims topmost even when the window is still "visible" but the
            // game has covered it.
            let _ = win.set_always_on_top(true);
        }
    }
}

/// Enter edit mode: stop ignoring the cursor so the user can drag the window,
/// and tell the overlay UI to show its move handle.
pub fn edit_start(app: &tauri::AppHandle) -> Result<(), String> {
    let win = app
        .get_webview_window(OVERLAY_LABEL)
        .ok_or("overlay window is not open")?;
    win.set_ignore_cursor_events(false).map_err(|e| e.to_string())?;
    let _ = app.emit("overlay:edit", true);
    Ok(())
}

/// Finish edit mode: read the dragged position (logical px), restore
/// click-through, and return the new x/y so the caller can persist it.
pub fn edit_finish(app: &tauri::AppHandle) -> Result<(f64, f64), String> {
    let win = app
        .get_webview_window(OVERLAY_LABEL)
        .ok_or("overlay window is not open")?;
    let pos = win.outer_position().map_err(|e| e.to_string())?;
    let sf = win.scale_factor().map_err(|e| e.to_string())?;
    win.set_ignore_cursor_events(true).map_err(|e| e.to_string())?;
    let _ = app.emit("overlay:edit", false);
    Ok((pos.x as f64 / sf, pos.y as f64 / sf))
}
