# How the code fits together

[ภาษาไทย](../th/architecture.md)

ai-mascot-hook is one Rust binary with two jobs:

- **The hook command.** `ai-mascot-hook --set <state>` writes one word to a file and exits at once.
- **The seal.** With no arguments, it opens the windows, watches that file, and plays the
  matching animation and bubble.

Most of the code is shared. Only the windowing layer is written separately for Windows
and for macOS.

## 1. From Claude Code to the screen

```mermaid
flowchart LR
    CC["Claude Code<br/>(CLI or desktop app)"] -- "hook event<br/>UserPromptSubmit, PreToolUse, Stop, …" --> H["ai-mascot-hook --set &lt;state&gt;"]
    H -- "status::write_state<br/>temp file + rename" --> F[("state file<br/>%LOCALAPPDATA%\ai-mascot-hook\state<br/>~/Library/Application Support/ai-mascot-hook/state")]
    F -- "polled every 400 ms<br/>store::read_state" --> S["running seal<br/>win::App / mac::App"]
    S -- "seal::act_for, Turns" --> A["animation strip"]
    S -- "bubble::Bubble" --> B["status bubble"]
    A --> W["seal window"]
    B --> BW["bubble window"]
```

| Claude Code event | State | Animation |
|---|---|---|
| `SessionStart`, `SessionEnd` | `idle` | napping; every minute (`idle.every`) sliding, a toy car, a ball or waving |
| `UserPromptSubmit` | `loading` | clapping, then sitting and thinking |
| `PreToolUse`, `PostToolUse` | `working` | swimming in a hard hat |
| `PermissionRequest`, `Notification` | `asking` | waving on an ice floe |
| `Stop` | `completed` | showing off in sunglasses |
| `StopFailure` | `error` | crying |

A state's `Act` lists its clips (and, for idle, a `rest` clip it naps on for `REST_MS`
between them). `Turns` (in `seal.rs`) keeps which one is playing; each time the clip
comes back to its first frame, the platform asks `Turns::looped` whether another takes
over, and if so unpacks it. A `CutIn` plays over the act and then lets it go on: `FEED`
from the menu's Feed button, `BREAK` when the poll finds the clock at a break time
(`status.breaks`, checked with `seal::minute_of_day`); a click on the seal ends a break.

The Claude desktop app is a packaged (MSIX) app. Windows redirects the state file that its
hooks write into `%LOCALAPPDATA%\Packages\<app>\LocalCache\Local\ai-mascot-hook\state`.
`store::state_files` also finds those copies, and `store::read_state` follows whichever
file was written most recently.

## 2. Module map

```mermaid
flowchart TB
    main["main.rs<br/>entry point"]

    subgraph shared["Shared"]
        seal["seal.rs<br/>sizes, timings, clips,<br/>acts, feed and break scenes, clock"]
        store["store.rs<br/>data folder, state file, placement"]
        status["status.rs<br/>words, colors, auto hide,<br/>break and feed settings, state parsing"]
        phrases["phrases.rs<br/>English and Thai message sets,<br/>10 lines a state"]
        bubble["bubble.rs<br/>bubble layout, drawing, animation"]
        menu["menu.rs<br/>icon menu layout, drawing, animation"]
        editor["editor.rs<br/>settings editor layout (two tabs),<br/>painting, clicks"]
        hooks["hooks.rs<br/>hook files of each agent"]
        anim["anim.rs<br/>easing"]
        gfx["gfx/mod.rs<br/>Canvas, shapes, Strip"]
    end

    subgraph backends["Per platform: gfx"]
        gwin["gfx/win.rs<br/>DIB + GDI text"]
        gmac["gfx/mac.rs<br/>CGBitmapContext + AppKit text"]
    end

    subgraph win["Windows: win/"]
        wmod["win/mod.rs<br/>App, seal window, pointer,<br/>timers, message loop"]
        wmenu["win/menu.rs<br/>icon menu window"]
        wed["win/editor.rs<br/>editor window, EDIT boxes,<br/>color dialog"]
    end

    subgraph mac["macOS: mac/"]
        mmod["mac/mod.rs<br/>App, NSPanels, SealView,<br/>NSTimer"]
        mmenu["mac/menu.rs<br/>icon menu panel"]
        med["mac/editor.rs<br/>editor panel, NSTextFields,<br/>NSColorPanel"]
    end

    main --> hooks
    main --> status
    main --> store
    main -- "windows" --> wmod
    main -- "macos" --> mmod

    wmod --> wmenu
    wmod --> wed
    mmod --> mmenu
    mmod --> med
    wmenu & mmenu --> menu
    wed & med --> editor
    wmod & mmod --> bubble
    wmod & mmod --> seal
    wmod & mmod --> store
    wmod & mmod --> status
    wmod & mmod --> gfx

    bubble --> anim
    bubble --> gfx
    menu & editor --> gfx
    editor --> status
    status & editor --> phrases
    phrases --> seal
    bubble --> seal
    store --> seal
    store --> status
    seal --> gfx
    status --> gfx
    gfx -. "cfg(windows)" .-> gwin
    gfx -. "cfg(macos)" .-> gmac
```

- **Shared code** has no platform calls, apart from `gfx`, which picks its back end at
  compile time.
- **The `win` and `mac` apps** do the same things in the same order. They are written
  separately because each uses its platform's own windows, events and timers.

## 3. Starting up

```mermaid
flowchart TD
    start(["ai-mascot-hook"]) --> args{"first argument"}
    args -- "--set state" --> write["status::write_state"] --> exit0(["exit"])
    args -- "--install-hooks / --remove-hooks" --> hk["hooks::run<br/>back up, drop old ai-mascot-hook hooks,<br/>add the hooks of each --agent"] --> exit1(["exit"])
    args -- "none" --> plat{"platform"}
    plat -- Windows --> wrun["win::run"]
    plat -- macOS --> mrun["mac::run"]
    wrun & mrun --> one{"already running?<br/>(mutex / socket)"}
    one -- yes --> quit(["exit"])
    one -- no --> init["create windows<br/>read_state, read_placement,<br/>StatusText::load"]
    init --> loop["event loop + timers"]
```

## 4. The running seal

Both apps keep one `App` in a thread-local and reach it through `with(|app| ...)`.
Timers and pointer events call into it:

```mermaid
flowchart LR
    subgraph sources["Time and input"]
        t1["frame timer<br/>(clip interval, 83 ms)"]
        t2["poll timer<br/>400 ms"]
        t3["bubble timer<br/>15 ms moving / 33 ms floating"]
        t4["auto-hide timer"]
        t5["hold timer<br/>220 ms"]
        mouse["mouse:<br/>down, move, up, wheel,<br/>right click"]
    end

    t1 --> fr["next frame<br/>draw_mascot"]
    t2 --> poll["poll<br/>read_state"]
    poll -- "state changed" --> sync["sync_clip<br/>(load new strip)"] --> rah["restart_autohide"]
    rah --> ub["update_bubble"]
    t3 --> bf["bubble_frame"]
    t4 --> hide["auto_hidden = true"] --> ub
    t5 --> drag["start drag"]
    mouse --> press["press_down / press_move / press_up"]
    press -- "hold or move > 4 px" --> drag --> mv["move_to + save placement"]
    press -- "click" --> menu["icon menu"]
    press -- "resize mode" --> rs["set_height + save"]
    menu --> act["feed / resize /<br/>settings / quit"]
```

On Windows each source is a Win32 timer (`TIMER_FRAME`, `TIMER_POLL`, `TIMER_BUBBLE`,
`TIMER_AUTOHIDE`, `TIMER_HOLD`, plus `TIMER_MENU` for the menu animation). On macOS one
60 Hz `NSTimer` calls `App::tick`, which checks when each of these is due.

## 5. Drawing the seal

```mermaid
flowchart LR
    png["assets/*.png<br/>frame strip (palette PNG)"] -- "build.rs<br/>one PNG per frame" --> frames["$OUT_DIR/*.frames<br/>(in the exe)"]
    frames -- "Strip::decode" --> strip["Strip<br/>compressed frames"]
    strip -- "Strip::zoomed<br/>(working: 1.15x, frames grow)" --> strip2["Strip + pad"]
    strip2 -- "Strip::draw<br/>unpack one frame, scale to seal size" --> mc["Canvas mc"]
    mc -- "resize mode:<br/>frame + handle" --> mc
    mc -- "Windows: UpdateLayeredWindow<br/>macOS: CGImage on the layer" --> win["seal window<br/>(box + transparent overhang)"]
```

The seal's **box** (x, y, height) is what gets saved, dragged and resized, and what the
bubble is placed against. For the zoomed `working` clip, the **window** reaches past the
box (`overhang`). Pointer positions are mapped back into the box (`in_box`).

## 6. The status bubble

`bubble::Bubble` does everything except show the result:

```mermaid
flowchart TD
    app["App::update_bubble<br/>picks text, dot, color, key<br/>(editor preview, resize hint,<br/>break or feed message, or state)"]
    app -- "visible" --> show["Bubble::show<br/>measure text, place above or below the seal,<br/>clamp to the work area, avoid the size readout"]
    app -- "hidden" --> hidebox["Bubble::hide"]
    show --> drawn["drawn: box in pixel-art sticker style<br/>+ smooth accent dot + text"]
    show --> tails["tails: two round bubbles,<br/>each on its own canvas"]
    frame["App::bubble_frame (timer)"] --> bfr["Bubble::frame"]
    hidebox --> bfr
    bfr --> comp["shown = drawn + tails composited<br/>with pop-in, bounce, float, ripple"]
    comp -- "Frame::Shown {x, y, alpha, moving}" --> present["platform shows the 'shown' canvas<br/>and sets the next frame's timer"]
    bfr -- "Frame::Hidden" --> off["platform hides the window,<br/>stops the timer"]
```

Animation inputs: `appear` (0 to 1: pop in over 0.5 s, fade out over 0.18 s), `pop` (the
bounce when `key` changes), and a clock for the 2.8 s float and the dot's ripple. Easing
comes from `anim.rs`.

## 7. Files on disk

```mermaid
flowchart LR
    hooks2["hooks::run"] -- "edits" --> settings[("hook files<br/>~/.claude/settings.json, ...")]
    setcmd["ai-mascot-hook --set"] -- "writes" --> state[("state")]
    state -- "read" --> app2["App::poll"]
    app2 -- "save_placement" --> placement[("placement<br/>x y height show_status")]
    placement -- "read_placement at start" --> app2
    editor2["editor Save"] -- "StatusText::save" --> statustxt[("status.txt<br/>words, color.&lt;state&gt;, autohide, idle.every,<br/>break.times, break.message, feed.message")]
    statustxt -- "StatusText::load" --> app2
```

All of these files except `settings.json` live in the data folder (`store::data_dir`):
`%LOCALAPPDATA%\ai-mascot-hook` on Windows, `~/Library/Application Support/ai-mascot-hook` on macOS.

## Where to change things

| To change | Look in |
|---|---|
| Which event sets which state | `hooks.rs` (the `Plan` of each agent), `tools/install-hooks.ps1` |
| A state's animations, speed or size | `seal.rs` (`Clip`, `Act`, `act_for`) |
| The bubble's look or motion | `bubble.rs` |
| Default words and colors | `status.rs` (`STATES`, `DEFAULT_COLOR`, `DEFAULT_BREAK_MESSAGE`, `DEFAULT_FEED_MESSAGE`) |
| The English and Thai message sets | `phrases.rs` (`ENGLISH`, `THAI`, `SETS`) |
| The feed and break scenes, idle's nap length | `seal.rs` (`FEED`, `BREAK`, `REST_MS`, `TURN_MS`) |
| The icon menu's or the editor's look | `menu.rs` / `editor.rs` |
| Windows menu / editor windows | `win/menu.rs` / `win/editor.rs` |
| Mac windows and input | `mac/mod.rs`, `mac/menu.rs`, `mac/editor.rs` |
| Text rendering or putting pixels on screen | `gfx/win.rs`, `gfx/mac.rs` |
