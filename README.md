# ai-mascot-hook

[ภาษาไทย: วิธีติดตั้งและใช้งาน](README.th.md)

A small seal that floats on the Windows or macOS desktop and acts out what Claude Code is doing:
claps while loading, swims in a hard hat while working, waves from an ice floe while asking,
and naps when the turn is done or idle.

<p align="center">
  <img src="assets/gif/loading.gif" width="120" alt="Seal clapping">
  <img src="assets/gif/working.gif" width="120" alt="Seal swimming in a hard hat">
  <img src="assets/gif/asking.gif" width="120" alt="Seal waving from an ice floe">
  <img src="assets/gif/completed.gif" width="120" alt="Seal napping">
</p>

## Contents

- [How it works](#how-it-works)
- [Install on Windows](#install-on-windows)
- [Install on macOS](#install-on-macos)
- [Using the seal](#using-the-seal)
- [Where settings are kept](#where-settings-are-kept)
- [Uninstall](#uninstall)
- [Troubleshooting](#troubleshooting)
- [Build from source](#build-from-source)
- [The art](#the-art)

## How it works

Two small pieces cooperate. They never talk to each other directly; they share a tiny file.

```text
Claude Code ──(hook runs)──▶ ai-mascot-hook --set working ──▶ writes the word "working"
                                                              into the `state` file
                                                                      │
the seal on your desktop ◀──(reads the file every 0.4 s)─────────────┘
        └─ sees a new word, plays that animation, shows the bubble
```

1. **Hooks.** Claude Code can run a command when something happens (you send a message,
   it uses a tool, it asks permission, it finishes). Installing ai-mascot-hook adds one such
   command per event to `~/.claude/settings.json`. Each command is just
   `ai-mascot-hook --set <state>`: it writes one word to the `state` file and exits at once.
2. **The seal.** The same program, started without arguments, is the seal on your desktop.
   It looks at the `state` file every 400 ms and plays the animation for the word it finds.
   Only one seal runs at a time.

| Claude Code event | State | What the seal does |
|---|---|---|
| `SessionStart`, `SessionEnd` | `idle` | naps |
| `UserPromptSubmit` | `loading` | claps while thinking |
| `PreToolUse`, `PostToolUse` | `working` | swims in a hard hat |
| `PermissionRequest`, `Notification` (permission, idle, MCP prompts) | `asking` | waves for your attention |
| `Stop` | `completed` | naps, turn is done |

Hooks only fire for Claude Code running on this computer. Nothing is sent over the network.

## Install on Windows

Needs 64-bit Windows 10 or later. Claude Code is optional
(without it the seal still runs, and you can drive it by hand with `--set`).

**With the installer (easiest).** Download `ai-mascot-hook-<version>-setup.exe` from the
[latest release](https://github.com/thanachaiakt/ai-mascot-hook/releases/latest) page (under **Assets**) and run it. It
installs for your user only (no admin prompt) into `%LOCALAPPDATA%\Programs\ai-mascot-hook`.
Before it touches Claude Code it shows a **Connect to Claude Code** page that explains what
it will add to `~\.claude\settings.json` and asks for permission; untick the box to skip
the hooks. It can also start the seal at sign-in. Restart Claude Code afterwards.
Uninstall it from **Settings > Apps**: that also removes the hooks.
The same page has a plain `.zip` with only the exe, for the manual steps below.

**By hand.**

1. **Build.** Install Rust from <https://rustup.rs> (skip if `cargo --version` already
   works), then in PowerShell, inside the project folder:

   ```powershell
   cargo build --release
   ```

   Copy `target\release\ai-mascot-hook.exe` to a folder you will keep, with **no spaces in
   the path**, for example `C:\Tools\ai-mascot-hook\ai-mascot-hook.exe`. Not Downloads: the
   hooks run the exe at exactly this path, and moving it later breaks them.
2. **Start the seal.** Double-click the exe. The seal appears on the screen.
3. **Install the hooks.** Open PowerShell and run:

   ```powershell
   C:\Tools\ai-mascot-hook\ai-mascot-hook.exe --install-hooks
   ```

   It backs up `~\.claude\settings.json` as `settings.json.bak-<time>`, adds the hooks from
   the table above, and keeps every other hook and setting you have.
   (From a clone of the repo, `powershell -ExecutionPolicy Bypass -File tools\install-hooks.ps1 -Exe <path>`
   does the same.)
4. **Restart Claude Code.** A session that was open before the install does not run the
   new hooks. Send Claude a message: the seal should start clapping.
5. **Start with Windows (optional).** Press `Win + R`, type `shell:startup`, press Enter,
   and put a shortcut to `ai-mascot-hook.exe` in that folder.

The Claude desktop app works too. It is a packaged (MSIX) app, so Windows redirects the
state file its hooks write into `%LOCALAPPDATA%\Packages\<app>\LocalCache\Local\ai-mascot-hook\state`;
the seal reads those copies as well and follows whichever was written last.

## Install on macOS

Needs macOS 11 or later (Apple silicon or Intel). The seal has no Dock icon.

**From a release (easiest).** Download `ai-mascot-hook-<version>-macos.tar.gz` (one binary
for both chip types) from the [latest release](https://github.com/thanachaiakt/ai-mascot-hook/releases/latest)
page, then in Terminal:

```bash
tar -xzf ~/Downloads/ai-mascot-hook-*-macos.tar.gz -C ~/Downloads
~/Downloads/ai-mascot-hook-*-macos/install.sh
```

It copies the binary to `~/bin`, explains what it will add to `~/.claude/settings.json`
and asks before adding the hooks, sets the seal to start at login, and starts it.
Restart Claude Code afterwards. `install.sh --uninstall` undoes it (settings stay).

**By hand** (build it once, about a minute):

1. **Install Rust** from <https://rustup.rs> (skip if `cargo --version` already works).
2. **Build.** In Terminal, inside the project folder:

   ```bash
   cargo build --release
   ```

3. **Put the binary somewhere permanent**, since the hooks run it by path:

   ```bash
   mkdir -p ~/bin && cp target/release/ai-mascot-hook ~/bin/
   ```

4. **Install the hooks** (run it from the place you copied the binary to):

   ```bash
   ~/bin/ai-mascot-hook --install-hooks
   ```

5. **Start the seal:**

   ```bash
   ~/bin/ai-mascot-hook &
   ```

6. **Restart Claude Code**, then send Claude a message: the seal should start clapping.
7. **Start at login (optional).** Save this as `~/Library/LaunchAgents/com.ai-mascot-hook.plist`
   (with your own path), then run
   `launchctl load ~/Library/LaunchAgents/com.ai-mascot-hook.plist`:

   ```xml
   <?xml version="1.0" encoding="UTF-8"?>
   <!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
   <plist version="1.0">
   <dict>
     <key>Label</key><string>com.ai-mascot-hook</string>
     <key>ProgramArguments</key><array><string>/Users/you/bin/ai-mascot-hook</string></array>
     <key>RunAtLoad</key><true/>
   </dict>
   </plist>
   ```

Everything else works as on Windows: dragging, resizing, the icon menu, the status editor
(its **+** opens the macOS color panel) and the bubble.

## Using the seal

- Borderless, transparent around the seal, always on top, no taskbar button.
- **Press and hold** (or press and move): drag the seal anywhere.
- **Click** the seal: a menu of three round buttons pops up above it
  1. arrows (**Resize**): resize mode. Turn the mouse wheel over the seal or drag its corner
     handle; the seal fits anywhere from a 32 x 32 to a 512 x 512 px box. Click the seal when done.
  2. gear (**Settings**): unfolds a second row with two buttons
     - speech bubble (**Show status**): show or hide a status bubble ("Working…", "Need your input!", ...)
     - pencil (**Edit status**): opens the status editor (see below).
  3. cross (**Close**): quit
- Drive it by hand (handy for testing): `ai-mascot-hook --set asking`. States:
  `loading`, `working`, `asking`, `completed`, `idle`.

### The status bubble

A thought bubble drawn in the seal's pixel-art style (dark outline, white sticker edge,
icy shading) with a little trail of round bubbles pointing at the seal. It pops out of
the seal one bubble at a time, floats gently while it is up, bounces when the state
changes, and fades away when hidden. While Claude is thinking, working or asking, a soft
ring ripples out of the colored dot.

### The status editor

- One text box per state. Type `\n` where the bubble should start a new line.
- Click a state's name (or its text box) to select it: the seal plays that state's
  animation and the bubble previews its words, so you can see each state as you edit it.
- **Text color** sets the selected state's color, so every state can have its own
  (eight swatches, or **+** for any color).
- **Bubble**: **Show status** turns the bubble on or off; **Auto hide** hides it again
  1–20 seconds after each state change.
- **Save** (or Enter) keeps the changes in `status.txt`;
  **Cancel**, × or Escape drops them; **Reset** restores the default words and colors.

## Where settings are kept

Everything lives in one folder, and nothing is stored anywhere else (no registry, no cloud):

| Platform | Folder |
|---|---|
| Windows | `%LOCALAPPDATA%\ai-mascot-hook\` |
| macOS | `~/Library/Application Support/ai-mascot-hook/` |

| File | What it holds | Written by |
|---|---|---|
| `state` | the current state word (`working`, ...) | the hooks (`--set`) |
| `placement` | seal position, size, and whether the bubble is shown | the seal, when you move, resize or toggle |
| `status.txt` | each state's words and color, and the auto-hide time | the status editor's **Save** |

`status.txt` has the same plain-text format on both platforms, so you can copy it between
a Windows PC and a Mac. Deleting a file just resets that setting to its default.
The hooks themselves live in Claude Code's `~/.claude/settings.json`.

## Uninstall

1. Quit the seal: click it, then the cross (**Close**).
2. Remove the hooks (backs up `settings.json` first):
   - Windows: `C:\Tools\ai-mascot-hook\ai-mascot-hook.exe --remove-hooks`
   - macOS: `~/bin/ai-mascot-hook --remove-hooks`
3. Delete what is left:
   - Windows: the `shell:startup` shortcut (if any), the exe, and `%LOCALAPPDATA%\ai-mascot-hook`
   - macOS: `launchctl unload ~/Library/LaunchAgents/com.ai-mascot-hook.plist` and delete that file
     (if you set up login start), the binary, and
     `rm -rf ~/Library/Application\ Support/ai-mascot-hook`

## Troubleshooting

**The seal does not follow Claude.**
- Restart Claude Code (or open a new session); sessions opened before the install do not run hooks.
- Run `ai-mascot-hook --set asking`. If the seal reacts, the program is fine and the problem
  is the hooks. If you moved the exe, run `--install-hooks` again from its new place.

**The Claude desktop app does not change the seal.** Use the latest exe and open a new session
(see the MSIX note under [Install on Windows](#install-on-windows)).

**I upgraded from the old name `seal-pet`.** Run `--install-hooks` once: it replaces the old
`seal-pet` hooks with the new ones. Settings now live in the `ai-mascot-hook` folder; copy
`status.txt` and `placement` over from the old `seal-pet` folder to keep them.

**`Access is denied` when building (Windows).** Windows locks a running exe: quit the seal first.

**No status bubble.** Turn on **Show status** in the Settings row or the editor. With
**Auto hide** on, the bubble hides itself and returns on the next state change. It also
hides while the icon menu is open.

## Build from source

Needs the stable Rust toolchain.

```bash
cargo build --release
```

The result is `target/release/ai-mascot-hook.exe` (Windows) or `target/release/ai-mascot-hook`
(macOS). Only one seal runs at a time.

## The art

Each state image is an animated frame strip. The seal plays the matching strip
for the current state. The previews below are the animated GIFs from `assets/gif/`
(transparent); click one to open its full frame strip.

| State | Image | What the seal does |
|---|---|---|
| `loading` | [<img src="assets/gif/loading.gif" width="120" alt="Loading animation preview">](assets/loading.png) | Claps happily and flaps its flippers while thinking. |
| `working` | [<img src="assets/gif/working.gif" width="120" alt="Working animation preview">](assets/working.png) | Swims and spins in a hard hat, shovel in hand, while working. Shown 1.15× larger, since the swimmer is drawn smaller than the other seals; the window reaches past the seal's box so the whole swimmer stays in view at the edges and mid-spin. |
| `asking` | [<img src="assets/gif/asking.gif" width="120" alt="Asking animation preview">](assets/asking.png) | Sits on an ice floe and waves to ask for input. |
| `completed` | [<img src="assets/gif/completed.gif" width="120" alt="Completed animation preview">](assets/completed.png) | Naps under floating zZ after the turn is done. |
| `idle` | [<img src="assets/gif/completed.gif" width="120" alt="Idle animation preview">](assets/completed.png) | Uses the same napping animation while idle. |

An unknown or missing state also uses the napping animation. The strips are
260 x 260 px (1:1) per frame, cut from the 2 x 2 clip `art/states.mp4` (not kept in the repository; one
state per quarter) by `tools/video_states.py`: it cuts each seal out of the blue
grid backdrop, trims it to a seamless loop, crops it square, and writes the strip
and a 300 x 300 GIF (`assets/gif/<state>.gif`). Asking loops only the
part where the seal waves its right flipper; the clip's left-flipper wave draws a
third flipper. The swimmer's tail, lost for two frames mid-spin, is put back
from a neighbouring frame. The program keeps the strips as palette indices (1 byte a
pixel) and scales the current frame to the seal's size as it draws it, keeping
the frame's aspect.

- `assets/menu.png`: the menu buttons, drawn by `tools/render_ui.py`.

The scripts need Pillow; `video_states.py` also needs NumPy, SciPy and `ffmpeg`
on `PATH`. Re-run them after changing the art, then rebuild.

For the full behavior, input format, and persistence contract, see
[SPEC.md](SPEC.md). For how the code fits together (with diagrams), see
[docs/en/architecture.md](docs/en/architecture.md). For token use, hook timing, CPU and memory,
see [docs/en/technical.md](docs/en/technical.md).
