# CS2 Helper — Specification

Root spec for the project. Supersedes the original `project-init.md` (see git history).

This is a statement of design intent and a wishlist, not a strict plan. Features may be built differently from what is described here; where this file and the code disagree, the code wins. Current implementation status lives in `AGENTS.md`.

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

Rust module layout under `src-tauri/src/`: see `AGENTS.md`.

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

If the `cs2.exe` audio session disappears (game closed or crashed) while a reduction is active, discard all duck state. When a new session appears, reattach and start fresh, with no app restart needed.

### Events

Detected from GSI state diffs:

* **Death**: local player dies → duck to `audio.death.volume` for `audio.death.duration_ms` (covers the death sound; the spectator reduction takes over while dead). Ends early on respawn or freeze time.
* **Flash**: player flashed → duck to `audio.flash.volume`; restore when the flash ends.
* **Bomb**: bomb explodes → duck to `audio.bomb.volume` for `audio.bomb.duration_ms`, then restore.
* **Spectator**: after death, while spectating → duck to `audio.spectator.volume`; restore when the player regains control.

### Overlay

A transparent, click-through, always-on-top window drawn over CS2 (needs borderless windowed, not exclusive fullscreen). It hosts small widgets; today only the **bomb timer**, which counts down from the plant (`overlay.c4_timer_s`, 40 s by default) and colours itself by whether a defuse can still finish (team and defuse kit from GSI).

* `overlay.enabled` turns the whole overlay on or off; each widget has its own switch (`overlay.bomb_timer`).
* Position and size are set in an edit mode: the overlay covers the monitor, the user drags the widget and scales it with a toolbar (Cancel, Smaller, Larger, Save). Saving is one undoable change.

## Configuration

File-based TOML — human-readable, hand-editable, reloadable at runtime without reinstalling. The config file is the primary customization mechanism; the UI stays minimal.

**Extensibility rule:** every feature family lives under its own parent table. All v1 hearing-protection settings sit under `[audio.*]`; other families get their own parents (`[app]`, `[overlay]`, `[hotkeys.*]`, later e.g. `[blackflash]`), so new features never collide with or reshape existing keys. Hotkeys use one table per action (`[hotkeys.<action>]`).

**Error handling:** parsing never hard-errors on content.

* Unknown tables/keys → **warn and ignore** (forward-compatible: a newer or hand-edited config never bricks the app).
* Invalid values (wrong type, out of range) → warn and fall back to that key's default.
* When any warnings exist, the app surfaces them and **asks the user whether to reset the config to defaults** (the old file is backed up first, e.g. `config.toml.bak`).

Excerpt (the full default file is `src-tauri/src/config/default.toml`):

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

[overlay]
enabled = true
bomb_timer = true
c4_timer_s = 40.0

[hotkeys.pause]
enabled = true
key = "F10"
```

## UI modes

The app has two UI modes:

* **Main mode**: the minimal end-user UI. A status bar, a setup dialog (install or remove the GSI config, opened automatically while it is missing), and one card per settings section. Settings save as soon as they change, with Undo/Redo in the header. The window height follows its content, so the layout must not change height on its own.
* **Debug mode**: a developer/diagnostics panel, and the primary tool for testing each feature. It grows with every feature: audio sessions and manual volume, live GSI payload/state viewer, event log, duck state and active reductions, config reload, config warnings + reset-to-defaults, pause state.

## Backlog (post-v1)

* **Multiple audio devices**: edge cases for USB DACs, external interfaces, device switching mid-game. (Enumeration across endpoints already exists; this covers the long tail.)

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
