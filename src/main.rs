//! Seal Pet: a small seal that floats on the desktop and acts out what Claude Code is doing.
//!
//! Claude Code hooks write the current phase (loading, working, asking, completed, idle)
//! to `%LOCALAPPDATA%\seal-pet\state`; this program polls that file and plays the matching
//! animation in a borderless, per-pixel transparent, always-on-top window.
//!
//! A short click opens a menu of round icon buttons (status bubble, edit status text,
//! resize, quit); pressing and holding drags the seal. In resize mode the seal grows and
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
use std::time::SystemTime;

use gfx::*;
use status::*;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::CreateMutexW;
use windows_sys::Win32::UI::Controls::Dialogs::*;
use windows_sys::Win32::UI::HiDpi::*;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

/// The frames' own size: every size the seal takes keeps this aspect.
const FRAME_W: i32 = 260;
const FRAME_H: i32 = 280;
/// The seal fits a 32 x 32 box at its smallest and a 512 x 512 box at its largest.
const MIN_H: i32 = 32;
const MAX_H: i32 = 512;

const HOLD_MS: u32 = 220;
const DRAG_SLOP: i32 = 4;
const POLL_MS: u32 = 400;
const MENU_STEPS: u32 = 6;

const TIMER_FRAME: usize = 1;
const TIMER_POLL: usize = 2;
const TIMER_HOLD: usize = 3;
const TIMER_MENU: usize = 4;

// not in the windows-sys features this crate pulls in
const WM_MOUSELEAVE: u32 = 0x02A3;
const EM_LIMITTEXT: u32 = 0x00C5;
const EN_CHANGE: u32 = 0x0300;
const WM_APP_ACT: u32 = WM_APP + 1;
const WM_APP_CLOSE_MENU: u32 = WM_APP + 2;

const ACT_STATUS: usize = 0;
const ACT_EDIT: usize = 1;
const ACT_RESIZE: usize = 2;
const ACT_QUIT: usize = 3;
const MENU_BUTTONS: usize = 4;

// the editor's controls
const ID_EDIT0: i32 = 100;
const ID_RESET: i32 = 10;
const ID_CUSTOM: i32 = 11;

const NAVY: Rgba = Rgba(24, 52, 110, 255);
const BLUE: Rgba = Rgba(74, 152, 216, 255);
const WHITE: Rgba = Rgba(255, 255, 255, 255);

/// One animation: its frame strip, frame count and milliseconds per frame.
struct Clip {
    png: &'static [u8],
    frames: usize,
    interval: u32,
}

const LOADING: Clip = Clip { png: include_bytes!("../assets/loading.png"), frames: 48, interval: 50 };
const WORKING: Clip = Clip { png: include_bytes!("../assets/working.png"), frames: 48, interval: 50 };
const ASKING: Clip = Clip { png: include_bytes!("../assets/asking.png"), frames: 60, interval: 50 };
const COMPLETED: Clip = Clip { png: include_bytes!("../assets/completed.png"), frames: 60, interval: 80 };
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

/// The open status-text editor: its window, its text boxes and the color being picked.
struct Editor {
    hwnd: HWND,
    edits: Vec<HWND>,
    color: Rgba,
    /// the color before editing began, put back on Cancel
    was: Rgba,
    font: HFONT,
    swatches: Canvas,
}

struct App {
    mascot: HWND,
    bubble: HWND,
    menu: HWND,
    mc: Canvas,
    bc: Canvas,
    nc: Canvas,
    x: i32,
    y: i32,
    height: i32,
    strip: Strip,
    state: String,
    stamp: Option<SystemTime>,
    frame: usize,
    press: Press,
    resize_mode: bool,
    show_status: bool,
    status: StatusText,
    menu_open: bool,
    menu_hover: Option<usize>,
    menu_step: u32,
    menu_at: (i32, i32),
    menu_below: bool,
    tracking_leave: bool,
    ui: f32,
    font: HFONT,
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

fn read_state() -> (String, Option<SystemTime>) {
    let path = data_dir().join("state");
    let stamp = fs::metadata(&path).and_then(|m| m.modified()).ok();
    let state = fs::read_to_string(&path).map(|s| s.trim().to_string()).unwrap_or_default();
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

    fn load(&mut self, state: &str) {
        let clip = clip_for(state);
        self.strip = Strip::decode(clip.png, clip.frames);
        self.frame = 0;
        self.state = state.to_string();
        unsafe { SetTimer(self.mascot, TIMER_FRAME, clip.interval, None) };
        self.draw_mascot();
        self.update_bubble();
    }

    fn draw_mascot(&mut self) {
        let (w, h) = self.size();
        self.mc.reset(w, h);
        self.strip.draw(self.frame, &mut self.mc);
        if self.resize_mode {
            let ui = self.ui;
            let (wf, hf) = (w as f32, h as f32);
            let frame = round_rect(1.0, 1.0, wf - 1.0, hf - 1.0, 10.0 * ui);
            self.mc.fill_sdf((0.0, 0.0, wf, hf), Rgba(74, 152, 216, 210), 1.0, move |x, y| frame(x, y).abs() - 0.9 * ui);
            let (cx, cy, r) = self.handle();
            self.mc.fill_sdf((cx - r - 2.0, cy - r - 2.0, cx + r + 2.0, cy + r + 2.0), BLUE, 1.0, circle(cx, cy, r));
            self.mc.fill_sdf((cx - r, cy - r, cx + r, cy + r), WHITE, 1.0, circle(cx, cy, r - 1.6 * ui));
        }
        let alpha = if matches!(self.press, Press::Drag { .. }) { 225 } else { 255 };
        self.mc.present(self.mascot, self.x, self.y, alpha);
    }

    fn move_to(&mut self, x: i32, y: i32) {
        self.x = x;
        self.y = y;
        let alpha = if matches!(self.press, Press::Drag { .. }) { 225 } else { 255 };
        self.mc.present(self.mascot, x, y, alpha);
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

    fn poll(&mut self) {
        let (state, stamp) = read_state();
        if stamp != self.stamp {
            self.stamp = stamp;
            if !std::ptr::eq(clip_for(&state), clip_for(&self.state)) {
                self.load(&state);
            } else if state != self.state {
                self.state = state;
                self.update_bubble();
            }
        }
    }

    // ---- pointer on the seal ---------------------------------------------------

    fn set_cursor(&self) {
        let mut p = cursor_pos();
        unsafe {
            ScreenToClient(self.mascot, &mut p);
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
                self.update_bubble();
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

    /// The text color the bubble uses right now: the editor's pick while it is open.
    fn text_color(&self) -> Rgba {
        self.editor.as_ref().map(|e| e.color).unwrap_or(self.status.color)
    }

    fn update_bubble(&mut self) {
        let visible = (self.show_status || self.resize_mode || self.editor.is_some()) && !self.menu_open;
        if !visible {
            unsafe { ShowWindow(self.bubble, SW_HIDE) };
            return;
        }
        let (text, dot) = if self.resize_mode {
            (format!("{} px · scroll or drag ↘ · click to finish", self.height), BLUE)
        } else if let Some(e) = &self.editor {
            // while editing, the bubble previews the words being typed for this state
            let i = STATES.iter().position(|(k, _, _)| *k == self.state).unwrap_or(4);
            let mut buf = [0u16; 128];
            let n = unsafe { GetWindowTextW(e.edits[i], buf.as_mut_ptr(), buf.len() as i32) } as usize;
            let typed = String::from_utf16_lossy(&buf[..n]);
            (if typed.trim().is_empty() { STATES[i].2.to_string() } else { typed }, accent(&self.state))
        } else {
            (self.status.text(&self.state).to_string(), accent(&self.state))
        };
        let ink = self.text_color();
        let ui = self.ui;
        let (tw, th) = self.bc.measure(self.font, &text);
        let pad_x = (14.0 * ui) as i32;
        let pad_y = (8.0 * ui) as i32;
        let dot_r = 4.0 * ui;
        let dot_room = (dot_r * 2.0 + 8.0 * ui) as i32;
        let tail = (8.0 * ui) as i32;
        let margin = (10.0 * ui) as i32; // room for the soft shadow
        let (bw, bh) = (tw + dot_room + 2 * pad_x, th + 2 * pad_y);
        let (cw, ch) = (bw + 2 * margin, bh + tail + 2 * margin);
        let (mw, mh) = self.size();
        let work = work_area(self.x + mw / 2, self.y);
        let mut bx = self.x + mw / 2 - cw / 2;
        let mut by = self.y - ch + margin + 2;
        let below = by < work.top;
        if below {
            by = self.y + mh - margin - 2;
        }
        bx = bx.clamp(work.left, (work.right - cw).max(work.left));
        by = by.clamp(work.top, (work.bottom - ch).max(work.top));

        // the body, and a tail that points at the seal and melts into the body
        let (x0, x1) = (margin as f32, (margin + bw) as f32);
        let (y0, y1) = if below { ((margin + tail) as f32, (margin + tail + bh) as f32) } else { (margin as f32, (margin + bh) as f32) };
        let r = bh as f32 / 2.0;
        let t = tail as f32;
        let tip_x = ((self.x + mw / 2 - bx) as f32).clamp(x0 + r, x1 - r);
        let (base_y, point_y) = if below { (y0 + r * 0.8, y0 - t) } else { (y1 - r * 0.8, y1 + t) };
        let body = round_rect(x0, y0, x1, y1, r);
        let tip = triangle((tip_x - t * 1.1, base_y), (tip_x + t * 1.1, base_y), (tip_x, point_y));
        let k = 6.0 * ui;
        let shape = move |x: f32, y: f32| smin(body(x, y), tip(x, y) - 1.2 * ui, k);

        self.bc.reset(cw, ch);
        let all = (0.0, 0.0, cw as f32, ch as f32);
        let lift = 3.0 * ui;
        self.bc.fill_sdf(all, Rgba(16, 32, 70, 70), 9.0 * ui, |x, y| shape(x, y - lift) + 1.0 * ui);
        self.bc.fill_sdf(all, Rgba(205, 222, 240, 255), 1.0, &shape);
        self.bc.fill_sdf(all, WHITE, 1.0, |x, y| shape(x, y) + 1.0 * ui);
        let (dx, dy) = (x0 + pad_x as f32 + dot_r, (y0 + y1) / 2.0);
        self.bc.fill_sdf((dx - dot_r - 2.0, dy - dot_r - 2.0, dx + dot_r + 2.0, dy + dot_r + 2.0), dot, 1.0, circle(dx, dy, dot_r));
        let text_left = x0 as i32 + pad_x + dot_room;
        let text_rect = RECT { left: text_left, top: y0 as i32, right: text_left + tw, bottom: y1 as i32 };
        self.bc.text(self.font, &text, text_rect, ink);
        self.bc.present(self.bubble, bx, by, 255);
        unsafe { ShowWindow(self.bubble, SW_SHOWNOACTIVATE) };
    }

    // ---- the icon menu ---------------------------------------------------------

    fn menu_geometry(&self) -> (i32, i32, i32, [(i32, i32); MENU_BUTTONS], i32) {
        let ui = self.ui;
        let btn = (48.0 * ui).round() as i32;
        let gap = (12.0 * ui) as i32;
        let pad = (10.0 * ui) as i32;
        let raise = (14.0 * ui) as i32;
        let label = (28.0 * ui) as i32;
        let n = MENU_BUTTONS as i32;
        let w = n * btn + (n - 1) * gap + 2 * pad;
        let h = pad + raise + btn + (8.0 * ui) as i32 + label + pad;
        // an arc: the outer buttons sit lower than the inner ones
        let centers = std::array::from_fn(|i| {
            let outer = i == 0 || i == MENU_BUTTONS - 1;
            (pad + i as i32 * (btn + gap) + btn / 2, pad + btn / 2 + if outer { raise } else { 0 })
        });
        (w, h, btn, centers, pad + raise + btn + (8.0 * ui) as i32)
    }

    fn menu_hit(&self, client: POINT) -> Option<usize> {
        let (_, _, btn, centers, _) = self.menu_geometry();
        let r = btn as f32 * 0.42;
        centers.iter().position(|&(cx, cy)| (((client.x - cx).pow(2) + (client.y - cy).pow(2)) as f32).sqrt() <= r)
    }

    fn toggle_menu(&mut self) {
        if self.menu_open {
            self.close_menu();
        } else {
            self.open_menu();
        }
    }

    fn open_menu(&mut self) {
        let (w, h, _, _, _) = self.menu_geometry();
        let (mw, mh) = self.size();
        let work = work_area(self.x + mw / 2, self.y);
        let mut mx = self.x + mw / 2 - w / 2;
        let mut my = self.y - h + (10.0 * self.ui) as i32;
        self.menu_below = my < work.top;
        if self.menu_below {
            my = self.y + mh - (10.0 * self.ui) as i32;
        }
        mx = mx.clamp(work.left, (work.right - w).max(work.left));
        my = my.clamp(work.top, (work.bottom - h).max(work.top));
        self.menu_at = (mx, my);
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
            self.menu_hover = None;
            self.tracking_leave = false;
            unsafe {
                KillTimer(self.menu, TIMER_MENU);
                ShowWindow(self.menu, SW_HIDE);
            }
            self.update_bubble();
        }
    }

    fn draw_menu(&mut self) {
        let (w, h, btn, centers, label_y) = self.menu_geometry();
        let ui = self.ui;
        self.nc.reset(w, h);
        // the buttons pop up from the seal: they slide the last few pixels as they fade in
        let t = self.menu_step as f32 / MENU_STEPS as f32;
        let ease = 1.0 - (1.0 - t).powi(3);
        let slide = ((1.0 - ease) * 10.0 * ui) as i32 * if self.menu_below { -1 } else { 1 };
        for (i, &(cx, cy)) in centers.iter().enumerate() {
            let base = match i {
                ACT_STATUS => if self.show_status { 2 } else { 0 },
                ACT_EDIT => 4,
                ACT_RESIZE => 6,
                _ => 8,
            };
            let tile = &self.buttons[base + (self.menu_hover == Some(i)) as usize];
            self.nc.blit(tile, cx - btn / 2, cy - btn / 2 + slide);
        }
        if let Some(i) = self.menu_hover {
            let text = match i {
                ACT_STATUS => if self.show_status { "Hide status" } else { "Show status" },
                ACT_EDIT => "Edit status text",
                ACT_RESIZE => "Resize",
                _ => "Quit",
            };
            let (tw, th) = self.nc.measure(self.font, text);
            let (pw, ph) = (tw + (20.0 * ui) as i32, th + (8.0 * ui) as i32);
            let px = (centers[i].0 - pw / 2).clamp(0, w - pw);
            let (x0, y0, x1, y1) = (px as f32, label_y as f32, (px + pw) as f32, (label_y + ph) as f32);
            self.nc.fill_sdf((x0, y0, x1, y1), NAVY, 1.0, round_rect(x0, y0, x1, y1, ph as f32 / 2.0));
            self.nc.text(self.font, text, RECT { left: px, top: label_y, right: px + pw, bottom: label_y + ph }, WHITE);
        }
        let alpha = (255.0 * ease) as u8;
        self.nc.present(self.menu, self.menu_at.0, self.menu_at.1, alpha);
    }

    fn menu_tick(&mut self) {
        if self.menu_step < MENU_STEPS {
            self.menu_step += 1;
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
        if let Some(i) = self.menu_hit(client) {
            self.close_menu();
            unsafe { PostMessageW(self.mascot, WM_APP_ACT, i, 0) };
        }
    }

    // ---- the status-text editor -------------------------------------------------

    fn swatch_layout(&self) -> (i32, i32, i32, i32) {
        // (left, top, diameter, step) of the swatch row in the editor's client area
        let ui = self.ui;
        ((16.0 * ui) as i32, (226.0 * ui) as i32, (24.0 * ui) as i32, (32.0 * ui) as i32)
    }

    fn open_editor(&mut self) {
        if let Some(e) = &self.editor {
            unsafe { SetForegroundWindow(e.hwnd) };
            return;
        }
        let ui = self.ui;
        let s = |v: f32| (v * ui).round() as i32;
        let (cw, ch) = (s(400.0), s(318.0));
        let style = WS_POPUP | WS_CAPTION | WS_SYSMENU;
        let ex = WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_DLGMODALFRAME;
        let mut rect = RECT { left: 0, top: 0, right: cw, bottom: ch };
        unsafe { AdjustWindowRectEx(&mut rect, style, 0, ex) };
        let (ww, wh) = (rect.right - rect.left, rect.bottom - rect.top);
        let (mw, _) = self.size();
        let work = work_area(self.x + mw / 2, self.y);
        let left_of_seal = self.x - ww - s(12.0);
        let wx = if left_of_seal >= work.left { left_of_seal } else { (self.x + mw + s(12.0)).min(work.right - ww) };
        let wy = (self.y - wh / 3).clamp(work.top, (work.bottom - wh).max(work.top));

        unsafe {
            let instance = GetModuleHandleW(null());
            let hwnd = CreateWindowExW(ex, wide("SealPetEditor").as_ptr(), wide("Seal Pet · Status text").as_ptr(), style, wx, wy, ww, wh, null_mut(), null_mut(), instance, null());
            let ui_font = font(15.0 * ui, 400, "Segoe UI");
            let child = |class: &str, text: &str, style: u32, x: i32, y: i32, w: i32, h: i32, id: i32| {
                let c = CreateWindowExW(0, wide(class).as_ptr(), wide(text).as_ptr(), WS_CHILD | WS_VISIBLE | style, x, y, w, h, hwnd, id as isize as HMENU, instance, null());
                SendMessageW(c, WM_SETFONT, ui_font as usize, 1);
                c
            };
            let mut edits = Vec::new();
            for (i, (_, label, _)) in STATES.iter().enumerate() {
                let y = s(16.0) + i as i32 * s(36.0);
                child("STATIC", label, 0, s(16.0), y + s(4.0), s(84.0), s(22.0), -1);
                let edit = child("EDIT", &self.status.texts[i], WS_BORDER | WS_TABSTOP | ES_AUTOHSCROLL as u32, s(100.0), y, cw - s(116.0), s(28.0), ID_EDIT0 + i as i32);
                SendMessageW(edit, EM_LIMITTEXT, 60, 0);
                edits.push(edit);
            }
            child("STATIC", "Text color", 0, s(16.0), s(198.0), s(200.0), s(22.0), -1);
            let (sx, sy, sd, step) = self.swatch_layout();
            child("BUTTON", "Custom…", WS_TABSTOP | BS_PUSHBUTTON as u32, sx + 8 * step + s(6.0), sy - s(2.0), cw - (sx + 8 * step + s(6.0)) - s(16.0), sd + s(4.0), ID_CUSTOM);
            let by = ch - s(16.0) - s(30.0);
            child("BUTTON", "Reset", WS_TABSTOP | BS_PUSHBUTTON as u32, s(16.0), by, s(84.0), s(30.0), ID_RESET);
            child("BUTTON", "Cancel", WS_TABSTOP | BS_PUSHBUTTON as u32, cw - s(16.0) - s(84.0) * 2 - s(8.0), by, s(84.0), s(30.0), IDCANCEL);
            child("BUTTON", "Save", WS_TABSTOP | BS_DEFPUSHBUTTON as u32, cw - s(16.0) - s(84.0), by, s(84.0), s(30.0), IDOK);

            let mut swatches = Canvas::new();
            swatches.reset(8 * step, sd + s(8.0));
            self.editor = Some(Editor { hwnd, edits, color: self.status.color, was: self.status.color, font: ui_font, swatches });
            EDITOR.with(|e| e.set(hwnd));
            ShowWindow(hwnd, SW_SHOW);
            SetForegroundWindow(hwnd);
            SetFocus(self.editor.as_ref().unwrap().edits[0]);
        }
        self.update_bubble();
    }

    fn paint_swatches(&mut self, hdc: HDC) {
        let (sx, sy, sd, step) = self.swatch_layout();
        let ui = self.ui;
        let Some(e) = self.editor.as_mut() else { return };
        e.swatches.fill(WHITE);
        let r = sd as f32 / 2.0;
        for (i, c) in SWATCHES.iter().enumerate() {
            let (cx, cy) = (i as f32 * step as f32 + r + 2.0 * ui, e.swatches.h as f32 / 2.0);
            let chosen = (c.0, c.1, c.2) == (e.color.0, e.color.1, e.color.2);
            if chosen {
                e.swatches.fill_sdf((cx - r - 4.0 * ui, cy - r - 4.0 * ui, cx + r + 4.0 * ui, cy + r + 4.0 * ui), *c, 1.0, circle(cx, cy, r + 3.0 * ui));
                e.swatches.fill_sdf((cx - r - 2.0 * ui, cy - r - 2.0 * ui, cx + r + 2.0 * ui, cy + r + 2.0 * ui), WHITE, 1.0, circle(cx, cy, r + 1.5 * ui));
            }
            e.swatches.fill_sdf((cx - r - 1.0, cy - r - 1.0, cx + r + 1.0, cy + r + 1.0), *c, 1.0, circle(cx, cy, r));
        }
        e.swatches.paint_to(hdc, sx, sy - (4.0 * ui) as i32);
    }

    fn editor_click(&mut self, client: POINT) {
        let (sx, sy, sd, step) = self.swatch_layout();
        if client.y < sy - 4 || client.y > sy + sd + 4 || client.x < sx {
            return;
        }
        let i = ((client.x - sx) / step) as usize;
        if i < SWATCHES.len() {
            self.pick_color(SWATCHES[i]);
        }
    }

    fn pick_color(&mut self, c: Rgba) {
        if let Some(e) = self.editor.as_mut() {
            e.color = c;
            unsafe { InvalidateRect(e.hwnd, null(), 0) };
        }
        self.update_bubble();
    }

    fn reset_editor(&mut self) {
        let defaults = StatusText::defaults();
        if let Some(e) = &self.editor {
            for (edit, text) in e.edits.iter().zip(defaults.texts.iter()) {
                unsafe { SetWindowTextW(*edit, wide(text).as_ptr()) };
            }
        }
        self.pick_color(defaults.color);
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
            self.status.color = e.color;
            let _ = fs::create_dir_all(data_dir());
            self.status.save(&data_dir().join("status.txt"));
        } else {
            self.status.color = e.was;
        }
        EDITOR.with(|h| h.set(null_mut()));
        unsafe {
            DestroyWindow(e.hwnd);
            DeleteObject(e.font);
        }
        self.update_bubble();
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
            with(|app| app.press_down(lparam_point(lp)));
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
            with(|app| app.close_menu());
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
    let Some(current) = with(|app| app.editor.as_ref().map(|e| e.color)).flatten() else { return };
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
            with(|app| app.paint_swatches(hdc));
            EndPaint(hwnd, &ps);
            return 0;
        }
        WM_CTLCOLORSTATIC => {
            SetBkColor(wp as HDC, 0x00FFFFFF);
            SetTextColor(wp as HDC, 0x00463A2E);
            return GetStockObject(WHITE_BRUSH) as LRESULT;
        }
        WM_LBUTTONDOWN => {
            with(|app| app.editor_click(lparam_point(lp)));
            return 0;
        }
        WM_SETCURSOR => {
            let mut p = cursor_pos();
            ScreenToClient(hwnd, &mut p);
            let over = with(|app| {
                let (sx, sy, sd, step) = app.swatch_layout();
                p.y >= sy - 4 && p.y <= sy + sd + 4 && p.x >= sx && p.x < sx + 8 * step
            });
            if over == Some(true) {
                SetCursor(LoadCursorW(null_mut(), IDC_HAND));
                return 1;
            }
        }
        WM_COMMAND => {
            let id = (wp & 0xFFFF) as i32;
            if (wp >> 16) as u32 == EN_CHANGE {
                with(|app| app.update_bubble());
                return 0;
            }
            match id {
                IDOK => {
                    with(|app| app.close_editor(true));
                }
                IDCANCEL => {
                    with(|app| app.close_editor(false));
                }
                ID_RESET => {
                    with(|app| app.reset_editor());
                }
                ID_CUSTOM => choose_custom_color(hwnd),
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
        let menu = layered_window("SealPetMenu", Some(menu_proc), 0);
        register("SealPetEditor", Some(editor_proc), GetStockObject(WHITE_BRUSH) as HBRUSH);

        let (x, y, height, show_status) = read_placement(default_h).unwrap_or_else(|| {
            // first run: bottom right, above the taskbar
            let work = work_area(0, 0);
            (work.right - default_h - 40, work.bottom - default_h - 40, default_h, false)
        });

        let sheet = Image::decode(MENU_PNG);
        let btn = (48.0 * ui).round() as usize;
        let buttons = (0..10).map(|i| sheet.tile(i, 10, btn)).collect();

        let (state, stamp) = read_state();
        let clip = clip_for(&state);
        let mut app = App {
            mascot,
            bubble,
            menu,
            mc: Canvas::new(),
            bc: Canvas::new(),
            nc: Canvas::new(),
            x,
            y,
            height,
            strip: Strip::decode(clip.png, clip.frames),
            state: String::new(),
            stamp,
            frame: 0,
            press: Press::None,
            resize_mode: false,
            show_status,
            status: StatusText::load(&data_dir().join("status.txt")),
            menu_open: false,
            menu_hover: None,
            menu_step: 0,
            menu_at: (0, 0),
            menu_below: false,
            tracking_leave: false,
            ui,
            font: font(14.0 * ui, 600, "Leelawadee UI"),
            buttons,
            editor: None,
        };
        app.load(&state);
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
