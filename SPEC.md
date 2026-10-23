# Seal Pet specification

## Purpose and platform

Seal Pet is a desktop companion for Windows and macOS that displays a small
animated seal and reflects a status supplied by Claude Code hooks. It runs as a
single instance, without a taskbar button or Dock icon, in an always-on-top,
borderless window with transparent pixels. The seal does not activate other
windows when shown. The status bubble does not intercept pointer input.

The program displays a status that hooks write. `seal-pet --set <state>` writes
the status file; `seal-pet --install-hooks` points Claude Code's hooks at the
program (`--remove-hooks` takes them out), and on Windows
`tools/install-hooks.ps1` does the same.

### macOS

On macOS the seal, its animations, the status bubble (with all its animation),
dragging and resizing (wheel or trackpad scroll, or the corner handle), the icon
menu and the status text editor look and behave as on Windows, with these
differences:

- The icon menu and the editor take the keyboard without making the app active.
  The menu closes when another window takes the keyboard. The editor's **+**
  opens the system color panel, which changes the color as it is used.
- The app is an accessory app: no Dock icon or menu bar. Its windows float above
  other windows on every Space, including beside full-screen apps.
- Sizes are in pixels (points × the main display's scale), as on Windows.
- One instance at a time is enforced with a socket, `instance.sock`, in the data
  folder.

## Status input

The status input file is:

```text
%LOCALAPPDATA%\seal-pet\state
```

Hooks run by a packaged (MSIX) app, such as the Claude desktop app, have that
write redirected to `%LOCALAPPDATA%\Packages\<app>\LocalCache\Local\seal-pet\state`.
Seal Pet also reads those copies (looking for new ones every 5 s) and follows
whichever state file was modified last.

The file contains a state name. Seal Pet checks for changes every 400 ms. It
accepts UTF-8 (with or without a BOM) and UTF-16 (as PowerShell's `>` and
`Out-File` write), ignores surrounding whitespace and quotes, and matches names
in any case. A file that holds no word yet (read mid-write) is read again on the
next check rather than treated as a state:

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

The bubble is drawn as pixel art to match the seal: on the seal's own pixel grid
(one art pixel is the seal's height ÷ 75, rounded, at least 1 px), with a dark
outline, a white sticker edge outside it, and an icy shade along its bottom. One
line of text makes a pill; more lines get corners of up to 22 px. Its tail is two
small round bubbles trailing toward the seal.

The accent dot itself is smooth (not pixelated), with a soft halo.

The bubble animates. It comes out of the seal one part at a time (~0.5 s): the
small round bubble, then the big one, then the box, each popping with a slight
overshoot. It fades and shrinks slightly away when hidden (~0.18 s), and the box
gives a small bounce when the state it shows changes. While up, the box and the
two round bubbles float smoothly up and down (about 2 px at 100% scaling, moved
by fractions of a pixel) on a 2.8 s cycle, each a beat behind the one before, so
the tail sways like a wave. For `loading` and `working` a soft ring ripples out
of the accent dot every 1.8 s, and for `asking` every 1.2 s. Frames are drawn
about 60 times a second while the bubble pops, fades or bounces, and about 30
times a second while it floats; none are drawn while it is hidden.

## Desktop interaction

- A short left-click or a right-click toggles the icon menu.
- Pressing and holding for 220 ms, or moving more than 4 px while pressed,
  starts dragging the seal. The pointer grab point remains fixed during the
  drag. The new position is saved on release.
- The menu shows three buttons, in this order:
  1. **Resize** enables resize mode.
  2. **Settings** unfolds (or folds) a second row, on the side away from the
     seal, with two buttons:
     - **Show status** toggles the bubble and saves that preference (its label
       reads **Hide status** while the bubble is on).
     - **Edit status** opens the text and color editor.
  3. **Close** exits the program.
- Selecting **Settings** keeps the menu open. The menu closes when any other
  button is selected or it loses activation; Escape folds the settings row
  first, then closes the menu. It is positioned within the current monitor's work area and opens
  below the seal instead when it cannot fit above.
- In resize mode, the corner handle can be dragged or the mouse wheel can be
  used over the seal. Each wheel notch changes the height by approximately
  10%. The seal keeps the source-frame aspect ratio and its height is limited
  to 32–512 px. A short click on the seal exits resize mode; resize changes are
  saved.
- While resizing, a small blue pill just below the frame shows the current size in px
  (above the frame when the screen ends below the seal), and the bubble shows three short
  lines: Scroll, Drag or Click
- The status bubble normally appears when enabled, and is temporarily shown
  while resizing or editing. It is hidden while the icon menu is open. The
  bubble is placed above the seal when possible, otherwise below, and kept
  within the monitor work area.

## Status text editor

The editor is a borderless, rounded window drawn in the seal's style, dragged by
its header and closed by its × button. It provides one text field per state
(each beside its state's accent dot), eight preset text-color swatches, a **+**
button for the system color picker, a **Bubble** section, and **Reset**,
**Cancel**, and **Save** actions.
The preset colors are `#18346e`, `#2f80ed`, `#0f9d8a`, `#2e9e44`, `#e07b00`,
`#d6336c`, `#7048e8`, and `#333333`.

- The selected field (the one last focused; at first, the current state's) is
  outlined. The swatches and **+** set the text color of the selected state
  only, so each state has its own color; the swatch heading names that state.
  Each field shows its text in its state's color.
- Clicking a state's name or dot selects it, as focusing its field does.
- The selected state is previewed: the seal plays that state's animation and
  the bubble shows its text, as typed, in its color. Closing the editor returns
  the seal and bubble to the state the hooks set.
- Each field accepts at most 60 characters. Typing `\n` breaks the line there
  in the bubble; the editor says so beside the Messages heading. On save, real
  line breaks are replaced with spaces and surrounding whitespace is removed. Empty values revert to
  that state's default text.
- **Bubble** has a **Show status** switch (the same preference as the menu's
  Show/Hide status) and an **Auto hide** switch with a seconds stepper (1–20,
  default 5). With auto hide on, the bubble shows after each state change (and
  when shown or saved) and hides again after that many seconds. Both apply on
  **Save**.
- **Reset** fills all fields and every state's text color with defaults; it does not save
  until **Save** is selected.
- **Cancel**, the × button, or pressing Escape discards edits; Enter saves.
- A state's text color applies to its bubble text. The accent dot keeps its
  fixed per-state color.

## Persistent data

Program data is stored in `%LOCALAPPDATA%\seal-pet` on Windows and in
`~/Library/Application Support/seal-pet` on macOS (where the status file is
`~/Library/Application Support/seal-pet/state`). If that location is not
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

`status.txt` stores customizable bubble text and text colors as UTF-8
`key=value` lines:

```text
loading=Thinking…
working=Working…
asking=Need your input!
completed=All done!
idle=Napping… zZ
color.loading=#18346e
color.working=#18346e
color.asking=#18346e
color.completed=#18346e
color.idle=#18346e
autohide=0
```

The five state keys are optional; empty or unrecognized entries are ignored.
A plain `color=` line, written by older versions, sets the color of every state
that has no `color.<state>` line. A color must be exactly six hexadecimal digits prefixed by `#`; invalid color
values are ignored. `autohide` is the auto-hide time in seconds, 0 for off;
values above 20 are treated as 20. Missing or invalid settings use the defaults.
Saving writes all five state values, all five `color.<state>` values, and
`autohide`.

## Visual assets and build

The seal uses the frame strips in `assets/`; menu button imagery is in
`assets/menu.png`. Frame strips are 260 × 260 px (1:1) per frame and are scaled
to the selected seal size without changing their aspect, centered horizontally
and aligned to the bottom. The `working` strip's seal is drawn smaller and
floating mid-frame, so that strip is enlarged 1.15× about frame point (127, 37)
when it loads, which brings the seal to the other states' size and lowers it to
where they stand. So that no frame is cut off when the swimmer reaches a side or
spins, the enlarged frames grow by exactly as much as the art extends past them
(on any side), and the seal's window reaches that far past the seal's box with
transparent pixels, which pass clicks through. Position, size, dragging,
resizing and the bubble all keep to the box. On Windows the windows are Win32
layered windows; on macOS they are borderless AppKit panels whose layers show
the drawn pixels. The menu and the editor are drawn by shared code, so they look
the same on both; only their text boxes are each platform's own controls.

Build the release executable with:

```text
cargo build --release
```

The resulting executable is `target/release/seal-pet.exe` on Windows and
`target/release/seal-pet` on macOS. A Windows shortcut to it may be placed in
`shell:startup` to run Seal Pet at sign-in; on macOS a LaunchAgent can start it
(see the README).

The source is split by purpose: `seal.rs` (sizes, timings, clips), `store.rs`
(data folder, state file, placement), `status.rs` (the bubble's words and
colors), `bubble.rs` (drawing and animating the bubble), `menu.rs` and `editor.rs` (the
icon menu's and the editor's layout and drawing), `anim.rs` (easing),
`hooks.rs` (installing the hooks) and `gfx` (the pixel surface, with
`gfx/win.rs` and `gfx/mac.rs` for each platform's text and display) are shared;
`win/` and `mac/` (each with the app, `menu.rs` and `editor.rs`) are each
platform's windows.
