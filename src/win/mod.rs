//! The Windows seal: a borderless, per-pixel transparent, always-on-top layered window that
//! plays the clip for the state the hooks set, with its status bubble, icon menu and editor.

mod editor;
mod menu;

use std::cell::{Cell, RefCell};
use std::fs;
use std::ptr::{null, null_mut};
use std::time::SystemTime;

use editor::*;
use menu::*;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Dwm::*;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::CreateMutexW;
use windows_sys::Win32::UI::Controls::Dialogs::*;
use windows_sys::Win32::UI::HiDpi::*;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

use crate::bubble::*;
use crate::editor::*;
use crate::gfx::*;
use crate::menu::*;
use crate::seal::*;
use crate::status::*;
use crate::store::*;

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



enum Press {
    None,
    /// down, not yet a drag: a release now is a click
    Pending { at: POINT, grab: POINT },
    Drag { grab: POINT },
    Resize { at: POINT, h0: i32 },
}

struct App {
    mascot: HWND,
    bubble: HWND,
    /// the size readout under the seal while resizing
    size_tag: HWND,
    menu_win: HWND,
    mc: Canvas,
    /// the status bubble and the size readout, as drawn
    bubble_view: Bubble,
    x: i32,
    y: i32,
    height: i32,
    strip: Strip,
    /// the act playing: the state's, or while editing, the edited state's
    turns: Turns,
    state: String,
    stamp: Option<SystemTime>,
    frame: usize,
    press: Press,
    resize_mode: bool,
    show_status: bool,
    /// the bubble has had its auto-hide time since the last state change
    auto_hidden: bool,
    /// the minute of the day last looked at for a break
    minute: u32,
    status: StatusText,
    menu: Menu,
    tracking_leave: bool,
    ui: f32,
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

fn bounds(r: RECT) -> Bounds {
    (r.left, r.top, r.right, r.bottom)
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
            Some(e) => STATES[e.draft.selected].0,
            None => &self.state,
        }
    }

    /// Play the shown state's act, if it is not playing already.
    fn sync_clip(&mut self) {
        let act = act_for(self.shown_state());
        if std::ptr::eq(act, self.turns.act) {
            return;
        }
        let was = self.turns.clip();
        self.turns.switch(act);
        // a feed or a break playing goes on
        if !std::ptr::eq(was, self.turns.clip()) {
            self.play();
        }
    }

    /// Start the clip whose turn it is, from its first frame.
    fn play(&mut self) {
        let clip = self.turns.clip();
        self.strip.release();
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
        save_placement(self.x, self.y, self.height, self.show_status);
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
        let minute = minute_of_day();
        if minute != self.minute {
            self.minute = minute;
            if self.status.breaks.contains(&minute) && !self.turns.on_break() {
                self.turns.cut_in(BREAK);
                self.play();
                self.update_bubble();
            }
        }
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
        // the file keeps saying completed, but the seal goes back to napping
        if done_showing(&self.state, self.stamp) {
            self.state = "idle".into();
            self.sync_clip();
            self.restart_autohide();
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
                } else if self.turns.on_break() {
                    // a click ends the break
                    self.turns.end_cut_in();
                    self.play();
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
            ACT_FEED => {
                self.turns.cut_in(FEED);
                self.play();
                self.update_bubble();
            }
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


    /// The size readout under the seal while resizing. Returns its screen bounds, so the
    /// bubble can stack clear of it.
    fn update_size_tag(&mut self) -> Option<Bounds> {
        if !self.resize_mode || self.menu.open {
            unsafe { ShowWindow(self.size_tag, SW_HIDE) };
            return None;
        }
        let (w, h) = self.size();
        let work = work_area(self.x + w / 2, self.y + h);
        let ((x, y), bounds) = self.bubble_view.size_tag(self.height, (self.x, self.y, w, h), bounds(work), self.ui);
        self.bubble_view.tag.present(self.size_tag, x, y, 255);
        unsafe { ShowWindow(self.size_tag, SW_SHOWNOACTIVATE) };
        Some(bounds)
    }

    fn update_bubble(&mut self) {
        let tag = self.update_size_tag();
        // a break or a feed says its words whether or not the bubble is on
        let scene = self.turns.scene().filter(|_| self.editor.is_none() && !self.resize_mode);
        let visible = ((self.show_status && !self.auto_hidden) || self.resize_mode || self.editor.is_some() || scene.is_some()) && !self.menu.open;
        if !visible {
            self.bubble_view.hide();
            self.bubble_frame();
            return;
        }
        let key = if self.resize_mode {
            "resize".to_string()
        } else if let Some(scene) = scene {
            scene.to_string()
        } else {
            self.shown_state().to_string()
        };
        let (text, dot) = if self.resize_mode {
            ("Scroll, Drag or Click".to_string(), None)
        } else if let Some(scene) = scene {
            (self.status.scene_text(scene).to_string(), Some(accent(scene)))
        } else if let Some(e) = &self.editor {
            // while editing, the bubble previews the state being edited, as typed and colored
            let i = e.draft.selected;
            let typed = window_text(e.edits[i]);
            (if typed.trim().is_empty() { STATES[i].2.to_string() } else { typed }, Some(accent(STATES[i].0)))
        } else {
            (self.status.text(&self.state).to_string(), Some(accent(&self.state)))
        };
        let ink = match &self.editor {
            Some(e) => e.draft.colors[e.draft.selected],
            None if scene.is_some() => DEFAULT_COLOR,
            None => self.status.color(&self.state),
        };
        let (w, h) = self.size();
        let work = work_area(self.x + w / 2, self.y);
        let content = Content { text: &text, dot, ink, key: &key };
        self.bubble_view.show(&content, (self.x, self.y, w, h), bounds(work), tag, self.ui);
        self.bubble_frame();
    }

    /// Show the bubble's next frame, keeping the frame timer running while it is up.
    fn bubble_frame(&mut self) {
        match self.bubble_view.frame(self.ui) {
            Frame::Hidden => unsafe {
                KillTimer(self.mascot, TIMER_BUBBLE);
                ShowWindow(self.bubble, SW_HIDE);
            },
            Frame::Shown { x, y, alpha, moving } => unsafe {
                self.bubble_view.shown.present(self.bubble, x, y, alpha);
                if IsWindowVisible(self.bubble) == 0 {
                    ShowWindow(self.bubble, SW_SHOWNOACTIVATE);
                }
                SetTimer(self.mascot, TIMER_BUBBLE, if moving { 15 } else { 33 }, None);
            },
        }
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
                    if app.frame == 0 && app.turns.looped() {
                        app.play();
                        // a scene's words come and go with its clips
                        app.update_bubble();
                    } else {
                        app.draw_mascot();
                    }
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

unsafe extern "system" fn bubble_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if msg == WM_NCHITTEST {
        return HTTRANSPARENT as LRESULT;
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
        wide("ai-mascot-hook").as_ptr(),
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

pub fn run() {
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
        let menu_win = layered_window("SealPetMenu", Some(menu_proc), 0);
        register("SealPetEditor", Some(editor_proc), GetStockObject(WHITE_BRUSH) as HBRUSH);

        let (x, y, height, show_status) = read_placement(default_h).unwrap_or_else(|| {
            // first run: bottom right, above the taskbar
            let work = work_area(0, 0);
            (work.right - default_h - 40, work.bottom - default_h - 40, default_h, false)
        });

        let (state, stamp) = read_state();
        // no word yet: forget the stamp so the first poll looks again
        let stamp = state.as_ref().and(stamp);
        let state = state.unwrap_or_default();
        let turns = Turns::new(act_for(&state));
        let clip = turns.clip();
        let mut app = App {
            mascot,
            bubble,
            size_tag,
            menu_win,
            mc: Canvas::new(),
            bubble_view: Bubble::new(font(14.0 * ui, 700, "Leelawadee UI"), font(12.0 * ui, 700, "Leelawadee UI")),
            x,
            y,
            height,
            strip: clip.strip(),
            turns,
            state: state.clone(),
            stamp,
            frame: 0,
            press: Press::None,
            resize_mode: false,
            show_status,
            auto_hidden: false,
            minute: minute_of_day(),
            status: StatusText::load(&data_dir().join("status.txt")),
            menu: Menu::new(font(14.0 * ui, 700, "Leelawadee UI"), ui),
            tracking_leave: false,
            ui,
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
