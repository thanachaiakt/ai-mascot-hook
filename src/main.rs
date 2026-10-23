//! Seal Pet: a small seal that floats on the desktop and acts out what Claude Code is doing.
//!
//! Claude Code hooks write the current phase (loading, working, asking, completed, idle)
//! to `%LOCALAPPDATA%\seal-pet\state`; this program polls that file and plays the matching
//! animation in a borderless, per-pixel transparent, always-on-top window.
//!
//! A short click opens a menu of three round icon buttons (resize, settings, close); settings
//! unfolds a second row (show status, edit status). Pressing and holding drags the seal. In resize mode the seal grows and
//! shrinks between 32 and 512 px with the mouse wheel or its corner handle. Placement,
//! size and the status bubble are remembered in `%LOCALAPPDATA%\seal-pet\placement`, the
//! bubble's words and color in `status.txt` beside it.
#![windows_subsystem = "windows"]

mod gfx;
mod status;

use std::cell::{Cell, RefCell};
use std::fs;
use std::path::PathBuf;
use std::ptr::{null, null_mut};
use std::time::{Duration, Instant, SystemTime};

use gfx::*;
use status::*;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Dwm::*;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::CreateMutexW;
use windows_sys::Win32::UI::Controls::Dialogs::*;
use windows_sys::Win32::UI::HiDpi::*;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

/// The frames' own size: every size the seal takes keeps this aspect.
const FRAME_W: i32 = 260;
const FRAME_H: i32 = 260;
/// The seal fits a 32 x 32 box at its smallest and a 512 x 512 box at its largest.
const MIN_H: i32 = 32;
const MAX_H: i32 = 512;

const HOLD_MS: u32 = 220;
const DRAG_SLOP: i32 = 4;
const POLL_MS: u32 = 400;
/// How often to look for state files redirected into packaged apps' folders.
const RESCAN: Duration = Duration::from_secs(5);
const MENU_STEPS: u32 = 6;

const TIMER_FRAME: usize = 1;
const TIMER_POLL: usize = 2;
const TIMER_HOLD: usize = 3;
const TIMER_MENU: usize = 4;
const TIMER_AUTOHIDE: usize = 5;
const TIMER_BUBBLE: usize = 6;

// not in the windows-sys features this crate pulls in
const WM_MOUSELEAVE: u32 = 0x02A3;
const EM_LIMITTEXT: u32 = 0x00C5;
const EN_CHANGE: u32 = 0x0300;
const EN_SETFOCUS: u32 = 0x0100;
const EN_KILLFOCUS: u32 = 0x0200;
const EM_SETSEL: u32 = 0x00B1;
const WM_APP_ACT: u32 = WM_APP + 1;
const WM_APP_CLOSE_MENU: u32 = WM_APP + 2;
/// focus the editor's text box `wparam`, once the app is free to color it as it repaints
const WM_APP_FOCUS: u32 = WM_APP + 3;

// what a menu button does, posted to the seal once the menu has closed
const ACT_STATUS: usize = 0;
const ACT_EDIT: usize = 1;
const ACT_RESIZE: usize = 2;
const ACT_QUIT: usize = 3;

/// The menu's buttons: the main row, left to right, then the settings row.
#[derive(Clone, Copy, PartialEq)]
enum Item {
    Resize,
    Settings,
    Close,
    Status,
    Edit,
}
const MAIN_ROW: [Item; 3] = [Item::Resize, Item::Settings, Item::Close];
const SETTINGS_ROW: [Item; 2] = [Item::Status, Item::Edit];

/// Where everything in the menu window sits right now.
struct MenuLayout {
    w: i32,
    h: i32,
    btn: i32,
    /// each visible button and its center
    buttons: Vec<(Item, i32, i32)>,
    /// top of the label strip under the main row, and under the settings row
    main_label: i32,
    settings_label: i32,
}

// the editor's controls
const ID_EDIT0: i32 = 100;
/// What the seconds box starts at when auto hide is first ticked.
const DEFAULT_AUTOHIDE: u32 = 5;

const NAVY: Rgba = Rgba(24, 52, 110, 255);
const BLUE: Rgba = Rgba(74, 152, 216, 255);
const WHITE: Rgba = Rgba(255, 255, 255, 255);
// the editor's quiet palette
const INK: Rgba = Rgba(31, 35, 40, 255);
const MUTED: Rgba = Rgba(107, 114, 128, 255);
const FAINT: Rgba = Rgba(156, 163, 175, 255);
const FIELD: Rgba = Rgba(243, 245, 248, 255);
const FIELD_HOVER: Rgba = Rgba(232, 235, 240, 255);
const LINE: Rgba = Rgba(226, 230, 236, 255);
const TRACK_OFF: Rgba = Rgba(206, 212, 220, 255);
const FOCUS_RING: Rgba = Rgba(74, 152, 216, 60);
const NAVY_HOVER: Rgba = Rgba(38, 74, 148, 255);
// the bubble's pixel-art palette, taken from the seal's
const OUTLINE: Rgba = Rgba(52, 60, 82, 255);
const ICE: Rgba = Rgba(214, 228, 244, 255);

/// One animation: its frame strip, frame count and milliseconds per frame.
struct Clip {
    png: &'static [u8],
    frames: usize,
    interval: u32,
    /// enlargement about a point of the frame, so the seal stands as big, and where, it
    /// does in the other clips
    zoom: f32,
    pivot: (f32, f32),
}

impl Clip {
    fn strip(&self) -> Strip {
        Strip::decode(self.png, self.frames).zoomed(self.zoom, self.pivot)
    }
}

const LOADING: Clip = Clip { png: include_bytes!("../assets/loading.png"), frames: 74, interval: 83, zoom: 1.0, pivot: (0.0, 0.0) };
// the swimming seal is drawn floating mid-frame and smaller: bring it to the others' size,
// its bottom down to where they stand (the window grows past the seal's box to keep the
// whole swimmer when it reaches an edge)
const WORKING: Clip = Clip { png: include_bytes!("../assets/working.png"), frames: 95, interval: 83, zoom: 1.15, pivot: (127.0, 37.0) };
const ASKING: Clip = Clip { png: include_bytes!("../assets/asking.png"), frames: 60, interval: 83, zoom: 1.0, pivot: (0.0, 0.0) };
const COMPLETED: Clip = Clip { png: include_bytes!("../assets/completed.png"), frames: 86, interval: 83, zoom: 1.0, pivot: (0.0, 0.0) };
const MENU_PNG: &[u8] = include_bytes!("../assets/menu.png");

fn clip_for(state: &str) -> &'static Clip {
    match state {
        "loading" => &LOADING,
        "working" => &WORKING,
        "asking" => &ASKING,
        // completed, idle, or nothing written yet: the seal naps
        _ => &COMPLETED,
    }
}

enum Press {
    None,
    /// down, not yet a drag: a release now is a click
    Pending { at: POINT, grab: POINT },
    Drag { grab: POINT },
    Resize { at: POINT, h0: i32 },
}

/// The open status-text editor: its window, its text boxes and the choices being made.
struct Editor {
    hwnd: HWND,
    edits: Vec<HWND>,
    /// each state's text color as picked so far
    colors: [Rgba; 5],
    /// the state whose words were last focused: the one the swatches color, and the
    /// bubble previews
    selected: usize,
    show: bool,
    autohide: bool,
    seconds: u32,
    hover: Option<Hit>,
    /// the text box with the keyboard focus
    focus: Option<usize>,
    tracking: bool,
    canvas: Canvas,
    fonts: EditorFonts,
    /// the text boxes' background, the same as the field painted around them
    field_brush: HBRUSH,
}

struct EditorFonts {
    title: HFONT,
    label: HFONT,
    heading: HFONT,
    hint: HFONT,
    button: HFONT,
    edit: HFONT,
}

/// What in the editor takes a click.
#[derive(Clone, Copy, PartialEq)]
enum Hit {
    Close,
    /// a state's dot and name, which select it for editing and preview
    State(usize),
    Swatch(usize),
    Custom,
    Show,
    AutoHide,
    Less,
    More,
    Reset,
    Cancel,
    Save,
}

/// (left, top, right, bottom) in client pixels.
type Area = (f32, f32, f32, f32);

/// Where everything in the editor sits.
struct EditorLayout {
    w: i32,
    h: i32,
    header: f32,
    close: Area,
    /// the tops of the section headings
    messages: f32,
    colors: f32,
    bubble: f32,
    fields: [Area; 5],
    swatches: [Area; 8],
    custom: Area,
    show_row: Area,
    auto_row: Area,
    /// the two switches
    show: Area,
    autohide: Area,
    /// the seconds stepper, and its minus and plus ends
    seconds: Area,
    less: Area,
    more: Area,
    divider: f32,
    reset: Area,
    cancel: Area,
    save: Area,
}

fn editor_layout(ui: f32) -> EditorLayout {
    let s = |v: f32| (v * ui).round();
    let w = s(384.0);
    let pad = s(20.0);
    let right = w - pad;
    let header = s(58.0);
    let close = (right - s(30.0), s(14.0), right, s(44.0));
    let messages = header;
    let field_h = s(38.0);
    let fields = std::array::from_fn(|i| {
        let y = messages + s(26.0) + i as f32 * (field_h + s(8.0));
        (pad + s(100.0), y, right, y + field_h)
    });
    let colors = fields[4].3 + s(22.0);
    let (d, step) = (s(26.0), s(35.0));
    let sy = colors + s(30.0);
    let swatches = std::array::from_fn(|i| {
        let x = pad + s(4.0) + i as f32 * step;
        (x, sy, x + d, sy + d)
    });
    let cx = pad + s(4.0) + 8.0 * step;
    let custom = (cx, sy, cx + d, sy + d);
    let bubble = sy + d + s(24.0);
    let row_h = s(42.0);
    let show_row = (pad, bubble + s(22.0), right, bubble + s(22.0) + row_h);
    let auto_row = (pad, show_row.3, right, show_row.3 + row_h);
    let switch = |r: Area| {
        let cy = (r.1 + r.3) / 2.0;
        (right - s(42.0), cy - s(12.0), right, cy + s(12.0))
    };
    let (show, autohide) = (switch(show_row), switch(auto_row));
    let cy = (auto_row.1 + auto_row.3) / 2.0;
    let seconds = (autohide.0 - s(16.0) - s(112.0), cy - s(16.0), autohide.0 - s(16.0), cy + s(16.0));
    let less = (seconds.0, seconds.1, seconds.0 + s(36.0), seconds.3);
    let more = (seconds.2 - s(36.0), seconds.1, seconds.2, seconds.3);
    let divider = auto_row.3 + s(16.0);
    let (by, bh) = (divider + s(16.0), s(38.0));
    let save = (right - s(96.0), by, right, by + bh);
    let cancel = (save.0 - s(10.0) - s(96.0), by, save.0 - s(10.0), by + bh);
    let reset = (pad - s(12.0), by, pad + s(64.0), by + bh);
    EditorLayout {
        w: w as i32,
        h: (by + bh + pad) as i32,
        header,
        close,
        messages,
        colors,
        bubble,
        fields,
        swatches,
        custom,
        show_row,
        auto_row,
        show,
        autohide,
        seconds,
        less,
        more,
        divider,
        reset,
        cancel,
        save,
    }
}

fn inside(r: Area, x: f32, y: f32) -> bool {
    x >= r.0 && x < r.2 && y >= r.1 && y < r.3
}

fn grow(r: Area, by: f32) -> Area {
    (r.0 - by, r.1 - by, r.2 + by, r.3 + by)
}

fn rect(r: Area) -> RECT {
    RECT { left: r.0 as i32, top: r.1 as i32, right: r.2 as i32, bottom: r.3 as i32 }
}

/// A filled rounded rectangle.
fn pill(c: &mut Canvas, r: Area, radius: f32, color: Rgba) {
    c.fill_sdf(grow(r, 1.0), color, 1.0, round_rect(r.0, r.1, r.2, r.3, radius));
}

/// A plus centered in `r`, or a cross when `turned`: arms `k` long, `w` thick.
fn cross(c: &mut Canvas, r: Area, k: f32, w: f32, color: Rgba, turned: bool) {
    let (cx, cy) = ((r.0 + r.2) / 2.0, (r.1 + r.3) / 2.0);
    let (a, b) = if turned { ((k, k), (k, -k)) } else { ((k, 0.0), (0.0, k)) };
    c.fill_sdf(r, color, 1.0, segment((cx - a.0, cy - a.1), (cx + a.0, cy + a.1), w));
    c.fill_sdf(r, color, 1.0, segment((cx - b.0, cy - b.1), (cx + b.0, cy + b.1), w));
}

/// An on/off switch.
fn toggle(c: &mut Canvas, r: Area, on: bool) {
    let h = r.3 - r.1;
    pill(c, r, h / 2.0, if on { BLUE } else { TRACK_OFF });
    let knob = h / 2.0 - h * 0.14;
    let cx = if on { r.2 - h / 2.0 } else { r.0 + h / 2.0 };
    let cy = (r.1 + r.3) / 2.0;
    c.fill_sdf(grow(r, 2.0), Rgba(16, 32, 70, 40), 2.5, circle(cx, cy + 0.8, knob));
    c.fill_sdf(r, WHITE, 1.0, circle(cx, cy, knob));
}

/// How the bubble moves: it pops up out of the seal, fades away, bounces when the state
/// changes, and, for the busy states, sends a ripple out of its dot.
struct BubbleAnim {
    /// whether it should be up
    shown: bool,
    /// 0 gone .. 1 fully up
    appear: f32,
    /// 0 a bounce just began .. 1 settled
    pop: f32,
    /// what the bubble is about (a state, or resizing); a new one bounces
    key: String,
    /// where the full-size drawing sits on screen, and the point of the box nearest the
    /// seal, which it grows out of
    at: (i32, i32),
    anchor: (f32, f32),
    /// where the tail's two round bubbles (the big one first) sit in the drawing: their
    /// drawings' top-left, and the distance from there to their centers
    tail: [(f32, f32, f32); 2],
    /// the dot: center, radius, color, and seconds per ripple (0 for none)
    dot: Option<(f32, f32, f32, Rgba, f32)>,
    last: Instant,
    clock: Instant,
}

impl BubbleAnim {
    fn new() -> BubbleAnim {
        let now = Instant::now();
        BubbleAnim { shown: false, appear: 0.0, pop: 1.0, key: String::new(), at: (0, 0), anchor: (0.0, 0.0), tail: [(0.0, 0.0, 0.0); 2], dot: None, last: now, clock: now }
    }
}

fn ease_out_cubic(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}

/// How far along `t` (0..1) is through the stretch from `start` lasting `len`.
fn stage(t: f32, start: f32, len: f32) -> f32 {
    ((t - start) / len).clamp(0.0, 1.0)
}

/// `shape` drawn as the seal is drawn: pixel art with a dark outline, a white sticker edge
/// outside that, and a band of icy shade along the bottom.
fn sticker(c: &mut Canvas, area: Area, shape: impl Fn(f32, f32) -> f32, px: f32) {
    let hard = 0.01;
    let shape = pixelated(shape, px);
    c.fill_sdf(area, WHITE, hard, |x, y| shape(x, y) - 2.5 * px);
    c.fill_sdf(area, OUTLINE, hard, |x, y| shape(x, y) - px);
    c.fill_sdf(area, ICE, hard, &shape);
    c.fill_sdf(area, WHITE, hard, |x, y| shape(x, y + 2.0 * px).max(shape(x, y) + px));
}

/// Overshoots a little before settling.
fn ease_out_back(t: f32) -> f32 {
    let c = 1.6;
    1.0 + (c + 1.0) * (t - 1.0).powi(3) + c * (t - 1.0).powi(2)
}

/// Seconds per ripple from the dot: the busy states pulse, asking the quickest.
fn ripple_period(state: &str) -> f32 {
    match state {
        "asking" => 1.2,
        "loading" | "working" => 1.8,
        _ => 0.0,
    }
}

struct App {
    mascot: HWND,
    bubble: HWND,
    /// the size readout under the seal while resizing
    size_tag: HWND,
    menu: HWND,
    mc: Canvas,
    /// the bubble as drawn, at full size
    bc: Canvas,
    /// the bubble as shown this frame: `bc` scaled, with its ripple
    bf: Canvas,
    /// the tail's two round bubbles, each drawn once to move on its own
    tails: [Canvas; 2],
    bubble_anim: BubbleAnim,
    tc: Canvas,
    nc: Canvas,
    x: i32,
    y: i32,
    height: i32,
    strip: Strip,
    /// the animation playing: the state's, or while editing, the edited state's
    clip: &'static Clip,
    state: String,
    stamp: Option<SystemTime>,
    frame: usize,
    press: Press,
    resize_mode: bool,
    show_status: bool,
    /// the bubble has had its auto-hide time since the last state change
    auto_hidden: bool,
    status: StatusText,
    menu_open: bool,
    menu_hover: Option<Item>,
    menu_step: u32,
    settings_open: bool,
    settings_step: u32,
    menu_at: (i32, i32),
    menu_below: bool,
    tracking_leave: bool,
    ui: f32,
    font: HFONT,
    tag_font: HFONT,
    buttons: Vec<Image>,
    editor: Option<Editor>,
}

thread_local! {
    static APP: RefCell<Option<App>> = const { RefCell::new(None) };
    /// the editor's window, for the message loop's dialog keys (Tab, Enter, Esc)
    static EDITOR: Cell<HWND> = const { Cell::new(null_mut()) };
}

/// Run `f` on the app, unless it is already busy further up this call stack (a window
/// message sent from inside a handler), in which case the message takes its default.
fn with<R>(f: impl FnOnce(&mut App) -> R) -> Option<R> {
    APP.with(|a| a.try_borrow_mut().ok().and_then(|mut guard| guard.as_mut().map(f)))
}

fn data_dir() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
    base.join("seal-pet")
}

/// Every state file a hook may have written. Hooks run by a packaged (MSIX) app, such as
/// the Claude desktop app, have their `%LOCALAPPDATA%` writes redirected to
/// `%LOCALAPPDATA%\Packages\<app>\LocalCache\Local`, so those copies count too. The
/// packages are looked through again every few seconds, not on every poll.
fn state_files() -> Vec<PathBuf> {
    thread_local! {
        static FOUND: RefCell<Option<(Instant, Vec<PathBuf>)>> = const { RefCell::new(None) };
    }
    FOUND.with(|found| {
        let mut found = found.borrow_mut();
        if found.as_ref().map_or(true, |(at, _)| at.elapsed() >= RESCAN) {
            let mut files = vec![data_dir().join("state")];
            if let Some(base) = std::env::var_os("LOCALAPPDATA") {
                if let Ok(packages) = fs::read_dir(PathBuf::from(base).join("Packages")) {
                    for package in packages.flatten() {
                        let file = package.path().join("LocalCache").join("Local").join("seal-pet").join("state");
                        if file.is_file() {
                            files.push(file);
                        }
                    }
                }
            }
            *found = Some((Instant::now(), files));
        }
        found.as_ref().map(|(_, files)| files.clone()).unwrap_or_default()
    })
}

/// The state word and modified time of the newest state file. The word is `None` when
/// there is no file or it holds no word yet (caught mid-write), so the caller can look again.
fn read_state() -> (Option<String>, Option<SystemTime>) {
    let newest = state_files()
        .into_iter()
        .filter_map(|path| Some((fs::metadata(&path).and_then(|m| m.modified()).ok()?, path)))
        .max_by_key(|(stamp, _)| *stamp);
    let Some((stamp, path)) = newest else { return (None, None) };
    let stamp = Some(stamp);
    let state = fs::read(&path).ok().and_then(|bytes| parse_state(&bytes));
    (state, stamp)
}

/// `x y height show_status` from the last run; older runs saved a scale in place of the height.
fn read_placement(default_h: i32) -> Option<(i32, i32, i32, bool)> {
    let text = fs::read_to_string(data_dir().join("placement")).ok()?;
    let mut parts = text.split_whitespace();
    let x = parts.next()?.parse().ok()?;
    let y = parts.next()?.parse().ok()?;
    let size: f32 = parts.next()?.parse().ok()?;
    let height = if size <= 4.0 { (default_h as f32 * size).round() as i32 } else { size as i32 };
    let status = parts.next().map(|s| s == "1").unwrap_or(false);
    Some((x, y, height.clamp(MIN_H, MAX_H), status))
}

fn cursor_pos() -> POINT {
    let mut p = POINT { x: 0, y: 0 };
    unsafe { GetCursorPos(&mut p) };
    p
}

fn lparam_point(lp: LPARAM) -> POINT {
    POINT { x: (lp & 0xFFFF) as u16 as i16 as i32, y: ((lp >> 16) & 0xFFFF) as u16 as i16 as i32 }
}

fn work_area(x: i32, y: i32) -> RECT {
    unsafe {
        let monitor = MonitorFromPoint(POINT { x, y }, MONITOR_DEFAULTTONEAREST);
        let mut info: MONITORINFO = std::mem::zeroed();
        info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        GetMonitorInfoW(monitor, &mut info);
        info.rcWork
    }
}

fn colorref(c: Rgba) -> u32 {
    c.0 as u32 | (c.1 as u32) << 8 | (c.2 as u32) << 16
}

fn font(px: f32, weight: i32, face: &str) -> HFONT {
    unsafe { CreateFontW(-(px.round() as i32), 0, 0, 0, weight, 0, 0, 0, 1, 0, 0, 5, 0, wide(face).as_ptr()) }
}

impl App {
    fn size(&self) -> (i32, i32) {
        ((self.height as f32 * FRAME_W as f32 / FRAME_H as f32).round() as i32, self.height)
    }

    fn handle(&self) -> (f32, f32, f32) {
        let (w, h) = self.size();
        let r = (7.0 * self.ui).min(h as f32 / 5.0).max(4.0);
        (w as f32 - r - 2.0, h as f32 - r - 2.0, r)
    }

    fn near_handle(&self, client: POINT) -> bool {
        let (cx, cy, r) = self.handle();
        self.resize_mode && ((client.x as f32 - cx).powi(2) + (client.y as f32 - cy).powi(2)).sqrt() <= r + 5.0
    }

    // ---- the seal --------------------------------------------------------------

    /// The state the seal shows: the one being edited while the editor is open, so it
    /// can be previewed, otherwise the one the hooks set.
    fn shown_state(&self) -> &str {
        match &self.editor {
            Some(e) => STATES[e.selected].0,
            None => &self.state,
        }
    }

    /// Play the shown state's animation, if it is not playing already.
    fn sync_clip(&mut self) {
        let clip = clip_for(self.shown_state());
        if std::ptr::eq(clip, self.clip) {
            return;
        }
        self.clip = clip;
        self.strip = clip.strip();
        self.frame = 0;
        unsafe { SetTimer(self.mascot, TIMER_FRAME, clip.interval, None) };
        self.draw_mascot();
    }

    /// How far the seal's window reaches past its box (left, top, right, bottom), for a
    /// clip whose art grows past its frame. Everything else works in the box.
    fn overhang(&self) -> (i32, i32, i32, i32) {
        let (_, h) = self.size();
        let (l, t, r, b) = self.strip.pad;
        let k = h as f32 / (self.strip.fh - t - b) as f32;
        let at = |v: usize| (v as f32 * k).round() as i32;
        (at(l), at(t), at(r), at(b))
    }

    /// A point in the seal's window, as a point in its box.
    fn in_box(&self, mut client: POINT) -> POINT {
        let (l, t, _, _) = self.overhang();
        client.x -= l;
        client.y -= t;
        client
    }

    fn draw_mascot(&mut self) {
        let (w, h) = self.size();
        let (l, t, r, b) = self.overhang();
        self.mc.reset(w + l + r, h + t + b);
        self.strip.draw(self.frame, &mut self.mc);
        if self.resize_mode {
            let ui = self.ui;
            let (ox, oy) = (l as f32, t as f32);
            let (wf, hf) = (w as f32, h as f32);
            let frame = round_rect(ox + 1.0, oy + 1.0, ox + wf - 1.0, oy + hf - 1.0, 10.0 * ui);
            self.mc.fill_sdf((ox, oy, ox + wf, oy + hf), Rgba(74, 152, 216, 210), 1.0, move |x, y| frame(x, y).abs() - 0.9 * ui);
            let (cx, cy, r) = self.handle();
            let (cx, cy) = (cx + ox, cy + oy);
            self.mc.fill_sdf((cx - r - 2.0, cy - r - 2.0, cx + r + 2.0, cy + r + 2.0), BLUE, 1.0, circle(cx, cy, r));
            self.mc.fill_sdf((cx - r, cy - r, cx + r, cy + r), WHITE, 1.0, circle(cx, cy, r - 1.6 * ui));
        }
        self.present_mascot();
    }

    fn present_mascot(&self) {
        let (l, t, _, _) = self.overhang();
        let alpha = if matches!(self.press, Press::Drag { .. }) { 225 } else { 255 };
        self.mc.present(self.mascot, self.x - l, self.y - t, alpha);
    }

    fn move_to(&mut self, x: i32, y: i32) {
        self.x = x;
        self.y = y;
        self.present_mascot();
        self.update_bubble();
    }

    fn set_height(&mut self, h: i32) {
        let h = h.clamp(MIN_H, MAX_H);
        if h != self.height {
            self.height = h;
            self.draw_mascot();
            self.update_bubble();
        }
    }

    fn save(&self) {
        let _ = fs::create_dir_all(data_dir());
        let _ = fs::write(data_dir().join("placement"), format!("{} {} {} {}", self.x, self.y, self.height, self.show_status as u8));
    }

    /// Show the bubble again and, when auto hide is on, start counting down to hiding it.
    fn restart_autohide(&mut self) {
        self.auto_hidden = false;
        unsafe {
            if self.show_status && self.status.autohide > 0 {
                SetTimer(self.mascot, TIMER_AUTOHIDE, self.status.autohide * 1000, None);
            } else {
                KillTimer(self.mascot, TIMER_AUTOHIDE);
            }
        }
        self.update_bubble();
    }

    fn poll(&mut self) {
        let (state, stamp) = read_state();
        if stamp != self.stamp {
            // an empty read is a write still in progress: leave the stamp so the next poll retries
            let Some(state) = state else {
                if stamp.is_none() {
                    self.stamp = None;
                }
                return;
            };
            self.stamp = stamp;
            if state != self.state {
                self.state = state;
                self.sync_clip();
                self.restart_autohide();
            }
        }
    }

    // ---- pointer on the seal ---------------------------------------------------

    fn set_cursor(&self) {
        let mut p = cursor_pos();
        unsafe {
            ScreenToClient(self.mascot, &mut p);
            let p = self.in_box(p);
            let cursor = match self.press {
                Press::Drag { .. } => IDC_SIZEALL,
                _ if self.near_handle(p) => IDC_SIZENWSE,
                _ => IDC_HAND,
            };
            SetCursor(LoadCursorW(null_mut(), cursor));
        }
    }

    fn press_down(&mut self, client: POINT) {
        unsafe { SetCapture(self.mascot) };
        let at = cursor_pos();
        if self.near_handle(client) {
            self.press = Press::Resize { at, h0: self.height };
        } else {
            self.press = Press::Pending { at, grab: client };
            unsafe { SetTimer(self.mascot, TIMER_HOLD, HOLD_MS, None) };
        }
    }

    fn start_drag(&mut self, grab: POINT) {
        self.press = Press::Drag { grab };
        self.close_menu();
        unsafe { SetCursor(LoadCursorW(null_mut(), IDC_SIZEALL)) };
        self.draw_mascot();
    }

    fn press_move(&mut self) {
        let p = cursor_pos();
        match self.press {
            Press::Pending { at, grab } => {
                if (p.x - at.x).abs() > DRAG_SLOP || (p.y - at.y).abs() > DRAG_SLOP {
                    self.start_drag(grab);
                    self.move_to(p.x - grab.x, p.y - grab.y);
                }
            }
            Press::Drag { grab } => self.move_to(p.x - grab.x, p.y - grab.y),
            Press::Resize { at, h0 } => {
                let dx = ((p.x - at.x) as f32 * FRAME_H as f32 / FRAME_W as f32) as i32;
                let dy = p.y - at.y;
                self.set_height(h0 + if dx.abs() > dy.abs() { dx } else { dy });
            }
            Press::None => {}
        }
    }

    fn press_up(&mut self) {
        unsafe { KillTimer(self.mascot, TIMER_HOLD) };
        let press = std::mem::replace(&mut self.press, Press::None);
        unsafe { ReleaseCapture() };
        match press {
            Press::Pending { .. } => {
                if self.resize_mode {
                    self.resize_mode = false;
                    self.draw_mascot();
                    self.update_bubble();
                } else {
                    self.toggle_menu();
                }
            }
            Press::Drag { .. } => {
                self.draw_mascot();
                self.save();
            }
            Press::Resize { .. } => self.save(),
            Press::None => {}
        }
    }

    fn wheel(&mut self, delta: i16) {
        if self.resize_mode {
            let h = (self.height as f32 * 1.1f32.powf(delta as f32 / 120.0)).round() as i32;
            self.set_height(if h == self.height { h + delta.signum() as i32 } else { h });
            self.save();
        }
    }

    fn act(&mut self, action: usize) {
        match action {
            ACT_STATUS => {
                self.show_status = !self.show_status;
                self.restart_autohide();
                self.save();
            }
            ACT_EDIT => self.open_editor(),
            ACT_RESIZE => {
                self.resize_mode = true;
                self.draw_mascot();
                self.update_bubble();
            }
            ACT_QUIT => unsafe { PostQuitMessage(0) },
            _ => {}
        }
    }

    // ---- the status bubble -----------------------------------------------------


    /// The size readout: a small pill just below the frame (above it when the screen ends
    /// there). Returns the pill's screen rect, so the bubble can stack clear of it.
    fn update_size_tag(&mut self) -> Option<RECT> {
        if !self.resize_mode || self.menu_open {
            unsafe { ShowWindow(self.size_tag, SW_HIDE) };
            return None;
        }
        let ui = self.ui;
        let text = format!("{} px", self.height);
        let (tw, th) = self.tc.measure(self.tag_font, &text);
        let pad_x = (10.0 * ui) as i32;
        let pad_y = (3.0 * ui) as i32;
        let gap = (6.0 * ui) as i32;
        let margin = (8.0 * ui) as i32; // room for the soft shadow
        let (bw, bh) = (tw + 2 * pad_x, th + 2 * pad_y);
        let (cw, ch) = (bw + 2 * margin, bh + 2 * margin);
        let (mw, mh) = self.size();
        let work = work_area(self.x + mw / 2, self.y + mh);
        let mut left = self.x + mw / 2 - bw / 2;
        let mut top = self.y + mh + gap;
        if top + bh > work.bottom {
            top = self.y - gap - bh;
        }
        left = left.clamp(work.left, (work.right - bw).max(work.left));

        let (x0, y0, x1, y1) = (margin as f32, margin as f32, (margin + bw) as f32, (margin + bh) as f32);
        let body = round_rect(x0, y0, x1, y1, bh as f32 / 2.0);
        self.tc.reset(cw, ch);
        let all = (0.0, 0.0, cw as f32, ch as f32);
        let lift = 2.0 * ui;
        self.tc.fill_sdf(all, Rgba(16, 32, 70, 60), 6.0 * ui, |x, y| body(x, y - lift) + 1.0 * ui);
        self.tc.fill_sdf(all, BLUE, 1.0, &body);
        let rect = RECT { left: margin, top: margin, right: margin + bw, bottom: margin + bh };
        self.tc.text(self.tag_font, &text, rect, WHITE);
        self.tc.present(self.size_tag, left - margin, top - margin, 255);
        unsafe { ShowWindow(self.size_tag, SW_SHOWNOACTIVATE) };
        Some(RECT { left, top, right: left + bw, bottom: top + bh })
    }

    fn update_bubble(&mut self) {
        let tag = self.update_size_tag();
        let visible = ((self.show_status && !self.auto_hidden) || self.resize_mode || self.editor.is_some()) && !self.menu_open;
        if !visible {
            self.bubble_anim.shown = false;
            self.bubble_frame();
            return;
        }
        let key = if self.resize_mode { "resize".to_string() } else { self.shown_state().to_string() };
        let (text, dot) = if self.resize_mode {
            ("Scroll, Drag or Click".to_string(), None)
        } else if let Some(e) = &self.editor {
            // while editing, the bubble previews the state being edited, as typed and colored
            let i = e.selected;
            let mut buf = [0u16; 128];
            let n = unsafe { GetWindowTextW(e.edits[i], buf.as_mut_ptr(), buf.len() as i32) } as usize;
            let typed = String::from_utf16_lossy(&buf[..n]);
            (if typed.trim().is_empty() { STATES[i].2.to_string() } else { typed }, Some(accent(STATES[i].0)))
        } else {
            (self.status.text(&self.state).to_string(), Some(accent(&self.state)))
        };
        // a typed `\n` breaks the line
        let text = text.replace("\\n", "\n");
        let ink = match &self.editor {
            Some(e) => e.colors[e.selected],
            None => self.status.color(&self.state),
        };
        let ui = self.ui;
        let lines: Vec<&str> = text.split('\n').collect();
        let widths: Vec<i32> = lines.iter().map(|l| self.bc.measure(self.font, l).0).collect();
        let line_h = self.bc.measure(self.font, "Ag").1;
        let tw = widths.iter().copied().max().unwrap_or(0);
        let th = line_h * lines.len() as i32;
        let pad_x = (16.0 * ui) as i32;
        let pad_y = (11.0 * ui) as i32;
        let dot_r = 3.5 * ui;
        let dot_room = if dot.is_some() { (dot_r * 2.0 + 10.0 * ui) as i32 } else { 0 };
        let tail = (24.0 * ui) as i32;
        let halo_r = 7.0 * ui;
        let margin = (16.0 * ui) as i32; // room for the soft shadow
        let (bw, bh) = (tw + dot_room + 2 * pad_x, th + 2 * pad_y);
        let (cw, ch) = (bw + 2 * margin, bh + tail + 2 * margin);
        let (mw, mh) = self.size();
        let work = work_area(self.x + mw / 2, self.y);
        let mut bx = self.x + mw / 2 - cw / 2;
        // stack clear of the size readout when it sits on the same side
        let (top_edge, bottom_edge) = match tag {
            Some(t) if t.bottom <= self.y => (t.top, self.y + mh),
            Some(t) => (self.y, t.bottom),
            None => (self.y, self.y + mh),
        };
        let mut by = top_edge - ch + margin + 2;
        let below = by < work.top;
        if below {
            by = bottom_edge - margin - 2;
        }
        bx = bx.clamp(work.left, (work.right - cw).max(work.left));
        by = by.clamp(work.top, (work.bottom - ch).max(work.top));

        // the body, and a tail that points at the seal and melts into the body
        let (x0, x1) = (margin as f32, (margin + bw) as f32);
        let (y0, y1) = if below { ((margin + tail) as f32, (margin + tail + bh) as f32) } else { (margin as f32, (margin + bh) as f32) };
        // one line is a pill; more lines keep softly rounded corners
        let r = if lines.len() == 1 { bh as f32 / 2.0 } else { (bh as f32 / 2.0).min(22.0 * ui) };
        let t = tail as f32;
        let tip_x = ((self.x + mw / 2 - bx) as f32).clamp(x0 + r, x1 - r);
        // a thought-bubble tail: two little circles trailing off toward the seal
        let toward = if below { -1.0 } else { 1.0 };
        let edge = if below { y0 } else { y1 };
        let point_y = edge + toward * t;
        let (big_r, small_r) = (4.5 * ui, 2.6 * ui);
        let tail = [(tip_x - 3.0 * ui, edge + toward * (big_r + 4.5 * ui), big_r), (tip_x + 2.0 * ui, point_y - toward * small_r, small_r)];

        // the box, drawn like the seal on the seal's own pixel grid
        self.bc.reset(cw, ch);
        let px = self.art_pixel();
        sticker(&mut self.bc, (0.0, 0.0, cw as f32, ch as f32), round_rect(x0, y0, x1, y1, r), px);
        let mut ripple = None;
        if let Some(dot) = dot {
            let (dx, dy) = (x0 + pad_x as f32 + dot_r, y0 + pad_y as f32 + line_h as f32 / 2.0);
            let halo = Rgba(dot.0, dot.1, dot.2, 46);
            self.bc.fill_sdf((dx - halo_r - 2.0, dy - halo_r - 2.0, dx + halo_r + 2.0, dy + halo_r + 2.0), halo, 1.0, circle(dx, dy, halo_r));
            self.bc.fill_sdf((dx - dot_r - 2.0, dy - dot_r - 2.0, dx + dot_r + 2.0, dy + dot_r + 2.0), dot, 1.0, circle(dx, dy, dot_r));
            ripple = Some((dx, dy, dot_r, dot, ripple_period(&key)));
        }
        let text_left = x0 as i32 + pad_x + dot_room;
        for (i, (line, w)) in lines.iter().zip(&widths).enumerate() {
            let top = y0 as i32 + pad_y + i as i32 * line_h;
            let rect = RECT { left: text_left, top, right: text_left + w, bottom: top + line_h };
            self.bc.text(self.font, line, rect, ink);
        }
        // each round bubble of the tail on a canvas of its own, to float on its own
        let mut sprites = [(0.0, 0.0, 0.0); 2];
        for (i, &(cx, cy, r)) in tail.iter().enumerate() {
            let reach = (r + 4.0 * px).ceil();
            let side = (2.0 * reach) as i32;
            let c = &mut self.tails[i];
            c.reset(side, side);
            sticker(c, (0.0, 0.0, side as f32, side as f32), circle(reach + (cx - cx.floor()), reach + (cy - cy.floor()), r), px);
            sprites[i] = (cx.floor() - reach, cy.floor() - reach, reach);
        }
        let a = &mut self.bubble_anim;
        if a.appear > 0.0 && a.key != key {
            a.pop = 0.0;
        }
        if !a.shown && a.appear <= 0.0 {
            a.last = Instant::now();
        }
        a.shown = true;
        a.key = key;
        a.at = (bx, by);
        a.anchor = (tip_x, edge);
        a.tail = sprites;
        a.dot = ripple;
        self.bubble_frame();
    }

    /// The size of one pixel of the seal's art on screen: the bubble is drawn on the same grid.
    fn art_pixel(&self) -> f32 {
        (self.height as f32 / 75.0).round().max(1.0)
    }

    /// Show the bubble as it is at this moment of its animation. It comes out of the seal
    /// one bubble at a time, the small round one first, then the big one, then the box;
    /// once up, the box and the two round bubbles float gently, a beat apart, in steps of
    /// one art pixel.
    fn bubble_frame(&mut self) {
        let ui = self.ui;
        let now = Instant::now();
        let a = &mut self.bubble_anim;
        let dt = now.duration_since(a.last).as_secs_f32().min(0.05);
        a.last = now;
        a.appear = if a.shown { a.appear + dt / 0.5 } else { a.appear - dt / 0.18 }.clamp(0.0, 1.0);
        a.pop = (a.pop + dt / 0.45).min(1.0);
        if a.appear <= 0.0 {
            unsafe {
                KillTimer(self.mascot, TIMER_BUBBLE);
                ShowWindow(self.bubble, SW_HIDE);
            }
            return;
        }
        // on the way in, each part pops with a little overshoot; on the way out, all shrink a touch
        let (small, big, body) = if a.shown {
            let t = a.appear;
            (ease_out_back(stage(t, 0.0, 0.3)), ease_out_back(stage(t, 0.12, 0.35)), ease_out_back(stage(t, 0.28, 0.72)))
        } else {
            let s = 0.94 + 0.06 * a.appear;
            (s, s, s)
        };
        let bounce = 1.0 + 0.14 * (a.pop * std::f32::consts::PI).sin() * (1.0 - a.pop);
        let scale = body * bounce;
        let fade = if a.shown { ease_out_cubic((a.appear * 4.0).min(1.0)) } else { ease_out_cubic(a.appear) };
        let alpha = (fade * 255.0).round() as u8;

        // the float: a slow sway up and down, the round bubbles a beat behind the box
        let clock = now.duration_since(a.clock).as_secs_f32();
        let sway = 2.0 * ui;
        let wave = |lag: f32| ((clock / 2.8) * std::f32::consts::TAU - lag).sin() * sway;
        let (float_box, float_big, float_small) = (wave(0.0), wave(0.9), wave(1.8));

        self.bf.reset(self.bc.w, self.bc.h);
        self.bf.draw_canvas(&self.bc, (0.0, float_box), scale, a.anchor);
        for (i, (grow, float)) in [(big, float_big), (small, float_small)].into_iter().enumerate() {
            let (x, y, reach) = a.tail[i];
            self.bf.draw_canvas(&self.tails[i], (x, y + float), grow, (reach, reach));
        }
        if let Some((x, y, r, color, period)) = a.dot {
            if period > 0.0 {
                let phase = (clock / period).fract();
                let (ax, ay) = a.anchor;
                let (cx, cy) = (ax + (x - ax) * scale, ay + (y - ay) * scale + float_box);
                let radius = (r * 2.0 + r * 1.6 * ease_out_cubic(phase)) * scale;
                let w = 1.4 * ui;
                let ring = Rgba(color.0, color.1, color.2, ((1.0 - phase).powi(2) * 170.0) as u8);
                let reach = radius + w + 2.0;
                self.bf.fill_sdf((cx - reach, cy - reach, cx + reach, cy + reach), ring, 1.0, move |x, y| {
                    circle(cx, cy, radius)(x, y).abs() - w / 2.0
                });
            }
        }
        self.bf.present(self.bubble, a.at.0, a.at.1, alpha);
        let moving = a.appear < 1.0 || a.pop < 1.0;
        unsafe {
            if IsWindowVisible(self.bubble) == 0 {
                ShowWindow(self.bubble, SW_SHOWNOACTIVATE);
            }
            SetTimer(self.mascot, TIMER_BUBBLE, if moving { 15 } else { 33 }, None);
        }
    }

    // ---- the icon menu ---------------------------------------------------------

    /// The menu's layout: a main row (resize, settings, close) bowed into an arc, and, while
    /// settings is open, a row of its two buttons on the side away from the seal, centered
    /// over the settings button. Each row has a label strip on its seal side.
    fn menu_layout(&self) -> MenuLayout {
        let ui = self.ui;
        let btn = (48.0 * ui).round() as i32;
        let gap = (12.0 * ui) as i32;
        let pad = (10.0 * ui) as i32;
        let raise = (14.0 * ui) as i32;
        let label_gap = (8.0 * ui) as i32;
        let label = (28.0 * ui) as i32;
        let n = MAIN_ROW.len() as i32;
        let w = n * btn + (n - 1) * gap + 2 * pad;
        let main_h = raise + btn + label_gap + label;
        let settings_h = btn + label_gap + label;
        let (main_top, settings_top) = match (self.settings_open, self.menu_below) {
            (false, _) => (pad, pad),
            // above the seal: the settings row goes on top, farthest from the seal
            (true, false) => (pad + settings_h, pad),
            // below the seal: it goes underneath
            (true, true) => (pad, pad + main_h),
        };
        let h = 2 * pad + main_h + if self.settings_open { settings_h } else { 0 };

        let mut buttons = Vec::with_capacity(MAIN_ROW.len() + SETTINGS_ROW.len());
        for (i, &item) in MAIN_ROW.iter().enumerate() {
            // an arc: the outer buttons sit lower than the middle one
            let outer = i == 0 || i == MAIN_ROW.len() - 1;
            buttons.push((item, pad + i as i32 * (btn + gap) + btn / 2, main_top + btn / 2 + if outer { raise } else { 0 }));
        }
        if self.settings_open {
            let m = SETTINGS_ROW.len() as i32;
            let left = w / 2 - (m * btn + (m - 1) * gap) / 2;
            for (i, &item) in SETTINGS_ROW.iter().enumerate() {
                buttons.push((item, left + i as i32 * (btn + gap) + btn / 2, settings_top + btn / 2));
            }
        }
        MenuLayout { w, h, btn, buttons, main_label: main_top + raise + btn + label_gap, settings_label: settings_top + btn + label_gap }
    }

    fn menu_hit(&self, client: POINT) -> Option<Item> {
        let layout = self.menu_layout();
        let r = layout.btn as f32 * 0.42;
        layout
            .buttons
            .iter()
            .find(|&&(_, cx, cy)| (((client.x - cx).pow(2) + (client.y - cy).pow(2)) as f32).sqrt() <= r)
            .map(|&(item, _, _)| item)
    }

    fn toggle_menu(&mut self) {
        if self.menu_open {
            self.close_menu();
        } else {
            self.open_menu();
        }
    }

    /// Put the menu window over (or under) the seal for its current size.
    fn place_menu(&mut self) {
        let MenuLayout { w, h, .. } = self.menu_layout();
        let (mw, mh) = self.size();
        let work = work_area(self.x + mw / 2, self.y);
        let mx = self.x + mw / 2 - w / 2;
        let my = if self.menu_below { self.y + mh - (10.0 * self.ui) as i32 } else { self.y - h + (10.0 * self.ui) as i32 };
        self.menu_at = (mx.clamp(work.left, (work.right - w).max(work.left)), my.clamp(work.top, (work.bottom - h).max(work.top)));
    }

    fn open_menu(&mut self) {
        self.settings_open = false;
        self.settings_step = 0;
        let h = self.menu_layout().h;
        let (mw, _) = self.size();
        let work = work_area(self.x + mw / 2, self.y);
        let lift = (10.0 * self.ui) as i32;
        self.menu_below = self.y - h + lift < work.top;
        self.place_menu();
        self.menu_open = true;
        self.menu_hover = None;
        self.menu_step = 0;
        self.update_bubble();
        self.draw_menu();
        unsafe {
            ShowWindow(self.menu, SW_SHOW);
            SetForegroundWindow(self.menu);
            SetTimer(self.menu, TIMER_MENU, 16, None);
        }
    }

    fn close_menu(&mut self) {
        if self.menu_open {
            self.menu_open = false;
            self.settings_open = false;
            self.menu_hover = None;
            self.tracking_leave = false;
            unsafe {
                KillTimer(self.menu, TIMER_MENU);
                ShowWindow(self.menu, SW_HIDE);
            }
            self.update_bubble();
        }
    }

    /// Unfold or fold the settings row; the main row stays where it is on screen.
    fn toggle_settings(&mut self) {
        self.settings_open = !self.settings_open;
        self.settings_step = 0;
        self.place_menu();
        // the window moved under a still pointer: find what it is over now
        let p = cursor_pos();
        self.menu_hover = self.menu_hit(POINT { x: p.x - self.menu_at.0, y: p.y - self.menu_at.1 });
        self.draw_menu();
        if self.settings_open {
            unsafe { SetTimer(self.menu, TIMER_MENU, 16, None) };
        }
    }

    fn label_for(&self, item: Item) -> &'static str {
        match item {
            Item::Resize => "Resize",
            Item::Settings => "Settings",
            Item::Close => "Close",
            Item::Status => if self.show_status { "Hide status" } else { "Show status" },
            Item::Edit => "Edit status",
        }
    }

    fn draw_menu(&mut self) {
        let layout = self.menu_layout();
        let (w, h, btn) = (layout.w, layout.h, layout.btn);
        let ui = self.ui;
        self.nc.reset(w, h);
        // the buttons pop up from the seal: they slide the last few pixels as they fade in
        let eased = |step: u32| 1.0 - (1.0 - step as f32 / MENU_STEPS as f32).powi(3);
        let ease = eased(self.menu_step);
        let sub_ease = eased(self.settings_step);
        let toward_seal = if self.menu_below { -1 } else { 1 };
        for &(item, cx, cy) in &layout.buttons {
            let hovered = self.menu_hover == Some(item);
            let base = match item {
                Item::Status => if self.show_status { 2 } else { 0 },
                Item::Edit => 4,
                Item::Resize => 6,
                Item::Close => 8,
                Item::Settings => 10,
            };
            // settings keeps its ring while its row is open
            let lit = hovered || (item == Item::Settings && self.settings_open);
            let tile = &self.buttons[base + lit as usize];
            if SETTINGS_ROW.contains(&item) {
                let slide = ((1.0 - sub_ease) * 8.0 * ui) as i32 * toward_seal;
                self.nc.blit_faded(tile, cx - btn / 2, cy - btn / 2 + slide, (255.0 * sub_ease) as u8);
            } else {
                let slide = ((1.0 - ease) * 10.0 * ui) as i32 * toward_seal;
                self.nc.blit(tile, cx - btn / 2, cy - btn / 2 + slide);
            }
        }
        if let Some(item) = self.menu_hover {
            let text = self.label_for(item);
            let center = layout.buttons.iter().find(|b| b.0 == item).map(|b| b.1).unwrap_or(w / 2);
            let label_y = if SETTINGS_ROW.contains(&item) { layout.settings_label } else { layout.main_label };
            let (tw, th) = self.nc.measure(self.font, text);
            let (pw, ph) = (tw + (20.0 * ui) as i32, th + (8.0 * ui) as i32);
            let px = (center - pw / 2).clamp(0, (w - pw).max(0));
            let (x0, y0, x1, y1) = (px as f32, label_y as f32, (px + pw) as f32, (label_y + ph) as f32);
            self.nc.fill_sdf((x0, y0, x1, y1), NAVY, 1.0, round_rect(x0, y0, x1, y1, ph as f32 / 2.0));
            self.nc.text(self.font, text, RECT { left: px, top: label_y, right: px + pw, bottom: label_y + ph }, WHITE);
        }
        let alpha = (255.0 * ease) as u8;
        self.nc.present(self.menu, self.menu_at.0, self.menu_at.1, alpha);
    }

    fn menu_tick(&mut self) {
        let settings_done = !self.settings_open || self.settings_step >= MENU_STEPS;
        if self.menu_step < MENU_STEPS || !settings_done {
            self.menu_step = (self.menu_step + 1).min(MENU_STEPS);
            if self.settings_open {
                self.settings_step = (self.settings_step + 1).min(MENU_STEPS);
            }
            self.draw_menu();
        } else {
            unsafe { KillTimer(self.menu, TIMER_MENU) };
        }
    }

    fn menu_move(&mut self, client: POINT) {
        if !self.tracking_leave {
            let mut track = TRACKMOUSEEVENT { cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32, dwFlags: TME_LEAVE, hwndTrack: self.menu, dwHoverTime: 0 };
            unsafe { TrackMouseEvent(&mut track) };
            self.tracking_leave = true;
        }
        let hover = self.menu_hit(client);
        if hover != self.menu_hover {
            self.menu_hover = hover;
            self.draw_menu();
        }
    }

    fn menu_leave(&mut self) {
        self.tracking_leave = false;
        if self.menu_hover.is_some() {
            self.menu_hover = None;
            self.draw_menu();
        }
    }

    fn menu_click(&mut self, client: POINT) {
        let action = match self.menu_hit(client) {
            Some(Item::Settings) => return self.toggle_settings(),
            Some(Item::Resize) => ACT_RESIZE,
            Some(Item::Close) => ACT_QUIT,
            Some(Item::Status) => ACT_STATUS,
            Some(Item::Edit) => ACT_EDIT,
            None => return,
        };
        self.close_menu();
        unsafe { PostMessageW(self.mascot, WM_APP_ACT, action, 0) };
    }

    /// Escape folds the settings row first, then closes the menu.
    fn menu_escape(&mut self) {
        if self.settings_open {
            self.toggle_settings();
        } else {
            self.close_menu();
        }
    }

    // ---- the status-text editor -------------------------------------------------

    fn open_editor(&mut self) {
        if let Some(e) = &self.editor {
            unsafe { SetForegroundWindow(e.hwnd) };
            return;
        }
        let ui = self.ui;
        let layout = editor_layout(ui);
        let (ww, wh) = (layout.w, layout.h);
        let s = |v: f32| (v * ui).round() as i32;
        let (mw, _) = self.size();
        let work = work_area(self.x + mw / 2, self.y);
        let left_of_seal = self.x - ww - s(12.0);
        let wx = if left_of_seal >= work.left { left_of_seal } else { (self.x + mw + s(12.0)).min(work.right - ww) };
        let wy = (self.y - wh / 3).clamp(work.top, (work.bottom - wh).max(work.top));

        unsafe {
            let instance = GetModuleHandleW(null());
            let hwnd = CreateWindowExW(
                WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
                wide("SealPetEditor").as_ptr(),
                wide("Seal Pet · Status text").as_ptr(),
                WS_POPUP | WS_CLIPCHILDREN,
                wx,
                wy,
                ww,
                wh,
                null_mut(),
                null_mut(),
                instance,
                null(),
            );
            // rounded corners and the system's thin border on Windows 11; ignored elsewhere
            let round = DWMWCP_ROUND;
            DwmSetWindowAttribute(hwnd, DWMWA_WINDOW_CORNER_PREFERENCE as u32, &round as *const _ as *const _, 4);
            let fonts = EditorFonts {
                title: font(17.0 * ui, 600, "Segoe UI"),
                label: font(14.0 * ui, 400, "Segoe UI"),
                heading: font(11.5 * ui, 700, "Segoe UI"),
                hint: font(12.0 * ui, 400, "Segoe UI"),
                button: font(14.0 * ui, 600, "Segoe UI"),
                edit: font(14.5 * ui, 400, "Leelawadee UI"),
            };
            let canvas = Canvas::new();
            // each text box sits borderless inside its painted field, centered on one line
            let line = canvas.measure(fonts.edit, "Ag").1 + 2;
            let inset = s(12.0);
            let mut edits = Vec::new();
            for (i, f) in layout.fields.iter().enumerate() {
                let top = ((f.1 + f.3) / 2.0) as i32 - line / 2;
                let edit = CreateWindowExW(
                    0,
                    wide("EDIT").as_ptr(),
                    wide(&self.status.texts[i]).as_ptr(),
                    WS_CHILD | WS_VISIBLE | WS_TABSTOP | ES_AUTOHSCROLL as u32,
                    f.0 as i32 + inset,
                    top,
                    (f.2 - f.0) as i32 - 2 * inset,
                    line,
                    hwnd,
                    (ID_EDIT0 + i as i32) as isize as HMENU,
                    instance,
                    null(),
                );
                SendMessageW(edit, WM_SETFONT, fonts.edit as usize, 1);
                SendMessageW(edit, EM_LIMITTEXT, 60, 0);
                edits.push(edit);
            }
            self.editor = Some(Editor {
                hwnd,
                edits,
                colors: self.status.colors,
                selected: index(&self.state),
                show: self.show_status,
                autohide: self.status.autohide > 0,
                seconds: if self.status.autohide > 0 { self.status.autohide } else { DEFAULT_AUTOHIDE },
                hover: None,
                focus: None,
                tracking: false,
                canvas,
                fonts,
                field_brush: CreateSolidBrush(colorref(FIELD)),
            });
            EDITOR.with(|e| e.set(hwnd));
            ShowWindow(hwnd, SW_SHOW);
            SetForegroundWindow(hwnd);
            let e = self.editor.as_ref().unwrap();
            PostMessageW(hwnd, WM_APP_FOCUS, e.selected, 0);
        }
        self.update_bubble();
    }

    fn paint_editor(&mut self, hdc: HDC) {
        let ui = self.ui;
        let l = editor_layout(ui);
        let Some(e) = self.editor.as_mut() else { return };
        let s = |v: f32| v * ui;
        let pad = s(20.0);
        let right = l.w as f32 - pad;
        let hover = e.hover;
        let fonts = &e.fonts;
        let c = &mut e.canvas;
        c.reset(l.w, l.h);
        c.fill(WHITE);
        let left = DT_LEFT | DT_VCENTER;
        let center = DT_CENTER | DT_VCENTER;

        // header: the title, and a close button
        c.draw_text(fonts.title, "Status text", rect((pad, 0.0, right, l.header)), INK, left);
        if hover == Some(Hit::Close) {
            pill(c, l.close, s(8.0), FIELD);
        }
        let ink = if hover == Some(Hit::Close) { INK } else { MUTED };
        cross(c, l.close, s(5.0), s(1.6), ink, true);

        let heading = |c: &mut Canvas, y: f32, text: &str| {
            c.draw_text(fonts.heading, text, rect((pad, y, right, y + s(18.0))), FAINT, left);
        };

        // the words for each state, each beside its state's dot
        heading(c, l.messages, "MESSAGES");
        let hint = rect((pad, l.messages, right, l.messages + s(18.0)));
        c.draw_text(fonts.hint, "Type \\n for a new line", hint, FAINT, DT_RIGHT | DT_VCENTER);
        for (i, f) in l.fields.iter().enumerate() {
            let cy = (f.1 + f.3) / 2.0;
            c.fill_sdf((pad, cy - s(5.0), pad + s(10.0), cy + s(5.0)), accent(STATES[i].0), 1.0, circle(pad + s(4.0), cy, s(4.0)));
            let ink = if e.selected == i || hover == Some(Hit::State(i)) { INK } else { MUTED };
            c.draw_text(fonts.label, STATES[i].1, rect((pad + s(16.0), f.1, f.0, f.3)), ink, left);
            if e.focus == Some(i) {
                pill(c, grow(*f, s(3.0)), s(13.0), FOCUS_RING);
            }
            if e.selected == i {
                pill(c, *f, s(10.0), BLUE);
            } else {
                pill(c, *f, s(10.0), LINE);
            }
            pill(c, grow(*f, -s(1.0)), s(9.0), FIELD);
        }

        // the text color: the presets, then the system picker
        heading(c, l.colors, &format!("TEXT COLOR  ·  {}", STATES[e.selected].1.to_uppercase()));
        let color = e.colors[e.selected];
        let picked = |sw: Rgba| (sw.0, sw.1, sw.2) == (color.0, color.1, color.2);
        let custom = !SWATCHES.iter().any(|sw| picked(*sw));
        let ring = |c: &mut Canvas, r: Area, color: Rgba| {
            let (cx, cy, rad) = ((r.0 + r.2) / 2.0, (r.1 + r.3) / 2.0, (r.2 - r.0) / 2.0);
            c.fill_sdf(grow(r, s(5.0)), color, 1.0, circle(cx, cy, rad + s(3.5)));
            c.fill_sdf(grow(r, s(5.0)), WHITE, 1.0, circle(cx, cy, rad + s(1.5)));
        };
        for (i, r) in l.swatches.iter().enumerate() {
            let sw = SWATCHES[i];
            if picked(sw) {
                ring(c, *r, sw);
            } else if hover == Some(Hit::Swatch(i)) {
                ring(c, *r, LINE);
            }
            c.fill_sdf(grow(*r, 1.0), sw, 1.0, circle((r.0 + r.2) / 2.0, (r.1 + r.3) / 2.0, (r.2 - r.0) / 2.0));
        }
        let r = l.custom;
        let (cx, cy, rad) = ((r.0 + r.2) / 2.0, (r.1 + r.3) / 2.0, (r.2 - r.0) / 2.0);
        if custom {
            ring(c, r, color);
            c.fill_sdf(grow(r, 1.0), color, 1.0, circle(cx, cy, rad));
        } else {
            let edge = if hover == Some(Hit::Custom) { MUTED } else { TRACK_OFF };
            let w = s(0.75);
            c.fill_sdf(grow(r, 1.0), edge, 1.0, move |x, y| circle(cx, cy, rad - w)(x, y).abs() - w);
        }
        let plus = if custom { WHITE } else if hover == Some(Hit::Custom) { INK } else { MUTED };
        cross(c, r, s(5.0), s(1.6), plus, false);

        // the bubble: shown or not, and hidden again a few seconds after each change
        heading(c, l.bubble, "BUBBLE");
        c.draw_text(fonts.label, "Show status", rect(l.show_row), INK, left);
        toggle(c, l.show, e.show);
        c.draw_text(fonts.label, "Auto hide", rect(l.auto_row), INK, left);
        toggle(c, l.autohide, e.autohide);
        let st = l.seconds;
        pill(c, st, (st.3 - st.1) / 2.0, FIELD);
        for (hit, r) in [(Hit::Less, l.less), (Hit::More, l.more)] {
            let usable = e.autohide && if hit == Hit::Less { e.seconds > 1 } else { e.seconds < MAX_AUTOHIDE };
            if usable && hover == Some(hit) {
                pill(c, grow(r, -s(3.0)), (r.3 - r.1) / 2.0 - s(3.0), LINE);
            }
            let ink = if usable { INK } else { TRACK_OFF };
            let (cx, cy, k) = ((r.0 + r.2) / 2.0, (r.1 + r.3) / 2.0, s(4.5));
            c.fill_sdf(r, ink, 1.0, segment((cx - k, cy), (cx + k, cy), s(1.6)));
            if hit == Hit::More {
                c.fill_sdf(r, ink, 1.0, segment((cx, cy - k), (cx, cy + k), s(1.6)));
            }
        }
        let ink = if e.autohide { INK } else { FAINT };
        c.draw_text(fonts.button, &format!("{} s", e.seconds), rect((l.less.2, st.1, l.more.0, st.3)), ink, center);

        // footer
        let line = l.divider;
        c.fill_sdf((pad, line, right, line + 1.0), LINE, 1.0, |_, _| -1.0);
        if hover == Some(Hit::Reset) {
            pill(c, l.reset, s(10.0), FIELD);
        }
        let ink = if hover == Some(Hit::Reset) { INK } else { MUTED };
        c.draw_text(fonts.button, "Reset", rect(l.reset), ink, center);
        pill(c, l.cancel, s(10.0), if hover == Some(Hit::Cancel) { FIELD_HOVER } else { FIELD });
        c.draw_text(fonts.button, "Cancel", rect(l.cancel), INK, center);
        pill(c, l.save, s(10.0), if hover == Some(Hit::Save) { NAVY_HOVER } else { NAVY });
        c.draw_text(fonts.button, "Save", rect(l.save), WHITE, center);

        c.paint_to(hdc, 0, 0);
    }

    /// What in the editor is under `client`, if anything there takes a click.
    fn editor_hit(&self, client: POINT) -> Option<Hit> {
        let l = editor_layout(self.ui);
        let (x, y) = (client.x as f32, client.y as f32);
        let mut targets = vec![(Hit::Close, l.close), (Hit::Custom, l.custom), (Hit::Less, l.less), (Hit::More, l.more)];
        targets.extend(l.swatches.iter().enumerate().map(|(i, r)| (Hit::Swatch(i), *r)));
        let pad = l.swatches[0].0 - 4.0 * self.ui;
        targets.extend(l.fields.iter().enumerate().map(|(i, f)| (Hit::State(i), (pad, f.1, f.0 - 4.0 * self.ui, f.3))));
        if let Some((hit, _)) = targets.into_iter().find(|(_, r)| inside(*r, x, y)) {
            return Some(hit);
        }
        if inside(l.seconds, x, y) {
            return None;
        }
        [(Hit::Show, l.show_row), (Hit::AutoHide, l.auto_row), (Hit::Reset, l.reset), (Hit::Cancel, l.cancel), (Hit::Save, l.save)]
            .into_iter()
            .find(|(_, r)| inside(*r, x, y))
            .map(|(hit, _)| hit)
    }

    /// Note what the pointer is over, repainting when that changes.
    fn editor_hover(&mut self, hit: Option<Hit>) {
        let Some(e) = self.editor.as_mut() else { return };
        if hit.is_some() && !e.tracking {
            let mut tme = TRACKMOUSEEVENT { cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32, dwFlags: TME_LEAVE, hwndTrack: e.hwnd, dwHoverTime: 0 };
            unsafe { TrackMouseEvent(&mut tme) };
            e.tracking = true;
        }
        if hit.is_none() {
            e.tracking = false;
        }
        if e.hover != hit {
            e.hover = hit;
            unsafe { InvalidateRect(e.hwnd, null(), 0) };
        }
    }

    fn editor_focus(&mut self, focus: Option<usize>) {
        let Some(e) = self.editor.as_mut() else { return };
        e.focus = focus;
        let moved = focus.is_some_and(|i| i != e.selected);
        if let Some(i) = focus {
            e.selected = i;
        }
        unsafe { InvalidateRect(e.hwnd, null(), 0) };
        if moved {
            self.sync_clip();
            self.update_bubble();
        }
    }

    /// Do what a click on `hit` means. The color picker is left to the caller, since its
    /// dialog runs a message loop of its own.
    fn editor_act(&mut self, hit: Hit) {
        match hit {
            Hit::Close | Hit::Cancel => return self.close_editor(false),
            Hit::Save => return self.close_editor(true),
            Hit::Reset => return self.reset_editor(),
            Hit::Swatch(i) => return self.pick_color(SWATCHES[i]),
            Hit::State(i) => {
                if let Some(e) = &self.editor {
                    unsafe { PostMessageW(e.hwnd, WM_APP_FOCUS, i, 0) };
                }
                return;
            }
            _ => {}
        }
        let Some(e) = self.editor.as_mut() else { return };
        match hit {
            Hit::Show => e.show = !e.show,
            Hit::AutoHide => e.autohide = !e.autohide,
            Hit::Less if e.autohide => e.seconds = e.seconds.saturating_sub(1).max(1),
            Hit::More if e.autohide => e.seconds = (e.seconds + 1).min(MAX_AUTOHIDE),
            _ => {}
        }
        unsafe { InvalidateRect(e.hwnd, null(), 0) };
    }

    /// Color the selected state's words.
    fn pick_color(&mut self, c: Rgba) {
        if let Some(e) = self.editor.as_mut() {
            e.colors[e.selected] = c;
            unsafe {
                InvalidateRect(e.hwnd, null(), 0);
                InvalidateRect(e.edits[e.selected], null(), 1);
            }
        }
        self.update_bubble();
    }

    fn reset_editor(&mut self) {
        let defaults = StatusText::defaults();
        if let Some(e) = self.editor.as_mut() {
            for (edit, text) in e.edits.iter().zip(defaults.texts.iter()) {
                unsafe { SetWindowTextW(*edit, wide(text).as_ptr()) };
            }
            e.colors = defaults.colors;
            unsafe { InvalidateRect(e.hwnd, null(), 1) };
        }
        self.update_bubble();
    }

    fn close_editor(&mut self, keep: bool) {
        let Some(e) = self.editor.take() else { return };
        if keep {
            for (i, edit) in e.edits.iter().enumerate() {
                let mut buf = [0u16; 128];
                let n = unsafe { GetWindowTextW(*edit, buf.as_mut_ptr(), buf.len() as i32) } as usize;
                let text = String::from_utf16_lossy(&buf[..n]).replace(['\r', '\n'], " ").trim().to_string();
                self.status.texts[i] = if text.is_empty() { STATES[i].2.to_string() } else { text };
            }
            self.status.colors = e.colors;
            self.status.autohide = if e.autohide { e.seconds.clamp(1, MAX_AUTOHIDE) } else { 0 };
            self.show_status = e.show;
            let _ = fs::create_dir_all(data_dir());
            self.status.save(&data_dir().join("status.txt"));
            self.save();
        }
        EDITOR.with(|h| h.set(null_mut()));
        unsafe {
            DestroyWindow(e.hwnd);
            for f in [e.fonts.title, e.fonts.label, e.fonts.heading, e.fonts.hint, e.fonts.button, e.fonts.edit] {
                DeleteObject(f);
            }
            DeleteObject(e.field_brush);
        }
        self.sync_clip();
        self.restart_autohide();
    }
}

unsafe extern "system" fn mascot_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_SETCURSOR => {
            if with(|app| app.set_cursor()).is_some() {
                return 1;
            }
        }
        WM_LBUTTONDOWN => {
            with(|app| app.press_down(app.in_box(lparam_point(lp))));
            return 0;
        }
        WM_MOUSEMOVE => {
            with(|app| app.press_move());
            return 0;
        }
        WM_LBUTTONUP => {
            with(|app| app.press_up());
            return 0;
        }
        WM_RBUTTONUP => {
            with(|app| app.toggle_menu());
            return 0;
        }
        WM_MOUSEWHEEL => {
            with(|app| app.wheel(((wp >> 16) & 0xFFFF) as u16 as i16));
            return 0;
        }
        WM_TIMER => {
            with(|app| match wp {
                TIMER_FRAME => {
                    app.frame = (app.frame + 1) % app.strip.frames.max(1);
                    app.draw_mascot();
                }
                TIMER_POLL => app.poll(),
                TIMER_BUBBLE => app.bubble_frame(),
                TIMER_AUTOHIDE => {
                    unsafe { KillTimer(app.mascot, TIMER_AUTOHIDE) };
                    app.auto_hidden = true;
                    app.update_bubble();
                }
                TIMER_HOLD => {
                    unsafe { KillTimer(app.mascot, TIMER_HOLD) };
                    if let Press::Pending { grab, .. } = app.press {
                        app.start_drag(grab);
                    }
                }
                _ => {}
            });
            return 0;
        }
        WM_APP_ACT => {
            with(|app| app.act(wp));
            return 0;
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            return 0;
        }
        _ => {}
    }
    DefWindowProcW(hwnd, msg, wp, lp)
}

unsafe extern "system" fn menu_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_SETCURSOR => {
            let over = with(|app| {
                let mut p = cursor_pos();
                unsafe { ScreenToClient(app.menu, &mut p) };
                app.menu_hit(p).is_some()
            });
            if let Some(over) = over {
                SetCursor(LoadCursorW(null_mut(), if over { IDC_HAND } else { IDC_ARROW }));
                return 1;
            }
        }
        WM_MOUSEMOVE => {
            with(|app| app.menu_move(lparam_point(lp)));
            return 0;
        }
        WM_MOUSELEAVE => {
            with(|app| app.menu_leave());
            return 0;
        }
        WM_LBUTTONUP => {
            with(|app| app.menu_click(lparam_point(lp)));
            return 0;
        }
        WM_KEYDOWN if wp == VK_ESCAPE as usize => {
            with(|app| app.menu_escape());
            return 0;
        }
        WM_ACTIVATE if (wp & 0xFFFF) as u32 == WA_INACTIVE => {
            // closed from the queue: the window taking focus may be mid-call here
            PostMessageW(hwnd, WM_APP_CLOSE_MENU, 0, 0);
        }
        WM_APP_CLOSE_MENU => {
            with(|app| app.close_menu());
            return 0;
        }
        WM_TIMER if wp == TIMER_MENU => {
            with(|app| app.menu_tick());
            return 0;
        }
        _ => {}
    }
    DefWindowProcW(hwnd, msg, wp, lp)
}

unsafe extern "system" fn bubble_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if msg == WM_NCHITTEST {
        return HTTRANSPARENT as LRESULT;
    }
    DefWindowProcW(hwnd, msg, wp, lp)
}

/// A custom color from the system color dialog, outside any borrow of the app: the
/// dialog runs its own message loop while it is open.
unsafe fn choose_custom_color(owner: HWND) {
    static mut CUSTOM: [u32; 16] = [0x00FFFFFF; 16];
    let Some(current) = with(|app| app.editor.as_ref().map(|e| e.colors[e.selected])).flatten() else { return };
    let mut cc: CHOOSECOLORW = std::mem::zeroed();
    cc.lStructSize = std::mem::size_of::<CHOOSECOLORW>() as u32;
    cc.hwndOwner = owner;
    cc.rgbResult = colorref(current);
    cc.lpCustColors = std::ptr::addr_of_mut!(CUSTOM) as *mut u32;
    cc.Flags = CC_RGBINIT | CC_FULLOPEN;
    if ChooseColorW(&mut cc) != 0 {
        let c = cc.rgbResult;
        with(|app| app.pick_color(Rgba(c as u8, (c >> 8) as u8, (c >> 16) as u8, 255)));
    }
}

unsafe extern "system" fn editor_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_PAINT => {
            let mut ps: PAINTSTRUCT = std::mem::zeroed();
            let hdc = BeginPaint(hwnd, &mut ps);
            with(|app| app.paint_editor(hdc));
            EndPaint(hwnd, &ps);
            return 0;
        }
        WM_ERASEBKGND => return 1,
        WM_APP_FOCUS => {
            let edit = GetDlgItem(hwnd, ID_EDIT0 + wp as i32);
            SetFocus(edit);
            SendMessageW(edit, EM_SETSEL, 0, -1isize as LPARAM);
            return 0;
        }
        // each text box shows its words in the color they will have
        WM_CTLCOLOREDIT => {
            let edit = lp as HWND;
            let found = with(|app| {
                app.editor.as_ref().map(|e| {
                    let ink = e.edits.iter().position(|h| *h == edit).map_or(INK, |i| e.colors[i]);
                    (e.field_brush, ink)
                })
            });
            if let Some(Some((brush, ink))) = found {
                SetBkColor(wp as HDC, colorref(FIELD));
                SetTextColor(wp as HDC, colorref(ink));
                return brush as LRESULT;
            }
        }
        // the header, away from the close button, drags the window
        WM_NCHITTEST => {
            let mut p = lparam_point(lp);
            ScreenToClient(hwnd, &mut p);
            let header = with(|app| (p.y as f32) < editor_layout(app.ui).header && app.editor_hit(p).is_none());
            return if header == Some(true) { HTCAPTION as LRESULT } else { HTCLIENT as LRESULT };
        }
        WM_MOUSEMOVE => {
            let p = lparam_point(lp);
            with(|app| {
                let hit = app.editor_hit(p);
                app.editor_hover(hit);
            });
            return 0;
        }
        WM_MOUSELEAVE => {
            with(|app| app.editor_hover(None));
            return 0;
        }
        WM_SETCURSOR => {
            let over = with(|app| app.editor.as_ref().is_some_and(|e| e.hover.is_some()));
            if (lp & 0xFFFF) as u32 == HTCLIENT && over == Some(true) {
                SetCursor(LoadCursorW(null_mut(), IDC_HAND));
                return 1;
            }
        }
        WM_LBUTTONDOWN => {
            let p = lparam_point(lp);
            match with(|app| app.editor_hit(p)).flatten() {
                Some(Hit::Custom) => choose_custom_color(hwnd),
                Some(hit) => {
                    with(|app| app.editor_act(hit));
                }
                None => {}
            }
            return 0;
        }
        WM_COMMAND => {
            let id = (wp & 0xFFFF) as i32;
            let field = (id - ID_EDIT0) as usize;
            match (wp >> 16) as u32 {
                EN_CHANGE => {
                    with(|app| app.update_bubble());
                }
                EN_SETFOCUS => {
                    with(|app| app.editor_focus(Some(field)));
                }
                EN_KILLFOCUS => {
                    with(|app| app.editor_focus(None));
                }
                // Enter and Escape, from IsDialogMessage
                _ if id == IDOK => {
                    with(|app| app.close_editor(true));
                }
                _ if id == IDCANCEL => {
                    with(|app| app.close_editor(false));
                }
                _ => {}
            }
            return 0;
        }
        WM_CLOSE => {
            with(|app| app.close_editor(false));
            return 0;
        }
        _ => {}
    }
    DefWindowProcW(hwnd, msg, wp, lp)
}

unsafe fn register(class: &str, proc_: WNDPROC, background: HBRUSH) {
    let name = wide(class);
    let wc = WNDCLASSW {
        style: 0,
        lpfnWndProc: proc_,
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: GetModuleHandleW(null()),
        hIcon: null_mut(),
        hCursor: LoadCursorW(null_mut(), IDC_ARROW),
        hbrBackground: background,
        lpszMenuName: null(),
        lpszClassName: name.as_ptr(),
    };
    RegisterClassW(&wc);
}

unsafe fn layered_window(class: &str, proc_: WNDPROC, ex_style: u32) -> HWND {
    register(class, proc_, null_mut());
    CreateWindowExW(
        WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | ex_style,
        wide(class).as_ptr(),
        wide("Seal Pet").as_ptr(),
        WS_POPUP,
        0,
        0,
        1,
        1,
        null_mut(),
        null_mut(),
        GetModuleHandleW(null()),
        null(),
    )
}

fn main() {
    // `seal-pet.exe --set working`: what the Claude Code hooks run. Write the state and leave;
    // the running seal picks it up on its next poll.
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("--set") {
        let word = args.get(1).map(String::as_str).unwrap_or("");
        std::process::exit(if write_state(&data_dir(), word).is_ok() { 0 } else { 2 });
    }
    unsafe {
        // one seal at a time
        CreateMutexW(null(), 0, wide("Local\\SealPetSingleInstance").as_ptr());
        if GetLastError() == ERROR_ALREADY_EXISTS {
            return;
        }
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        let ui = GetDpiForSystem() as f32 / 96.0;
        let default_h = ((140.0 * ui).round() as i32).clamp(MIN_H, MAX_H);

        let mascot = layered_window("SealPetMascot", Some(mascot_proc), WS_EX_NOACTIVATE);
        let bubble = layered_window("SealPetBubble", Some(bubble_proc), WS_EX_NOACTIVATE | WS_EX_TRANSPARENT);
        let size_tag = layered_window("SealPetSize", Some(bubble_proc), WS_EX_NOACTIVATE | WS_EX_TRANSPARENT);
        let menu = layered_window("SealPetMenu", Some(menu_proc), 0);
        register("SealPetEditor", Some(editor_proc), GetStockObject(WHITE_BRUSH) as HBRUSH);

        let (x, y, height, show_status) = read_placement(default_h).unwrap_or_else(|| {
            // first run: bottom right, above the taskbar
            let work = work_area(0, 0);
            (work.right - default_h - 40, work.bottom - default_h - 40, default_h, false)
        });

        let sheet = Image::decode(MENU_PNG);
        let btn = (48.0 * ui).round() as usize;
        let buttons = (0..12).map(|i| sheet.tile(i, 12, btn)).collect();

        let (state, stamp) = read_state();
        // no word yet: forget the stamp so the first poll looks again
        let stamp = state.as_ref().and(stamp);
        let state = state.unwrap_or_default();
        let clip = clip_for(&state);
        let mut app = App {
            mascot,
            bubble,
            size_tag,
            menu,
            mc: Canvas::new(),
            bc: Canvas::new(),
            bf: Canvas::new(),
            tails: [Canvas::new(), Canvas::new()],
            bubble_anim: BubbleAnim::new(),
            tc: Canvas::new(),
            nc: Canvas::new(),
            x,
            y,
            height,
            strip: clip.strip(),
            clip,
            state: state.clone(),
            stamp,
            frame: 0,
            press: Press::None,
            resize_mode: false,
            show_status,
            auto_hidden: false,
            status: StatusText::load(&data_dir().join("status.txt")),
            menu_open: false,
            menu_hover: None,
            menu_step: 0,
            settings_open: false,
            settings_step: 0,
            menu_at: (0, 0),
            menu_below: false,
            tracking_leave: false,
            ui,
            font: font(14.0 * ui, 700, "Leelawadee UI"),
            tag_font: font(12.0 * ui, 700, "Leelawadee UI"),
            buttons,
            editor: None,
        };
        SetTimer(mascot, TIMER_FRAME, clip.interval, None);
        app.draw_mascot();
        app.restart_autohide();
        ShowWindow(mascot, SW_SHOWNOACTIVATE);
        APP.with(|a| *a.borrow_mut() = Some(app));
        SetTimer(mascot, TIMER_POLL, POLL_MS, None);

        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
            let editor = EDITOR.with(|e| e.get());
            if !editor.is_null() && IsDialogMessageW(editor, &msg) != 0 {
                continue;
            }
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}
