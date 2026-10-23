//! The status editor's look: where its parts sit, how it is painted and what a click on it
//! means, for each state's words and color and the bubble's show and auto-hide settings.
//! The text boxes are each platform's own controls, placed over the painted fields.

use crate::gfx::*;
use crate::status::*;

/// What the seconds box starts at when auto hide is first ticked.
const DEFAULT_AUTOHIDE: u32 = 5;

// the editor's quiet palette
pub const INK: Rgba = Rgba(31, 35, 40, 255);
const MUTED: Rgba = Rgba(107, 114, 128, 255);
const FAINT: Rgba = Rgba(156, 163, 175, 255);
pub const FIELD: Rgba = Rgba(243, 245, 248, 255);
const FIELD_HOVER: Rgba = Rgba(232, 235, 240, 255);
const LINE: Rgba = Rgba(226, 230, 236, 255);
const TRACK_OFF: Rgba = Rgba(206, 212, 220, 255);
const FOCUS_RING: Rgba = Rgba(74, 152, 216, 60);
const NAVY_HOVER: Rgba = Rgba(38, 74, 148, 255);

pub struct EditorFonts {
    pub title: Font,
    pub label: Font,
    pub heading: Font,
    pub hint: Font,
    pub button: Font,
    pub edit: Font,
}

/// What in the editor takes a click.
#[derive(Clone, Copy, PartialEq)]
pub enum Hit {
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

/// (left, top, right, bottom) in the editor's pixels.
pub type Area = (f32, f32, f32, f32);

/// Where everything in the editor sits.
pub struct EditorLayout {
    pub w: i32,
    pub h: i32,
    pub header: f32,
    close: Area,
    /// the tops of the section headings
    messages: f32,
    colors: f32,
    bubble: f32,
    pub fields: [Area; 5],
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

pub fn editor_layout(ui: f32) -> EditorLayout {
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

fn rect(r: Area) -> Rect {
    Rect { left: r.0 as i32, top: r.1 as i32, right: r.2 as i32, bottom: r.3 as i32 }
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

/// What under (x, y), in the editor's pixels, takes a click, if anything.
pub fn editor_hit(ui: f32, x: f32, y: f32) -> Option<Hit> {
    let l = editor_layout(ui);
    let mut targets = vec![(Hit::Close, l.close), (Hit::Custom, l.custom), (Hit::Less, l.less), (Hit::More, l.more)];
    targets.extend(l.swatches.iter().enumerate().map(|(i, r)| (Hit::Swatch(i), *r)));
    let pad = l.swatches[0].0 - 4.0 * ui;
    targets.extend(l.fields.iter().enumerate().map(|(i, f)| (Hit::State(i), (pad, f.1, f.0 - 4.0 * ui, f.3))));
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

/// The choices made in the open editor, kept until Save. The words are in the platform's
/// text boxes.
pub struct Draft {
    /// each state's text color as picked so far
    pub colors: [Rgba; 5],
    /// the state whose words were last focused: the one the swatches color, and the
    /// bubble previews
    pub selected: usize,
    pub show: bool,
    pub autohide: bool,
    pub seconds: u32,
    pub hover: Option<Hit>,
    /// the text box with the keyboard focus
    pub focus: Option<usize>,
}

impl Draft {
    pub fn new(status: &StatusText, show_status: bool, state: &str) -> Draft {
        Draft {
            colors: status.colors,
            selected: index(state),
            show: show_status,
            autohide: status.autohide > 0,
            seconds: if status.autohide > 0 { status.autohide } else { DEFAULT_AUTOHIDE },
            hover: None,
            focus: None,
        }
    }

    /// Flip a switch or step the seconds, for a click on `hit`. The other hits are the
    /// platform's to act on.
    pub fn act(&mut self, hit: Hit) {
        match hit {
            Hit::Show => self.show = !self.show,
            Hit::AutoHide => self.autohide = !self.autohide,
            Hit::Less if self.autohide => self.seconds = self.seconds.saturating_sub(1).max(1),
            Hit::More if self.autohide => self.seconds = (self.seconds + 1).min(MAX_AUTOHIDE),
            _ => {}
        }
    }

    /// Write the choices, and `texts` (each box's words), into `status`.
    pub fn commit(&self, status: &mut StatusText, texts: [String; 5]) {
        for (i, text) in texts.into_iter().enumerate() {
            let text = text.replace(['\r', '\n'], " ").trim().to_string();
            status.texts[i] = if text.is_empty() { STATES[i].2.to_string() } else { text };
        }
        status.colors = self.colors;
        status.autohide = if self.autohide { self.seconds.clamp(1, MAX_AUTOHIDE) } else { 0 };
    }

    /// Paint the whole editor, apart from the text in its boxes, into `c`.
    pub fn paint(&self, c: &mut Canvas, fonts: &EditorFonts, ui: f32) {
        let l = editor_layout(ui);
        let s = |v: f32| v * ui;
        let pad = s(20.0);
        let right = l.w as f32 - pad;
        let hover = self.hover;
        c.reset(l.w, l.h);
        c.fill(WHITE);

        // header: the title, and a close button
        c.label(fonts.title, "Status text", rect((pad, 0.0, right, l.header)), INK, Align::Left);
        if hover == Some(Hit::Close) {
            pill(c, l.close, s(8.0), FIELD);
        }
        let ink = if hover == Some(Hit::Close) { INK } else { MUTED };
        cross(c, l.close, s(5.0), s(1.6), ink, true);

        let heading = |c: &mut Canvas, y: f32, text: &str| {
            c.label(fonts.heading, text, rect((pad, y, right, y + s(18.0))), FAINT, Align::Left);
        };

        // the words for each state, each beside its state's dot
        heading(c, l.messages, "MESSAGES");
        let hint = rect((pad, l.messages, right, l.messages + s(18.0)));
        c.label(fonts.hint, "Type \\n for a new line", hint, FAINT, Align::Right);
        for (i, f) in l.fields.iter().enumerate() {
            let cy = (f.1 + f.3) / 2.0;
            c.fill_sdf((pad, cy - s(5.0), pad + s(10.0), cy + s(5.0)), accent(STATES[i].0), 1.0, circle(pad + s(4.0), cy, s(4.0)));
            let ink = if self.selected == i || hover == Some(Hit::State(i)) { INK } else { MUTED };
            c.label(fonts.label, STATES[i].1, rect((pad + s(16.0), f.1, f.0, f.3)), ink, Align::Left);
            if self.focus == Some(i) {
                pill(c, grow(*f, s(3.0)), s(13.0), FOCUS_RING);
            }
            if self.selected == i {
                pill(c, *f, s(10.0), BLUE);
            } else {
                pill(c, *f, s(10.0), LINE);
            }
            pill(c, grow(*f, -s(1.0)), s(9.0), FIELD);
        }

        // the text color: the presets, then the system picker
        heading(c, l.colors, &format!("TEXT COLOR  ·  {}", STATES[self.selected].1.to_uppercase()));
        let color = self.colors[self.selected];
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
        c.label(fonts.label, "Show status", rect(l.show_row), INK, Align::Left);
        toggle(c, l.show, self.show);
        c.label(fonts.label, "Auto hide", rect(l.auto_row), INK, Align::Left);
        toggle(c, l.autohide, self.autohide);
        let st = l.seconds;
        pill(c, st, (st.3 - st.1) / 2.0, FIELD);
        for (hit, r) in [(Hit::Less, l.less), (Hit::More, l.more)] {
            let usable = self.autohide && if hit == Hit::Less { self.seconds > 1 } else { self.seconds < MAX_AUTOHIDE };
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
        let ink = if self.autohide { INK } else { FAINT };
        c.label(fonts.button, &format!("{} s", self.seconds), rect((l.less.2, st.1, l.more.0, st.3)), ink, Align::Center);

        // footer
        let line = l.divider;
        c.fill_sdf((pad, line, right, line + 1.0), LINE, 1.0, |_, _| -1.0);
        if hover == Some(Hit::Reset) {
            pill(c, l.reset, s(10.0), FIELD);
        }
        let ink = if hover == Some(Hit::Reset) { INK } else { MUTED };
        c.label(fonts.button, "Reset", rect(l.reset), ink, Align::Center);
        pill(c, l.cancel, s(10.0), if hover == Some(Hit::Cancel) { FIELD_HOVER } else { FIELD });
        c.label(fonts.button, "Cancel", rect(l.cancel), INK, Align::Center);
        pill(c, l.save, s(10.0), if hover == Some(Hit::Save) { NAVY_HOVER } else { NAVY });
        c.label(fonts.button, "Save", rect(l.save), WHITE, Align::Center);
    }
}
