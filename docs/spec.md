# CS2 Helper — Specification

Root spec for the project. Supersedes the original `project-init.md` (see git history).

## Vision

CS2 Helper is a Windows desktop utility for Counter-Strike 2 players, focused on **hearing protection and accessibility**. It ducks the volume of `cs2.exe` (and only `cs2.exe`) during loud game events — death, flashbang, bomb explosion, spectating — driven by official CS2 Game State Integration (GSI) callbacks and Windows Core Audio.

The name "CS2 Helper" leaves room for future player-assist features, but v1 scope is audio protection.

Primary goals:

* Reduce hearing damage risk from high-volume game events
* Reduce listener fatigue
* Improve accessibility
* Remain anti-cheat friendly
* Ship as a simple downloadable Windows binary

## Principles

### Anti-cheat safe by design

The application must **never**:

* Read CS2 memory
* Inject DLLs
* Hook DirectX/Vulkan or intercept rendering
* Modify game files

The application **only**:

* Receives official GSI events over HTTP
* Controls OS audio APIs (per-app session volume)
* Controls optional desktop accessibility features

### Opinionated and simple

The tool makes deliberate, simple choices and asks the user to adapt to them rather than supporting every configuration. The flagship example is the volume model below.

## Supported platforms

* Windows 11 only (Windows 10 is not a target)

## Architecture

```text
CS2 ──GSI HTTP POST──> GSI server ──> Event engine ──> Audio engine ──> Windows Core Audio
                                                                        (cs2.exe session only)
```

Rust module layout under `src-tauri/src/`: `audio/`, `gsi/`, `events/`, `tray/`, `config/`, `platform/`.

Frontend: React 19 + TypeScript + Vite, shadcn/ui components only (see `AGENTS.md` for UI conventions).

## Behavior model

Every feature depends on these rules; they are canonical.

### Volume model: absolute

* **Ducking:** set the `cs2.exe` session volume directly to the configured `volume` value. The config value is the **absolute target** on the 0.0–1.0 scale (`0.15` = 15%, matching the Core Audio volume scalar), not relative to anything.
* **Restore:** when the last active reduction ends, set the volume to **100%**. Always.
* **No baseline, no captured state.** Users who keep their Windows-mixer level for CS2 below 100% must adapt: set loudness with in-game volume, keep the mixer at 100%. This is an intentional, opinionated trade-off — it makes the system stateless and drift-proof.

### Event precedence: most protective wins

When multiple reductions are active simultaneously, apply the **minimum** of the active `volume` values, computed as an absolute set. There is no multiplication anywhere, so drift is impossible.

### Session loss

If the `cs2.exe` audio session disappears (game closed or crashed) while a reduction is active, discard all duck state. When a new session appears, reattach and start fresh (see Session recovery, M3).

## Configuration

File-based TOML — human-readable, hand-editable, reloadable at runtime without reinstalling. The config file is the primary customization mechanism; the UI stays minimal.

**Extensibility rule:** every feature family lives under its own parent table. All v1 hearing-protection settings sit under `[audio.*]`; future families get their own parents (e.g. `[blackflash]`), so new features never collide with or reshape existing keys.

**Error handling:** parsing never hard-errors on content.

* Unknown tables/keys → **warn and ignore** (forward-compatible: a newer or hand-edited config never bricks the app).
* Invalid values (wrong type, out of range) → warn and fall back to that key's default.
* When any warnings exist, the app surfaces them and **asks the user whether to reset the config to defaults** (the old file is backed up first, e.g. `config.toml.bak`). Shown in debug mode from M3; the main UI prompt comes with M4.

```toml
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
enabled = true
volume = 0.60

# M5
[audio.hotkey]
enabled = true
key = "F10"
```

## Debug mode (cross-cutting)

The app has two UI modes:

* **Main mode** — the minimal end-user UI (built in M4).
* **Debug mode** — a developer/diagnostics panel. The current `src/App.tsx` audio test panel is its seed.

Debug mode grows with every milestone and is the primary tool for testing each one:

* M1 (already present): list audio sessions, get/set `cs2.exe` volume manually.
* M2: live GSI payload viewer — show raw incoming POSTs and parsed game state.
* M3: event log (event detected → action taken), current duck state, active reductions and their target volumes, config-reload trigger, config warnings + reset-to-defaults action.
* M5: hotkey state display.

In M4.2, routing adds a Main page and a Debug page; until then, debug mode **is** the app's UI. M4.1 adds a pause/resume control to the debug panel.

## Milestones

### M1: Audio engine ✅ (done)

* Enumerate audio sessions across all active render endpoints.
* Match sessions by process name; get/set per-process master volume.
* All COM work on a dedicated MTA thread (`with_com`); interface pointers never escape it.
* Exposed as Tauri commands: `list_audio_sessions`, `set_process_volume`, `get_process_volume`.

**Acceptance:** commands work against a running `cs2.exe` from the debug panel.

### M2: GSI server ✅ (done)

* HTTP server (`tiny_http`) on localhost in its own thread.
* Generate and install `gamestate_integration_cs2helper.cfg` into the CS2 cfg directory, including an auth token; validate the token on incoming requests.
* Parse GSI payloads into typed game state (player status, round phase, bomb state, flash amount).
* Debug mode: live payload/state viewer.

**Acceptance:** with CS2 running, the app receives, validates, parses, and displays live state changes in the debug panel.

### M3: Event engine + config + session recovery ✅ (done)

* TOML config as specified above; loaded at startup, hot-reloadable.
* Event detection from GSI state diffs:
  * **Death** — local player dies → duck to `audio.death.volume` for `audio.death.duration_ms` (covers the death sound; the spectator reduction takes over while dead). Ends early on respawn or freeze time.
  * **Flash** — player flashed → duck to `audio.flash.volume`; restore when the flash ends.
  * **Bomb** — bomb explodes → duck to `audio.bomb.volume` for `audio.bomb.duration_ms`, then restore.
  * **Spectator** — after death, while spectating → duck to `audio.spectator.volume`; restore when the player regains control.
* Duck/restore state machine implementing the behavior model (absolute set, min precedence, restore to 100%).
* Session recovery: detect `cs2.exe` session loss and reattach automatically when the game restarts — no app restart needed.
* Debug mode: event log, active reductions, config reload.

**Acceptance:** each of the four events ducks and restores correctly in a real game; overlapping events resolve to the minimum; killing and relaunching CS2 mid-session resumes protection automatically.

### M4.1: Tray, autostart, window lifecycle ✅

* Engine global pause (restore to 100% + ignore events; runtime-only, never persists).
* System tray: status (Running/Paused/Waiting for CS2), Pause/Resume, Run on startup, Reload config, Open config folder, Show window, Exit.
* Window lifecycle: hide-to-tray on close (one-time hint), start minimized, single-instance.
* Config `[app]` table (`start_with_windows`, `start_minimized`) + value-parameterized commented-TOML serializer.
* Autostart via a Startup-folder shortcut (`.lnk`), not the registry; `config.toml` is the source of truth.
* Debug mode: pause/resume control + paused state.

**Acceptance:** app runs in the tray; pause/resume works; "Run on startup" creates/removes the Startup-folder shortcut and survives reboot; closing the window hides to tray with the engine still running; a second launch focuses the existing window; Exit restores volume.

### M4.2: Main-mode UI ✅

* Routing (HashRouter): Main page + Debug page.
* Main mode (shadcn/ui): Running/Paused/Waiting status + Pause; setup checklist (GSI cfg installed, receiving live data) + Install; editable config (audio events + Startup) written straight to `config.toml` via the M4.1 serializer; Reset-to-defaults + warnings.

**Acceptance:** a non-developer can install, configure entirely in-app, follow the setup steps, minimize to tray, and survive reboot.

### M5: Hotkey volume override

* Global hotkey (default `F10`) acting as a **toggle**: first press forces volume to 100% (e.g., while spectating teammates), second press re-applies whatever reduction is currently active. The override also clears automatically at the next round start.
* Configured under `[audio.hotkey]` (`enabled`, `key`); state shown in debug mode.

**Acceptance:** pressing the hotkey during an active reduction restores full volume; pressing again (or the next round starting) re-applies the active reduction.

## Backlog (post-v1)

* **Multiple audio devices** — edge cases for USB DACs, external interfaces, device switching mid-game. (Enumeration across endpoints already exists in M1; this covers the long tail.)

## Experimental (not scheduled)

Kept for reference; explicitly not part of v1.

* **Black Flash** — convert the flashbang white screen into a dark screen via desktop gamma/LUT manipulation. No injection, no hooks, no memory access. Risks: HDR, multi-monitor, GPU drivers, ICC profiles, fullscreen modes. Reference: `tmp/csgo_dont_blind_me-master`.
* **Helmet Dink Protection** — briefly duck on helmet headshots. Problem: GSI has no reliable headshot-hit event; accuracy would be poor.
* **EQ-Based Flash Protection** — attenuate the flash ring frequency peak (reportedly ~3 kHz) via Equalizer APO notch filter / user EQ profiles.

## Non-goals

The project will not: modify gameplay, provide competitive advantages, read game memory, inject into CS2, hook graphics APIs, implement cheats, or bypass anti-cheat. It exists solely for hearing protection and accessibility.

## Reference projects (read-only, in `tmp/`, cloned manually)

* [PatrikZeros CSGO Sound Fix](https://github.com/patrikzudel/PatrikZeros-CSGO-Sound-Fix) — `tmp/PatrikZeros-CSGO-Sound-Fix-main`. Python proof that GSI + per-process volume works for CS2 (flash/death/bomb reduction).
* [csgo_dont_blind_me](https://github.com/dev7355608/csgo_dont_blind_me) — `tmp/csgo_dont_blind_me-master`. Gamma/LUT research for Black Flash.
