use std::fs;
use std::path::PathBuf;

/// Arbitrary high port, clear of common dev servers and the PatrikZeros tool
/// (3202).
pub const GSI_PORT: u16 = 31211;
pub const CFG_FILE_NAME: &str = "gamestate_integration_cs2helper.cfg";
pub const CS2_APP_ID: &str = "730";

/// `buffer 0.0` so flash events arrive with minimal delay; `throttle 0.1`
/// caps the POST rate between changes. `allplayers_id` is observer-only, so
/// its presence signals spectating, even free-roaming with no camera target
/// (which the `player` block does not cover). `heartbeat 10.0` (Valve's
/// default is 30.0) so heartbeat-only state recovers within ~10s.
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

/// vdf strings escape backslashes (`C:\\Steam`).
fn unescape_vdf(value: &str) -> String {
    value.replace("\\\\", "\\")
}

/// Every library `"path"` in libraryfolders.vdf, in file order. A minimal
/// line scan: trusted local data, not worth a full VDF parser.
pub fn parse_library_paths(vdf: &str) -> Vec<String> {
    vdf.lines()
        .filter_map(vdf_key_value)
        .filter(|(k, _)| *k == "path")
        .map(|(_, v)| unescape_vdf(v))
        .collect()
}

/// Path of the library whose `apps` block contains `app_id`. Each library
/// block lists its `path` before its `apps`, so the most recent path seen when
/// a line's key equals the appid is that app's library. Only keys are
/// compared, so size values cannot false-match.
pub fn find_library_with_app(vdf: &str, app_id: &str) -> Option<String> {
    let mut current_path: Option<String> = None;
    for line in vdf.lines() {
        let Some((key, value)) = vdf_key_value(line) else {
            continue;
        };
        if key == "path" {
            current_path = Some(unescape_vdf(value));
        } else if key == app_id {
            return current_path;
        }
    }
    None
}

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

fn cfg_dir_in_library(lib: &std::path::Path) -> PathBuf {
    lib.join("steamapps")
        .join("common")
        .join("Counter-Strike Global Offensive")
        .join("game")
        .join("csgo")
        .join("cfg")
}

/// Primary: the library whose `apps` block in libraryfolders.vdf lists CS2.
/// Fallback (vdf missing or reshaped): the Steam root or any library path
/// with CS2's app manifest on disk.
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

    let manifest = format!("appmanifest_{CS2_APP_ID}.acf");
    let mut libraries = vec![root.clone()];
    if let Some(vdf) = &vdf {
        libraries.extend(parse_library_paths(vdf).into_iter().map(PathBuf::from));
    }
    for lib in libraries {
        if lib.join("steamapps").join(&manifest).exists() {
            let cfg = cfg_dir_in_library(&lib);
            if cfg.is_dir() {
                return Some(cfg);
            }
        }
    }
    None
}

pub fn cfg_target_path() -> Option<PathBuf> {
    find_cs2_cfg_dir().map(|d| d.join(CFG_FILE_NAME))
}

/// CS2 loads GSI cfgs at startup, so the user must (re)start CS2 afterwards.
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

    /// Trimmed real libraryfolders.vdf: CS2 (730) is in the first of two
    /// libraries.
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
        // The size value of appid 570 in library 0.
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
