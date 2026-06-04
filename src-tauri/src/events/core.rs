//! Pure decision logic: state diff -> triggers, latches -> active reduction
//! set -> min target. No I/O, no clocks (callers pass `now`), fully testable.

use std::time::{Duration, Instant};

use serde::Serialize;

use crate::config::Config;
use crate::gsi::payload::GameState;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Death,
    Flash,
    Bomb,
    Spectator,
}

/// One currently-active reduction and its absolute target volume.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Active {
    pub kind: Kind,
    pub volume: f32,
}

/// Timed state that cannot be derived from the current `GameState` alone.
#[derive(Debug, Default, Clone, Copy)]
pub struct Latches {
    pub death_until: Option<Instant>,
    pub bomb_until: Option<Instant>,
}

impl Latches {
    pub fn clear(&mut self) {
        *self = Self::default();
    }
}

fn in_menu(s: &GameState) -> bool {
    s.activity.as_deref() == Some("menu")
}

/// True while observing the match rather than playing it. Never in the menu.
///
/// `observing` (the presence of the observer-only `allplayers` block) is the
/// primary signal: it holds even when free-roaming with no locked camera
/// target. The `is_local_player` fallback covers the camera-on-someone-else
/// case and requires both steamids so the no-data default never counts.
fn spectating(s: &GameState) -> bool {
    if in_menu(s) {
        return false;
    }
    s.observing
        || (!s.is_local_player && s.provider_steamid.is_some() && s.player_steamid.is_some())
}

/// The set of enabled, currently-active reductions.
pub fn active(state: &GameState, latches: &Latches, cfg: &Config) -> Vec<Active> {
    if in_menu(state) {
        return Vec::new();
    }
    let mut out = Vec::new();
    if cfg.death.enabled && latches.death_until.is_some() {
        out.push(Active { kind: Kind::Death, volume: cfg.death.volume });
    }
    if cfg.flash.enabled && state.flashed.unwrap_or(0) > 0 {
        out.push(Active { kind: Kind::Flash, volume: cfg.flash.volume });
    }
    if cfg.bomb.enabled && latches.bomb_until.is_some() {
        out.push(Active { kind: Kind::Bomb, volume: cfg.bomb.volume });
    }
    if cfg.spectator.enabled && spectating(state) {
        out.push(Active { kind: Kind::Spectator, volume: cfg.spectator.volume });
    }
    out
}

/// Most protective wins: the minimum volume, or `None` for "no reduction".
pub fn target(active: &[Active]) -> Option<f32> {
    active.iter().map(|a| a.volume).min_by(|a, b| a.total_cmp(b))
}

fn winner(active: &[Active]) -> Option<&Active> {
    active.iter().min_by(|a, b| a.volume.total_cmp(&b.volume))
}

fn kind_name(kind: Kind) -> &'static str {
    match kind {
        Kind::Death => "death",
        Kind::Flash => "flash",
        Kind::Bomb => "bomb",
        Kind::Spectator => "spectator",
    }
}

/// Render the active reductions most-protective first, marking the winner, so
/// superseded events stay visible. e.g. "flash 15% ◀ win, spectator 60%".
fn active_list(active: &[Active]) -> String {
    let mut sorted: Vec<&Active> = active.iter().collect();
    sorted.sort_by(|a, b| a.volume.total_cmp(&b.volume));
    sorted
        .iter()
        .enumerate()
        .map(|(i, a)| {
            let mark = if i == 0 { " ◀ win" } else { "" };
            format!("{} {:.0}%{mark}", kind_name(a.kind), a.volume * 100.0)
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Human-readable decision for the debug event log. With two or more active
/// reductions it enumerates all of them (winner marked) so a superseded event
/// like spectator stays visible; a lone reduction keeps the terse phrasing.
pub fn describe_decision(prev: Option<f32>, new: Option<f32>, active: &[Active]) -> String {
    match (prev, new) {
        (Some(_), None) => "restore → 100% (no active reductions)".to_string(),
        (None, None) => "no change (no active reductions)".to_string(),
        (p, Some(v)) if active.len() >= 2 => {
            let list = active_list(active);
            if p == Some(v) {
                format!("no change (active: {list})")
            } else {
                format!("reduce → {:.0}% (active: {list})", v * 100.0)
            }
        }
        (p, Some(v)) => {
            let name = winner(active).map(|w| kind_name(w.kind)).unwrap_or("?");
            if p == Some(v) {
                format!("no change ({name} {:.0}% still wins)", v * 100.0)
            } else {
                format!("reduce → {:.0}% ({name})", v * 100.0)
            }
        }
    }
}

/// Edge-detect transitions between two states, updating timed latches.
/// Returns human-readable trigger descriptions for the event log.
pub fn detect(
    prev: &GameState,
    next: &GameState,
    latches: &mut Latches,
    cfg: &Config,
    now: Instant,
) -> Vec<String> {
    let mut triggers = Vec::new();

    // Menu safety net: leaving a match clears everything.
    if in_menu(next) {
        if !in_menu(prev) {
            latches.clear();
            triggers.push("entered menu → all reductions cleared".to_string());
        }
        return triggers;
    }

    // Death: LOCAL player health goes >0 -> 0.
    let died = prev.is_local_player
        && next.is_local_player
        && prev.health.is_some_and(|h| h > 0)
        && next.health == Some(0);
    if died {
        latches.death_until = Some(now + Duration::from_millis(cfg.death.duration_ms));
        triggers.push(if cfg.death.enabled {
            "death detected".to_string()
        } else {
            "death detected (disabled)".to_string()
        });
    } else if latches.death_until.is_some() {
        // Early end: respawn, or the next round's freeze time.
        let respawned = next.is_local_player && next.health.is_some_and(|h| h > 0);
        let freezetime = next.round_phase.as_deref() == Some("freezetime")
            && prev.round_phase.as_deref() != Some("freezetime");
        if respawned || freezetime {
            latches.death_until = None;
            triggers.push("death reduction ended early (respawn/freeze time)".to_string());
        }
    }

    // Bomb: edge into "exploded".
    if next.bomb.as_deref() == Some("exploded") && prev.bomb.as_deref() != Some("exploded") {
        latches.bomb_until = Some(now + Duration::from_millis(cfg.bomb.duration_ms));
        triggers.push(if cfg.bomb.enabled {
            "bomb exploded".to_string()
        } else {
            "bomb exploded (disabled)".to_string()
        });
    }

    // Flash and spectator are level-triggered (derived in `active`); the
    // edges here exist only so the log explains decisions.
    let was_flashed = prev.flashed.unwrap_or(0) > 0;
    let is_flashed = next.flashed.unwrap_or(0) > 0;
    if !was_flashed && is_flashed {
        triggers.push(format!("flash started (flashed={})", next.flashed.unwrap_or(0)));
    } else if was_flashed && !is_flashed {
        triggers.push("flash ended".to_string());
    }
    match (spectating(prev), spectating(next)) {
        (false, true) => triggers.push("spectating started".to_string()),
        (true, false) => triggers.push("spectating ended".to_string()),
        _ => {}
    }

    triggers
}

/// Expire timed latches. Returns trigger descriptions for the event log.
pub fn expire(latches: &mut Latches, now: Instant) -> Vec<String> {
    let mut triggers = Vec::new();
    if latches.death_until.is_some_and(|d| now >= d) {
        latches.death_until = None;
        triggers.push("death reduction expired".to_string());
    }
    if latches.bomb_until.is_some_and(|d| now >= d) {
        latches.bomb_until = None;
        triggers.push("bomb reduction expired".to_string());
    }
    triggers
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::gsi::payload::GameState;
    use std::time::{Duration, Instant};

    /// Local player, alive, in a live round.
    fn alive() -> GameState {
        GameState {
            provider_steamid: Some("A".into()),
            player_steamid: Some("A".into()),
            is_local_player: true,
            activity: Some("playing".into()),
            round_phase: Some("live".into()),
            health: Some(100),
            flashed: Some(0),
            ..Default::default()
        }
    }

    #[test]
    fn no_reductions_when_nothing_happens() {
        let cfg = Config::default();
        let latches = Latches::default();
        let a = active(&alive(), &latches, &cfg);
        assert!(a.is_empty());
        assert_eq!(target(&a), None);
    }

    #[test]
    fn flash_is_level_triggered() {
        let cfg = Config::default();
        let latches = Latches::default();
        let mut s = alive();
        s.flashed = Some(255);
        let a = active(&s, &latches, &cfg);
        assert_eq!(a, vec![Active { kind: Kind::Flash, volume: 0.15 }]);
        assert_eq!(target(&a), Some(0.15));
        s.flashed = Some(0);
        assert!(active(&s, &latches, &cfg).is_empty());
    }

    #[test]
    fn flash_applies_even_when_spectating() {
        let cfg = Config::default();
        let latches = Latches::default();
        let mut s = alive();
        s.player_steamid = Some("B".into()); // camera on a teammate
        s.is_local_player = false;
        s.flashed = Some(200);
        let a = active(&s, &latches, &cfg);
        assert!(a.iter().any(|r| r.kind == Kind::Flash));
        assert!(a.iter().any(|r| r.kind == Kind::Spectator));
        // min precedence: flash 0.15 beats spectator 0.60
        assert_eq!(target(&a), Some(0.15));
    }

    #[test]
    fn observing_flag_triggers_spectator_without_steamid_mismatch() {
        let cfg = Config::default();
        let latches = Latches::default();
        // Free-roam observer: allplayers present, but the `player` block still
        // looks local (is_local_player true, no retargeted steamid). Must still
        // count as spectating — this is the case is_local_player alone missed.
        let mut s = alive();
        s.observing = true;
        let a = active(&s, &latches, &cfg);
        assert!(a.iter().any(|r| r.kind == Kind::Spectator), "{a:?}");
        // Menu still suppresses everything.
        s.activity = Some("menu".into());
        assert!(active(&s, &latches, &cfg).is_empty());
    }

    #[test]
    fn spectator_requires_steamids_and_not_menu() {
        let cfg = Config::default();
        let latches = Latches::default();
        // Default state has is_local_player == false but no steamids: NOT spectating.
        assert!(active(&GameState::default(), &latches, &cfg).is_empty());
        // In the menu nothing is ever active, even with stale flash values.
        let mut s = alive();
        s.activity = Some("menu".into());
        s.flashed = Some(255);
        assert!(active(&s, &latches, &cfg).is_empty());
    }

    #[test]
    fn latched_death_and_bomb_duck_at_configured_volumes() {
        let cfg = Config::default();
        let now = Instant::now();
        let latches = Latches {
            death_until: Some(now + Duration::from_millis(1000)),
            bomb_until: Some(now + Duration::from_millis(1500)),
        };
        let a = active(&alive(), &latches, &cfg);
        assert_eq!(a.len(), 2);
        assert_eq!(target(&a), Some(0.30));
    }

    #[test]
    fn disabled_events_never_duck() {
        let mut cfg = Config::default();
        cfg.flash.enabled = false;
        let mut s = alive();
        s.flashed = Some(255);
        assert!(active(&s, &Latches::default(), &cfg).is_empty());
    }

    #[test]
    fn decision_strings_cover_duck_restore_and_no_change() {
        let a = vec![Active { kind: Kind::Flash, volume: 0.15 }];
        assert_eq!(
            describe_decision(None, Some(0.15), &a),
            "reduce → 15% (flash)"
        );
        assert_eq!(
            describe_decision(Some(0.15), Some(0.15), &a),
            "no change (flash 15% still wins)"
        );
        assert_eq!(
            describe_decision(Some(0.15), None, &[]),
            "restore → 100% (no active reductions)"
        );
        assert_eq!(
            describe_decision(None, None, &[]),
            "no change (no active reductions)"
        );
    }

    #[test]
    fn decision_lists_superseded_events_when_multiple_active() {
        // Flash wins (0.15) while spectator (0.60) is also active but overridden.
        // Order is normalized most-protective first regardless of input order.
        let a = vec![
            Active { kind: Kind::Spectator, volume: 0.60 },
            Active { kind: Kind::Flash, volume: 0.15 },
        ];
        assert_eq!(
            describe_decision(None, Some(0.15), &a),
            "reduce → 15% (active: flash 15% ◀ win, spectator 60%)"
        );
        assert_eq!(
            describe_decision(Some(0.15), Some(0.15), &a),
            "no change (active: flash 15% ◀ win, spectator 60%)"
        );
    }

    fn dead() -> GameState {
        let mut s = alive();
        s.health = Some(0);
        s
    }

    #[test]
    fn death_latches_for_duration_then_expires() {
        let cfg = Config::default();
        let mut latches = Latches::default();
        let t0 = Instant::now();
        let triggers = detect(&alive(), &dead(), &mut latches, &cfg, t0);
        assert!(triggers.iter().any(|t| t.contains("death")), "{triggers:?}");
        assert!(latches.death_until.is_some());
        // Not yet expired just before the deadline.
        assert!(expire(&mut latches, t0 + Duration::from_millis(999)).is_empty());
        assert!(latches.death_until.is_some());
        // Expired at the deadline.
        let expired = expire(&mut latches, t0 + Duration::from_millis(1000));
        assert!(expired.iter().any(|t| t.contains("death")), "{expired:?}");
        assert!(latches.death_until.is_none());
    }

    #[test]
    fn death_to_spectator_handover() {
        let cfg = Config::default();
        let mut latches = Latches::default();
        let t0 = Instant::now();
        detect(&alive(), &dead(), &mut latches, &cfg, t0);
        // Camera moves to a teammate while the death timer runs.
        let mut spectating_state = dead();
        spectating_state.player_steamid = Some("B".into());
        spectating_state.is_local_player = false;
        spectating_state.health = Some(100); // the TEAMMATE's health
        detect(&dead(), &spectating_state, &mut latches, &cfg, t0);
        // The teammate's healthy `health` must NOT trip the respawn early-end.
        assert!(latches.death_until.is_some());
        let a = active(&spectating_state, &latches, &cfg);
        assert_eq!(target(&a), Some(0.30)); // death 0.30 wins during overlap
        // Death timer expires -> spectator volume governs.
        expire(&mut latches, t0 + Duration::from_millis(1000));
        let a = active(&spectating_state, &latches, &cfg);
        assert_eq!(target(&a), Some(0.60));
    }

    #[test]
    fn death_ends_early_on_respawn_and_freezetime() {
        let cfg = Config::default();
        let t0 = Instant::now();
        // Respawn: local player healthy again.
        let mut latches = Latches::default();
        detect(&alive(), &dead(), &mut latches, &cfg, t0);
        detect(&dead(), &alive(), &mut latches, &cfg, t0);
        assert!(latches.death_until.is_none());
        // Freeze time: next round starts while still dead.
        let mut latches = Latches::default();
        detect(&alive(), &dead(), &mut latches, &cfg, t0);
        let mut freeze = dead();
        freeze.round_phase = Some("freezetime".into());
        detect(&dead(), &freeze, &mut latches, &cfg, t0);
        assert!(latches.death_until.is_none());
    }

    #[test]
    fn spectated_teammate_health_drop_does_not_latch_death() {
        let cfg = Config::default();
        let mut latches = Latches::default();
        let mut mate_alive = alive();
        mate_alive.player_steamid = Some("B".into());
        mate_alive.is_local_player = false;
        let mut mate_dead = mate_alive.clone();
        mate_dead.health = Some(0);
        detect(&mate_alive, &mate_dead, &mut latches, &cfg, Instant::now());
        assert!(latches.death_until.is_none());
    }

    #[test]
    fn bomb_explosion_is_edge_triggered() {
        let cfg = Config::default();
        let mut latches = Latches::default();
        let t0 = Instant::now();
        let mut planted = alive();
        planted.bomb = Some("planted".into());
        let mut exploded = alive();
        exploded.bomb = Some("exploded".into());
        let triggers = detect(&planted, &exploded, &mut latches, &cfg, t0);
        assert!(triggers.iter().any(|t| t.contains("bomb")), "{triggers:?}");
        let deadline = latches.bomb_until.unwrap();
        // Same state again: no re-trigger, deadline unchanged.
        detect(&exploded, &exploded, &mut latches, &cfg, t0 + Duration::from_millis(100));
        assert_eq!(latches.bomb_until, Some(deadline));
        // Expires after duration_ms.
        expire(&mut latches, t0 + Duration::from_millis(1500));
        assert!(latches.bomb_until.is_none());
    }

    #[test]
    fn entering_menu_clears_all_latches() {
        let cfg = Config::default();
        let mut latches = Latches::default();
        let t0 = Instant::now();
        detect(&alive(), &dead(), &mut latches, &cfg, t0);
        assert!(latches.death_until.is_some());
        let mut menu = dead();
        menu.activity = Some("menu".into());
        let triggers = detect(&dead(), &menu, &mut latches, &cfg, t0);
        assert!(triggers.iter().any(|t| t.contains("menu")), "{triggers:?}");
        assert!(latches.death_until.is_none());
    }

    #[test]
    fn flash_and_spectator_edges_produce_log_triggers() {
        let cfg = Config::default();
        let mut latches = Latches::default();
        let t0 = Instant::now();
        let mut flashed = alive();
        flashed.flashed = Some(255);
        let triggers = detect(&alive(), &flashed, &mut latches, &cfg, t0);
        assert!(triggers.iter().any(|t| t.contains("flash started")), "{triggers:?}");
        let triggers = detect(&flashed, &alive(), &mut latches, &cfg, t0);
        assert!(triggers.iter().any(|t| t.contains("flash ended")), "{triggers:?}");
    }
}
