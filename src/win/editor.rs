//! The status editor: a custom-drawn window for each state's words and color, and the
//! bubble's show and auto-hide settings.

use super::*;

// the editor's controls
pub(super) const ID_EDIT0: i32 = 100;
/// What the seconds box starts at when auto hide is first ticked.
pub(super) const DEFAULT_AUTOHIDE: u32 = 5;

// the editor's quiet palette
pub(super) const INK: Rgba = Rgba(31, 35, 40, 255);
pub(super) const MUTED: Rgba = Rgba(107, 114, 128, 255);
pub(super) const FAINT: Rgba = Rgba(156, 163, 175, 255);
pub(super) const FIELD: Rgba = Rgba(243, 245, 248, 255);
pub(super) const FIELD_HOVER: Rgba = Rgba(232, 235, 240, 255);
pub(super) const LINE: Rgba = Rgba(226, 230, 236, 255);
pub(super) const TRACK_OFF: Rgba = Rgba(206, 212, 220, 255);
pub(super) const FOCUS_RING: Rgba = Rgba(74, 152, 216, 60);
pub(super) const NAVY_HOVER: Rgba = Rgba(38, 74, 148, 255);

/// The open status-text editor: its window, its text boxes and the choices being made.
pub(super) struct Editor {
    pub(super) hwnd: HWND,
    pub(super) edits: Vec<HWND>,
    /// each state's text color as picked so far
    pub(super) colors: [Rgba; 5],
    /// the state whose words were last focused: the one the swatches color, and the
    /// bubble previews
    pub(super) selected: usize,
    pub(super) show: bool,
    pub(super) autohide: bool,
    pub(super) seconds: u32,
    pub(super) hover: Option<Hit>,
    /// the text box with the keyboard focus
    pub(super) focus: Option<usize>,
    pub(super) tracking: bool,
    pub(super) canvas: Canvas,
    pub(super) fonts: EditorFonts,
    /// the text boxes' background, the same as the field painted around them
    pub(super) field_brush: HBRUSH,
}

pub(super) struct EditorFonts {
    pub(super) title: HFONT,
    pub(super) label: HFONT,
    pub(super) heading: HFONT,
    pub(super) hint: HFONT,
    pub(super) button: HFONT,
    pub(super) edit: HFONT,
}

/// What in the editor takes a click.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Hit {
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
pub(super) type Area = (f32, f32, f32, f32);

/// Where everything in the editor sits.
pub(super) struct EditorLayout {
    pub(super) w: i32,
    pub(super) h: i32,
    pub(super) header: f32,
    pub(super) close: Area,
    /// the tops of the section headings
    pub(super) messages: f32,
    pub(super) colors: f32,
    pub(super) bubble: f32,
    pub(super) fields: [Area; 5],
    pub(super) swatches: [Area; 8],
    pub(super) custom: Area,
    pub(super) show_row: Area,
    pub(super) auto_row: Area,
    /// the two switches
    pub(super) show: Area,
    pub(super) autohide: Area,
    /// the seconds stepper, and its minus and plus ends
    pub(super) seconds: Area,
    pub(super) less: Area,
    pub(super) more: Area,
    pub(super) divider: f32,
    pub(super) reset: Area,
    pub(super) cancel: Area,
    pub(super) save: Area,
}

pub(super) fn editor_layout(ui: f32) -> EditorLayout {
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

pub(super) fn inside(r: Area, x: f32, y: f32) -> bool {
    x >= r.0 && x < r.2 && y >= r.1 && y < r.3
}

pub(super) fn grow(r: Area, by: f32) -> Area {
    (r.0 - by, r.1 - by, r.2 + by, r.3 + by)
}

pub(super) fn rect(r: Area) -> RECT {
    RECT { left: r.0 as i32, top: r.1 as i32, right: r.2 as i32, bottom: r.3 as i32 }
}

/// A filled rounded rectangle.
pub(super) fn pill(c: &mut Canvas, r: Area, radius: f32, color: Rgba) {
    c.fill_sdf(grow(r, 1.0), color, 1.0, round_rect(r.0, r.1, r.2, r.3, radius));
}

/// A plus centered in `r`, or a cross when `turned`: arms `k` long, `w` thick.
pub(super) fn cross(c: &mut Canvas, r: Area, k: f32, w: f32, color: Rgba, turned: bool) {
    let (cx, cy) = ((r.0 + r.2) / 2.0, (r.1 + r.3) / 2.0);
    let (a, b) = if turned { ((k, k), (k, -k)) } else { ((k, 0.0), (0.0, k)) };
    c.fill_sdf(r, color, 1.0, segment((cx - a.0, cy - a.1), (cx + a.0, cy + a.1), w));
    c.fill_sdf(r, color, 1.0, segment((cx - b.0, cy - b.1), (cx + b.0, cy + b.1), w));
}

/// An on/off switch.
pub(super) fn toggle(c: &mut Canvas, r: Area, on: bool) {
    let h = r.3 - r.1;
    pill(c, r, h / 2.0, if on { BLUE } else { TRACK_OFF });
    let knob = h / 2.0 - h * 0.14;
    let cx = if on { r.2 - h / 2.0 } else { r.0 + h / 2.0 };
    let cy = (r.1 + r.3) / 2.0;
    c.fill_sdf(grow(r, 2.0), Rgba(16, 32, 70, 40), 2.5, circle(cx, cy + 0.8, knob));
    c.fill_sdf(r, WHITE, 1.0, circle(cx, cy, knob));
}

impl App {
    // ---- the status-text editor -------------------------------------------------

    pub(super) fn open_editor(&mut self) {
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

    pub(super) fn paint_editor(&mut self, hdc: HDC) {
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
    pub(super) fn editor_hit(&self, client: POINT) -> Option<Hit> {
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
    pub(super) fn editor_hover(&mut self, hit: Option<Hit>) {
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

    pub(super) fn editor_focus(&mut self, focus: Option<usize>) {
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
    pub(super) fn editor_act(&mut self, hit: Hit) {
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
    pub(super) fn pick_color(&mut self, c: Rgba) {
        if let Some(e) = self.editor.as_mut() {
            e.colors[e.selected] = c;
            unsafe {
                InvalidateRect(e.hwnd, null(), 0);
                InvalidateRect(e.edits[e.selected], null(), 1);
            }
        }
        self.update_bubble();
    }

    pub(super) fn reset_editor(&mut self) {
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

    pub(super) fn close_editor(&mut self, keep: bool) {
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

/// A custom color from the system color dialog, outside any borrow of the app: the
/// dialog runs its own message loop while it is open.
pub(super) unsafe fn choose_custom_color(owner: HWND) {
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

pub(super) unsafe extern "system" fn editor_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
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
