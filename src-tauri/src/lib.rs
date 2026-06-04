use std::sync::{Arc, Mutex};

use tauri::menu::{CheckMenuItemBuilder, Menu, MenuItemBuilder, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Emitter, Manager, WindowEvent};

#[cfg(windows)]
mod audio;
#[cfg(windows)]
mod autostart;
mod config;
mod events;
mod gsi;

/// Managed state: the engine handle (shared with the GSI callback).
struct Engine(Arc<events::runtime::EngineHandle>);

/// Managed state: absolute path of config.toml.
struct ConfigPath(std::path::PathBuf);

/// Managed state: the live app-level settings (the `[app]` table).
struct AppSettings(Mutex<config::AppConfig>);

/// Managed state: a cheap mirror of the engine's paused flag (for the tray
/// status tick, which should not lock the full snapshot every second).
struct Paused(Arc<Mutex<bool>>);

/// List all per-process audio sessions on active output devices (diagnostics).
#[cfg(windows)]
#[tauri::command]
fn list_audio_sessions() -> Result<Vec<audio::AudioSessionInfo>, String> {
    audio::list_sessions().map_err(|e| e.to_string())
}

/// Set the volume (0.0..=1.0) for every session of `process` (e.g. "cs2.exe").
/// Returns how many sessions were changed.
#[cfg(windows)]
#[tauri::command]
fn set_process_volume(process: String, volume: f32) -> Result<usize, String> {
    audio::set_volume_for_process(&process, volume).map_err(|e| e.to_string())
}

/// Read the volume of the first session matching `process`.
#[cfg(windows)]
#[tauri::command]
fn get_process_volume(process: String) -> Result<Option<f32>, String> {
    audio::get_volume_for_process(&process).map_err(|e| e.to_string())
}

/// GSI server + cfg status for the debug panel.
#[tauri::command]
fn gsi_status(shared: tauri::State<'_, gsi::SharedGsi>) -> gsi::GsiStatus {
    gsi::status(&shared)
}

/// Write gamestate_integration_cs2helper.cfg into the CS2 cfg directory.
/// Returns the written path. User must restart CS2 afterwards.
#[tauri::command]
fn install_gsi_cfg(token: tauri::State<'_, gsi::GsiToken>) -> Result<String, String> {
    gsi::install::install_cfg(&token.0).map(|p| p.display().to_string())
}

/// Open the CS2 cfg directory in the OS file explorer. Reveals our cfg file if
/// it exists, otherwise opens the containing folder.
#[tauri::command]
fn open_gsi_cfg_dir(app: tauri::AppHandle) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    let dir = gsi::install::find_cs2_cfg_dir().ok_or("CS2 installation not found")?;
    let target = gsi::install::cfg_target_path().filter(|p| p.exists());
    match target {
        Some(cfg) => app.opener().reveal_item_in_dir(cfg),
        None => app.opener().open_path(dir.display().to_string(), None::<&str>),
    }
    .map_err(|e| e.to_string())
}

/// Current engine snapshot (duck state, event log, config) for the debug panel.
#[tauri::command]
fn engine_status(engine: tauri::State<'_, Engine>) -> events::runtime::EngineSnapshot {
    engine.0.snapshot()
}

/// The config after a (re)load, so the UI can show it without waiting for the
/// async `engine:update` round-trip (which races command completion).
#[derive(serde::Serialize)]
struct ConfigPayload {
    config: config::Config,
    app: config::AppConfig,
    warnings: Vec<String>,
}

/// Re-read config.toml and push it to the engine. Returns the loaded config.
#[tauri::command]
fn reload_config(
    engine: tauri::State<'_, Engine>,
    path: tauri::State<'_, ConfigPath>,
    app: tauri::State<'_, AppSettings>,
) -> ConfigPayload {
    let (cfg, app_cfg, warnings) = config::load_or_create(&path.0);
    engine.0.set_config(cfg, warnings.clone());
    *app.0.lock().unwrap() = app_cfg;
    #[cfg(windows)]
    let _ = autostart::set_enabled(app_cfg.start_with_windows);
    ConfigPayload {
        config: cfg,
        app: app_cfg,
        warnings,
    }
}

/// Back up config.toml, restore defaults, push to the engine. Returns defaults.
#[tauri::command]
fn reset_config(
    engine: tauri::State<'_, Engine>,
    path: tauri::State<'_, ConfigPath>,
    app: tauri::State<'_, AppSettings>,
) -> Result<ConfigPayload, String> {
    config::reset(&path.0)?;
    let (cfg, app_cfg, warnings) = config::load_or_create(&path.0);
    engine.0.set_config(cfg, warnings.clone());
    *app.0.lock().unwrap() = app_cfg;
    #[cfg(windows)]
    let _ = autostart::set_enabled(app_cfg.start_with_windows);
    Ok(ConfigPayload {
        config: cfg,
        app: app_cfg,
        warnings,
    })
}

/// Absolute path of config.toml (for display in the debug panel).
#[tauri::command]
fn config_path(path: tauri::State<'_, ConfigPath>) -> String {
    path.0.display().to_string()
}

/// Reveal config.toml in the OS file explorer.
#[tauri::command]
fn open_config_dir(app: tauri::AppHandle, path: tauri::State<'_, ConfigPath>) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .reveal_item_in_dir(path.0.clone())
        .map_err(|e| e.to_string())
}

/// Toggle the global pause (tray + debug panel). Restores 100% when pausing.
#[tauri::command]
fn set_paused(paused: bool, engine: tauri::State<'_, Engine>, flag: tauri::State<'_, Paused>) {
    engine.0.set_paused(paused);
    *flag.0.lock().unwrap() = paused;
}

/// Current app-level settings (the `[app]` table).
#[tauri::command]
fn get_app_settings(app: tauri::State<'_, AppSettings>) -> config::AppConfig {
    *app.0.lock().unwrap()
}

/// Persist UI-edited config straight to config.toml (M4.2), push it to the
/// engine, and re-sync autostart. Mirrors `reload_config` but takes the values
/// from the UI instead of re-reading the file. No warnings: values are typed.
#[tauri::command]
fn save_config(
    config: config::Config,
    app: config::AppConfig,
    engine: tauri::State<'_, Engine>,
    path: tauri::State<'_, ConfigPath>,
    settings: tauri::State<'_, AppSettings>,
) -> Result<(), String> {
    config::write(&path.0, &config, &app)?;
    engine.0.set_config(config, Vec::new());
    *settings.0.lock().unwrap() = app;
    #[cfg(windows)]
    let _ = autostart::set_enabled(app.start_with_windows);
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
                let _ = w.set_focus();
            }
        }))
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            let token = gsi::load_or_create_token(&data_dir)
                .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;

            // Config + event engine (M3).
            let config_file = data_dir.join(config::CONFIG_FILE_NAME);
            let (cfg, app_config, warnings) = config::load_or_create(&config_file);
            let engine_emit = app.handle().clone();
            let engine = Arc::new(events::runtime::start_engine(
                events::Cs2Audio,
                cfg,
                warnings,
                events::runtime::EngineOptions::default(),
                move |snap| {
                    let _ = engine_emit.emit("engine:update", &snap);
                },
            ));

            // GSI server (M2) — also feeds the engine.
            let shared: gsi::SharedGsi = Default::default();
            let handle = app.handle().clone();
            let engine_for_gsi = engine.clone();
            let bind = format!("127.0.0.1:{}", gsi::install::GSI_PORT);
            match gsi::server::start_server(&bind, token.clone(), shared.clone(), move |update| {
                engine_for_gsi.send_state(update.state.clone());
                let _ = handle.emit("gsi:update", &update);
            }) {
                Ok(port) => {
                    let mut g = shared.lock().unwrap();
                    g.running = true;
                    g.port = port;
                }
                // Port taken / bind failure: app stays usable, status shows
                // not-running, debug panel surfaces it. Don't crash.
                Err(e) => eprintln!("GSI server failed to start: {e}"),
            }

            let paused_flag = Arc::new(Mutex::new(false));

            // Autostart: config is the source of truth; make the OS match it.
            #[cfg(windows)]
            let _ = autostart::set_enabled(app_config.start_with_windows);

            app.manage(shared);
            app.manage(gsi::GsiToken(token));
            app.manage(Engine(engine));
            app.manage(ConfigPath(config_file));
            app.manage(AppSettings(Mutex::new(app_config)));
            app.manage(Paused(paused_flag));

            let window = app.get_webview_window("main").expect("main window exists");

            // Show now unless the user opted to start minimized in the tray.
            if !app.state::<AppSettings>().0.lock().unwrap().start_minimized {
                let _ = window.show();
                let _ = window.set_focus();
            }

            // Close (X) hides to the tray instead of quitting; the engine keeps
            // running. Show a one-time "still running" notification on first hide.
            let hint_shown = Arc::new(std::sync::atomic::AtomicBool::new(false));
            let app_handle = app.handle().clone();
            window.clone().on_window_event(move |event| {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    if let Some(w) = app_handle.get_webview_window("main") {
                        let _ = w.hide();
                    }
                    if !hint_shown.swap(true, std::sync::atomic::Ordering::SeqCst) {
                        use tauri_plugin_notification::NotificationExt;
                        let _ = app_handle
                            .notification()
                            .builder()
                            .title("CS2 Helper")
                            .body("Still running in the tray. Right-click the tray icon to exit.")
                            .show();
                    }
                }
            });

            // ---- System tray -------------------------------------------------
            let start_with_windows =
                app.state::<AppSettings>().0.lock().unwrap().start_with_windows;

            let status_i = MenuItemBuilder::with_id("status", "Waiting for CS2")
                .enabled(false)
                .build(app)?;
            let pause_i = MenuItemBuilder::with_id("pause", "Pause").build(app)?;
            let startup_i = CheckMenuItemBuilder::with_id("startup", "Run on startup")
                .checked(start_with_windows)
                .build(app)?;
            let reload_i = MenuItemBuilder::with_id("reload", "Reload config").build(app)?;
            let openfolder_i =
                MenuItemBuilder::with_id("openfolder", "Open config folder").build(app)?;
            let show_i = MenuItemBuilder::with_id("show", "Show window").build(app)?;
            let exit_i = MenuItemBuilder::with_id("exit", "Exit").build(app)?;
            let sep1 = PredefinedMenuItem::separator(app)?;
            let sep2 = PredefinedMenuItem::separator(app)?;

            let menu = Menu::with_items(
                app,
                &[
                    &status_i, &sep1, &pause_i, &startup_i, &reload_i, &openfolder_i, &sep2,
                    &show_i, &exit_i,
                ],
            )?;

            let startup_for_event = startup_i.clone();
            let _tray = TrayIconBuilder::with_id("main")
                .icon(app.default_window_icon().expect("bundled icon").clone())
                .tooltip("CS2 Helper")
                .menu(&menu)
                .on_menu_event(move |app, event| match event.id().as_ref() {
                    "pause" => {
                        let flag = app.state::<Paused>();
                        let next = !*flag.0.lock().unwrap();
                        app.state::<Engine>().0.set_paused(next);
                        *flag.0.lock().unwrap() = next;
                    }
                    "startup" => {
                        let enabled = startup_for_event.is_checked().unwrap_or(false);
                        #[cfg(windows)]
                        let _ = autostart::set_enabled(enabled);
                        let app_state = app.state::<AppSettings>();
                        let mut app_cfg = *app_state.0.lock().unwrap();
                        app_cfg.start_with_windows = enabled;
                        *app_state.0.lock().unwrap() = app_cfg;
                        let cfg = app.state::<Engine>().0.snapshot().config.unwrap_or_default();
                        let path = app.state::<ConfigPath>();
                        let _ = config::write(&path.0, &cfg, &app_cfg);
                    }
                    "reload" => {
                        let path = app.state::<ConfigPath>();
                        let (cfg, app_cfg, warnings) = config::load_or_create(&path.0);
                        app.state::<Engine>().0.set_config(cfg, warnings);
                        *app.state::<AppSettings>().0.lock().unwrap() = app_cfg;
                        #[cfg(windows)]
                        let _ = autostart::set_enabled(app_cfg.start_with_windows);
                    }
                    "openfolder" => {
                        use tauri_plugin_opener::OpenerExt;
                        let path = app.state::<ConfigPath>();
                        let _ = app.opener().reveal_item_in_dir(path.0.clone());
                    }
                    "show" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                    "exit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        if let Some(w) = tray.app_handle().get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                })
                .build(app)?;

            // Live tray status: paused mirror is event-driven, but "Waiting for
            // CS2" depends on GSI freshness (time-based), so poll once a second.
            let paused_for_tick = app.state::<Paused>().0.clone();
            let gsi_for_tick = app.state::<gsi::SharedGsi>().inner().clone();
            std::thread::spawn(move || loop {
                std::thread::sleep(std::time::Duration::from_secs(1));
                let paused = *paused_for_tick.lock().unwrap();
                let fresh = gsi::status(&gsi_for_tick)
                    .last_payload_age_ms
                    .is_some_and(|ms| ms < 5000);
                let label = if paused {
                    "Paused"
                } else if fresh {
                    "Running"
                } else {
                    "Waiting for CS2"
                };
                let _ = status_i.set_text(label);
                let _ = pause_i.set_text(if paused { "Resume" } else { "Pause" });
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_audio_sessions,
            set_process_volume,
            get_process_volume,
            gsi_status,
            install_gsi_cfg,
            open_gsi_cfg_dir,
            engine_status,
            reload_config,
            reset_config,
            config_path,
            open_config_dir,
            set_paused,
            get_app_settings,
            save_config
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            // Never leave the game ducked: restore on exit, then stop the thread.
            if let tauri::RunEvent::Exit = event {
                app.state::<Engine>().0.shutdown();
            }
        });
}
