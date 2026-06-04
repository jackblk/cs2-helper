# AGENTS.md

Guidance for working in this repository.

## Project

**CS2 Helper** — a Windows-only desktop utility for hearing protection / accessibility for Counter-Strike 2. It ducks **only cs2.exe's** per-app volume during loud game events (death, flashbang, bomb, spectating), driven by CS2 **Game State Integration (GSI)** HTTP callbacks and **Windows Core Audio**.

**Anti-cheat safe by design:** never reads game memory, injects DLLs, hooks rendering, or modifies game files. Only consumes official GSI events and controls OS audio APIs. Full spec: [docs/spec.md](docs/spec.md) (milestone roadmap M1–M5; M-numbers below refer to it).

## Stack

* **Tauri v2** (Rust backend) + **React 19** + **TypeScript** + **Vite 7**, package manager **pnpm**.
* **Styling:** Tailwind CSS v4 (`@import "tailwindcss"` in `src/App.css`) + **shadcn/ui**.
* **Formatting/linting:** Biome (tabs, not spaces — don't fight it).
* **Audio:** `windows` crate (windows-rs), gated `#![cfg(windows)]`.

## Commands

```sh
pnpm tauri dev            # run the app (Vite + Rust, hot reload) — primary dev loop
pnpm build               # tsc typecheck + vite build (frontend)
pnpm format              # biome format --write
cargo check --manifest-path src-tauri/Cargo.toml   # typecheck Rust without launching
```

Run the frontend typechecker with `pnpm tsc --noEmit`.

Guidelines:

* Do not use `cd` or `git -C` command if you don't need to. You're already in the repo root, and all commands are designed to be run from there.
* Developer should run the dev command in another terminal himself, not via an agent. Agents should only run build/test/lint commands as needed.

## UI conventions (important)

* **Use shadcn/ui components only.** Do NOT hand-write raw `<button>`/`<input>`/`<table>` etc. Either use an existing component from `src/components/ui/` or compose a new component from shadcn primitives.
* Add components with `pnpm dlx shadcn@latest add <name>`. Config in [components.json](components.json): style `radix-nova`, baseColor `neutral`, icons `lucide-react`.
* Import via the `@/` alias (`@/components/ui/button`, `@/lib/utils`). The alias resolves to `src/` (wired in both `vite.config.ts` and `tsconfig.json`).
* Use the `cn()` helper from `@/lib/utils` for conditional classNames.
* Note: `src/App.tsx` is currently an audio-engine test panel using raw elements (predates the shadcn rule) — migrate it to shadcn when touched. It is the seed of **debug mode** (see spec): a diagnostics panel that grows with each milestone; the minimal end-user UI ("main mode") comes in M4 with a button to swap into debug mode.

## Architecture

```
CS2 ──GSI HTTP POST──> GSI server ──> Event engine ──> Audio engine ──> Windows Core Audio (cs2.exe session only)
```

Planned Rust module layout under `src-tauri/src/` (per the spec): `audio/`, `gsi/`, `events/`, `tray/`, `config/`, `platform/`.

**Status:**

* ✅ `src-tauri/src/audio/mod.rs` — Core Audio engine. Enumerates sessions across all active render endpoints, matches by process name, get/set per-process master volume. All COM work runs on a dedicated MTA thread via `with_com` (interface pointers never escape it). Exposed as Tauri commands `list_audio_sessions` / `set_process_volume` / `get_process_volume` in `lib.rs`.
* ✅ `src-tauri/src/gsi/` — GSI server (M2). `tiny_http` on `127.0.0.1:31211` in its own thread; per-install auth token in app data dir; cfg generator/installer (Steam registry + libraryfolders.vdf `apps`-block discovery, appid 730); lenient typed payload parsing merged into a shared `GameState`; `gsi:update` Tauri event feeds the debug panel. Commands: `gsi_status` / `install_gsi_cfg`.
* ✅ `src-tauri/src/config/` — TOML config (M3). Lenient parse (warn + default, never hard-error), commented default file in app data dir, reset with `.bak` backup. Commands: `reload_config` / `reset_config` / `config_path`.
* ✅ `src-tauri/src/events/` — event engine (M3). Pure core (`core.rs`: state diff → death/flash/bomb/spectator triggers → min-volume target) + engine thread (`runtime.rs`: mpsc loop, `recv_timeout` tick for timers and poll-while-ducked session recovery, audio behind `AudioControl` trait, restore-on-exit). `engine:update` Tauri event + `engine_status` command feed the debug panel.
* ✅ M4.1 tray/autostart/window lifecycle (`src-tauri/src/autostart/`, tray + window wiring in `lib.rs`): engine pause, tray menu, hide-to-tray, single-instance, Startup-folder shortcut, `[app]` table. Commands: `set_paused` / `get_app_settings`.
* ✅ M4.2 main-mode UI (`src/pages/`, `src/components/Layout.tsx`): `HashRouter` with a Main page (status + setup checklist + editable settings, Save/Discard, reset-on-warnings) and a Debug page (the former diagnostics panel). In-app edits persist via the `save_config` command (reuses the M4.1 serializer). Shared TS types in `src/lib/types.ts`.
* ⬜ M5 hotkey override.

## Key design decisions

* **Volume model: absolute** — set the cs2.exe session volume directly to the configured `volume` target (0.5, 0.15, … on the 0.0–1.0 scale; named `volume`, not "multiplier") and restore to 100%. Not baseline-relative.
* **Event precedence: most protective (minimum)** — when multiple reductions are active, apply the smallest target volume, computed as an absolute set to avoid drift.
* Config is file-based TOML (human-editable, reloadable); the UI is intentionally minimal (status, install steps, open config, autostart, diagnostics).

## Reference projects (read-only, in `tmp/`, developer needs to clone it manually)

* `tmp/PatrikZeros-CSGO-Sound-Fix-main` — Python reference for GSI + per-process volume control (proves the approach works on CS2).
* `tmp/csgo_dont_blind_me-master` — experimental black-flash / gamma manipulation (post-v1).
