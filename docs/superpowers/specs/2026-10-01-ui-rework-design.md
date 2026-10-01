# Main-window UI rework: design

Date: 2026-10-01. Status: approved in discussion, pending spec review.

## Goal

Rework the main window so it is nicer, consistent, and free of layout shift. The window auto-fits its height to content (`src/lib/fit-window.ts`), so any content height change resizes the window; "no layout shift" therefore means the main page keeps a constant height across all runtime states (status changes, editing, recording, warnings), changing only on real one-off state changes.

Scope is a moderate redesign: one scrolling page, regrouped and restyled. Rust behaviour is unchanged except for the additions listed under "Backend changes". The Debug page layout is out of scope.

## Decisions summary

| Area | Decision |
| --- | --- |
| Saving | Autosave on change. Save/Discard and "unsaved changes" are removed. |
| Undo | Undo/Redo icon buttons in the header, plus a reused "Saved" toast. Stack capped at 50. |
| Setup | Setup dialog opened from a status-bar button; auto-opens once per app run when not installed. Settings are never disabled. |
| Remove GSI | New `uninstall_gsi_cfg` command; "Remove" button in the dialog, no confirmation. |
| Overlay | "Overlay" section (whole overlay); bomb timer is a widget sub-section. |
| Overlay edit | In-overlay edit mode on a full-monitor surface with a top toolbar (Cancel, Smaller, Larger, OK). Size slider removed from the main page. |
| Config warnings | Sticky warning toast (until dismissed), shown only when the warning set changes. |
| Loading | Page renders below the header only after initial data loads; no placeholder swap. |

## Page layout

Window minimum width is 720 px (`tauri.conf.json`), so content width is at least 672 px after `p-6`. Every element below must fit on one line at that width.

```text
 [logo] CS2 Helper  by jackblk                          [undo] [redo]  0.3.3
╭──────────────────────────────────────────────────────────────────────────╮
│  ● Running    Audio  15%    Flash  Death  Bomb  Spec     [GSI ✓] [Pause] │
╰──────────────────────────────────────────────────────────────────────────╯
╭──────────────────────────────────────────────────────────────────────────╮
│  AUDIO EVENTS                                                            │
│  Absolute targets. When events overlap, the lowest wins. Restores 100%.  │
│                                                                          │
│  [on ]  Flash          (i)  ━━━━━━○───────────────────   15%             │
│  [on ]  Death          (i)  ━━━━━━━━━━━○──────────────   50%  [3000] ms  │
│  [on ]  Bomb           (i)  ━━━━○─────────────────────   10%  [5000] ms  │
│  [off]  Spectator      (i)  ━━━━━━━━━━━━━━━━━○────────   70%             │
│ ──────────────────────────────────────────────────────────────────────── │
│  OVERLAY                                                          [on ]  │
│  Draws over CS2. Needs borderless windowed, not fullscreen.              │
│                                                                          │
│         Position & size     [ Edit overlay ]  [ Reset ]                  │
│                                                                          │
│    Bomb timer                                                            │
│         Fuse length         [ 40 ] sec                                   │
│ ──────────────────────────────────────────────────────────────────────── │
│  HOTKEYS                                                                 │
│  Work anywhere, even in game. Captured system-wide, CS2 won't see them.  │
│                                                                          │
│  [on ]  Pause / Resume      [ F10                        ]               │
│ ──────────────────────────────────────────────────────────────────────── │
│  APP                                                                     │
│  [x] Run on startup         [x] Start minimized to tray                  │
╰──────────────────────────────────────────────────────────────────────────╯
  [Reset to defaults]                  [Open CS2 config]  [Open app config]
```

### Row grid

All setting rows share one grid: switch slot | label column | control | value | extra. Rows without a switch leave the slot empty so labels align. The label column is wide enough for "Pause / Resume" on one line (replaces the current `w-36`). Section titles, descriptions and sub-headings use one shared component so spacing is identical across sections.

### Status bar

Every slot has a fixed width so state changes never move or wrap anything:

```text
Waiting   ○ Waiting for CS2   Audio 100%   flash death bomb spec   [GSI ✓]   [Pause ]
Running   ● Running           Audio 100%   flash death bomb spec   [GSI ✓]   [Pause ]
Ducked    ● Running           Audio  15%   FLASH death BOMB spec   [GSI ✓]   [Pause ]
Paused    ‖ Paused            Audio 100%   flash death bomb spec   [GSI ✓]   [Resume]
No setup  ○ Waiting for CS2   Audio 100%   flash death bomb spec   [! Set up] [Pause ]
```

* State badge: Paused / Running / Waiting for CS2, fixed width.
* Audio: the applied target (`snap.target`, or 100% when idle or paused), fixed width, tabular numerals.
* Event chips: always four (Flash, Death, Bomb, Spec), highlighted when active in `snap.active`; tooltip shows the event's volume. Replaces the appearing/disappearing badges.
* Setup button: amber "Set up" with an alert icon when the GSI cfg is not installed, quiet "GSI ✓" when installed. Same fixed width in both states. Opens the Setup dialog.
* Pause/Resume button: fixed width. Engine state, not a setting: never enters undo history.

### Overlay section

* The header switch is `overlay.enabled`. When off, rows dim and are disabled; no hint text.
* "Position & size": `[Edit overlay] [Reset]`. While editing, the row shows `[Done] [Cancel]` at the same widths. Edit overlay works even when the overlay is disabled (see "Overlay edit mode").
* Reset restores the default position and the default scale from `default.toml` (currently 1.00), as one undoable step.
* Widget sub-sections ("Bomb timer": Fuse length) sit below. Future widgets add their own sub-heading and rows; all widgets live in the one overlay and move/scale together.

### Footer

Reset to defaults (one undoable step), Open CS2 config folder, Open app config folder.

## Autosave

Every control commits through one function, `commit(label, update)`:

* Switches and checkboxes commit immediately.
* Sliders show the dragged value locally and commit on `onValueCommit` (pointer release).
* Number inputs keep local text and commit on blur or Enter. An invalid value (not finite, not > 0) reverts to the current value without saving.
* The hotkey recorder commits when a key is captured.

Saves run strictly in order through a queue. The provider keeps `current` (optimistic, shown in the UI) and `saved` (last confirmed). On success the entry `{ label, before, after }` is pushed to history. On failure: `current` reverts to `saved`, queued saves built on top of the failed one are dropped, and an error toast shows the backend message (e.g. hotkey taken by another app).

## Undo and redo

* Header, top right, left of the version badge: Undo and Redo icon buttons. Shown on the main page only. Disabled when the respective stack is empty, and while overlay edit mode is active.
* Tooltips name the change: "Undo Flash volume", "Redo Flash volume".
* History: entries `{ label, before, after }` where `before`/`after` are full `{ config, app }` snapshots. Cap 50. A new commit clears the redo stack. Undo saves `before`; redo saves `after`. If that save fails, the stack position does not move and an error toast shows.
* Undoable: every form control, overlay edit (position and size as one entry, "Overlay position and size"), overlay Reset, Reset to defaults.
* Not undoable: Pause/Resume, GSI install/remove.
* Keyboard: Ctrl+Z undo, Ctrl+Y and Ctrl+Shift+Z redo. Ignored while focus is in a text or number input (native text undo) and while the hotkey recorder is listening.
* External changes (tray "Reload config", tray "Run on startup", a reload that picks up a hand-edited file) replace `current`/`saved` with the new settings and clear history, so stale snapshots can never overwrite the file.

### Toasts

* Toaster moves below the header via sonner's `offset`, so toasts never cover the Undo/Redo buttons.
* Success: one toast with a fixed id, updated in place: "Saved: Flash volume 15%", "Undone: Flash volume", "Redone: Flash volume". Default duration.
* Errors: 8 s.

## Setup dialog

New shadcn `dialog` component.

```text
╭─ Setup ─────────────────────────────────────────────── [x] ╮
│  Connect CS2 Helper to CS2.                                │
│                                                            │
│  (✓) 1. GSI config installed                               │
│         [Reinstall]  [Open CS2 config folder]  [Remove]    │
│                                                            │
│  ( ) 2. Receiving data from CS2                            │
│         Restart CS2 if it is already running.              │
╰────────────────────────────────────────────────────────────╯

 not installed:  ( ) 1. GSI config installed
                        [Install GSI config]  [Open CS2 config folder]
 not found:             CS2 installation not found. Install CS2, then try again.
```

* Step 1 ticks on `gsi.cfg_installed`. Install/Reinstall call `install_gsi_cfg`; Remove calls `uninstall_gsi_cfg` (destructive variant, no confirmation since reinstall is one click). Open CS2 config folder calls `open_gsi_cfg_dir`, disabled when `cfg_path` is null.
* Step 2 ticks when a fresh GSI payload has arrived (same freshness check as the Running badge).
* Toasts: install "Installed GSI config. Restart CS2 to take effect"; remove "GSI config removed. Restart CS2 to stop it sending data".
* Auto-open: once per app run, on launch, when the cfg is not installed. Closing it does not reopen it; removing the cfg later does not auto-open it.
* The dialog is a portal, so it never changes the main page height.

## Overlay edit mode

### Interaction

```text
                   ╭───┬───┬───┬───╮
                   │ ✕ │ − │ + │ ✓ │
                   ╰───┴───┴───┴───╯
                 ╭───────────────────╮
                 │       32.5        │
                 │ drag to move 1.25x│
                 ╰───────────────────╯
```

* Edit overlay (main page) enters edit mode. The overlay shows a sample reading and a hint line "drag to move · 1.25x".
* Toolbar above the overlay, order: Cancel (✕), Smaller (−), Larger (+), OK (✓). Fixed size, not scaled. Tooltips: Cancel, Smaller, Larger, Save.
* Placement: centred above the overlay; flips below when there is no room above; clamped horizontally to the monitor so it never leaves the screen, including at 0.5x where the toolbar is wider than the overlay.
* −/+ step the scale by 0.05 within the existing bounds (0.5 to 2.5).
* Drag: pointer down on the overlay and move. The overlay is clamped to the monitor.
* OK saves position and scale as one undo entry. Cancel restores the pre-edit position and scale and saves nothing. The main page's Done/Cancel do the same.
* No Esc: the overlay window is `focusable(false)` to avoid stealing focus from CS2, so it receives no key presses.

### Mechanism: full-monitor edit surface

* On edit start, Rust resizes the overlay window to cover the monitor it is on, disables click-through, and emits `overlay:edit` with the monitor rect and the current overlay rect (logical px). If the overlay is disabled, the window is created for the edit.
* The overlay page renders a light dim, the overlay at its position, and the toolbar, all positioned in JS. Drag uses pointer events, not `startDragging()`. Scale changes re-layout inside the surface; the OS window is not resized.
* Dragging to another monitor during edit is not supported (accepted trade-off).
* On OK or Cancel, the overlay invokes the end command; Rust restores the window to badge size at the resulting (or original) geometry, re-enables click-through, closes the window if the overlay is disabled, and emits `overlay:edit-result` with `{ x, y, scale }` or a cancel marker. The main window commits the result through `commit`, so the save path and history are the same as every other setting.
* Toolbar placement is a pure function (`src/lib/overlay-toolbar.ts`) so it can be unit tested.

## Loading

The main page renders nothing below the header until `engine_status`, `gsi_status` and `get_app_settings` have all resolved; the window fits once to the real layout. If any fails, one error line with a Retry button shows instead.

## Config warnings

* `useConfigWarningsToast(snap)` builds a key from the sorted warning list and toasts only when the key changes and is non-empty. Re-emitted snapshots never re-toast.
* Warning style, fixed id (a new set replaces the old toast), sticky until dismissed. Lists the warnings and offers a "Reset to defaults" action (undoable).
* When the warning set becomes empty, the toast is dismissed.
* The inline amber box is removed. The Debug page (`EngineDebug.tsx`) keeps its list unchanged.

## Backend changes

* `uninstall_gsi_cfg`: deletes `gamestate_integration_cs2helper.cfg` in the CS2 cfg dir; no-op when absent; error when the CS2 dir is not found. Lives next to `install_cfg` in `gsi/install.rs`, registered in `lib.rs`.
* Overlay edit commands: `overlay_edit_start` as described; `overlay_edit_finish` and `overlay_preview_scale` are replaced by `overlay_edit_end(result: Option<{ x, y, scale }>)` which only restores the window and emits `overlay:edit-result`; it no longer writes config (the main window saves via `save_config`). `overlay::preview_scale` and the `overlay:sample` event are removed.
* `overlay_reset_position` is removed; Reset becomes a normal commit setting `pos_x`/`pos_y` to null and `scale` to the default scale.
* External-change signal: Rust emits `settings:changed` with `{ config, app }` from the tray "Reload config" and tray "Run on startup" handlers and from `reload_config`. Today the UI only notices engine-config changes, so `[app]` changes from the tray go unnoticed. The tray "Reload config" handler switches to `load_and_apply_config`, which also reconciles the overlay (it currently does not).

## Frontend structure

```text
src/
  components/
    Layout.tsx              header Undo/Redo (main page only); SettingsProvider around Outlet;
                            Toaster offset below header
    settings/
      section.tsx           SettingsSection (title, description, optional header switch),
                            SettingRow (shared grid)
      StatusBar.tsx
      SetupDialog.tsx
      AudioSection.tsx
      OverlaySection.tsx
      HotkeysSection.tsx
      HotkeyRecorder.tsx
      AppSection.tsx
      SettingsFooter.tsx
  hooks/
    use-settings.tsx        SettingsProvider/context: current/saved, save queue, commit,
                            undo/redo, external-change handling
    use-undo-shortcuts.ts
    use-config-warnings-toast.ts
    use-engine.ts           engine:update subscription + engine_status
    use-gsi-status.ts       gsi_status polling
  lib/
    history.ts              pure undo/redo stack
    overlay-toolbar.ts      pure toolbar placement
  pages/
    MainPage.tsx            initial-load gate + section composition
    Overlay.tsx             edit surface + toolbar
```

`SettingsProvider` sits in `Layout` so the header can reach undo/redo; history survives switching to the Debug page and back.

## Small fixes included

* `text-s` typo becomes `text-sm`.
* The raw `<button>` tooltip trigger in the event row becomes a shadcn `Button` (ghost, icon size).
* `pct` moves to `@/lib/utils`; `EngineDebug.tsx` uses it.
* Em dashes removed from UI strings.
* `<img>` and text `<a>` in `Layout.tsx` stay; AGENTS.md documents them as allowed (no shadcn equivalent).
* AGENTS.md updated for the new structure, commands and events.

## Testing

* Vitest: `history.ts` (push, undo, redo, cap, redo cleared on new commit); `overlay-toolbar.ts` (above/below flip, horizontal clamp at both edges, 0.5x); warning-set key.
* Rust: `uninstall_gsi_cfg` removes the file and is a no-op when absent.
* `pnpm tsc --noEmit`, `pnpm test`, `cargo test`, `cargo check`.
* Manual (developer, in the app): no window resize from status changes, hotkey recording, warnings, or overlay editing; status bar on one line at 720 px width; overlay edit OK and Cancel at each screen edge; undo/redo across each control type, overlay edit and Reset to defaults; tray reload clears history and refreshes the form.

## Noted, not changed

* The main page's freshness threshold is 10 s (`FRESH_MS` in `MainPage.tsx`) while the tray uses 5 s (`GSI_FRESH_MS` in `lib.rs`), so the window and tray can disagree for a few seconds. Kept as is unless decided otherwise.
