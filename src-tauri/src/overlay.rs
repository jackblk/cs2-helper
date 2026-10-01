//! Transparent, click-through, always-on-top window for the bomb defuse timer.
//! Exists while `OverlayConfig::enabled`, or for the length of an edit.
//! Geometry is in logical pixels.

use std::sync::Mutex;

use serde::{Deserialize, Serialize};
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

#[derive(Debug, Clone, Copy, Serialize)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// `x`/`y`/`scale` are the overlay's geometry when the edit began.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct EditSession {
    pub monitor: Rect,
    pub x: f64,
    pub y: f64,
    pub scale: f32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct EditResult {
    pub x: f64,
    pub y: f64,
    pub scale: f32,
}

/// While set, the window covers its monitor, so `reconcile` must neither
/// shrink nor close it. Kept here, not only sent as an event, because an
/// overlay page created for the edit loads after the event fired.
static EDIT_SESSION: Mutex<Option<EditSession>> = Mutex::new(None);

pub fn edit_session() -> Option<EditSession> {
    *EDIT_SESSION.lock().unwrap()
}

/// Saved or default top-left.
fn origin(win: &WebviewWindow, cfg: &OverlayConfig) -> tauri::Result<(f64, f64)> {
    if let (Some(x), Some(y)) = (cfg.pos_x, cfg.pos_y) {
        return Ok((x, y));
    }
    let (w, h) = logical_size(cfg.scale);
    Ok(match win.primary_monitor()? {
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
    })
}

fn position(win: &WebviewWindow, cfg: &OverlayConfig) -> tauri::Result<()> {
    let (w, h) = logical_size(cfg.scale);
    let (x, y) = origin(win, cfg)?;
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
    if edit_session().is_some() {
        // `edit_end` applies the final geometry and visibility.
        let _ = app.emit("overlay:config", cfg);
        return;
    }
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

/// Watchdog repair, run about once a second off the main thread. The OS can hide
/// the overlay or drop it behind a fullscreen game without destroying it, which
/// `reconcile` never notices. Deliberately does not reposition: that would
/// fight an edit in progress.
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

fn monitor_rect(win: &WebviewWindow) -> Result<Rect, String> {
    let mon = match win.current_monitor().map_err(|e| e.to_string())? {
        Some(m) => m,
        None => win
            .primary_monitor()
            .map_err(|e| e.to_string())?
            .ok_or("no monitor found")?,
    };
    let sf = mon.scale_factor();
    Ok(Rect {
        x: mon.position().x as f64 / sf,
        y: mon.position().y as f64 / sf,
        width: mon.size().width as f64 / sf,
        height: mon.size().height as f64 / sf,
    })
}

/// Covers the overlay's monitor so the page can drag and resize the badge
/// in JS. Creates the window when the overlay is disabled.
pub fn edit_start(app: &tauri::AppHandle, cfg: &OverlayConfig) -> Result<(), String> {
    if edit_session().is_some() {
        return Ok(());
    }
    if app.get_webview_window(OVERLAY_LABEL).is_none() {
        create(app, cfg).map_err(|e| e.to_string())?;
    }
    let win = app
        .get_webview_window(OVERLAY_LABEL)
        .ok_or("overlay window is not open")?;
    let (x, y) = origin(&win, cfg).map_err(|e| e.to_string())?;
    let monitor = monitor_rect(&win)?;
    let session = EditSession {
        monitor,
        x,
        y,
        scale: cfg.scale,
    };
    *EDIT_SESSION.lock().unwrap() = Some(session);
    let cover = (|| -> tauri::Result<()> {
        win.set_position(LogicalPosition::new(monitor.x, monitor.y))?;
        win.set_size(LogicalSize::new(monitor.width, monitor.height))?;
        win.set_ignore_cursor_events(false)
    })();
    if let Err(e) = cover {
        *EDIT_SESSION.lock().unwrap() = None;
        reconcile(app, cfg);
        return Err(e.to_string());
    }
    let _ = app.emit("overlay:edit", Some(session));
    Ok(())
}

/// `None` cancels. Window calls are best effort so both windows always leave
/// edit mode; a second call (Done pressed in both windows) is a no-op.
pub fn edit_end(app: &tauri::AppHandle, saved: &OverlayConfig, result: Option<EditResult>) {
    if EDIT_SESSION.lock().unwrap().take().is_none() {
        return;
    }
    if let Some(win) = app.get_webview_window(OVERLAY_LABEL) {
        if saved.enabled {
            let mut cfg = *saved;
            if let Some(r) = result {
                cfg.pos_x = Some(r.x);
                cfg.pos_y = Some(r.y);
                cfg.scale = r.scale;
            }
            let _ = win.set_ignore_cursor_events(true);
            let _ = position(&win, &cfg);
        } else {
            let _ = win.close();
        }
    }
    let _ = app.emit("overlay:edit", None::<EditSession>);
    let _ = app.emit("overlay:edit-result", result);
}
