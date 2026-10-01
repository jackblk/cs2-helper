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
mod hotkey;
mod overlay;

/// Payload age after which the game counts as not running.
const GSI_FRESH_MS: u64 = 5000;

struct Engine(Arc<events::runtime::EngineHandle>);

struct ConfigPath(std::path::PathBuf);

struct AppSettings(Mutex<config::AppConfig>);

/// Mirror of the engine's paused flag, so the tray status tick does not lock
/// the full snapshot every second.
struct Paused(Arc<Mutex<bool>>);

#[cfg(windows)]
#[tauri::command]
fn list_audio_sessions() -> Result<Vec<audio::AudioSessionInfo>, String> {
    audio::list_sessions().map_err(|e| e.to_string())
}

/// `volume` is 0.0..=1.0. Returns the number of sessions changed.
#[cfg(windows)]
#[tauri::command]
fn set_process_volume(process: String, volume: f32) -> Result<usize, String> {
    audio::set_volume_for_process(&process, volume).map_err(|e| e.to_string())
}

#[cfg(windows)]
#[tauri::command]
fn get_process_volume(process: String) -> Result<Option<f32>, String> {
    audio::get_volume_for_process(&process).map_err(|e| e.to_string())
}

#[tauri::command]
fn gsi_status(shared: tauri::State<'_, gsi::SharedGsi>) -> gsi::GsiStatus {
    gsi::status(&shared)
}

/// Returns the written path. CS2 only picks up the cfg after a restart.
#[tauri::command]
fn install_gsi_cfg(token: tauri::State<'_, gsi::GsiToken>) -> Result<String, String> {
    gsi::install::install_cfg(&token.0).map(|p| p.display().to_string())
}

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

/// CS2 keeps a loaded cfg until it restarts.
#[tauri::command]
fn uninstall_gsi_cfg() -> Result<(), String> {
    gsi::install::uninstall_cfg()
}

#[tauri::command]
fn engine_status(engine: tauri::State<'_, Engine>) -> events::runtime::EngineSnapshot {
    engine.0.snapshot()
}

/// Returned directly so the UI need not wait for the async `engine:update`,
/// which races command completion.
#[derive(serde::Serialize)]
struct ConfigPayload {
    config: config::Config,
    app: config::AppConfig,
    warnings: Vec<String>,
}

/// Settings changed outside the main window (tray), so it can refresh the form
/// and drop undo history that no longer matches the file.
const SETTINGS_CHANGED: &str = "settings:changed";

#[derive(Clone, serde::Serialize)]
struct SettingsChanged {
    config: config::Config,
    app: config::AppConfig,
}

fn load_and_apply_config(
    app_handle: &tauri::AppHandle,
    engine: &Engine,
    path: &ConfigPath,
    app: &AppSettings,
) -> ConfigPayload {
    let (cfg, app_cfg, mut warnings) = config::load_or_create(&path.0);
    if let Err(e) = hotkey::apply(app_handle, &app_cfg.hotkeys) {
        warnings.push(e);
    }
    engine.0.set_config(cfg, warnings.clone());
    *app.0.lock().unwrap() = app_cfg.clone();
    #[cfg(windows)]
    let _ = autostart::set_enabled(app_cfg.start_with_windows);
    overlay::reconcile(app_handle, &app_cfg.overlay);
    ConfigPayload {
        config: cfg,
        app: app_cfg,
        warnings,
    }
}

// Commands that can create the overlay window are `async`: on Windows,
// building a webview inside a synchronous command deadlocks (wry#583).
#[tauri::command]
async fn reload_config(
    app_handle: tauri::AppHandle,
    engine: tauri::State<'_, Engine>,
    path: tauri::State<'_, ConfigPath>,
    app: tauri::State<'_, AppSettings>,
) -> Result<ConfigPayload, String> {
    Ok(load_and_apply_config(&app_handle, &engine, &path, &app))
}

#[tauri::command]
async fn reset_config(
    app_handle: tauri::AppHandle,
    engine: tauri::State<'_, Engine>,
    path: tauri::State<'_, ConfigPath>,
    app: tauri::State<'_, AppSettings>,
) -> Result<ConfigPayload, String> {
    config::reset(&path.0)?;
    Ok(load_and_apply_config(&app_handle, &engine, &path, &app))
}

#[tauri::command]
fn config_path(path: tauri::State<'_, ConfigPath>) -> String {
    path.0.display().to_string()
}

#[tauri::command]
fn open_config_dir(app: tauri::AppHandle, path: tauri::State<'_, ConfigPath>) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .reveal_item_in_dir(path.0.clone())
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn set_paused(paused: bool, engine: tauri::State<'_, Engine>, flag: tauri::State<'_, Paused>) {
    engine.0.set_paused(paused);
    *flag.0.lock().unwrap() = paused;
}

fn toggle_paused(app: &tauri::AppHandle) {
    // `try_state`: the hotkey can fire before setup has managed the state.
    let (Some(engine), Some(flag)) = (app.try_state::<Engine>(), app.try_state::<Paused>()) else {
        return;
    };
    let next = !*flag.0.lock().unwrap();
    engine.0.set_paused(next);
    *flag.0.lock().unwrap() = next;
}

#[tauri::command]
fn hotkey_capture_start(app_handle: tauri::AppHandle) -> Result<(), String> {
    hotkey::suspend(&app_handle)
}

#[tauri::command]
fn hotkey_capture_end(
    app_handle: tauri::AppHandle,
    settings: tauri::State<'_, AppSettings>,
) -> Result<(), String> {
    let cfg = settings.0.lock().unwrap().hotkeys.clone();
    hotkey::apply(&app_handle, &cfg)
}

#[tauri::command]
fn get_app_settings(app: tauri::State<'_, AppSettings>) -> config::AppConfig {
    app.0.lock().unwrap().clone()
}

#[tauri::command]
async fn save_config(
    config: config::Config,
    app: config::AppConfig,
    app_handle: tauri::AppHandle,
    engine: tauri::State<'_, Engine>,
    path: tauri::State<'_, ConfigPath>,
    settings: tauri::State<'_, AppSettings>,
) -> Result<(), String> {
    // Register the hotkeys first: if a key is taken, keep the old ones and
    // save nothing, so the file never holds a key that does not work.
    let prev_hotkeys = settings.0.lock().unwrap().hotkeys.clone();
    hotkey::apply_or_revert(&app_handle, &app.hotkeys, &prev_hotkeys)?;
    config::write(&path.0, &config, &app)?;
    // UI values are already typed, so there are no parse warnings.
    engine.0.set_config(config, Vec::new());
    *settings.0.lock().unwrap() = app.clone();
    #[cfg(windows)]
    let _ = autostart::set_enabled(app.start_with_windows);
    overlay::reconcile(&app_handle, &app.overlay);
    Ok(())
}

#[tauri::command]
async fn overlay_edit_start(
    app_handle: tauri::AppHandle,
    settings: tauri::State<'_, AppSettings>,
) -> Result<(), String> {
    let cfg = settings.0.lock().unwrap().overlay;
    overlay::edit_start(&app_handle, &cfg)
}

/// Lets an overlay page created for the edit pick up the session it missed.
#[tauri::command]
fn overlay_edit_session() -> Option<overlay::EditSession> {
    overlay::edit_session()
}

/// `result: None` cancels. Writes nothing: the main window saves the result
/// with `save_config`, so it lands in its undo history.
#[tauri::command]
fn overlay_edit_end(
    result: Option<overlay::EditResult>,
    app_handle: tauri::AppHandle,
    settings: tauri::State<'_, AppSettings>,
) {
    let cfg = settings.0.lock().unwrap().overlay;
    overlay::edit_end(&app_handle, &cfg, result);
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
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if event.state() != tauri_plugin_global_shortcut::ShortcutState::Pressed {
                        return;
                    }
                    match hotkey::action_for(shortcut) {
                        Some(hotkey::Action::Pause) => toggle_paused(app),
                        None => {}
                    }
                })
                .build(),
        )
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            let token = gsi::load_or_create_token(&data_dir)
                .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;

            let config_file = data_dir.join(config::CONFIG_FILE_NAME);
            let (cfg, app_config, mut warnings) = config::load_or_create(&config_file);
            // A taken or invalid hotkey must not block startup.
            if let Err(e) = hotkey::apply(app.handle(), &app_config.hotkeys) {
                warnings.push(e);
            }
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
                // Non-fatal: the app stays usable and the status shows not-running.
                Err(e) => eprintln!("GSI server failed to start: {e}"),
            }

            let paused_flag = Arc::new(Mutex::new(false));

            // Config is the source of truth for autostart: make the OS match it.
            #[cfg(windows)]
            let _ = autostart::set_enabled(app_config.start_with_windows);

            app.manage(shared);
            app.manage(gsi::GsiToken(token));
            app.manage(Engine(engine));
            app.manage(ConfigPath(config_file));
            app.manage(AppSettings(Mutex::new(app_config)));
            app.manage(Paused(paused_flag));

            let window = app.get_webview_window("main").expect("main window exists");

            {
                let overlay_cfg = app.state::<AppSettings>().0.lock().unwrap().overlay;
                overlay::reconcile(app.handle(), &overlay_cfg);
            }

            // Started minimized: no window confirms the launch, so notify.
            if !app.state::<AppSettings>().0.lock().unwrap().start_minimized {
                let _ = window.show();
                let _ = window.set_focus();
            } else {
                use tauri_plugin_notification::NotificationExt;
                let _ = app
                    .notification()
                    .builder()
                    .title("CS2 Helper")
                    .body("Started in the tray. Click the tray icon to open.")
                    .show();
            }

            // Close (X) hides to the tray instead of quitting.
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
                    "pause" => toggle_paused(app),
                    "startup" => {
                        let enabled = startup_for_event.is_checked().unwrap_or(false);
                        #[cfg(windows)]
                        let _ = autostart::set_enabled(enabled);
                        let app_state = app.state::<AppSettings>();
                        let mut app_cfg = app_state.0.lock().unwrap().clone();
                        app_cfg.start_with_windows = enabled;
                        *app_state.0.lock().unwrap() = app_cfg.clone();
                        let cfg = app.state::<Engine>().0.snapshot().config.unwrap_or_default();
                        let path = app.state::<ConfigPath>();
                        let _ = config::write(&path.0, &cfg, &app_cfg);
                        let _ = app.emit(
                            SETTINGS_CHANGED,
                            SettingsChanged {
                                config: cfg,
                                app: app_cfg,
                            },
                        );
                    }
                    "reload" => {
                        // Off the main thread: it can create the overlay
                        // window, which deadlocks inside a menu handler.
                        let app = app.clone();
                        std::thread::spawn(move || {
                            let payload = load_and_apply_config(
                                &app,
                                &app.state::<Engine>(),
                                &app.state::<ConfigPath>(),
                                &app.state::<AppSettings>(),
                            );
                            let _ = app.emit(
                                SETTINGS_CHANGED,
                                SettingsChanged {
                                    config: payload.config,
                                    app: payload.app,
                                },
                            );
                        });
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

            // Polled because "Waiting for CS2" depends on GSI freshness, which
            // is time-based.
            let paused_for_tick = app.state::<Paused>().0.clone();
            let gsi_for_tick = app.state::<gsi::SharedGsi>().inner().clone();
            let tick_app = app.handle().clone();
            std::thread::spawn(move || loop {
                std::thread::sleep(std::time::Duration::from_secs(1));
                let paused = *paused_for_tick.lock().unwrap();
                let fresh = gsi::status(&gsi_for_tick)
                    .last_payload_age_ms
                    .is_some_and(|ms| ms < GSI_FRESH_MS);
                let label = if paused {
                    "Paused"
                } else if fresh {
                    "Running"
                } else {
                    "Waiting for CS2"
                };
                let _ = status_i.set_text(label);
                let _ = pause_i.set_text(if paused { "Resume" } else { "Pause" });

                // Overlay watchdog: fullscreen or resolution changes and GPU
                // surface loss can destroy, hide, or bury the overlay, and the
                // OS never restores it. Runs on this thread, not the main
                // one: recreating the window there deadlocks.
                let cfg = tick_app.state::<AppSettings>().0.lock().unwrap().overlay;
                overlay::ensure_alive(&tick_app, &cfg);
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
            uninstall_gsi_cfg,
            engine_status,
            reload_config,
            reset_config,
            config_path,
            open_config_dir,
            set_paused,
            get_app_settings,
            save_config,
            overlay_edit_start,
            overlay_edit_session,
            overlay_edit_end,
            hotkey_capture_start,
            hotkey_capture_end
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
