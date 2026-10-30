# ai-mascot-hook

[ภาษาไทย: วิธีติดตั้งและใช้งาน](README.th.md)

A small seal that floats on the Windows or macOS desktop and acts out what Claude Code is doing:
claps and then sits and thinks while loading, swims in a hard hat while working, waves from an
ice floe while asking, shows off in sunglasses when the turn is done, cries when it fails,
and naps when idle, waking every minute (or as often as you set) to play. Feed it a salmon from its menu, and set
break times for it to ring an alarm clock with your own message.

<p align="center">
  <img src="assets/gif/happy.gif" width="160" alt="Seal clapping">
  <img src="assets/gif/working.gif" width="160" alt="Seal swimming in a hard hat">
  <img src="assets/gif/asking.gif" width="160" alt="Seal waving from an ice floe">
  <img src="assets/gif/completed.gif" width="160" alt="Seal napping"><br>
  <img src="assets/gif/thinking.gif" width="160" alt="Seal thinking">
  <img src="assets/gif/active.gif" width="160" alt="Seal in sunglasses">
  <img src="assets/gif/error.gif" width="160" alt="Seal crying">
  <img src="assets/gif/slide.gif" width="160" alt="Seal sliding on its belly"><br>
  <img src="assets/gif/break-time.gif" width="160" alt="Alarm clock ringing for a break">
  <img src="assets/gif/toy-car.gif" width="160" alt="Seal playing with a toy car">
  <img src="assets/gif/ball.gif" width="160" alt="Seal playing with a ball">
  <img src="assets/gif/salmon.gif" width="160" alt="Seal eating a salmon">
</p>

## Contents

- [Download](#download)
- [How it works](#how-it-works)
- [Install on Windows](#install-on-windows)
- [Install on macOS](#install-on-macos)
- [Using the seal](#using-the-seal)
- [Where settings are kept](#where-settings-are-kept)
- [Uninstall](#uninstall)
- [Troubleshooting](#troubleshooting)
- [Build from source](#build-from-source)
- [The art](#the-art)

## Download

Get the installer from the [latest release](https://github.com/thanachaiakt/ai-mascot-hook/releases/latest)
page: scroll down to **Assets** and pick the file for your system.

| System | File in Assets | Then |
| --- | --- | --- |
| Windows 10+ (64-bit) | `ai-mascot-hook-<version>-setup.exe` | [Run it](#windows-installer) |
| macOS 11+ (Apple silicon or Intel) | `ai-mascot-hook-<version>-macos.tar.gz` | [Run `install.sh`](#macos-installer) |

Prefer to build it yourself? See [Windows by hand](#windows-by-hand),
[macOS by hand](#macos-by-hand) or [Build from source](#build-from-source).

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
| `SessionStart`, `SessionEnd` | `idle` | naps, and wakes every minute (adjustable) to play |
| `UserPromptSubmit` | `loading` | claps, then sits and thinks |
| `PreToolUse`, `PostToolUse` | `working` | swims in a hard hat |
| `PermissionRequest`, `Notification` (permission, idle, MCP prompts) | `asking` | waves for your attention |
| `Stop` | `completed` | shows off in sunglasses, turn is done; naps again after 10 s |
| `StopFailure` (the turn ended on an API error) | `error` | cries |

Hooks only fire for Claude Code running on this computer. Nothing is sent over the network.

### Other AI assistants

The other assistants get the same command on their own events. Not every assistant has an
event for every state; a `–` means that assistant never shows that state.

| Assistant | `idle` | `loading` | `working` | `asking` | `completed` | `error` |
|---|---|---|---|---|---|---|
| Claude Code, Qwen Code | `SessionStart`, `SessionEnd` | `UserPromptSubmit` | `PreToolUse`, `PostToolUse` | `PermissionRequest`, `Notification` | `Stop` | `StopFailure` |
| Cursor | `sessionStart`, `sessionEnd` | `beforeSubmitPrompt` | `preToolUse`, `postToolUse` | – | `stop` | – |
| GitHub Copilot CLI | `sessionStart`, `sessionEnd` | `userPromptSubmitted` | `preToolUse`, `postToolUse` | `notification` | `agentStop` | `errorOccurred` |
| Gemini CLI | `SessionStart`, `SessionEnd` | `BeforeAgent` | `BeforeTool`, `AfterTool` | `Notification` | `AfterAgent` | – |
| OpenAI Codex | `SessionStart`, `SessionEnd` | `UserPromptSubmit` | `PreToolUse`, `PostToolUse` | `PermissionRequest` | `Stop` | – |
| Windsurf | – | `pre_user_prompt` | `pre_read_code`, `pre_write_code`, `pre_run_command`, `pre_mcp_tool_use` | – | `post_cascade_response` | – |
| Kiro | `SessionStart` | `UserPromptSubmit` | `PreToolUse`, `PostToolUse` | – | `Stop` | – |

- **Qwen Code** takes Claude Code's hooks as they are. Its `StopFailure` also fires when
  loop detection stops the turn, so the seal cries then too.
- **Cursor** reports a turn that ended on an error or was stopped through the same `stop`
  event, so the seal shows `completed` for those as well. Its `preToolUse` and
  `beforeSubmitPrompt` hooks must answer on stdout, which the hook does
  (`--reply allow`, `--reply continue`).
- **Copilot CLI** runs `permissionRequest` before every tool, even one it allows on its
  own, so `asking` waits for a `notification` that you are actually being asked
  (`permission_prompt` or `elicitation_dialog`). `errorOccurred` also fires for errors
  Copilot recovers from; the next hook then takes the seal back to work.
- **Gemini CLI**'s `Notification` fires for permission prompts. It counts hook timeouts in
  milliseconds, so its hooks get 10 000 rather than 10.
- **Codex** wants JSON from its `Stop` hook (`--reply empty` prints `{}`), and asks you to
  review new hooks once with `/hooks`.
- **Windsurf** (Cascade) has no session events, so after a session ends the seal stays on
  the last state until the next prompt.
- **Kiro** has `SessionEnd` only in its CLI, so the hooks leave it out; the seal goes back
  to `idle` when the next session starts.

## Install on Windows

Needs 64-bit Windows 10 or later. Claude Code is optional
(without it the seal still runs, and you can drive it by hand with `--set`).

### Windows installer

Easiest way. The installer is for your user only (no admin prompt) and installs into
`%LOCALAPPDATA%\Programs\ai-mascot-hook`.

1. **Download.** Open the [latest release](https://github.com/thanachaiakt/ai-mascot-hook/releases/latest)
   page, scroll to **Assets** and download `ai-mascot-hook-<version>-setup.exe`.
2. **Run it.** Double-click the file.
   The first screen asks for the language, English or Thai; the installer and uninstaller
   then speak it.
3. **Choose your AI assistants.** Before it touches any settings, the installer shows a page
   that explains what it will add and to which file, with a box for each of Claude Code,
   Cursor, GitHub Copilot CLI, Gemini CLI, OpenAI Codex, Qwen Code, Windsurf and Kiro.
   Claude Code and any assistant found on this computer start ticked. Untick all to skip hooks.
4. **Start at sign-in (optional).** Tick the option in the installer.
5. **Restart the assistant.** Send it a message: the seal should clap, then sit and think.

To remove it, use **Settings > Apps**. That removes the hooks from every assistant (not only
the ones you ticked) and deletes the seal's settings (`%LOCALAPPDATA%\ai-mascot-hook`) too, so
nothing is left behind except the `.bak-<time>` copies of the files it edited.

To update, run the newer `setup.exe` over the old install (or `install.sh` again on a Mac): it
says which version it is replacing, keeps your settings and hooks, and restarts the seal.
Assets also has a plain `.zip` with only the exe and its `ai-mascot-hook.frames`, for the
steps below.

### Windows by hand

1. **Build.** Install Rust from <https://rustup.rs> (skip if `cargo --version` already
   works), then in PowerShell, inside the project folder:

   ```powershell
   cargo build --release
   ```

   Copy `target\release\ai-mascot-hook.exe` and `target\release\ai-mascot-hook.frames` (the
   seal's art, which must sit beside the exe) to a folder you will keep, with **no spaces in
   the path**, for example `C:\Tools\ai-mascot-hook\ai-mascot-hook.exe`. Not Downloads: the
   hooks run the exe at exactly this path, and moving it later breaks them.
2. **Start the seal.** Double-click the exe. The seal appears on the screen.
3. **Install the hooks.** Open PowerShell and run:

   ```powershell
   C:\Tools\ai-mascot-hook\ai-mascot-hook.exe --install-hooks
   ```

   It backs up `~\.claude\settings.json` as `settings.json.bak-<time>`, adds the hooks from
   the table above, and keeps every other hook and setting you have, in the same order.
   Only ai-mascot-hook's own hooks are added or replaced; the file is saved as indented JSON.
   (From a clone of the repo, `powershell -ExecutionPolicy Bypass -File tools\install-hooks.ps1 -Exe <path>`
   does the same.)
4. **Restart Claude Code.** A session that was open before the install does not run the
   new hooks. Send Claude a message: the seal should clap, then sit and think.

   **Other agents.** Add `--agent` with `cursor`, `copilot` (Copilot CLI), `gemini`, `codex`
   (OpenAI's Codex CLI; ChatGPT on the web has no hooks), `qwen`, `windsurf` or `kiro` to
   `--install-hooks` to hook that agent instead. Several go in one list (`--agent claude,kiro`),
   and `all` means Claude Code plus every agent found on this computer. `--remove-hooks --agent
   <name>` takes them out (`all` removes every agent's). Codex asks you to review the new hooks
   once with `/hooks`. Which events set which state for each agent is in
   [Other AI assistants](#other-ai-assistants).
5. **Start with Windows (optional).** Press `Win + R`, type `shell:startup`, press Enter,
   and put a shortcut to `ai-mascot-hook.exe` in that folder.

The Claude desktop app works too. It is a packaged (MSIX) app, so Windows redirects the
state file its hooks write into `%LOCALAPPDATA%\Packages\<app>\LocalCache\Local\ai-mascot-hook\state`;
the seal reads those copies as well and follows whichever was written last.

## Install on macOS

Needs macOS 11 or later (Apple silicon or Intel). The seal has no Dock icon.

### macOS installer

Easiest way. One binary works on both chip types.

1. **Download.** Open the [latest release](https://github.com/thanachaiakt/ai-mascot-hook/releases/latest)
   page, scroll to **Assets** and download `ai-mascot-hook-<version>-macos.tar.gz`.
2. **Unpack and run it** in Terminal:

   ```bash
   tar -xzf ~/Downloads/ai-mascot-hook-*-macos.tar.gz -C ~/Downloads
   ~/Downloads/ai-mascot-hook-*-macos/install.sh
   ```

   The script asks for the language (English or Thai), copies the binary to `~/bin`, explains
   what it will add and to which files, asks which AI assistants to hook, sets the seal to
   start at login, and starts it. `--lang en|th`, `--agents claude,cursor` (or `all`, `none`)
   and `--yes` answer those questions up front.
3. **Restart the assistant.** Send it a message: the seal should clap, then sit and think.

`install.sh --uninstall` undoes everything: the hooks of every assistant, the login item, the
binary and the seal's settings (`--keep` keeps the settings).

### macOS by hand

Build it once, about a minute:

1. **Install Rust** from <https://rustup.rs> (skip if `cargo --version` already works).
2. **Build.** In Terminal, inside the project folder:

   ```bash
   cargo build --release
   ```

3. **Put the binary somewhere permanent**, since the hooks run it by path:

   ```bash
   mkdir -p ~/bin && cp target/release/ai-mascot-hook target/release/ai-mascot-hook.frames ~/bin/
   ```

4. **Install the hooks** (run it from the place you copied the binary to):

   ```bash
   ~/bin/ai-mascot-hook --install-hooks
   ```

5. **Start the seal:**

   ```bash
   ~/bin/ai-mascot-hook &
   ```

6. **Restart Claude Code**, then send Claude a message: the seal should clap, then sit and think.
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

Everything else works as on Windows: dragging, resizing, the icon menu, the settings editor
(its **+** opens the macOS color panel) and the bubble.

## Using the seal

- Borderless, transparent around the seal, always on top, no taskbar button.
- **Press and hold** (or press and move): drag the seal anywhere.
- **Click** the seal: a menu of four round buttons pops up above it (while a break is
  ringing, a click ends the break instead)
  1. fish (**Feed**): the seal eats a salmon and thanks you (the feed message, set in the
     editor), then goes back to what it was doing.
  2. arrows (**Resize**): resize mode. Turn the mouse wheel over the seal or drag its corner
     handle; the seal fits anywhere from a 32 x 32 to a 512 x 512 px box. Click the seal when done.
  3. gear (**Settings**): opens the settings editor (see below), where you also show or
     hide the status bubble ("Working…", "Need your input!", ...)
  4. cross (**Close**): quit
- At each **break time** (set in the editor) the seal rings an alarm clock for a minute and
  the bubble shows your break message, even with the status bubble off. Click the seal to
  end the break early.
- Drive it by hand (handy for testing): `ai-mascot-hook --set asking`. States:
  `loading`, `working`, `asking`, `completed`, `error`, `idle`.

### The status bubble

A thought bubble drawn in the seal's pixel-art style (dark outline, white sticker edge,
icy shading) with a little trail of round bubbles pointing at the seal. It pops out of
the seal one bubble at a time, floats gently while it is up, bounces when the state
changes, and fades away when hidden. While Claude is thinking, working or asking, a soft
ring ripples out of the colored dot.

### The settings editor

Two tabs at the top; changes on both are kept until you press **Save** or **Cancel**.

**Status messages**
- One text box per state, idle aside: the bubble says nothing and hides while the seal
  naps. Put several lines in it, split by `|`, and the bubble picks one
  at random each time the state or the seal's pose changes. Type `\n` where the bubble
  should start a new line.
- **Starter messages** **English** or **ไทย** (Thai) loads a starter set of messages into
  every box: ten built-in lines per state, clingy and sweet, the seal talking to its Papa
  (ป๊า in the Thai), written to match what the seal is doing. Edit them as you like; nothing is kept until **Save**.
- Click a state's name (or its text box) to select it: the seal plays that state's
  animation and the bubble previews its words, so you can see each state as you edit it.
- **Text color** sets the selected state's color, so every state can have its own
  (eight swatches, or **+** for any color).

**Other settings**
- **Status bubble**: **Show status** turns on the bubble over the seal that says what the
  AI is doing. **Auto hide** hides it again 1–60 seconds after it pops up, and it pops up
  again on the next state change or change of pose; with Auto hide off, it stays up.
- **Idle**: **Play every** sets how long the idle seal naps before it wakes to play a
  pose, 10–120 seconds (default 60).
- **Break**: **Break at** lists the times of day to take a break, such as
  `12:00, 15:30`; **Break text** is what the bubble says for the minute the alarm clock
  rings.
- **Feed**: **Fed text** is what the bubble says while the seal eats the salmon that
  **Feed** in the menu gives it.

**Save** (or Enter) keeps the changes in `status.txt`; **Cancel**, × or Escape drops them;
**Reset** restores the default words and colors.

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
| `status.txt` | each state's words and color, the auto-hide time, the idle nap time, the break times, and the break and feed messages | the settings editor's **Save** |

`status.txt` has the same plain-text format on both platforms, so you can copy it between
a Windows PC and a Mac. Deleting a file just resets that setting to its default.
The hooks themselves live in Claude Code's `~/.claude/settings.json`.

## Uninstall

1. Quit the seal: click it, then the cross (**Close**).
2. Remove the hooks of every assistant (backs up each file first):
   - Windows: `C:\Tools\ai-mascot-hook\ai-mascot-hook.exe --remove-hooks --agent all`
   - macOS: `~/bin/ai-mascot-hook --remove-hooks --agent all`
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

**No status bubble.** Turn on **Show status** under **Settings** → **Other settings**. With
**Auto hide** on, the bubble hides itself and returns on the next state change or change of pose. It also
hides while the icon menu is open.

## Build from source

Needs the stable Rust toolchain.

```bash
cargo build --release
```

The result is `target/release/ai-mascot-hook.exe` (Windows) or `target/release/ai-mascot-hook`
(macOS), with the seal's art beside it in `ai-mascot-hook.frames`. Only one seal runs at a time.

## The art

Each animation is a frame strip. A state plays one strip, or several in turn: each
plays for at least 10 s, then the next takes over as it comes back to its first frame.
The previews below are the animated GIFs from `assets/gif/`
(transparent); click one to open its full frame strip.

| State | Image | What the seal does |
|---|---|---|
| `loading` | [<img src="assets/gif/happy.gif" width="120" alt="Clapping animation preview">](assets/happy.png) [<img src="assets/gif/thinking.gif" width="120" alt="Thinking animation preview">](assets/thinking.png) | Claps happily once through for the new prompt, then sits with a flipper to its chin and thinks, blinking. |
| `working` | [<img src="assets/gif/working.gif" width="120" alt="Working animation preview">](assets/working.png) | Swims and spins in a hard hat, shovel in hand. The swimmer is shown 1.15× larger, since it is drawn smaller than the other seals; the window reaches past the seal's box so the whole swimmer stays in view at the edges and mid-spin. |
| `asking` | [<img src="assets/gif/asking.gif" width="120" alt="Asking animation preview">](assets/asking.png) | Sits on an ice floe and waves to ask for input. |
| `completed` | [<img src="assets/gif/active.gif" width="120" alt="Completed animation preview">](assets/active.png) | Shows off in sunglasses, sparkling, after the turn is done, for 10 s, then naps as in `idle`. |
| `error` | [<img src="assets/gif/error.gif" width="120" alt="Error animation preview">](assets/error.png) | Cries into its flippers when the turn ends on an error. |
| `idle` | [<img src="assets/gif/completed.gif" width="120" alt="Napping animation preview">](assets/completed.png) [<img src="assets/gif/slide.gif" width="120" alt="Sliding animation preview">](assets/slide.png) [<img src="assets/gif/toy-car.gif" width="120" alt="Toy car animation preview">](assets/toy-car.png) [<img src="assets/gif/ball.gif" width="120" alt="Ball animation preview">](assets/ball.png) [<img src="assets/gif/asking.gif" width="120" alt="Waving animation preview">](assets/asking.png) | Naps under floating zZ; every minute it wakes to slide on its belly, play with a toy car or a ball, or wave from its ice floe, one at random, then naps again. |
| Feed | [<img src="assets/gif/salmon.gif" width="120" alt="Salmon animation preview">](assets/salmon.png) | Hugs and eats a salmon bite by bite, licks its lips, waves, and glows with a heart and sparkles. |
| Break | [<img src="assets/gif/break-time.gif" width="120" alt="Break time animation preview">](assets/break-time.png) | The alarm clock rings: time for a break. |

An unknown or missing state is idle. The first four strips are
260 x 260 px (1:1) per frame, cut from the 2 x 2 clip `art/states.mp4` (not kept in the repository; one
state per quarter) by `tools/video_states.py`: it cuts each seal out of the blue
grid backdrop, trims it to a seamless loop, crops it square, and writes the strip
and a 300 x 300 GIF (`assets/gif/<state>.gif`). The clapping seal is saved as `happy`
(it also makes the program's icon). Asking loops only the
part where the seal waves its right flipper; the clip's left-flipper wave draws a
third flipper. The swimmer's tail, lost for two frames mid-spin, is put back
from a neighbouring frame. The program keeps the strips as palette indices (1 byte a
pixel) and scales the current frame to the seal's size as it draws it, keeping
the frame's aspect.

The other strips (`thinking`, `active`, `error`, and the idle poses) are cut the same
way from a 4 x 3 clip of sticker poses by `tools/video_poses.py <clip.mp4>`, which also
writes `assets/gif/<pose>.gif`. Thinking keeps the seal alone: the "?" and zZ marks
floating by its head are taken out, and where they touch or cross the head it is painted
over in the seal's own colors (its edge redrawn round, white rim then navy outline, and
strokes on the fur filled with fur). Error's tears are the backdrop's own blue, so only
bright blue counts as backdrop there and each tear's darker outline keeps it whole; a
flying drop whose outline breaks is filled back in.

The salmon has a 20 s clip of its own (`art/salmon.mp4`, not kept in the repository), one
seal on the grid, cut whole by `tools/clip_pose.py <clip.mp4> salmon 0.3` the same way:
the GIF runs as long as the clip, and the feed plays the strip once through. The first
10 strip frames (20 GIF frames), where the seal sits still before it eats, were then cut
off by hand, so the feed starts sooner; cut them again after re-running. The `0.3`
whitens the fur more than the other poses' 0.55. Bubbles drifting off across the grid
are left out; the heart and sparkles near the seal stay. `tools/sheet_pose.py` cuts a pose
drawn as a 4 x 4 sprite sheet instead.

- `assets/menu.png`: the menu buttons, drawn by `tools/render_ui.py`.

The scripts need Pillow; `video_states.py`, `video_poses.py`, `clip_pose.py` and
`sheet_pose.py` also need NumPy and SciPy, and the first three `ffmpeg` on `PATH`. Re-run
them after changing the art, then rebuild.

For the full behavior, input format, and persistence contract, see
[SPEC.md](SPEC.md). For how the code fits together (with diagrams), see
[docs/en/architecture.md](docs/en/architecture.md). For token use, hook timing, CPU and memory,
see [docs/en/technical.md](docs/en/technical.md).
