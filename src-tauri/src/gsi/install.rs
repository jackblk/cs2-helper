//! Locate the CS2 cfg directory and install our GSI cfg file into it.

use std::fs;
use std::path::PathBuf;

/// Fixed local port CS2 POSTs to. Arbitrary high port to avoid clashes with
/// common dev servers and the PatrikZeros tool (3202).
pub const GSI_PORT: u16 = 31211;
pub const CFG_FILE_NAME: &str = "gamestate_integration_cs2helper.cfg";
/// CS2's Steam appid.
pub const CS2_APP_ID: &str = "730";

/// Render the GSI cfg CS2 reads at startup. `buffer 0.0` so flash events
/// arrive with minimal delay; `throttle 0.1` caps POST rate between changes.
///
/// `allplayers_id` is an observer-only component (it returns data only while
/// spectating/observing), so its mere presence is a reliable "you are now
/// observing" signal — it appears the instant you become a spectator, even
/// while free-roaming with no locked camera target, which the local `player`
/// block does not cover. `heartbeat 10.0` (down from Valve's 30.0 default) is a
/// safety net so any state that only refreshes on heartbeat recovers within
/// ~10s instead of ~30s.
pub fn render_cfg(token: &str, port: u16) -> String {
    format!(
        r#""CS2 Helper"
{{
    "uri" "http://127.0.0.1:{port}"
    "timeout" "5.0"
    "buffer" "0.0"
    "throttle" "0.1"
    "heartbeat" "10.0"
    "auth"
    {{
        "token" "{token}"
    }}
    "data"
    {{
        "provider" "1"
        "map" "1"
        "round" "1"
        "player_id" "1"
        "player_state" "1"
        "allplayers_id" "1"
    }}
}}
"#
    )
}

/// First and second quoted strings on a vdf line, e.g.
/// `"path"  "C:\\Steam"` -> `Some(("path", "C:\\Steam"))`.
fn vdf_key_value(line: &str) -> Option<(&str, &str)> {
    let mut quoted = line.split('"').skip(1).step_by(2); // contents of each "..."
    Some((quoted.next()?, quoted.next()?))
}

/// Extract every library `"path"` from libraryfolders.vdf, in file order.
/// Minimal line scan — trusted local data, not worth a full VDF parser.
pub fn parse_library_paths(vdf: &str) -> Vec<String> {
    vdf.lines()
        .filter_map(vdf_key_value)
        .filter(|(k, _)| *k == "path")
        .map(|(_, v)| v.replace("\\\\", "\\"))
        .collect()
}

/// Path of the library whose `apps` block contains `app_id`.
///
/// Line scan that tracks the most recent `"path"` seen: each library block
/// lists its `path` before its `apps`, so when we hit a line whose KEY (first
/// quoted string) equals the appid, the current path is that app's library.
/// Size values (second quoted string) can't false-match because only keys are
/// compared.
pub fn find_library_with_app(vdf: &str, app_id: &str) -> Option<String> {
    let mut current_path: Option<String> = None;
    for line in vdf.lines() {
        let Some((key, value)) = vdf_key_value(line) else {
            continue;
        };
        if key == "path" {
            current_path = Some(value.replace("\\\\", "\\"));
        } else if key == app_id {
            return current_path;
        }
    }
    None
}

/// Steam install root from the registry (HKCU\Software\Valve\Steam).
#[cfg(windows)]
pub fn steam_root() -> Option<PathBuf> {
    let hkcu = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
    let key = hkcu.open_subkey("Software\\Valve\\Steam").ok()?;
    let path: String = key.get_value("SteamPath").ok()?;
    Some(PathBuf::from(path))
}

#[cfg(not(windows))]
pub fn steam_root() -> Option<PathBuf> {
    None
}

/// cfg dir inside a Steam library root.
fn cfg_dir_in_library(lib: &std::path::Path) -> PathBuf {
    lib.join("steamapps")
        .join("common")
        .join("Counter-Strike Global Offensive")
        .join("game")
        .join("csgo")
        .join("cfg")
}

/// Find `<library>/steamapps/common/Counter-Strike Global Offensive/game/csgo/cfg`.
///
/// Primary: the library whose `apps` block in libraryfolders.vdf lists CS2
/// (appid 730). Fallback (vdf missing/reshaped): scan the Steam root plus all
/// library paths for `steamapps/appmanifest_730.acf` on disk.
pub fn find_cs2_cfg_dir() -> Option<PathBuf> {
    let root = steam_root()?;
    let vdf = fs::read_to_string(root.join("steamapps").join("libraryfolders.vdf")).ok();

    if let Some(vdf) = &vdf {
        if let Some(lib) = find_library_with_app(vdf, CS2_APP_ID) {
            let cfg = cfg_dir_in_library(&PathBuf::from(lib));
            if cfg.is_dir() {
                return Some(cfg);
            }
        }
    }

    // Fallback: look for the app manifest on disk.
    let mut libraries = vec![root.clone()];
    if let Some(vdf) = &vdf {
        libraries.extend(parse_library_paths(vdf).into_iter().map(PathBuf::from));
    }
    for lib in libraries {
        if lib.join("steamapps").join("appmanifest_730.acf").exists() {
            let cfg = cfg_dir_in_library(&lib);
            if cfg.is_dir() {
                return Some(cfg);
            }
        }
    }
    None
}

/// Absolute path our cfg file would have (whether or not it exists yet).
pub fn cfg_target_path() -> Option<PathBuf> {
    find_cs2_cfg_dir().map(|d| d.join(CFG_FILE_NAME))
}

/// Write the cfg file into the CS2 cfg directory. Returns the written path.
/// CS2 loads GSI cfgs at game startup, so the user must (re)start CS2 after.
pub fn install_cfg(token: &str) -> Result<PathBuf, String> {
    let path = cfg_target_path()
        .ok_or("CS2 installation not found (is Steam + CS2 installed?)")?;
    fs::write(&path, render_cfg(token, GSI_PORT))
        .map_err(|e| format!("write {}: {e}", path.display()))?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Trimmed copy of a real libraryfolders.vdf: two libraries, CS2 (730)
    /// installed in the FIRST one — the right answer is C:\Data\Games\Steam,
    /// not the second library.
    const SAMPLE_VDF: &str = r#"
"libraryfolders"
{
    "0"
    {
        "path"        "C:\\Data\\Games\\Steam"
        "label"        ""
        "contentid"        "3004782507896185515"
        "totalsize"        "0"
        "apps"
        {
            "570"        "72052285161"
            "730"        "68507630986"
            "223850"        "4607533247"
        }
    }
    "1"
    {
        "path"        "D:\\G\\SteamLibrary"
        "label"        ""
        "contentid"        "1513887045211731446"
        "totalsize"        "1000203816960"
        "apps"
        {
            "620"        "12754915600"
            "1222140"        "63077578235"
        }
    }
}
"#;

    #[test]
    fn cfg_contains_uri_token_and_subscriptions() {
        let cfg = render_cfg("SECRET123", 31211);
        assert!(cfg.contains(r#""uri" "http://127.0.0.1:31211""#));
        assert!(cfg.contains(r#""token" "SECRET123""#));
        // allplayers_id is the observer-only spectator signal (see render_cfg).
        for component in [
            "provider",
            "map",
            "round",
            "player_id",
            "player_state",
            "allplayers_id",
        ] {
            assert!(
                cfg.contains(&format!(r#""{component}" "1""#)),
                "missing subscription: {component}"
            );
        }
        // Heartbeat trimmed from Valve's 30s default so heartbeat-only state
        // (e.g. spectator with no locked target) recovers within ~10s.
        assert!(cfg.contains(r#""heartbeat" "10.0""#), "heartbeat not trimmed");
    }

    #[test]
    fn finds_library_containing_cs2() {
        assert_eq!(
            find_library_with_app(SAMPLE_VDF, "730"),
            Some("C:\\Data\\Games\\Steam".to_string())
        );
    }

    #[test]
    fn finds_app_in_second_library() {
        assert_eq!(
            find_library_with_app(SAMPLE_VDF, "1222140"),
            Some("D:\\G\\SteamLibrary".to_string())
        );
    }

    #[test]
    fn app_size_values_do_not_false_match() {
        // "72052285161" is the SIZE value of appid 570 in library 0; a number
        // appearing in the size column (second token) must not count as a match.
        assert_eq!(find_library_with_app(SAMPLE_VDF, "72052285161"), None);
    }

    #[test]
    fn missing_app_yields_none() {
        assert_eq!(find_library_with_app(SAMPLE_VDF, "999999"), None);
        assert_eq!(find_library_with_app("\"libraryfolders\"\n{\n}\n", "730"), None);
    }

    #[test]
    fn collects_all_library_paths() {
        assert_eq!(
            parse_library_paths(SAMPLE_VDF),
            vec![
                "C:\\Data\\Games\\Steam".to_string(),
                "D:\\G\\SteamLibrary".to_string()
            ]
        );
    }
}
