//! Every payload field is `Option`: Valve adds and omits fields freely, and a
//! parse failure must never take the server down. Only fields something reads
//! are modeled; serde ignores the rest.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Deserialize)]
pub struct GsiPayload {
    pub auth: Option<Auth>,
    pub provider: Option<Provider>,
    pub map: Option<Map>,
    pub round: Option<Round>,
    pub player: Option<Player>,
    /// Observer-only (subscribed via `allplayers_id`): CS2 sends it only while
    /// spectating, so its presence drives `GameState::observing`. Contents are
    /// unused, hence untyped.
    pub allplayers: Option<serde_json::Map<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Auth {
    pub token: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Provider {
    pub steamid: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Map {
    pub name: Option<String>,
    pub phase: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Round {
    pub phase: Option<String>,
    pub bomb: Option<String>,
    pub win_team: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Player {
    pub steamid: Option<String>,
    pub name: Option<String>,
    pub team: Option<String>,
    pub activity: Option<String>,
    pub state: Option<PlayerState>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct PlayerState {
    pub health: Option<i32>,
    pub armor: Option<i32>,
    pub helmet: Option<bool>,
    pub flashed: Option<i32>,
    pub defusekit: Option<bool>,
}

/// GSI omits unchanged sections, so `apply` only overwrites what the payload
/// carries, except for the non-sticky fields that mirror the latest payload.
#[derive(Debug, Clone, Default, Serialize)]
pub struct GameState {
    pub provider_steamid: Option<String>,
    pub player_steamid: Option<String>,
    /// `player.steamid == provider.steamid`: the `player` block is us, not a
    /// spectated player.
    pub is_local_player: bool,
    /// Presence of `allplayers`. Unlike `is_local_player` this also holds when
    /// free-roaming with no camera target (no `player` block). Non-sticky.
    pub observing: bool,
    pub player_name: Option<String>,
    pub activity: Option<String>,
    pub map_name: Option<String>,
    pub map_phase: Option<String>,
    pub round_phase: Option<String>,
    pub bomb: Option<String>,
    pub win_team: Option<String>,
    pub health: Option<i32>,
    pub armor: Option<i32>,
    pub helmet: Option<bool>,
    pub flashed: Option<i32>,
    /// "CT" or "T", for the local or spectated player.
    pub team: Option<String>,
    /// Non-sticky: CS2 omits `defusekit` when there is no kit (it never sends
    /// `false`).
    pub defusekit: Option<bool>,
}

impl GameState {
    pub fn apply(&mut self, p: &GsiPayload) {
        if let Some(provider) = &p.provider {
            if provider.steamid.is_some() {
                self.provider_steamid = provider.steamid.clone();
            }
        }
        if let Some(map) = &p.map {
            if map.name.is_some() {
                self.map_name = map.name.clone();
            }
            if map.phase.is_some() {
                self.map_phase = map.phase.clone();
            }
        }
        if let Some(round) = &p.round {
            if round.phase.is_some() {
                self.round_phase = round.phase.clone();
            }
            // Non-sticky: `bomb` and `win_team` disappear between rounds.
            self.bomb = round.bomb.clone();
            self.win_team = round.win_team.clone();
        }
        if let Some(player) = &p.player {
            if player.steamid.is_some() {
                self.player_steamid = player.steamid.clone();
            }
            if player.name.is_some() {
                self.player_name = player.name.clone();
            }
            if player.activity.is_some() {
                self.activity = player.activity.clone();
            }
            if player.team.is_some() {
                self.team = player.team.clone();
            }
            if let Some(state) = &player.state {
                if state.health.is_some() {
                    self.health = state.health;
                }
                if state.armor.is_some() {
                    self.armor = state.armor;
                }
                if state.helmet.is_some() {
                    self.helmet = state.helmet;
                }
                if state.flashed.is_some() {
                    self.flashed = state.flashed;
                }
                self.defusekit = state.defusekit;
            }
        }
        if let (Some(a), Some(b)) = (&self.provider_steamid, &self.player_steamid) {
            self.is_local_player = a == b;
        }
        self.observing = p.allplayers.as_ref().is_some_and(|m| !m.is_empty());
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    pub const SAMPLE_PAYLOAD: &str = r#"{
        "provider": {
            "name": "Counter-Strike: Global Offensive",
            "appid": 730,
            "version": 14023,
            "steamid": "76561198000000001",
            "timestamp": 1750000000
        },
        "map": {
            "mode": "competitive",
            "name": "de_dust2",
            "phase": "live",
            "round": 12
        },
        "round": { "phase": "live", "bomb": "planted" },
        "player": {
            "steamid": "76561198000000001",
            "name": "TestPlayer",
            "activity": "playing",
            "state": {
                "health": 87,
                "armor": 50,
                "helmet": true,
                "flashed": 0,
                "smoked": 0,
                "burning": 0,
                "money": 3000,
                "round_kills": 1,
                "round_killhs": 0,
                "equip_value": 4700
            }
        },
        "auth": { "token": "testtoken" }
    }"#;

    #[test]
    fn parses_full_payload() {
        let p: GsiPayload = serde_json::from_str(SAMPLE_PAYLOAD).unwrap();
        assert_eq!(p.auth.unwrap().token.as_deref(), Some("testtoken"));
        assert_eq!(
            p.provider.unwrap().steamid.as_deref(),
            Some("76561198000000001")
        );
        assert_eq!(p.map.as_ref().unwrap().phase.as_deref(), Some("live"));
        assert_eq!(p.round.as_ref().unwrap().bomb.as_deref(), Some("planted"));
        let player = p.player.unwrap();
        assert_eq!(player.activity.as_deref(), Some("playing"));
        let state = player.state.unwrap();
        assert_eq!(state.health, Some(87));
        assert_eq!(state.flashed, Some(0));
        assert_eq!(state.helmet, Some(true));
    }

    #[test]
    fn parses_partial_payload_missing_sections() {
        // Heartbeat-style POST: only provider + auth.
        let p: GsiPayload = serde_json::from_str(
            r#"{ "provider": { "steamid": "x" }, "auth": { "token": "t" } }"#,
        )
        .unwrap();
        assert!(p.map.is_none());
        assert!(p.round.is_none());
        assert!(p.player.is_none());
    }

    #[test]
    fn unknown_fields_are_ignored() {
        let p: GsiPayload = serde_json::from_str(
            r#"{ "previously": { "player": { "state": { "health": 100 } } },
                 "added": { "x": true },
                 "round": { "phase": "over", "some_future_field": 1 } }"#,
        )
        .unwrap();
        assert_eq!(p.round.unwrap().phase.as_deref(), Some("over"));
    }

    #[test]
    fn apply_merges_full_payload() {
        let p: GsiPayload = serde_json::from_str(SAMPLE_PAYLOAD).unwrap();
        let mut gs = GameState::default();
        gs.apply(&p);
        assert_eq!(gs.round_phase.as_deref(), Some("live"));
        assert_eq!(gs.bomb.as_deref(), Some("planted"));
        assert_eq!(gs.health, Some(87));
        assert_eq!(gs.flashed, Some(0));
        assert_eq!(gs.map_name.as_deref(), Some("de_dust2"));
        assert_eq!(gs.activity.as_deref(), Some("playing"));
        assert!(gs.is_local_player);
    }

    #[test]
    fn apply_keeps_old_values_for_absent_sections() {
        let mut gs = GameState::default();
        gs.apply(&serde_json::from_str(SAMPLE_PAYLOAD).unwrap());
        let partial: GsiPayload =
            serde_json::from_str(r#"{ "round": { "phase": "over", "win_team": "CT" } }"#).unwrap();
        gs.apply(&partial);
        assert_eq!(gs.round_phase.as_deref(), Some("over"));
        assert_eq!(gs.health, Some(87));
        assert_eq!(gs.map_name.as_deref(), Some("de_dust2"));
    }

    #[test]
    fn apply_detects_spectated_player() {
        let mut gs = GameState::default();
        gs.apply(&serde_json::from_str(SAMPLE_PAYLOAD).unwrap());
        assert!(gs.is_local_player);
        let spectating: GsiPayload = serde_json::from_str(
            r#"{ "provider": { "steamid": "76561198000000001" },
                 "player": { "steamid": "76561198999999999", "name": "Mate",
                             "state": { "health": 100 } } }"#,
        )
        .unwrap();
        gs.apply(&spectating);
        assert!(!gs.is_local_player);
        assert_eq!(gs.health, Some(100));
    }

    #[test]
    fn apply_merges_team_and_defusekit() {
        let p: GsiPayload = serde_json::from_str(
            r#"{ "provider": { "steamid": "A" },
                 "player": { "steamid": "A", "team": "CT",
                             "state": { "health": 100, "defusekit": true } } }"#,
        )
        .unwrap();
        let mut gs = GameState::default();
        gs.apply(&p);
        assert_eq!(gs.team.as_deref(), Some("CT"));
        assert_eq!(gs.defusekit, Some(true));
    }

    #[test]
    fn defusekit_is_non_sticky() {
        let mut gs = GameState::default();
        gs.apply(
            &serde_json::from_str(
                r#"{ "player": { "steamid": "A", "state": { "health": 100, "defusekit": true } } }"#,
            )
            .unwrap(),
        );
        assert_eq!(gs.defusekit, Some(true));
        gs.apply(
            &serde_json::from_str(
                r#"{ "player": { "steamid": "A", "state": { "health": 80 } } }"#,
            )
            .unwrap(),
        );
        assert_eq!(gs.defusekit, None);
    }

    #[test]
    fn observing_mirrors_allplayers_presence() {
        let mut gs = GameState::default();
        assert!(!gs.observing);
        // No `player` block: free-roam camera.
        let observing: GsiPayload = serde_json::from_str(
            r#"{ "provider": { "steamid": "ME" },
                 "allplayers": { "1": { "name": "A" }, "2": { "name": "B" } } }"#,
        )
        .unwrap();
        gs.apply(&observing);
        assert!(gs.observing);
        let playing: GsiPayload =
            serde_json::from_str(r#"{ "round": { "phase": "live" } }"#).unwrap();
        gs.apply(&playing);
        assert!(!gs.observing);
    }
}
