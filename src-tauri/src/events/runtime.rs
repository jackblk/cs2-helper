//! The engine thread: single owner of all duck state. GSI snapshots and
//! config changes arrive over an mpsc channel; `recv_timeout` doubles as the
//! tick for death/bomb timers and the poll-while-ducked session recovery.

use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::Serialize;

use super::core::{self, Active, Latches};
use crate::config::Config;
use crate::gsi::payload::GameState;

/// Audio side effects, behind a trait so the runtime is testable without COM.
pub trait AudioControl: Send + 'static {
    /// Set the target process volume. `Ok(true)` if a session existed.
    fn set_volume(&self, volume: f32) -> Result<bool, String>;
    /// Current volume; `Ok(None)` when the process has no audio session.
    fn get_volume(&self) -> Result<Option<f32>, String>;
}

enum EngineMsg {
    State(Box<GameState>),
    NewConfig(Config, Vec<String>),
    SetPaused(bool),
    Shutdown,
}

/// One debug-log line: what happened and what the engine decided.
#[derive(Debug, Clone, Serialize)]
pub struct LogEntry {
    /// Unix epoch milliseconds.
    pub at_ms: u64,
    pub trigger: String,
    pub decision: String,
}

/// Exported engine state for the debug panel (`engine:update` + `engine_status`).
#[derive(Debug, Clone, Default, Serialize)]
pub struct EngineSnapshot {
    /// Current duck target; `None` = no reduction (100%).
    pub target: Option<f32>,
    pub active: Vec<Active>,
    /// Oldest first, capped at `LOG_CAP`.
    pub log: Vec<LogEntry>,
    pub config: Option<Config>,
    pub config_warnings: Vec<String>,
    /// Global pause: when true the engine holds 100% and ignores events.
    pub paused: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct EngineOptions {
    /// Main loop tick (recv_timeout): bounds timer latency.
    pub tick: Duration,
    /// Session-recovery poll interval (only while ducked).
    pub poll: Duration,
}

impl Default for EngineOptions {
    fn default() -> Self {
        Self {
            tick: Duration::from_millis(250),
            poll: Duration::from_secs(1),
        }
    }
}

/// Handle owned by Tauri state. Senders are mutex-wrapped so the handle is
/// `Sync`; the engine thread itself is the only owner of duck state.
pub struct EngineHandle {
    tx: Mutex<mpsc::Sender<EngineMsg>>,
    snapshot: Arc<Mutex<EngineSnapshot>>,
    thread: Mutex<Option<JoinHandle<()>>>,
}

impl EngineHandle {
    pub fn send_state(&self, state: GameState) {
        let _ = self.tx.lock().unwrap().send(EngineMsg::State(Box::new(state)));
    }

    pub fn set_config(&self, config: Config, warnings: Vec<String>) {
        let _ = self.tx.lock().unwrap().send(EngineMsg::NewConfig(config, warnings));
    }

    pub fn set_paused(&self, paused: bool) {
        let _ = self.tx.lock().unwrap().send(EngineMsg::SetPaused(paused));
    }

    pub fn snapshot(&self) -> EngineSnapshot {
        self.snapshot.lock().unwrap().clone()
    }

    /// Restore volume (if ducked) and stop the engine thread. Idempotent.
    pub fn shutdown(&self) {
        let _ = self.tx.lock().unwrap().send(EngineMsg::Shutdown);
        if let Some(thread) = self.thread.lock().unwrap().take() {
            let _ = thread.join();
        }
    }
}

const LOG_CAP: usize = 100;

fn epoch_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub fn start_engine<A: AudioControl>(
    audio: A,
    config: Config,
    warnings: Vec<String>,
    options: EngineOptions,
    on_update: impl Fn(EngineSnapshot) + Send + 'static,
) -> EngineHandle {
    let (tx, rx) = mpsc::channel::<EngineMsg>();
    let snapshot = Arc::new(Mutex::new(EngineSnapshot {
        config: Some(config),
        config_warnings: warnings.clone(),
        ..Default::default()
    }));
    let shared = snapshot.clone();

    let thread = std::thread::spawn(move || {
        let mut cfg = config;
        let mut warnings = warnings;
        let mut state = GameState::default();
        let mut latches = Latches::default();
        // Last volume we applied; None = restored (or never ducked).
        let mut applied: Option<f32> = None;
        let mut last_poll = Instant::now();
        let mut log: Vec<LogEntry> = Vec::new();
        let mut was_gone = false;
        let mut paused = false;

        loop {
            let msg = rx.recv_timeout(options.tick);
            let now = Instant::now();
            let mut triggers: Vec<String> = Vec::new();

            match msg {
                Ok(EngineMsg::State(next)) => {
                    if !paused {
                        triggers.extend(core::detect(&state, &next, &mut latches, &cfg, now));
                    }
                    state = *next;
                }
                Ok(EngineMsg::NewConfig(new_cfg, new_warnings)) => {
                    cfg = new_cfg;
                    warnings = new_warnings;
                    triggers.push("config reloaded".to_string());
                }
                Ok(EngineMsg::SetPaused(p)) => {
                    if p != paused {
                        paused = p;
                        if paused {
                            latches.clear();
                            triggers.push("paused — volume restored to 100%".to_string());
                        } else {
                            triggers.push("resumed".to_string());
                        }
                    }
                }
                Ok(EngineMsg::Shutdown) | Err(RecvTimeoutError::Disconnected) => {
                    if applied.is_some() {
                        let _ = audio.set_volume(1.0);
                    }
                    return;
                }
                Err(RecvTimeoutError::Timeout) => {}
            }

            if !paused {
                triggers.extend(core::expire(&mut latches, now));
            }

            // Session recovery: poll only while ducked (or wanting to duck).
            let mut session_gone = false;
            let mut reapply = false;
            if !paused {
                let wants_duck = core::target(&core::active(&state, &latches, &cfg)).is_some();
                if (wants_duck || applied.is_some())
                    && now.duration_since(last_poll) >= options.poll
                {
                    last_poll = now;
                    match audio.get_volume() {
                        Ok(None) => {
                            // Game closed/crashed: discard ALL duck state (spec:
                            // "Session loss"). New GSI events rebuild from fresh.
                            session_gone = true;
                            was_gone = true;
                            latches.clear();
                            state = GameState::default();
                            applied = None;
                            triggers.push(
                                "cs2.exe session gone → reduction state discarded".to_string(),
                            );
                        }
                        // Session present: re-apply below so a freshly restarted
                        // cs2.exe immediately regains protection.
                        Ok(Some(_)) => {
                            reapply = true;
                            if was_gone {
                                was_gone = false;
                                triggers.push("cs2.exe session reattached".to_string());
                            }
                        }
                        Err(e) => triggers.push(format!("audio poll failed: {e}")),
                    }
                }
            }

            let active = if paused {
                Vec::new()
            } else {
                core::active(&state, &latches, &cfg)
            };
            let new_target = core::target(&active);
            let mut decision = core::describe_decision(applied, new_target, &active);

            if !session_gone {
                match new_target {
                    Some(v) if applied != Some(v) => match audio.set_volume(v) {
                        Ok(_) => applied = Some(v),
                        Err(e) => decision = format!("audio error: {e}"),
                    },
                    Some(v) if reapply => {
                        let _ = audio.set_volume(v);
                    }
                    None if applied.is_some() => match audio.set_volume(1.0) {
                        Ok(_) => applied = None,
                        Err(e) => decision = format!("audio error: {e}"),
                    },
                    _ => {}
                }
            }

            // Publish only when something happened: log entries are the signal.
            if !triggers.is_empty() {
                let at_ms = epoch_ms();
                for trigger in triggers {
                    log.push(LogEntry {
                        at_ms,
                        trigger,
                        decision: decision.clone(),
                    });
                }
                if log.len() > LOG_CAP {
                    let excess = log.len() - LOG_CAP;
                    log.drain(..excess);
                }
                let snap = EngineSnapshot {
                    target: new_target,
                    active,
                    log: log.clone(),
                    config: Some(cfg),
                    config_warnings: warnings.clone(),
                    paused,
                };
                *shared.lock().unwrap() = snap.clone();
                on_update(snap);
            }
        }
    });

    EngineHandle {
        tx: Mutex::new(tx),
        snapshot,
        thread: Mutex::new(Some(thread)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gsi::payload::GameState;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{mpsc, Arc, Mutex};
    use std::time::{Duration, Instant};

    /// Records every set_volume call; session presence is switchable.
    #[derive(Clone, Default)]
    struct FakeAudio {
        sets: Arc<Mutex<Vec<f32>>>,
        gone: Arc<AtomicBool>,
    }

    impl AudioControl for FakeAudio {
        fn set_volume(&self, volume: f32) -> Result<bool, String> {
            if self.gone.load(Ordering::SeqCst) {
                return Ok(false);
            }
            self.sets.lock().unwrap().push(volume);
            Ok(true)
        }
        fn get_volume(&self) -> Result<Option<f32>, String> {
            if self.gone.load(Ordering::SeqCst) {
                Ok(None)
            } else {
                Ok(Some(1.0))
            }
        }
    }

    fn fast_options() -> EngineOptions {
        EngineOptions {
            tick: Duration::from_millis(10),
            poll: Duration::from_millis(30),
        }
    }

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

    fn start(
        audio: FakeAudio,
    ) -> (EngineHandle, mpsc::Receiver<EngineSnapshot>) {
        let (tx, rx) = mpsc::channel();
        let handle = start_engine(
            audio,
            crate::config::Config::default(),
            Vec::new(),
            fast_options(),
            move |snap| {
                let _ = tx.send(snap);
            },
        );
        (handle, rx)
    }

    /// Wait until a pushed snapshot satisfies `pred` (2s deadline).
    fn wait_for(
        rx: &mpsc::Receiver<EngineSnapshot>,
        pred: impl Fn(&EngineSnapshot) -> bool,
    ) -> EngineSnapshot {
        let deadline = Instant::now() + Duration::from_secs(2);
        let mut last: Option<EngineSnapshot> = None;
        while Instant::now() < deadline {
            if let Ok(s) = rx.recv_timeout(Duration::from_millis(100)) {
                let ok = pred(&s);
                last = Some(s);
                if ok {
                    return last.unwrap();
                }
            }
        }
        panic!("snapshot condition not met; last: {last:?}");
    }

    #[test]
    fn flash_ducks_then_restores() {
        let audio = FakeAudio::default();
        let (handle, rx) = start(audio.clone());

        let mut flashed = alive();
        flashed.flashed = Some(255);
        handle.send_state(flashed);
        let snap = wait_for(&rx, |s| s.target == Some(0.15));
        assert!(snap.log.iter().any(|e| e.trigger.contains("flash started")));
        assert!(audio.sets.lock().unwrap().contains(&0.15));

        handle.send_state(alive());
        wait_for(&rx, |s| s.target.is_none());
        assert_eq!(audio.sets.lock().unwrap().last(), Some(&1.0));
        handle.shutdown();
    }

    #[test]
    fn session_loss_discards_duck_state_without_restoring() {
        let audio = FakeAudio::default();
        let (handle, rx) = start(audio.clone());

        let mut flashed = alive();
        flashed.flashed = Some(255);
        handle.send_state(flashed);
        wait_for(&rx, |s| s.target == Some(0.15));

        audio.gone.store(true, Ordering::SeqCst); // cs2.exe closed
        let snap = wait_for(&rx, |s| s.target.is_none());
        assert!(snap.log.iter().any(|e| e.trigger.contains("session gone")));
        // No restore call: there is no session to restore.
        assert_eq!(audio.sets.lock().unwrap().last(), Some(&0.15));
        handle.shutdown();
    }

    #[test]
    fn shutdown_restores_volume_when_ducked() {
        let audio = FakeAudio::default();
        let (handle, rx) = start(audio.clone());
        let mut flashed = alive();
        flashed.flashed = Some(255);
        handle.send_state(flashed);
        wait_for(&rx, |s| s.target == Some(0.15));
        handle.shutdown();
        assert_eq!(audio.sets.lock().unwrap().last(), Some(&1.0));
    }

    #[test]
    fn session_reattach_is_logged_and_reapplies_duck() {
        let audio = FakeAudio::default();
        let (handle, rx) = start(audio.clone());

        let mut flashed = alive();
        flashed.flashed = Some(255);
        handle.send_state(flashed.clone());
        wait_for(&rx, |s| s.target == Some(0.15));

        // CS2 dies: duck state is discarded.
        audio.gone.store(true, Ordering::SeqCst);
        wait_for(&rx, |s| s.target.is_none());

        // CS2 relaunches and a new flash event arrives.
        audio.gone.store(false, Ordering::SeqCst);
        handle.send_state(flashed);
        let snap = wait_for(&rx, |s| {
            s.target == Some(0.15)
                && s.log.iter().any(|e| e.trigger.contains("session reattached"))
        });
        assert!(snap.log.iter().any(|e| e.trigger.contains("session reattached")));
        assert_eq!(audio.sets.lock().unwrap().last(), Some(&0.15));
        handle.shutdown();
    }

    #[test]
    fn pause_restores_to_100_and_ignores_events() {
        let audio = FakeAudio::default();
        let (handle, rx) = start(audio.clone());

        let mut flashed = alive();
        flashed.flashed = Some(255);
        handle.send_state(flashed.clone());
        wait_for(&rx, |s| s.target == Some(0.15));

        handle.set_paused(true);
        let snap = wait_for(&rx, |s| s.paused && s.target.is_none());
        assert!(snap.paused);
        assert_eq!(audio.sets.lock().unwrap().last(), Some(&1.0));

        // Events while paused are ignored: no further duck.
        handle.send_state(flashed);
        std::thread::sleep(Duration::from_millis(100));
        assert_eq!(audio.sets.lock().unwrap().last(), Some(&1.0));
        assert_eq!(
            audio.sets.lock().unwrap().iter().filter(|&&v| v == 0.15).count(),
            1,
            "only the pre-pause duck should exist"
        );
        handle.shutdown();
    }

    #[test]
    fn resume_allows_ducking_again() {
        let audio = FakeAudio::default();
        let (handle, rx) = start(audio.clone());

        handle.set_paused(true);
        wait_for(&rx, |s| s.paused);
        handle.set_paused(false);
        wait_for(&rx, |s| !s.paused);

        let mut flashed = alive();
        flashed.flashed = Some(255);
        handle.send_state(flashed);
        wait_for(&rx, |s| s.target == Some(0.15));
        assert!(audio.sets.lock().unwrap().contains(&0.15));
        handle.shutdown();
    }

    #[test]
    fn config_reload_reevaluates_active_reductions() {
        let audio = FakeAudio::default();
        let (handle, rx) = start(audio.clone());
        let mut flashed = alive();
        flashed.flashed = Some(255);
        handle.send_state(flashed);
        wait_for(&rx, |s| s.target == Some(0.15));

        let mut cfg = crate::config::Config::default();
        cfg.flash.volume = 0.05;
        handle.set_config(cfg, vec!["some warning".to_string()]);
        let snap = wait_for(&rx, |s| s.target == Some(0.05));
        assert_eq!(snap.config_warnings, vec!["some warning".to_string()]);
        handle.shutdown();
    }
}
