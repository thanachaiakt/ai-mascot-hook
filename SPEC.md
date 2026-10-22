# Seal Pet specification

## Purpose and platform

Seal Pet is a Windows desktop companion that displays a small animated seal and
reflects a status supplied by Claude Code hooks. It runs as a single instance,
without a taskbar button, in an always-on-top, borderless window with transparent
pixels. The seal does not activate other windows when shown. The status bubble
does not intercept pointer input.

The program is a status display, not a Claude Code hook installer: another
process or hook must write the current status file.

## Status input

The status input file is:

```text
%LOCALAPPDATA%\seal-pet\state
```

The file contains a state name. Seal Pet trims surrounding whitespace and checks
for changes every 400 ms. The names are case-sensitive:

| State | Animation | Default bubble text | Bubble accent |
|---|---|---|---|
| `loading` | Thinking/clapping | `Thinking…` | Blue |
| `working` | Working/swimming | `Working…` | Amber |
| `asking` | Asking/waving | `Need your input!` | Pink |
| `completed` | Completion/napping | `All done!` | Green |
| `idle` | Napping | `Napping… zZ` | Gray |

An unknown or missing state uses the napping animation and the idle bubble text
and accent. The animation switches when a state change is detected; the bubble
shows the matching configured text when visible.

## Desktop interaction

- A short left-click or a right-click toggles the icon menu.
- Pressing and holding for 220 ms, or moving more than 4 px while pressed,
  starts dragging the seal. The pointer grab point remains fixed during the
  drag. The new position is saved on release.
- The menu contains four actions:
  - **Status** toggles the bubble and saves that preference.
  - **Edit status text** opens the text and color editor.
  - **Resize** enables resize mode.
  - **Quit** exits the program.
- The menu closes when an action is selected, Escape is pressed, or it loses
  activation. It is positioned within the current monitor's work area and opens
  below the seal instead when it cannot fit above.
- In resize mode, the corner handle can be dragged or the mouse wheel can be
  used over the seal. Each wheel notch changes the height by approximately
  10%. The seal keeps the source-frame aspect ratio and its height is limited
  to 32–512 px. A short click on the seal exits resize mode; resize changes are
  saved.
- While resizing, the bubble displays the current size and resize instructions.
- The status bubble normally appears when enabled, and is temporarily shown
  while resizing or editing. It is hidden while the icon menu is open. The
  bubble is placed above the seal when possible, otherwise below, and kept
  within the monitor work area.

## Status text editor

The editor provides one text field per state, eight preset text-color swatches,
a system color picker, and **Reset**, **Cancel**, and **Save** actions.
The preset colors are `#18346e`, `#2f80ed`, `#0f9d8a`, `#2e9e44`, `#e07b00`,
`#d6336c`, `#7048e8`, and `#333333`.

- Text changes preview in the bubble for the current state.
- Each field accepts at most 60 characters. On save, line breaks are replaced
  with spaces and surrounding whitespace is removed. Empty values revert to
  that state's default text.
- **Reset** fills all fields and the text color with defaults; it does not save
  until **Save** is selected.
- **Cancel**, closing the editor, or pressing Escape discards edits.
- The selected text color applies to the bubble text. The state accent dot
  remains state-specific.

## Persistent data

Program data is stored in `%LOCALAPPDATA%\seal-pet`. If `LOCALAPPDATA` is not
available, the program uses the system temporary directory as its data base.

### Placement file

`placement` contains four whitespace-separated values:

```text
x y height show_status
```

`x` and `y` are the seal's screen position, `height` is its pixel height, and
`show_status` is `1` when the bubble is enabled and `0` otherwise. Heights are
clamped to the supported 32–512 px range when loaded. Older files that stored a
scale value (at most `4.0`) in place of the height are interpreted as a scale
of the current default size. On first run, the seal is placed near the
bottom-right of its initial work area at its default 140 px height (scaled for
system DPI), with the bubble hidden.

### Status text file

`status.txt` stores customizable bubble text and text color as UTF-8
`key=value` lines:

```text
loading=Thinking…
working=Working…
asking=Need your input!
completed=All done!
idle=Napping… zZ
color=#18346e
```

The five state keys are optional; empty or unrecognized entries are ignored.
The color must be exactly six hexadecimal digits prefixed by `#`; invalid color
values are ignored. Missing or invalid settings use the defaults. Saving writes
all five state values and the text color.

## Visual assets and build

The seal uses the frame strips in `assets/`; menu button imagery is in
`assets/menu.png`. Frame strips are 260 × 280 px per frame and are scaled to the
selected seal size. The renderer and menu are implemented with Win32 layered
windows.

Build the release executable with:

```text
cargo build --release
```

The resulting executable is `target/release/seal-pet.exe`. A Windows shortcut
to it may be placed in `shell:startup` to run Seal Pet at sign-in.
