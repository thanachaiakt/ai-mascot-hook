# Seal Pet

A desktop seal (Windows and macOS) that plays an animation for the state Claude Code
hooks write. `SPEC.md` is the behavior contract; `README.md` and `README.th.md` (Thai)
are the user docs. `docs/en/` holds the English docs and `docs/th/` the same files in
Thai: `architecture.md` diagrams how the modules connect (update it when modules or data
flow change), `technical.md` gives measured token, CPU and memory costs.

## Layout

Shared by both platforms:

- `src/main.rs`: entry point. Handles `--set <state>`, `--install-hooks`,
  `--remove-hooks`, then runs `win::run()` or `mac::run()`.
- `src/seal.rs`: size limits, timings, the clip for each state.
- `src/store.rs`: data folder, state file (on Windows it also scans MSIX package
  folders), placement.
- `src/status.rs`: the bubble's words, per-state colors and auto hide (`status.txt`),
  and state-file parsing and writing.
- `src/bubble.rs`: lays out, draws and animates the status bubble and the size readout.
  Platforms only show its canvases.
- `src/menu.rs`, `src/editor.rs`: the icon menu's and the status editor's layout,
  drawing and hit-testing, shared the same way. The editor's text boxes are native.
- `src/hooks.rs`: edits `~/.claude/settings.json`.
- `src/anim.rs`: easing curves.
- `src/gfx/mod.rs`: pixel `Canvas`, SDF shapes, `Strip`.

Per platform:

- `src/gfx/win.rs` and `src/gfx/mac.rs`: surface, text, and getting pixels on screen.
- `src/win/`: Win32 app (`mod.rs`: seal window, pointer handling, bubble glue),
  `menu.rs` (icon menu window), `editor.rs` (editor window, EDIT boxes, color dialog).
- `src/mac/`: AppKit app (objc2), laid out the same way: `menu.rs` (icon menu panel),
  `editor.rs` (editor panel, NSTextFields, NSColorPanel).
- `assets/`: frame strips. `build.rs` cuts each into one PNG per frame, embedded with
  `include_bytes!`, so only the frame on screen is unpacked. `tools/`: art and hook scripts.

## Commands

```bash
cargo build --release                               # Windows exe: target/release/seal-pet.exe
cargo test --release
cargo check --target aarch64-apple-darwin           # Mac can only be compile-checked here
cargo check --target x86_64-apple-darwin
target/release/seal-pet.exe --set asking            # drive the running seal by hand
```

- Windows locks a running exe: run `Stop-Process -Name seal-pet` before a release build.
  Relaunch it afterwards.
- Every change must build warning-free on all three targets. Code used by only one
  platform gets `#[cfg(...)]` or `#[cfg_attr(not(windows), allow(dead_code))]`.

## Conventions and gotchas

- Positions and sizes are pixels from the top left of the main screen on both platforms.
  `ui`/`scale` is the display scale. The Mac converts to points only where it places a
  window.
- App state lives in a thread-local, reached through `with(|app| ...)`. Any call made
  while the app is borrowed is dropped. This includes window messages sent from inside
  a handler, a native menu or color dialog (they run their own loop), and `SetFocus`.
  Do such calls outside `with`, or post a message (see `WM_APP_FOCUS`).
- The bubble and seal art are pixel art on the seal's grid (`art_pixel`). The accent dot
  is deliberately smooth.
- The `working` clip is zoomed (`Clip::zoom`). `Strip::zoomed` grows the frames, and the
  seal window gets transparent overhang so nothing is clipped. Hit-testing and placement
  use the box, not the window (`in_box`, `overhang`).
- Comments explain why, in plain sentences, at the density of the surrounding code.
- When behavior changes, update `SPEC.md`, `README.md` and `README.th.md` together.

## Verifying UI changes on Windows

Launch the exe and take screenshots with `System.Drawing` `CopyFromScreen`. Read the
seal's position from `%LOCALAPPDATA%\seal-pet\placement`. To click inside the editor,
post `WM_LBUTTONDOWN` from a DPI-aware process (`SetProcessDPIAware`); otherwise the
coordinates get scaled and miss. `PostMessage(mascot, WM_APP+1, 1, 0)` opens the editor.
If you change `status.txt` or `placement` while testing, back them up first and restore
them afterwards: they are the user's settings.
