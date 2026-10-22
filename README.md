# Seal Pet

A small seal that floats on the Windows desktop and acts out what Claude Code is doing:
claps while loading, swims in a hard hat while working, waves under a `?` while asking,
and naps when the turn is done.

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
`%LOCALAPPDATA%\seal-pet\state` (`loading`, `working`, `asking`, `completed`).
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

## The art

- `assets/<state>.png`: frame strips, 260 x 280 px a frame, 256 colors, rendered from
  `art/` by `tools/render_frames.py`. The pet keeps them as palette indices (1 byte a
  pixel) and scales the current frame to the seal's size as it draws it.
- `assets/menu.png`: the menu buttons, drawn by `tools/render_ui.py`.

Both scripts need Pillow. Re-run them after changing the art, then rebuild.
