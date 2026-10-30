//! The Mac seal: borderless, transparent, always-on-top panels (the seal, its bubble, the
//! size readout, the icon menu and the status editor) that show the shared drawings, in an
//! accessory app with no Dock icon.
//!
//! Positions and sizes are kept in pixels with the origin at the top left of the main
//! screen, as on Windows, so the shared code and the placement file work the same; they are
//! turned into AppKit's points (origin at the bottom left) only where a window is placed.

mod editor;
mod menu;

use std::cell::RefCell;
use std::os::unix::net::{UnixListener, UnixStream};
use std::time::{Instant, SystemTime};

use block2::RcBlock;
use editor::*;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{define_class, msg_send, AllocAnyThread, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{
    NSApplication, NSApplicationActivationPolicy, NSBackingStoreType, NSColor, NSEvent, NSPanel, NSResponder, NSScreen, NSStatusWindowLevel, NSView, NSWindow,
    NSWindowCollectionBehavior, NSWindowStyleMask,
};
use objc2_core_foundation::{CGPoint, CGRect, CGSize};
use objc2_foundation::{NSObject, NSRunLoop, NSRunLoopCommonModes, NSTimer};

use crate::bubble::*;
use crate::editor::*;
use crate::gfx::*;
use crate::menu::*;
use crate::seal::*;
use crate::status::*;
use crate::store::*;

extern "C" {
    /// libSystem: return free memory held by a malloc zone (all zones for null) to the system.
    fn malloc_zone_pressure_relief(zone: *mut std::ffi::c_void, goal: usize) -> usize;
}

/// Decoding a clip leaves megabytes of freed buffers that the Mac's malloc keeps for reuse.
/// The next decode is a state change away, so give them back to the system.
fn trim_heap() {
    unsafe { malloc_zone_pressure_relief(std::ptr::null_mut(), 0) };
}

/// What a press on the seal is turning into.
enum Press {
    None,
    /// down, not yet a drag: a release now is a click
    Pending { at: (i32, i32), grab: (i32, i32), since: Instant },
    Drag { grab: (i32, i32) },
    Resize { at: (i32, i32), h0: i32 },
}

struct App {
    mtm: MainThreadMarker,
    /// the seal's panel; its content view, a `SealView`, takes the pointer
    mascot: Retained<NSPanel>,
    bubble_win: Retained<NSPanel>,
    tag_win: Retained<NSPanel>,
    menu_win: Retained<NSPanel>,
    menu: Menu,
    /// the menu has had the keyboard since it opened: losing it closes the menu
    menu_key: bool,
    editor: Option<Editor>,
    editor_fonts: EditorFonts,
    /// takes the editor's text-box and color-panel messages
    editor_target: Retained<EditorTarget>,
    /// the seal as drawn this frame
    mc: Canvas,
    bubble: Bubble,
    /// the display's scale: pixels per point
    scale: f32,
    x: i32,
    y: i32,
    height: i32,
    strip: Strip,
    /// the act playing: the state's, or while editing, the edited state's
    turns: Turns,
    state: String,
    stamp: Option<SystemTime>,
    frame: usize,
    frame_at: Instant,
    next_poll: Instant,
    press: Press,
    resize_mode: bool,
    show_status: bool,
    status: StatusText,
    /// when the bubble hides itself, with auto hide on
    hide_at: Option<Instant>,
    auto_hidden: bool,
    /// the minute of the day last looked at for a break
    minute: u32,
    /// when the bubble's next frame is due, while it is up or on its way out
    bubble_at: Option<Instant>,
}

thread_local! {
    static APP: RefCell<Option<App>> = const { RefCell::new(None) };
}

/// Run `f` on the app, unless it is already busy further up this call stack.
fn with<R>(f: impl FnOnce(&mut App) -> R) -> Option<R> {
    APP.with(|a| a.try_borrow_mut().ok().and_then(|mut guard| guard.as_mut().map(f)))
}

define_class!(
    // SAFETY: NSView has no subclassing requirements, and SealView has no Drop.
    #[unsafe(super(NSView))]
    #[thread_kind = MainThreadOnly]
    #[name = "SealPetView"]
    struct SealView;

    impl SealView {
        #[unsafe(method(acceptsFirstMouse:))]
        fn accepts_first_mouse(&self, _event: Option<&NSEvent>) -> bool {
            true
        }

        #[unsafe(method(mouseDown:))]
        fn mouse_down(&self, event: &NSEvent) {
            let at = event.locationInWindow();
            with(|app| app.press_down(at));
        }

        #[unsafe(method(mouseDragged:))]
        fn mouse_dragged(&self, _event: &NSEvent) {
            with(|app| app.press_move());
        }

        #[unsafe(method(mouseUp:))]
        fn mouse_up(&self, _event: &NSEvent) {
            with(|app| app.press_up());
        }

        #[unsafe(method(rightMouseUp:))]
        fn right_mouse_up(&self, _event: &NSEvent) {
            with(|app| app.toggle_menu());
        }

        #[unsafe(method(scrollWheel:))]
        fn scroll_wheel(&self, event: &NSEvent) {
            let notches = if event.hasPreciseScrollingDeltas() { event.scrollingDeltaY() / 10.0 } else { event.scrollingDeltaY() };
            with(|app| app.wheel(notches as f32));
        }
    }
);

impl SealView {
    fn new(mtm: MainThreadMarker) -> Retained<SealView> {
        let frame = CGRect::new(CGPoint::new(0.0, 0.0), CGSize::new(1.0, 1.0));
        unsafe { msg_send![SealView::alloc(mtm), initWithFrame: frame] }
    }
}

define_class!(
    // SAFETY: NSPanel has no subclassing requirements, and KeyPanel has no Drop.
    #[unsafe(super(NSPanel, NSWindow, NSResponder, NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "SealPetKeyPanel"]
    /// A borderless panel that can still take the keyboard, for the menu's Escape and the
    /// editor's text boxes.
    struct KeyPanel;

    impl KeyPanel {
        #[unsafe(method(canBecomeKeyWindow))]
        fn can_become_key_window(&self) -> bool {
            true
        }
    }
);

/// A borderless, transparent panel above other windows, on every Space, that never makes
/// the app active. `clicks` false lets the pointer through to what is underneath; `key`
/// lets it take the keyboard.
fn panel(mtm: MainThreadMarker, content: &NSView, clicks: bool, key: bool) -> Retained<NSPanel> {
    let frame = CGRect::new(CGPoint::new(0.0, 0.0), CGSize::new(1.0, 1.0));
    let style = NSWindowStyleMask::Borderless | NSWindowStyleMask::NonactivatingPanel;
    let panel = if key {
        let panel: Retained<KeyPanel> = unsafe { msg_send![KeyPanel::alloc(mtm), initWithContentRect: frame, styleMask: style, backing: NSBackingStoreType::Buffered, defer: false] };
        Retained::into_super(panel)
    } else {
        NSPanel::initWithContentRect_styleMask_backing_defer(NSPanel::alloc(mtm), frame, style, NSBackingStoreType::Buffered, false)
    };
    // the app keeps its own reference to each panel
    unsafe { panel.setReleasedWhenClosed(false) };
    panel.setOpaque(false);
    panel.setBackgroundColor(Some(&NSColor::clearColor()));
    panel.setHasShadow(false);
    panel.setLevel(NSStatusWindowLevel);
    panel.setIgnoresMouseEvents(!clicks);
    panel.setHidesOnDeactivate(false);
    panel.setCollectionBehavior(
        NSWindowCollectionBehavior::CanJoinAllSpaces | NSWindowCollectionBehavior::Stationary | NSWindowCollectionBehavior::FullScreenAuxiliary | NSWindowCollectionBehavior::IgnoresCycle,
    );
    content.setWantsLayer(true);
    panel.setContentView(Some(content));
    panel
}

fn plain_view(mtm: MainThreadMarker) -> Retained<NSView> {
    let frame = CGRect::new(CGPoint::new(0.0, 0.0), CGSize::new(1.0, 1.0));
    NSView::initWithFrame(NSView::alloc(mtm), frame)
}

/// The main screen's height in points: AppKit counts y up from its bottom edge.
fn main_height(mtm: MainThreadMarker) -> f64 {
    NSScreen::screens(mtm).firstObject().map(|s| s.frame().size.height).unwrap_or(0.0)
}

/// Show `canvas` as `win`'s content, its top-left at pixel (x, y), faded by `alpha`.
fn present(mtm: MainThreadMarker, win: &NSPanel, canvas: &Canvas, x: i32, y: i32, alpha: u8, scale: f32) {
    let s = scale as f64;
    let (w, h) = (canvas.w as f64 / s, canvas.h as f64 / s);
    let origin = CGPoint::new(x as f64 / s, main_height(mtm) - y as f64 / s - h);
    win.setFrame_display(CGRect::new(origin, CGSize::new(w, h)), false);
    show_canvas(win, canvas, scale);
    win.setAlphaValue(alpha as f64 / 255.0);
    if !win.isVisible() {
        win.orderFrontRegardless();
    }
}

/// Show `canvas` as `win`'s content, leaving the window where it is.
fn show_canvas(win: &NSPanel, canvas: &Canvas, scale: f32) {
    if let (Some(view), Some(image)) = (win.contentView(), canvas.image()) {
        if let Some(layer) = view.layer() {
            layer.setContentsScale(scale as f64);
            let image: &AnyObject = unsafe { &*(&*image as *const _ as *const AnyObject) };
            unsafe { layer.setContents(Some(image)) };
        }
    }
}

/// A point in `view`, from AppKit's points up from its bottom left to pixels down from its
/// top left.
fn view_pixels(view: &NSView, at: CGPoint, scale: f32) -> (i32, i32) {
    let s = scale as f64;
    ((at.x * s) as i32, ((view.bounds().size.height - at.y) * s) as i32)
}

impl App {
    fn size(&self) -> (i32, i32) {
        ((self.height as f32 * FRAME_W as f32 / FRAME_H as f32).round() as i32, self.height)
    }

    /// The pointer, in pixels from the main screen's top left.
    fn cursor(&self) -> (i32, i32) {
        let p = NSEvent::mouseLocation();
        let s = self.scale as f64;
        ((p.x * s) as i32, ((main_height(self.mtm) - p.y) * s) as i32)
    }

    /// The usable part (no menu bar or Dock) of the screen holding pixel (x, y).
    fn work_area(&self, x: i32, y: i32) -> Bounds {
        let s = self.scale as f64;
        let top = main_height(self.mtm);
        let areas: Vec<Bounds> = NSScreen::screens(self.mtm)
            .iter()
            .map(|screen| {
                let f = screen.visibleFrame();
                ((f.origin.x * s) as i32, ((top - f.origin.y - f.size.height) * s) as i32, ((f.origin.x + f.size.width) * s) as i32, ((top - f.origin.y) * s) as i32)
            })
            .collect();
        areas.iter().copied().find(|a| x >= a.0 && x < a.2 && y >= a.1 && y < a.3).or(areas.first().copied()).unwrap_or((0, 0, 1280, 800))
    }

    /// How far the seal's window reaches past its box, for a clip whose art grows past its
    /// frame (see `Strip::zoomed`).
    fn overhang(&self) -> (i32, i32, i32, i32) {
        let (_, h) = self.size();
        let (l, t, r, b) = self.strip.pad;
        let k = h as f32 / (self.strip.fh - t - b) as f32;
        let at = |v: usize| (v as f32 * k).round() as i32;
        (at(l), at(t), at(r), at(b))
    }

    /// The resize handle: center and radius, in the box.
    fn handle(&self) -> (f32, f32, f32) {
        let (w, h) = self.size();
        let r = (7.0 * self.scale).min(h as f32 / 5.0).max(4.0);
        (w as f32 - r - 2.0, h as f32 - r - 2.0, r)
    }

    // ---- the seal --------------------------------------------------------------

    fn draw_mascot(&mut self) {
        let (w, h) = self.size();
        let (l, t, r, b) = self.overhang();
        self.mc.reset(w + l + r, h + t + b);
        self.strip.draw(self.frame, &mut self.mc);
        if self.resize_mode {
            let ui = self.scale;
            let (ox, oy) = (l as f32, t as f32);
            let (wf, hf) = (w as f32, h as f32);
            let frame = round_rect(ox + 1.0, oy + 1.0, ox + wf - 1.0, oy + hf - 1.0, 10.0 * ui);
            self.mc.fill_sdf((ox, oy, ox + wf, oy + hf), Rgba(74, 152, 216, 210), 1.0, move |x, y| frame(x, y).abs() - 0.9 * ui);
            let (cx, cy, r) = self.handle();
            let (cx, cy) = (cx + ox, cy + oy);
            self.mc.fill_sdf((cx - r - 2.0, cy - r - 2.0, cx + r + 2.0, cy + r + 2.0), BLUE, 1.0, circle(cx, cy, r));
            self.mc.fill_sdf((cx - r, cy - r, cx + r, cy + r), WHITE, 1.0, circle(cx, cy, r - 1.6 * ui));
        }
        let alpha = if matches!(self.press, Press::Drag { .. }) { 225 } else { 255 };
        present(self.mtm, &self.mascot, &self.mc, self.x - l, self.y - t, alpha, self.scale);
    }

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
        self.strip.release();
        self.strip = self.turns.clip().strip();
        trim_heap();
        self.frame = 0;
        self.draw_mascot();
    }

    fn move_to(&mut self, x: i32, y: i32) {
        self.x = x;
        self.y = y;
        self.draw_mascot();
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
    }

    /// Everything that moves with time, run about 60 times a second.
    fn tick(&mut self) {
        let now = Instant::now();
        if now.duration_since(self.frame_at).as_millis() as u32 >= self.turns.clip().interval {
            self.frame_at = now;
            self.frame = (self.frame + 1) % self.strip.frames.max(1);
            if self.frame == 0 && self.turns.looped() {
                self.play();
                // a scene's words come and go with its clips
                self.update_bubble();
            } else {
                self.draw_mascot();
            }
        }
        if now >= self.next_poll {
            self.next_poll = now + std::time::Duration::from_millis(POLL_MS as u64);
            self.poll();
        }
        if let Press::Pending { grab, since, .. } = self.press {
            if now.duration_since(since).as_millis() as u32 >= HOLD_MS {
                self.start_drag(grab);
            }
        }
        if self.menu.open {
            self.menu_tick();
        }
        if self.editor.is_some() {
            self.editor_tick();
        }
        if self.hide_at.is_some_and(|at| now >= at) {
            self.hide_at = None;
            self.auto_hidden = true;
            self.update_bubble();
        }
        if self.bubble_at.is_some_and(|at| now >= at) {
            self.bubble_frame();
        }
    }

    // ---- pointer on the seal ---------------------------------------------------

    /// A press at `at`, in the window's points from its bottom left.
    fn press_down(&mut self, at: CGPoint) {
        let (l, t, _, _) = self.overhang();
        let s = self.scale as f64;
        let win_h = self.mc.h as f64;
        // in the box, in pixels from its top left
        let (bx, by) = ((at.x * s) as f32 - l as f32, (win_h - at.y * s) as f32 - t as f32);
        let cursor = self.cursor();
        let (cx, cy, r) = self.handle();
        if self.resize_mode && ((bx - cx).powi(2) + (by - cy).powi(2)).sqrt() <= r + 5.0 {
            self.press = Press::Resize { at: cursor, h0: self.height };
        } else {
            self.press = Press::Pending { at: cursor, grab: (cursor.0 - self.x, cursor.1 - self.y), since: Instant::now() };
        }
    }

    fn start_drag(&mut self, grab: (i32, i32)) {
        self.press = Press::Drag { grab };
        self.close_menu();
        self.draw_mascot();
    }

    fn press_move(&mut self) {
        let p = self.cursor();
        match self.press {
            Press::Pending { at, grab, .. } => {
                if (p.0 - at.0).abs() > DRAG_SLOP || (p.1 - at.1).abs() > DRAG_SLOP {
                    self.start_drag(grab);
                    self.move_to(p.0 - grab.0, p.1 - grab.1);
                }
            }
            Press::Drag { grab } => self.move_to(p.0 - grab.0, p.1 - grab.1),
            Press::Resize { at, h0 } => {
                let dx = ((p.0 - at.0) as f32 * FRAME_H as f32 / FRAME_W as f32) as i32;
                let dy = p.1 - at.1;
                self.set_height(h0 + if dx.abs() > dy.abs() { dx } else { dy });
            }
            Press::None => {}
        }
    }

    /// The press ends: a click leaves resizing, or else opens or closes the menu.
    fn press_up(&mut self) {
        match std::mem::replace(&mut self.press, Press::None) {
            Press::Pending { .. } => {
                if self.resize_mode {
                    self.set_resize_mode(false);
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

    fn wheel(&mut self, notches: f32) {
        if self.resize_mode && notches != 0.0 {
            let h = (self.height as f32 * 1.1f32.powf(notches)).round() as i32;
            self.set_height(if h == self.height { h + notches.signum() as i32 } else { h });
            self.save();
        }
    }

    fn set_resize_mode(&mut self, on: bool) {
        self.resize_mode = on;
        self.draw_mascot();
        self.update_bubble();
    }

    fn feed(&mut self) {
        self.turns.cut_in(FEED);
        self.play();
        self.update_bubble();
    }

    fn toggle_status(&mut self) {
        self.show_status = !self.show_status;
        self.restart_autohide();
        self.save();
    }

    // ---- the status bubble -----------------------------------------------------

    /// Show the bubble again and, when auto hide is on, start counting down to hiding it.
    fn restart_autohide(&mut self) {
        self.auto_hidden = false;
        self.hide_at = (self.show_status && self.status.autohide > 0).then(|| Instant::now() + std::time::Duration::from_secs(self.status.autohide as u64));
        self.update_bubble();
    }

    fn update_bubble(&mut self) {
        let (w, h) = self.size();
        let seal = (self.x, self.y, w, h);
        let tag = if self.resize_mode && !self.menu.open {
            let work = self.work_area(self.x + w / 2, self.y + h);
            let ((x, y), bounds) = self.bubble.size_tag(self.height, seal, work, self.scale);
            present(self.mtm, &self.tag_win, &self.bubble.tag, x, y, 255, self.scale);
            Some(bounds)
        } else {
            self.tag_win.orderOut(None);
            None
        };
        // a break or a feed says its words whether or not the bubble is on
        let scene = self.turns.scene().filter(|_| self.editor.is_none() && !self.resize_mode);
        let visible = ((self.show_status && !self.auto_hidden) || self.resize_mode || self.editor.is_some() || scene.is_some()) && !self.menu.open;
        if !visible {
            self.bubble.hide();
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
            let typed = e.fields[i].stringValue().to_string();
            (if typed.trim().is_empty() { STATES[i].2.to_string() } else { typed }, Some(accent(STATES[i].0)))
        } else {
            (self.status.text(&self.state).to_string(), Some(accent(&self.state)))
        };
        let ink = match &self.editor {
            Some(e) => e.draft.colors[e.draft.selected],
            None if scene.is_some() => DEFAULT_COLOR,
            None => self.status.color(&self.state),
        };
        let content = Content { text: &text, dot, ink, key: &key };
        let work = self.work_area(self.x + w / 2, self.y);
        self.bubble.show(&content, seal, work, tag, self.scale);
        self.bubble_frame();
    }

    fn bubble_frame(&mut self) {
        match self.bubble.frame(self.scale) {
            Frame::Hidden => {
                self.bubble_at = None;
                self.bubble_win.orderOut(None);
            }
            Frame::Shown { x, y, alpha, moving } => {
                present(self.mtm, &self.bubble_win, &self.bubble.shown, x, y, alpha, self.scale);
                self.bubble_at = Some(Instant::now() + std::time::Duration::from_millis(if moving { 15 } else { 33 }));
            }
        }
    }
}

/// Only one seal at a time: the first holds a socket in the data folder; a later one finds
/// it answering and leaves. A socket left by a seal that crashed answers no one, and is
/// replaced.
fn single_instance() -> Option<UnixListener> {
    let dir = data_dir();
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("instance.sock");
    if UnixStream::connect(&path).is_ok() {
        return None;
    }
    let _ = std::fs::remove_file(&path);
    UnixListener::bind(&path).ok()
}

pub fn run() {
    let Some(_instance) = single_instance() else { return };
    let mtm = MainThreadMarker::new().expect("ai-mascot-hook runs on the main thread");
    let app = NSApplication::sharedApplication(mtm);
    // no Dock icon or menu bar: the seal is all there is
    app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);

    let scale = NSScreen::mainScreen(mtm).map(|s| s.backingScaleFactor()).unwrap_or(1.0) as f32;
    let default_h = ((140.0 * scale).round() as i32).clamp(MIN_H, MAX_H);
    let view = SealView::new(mtm);
    let mascot = panel(mtm, &view, true, false);
    let bubble_win = panel(mtm, &plain_view(mtm), false, false);
    let tag_win = panel(mtm, &plain_view(mtm), false, false);
    let menu_win = panel(mtm, &menu::MenuView::new(mtm), true, true);

    let (state, stamp) = read_state();
    // no word yet: forget the stamp so the first poll looks again
    let stamp = state.as_ref().and(stamp);
    let state = state.unwrap_or_default();
    let turns = Turns::new(act_for(&state));
    let mut seal = App {
        mtm,
        mascot,
        bubble_win,
        tag_win,
        menu_win,
        menu: Menu::new(font(14.0 * scale, 700), scale),
        menu_key: false,
        editor: None,
        editor_fonts: EditorFonts {
            title: font(17.0 * scale, 600),
            label: font(14.0 * scale, 400),
            heading: font(11.5 * scale, 700),
            hint: font(12.0 * scale, 400),
            button: font(14.0 * scale, 600),
            // for the text boxes, which AppKit sizes in points
            edit: font(14.5, 400),
        },
        editor_target: EditorTarget::new(mtm),
        mc: Canvas::new(),
        bubble: Bubble::new(font(14.0 * scale, 700), font(12.0 * scale, 700)),
        scale,
        x: 0,
        y: 0,
        height: default_h,
        strip: turns.clip().strip(),
        turns,
        state,
        stamp,
        frame: 0,
        frame_at: Instant::now(),
        next_poll: Instant::now(),
        press: Press::None,
        resize_mode: false,
        show_status: false,
        status: StatusText::load(&data_dir().join("status.txt")),
        hide_at: None,
        auto_hidden: false,
        minute: minute_of_day(),
        bubble_at: None,
    };
    match read_placement(default_h) {
        Some((x, y, height, show_status)) => {
            (seal.x, seal.y, seal.height, seal.show_status) = (x, y, height, show_status);
        }
        None => {
            // first run: bottom right, above the Dock
            let work = seal.work_area(0, 0);
            seal.x = work.2 - default_h - (40.0 * scale) as i32;
            seal.y = work.3 - default_h - (40.0 * scale) as i32;
        }
    }
    seal.draw_mascot();
    seal.restart_autohide();
    trim_heap();
    APP.with(|a| *a.borrow_mut() = Some(seal));

    let tick = RcBlock::new(|_timer| {
        with(|app| app.tick());
    });
    let timer = unsafe { NSTimer::scheduledTimerWithTimeInterval_repeats_block(1.0 / 60.0, true, &tick) };
    // keep the seal moving while a text box or the color panel tracks the pointer, too
    unsafe { NSRunLoop::currentRunLoop().addTimer_forMode(&timer, NSRunLoopCommonModes) };
    app.run();
}
