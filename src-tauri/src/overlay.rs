//! Transparent, click-through, always-on-top window for the bomb defuse timer.
//! Exists only while `OverlayConfig::enabled`. Geometry is in logical pixels.

use tauri::{
    Emitter, LogicalPosition, LogicalSize, Manager, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
};

use crate::config::OverlayConfig;

pub const OVERLAY_LABEL: &str = "overlay";
/// Logical size at scale 1.0; the badge fills it.
const BASE_W: f64 = 240.0;
const BASE_H: f64 = 96.0;

fn logical_size(scale: f32) -> (f64, f64) {
    let s = scale as f64;
    (BASE_W * s, BASE_H * s)
}

/// Logical px between the screen edge and the default bottom-center position.
const EDGE_MARGIN: f64 = 24.0;

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

/// Watchdog repair, run about once a second on the main thread. The OS can hide
/// the overlay or drop it behind a fullscreen game without destroying it, which
/// `reconcile` never notices. Deliberately does not reposition: that would
/// fight a drag in edit mode.
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
            // Focus-safe (SWP_NOACTIVATE). Re-claims topmost even when the
            // window is still "visible" but the game has covered it.
            let _ = win.set_always_on_top(true);
        }
    }
}

/// Unsaved size preview: `Some(scale)` resizes and shows a sample reading,
/// `None` restores the saved size and hides the sample.
pub fn preview_scale(app: &tauri::AppHandle, saved: &OverlayConfig, scale: Option<f32>) {
    let Some(win) = app.get_webview_window(OVERLAY_LABEL) else {
        return;
    };
    let mut cfg = *saved;
    if let Some(s) = scale {
        cfg.scale = s;
    }
    let _ = position(&win, &cfg);
    let _ = app.emit("overlay:config", cfg);
    let _ = app.emit("overlay:sample", scale.is_some());
}

pub fn edit_start(app: &tauri::AppHandle) -> Result<(), String> {
    let win = app
        .get_webview_window(OVERLAY_LABEL)
        .ok_or("overlay window is not open")?;
    win.set_ignore_cursor_events(false).map_err(|e| e.to_string())?;
    let _ = app.emit("overlay:edit", true);
    Ok(())
}

/// Returns the dragged position in logical px.
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
