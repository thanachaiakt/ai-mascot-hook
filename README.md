# Seal Pet

A small seal that floats on the Windows desktop and acts out what Claude Code is doing:
claps while loading, swims in a hard hat while working, waves under a `?` while asking,
and naps when the turn is done or idle.

- Borderless, transparent around the seal, always on top, no taskbar button.
- Pointing at the seal shows the system's hand pointer.
- **Click** the seal: a menu of round buttons pops up above it
  - speech bubble: show or hide a status bubble ("Working…", "Need your input!", ...)
  - pencil: edit what the bubble says for each state, and its text color (eight swatches
    or any color); the bubble previews it as you type. Saved in `status.txt`.
  - arrows: resize mode. Turn the mouse wheel over the seal or drag its corner handle;
    the seal fits anywhere from a 32 x 32 to a 512 x 512 px box. Click the seal when done.
  - cross: quit
- **Press and hold** (or press and move): drag the seal anywhere.
- Position, size and the status bubble are remembered in `%LOCALAPPDATA%\seal-pet\placement`.

## How it knows the state

Claude Code hooks in `~/.claude/settings.json` write one word to
`%LOCALAPPDATA%\seal-pet\state` (`loading`, `working`, `asking`, `completed`,
or `idle`).
Seal Pet checks that file every 400 ms and switches animation when it changes.

## Build and run

```bash
cargo build --release
```

```bash
target/release/seal-pet.exe
```

Only one seal runs at a time. To start it with Windows, put a shortcut to
`seal-pet.exe` in `shell:startup`.

## Install a release

Download `seal-pet.exe` from the [GitHub Releases](https://github.com/thanachaiakt/ai-mascot-status/releases)
page and save it where you want to keep the app. Run the executable to start
Seal Pet. To launch it automatically at sign-in, put a shortcut to the
executable in `shell:startup`. Releases provide the standalone executable; no
installer is included.

## CI and releases

GitHub Actions runs `cargo test --locked` and builds the release executable on
Windows for pushes, pull requests, and manual CI runs. To publish a release,
update the package version in `Cargo.toml`, commit the change, and push a
matching version tag such as `v0.1.0`. The release workflow verifies the tag,
tests and builds the app, then attaches `seal-pet.exe` to the GitHub Release.

## The art

Each state image is an animated frame strip. Seal Pet plays the matching strip
for the current state. The previews below show the first frame; click a preview
to open its full animation strip.

| State | Image | What the seal does |
|---|---|---|
| `loading` | [![Loading animation preview](assets/previews/loading.png)](assets/loading.png) | Claps and sparkles while thinking. |
| `working` | [![Working animation preview](assets/previews/working.png)](assets/working.png) | Swims in a hard hat while working. |
| `asking` | [![Asking animation preview](assets/previews/asking.png)](assets/asking.png) | Waves beneath question marks to ask for input. |
| `completed` | [![Completed animation preview](assets/previews/completed.png)](assets/completed.png) | Settles down to nap after the turn is done. |
| `idle` | [![Idle animation preview](assets/previews/completed.png)](assets/completed.png) | Uses the same napping animation while idle. |

An unknown or missing state also uses the napping animation. The strips are
260 x 280 px per frame, rendered from `art/` by `tools/render_frames.py`. The
pet keeps them as palette indices (1 byte a pixel) and scales the current
frame to the seal's size as it draws it.

- `assets/menu.png`: the menu buttons, drawn by `tools/render_ui.py`.

Both scripts need Pillow. Re-run them after changing the art, then rebuild.

For the full behavior, input format, and persistence contract, see
[SPEC.md](SPEC.md).
