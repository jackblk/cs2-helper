//! TOML config. Parsing never hard-errors: unknown tables/keys warn and are
//! ignored, invalid values warn and keep that key's default, and an
//! unreadable file falls back to all defaults.

use std::fs;
use std::path::Path;
use std::sync::LazyLock;

use serde::{Deserialize, Serialize};

pub const CONFIG_FILE_NAME: &str = "config.toml";

/// Single source of truth for default values (the `Default` impls read it),
/// comments and layout (`serialize` fills it in with current values).
pub const DEFAULT_TOML: &str = include_str!("default.toml");

#[derive(Deserialize)]
struct DefaultFile {
    audio: Config,
    app: AppFlags,
    overlay: OverlayConfig,
    hotkeys: HotkeysConfig,
}

#[derive(Deserialize)]
struct AppFlags {
    start_with_windows: bool,
    start_minimized: bool,
    language: String,
}

static DEFAULTS: LazyLock<DefaultFile> = LazyLock::new(|| {
    toml::from_str(DEFAULT_TOML).expect("built-in default.toml must be valid")
});

/// `volume` is an absolute target in 0.0..=1.0.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EventCfg {
    pub enabled: bool,
    pub volume: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TimedEventCfg {
    pub enabled: bool,
    pub volume: f32,
    pub duration_ms: u64,
}

/// Volume held while Deathmatch mode is on. The on/off state is runtime only
/// (the engine's), so there is no `enabled`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DeathmatchCfg {
    pub volume: f32,
}

impl Default for DeathmatchCfg {
    fn default() -> Self {
        DEFAULTS.audio.deathmatch
    }
}

/// `pos_*` are logical pixels (`None`: default bottom-center placement);
/// `scale` multiplies the base size.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct OverlayConfig {
    pub enabled: bool,
    pub bomb_timer: bool,
    pub volume_status: bool,
    pub c4_timer_s: f32,
    pub safety_margin_s: f32,
    pub pos_x: Option<f64>,
    pub pos_y: Option<f64>,
    pub scale: f32,
}

impl Default for OverlayConfig {
    fn default() -> Self {
        DEFAULTS.overlay
    }
}

/// `key` uses the global-shortcut plugin's syntax, e.g. "F10" or "Ctrl+Shift+P".
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HotkeyCfg {
    pub enabled: bool,
    pub key: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HotkeysConfig {
    pub pause: HotkeyCfg,
    #[serde(default = "default_deathmatch_hotkey")]
    pub deathmatch: HotkeyCfg,
}

fn default_deathmatch_hotkey() -> HotkeyCfg {
    DEFAULTS.hotkeys.deathmatch.clone()
}

impl Default for HotkeysConfig {
    fn default() -> Self {
        DEFAULTS.hotkeys.clone()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Config {
    pub death: TimedEventCfg,
    pub flash: EventCfg,
    pub bomb: TimedEventCfg,
    pub spectator: EventCfg,
    // `serde(default)`: UI JSON may omit it.
    #[serde(default)]
    pub deathmatch: DeathmatchCfg,
}

impl Default for Config {
    fn default() -> Self {
        DEFAULTS.audio
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppConfig {
    /// Source of truth for autostart, implemented as a Startup-folder
    /// shortcut (not the registry).
    pub start_with_windows: bool,
    pub start_minimized: bool,
    /// One of `i18n::LANGUAGES`.
    #[serde(default = "default_language")]
    pub language: String,
    // `serde(default)`: UI JSON and older files may omit these tables.
    #[serde(default)]
    pub overlay: OverlayConfig,
    #[serde(default)]
    pub hotkeys: HotkeysConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            start_with_windows: DEFAULTS.app.start_with_windows,
            start_minimized: DEFAULTS.app.start_minimized,
            language: default_language(),
            overlay: OverlayConfig::default(),
            hotkeys: HotkeysConfig::default(),
        }
    }
}

fn default_language() -> String {
    DEFAULTS.app.language.clone()
}

fn default_doc() -> toml_edit::DocumentMut {
    DEFAULT_TOML
        .parse()
        .expect("built-in default.toml must be valid")
}

/// f32 → TOML float via its shortest decimal form, so 0.3f32 is written as
/// `0.3`, not `0.30000001192092896`.
fn float(v: f32) -> toml_edit::Item {
    toml_edit::value(v.to_string().parse::<f64>().unwrap_or(v as f64))
}

/// `default.toml` with each value replaced, comments kept. Round-trips:
/// `parse(serialize(c, a))` yields `(c, a, [])`.
pub fn serialize(config: &Config, app: &AppConfig) -> String {
    use toml_edit::value;
    let mut doc = default_doc();

    let audio = &mut doc["audio"];
    for (name, ev) in [("death", &config.death), ("bomb", &config.bomb)] {
        audio[name]["enabled"] = value(ev.enabled);
        audio[name]["volume"] = float(ev.volume);
        audio[name]["duration_ms"] = value(ev.duration_ms as i64);
    }
    for (name, ev) in [("flash", &config.flash), ("spectator", &config.spectator)] {
        audio[name]["enabled"] = value(ev.enabled);
        audio[name]["volume"] = float(ev.volume);
    }
    audio["deathmatch"]["volume"] = float(config.deathmatch.volume);

    doc["app"]["start_with_windows"] = value(app.start_with_windows);
    doc["app"]["start_minimized"] = value(app.start_minimized);
    doc["app"]["language"] = value(app.language.as_str());

    let o = &app.overlay;
    let overlay = &mut doc["overlay"];
    overlay["enabled"] = value(o.enabled);
    overlay["bomb_timer"] = value(o.bomb_timer);
    overlay["volume_status"] = value(o.volume_status);
    overlay["c4_timer_s"] = float(o.c4_timer_s);
    overlay["safety_margin_s"] = float(o.safety_margin_s);
    overlay["scale"] = float(o.scale);
    if let (Some(x), Some(y)) = (o.pos_x, o.pos_y) {
        overlay["pos_x"] = value(x);
        overlay["pos_y"] = value(y);
    }

    let pause = &mut doc["hotkeys"]["pause"];
    pause["enabled"] = value(app.hotkeys.pause.enabled);
    pause["key"] = value(app.hotkeys.pause.key.as_str());
    let deathmatch = &mut doc["hotkeys"]["deathmatch"];
    deathmatch["enabled"] = value(app.hotkeys.deathmatch.enabled);
    deathmatch["key"] = value(app.hotkeys.deathmatch.key.as_str());

    doc.to_string()
}

pub fn default_file_contents() -> String {
    DEFAULT_TOML.to_string()
}

/// Add every table/key of `default.toml` missing from `text` (e.g. settings
/// added after the file was written), with default values and comments. New
/// tables go after their existing siblings (or at the end), new keys at the
/// end of their table; existing content is never modified. Returns `None`
/// when nothing is missing or `text` is not valid TOML (`parse` reports it; a
/// broken file is never rewritten).
pub fn add_missing(text: &str) -> Option<String> {
    let mut doc: toml_edit::DocumentMut = text.parse().ok()?;
    let added = merge_missing(doc.as_table_mut(), default_doc().as_table());
    added.then(|| doc.to_string())
}

fn merge_missing(dst: &mut toml_edit::Table, src: &toml_edit::Table) -> bool {
    let mut added = false;
    for (key, item) in src.iter() {
        match (dst.get_mut(key), item) {
            (Some(toml_edit::Item::Table(d)), toml_edit::Item::Table(s)) => {
                added |= merge_missing(d, s);
            }
            (Some(_), _) => {}
            (None, _) => {
                let (k, _) = src.get_key_value(key).expect("key from iter");
                dst.insert_formatted(k, fresh_copy(item));
                added = true;
            }
        }
    }
    added
}

/// Copy an item from the default doc, rebuilding tables so they carry no
/// document position from `default.toml` (positionless tables render right
/// after the preceding existing table).
fn fresh_copy(item: &toml_edit::Item) -> toml_edit::Item {
    let toml_edit::Item::Table(src) = item else {
        return item.clone();
    };
    let mut t = toml_edit::Table::new();
    t.set_implicit(src.is_implicit());
    *t.decor_mut() = src.decor().clone();
    // toml_edit attaches the file's header comment (up to the first blank
    // line, LF endings) to the first table; don't copy it along with it.
    let header = DEFAULT_TOML.split_inclusive("\n\n").next().unwrap_or("");
    if let Some(prefix) = src.decor().prefix().and_then(|p| p.as_str()) {
        if let Some(rest) = prefix.strip_prefix(header) {
            t.decor_mut().set_prefix(format!("\n{rest}"));
        }
    }
    for (key, child) in src.iter() {
        let (k, _) = src.get_key_value(key).expect("key from iter");
        t.insert_formatted(k, fresh_copy(child));
    }
    toml_edit::Item::Table(t)
}

pub fn write(path: &Path, config: &Config, app: &AppConfig) -> Result<(), String> {
    fs::write(path, serialize(config, app))
        .map_err(|e| format!("write {}: {e}", path.display()))
}

/// Creates the default file on first run and adds missing settings to older
/// files. Never fails: any problem falls back to defaults plus a warning.
pub fn load_or_create(path: &Path) -> (Config, AppConfig, Vec<String>) {
    match fs::read_to_string(path) {
        Ok(text) => match add_missing(&text) {
            Some(patched) => {
                let (c, a, mut warnings) = parse(&patched);
                if let Err(e) = fs::write(path, &patched) {
                    warnings.push(format!(
                        "could not add missing settings to {}: {e}",
                        path.display()
                    ));
                }
                (c, a, warnings)
            }
            None => parse(&text),
        },
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

pub fn reset(path: &Path) -> Result<(), String> {
    if path.exists() {
        let backup = path.with_extension("toml.bak");
        fs::copy(path, &backup).map_err(|e| format!("backup {}: {e}", backup.display()))?;
    }
    fs::write(path, default_file_contents()).map_err(|e| format!("write {}: {e}", path.display()))
}

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
                        "deathmatch" => {
                            apply_deathmatch(table, &mut cfg.deathmatch, &mut warnings)
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
            "hotkeys" => {
                let Some(hotkeys) = value.as_table() else {
                    warnings.push("\"hotkeys\" is not a table; ignored".to_string());
                    continue;
                };
                for (action, value) in hotkeys {
                    let Some(table) = value.as_table() else {
                        warnings.push(format!("[hotkeys.{action}] is not a table; ignored"));
                        continue;
                    };
                    match action.as_str() {
                        "pause" => {
                            apply_hotkey(table, "pause", &mut app.hotkeys.pause, &mut warnings)
                        }
                        "deathmatch" => apply_hotkey(
                            table,
                            "deathmatch",
                            &mut app.hotkeys.deathmatch,
                            &mut warnings,
                        ),
                        other => {
                            warnings.push(format!("unknown table [hotkeys.{other}] ignored"))
                        }
                    }
                }
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
            "language" => match value.as_str() {
                Some(code) if crate::i18n::LANGUAGES.contains(&code) => {
                    out.language = code.to_string()
                }
                _ => warnings.push(format!(
                    "app.language: expected one of {:?}, got {value}; keeping \"{}\"",
                    crate::i18n::LANGUAGES,
                    out.language
                )),
            },
            _ => warnings.push(format!("unknown key app.{key} ignored")),
        }
    }
}

fn apply_overlay(table: &toml::Table, out: &mut OverlayConfig, warnings: &mut Vec<String>) {
    for (key, value) in table {
        match key.as_str() {
            "enabled" => set_bool(value, "overlay", key, &mut out.enabled, warnings),
            "bomb_timer" => set_bool(value, "overlay", key, &mut out.bomb_timer, warnings),
            "volume_status" => set_bool(value, "overlay", key, &mut out.volume_status, warnings),
            "c4_timer_s" => set_pos_float(value, key, &mut out.c4_timer_s, warnings),
            "safety_margin_s" => set_nonneg_float(value, key, &mut out.safety_margin_s, warnings),
            "scale" => set_scale(value, key, &mut out.scale, warnings),
            "pos_x" => set_opt_float(value, key, &mut out.pos_x, warnings),
            "pos_y" => set_opt_float(value, key, &mut out.pos_y, warnings),
            _ => warnings.push(format!("unknown key overlay.{key} ignored")),
        }
    }
}

fn apply_hotkey(table: &toml::Table, name: &str, out: &mut HotkeyCfg, warnings: &mut Vec<String>) {
    let prefix = format!("hotkeys.{name}");
    for (key, value) in table {
        match key.as_str() {
            "enabled" => set_bool(value, &prefix, key, &mut out.enabled, warnings),
            "key" => match value.as_str() {
                Some(s) if is_valid_shortcut(s) => out.key = s.to_string(),
                _ => warnings.push(format!(
                    "{prefix}.key: expected a shortcut like \"F10\" or \"Ctrl+Shift+P\", got {value}; keeping \"{}\"",
                    out.key
                )),
            },
            _ => warnings.push(format!("unknown key {prefix}.{key} ignored")),
        }
    }
}

pub fn is_valid_shortcut(s: &str) -> bool {
    s.parse::<tauri_plugin_global_shortcut::Shortcut>().is_ok()
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

fn apply_deathmatch(table: &toml::Table, out: &mut DeathmatchCfg, warnings: &mut Vec<String>) {
    for (key, value) in table {
        match key.as_str() {
            "volume" => set_volume(value, "deathmatch", key, &mut out.volume, warnings),
            _ => warnings.push(format!("unknown key audio.deathmatch.{key} ignored")),
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
        assert!(o.bomb_timer);
        assert!(o.volume_status);
        assert_eq!(o.c4_timer_s, 40.0);
        assert_eq!(o.safety_margin_s, 0.0);
        assert_eq!(o.scale, 1.0);
        assert_eq!(o.pos_x, None);
        assert_eq!(o.pos_y, None);

        let mut a = AppConfig::default();
        a.overlay.enabled = false;
        a.overlay.bomb_timer = false;
        a.overlay.volume_status = false;
        a.overlay.scale = 1.5;
        a.overlay.pos_x = Some(100.0);
        a.overlay.pos_y = Some(40.0);
        let (_c, a2, warnings) = parse(&serialize(&Config::default(), &a));
        assert_eq!(a2.overlay, a.overlay);
        assert!(warnings.is_empty(), "{warnings:?}");
    }

    #[test]
    fn overlay_without_bomb_timer_key_keeps_it_on() {
        let (_c, a, warnings) = parse("[overlay]\nenabled = true\n");
        assert!(a.overlay.bomb_timer);
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
    fn default_file_parses_cleanly_and_is_the_default() {
        let (c, a, warnings) = parse(DEFAULT_TOML);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(c, Config::default());
        assert_eq!(a, AppConfig::default());
        let (c2, a2, w2) = parse(&serialize(&c, &a));
        assert_eq!((c2, a2), (c, a));
        assert!(w2.is_empty(), "{w2:?}");
    }

    #[test]
    fn pause_hotkey_defaults_to_f10_and_parses() {
        let h = HotkeysConfig::default().pause;
        assert!(h.enabled);
        assert_eq!(h.key, "F10");
        assert!(is_valid_shortcut(&h.key));
        // Strings the UI recorder produces.
        for s in ["Ctrl+Shift+P", "Alt+1", "Ctrl+Numpad5", "Shift+Space", "Super+F12"] {
            assert!(is_valid_shortcut(s), "{s} should parse");
        }

        let (_c, a, warnings) =
            parse("[hotkeys.pause]\nenabled = false\nkey = \"Ctrl+Alt+K\"\n");
        assert!(!a.hotkeys.pause.enabled);
        assert_eq!(a.hotkeys.pause.key, "Ctrl+Alt+K");
        assert!(warnings.is_empty(), "{warnings:?}");
    }

    #[test]
    fn hotkey_invalid_key_and_unknown_fields_warn() {
        let (_c, a, warnings) = parse(
            "[hotkeys.pause]\nkey = \"NotAKey\"\nfoo = 1\n\n[hotkeys.bogus]\nkey = \"F9\"\n",
        );
        assert_eq!(a.hotkeys.pause.key, "F10", "invalid key keeps default");
        assert_eq!(warnings.len(), 3, "{warnings:?}");
        assert!(warnings.iter().any(|w| w.contains("hotkeys.pause.key")));
        assert!(warnings.iter().any(|w| w.contains("hotkeys.pause.foo")));
        assert!(warnings.iter().any(|w| w.contains("hotkeys.bogus")));
    }

    #[test]
    fn add_missing_appends_hotkeys_to_a_pre_hotkeys_file() {
        let old = DEFAULT_TOML
            .split("\n# Global hotkeys")
            .next()
            .unwrap();
        assert!(!old.contains("hotkeys"));
        let patched = add_missing(old).expect("hotkeys were missing");
        assert_eq!(patched, DEFAULT_TOML, "appended exactly, comments included");
    }

    #[test]
    fn add_missing_fills_gaps_and_keeps_existing_text() {
        let old = "# my notes\n[audio.flash]\nvolume = 0.2\n\n[overlay]\nenabled = false\n";
        let patched = add_missing(old).expect("something was missing");
        let mut rest = patched.as_str();
        for line in old.lines().filter(|l| !l.is_empty()) {
            let at = rest.find(line).unwrap_or_else(|| panic!("lost {line:?}:\n{patched}"));
            rest = &rest[at + line.len()..];
        }
        assert!(!patched.contains("CS2 Helper configuration"), "no header copied:\n{patched}");
        assert!(patched.contains("[hotkeys.pause]"), "{patched}");
        assert!(patched.contains("[audio.death]"), "{patched}");
        assert!(patched.contains("Toggle pause"), "comments come along:\n{patched}");

        let (c, a, warnings) = parse(&patched);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(c.flash.volume, 0.2, "user value kept");
        assert!(!a.overlay.enabled, "user value kept");
        assert_eq!(a.overlay.scale, 1.0, "missing key added with default");
        assert_eq!(a.hotkeys.pause.key, "F10");

        assert_eq!(add_missing(&patched), None);
        assert_eq!(add_missing(DEFAULT_TOML), None);
        assert_eq!(add_missing("this is { not toml"), None);
    }

    #[test]
    fn load_or_create_adds_missing_sections_to_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(CONFIG_FILE_NAME);
        std::fs::write(&path, "[audio.flash]\nvolume = 0.2\n").unwrap();
        let (c, a, warnings) = load_or_create(&path);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(c.flash.volume, 0.2);
        assert_eq!(a.hotkeys.pause.key, "F10");
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.starts_with("[audio.flash]\nvolume = 0.2\n"), "{text}");
        assert!(text.contains("[hotkeys.pause]"), "{text}");
    }

    #[test]
    fn app_table_parses_and_absence_is_default_without_warning() {
        let (_c, app, warnings) =
            parse("[app]\nstart_with_windows = true\nstart_minimized = true\n");
        assert!(app.start_with_windows);
        assert!(app.start_minimized);
        assert!(warnings.is_empty(), "{warnings:?}");

        let (_c2, app2, warnings2) = parse("[audio.flash]\nvolume = 0.2\n");
        assert_eq!(app2, AppConfig::default());
        assert!(warnings2.is_empty(), "{warnings2:?}");
    }

    #[test]
    fn language_defaults_to_en_and_rejects_unknown_codes() {
        assert_eq!(AppConfig::default().language, "en");
        let (_c, app, warnings) = parse("[app]\nlanguage = \"vi\"\n");
        assert_eq!(app.language, "vi");
        assert!(warnings.is_empty(), "{warnings:?}");

        let (_c, app, warnings) = parse("[app]\nlanguage = \"xx\"\n");
        assert_eq!(app.language, "en", "unknown code keeps the default");
        assert_eq!(warnings.len(), 1, "{warnings:?}");
        assert!(warnings[0].contains("app.language"), "{warnings:?}");
    }

    #[test]
    fn app_unknown_key_and_bad_type_warn_and_fall_back() {
        let (_c, app, warnings) = parse("[app]\nstart_with_windows = \"yes\"\nfoo = 1\n");
        assert!(!app.start_with_windows, "bad type keeps default false");
        assert_eq!(warnings.len(), 2, "{warnings:?}");
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
        c.deathmatch.volume = 0.1;
        let a = AppConfig {
            start_with_windows: true,
            start_minimized: true,
            language: "en".to_string(),
            overlay: OverlayConfig::default(),
            hotkeys: HotkeysConfig {
                pause: HotkeyCfg {
                    enabled: false,
                    key: "Ctrl+Shift+F9".to_string(),
                },
                deathmatch: HotkeyCfg {
                    enabled: false,
                    key: "Ctrl+F8".to_string(),
                },
            },
        };
        let (c2, a2, warnings) = parse(&serialize(&c, &a));
        assert_eq!(c2, c);
        assert_eq!(a2, a);
        assert!(warnings.is_empty(), "{warnings:?}");
        // Comments survive serialization.
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
        assert_eq!(c.deathmatch.volume, 0.0);
        let h = HotkeysConfig::default();
        assert!(h.deathmatch.enabled);
        assert_eq!(h.deathmatch.key, "F8");
    }

    const AUDIO_SAMPLE: &str = r#"
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
    fn parses_audio_sample_without_warnings() {
        let (c, _app, warnings) = parse(AUDIO_SAMPLE);
        assert_eq!(warnings, Vec::<String>::new());
        assert!(!c.spectator.enabled); // non-default, so it was really parsed
        assert_eq!(c.bomb.duration_ms, 1500);
    }

    #[test]
    fn unknown_table_and_key_warn_and_are_ignored() {
        let (c, _app, warnings) = parse(
            "[blackflash]\nenabled = true\n\n[audio.death]\nvolume = 0.4\nfoo = 1\n\n[audio.hotkey]\nkey = \"F10\"\n",
        );
        assert_eq!(c.death.volume, 0.4);
        assert_eq!(warnings.len(), 3, "{warnings:?}");
        assert!(warnings.iter().any(|w| w.contains("blackflash")));
        assert!(warnings.iter().any(|w| w.contains("audio.death.foo")));
        assert!(warnings.iter().any(|w| w.contains("audio.hotkey")));
    }

    #[test]
    fn invalid_values_warn_and_fall_back_to_defaults() {
        let (c, _app, warnings) = parse(
            "[audio.flash]\nenabled = \"yes\"\nvolume = 1.5\n\n[audio.bomb]\nduration_ms = 0\n",
        );
        assert!(c.flash.enabled);
        assert_eq!(c.flash.volume, 0.15);
        assert_eq!(c.bomb.duration_ms, 1500);
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
        let (c2, _app2, warnings2) = load_or_create(&path);
        assert_eq!(c2, Config::default());
        assert!(warnings2.is_empty(), "{warnings2:?}");
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
