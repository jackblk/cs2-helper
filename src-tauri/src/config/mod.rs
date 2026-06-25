//! File-based TOML configuration (M3).
//!
//! Lenient by design (root spec "Configuration"): parsing never hard-errors.
//! Unknown tables/keys warn and are ignored; invalid values warn and fall
//! back to that key's default; an unreadable file falls back to all defaults.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

pub const CONFIG_FILE_NAME: &str = "config.toml";

/// One on/off event reduction. `volume` is the absolute target (0.0..=1.0).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EventCfg {
    pub enabled: bool,
    pub volume: f32,
}

/// A timed reduction (death, bomb): duck for `duration_ms`, then restore.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TimedEventCfg {
    pub enabled: bool,
    pub volume: f32,
    pub duration_ms: u64,
}

/// In-game overlay settings (the `[overlay]` table). `pos_*` in logical pixels;
/// `None` means default top-center placement. `scale` multiplies the base size.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct OverlayConfig {
    pub enabled: bool,
    pub c4_timer_s: f32,
    pub safety_margin_s: f32,
    pub pos_x: Option<f64>,
    pub pos_y: Option<f64>,
    pub scale: f32,
}

impl Default for OverlayConfig {
    fn default() -> Self {
        OverlayConfig {
            enabled: true,
            c4_timer_s: 40.0,
            safety_margin_s: 0.0,
            pos_x: None,
            pos_y: None,
            scale: 1.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Config {
    pub death: TimedEventCfg,
    pub flash: EventCfg,
    pub bomb: TimedEventCfg,
    pub spectator: EventCfg,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            death: TimedEventCfg {
                enabled: true,
                volume: 0.30,
                duration_ms: 1000,
            },
            flash: EventCfg {
                enabled: true,
                volume: 0.15,
            },
            bomb: TimedEventCfg {
                enabled: true,
                volume: 0.30,
                duration_ms: 1500,
            },
            spectator: EventCfg {
                enabled: true,
                volume: 0.60,
            },
        }
    }
}

/// App-level settings (the `[app]` table). Source of truth for autostart.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AppConfig {
    /// Launch with Windows (a Startup-folder shortcut; not the registry).
    pub start_with_windows: bool,
    /// Start hidden in the tray instead of showing the window.
    pub start_minimized: bool,
    /// `#[serde(default)]` so UI JSON / older files without `[overlay]` still load.
    #[serde(default)]
    pub overlay: OverlayConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            start_with_windows: false,
            start_minimized: false,
            overlay: OverlayConfig::default(),
        }
    }
}

/// Render the config as commented TOML with the given values. Round-trips:
/// `parse(serialize(c, a))` yields `(c, a, [])`.
pub fn serialize(config: &Config, app: &AppConfig) -> String {
    let b = |v: bool| if v { "true" } else { "false" };
    let base = format!(
        r#"# CS2 Helper configuration.
# Volumes are absolute targets on the 0.0-1.0 scale (0.15 = 15%).
# When several reductions are active the smallest volume wins.
# Restore always goes back to 100%.
# Reload from the app after editing (debug panel / tray).

# Local player dies: reduce volume for duration_ms (spectator takes over while dead).
[audio.death]
enabled = {death_enabled}
volume = {death_volume:.2}
duration_ms = {death_duration}

# Player (or spectated teammate) is flashed: reduce volume until the flash ends.
[audio.flash]
enabled = {flash_enabled}
volume = {flash_volume:.2}

# Bomb explodes: reduce volume for duration_ms.
[audio.bomb]
enabled = {bomb_enabled}
volume = {bomb_volume:.2}
duration_ms = {bomb_duration}

# Watching a teammate after death: reduce volume until you regain control.
[audio.spectator]
enabled = {spectator_enabled}
volume = {spectator_volume:.2}

# App behavior.
# start_with_windows: launch on login via a Startup-folder shortcut (no registry).
# start_minimized: start hidden in the tray instead of showing the window.
[app]
start_with_windows = {start_with_windows}
start_minimized = {start_minimized}
"#,
        death_enabled = b(config.death.enabled),
        death_volume = config.death.volume,
        death_duration = config.death.duration_ms,
        flash_enabled = b(config.flash.enabled),
        flash_volume = config.flash.volume,
        bomb_enabled = b(config.bomb.enabled),
        bomb_volume = config.bomb.volume,
        bomb_duration = config.bomb.duration_ms,
        spectator_enabled = b(config.spectator.enabled),
        spectator_volume = config.spectator.volume,
        start_with_windows = b(app.start_with_windows),
        start_minimized = b(app.start_minimized),
    );

    let overlay_pos = match (app.overlay.pos_x, app.overlay.pos_y) {
        (Some(x), Some(y)) => format!("pos_x = {x}\npos_y = {y}\n"),
        _ => String::new(),
    };
    let overlay_section = format!(
        "\n# In-game bomb defuse timer overlay.\n\
         # enabled: show the overlay window. c4_timer_s: bomb fuse seconds (40 default).\n\
         # safety_margin_s: subtract from remaining time before coloring (0 = pure math).\n\
         # scale: size multiplier. pos_x/pos_y: logical-pixel top-left (omit for top-center).\n\
         [overlay]\n\
         enabled = {enabled}\n\
         c4_timer_s = {c4:.1}\n\
         safety_margin_s = {margin:.1}\n\
         scale = {scale:.2}\n\
         {pos}",
        enabled = b(app.overlay.enabled),
        c4 = app.overlay.c4_timer_s,
        margin = app.overlay.safety_margin_s,
        scale = app.overlay.scale,
        pos = overlay_pos,
    );
    format!("{base}{overlay_section}")
}

/// The commented default config written on first run and by `reset`.
pub fn default_file_contents() -> String {
    serialize(&Config::default(), &AppConfig::default())
}

/// Write the config file from the given values (no backup). Used to persist
/// settings changed from the UI / tray.
pub fn write(path: &Path, config: &Config, app: &AppConfig) -> Result<(), String> {
    fs::write(path, serialize(config, app))
        .map_err(|e| format!("write {}: {e}", path.display()))
}

/// Load the config, creating a commented default file on first run.
/// Never fails: any problem falls back to defaults plus a warning.
pub fn load_or_create(path: &Path) -> (Config, AppConfig, Vec<String>) {
    match fs::read_to_string(path) {
        Ok(text) => parse(&text),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            if let Some(dir) = path.parent() {
                let _ = fs::create_dir_all(dir);
            }
            if let Err(e) = fs::write(path, default_file_contents()) {
                return (
                    Config::default(),
                    AppConfig::default(),
                    vec![format!("could not write default config {}: {e}", path.display())],
                );
            }
            (Config::default(), AppConfig::default(), Vec::new())
        }
        Err(e) => (
            Config::default(),
            AppConfig::default(),
            vec![format!("could not read {}: {e}; using defaults", path.display())],
        ),
    }
}

/// Back the current file up to `config.toml.bak`, then write defaults.
pub fn reset(path: &Path) -> Result<(), String> {
    if path.exists() {
        let backup = path.with_extension("toml.bak");
        fs::copy(path, &backup).map_err(|e| format!("backup {}: {e}", backup.display()))?;
    }
    fs::write(path, default_file_contents()).map_err(|e| format!("write {}: {e}", path.display()))
}

/// Parse config text leniently. Always returns a usable `Config`; problems
/// are reported as human-readable warnings (shown in the debug panel).
pub fn parse(text: &str) -> (Config, AppConfig, Vec<String>) {
    let mut cfg = Config::default();
    let mut app = AppConfig::default();
    let mut warnings = Vec::new();

    let root: toml::Table = match text.parse() {
        Ok(t) => t,
        Err(e) => {
            warnings.push(format!("config is not valid TOML, using defaults: {e}"));
            return (cfg, app, warnings);
        }
    };

    for (key, value) in &root {
        match key.as_str() {
            "audio" => {
                let Some(audio) = value.as_table() else {
                    warnings.push("\"audio\" is not a table; ignored".to_string());
                    continue;
                };
                for (event, value) in audio {
                    let Some(table) = value.as_table() else {
                        warnings.push(format!("[audio.{event}] is not a table; ignored"));
                        continue;
                    };
                    match event.as_str() {
                        "death" => apply_timed(table, "death", &mut cfg.death, &mut warnings),
                        "flash" => apply_event(table, "flash", &mut cfg.flash, &mut warnings),
                        "bomb" => apply_timed(table, "bomb", &mut cfg.bomb, &mut warnings),
                        "spectator" => {
                            apply_event(table, "spectator", &mut cfg.spectator, &mut warnings)
                        }
                        other => {
                            warnings.push(format!("unknown table [audio.{other}] ignored"))
                        }
                    }
                }
            }
            "app" => {
                let Some(table) = value.as_table() else {
                    warnings.push("\"app\" is not a table; ignored".to_string());
                    continue;
                };
                apply_app(table, &mut app, &mut warnings);
            }
            "overlay" => {
                let Some(table) = value.as_table() else {
                    warnings.push("\"overlay\" is not a table; ignored".to_string());
                    continue;
                };
                apply_overlay(table, &mut app.overlay, &mut warnings);
            }
            other => warnings.push(format!("unknown table [{other}] ignored")),
        }
    }
    (cfg, app, warnings)
}

fn apply_app(table: &toml::Table, out: &mut AppConfig, warnings: &mut Vec<String>) {
    for (key, value) in table {
        match key.as_str() {
            "start_with_windows" => {
                set_bool(value, "app", key, &mut out.start_with_windows, warnings)
            }
            "start_minimized" => set_bool(value, "app", key, &mut out.start_minimized, warnings),
            _ => warnings.push(format!("unknown key app.{key} ignored")),
        }
    }
}

fn apply_overlay(table: &toml::Table, out: &mut OverlayConfig, warnings: &mut Vec<String>) {
    for (key, value) in table {
        match key.as_str() {
            "enabled" => set_bool(value, "overlay", key, &mut out.enabled, warnings),
            "c4_timer_s" => set_pos_float(value, key, &mut out.c4_timer_s, warnings),
            "safety_margin_s" => set_nonneg_float(value, key, &mut out.safety_margin_s, warnings),
            "scale" => set_scale(value, key, &mut out.scale, warnings),
            "pos_x" => set_opt_float(value, key, &mut out.pos_x, warnings),
            "pos_y" => set_opt_float(value, key, &mut out.pos_y, warnings),
            _ => warnings.push(format!("unknown key overlay.{key} ignored")),
        }
    }
}

fn as_f64(value: &toml::Value) -> Option<f64> {
    value.as_float().or_else(|| value.as_integer().map(|i| i as f64))
}

fn set_pos_float(value: &toml::Value, key: &str, out: &mut f32, warnings: &mut Vec<String>) {
    match as_f64(value) {
        Some(v) if v > 0.0 => *out = v as f32,
        _ => warnings.push(format!(
            "overlay.{key}: expected a number > 0, got {value}; keeping {out}"
        )),
    }
}

fn set_nonneg_float(value: &toml::Value, key: &str, out: &mut f32, warnings: &mut Vec<String>) {
    match as_f64(value) {
        Some(v) if v >= 0.0 => *out = v as f32,
        _ => warnings.push(format!(
            "overlay.{key}: expected a number >= 0, got {value}; keeping {out}"
        )),
    }
}

fn set_scale(value: &toml::Value, key: &str, out: &mut f32, warnings: &mut Vec<String>) {
    match as_f64(value) {
        Some(v) if (0.25..=4.0).contains(&v) => *out = v as f32,
        _ => warnings.push(format!(
            "overlay.{key}: expected a number in 0.25..=4.0, got {value}; keeping {out}"
        )),
    }
}

fn set_opt_float(value: &toml::Value, key: &str, out: &mut Option<f64>, warnings: &mut Vec<String>) {
    match as_f64(value) {
        Some(v) if v.is_finite() => *out = Some(v),
        _ => warnings.push(format!(
            "overlay.{key}: expected a finite number, got {value}; ignored"
        )),
    }
}

fn apply_event(table: &toml::Table, name: &str, out: &mut EventCfg, warnings: &mut Vec<String>) {
    for (key, value) in table {
        match key.as_str() {
            "enabled" => set_bool(value, &format!("audio.{name}"), key, &mut out.enabled, warnings),
            "volume" => set_volume(value, name, key, &mut out.volume, warnings),
            _ => warnings.push(format!("unknown key audio.{name}.{key} ignored")),
        }
    }
}

fn apply_timed(
    table: &toml::Table,
    name: &str,
    out: &mut TimedEventCfg,
    warnings: &mut Vec<String>,
) {
    for (key, value) in table {
        match key.as_str() {
            "enabled" => set_bool(value, &format!("audio.{name}"), key, &mut out.enabled, warnings),
            "volume" => set_volume(value, name, key, &mut out.volume, warnings),
            "duration_ms" => set_duration(value, name, key, &mut out.duration_ms, warnings),
            _ => warnings.push(format!("unknown key audio.{name}.{key} ignored")),
        }
    }
}

fn set_bool(
    value: &toml::Value,
    prefix: &str,
    key: &str,
    out: &mut bool,
    warnings: &mut Vec<String>,
) {
    match value.as_bool() {
        Some(b) => *out = b,
        None => warnings.push(format!(
            "{prefix}.{key}: expected true/false, got {value}; keeping {out}"
        )),
    }
}

fn set_volume(
    value: &toml::Value,
    section: &str,
    key: &str,
    out: &mut f32,
    warnings: &mut Vec<String>,
) {
    match as_f64(value) {
        Some(v) if (0.0..=1.0).contains(&v) => *out = v as f32,
        _ => warnings.push(format!(
            "audio.{section}.{key}: expected a number in 0.0..=1.0, got {value}; keeping {out}"
        )),
    }
}

fn set_duration(
    value: &toml::Value,
    section: &str,
    key: &str,
    out: &mut u64,
    warnings: &mut Vec<String>,
) {
    match value.as_integer() {
        Some(ms) if ms > 0 => *out = ms as u64,
        _ => warnings.push(format!(
            "audio.{section}.{key}: expected an integer > 0, got {value}; keeping {out}"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlay_defaults_and_round_trip() {
        let o = OverlayConfig::default();
        assert!(o.enabled);
        assert_eq!(o.c4_timer_s, 40.0);
        assert_eq!(o.safety_margin_s, 0.0);
        assert_eq!(o.scale, 1.0);
        assert_eq!(o.pos_x, None);
        assert_eq!(o.pos_y, None);

        // Round-trip with a saved position and non-default scale.
        let mut a = AppConfig::default();
        a.overlay.enabled = false;
        a.overlay.scale = 1.5;
        a.overlay.pos_x = Some(100.0);
        a.overlay.pos_y = Some(40.0);
        let (_c, a2, warnings) = parse(&serialize(&Config::default(), &a));
        assert_eq!(a2.overlay, a.overlay);
        assert!(warnings.is_empty(), "{warnings:?}");
    }

    #[test]
    fn overlay_unknown_key_and_bad_type_warn() {
        let (_c, a, warnings) =
            parse("[overlay]\nenabled = \"yes\"\nscale = 1.25\nfoo = 1\n");
        assert!(a.overlay.enabled, "bad bool keeps default true");
        assert_eq!(a.overlay.scale, 1.25);
        assert!(warnings.iter().any(|w| w.contains("overlay.enabled")));
        assert!(warnings.iter().any(|w| w.contains("overlay.foo")));
        assert!(
            warnings.iter().all(|w| !w.contains("audio.overlay")),
            "overlay warnings must not be prefixed audio.: {warnings:?}"
        );
    }

    #[test]
    fn app_table_parses_and_absence_is_default_without_warning() {
        let (_c, app, warnings) =
            parse("[app]\nstart_with_windows = true\nstart_minimized = true\n");
        assert!(app.start_with_windows);
        assert!(app.start_minimized);
        assert!(warnings.is_empty(), "{warnings:?}");

        // No [app] table → defaults, no warning.
        let (_c2, app2, warnings2) = parse("[audio.flash]\nvolume = 0.2\n");
        assert_eq!(app2, AppConfig::default());
        assert!(warnings2.is_empty(), "{warnings2:?}");
    }

    #[test]
    fn app_unknown_key_and_bad_type_warn_and_fall_back() {
        let (_c, app, warnings) = parse("[app]\nstart_with_windows = \"yes\"\nfoo = 1\n");
        assert!(!app.start_with_windows, "bad type keeps default false");
        assert_eq!(warnings.len(), 2, "{warnings:?}"); // bad type + unknown key
        assert!(warnings.iter().any(|w| w.contains("app.start_with_windows")));
        assert!(warnings.iter().any(|w| w.contains("app.foo")));
    }

    #[test]
    fn serialize_round_trips_defaults() {
        let c = Config::default();
        let a = AppConfig::default();
        let (c2, a2, warnings) = parse(&serialize(&c, &a));
        assert_eq!(c2, c);
        assert_eq!(a2, a);
        assert!(warnings.is_empty(), "{warnings:?}");
    }

    #[test]
    fn serialize_round_trips_non_default_values() {
        let mut c = Config::default();
        c.flash.volume = 0.05;
        c.death.enabled = false;
        c.bomb.duration_ms = 2000;
        let a = AppConfig {
            start_with_windows: true,
            start_minimized: true,
            overlay: OverlayConfig::default(),
        };
        let (c2, a2, warnings) = parse(&serialize(&c, &a));
        assert_eq!(c2, c);
        assert_eq!(a2, a);
        assert!(warnings.is_empty(), "{warnings:?}");
        // It must still be human-commented.
        assert!(serialize(&c, &a).contains('#'));
    }

    #[test]
    fn config_and_app_deserialize_from_ui_json() {
        // Mirrors the JSON the frontend sends to `save_config`.
        let cfg: Config = serde_json::from_str(
            r#"{"death":{"enabled":true,"volume":0.3,"duration_ms":1000},
                "flash":{"enabled":false,"volume":0.15},
                "bomb":{"enabled":true,"volume":0.3,"duration_ms":1500},
                "spectator":{"enabled":true,"volume":0.6}}"#,
        )
        .unwrap();
        assert!(!cfg.flash.enabled);
        assert_eq!(cfg.bomb.duration_ms, 1500);

        let app: AppConfig =
            serde_json::from_str(r#"{"start_with_windows":true,"start_minimized":false}"#).unwrap();
        assert!(app.start_with_windows);
        assert!(!app.start_minimized);
    }

    #[test]
    fn defaults_match_spec() {
        let c = Config::default();
        assert!(c.death.enabled);
        assert_eq!(c.death.volume, 0.30);
        assert_eq!(c.death.duration_ms, 1000);
        assert!(c.flash.enabled);
        assert_eq!(c.flash.volume, 0.15);
        assert!(c.bomb.enabled);
        assert_eq!(c.bomb.volume, 0.30);
        assert_eq!(c.bomb.duration_ms, 1500);
        assert!(c.spectator.enabled);
        assert_eq!(c.spectator.volume, 0.60);
    }

    /// The full config sample from docs/spec.md (minus the M5 hotkey table).
    const SPEC_SAMPLE: &str = r#"
[audio.death]
enabled = true
volume = 0.30
duration_ms = 1000

[audio.flash]
enabled = true
volume = 0.15

[audio.bomb]
enabled = true
volume = 0.30
duration_ms = 1500

[audio.spectator]
enabled = false
volume = 0.60
"#;

    #[test]
    fn parses_spec_sample_without_warnings() {
        let (c, _app, warnings) = parse(SPEC_SAMPLE);
        assert_eq!(warnings, Vec::<String>::new());
        assert!(!c.spectator.enabled); // differs from default — really parsed
        assert_eq!(c.bomb.duration_ms, 1500);
    }

    #[test]
    fn unknown_table_and_key_warn_and_are_ignored() {
        let (c, _app, warnings) = parse(
            "[blackflash]\nenabled = true\n\n[audio.death]\nvolume = 0.4\nfoo = 1\n\n[audio.hotkey]\nkey = \"F10\"\n",
        );
        assert_eq!(c.death.volume, 0.4); // known key still applied
        assert_eq!(warnings.len(), 3, "{warnings:?}"); // blackflash, foo, hotkey
        assert!(warnings.iter().any(|w| w.contains("blackflash")));
        assert!(warnings.iter().any(|w| w.contains("audio.death.foo")));
        assert!(warnings.iter().any(|w| w.contains("audio.hotkey")));
    }

    #[test]
    fn invalid_values_warn_and_fall_back_to_defaults() {
        let (c, _app, warnings) = parse(
            "[audio.flash]\nenabled = \"yes\"\nvolume = 1.5\n\n[audio.bomb]\nduration_ms = 0\n",
        );
        assert!(c.flash.enabled); // default kept
        assert_eq!(c.flash.volume, 0.15); // default kept (1.5 out of range)
        assert_eq!(c.bomb.duration_ms, 1500); // default kept (must be > 0)
        assert_eq!(warnings.len(), 3, "{warnings:?}");
    }

    #[test]
    fn integer_volume_in_range_is_accepted() {
        let (c, _app, warnings) = parse("[audio.flash]\nvolume = 1\n");
        assert_eq!(c.flash.volume, 1.0);
        assert!(warnings.is_empty(), "{warnings:?}");
    }

    #[test]
    fn unparseable_text_yields_defaults_plus_warning() {
        let (c, _app, warnings) = parse("this is { not toml");
        assert_eq!(c, Config::default());
        assert_eq!(warnings.len(), 1);
    }

    #[test]
    fn missing_file_writes_commented_defaults_and_reloads_cleanly() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(CONFIG_FILE_NAME);
        let (c, _app, warnings) = load_or_create(&path);
        assert_eq!(c, Config::default());
        assert!(warnings.is_empty(), "{warnings:?}");
        assert!(path.exists());
        // The generated file must round-trip without warnings.
        let (c2, _app2, warnings2) = load_or_create(&path);
        assert_eq!(c2, Config::default());
        assert!(warnings2.is_empty(), "{warnings2:?}");
        // And it should be commented for humans.
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains('#'));
    }

    #[test]
    fn reset_backs_up_existing_file_and_writes_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(CONFIG_FILE_NAME);
        std::fs::write(&path, "[audio.flash]\nvolume = 0.99\n").unwrap();
        reset(&path).unwrap();
        let backup = std::fs::read_to_string(dir.path().join("config.toml.bak")).unwrap();
        assert!(backup.contains("0.99"));
        let (c, _app, warnings) = load_or_create(&path);
        assert_eq!(c, Config::default());
        assert!(warnings.is_empty(), "{warnings:?}");
    }
}
