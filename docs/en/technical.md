# Technical notes: cost and footprint

[ภาษาไทย](../th/technical.md)

Short answers to "what does ai-mascot-hook cost me?" For how the modules connect, see
[architecture.md](architecture.md).

## Does it use Claude tokens?

**No.** The hooks add nothing to Claude's context and make no model or network calls.

- Each hook is one command, `ai-mascot-hook --set <state>` (see `src/hooks.rs`). It writes one
  word to a local file and exits.
- Claude Code adds a hook's **stdout** to the conversation only for some events
  (`UserPromptSubmit`, `SessionStart`). `--set` prints nothing: stdout is 0 bytes, so 0
  tokens are added.
- It exits with code 0 and prints nothing to stderr, so Claude never sees an error message
  either. An unknown state word exits with code 2 and still prints nothing.
- ai-mascot-hook has no network code: no API key, no telemetry, no update check.

The only thing a hook costs is time: Claude Code waits for the command before going on.

## How long does each hook take?

| | macOS (Apple M2 Pro, measured) |
|---|---|
| One `ai-mascot-hook --set working` run | about **5 ms** (average of 200 runs) |
| Peak memory of that run | about 8.5 MB, freed when it exits |
| Data written | the state word (4–9 bytes), to a temporary file that is then renamed over `state` |

`PreToolUse` and `PostToolUse` run on every tool call, so each tool call costs about
2 × 5 ms. Windows was not measured; starting a process is slower there than on macOS, but
the work is the same. Each hook has a 10 s timeout, so a hook that hangs cannot stall
Claude for long.

## How much CPU and memory does the running seal use?

Measured on macOS (Apple M2 Pro) with `top`, sampling every 2 s:

| State | Clip | CPU (one core) | Memory |
|---|---|---|---|
| `idle` (napping) | napping, 86 frames | about 3–6% | 16–17 MB |
| `asking` | waving, 60 frames | about 7–11% | 17 MB |
| `loading` | clapping (now only its first 6 s, then thinking), 74 frames | about 7–10% | 17 MB |
| `working` | swimming, 95 frames | about 10–12% | 17–19 MB |

The figures move around between samples, so read them as rough ranges. Windows was not
measured. They predate the sticker poses (thinking, sunglasses, crying and the idle
poses), which were not measured; they play at the same rate from strips of 26–230 frames
the same size, so they should cost about the same. Memory is `footprint`'s physical footprint, with the status bubble showing; the
peak while switching states was 20 MB. Opening the status editor adds its window's memory
(about 25 MB more while it is open).

A later round of tuning on macOS (the timer, color space, Core Animation and run cache
changes below) was measured as the CPU time the process used over 20 s, once the clip had
played through, auto hide off, same Mac:

| Case | Before | After | Memory after |
|---|---|---|---|
| `working`, bubble showing | 14.5% | 0.75% | 16 MB |
| `working`, bubble off | 5.3% | 0.75% | 15 MB |
| `asking`, bubble showing | 12.7% | 0.85% | 16 MB |
| `loading`, bubble showing | 10.9% | 0.8% | 17 MB |
| `completed`, bubble showing | | 0.9% | 16 MB |
| `error`, bubble showing | | 0.7% | 15 MB |
| `idle` (napping), bubble off | 4.7% | 0.75% | 14 MB |

Memory went up about 2–3 MB for the run cache (see **Memory** below); that bought about a
third of what is left of the CPU. Opening the menu or the editor, dragging and resizing
cost more while they last.

A plain AppKit app with one transparent window and nothing else already takes about 11 MB
on the same Mac, so that is the floor on macOS: most of what is left is AppKit, the
windows' backing stores and the bubble.

Where the cost comes from:

- **Animation:** every clip plays at 12 frames per second (83 ms per frame). On Windows
  each frame is scaled to the seal's size on the CPU and copied to the screen, so a bigger
  seal costs a little more. On macOS the frame goes to a Core Animation layer at the size
  it is in its PNG and the GPU scales it, so the seal's size costs nothing; the image is
  made over a reused buffer, without a copy, and the window is only moved when it moves
  (AppKit keeps windows to whole points, so the frame asked for is whole points too).
- **Timer (macOS):** one `NSTimer` runs whatever is due: the next frame, the state poll,
  the bubble, a press turning into a drag. It sleeps until the next of these instead of
  ticking 60 times a second, so it wakes about 12 times a second with the bubble off.
  While the menu or the status editor is open it ticks 60 times a second, since they
  watch for losing the keyboard, which sends no message.
- **Color (macOS):** the canvases and the windows are both sRGB, so the window server
  matches colors to the display on the GPU. With the old device-RGB canvases, Core
  Animation converted every frame on the CPU, about a third of the seal's CPU time.
- **Timers (Windows):** separate `SetTimer` timers for frames, the poll and the bubble, so
  nothing runs between them.
- **Status bubble (macOS):** the box and its two round bubbles are each a Core Animation
  layer, drawn once when the words change. Popping in, bouncing and fading set the
  layers' scale every 15 ms; once settled, the float and the dot's ripple are endless
  Core Animation animations, so the bubble costs no CPU while it is up.
- **Status bubble (Windows):** while it moves (pops in, bounces, fades) it is redrawn every
  15 ms, otherwise every 33 ms, since it floats. Each frame scales its parts with
  fixed-point sums worked out once per row and column, skipping the clear space around
  them. Turning the bubble off saves this.
- **Memory:** `build.rs` cuts each strip in `assets/` into one small PNG per frame, and
  those go into `ai-mascot-hook.frames` beside the exe (about 12.5 MB for the 12 strips;
  the exe is under 1 MB). The seal maps that file, so it is read from disk as needed and
  not counted as the app's own memory, as when the frames were in the exe. Only the frame on
  screen is unpacked, one byte a pixel (260 × 260, about 66 KB), and the buffers are
  reused from frame to frame. On macOS each frame, once drawn, is also kept as runs of one
  palette index (about 2–3 MB a clip), so the PNG is unpacked only the first time round
  the clip: the runs unpack several times faster, and that was about a third of the CPU
  left. Unpacking a whole strip up front, as earlier versions did,
  took 6–8 MB per clip and left about 25 MB of freed buffers that macOS's malloc kept; on
  a state change the Mac version now also hands freed memory back to the system
  (`malloc_zone_pressure_relief`).

## How does it read the state? Does it poll the disk?

- The seal checks the state file every **400 ms** (`POLL_MS` in `src/seal.rs`). A check
  reads the file's modified time and reads its few bytes only when that time has changed.
  The OS answers from its file cache, so the disk itself is barely touched.
- On Windows it also looks for copies of the state file in packaged (MSIX) app folders,
  such as the Claude desktop app's, at most once every 5 s (`RESCAN`).
- Because of the poll, the seal changes animation up to 400 ms after a hook runs.

## Files it uses

| File | Written by | When |
|---|---|---|
| `state` | the hooks | every hook event |
| `placement` | the seal | after you move or resize it, or turn the bubble on or off |
| `status.txt` | the status editor | when you press Save |
| `~/.claude/settings.json`, and for the other agents their own hook files (`~/.cursor/hooks.json`, `~/.copilot/hooks/ai-mascot-hook.json`, `~/.gemini/settings.json`, `~/.codex/hooks.json`, `~/.qwen/settings.json`, `~/.codeium/windsurf/hooks.json`, `~/.kiro/hooks/ai-mascot-hook.json`) | `--install-hooks` / `--remove-hooks` (`--agent`) | only when you run them; an old file is saved as `<name>.bak-<time>` first |

The data folder is `%LOCALAPPDATA%\ai-mascot-hook` on Windows and
`~/Library/Application Support/ai-mascot-hook` on macOS.

## Measure it yourself

```bash
# time a hook run (macOS / Linux shell)
time target/release/ai-mascot-hook --set working

# what the hook prints into Claude's context (expect 0)
target/release/ai-mascot-hook --set working | wc -c

# CPU and memory of the running seal on macOS
top -l 5 -s 2 -pid $(pgrep -f ai-mascot-hook | head -1) -stats cpu,mem
```

On Windows, use Task Manager's Details tab, or
`Get-Process ai-mascot-hook | Select-Object CPU, WorkingSet64` in PowerShell.
