# AGENTS.md

Guidance for working in this repository.

## Project

**CS2 Helper** — a Windows-only desktop utility for hearing protection / accessibility for Counter-Strike 2. It ducks **only cs2.exe's** per-app volume during loud game events (death, flashbang, bomb, spectating), driven by CS2 **Game State Integration (GSI)** HTTP callbacks and **Windows Core Audio**.

**Anti-cheat safe by design:** never reads game memory, injects DLLs, hooks rendering, or modifies game files. Only consumes official GSI events and controls OS audio APIs. Design intent and wishlist: [docs/spec.md](docs/spec.md). It is not a strict plan; where it and the code disagree, the code wins.

## Stack

* **Tauri v2** (Rust backend) + **React 19** + **TypeScript** + **Vite 8**, package manager **pnpm**.
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

### Versioning

`package.json` is the single source of the app version. `src-tauri/tauri.conf.json` reads it via `"version": "../package.json"`, and `scripts/version.mjs` keeps `src-tauri/Cargo.toml` and `Cargo.lock` in sync.

```sh
pnpm bump 0.3.4          # set the version in package.json, Cargo.toml and Cargo.lock
pnpm version:check       # fail if they disagree
```

Releases are cut by pushing a tag equal to the version (e.g. `0.3.4`). The release workflow runs `node scripts/version.mjs check <tag>` first and fails on a mismatch.

Guidelines:

* Do not use `cd` or `git -C` command if you don't need to. You're already in the repo root, and all commands are designed to be run from there.
* Developer should run the dev command in another terminal himself, not via an agent. Agents should only run build/test/lint commands as needed.

## UI conventions (important)

* **Use shadcn/ui components only.** Do NOT hand-write raw `<button>`/`<input>`/`<table>` etc. Either use an existing component from `src/components/ui/` or compose a new component from shadcn primitives.
* Raw `<img>` and plain text `<a>` links are fine (shadcn has no equivalent), as are non-interactive text elements (`p`, `span`, `ul`, headings).
* Add components with `pnpm dlx shadcn@latest add <name>`. Config in [components.json](components.json): style `radix-nova`, baseColor `neutral`, icons `lucide-react`.
* Import via the `@/` alias (`@/components/ui/button`, `@/lib/utils`). The alias resolves to `src/` (wired in both `vite.config.ts` and `tsconfig.json`).
* Use `cn()` from the `cn` package (`import { cn } from "cn"`) for conditional classNames. Do not re-export it from `@/lib/utils`.
* Two pages under `HashRouter`: the **Main** page (`src/pages/MainPage.tsx`, the end-user UI) and the **Debug** page (`src/pages/DebugPage.tsx`, diagnostics that grow with each feature). `src/components/Layout.tsx` holds the shared header and the page switch.

## Architecture

```
CS2 ──GSI HTTP POST──> GSI server ──> Event engine ──> Audio engine ──> Windows Core Audio (cs2.exe session only)
```

Rust modules under `src-tauri/src/`: `audio/`, `gsi/`, `events/`, `config/`, `autostart/`, `hotkey.rs`, `overlay.rs`. Tray, window lifecycle and the Tauri commands live in `lib.rs`.

**Window-creation gotcha:** on Windows, building a webview window on the main thread while the event loop is running deadlocks the app and can leave a frozen always-on-top overlay over the desktop (wry#583; documented on `WebviewWindowBuilder::new`). That covers synchronous Tauri commands, tray/menu handlers and `run_on_main_thread` closures. Any command that can create a window (today `save_config`, `reload_config`, `reset_config`, `overlay_edit_start`, all via `overlay::reconcile`/`create`) must be `async`; the tray "Reload config" handler and the overlay watchdog do their work on a background thread. An `async` command that takes `State<'_>` must return `Result`.

**Status:**

* ✅ `src-tauri/src/audio/mod.rs` — Core Audio engine. Enumerates sessions across all active render endpoints, matches by process name, get/set per-process master volume. All COM work runs on a dedicated MTA thread via `with_com` (interface pointers never escape it). Exposed as Tauri commands `list_audio_sessions` / `set_process_volume` / `get_process_volume` in `lib.rs`.
* ✅ `src-tauri/src/gsi/` — GSI server. `tiny_http` on `127.0.0.1:31211` in its own thread; per-install auth token in app data dir; cfg generator/installer (Steam registry + libraryfolders.vdf `apps`-block discovery, appid 730); lenient typed payload parsing merged into a shared `GameState`; `gsi:update` Tauri event feeds the debug panel. Commands: `gsi_status` / `install_gsi_cfg` / `uninstall_gsi_cfg` / `open_gsi_cfg_dir`.
* ✅ `src-tauri/src/config/` — TOML config. Lenient parse (warn + default, never hard-error), reset with `.bak` backup. `default.toml` (embedded with `include_str!`) is the single source of default values, comments and layout: the Rust `Default` impls deserialize it, first run and Reset write it verbatim, `serialize()` fills current values into it with `toml_edit` (so saved files keep the comments), and `add_missing()` appends tables/keys missing from a user's file on load without touching existing content. Commands: `reload_config` / `reset_config` / `save_config` / `config_path` / `open_config_dir`.
* ✅ `src-tauri/src/events/` — event engine. Pure core (`core.rs`: state diff → death/flash/bomb/spectator triggers → min-volume target) + engine thread (`runtime.rs`: mpsc loop, `recv_timeout` tick for timers and poll-while-ducked session recovery, audio behind `AudioControl` trait, restore-on-exit). `engine:update` Tauri event + `engine_status` command feed the debug panel.
* ✅ Tray/autostart/window lifecycle (`src-tauri/src/autostart/`, tray + window wiring in `lib.rs`): engine pause, tray menu, hide-to-tray, single-instance, Startup-folder shortcut, `[app]` table. Commands: `set_paused` / `get_app_settings`.
* ✅ Main/Debug UI (`src/pages/`, `src/components/Layout.tsx`, `src/components/settings/`): Main page = status bar (fixed-width slots) + setup dialog (`SetupDialog.tsx`, auto-opens once per run when the GSI cfg is missing) + one card per settings section (`SettingsSection`), all sharing the `SettingRow` grid (`settings/section.tsx`) + footer. A section-wide on/off is a small ghost Enable/Disable button in the card header (`action` prop, red "Disable" / green "Enable", fixed width), and `off` dims the row labels; per-row toggles are switches in the grid's first column. Settings autosave: every control calls `commit(label, update)` from `useSettings()` (`src/hooks/use-settings.tsx`), backed by the framework-free store in `src/lib/settings-store.ts` (serialized save queue, undo/redo history capped at 50, unit tested). Undo/Redo live in the header (`UndoRedo.tsx`, Ctrl+Z / Ctrl+Y / Ctrl+Shift+Z); `SettingsProvider` sits in `Layout`. Changes made outside the page (tray, Debug page reload/reset) replace the settings and clear history; Rust emits `settings:changed` from the tray. Config warnings show as a sticky toast (`ConfigWarningsToast.tsx`) only when the warning set changes. Shared TS types in `src/lib/types.ts`. `src/lib/fit-window.ts` (`useFitWindowToContent`, used in `Layout.tsx`) resizes the main window's height to its content via a ResizeObserver (capped to the monitor work area; width left to the user; skipped while maximized), so any content height change resizes the window: keep main-page layout free of height changes.
* ✅ Overlay: transparent always-on-top `overlay` window (`src-tauri/src/overlay.rs`, route in `src/pages/Overlay.tsx`) showing a bomb defuse timer from GSI plant state + `player.team`/`defusekit`. Config `[overlay]` in `AppConfig`: `enabled` is the whole overlay (the window exists only while it is on), `bomb_timer` is the bomb timer widget (off hides the badge but keeps the window), plus `c4_timer_s`, `safety_margin_s`, `scale`, `pos_x`/`pos_y`. New widgets get their own key next to `bomb_timer`. Edit mode: the window temporarily covers its monitor and `Overlay.tsx` drags and scales the badge in JS with a toolbar (Cancel, Smaller, Larger, Save; placement math in `src/lib/overlay-edit.ts`). Outside an edit the window is badge-sized, click-through and not focusable, and `create` keeps it hidden until it is all three, so a hang or error never leaves a screen-blocking window. Exits from an edit: Cancel/Save, Esc (the window is focusable only during an edit), the main window's Done/Cancel, and the tray's "Cancel overlay edit". Rust only resizes the window and reports `overlay:edit-result`; the main window saves it through `save_config` so it is one undo step. While a session is active `reconcile` leaves the window alone. The main page's Preview emits `overlay:preview-start`, and the overlay page shows a sample countdown for 5 s (a live bomb timer wins). Commands: `overlay_edit_start` / `overlay_edit_session` / `overlay_edit_end`; events `overlay:edit`, `overlay:edit-request`, `overlay:edit-result`, `overlay:preview-start`.
* ✅ Hotkeys (`src-tauri/src/hotkey.rs`): global hotkeys through `tauri-plugin-global-shortcut` (Win32 `RegisterHotKey`, no keyboard hooks). Config is one table per action, `[hotkeys.<action>]` with `enabled` / `key` (e.g. `"F10"`, `"Ctrl+Shift+P"`); today only `[hotkeys.pause]` (default F10, shares `toggle_paused()` with the tray). To add one: a field in `HotkeysConfig`, a line in `bindings()`, a UI row. Duplicate keys are rejected; a key taken by another app fails the save and keeps the old bindings. A bound key is captured system-wide, so CS2 never receives it. Commands: `hotkey_capture_start` / `hotkey_capture_end` (unregister while the UI records a key).

## Key design decisions

* **Volume model: absolute** — set the cs2.exe session volume directly to the configured `volume` target (0.5, 0.15, … on the 0.0–1.0 scale; named `volume`, not "multiplier") and restore to 100%. Not baseline-relative.
* **Event precedence: most protective (minimum)** — when multiple reductions are active, apply the smallest target volume, computed as an absolute set to avoid drift.
* Config is file-based TOML (human-editable, reloadable); the UI is intentionally minimal (status, install steps, open config, autostart, diagnostics).

## Reference projects (read-only, in `tmp/`, developer needs to clone it manually)

* `tmp/PatrikZeros-CSGO-Sound-Fix-main` — Python reference for GSI + per-process volume control (proves the approach works on CS2).
* `tmp/csgo_dont_blind_me-master` — experimental black-flash / gamma manipulation (post-v1).
