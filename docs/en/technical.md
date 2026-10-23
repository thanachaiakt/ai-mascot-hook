# Technical notes: cost and footprint

[ภาษาไทย](../th/technical.md)

Short answers to "what does Seal Pet cost me?" For how the modules connect, see
[architecture.md](architecture.md).

## Does it use Claude tokens?

**No.** The hooks add nothing to Claude's context and make no model or network calls.

- Each hook is one command, `seal-pet --set <state>` (see `src/hooks.rs`). It writes one
  word to a local file and exits.
- Claude Code adds a hook's **stdout** to the conversation only for some events
  (`UserPromptSubmit`, `SessionStart`). `--set` prints nothing: stdout is 0 bytes, so 0
  tokens are added.
- It exits with code 0 and prints nothing to stderr, so Claude never sees an error message
  either. An unknown state word exits with code 2 and still prints nothing.
- Seal Pet has no network code: no API key, no telemetry, no update check.

The only thing a hook costs is time: Claude Code waits for the command before going on.

## How long does each hook take?

| | macOS (Apple M2 Pro, measured) |
|---|---|
| One `seal-pet --set working` run | about **5 ms** (average of 200 runs) |
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
| `completed` / `idle` | napping, 86 frames | about 3–6% | 16–17 MB |
| `asking` | waving, 60 frames | about 7–11% | 17 MB |
| `loading` | clapping, 74 frames | about 7–10% | 17 MB |
| `working` | swimming, 95 frames | about 10–12% | 17–19 MB |

The figures move around between samples, so read them as rough ranges. Windows was not
measured. Memory is `footprint`'s physical footprint, with the status bubble showing; the
peak while switching states was 20 MB. Opening the status editor adds its window's memory
(about 25 MB more while it is open).

A plain AppKit app with one transparent window and nothing else already takes about 11 MB
on the same Mac, so that is the floor on macOS: most of what is left is AppKit, the
windows' backing stores and the bubble.

Where the cost comes from:

- **Animation:** every clip plays at 12 frames per second (83 ms per frame). Each frame is
  scaled to the seal's size and copied to the screen, so a bigger seal costs a little
  more.
- **Timer (macOS):** one `NSTimer` ticks 60 times a second and runs whatever is due: the
  next frame, the state poll, drag and menu handling. Most ticks do nothing.
- **Timers (Windows):** separate `SetTimer` timers for frames, the poll and the bubble, so
  nothing runs between them.
- **Status bubble:** while it moves (pops in, bounces, fades) it redraws every 15 ms,
  otherwise every 33 ms. Turning the bubble off saves this.
- **Memory:** `build.rs` cuts each strip in `assets/` into one small PNG per frame, and
  those are built into the exe (about 4 MB in all; the exe is about 4.8 MB). Embedded data
  is mapped from the file and not counted as the app's own memory. Only the frame on
  screen is unpacked, one byte a pixel (260 × 260, about 66 KB), and the buffers are
  reused from frame to frame. Unpacking a whole strip up front, as earlier versions did,
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
| `~/.claude/settings.json` | `--install-hooks` / `--remove-hooks` | only when you run them; the old file is saved as `settings.json.bak-<time>` first |

The data folder is `%LOCALAPPDATA%\seal-pet` on Windows and
`~/Library/Application Support/seal-pet` on macOS.

## Measure it yourself

```bash
# time a hook run (macOS / Linux shell)
time target/release/seal-pet --set working

# what the hook prints into Claude's context (expect 0)
target/release/seal-pet --set working | wc -c

# CPU and memory of the running seal on macOS
top -l 5 -s 2 -pid $(pgrep -f seal-pet | head -1) -stats cpu,mem
```

On Windows, use Task Manager's Details tab, or
`Get-Process seal-pet | Select-Object CPU, WorkingSet64` in PowerShell.
