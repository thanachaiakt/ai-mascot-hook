# ai-mascot-hook specification

## Purpose and platform

ai-mascot-hook is a desktop companion for Windows and macOS that displays a small
animated seal and reflects a status supplied by Claude Code hooks. It runs as a
single instance, without a taskbar button or Dock icon, in an always-on-top,
borderless window with transparent pixels. The seal does not activate other
windows when shown. The status bubble does not intercept pointer input.

The program displays a status that hooks write. `ai-mascot-hook --set <state>` writes
the status file; `ai-mascot-hook --install-hooks` points Claude Code's hooks at the
program (`--remove-hooks` takes them out), and on Windows
`tools/install-hooks.ps1` does the same by running the program. The Windows installer
and `install-mac.sh` ask for the language (English or Thai; the Windows uninstaller then uses
it too) and which AI assistants to hook (a box for each of the agents below; Claude Code and
every assistant found on the computer start ticked, or on an update the ones already hooked).
On uninstall they remove the hooks of every assistant and the whole data folder (placement,
`status.txt`, state; `install-mac.sh --keep` keeps it), silent or not. Running a newer
installer over an install updates it: the version found is shown, settings and the hooks
already in place are kept and refreshed, and the seal is restarted; an older installer asks before replacing a newer build. `--version` prints the
version. The exe
carries the seal icon (`assets/icon.ico`, from `tools/make_icon.py`). Only the program's own
hooks are added, replaced or removed: every other hook and setting stays, in its order.
The file is rewritten as indented JSON after a `settings.json.bak-<time>` backup, and a
removal that finds none of its hooks leaves the file alone.

Other agents: `--install-hooks --agent <name>` (and `--remove-hooks --agent <name>`) does
the same for `cursor` (`~/.cursor/hooks.json`), `copilot` (Copilot CLI, a file of its own,
`~/.copilot/hooks/ai-mascot-hook.json`, or under `COPILOT_HOME`), `gemini`
(`~/.gemini/settings.json`) and `codex` (OpenAI's Codex CLI, `~/.codex/hooks.json`; ChatGPT
on the web has no hooks), `qwen` (`~/.qwen/settings.json`), `windsurf`
(`~/.codeium/windsurf/hooks.json`) and `kiro` (`~/.kiro/hooks/ai-mascot-hook.json`, a file of its
own). Several names may be given, separated by commas. `all` covers Claude Code and every
other agent whose folder exists (on removal, every agent that has a file); with no `--agent`
only Claude Code is touched. A removal that leaves a file empty deletes it. Each agent's events map to the same
six states, as `README.md` tabulates under "Other AI assistants". Qwen Code takes Claude
Code's events as they are. Cursor, Windsurf and Kiro have no asking event, so they never
show asking. Only `StopFailure` (Claude Code and Qwen Code, a turn ended by an API error)
and Copilot CLI's `errorOccurred` set error. Cursor's `stop` covers turns that ended on an
error or were aborted too, and sets completed. Copilot CLI's asking comes from its
`notification` with matcher `permission_prompt|elicitation_dialog`, not `permissionRequest`,
which runs before every tool, even one Copilot allows on its own. Windsurf has no session
events, so it never sets idle. Kiro's `SessionEnd` exists only in its CLI and is left out. Some agents need an
answer on stdout, which `--set <state> --reply <kind>` prints: `allow` (Cursor's
`preToolUse`, which blocks the tool on an empty answer), `continue` (Cursor's
`beforeSubmitPrompt`), `empty` (`{}`, Codex's `Stop`). Codex asks you to review new hooks
once with `/hooks`.

### macOS

On macOS the seal, its animations, the status bubble (with all its animation),
dragging and resizing (wheel or trackpad scroll, or the corner handle), the icon
menu and the settings editor look and behave as on Windows, with these
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
%LOCALAPPDATA%\ai-mascot-hook\state
```

Hooks run by a packaged (MSIX) app, such as the Claude desktop app, have that
write redirected to `%LOCALAPPDATA%\Packages\<app>\LocalCache\Local\ai-mascot-hook\state`.
ai-mascot-hook also reads those copies (looking for new ones every 5 s) and follows
whichever state file was modified last.

The file contains a state name. ai-mascot-hook checks for changes every 400 ms. It
accepts UTF-8 (with or without a BOM) and UTF-16 (as PowerShell's `>` and
`Out-File` write), ignores surrounding whitespace and quotes, and matches names
in any case. A file that holds no word yet (read mid-write) is read again on the
next check rather than treated as a state:

| State | Animation | Default bubble text | Bubble accent |
|---|---|---|---|
| `loading` | Clapping once, then sitting and thinking | `Thinking…` | Blue |
| `working` | Working/swimming | `Working…` | Amber |
| `asking` | Asking/waving | `Need your input!` | Pink |
| `completed` | Showing off in sunglasses | `All done!` | Green |
| `error` | Crying | `Something went wrong` | Red |
| `idle` | Napping; now and then sliding, a toy car, a ball or waving | none (bubble hidden) | none |

An unknown or missing state is idle: its animations, and no bubble. Idle has no words,
so the bubble hides while the seal naps, even with **Show status** on; the editor has no
field or text color for it.
`completed` settles into idle once the file has said it for 10 s (by its modified time),
since no hook reports that the user has walked away; a new write starts it over.
The animation switches when a state change is detected; the bubble shows the
matching configured text when visible.

`loading` and `idle` have more than one animation; a new one takes over only when the
playing one comes back to its first frame. `loading` claps once through (about 6 s), then
sits and thinks until the state changes. `idle` naps; after each nap (60 s unless set
under **Idle** › **Play every**, 10–120 s) the seal wakes
and plays one of sliding, the toy car, the ball or waving from the ice floe, chosen at random, for at least 10 s,
then naps again.

Two short scenes can play over any state, which goes on afterwards:

- **Feed** (in the icon menu): the seal eats a salmon (about 20 s) while the bubble
  shows the feed message (with a salmon dot) even when the status bubble is off.
- **Break**: at each break time set in the editor, the seal rings an alarm clock for a
  minute, and the bubble shows the break message (with an orange dot) even when the
  status bubble is off. Clicking the seal ends the break at once, instead of opening the
  menu. Break times are checked on every state poll against the computer's local clock,
  and each fires once, when its minute begins.

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

- A short left-click or a right-click toggles the icon menu; while a break is ringing, a
  short left-click ends the break instead.
- Pressing and holding for 220 ms, or moving more than 4 px while pressed,
  starts dragging the seal. The pointer grab point remains fixed during the
  drag. The new position is saved on release.
- The menu shows four buttons, in this order:
  1. **Feed** gives the seal a salmon (see Break and Feed above).
  2. **Resize** enables resize mode.
  3. **Settings** opens the settings editor (below), where the bubble is also shown
     or hidden.
  4. **Close** exits the program.
- The menu closes when a button is selected, when it loses activation, or on Escape. It
  is positioned within the current monitor's work area and opens below the seal instead
  when it cannot fit above.
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

## Settings editor

The editor is a borderless, rounded window drawn in the seal's style, dragged by
its header and closed by its × button. It is one column, small enough for small or
zoomed screens, with two tabs in its header, shown as a segmented control:

- **Status messages** (the one it opens on): **Starter messages** buttons for the message sets
  (**English** and **ไทย**), one text field per state (each beside its state's accent dot),
  eight preset text-color swatches and a **+** button for the system color picker.
- **Other settings**: a **Status bubble** section, an **Idle** section, a **Break** section
  and a **Feed** section.

**Show status**, **Auto hide**, **Play every** and each of the Break and Feed fields have a short line
under them saying what they do and when the words show (the Auto hide line names its
seconds, or says the bubble stays up while it is off; the Play every line names its
seconds), and **Starter messages** has two lines under it saying it loads a starter
set of messages, in English or Thai, into every box.

Only the selected tab's fields are shown, and selecting a tab moves the keyboard to its
first field (on **Status messages**, the selected state's). Edits on both tabs are kept
until the editor closes. **Reset**, **Cancel** and **Save** sit under both tabs.
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
- A state's field may hold several lines split by `|`. The bubble says one of them,
  picked at random and never the one just said, each time it speaks up again: on every
  state change, every change of animation within a state, and when the bubble is shown or
  saved. Empty lines are skipped. The preview picks the same way.
- **English** and **ไทย** fill every state's field with that set's ten lines for the
  state, split by `|`: built-in lines written for its poses (clapping and thinking,
  digging and swimming, waving, showing off, crying), clingy and
  sweet, the seal a little one talking to its Papa (หนู to ป๊า in the Thai), many of them
  two rows split by `
`. The fields stay editable, and
  like any typing nothing is kept until **Save**.
- A state's field accepts at most 400 characters (room for a set's lines), and the
  Break and Feed fields at most 60. Typing `\n` breaks the line there in the bubble;
  the editor says so beside the Messages heading, with the `|`. On save, real
  line breaks are replaced with spaces and surrounding whitespace is removed. Empty values revert to
  that state's default text.
- **Status bubble** has a **Show status** switch (whether the bubble is shown, saved in
  `placement`) and an **Auto hide** switch with a seconds stepper (1–60, by 1 s up
  to 10 s and by 5 s from there, default 5). With auto hide on, the bubble shows after each state change and
  each change of animation within a state (and when shown or saved) and hides again after that many seconds. Both apply on
  **Save**.
- **Idle** has a **Play every** seconds stepper (10–120, by 10 s, default 60): how long
  the idle seal naps before it wakes to play a pose for 10 s or more. It applies on
  **Save**, from the next nap.
- **Break** has a **Break at** field (times of day such as `12:00, 15:30`;
  separated by commas, semicolons or spaces, `.` accepted for `:`, anything else
  ignored) and a **Break text** field, shown for the minute the alarm clock rings (empty
  reverts to `Time for a break!`). **Feed** has a **Fed text** field, shown while the
  seal eats the salmon the menu's **Feed** gives it (empty reverts to `Yum, thank you!`).
  They apply on **Save**; the times are saved sorted, each once.
- **Reset** fills all fields (no break times) and every state's text color with defaults; it does not save
  until **Save** is selected.
- **Cancel**, the × button, or pressing Escape discards edits; Enter saves.
- A state's text color applies to its bubble text. The accent dot keeps its
  fixed per-state color.

## Persistent data

Program data is stored in `%LOCALAPPDATA%\ai-mascot-hook` on Windows and in
`~/Library/Application Support/ai-mascot-hook` on macOS (where the status file is
`~/Library/Application Support/ai-mascot-hook/state`). If that location is not
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
error=Something went wrong
color.loading=#18346e
color.working=#18346e
color.asking=#18346e
color.completed=#18346e
color.error=#18346e
autohide=0
idle.every=60
break.times=12:00, 15:30
break.message=Time for a break!
feed.message=Yum, thank you!
```

The five state keys are optional; empty or unrecognized entries are ignored (including
`idle=` and `color.idle=` from older versions).
A plain `color=` line, written by older versions, sets the color of every state
that has no `color.<state>` line. A color must be exactly six hexadecimal digits prefixed by `#`; invalid color
values are ignored. A state's value may hold several lines split by `|`. `autohide` is
the auto-hide time in seconds, 0 for off;
values above 60 are treated as 60. `idle.every` is how long the idle seal naps
between plays, in seconds (10–120, default 60; values outside are brought into range). `break.times` lists the break times (none by
default) and `break.message` what the bubble says then; `feed.message` is what it says
while the seal eats its salmon. A leading byte-order mark is
ignored. Missing or invalid settings use the defaults. Saving writes all five state
values, all five `color.<state>` values, `autohide`, `idle.every`, `break.times`, `break.message` and
`feed.message`.

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

The resulting executable is `target/release/ai-mascot-hook.exe` on Windows and
`target/release/ai-mascot-hook` on macOS. The seal's art is in `ai-mascot-hook.frames`,
which must sit beside the executable: without it, or with one from another build, the seal
says so and does not start (`--set` and the other command-line options do not need it).
A Windows shortcut to it may be placed in
`shell:startup` to run ai-mascot-hook at sign-in; on macOS a LaunchAgent can start it
(see the README).

The source is split by purpose: `seal.rs` (sizes, timings, clips), `store.rs`
(data folder, state file, placement), `status.rs` (the bubble's words and
colors), `bubble.rs` (drawing and animating the bubble), `menu.rs` and `editor.rs` (the
icon menu's and the editor's layout and drawing), `anim.rs` (easing),
`hooks.rs` (installing the hooks) and `gfx` (the pixel surface, with
`gfx/win.rs` and `gfx/mac.rs` for each platform's text and display) are shared;
`win/` and `mac/` (each with the app, `menu.rs` and `editor.rs`) are each
platform's windows.
