//! The settings editor's look: where its parts sit, how it is painted and what a click on it
//! means. Two tabs share one column: each state's words and color, then the bubble's show
//! and auto-hide switches with the break and feed. The text boxes are each platform's own
//! controls, placed over the painted fields and shown with their tab.

use crate::gfx::*;
use crate::phrases::{split, SETS};
use crate::status::*;

/// What the seconds box starts at when auto hide is first ticked.
const DEFAULT_AUTOHIDE: u32 = 5;
/// Seconds per click of the idle nap's stepper.
const IDLE_STEP: u32 = 10;

/// The auto-hide seconds one click up or down: by 1 up to 10, then by 5, so a minute is a
/// few clicks away. A value saved off those steps lands on the next one.
fn step_autohide(secs: u32, up: bool) -> u32 {
    let next = match (up, secs < 10) {
        (true, true) => secs + 1,
        (true, false) => (secs / 5 + 1) * 5,
        (false, _) if secs <= 10 => secs.saturating_sub(1),
        (false, _) => (secs - 1) / 5 * 5,
    };
    next.clamp(1, MAX_AUTOHIDE)
}

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
    Tab(usize),
    /// a message set's button, from `phrases::SETS`, which fills the states' boxes
    Set(usize),
    /// a state's dot and name, which select it for editing and preview
    State(usize),
    Swatch(usize),
    Custom,
    Show,
    AutoHide,
    Less,
    More,
    /// the idle nap's stepper
    IdleLess,
    IdleMore,
    Reset,
    Cancel,
    Save,
}

/// (left, top, right, bottom) in the editor's pixels.
pub type Area = (f32, f32, f32, f32);

/// The editor's tabs, in the header: the states' words and colors, then everything else.
const TABS: [&str; 2] = ["Status messages", "Other settings"];

/// Where everything in the editor sits.
pub struct EditorLayout {
    pub w: i32,
    pub h: i32,
    pub header: f32,
    close: Area,
    tabs: [Area; 2],
    /// the column's edges
    col: (f32, f32),
    /// the row of message sets' buttons on the first tab, the buttons, and the line under
    /// them that says what they do
    set_row: Area,
    sets: [Area; SETS.len()],
    set_hint: Area,
    /// the tops of the section headings: messages and colors on the first tab, bubble, idle
    /// and breaks on the second
    messages: f32,
    colors: f32,
    bubble: f32,
    idle: f32,
    breaks: f32,
    feed: f32,
    pub fields: [Area; STATES.len()],
    /// the boxes past the states': the break's times and message, what the seal says fed
    pub extra_fields: [Area; 3],
    extra_hints: [Area; 3],
    swatches: [Area; 8],
    custom: Area,
    show_row: Area,
    auto_row: Area,
    /// the lines under the two rows that say what they do
    show_hint: Area,
    auto_hint: Area,
    /// the two switches
    show: Area,
    autohide: Area,
    /// the seconds stepper, and its minus and plus ends
    seconds: Area,
    less: Area,
    more: Area,
    /// the idle nap's row, the line under it, and its stepper with minus and plus ends
    idle_row: Area,
    idle_hint: Area,
    idle_seconds: Area,
    idle_less: Area,
    idle_more: Area,
    divider: f32,
    reset: Area,
    cancel: Area,
    save: Area,
}

/// One column, its sections split over two tabs, so the editor stays small enough for a
/// small or zoomed screen.
pub fn editor_layout(ui: f32) -> EditorLayout {
    let s = |v: f32| (v * ui).round();
    let (pad, col) = (s(20.0), s(344.0));
    let w = 2.0 * pad + col;
    let right = w - pad;
    let header = s(58.0);
    let close = (right - s(30.0), s(14.0), right, s(44.0));
    let tab_w = s(148.0);
    let tabs = std::array::from_fn(|i| {
        let x = pad + i as f32 * tab_w;
        (x, s(13.0), x + tab_w, s(45.0))
    });

    // the first tab: the message sets' buttons, each state's words, then the text color
    let set_row = (pad, header, right, header + s(42.0));
    let (set_w, set_gap) = (s(84.0), s(8.0));
    let cy = (set_row.1 + set_row.3) / 2.0;
    let sets = std::array::from_fn(|i| {
        let x = right - (SETS.len() - i) as f32 * (set_w + set_gap) + set_gap;
        (x, cy - s(16.0), x + set_w, cy + s(16.0))
    });
    let set_hint = (pad, set_row.3, right, set_row.3 + s(34.0));
    let messages = set_hint.3 + s(14.0);
    let field_h = s(38.0);
    let fields = std::array::from_fn(|i| {
        let y = messages + s(26.0) + i as f32 * (field_h + s(8.0));
        (pad + s(100.0), y, right, y + field_h)
    });
    let colors = fields[STATES.len() - 1].3 + s(22.0);
    let (d, step) = (s(26.0), s(35.0));
    let sy = colors + s(30.0);
    let swatches = std::array::from_fn(|i| {
        let x = pad + s(4.0) + i as f32 * step;
        (x, sy, x + d, sy + d)
    });
    let cx = pad + s(4.0) + 8.0 * step;
    let custom = (cx, sy, cx + d, sy + d);

    // the second tab: the bubble's switches, each with a line under it that says what it
    // does, then the break and the feed
    let bubble = header;
    let row_h = s(38.0);
    let show_row = (pad, bubble + s(22.0), right, bubble + s(22.0) + row_h);
    let under = |r: Area| (pad, r.3 - s(4.0), right, r.3 + s(14.0));
    let show_hint = under(show_row);
    let auto_row = (pad, show_hint.3 + s(4.0), right, show_hint.3 + s(4.0) + row_h);
    let auto_hint = under(auto_row);
    let switch = |r: Area| {
        let cy = (r.1 + r.3) / 2.0;
        (right - s(42.0), cy - s(12.0), right, cy + s(12.0))
    };
    let (show, autohide) = (switch(show_row), switch(auto_row));
    // a stepper ending at `x`, centered on row `r`, and its minus and plus ends
    let stepper = |r: Area, x: f32| {
        let cy = (r.1 + r.3) / 2.0;
        let box_ = (x - s(112.0), cy - s(16.0), x, cy + s(16.0));
        (box_, (box_.0, box_.1, box_.0 + s(36.0), box_.3), (box_.2 - s(36.0), box_.1, box_.2, box_.3))
    };
    let (seconds, less, more) = stepper(auto_row, autohide.0 - s(16.0));
    // then how long the idle seal naps between plays, a stepper with no switch
    let idle = auto_hint.3 + s(18.0);
    let idle_row = (pad, idle + s(22.0), right, idle + s(22.0) + row_h);
    let idle_hint = under(idle_row);
    let (idle_seconds, idle_less, idle_more) = stepper(idle_row, right);
    let breaks = idle_hint.3 + s(18.0);
    // each box with a line under it that says what it is for: the break's two, then, under
    // a heading of its own, the feed's
    let pitch = field_h + s(28.0);
    let feed = breaks + s(26.0) + 2.0 * pitch + s(4.0);
    let extra_fields = std::array::from_fn(|i| {
        let y = if i < 2 { breaks + s(26.0) + i as f32 * pitch } else { feed + s(26.0) };
        (pad + s(100.0), y, right, y + field_h)
    });
    let extra_hints = extra_fields.map(|f: Area| (f.0 + s(4.0), f.3 + s(2.0), f.2, f.3 + s(20.0)));

    // the footer, under the longer tab
    let divider = (sy + d).max(extra_hints[2].3) + s(20.0);
    let (by, bh) = (divider + s(16.0), s(38.0));
    let save = (right - s(96.0), by, right, by + bh);
    let cancel = (save.0 - s(10.0) - s(96.0), by, save.0 - s(10.0), by + bh);
    let reset = (pad - s(12.0), by, pad + s(64.0), by + bh);
    EditorLayout {
        w: w as i32,
        h: (by + bh + pad) as i32,
        header,
        close,
        tabs,
        col: (pad, right),
        set_row,
        sets,
        set_hint,
        messages,
        colors,
        bubble,
        idle,
        breaks,
        feed,
        fields,
        extra_fields,
        extra_hints,
        swatches,
        custom,
        show_row,
        auto_row,
        show_hint,
        auto_hint,
        show,
        autohide,
        seconds,
        less,
        more,
        idle_row,
        idle_hint,
        idle_seconds,
        idle_less,
        idle_more,
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

/// The choices made in the open editor, kept until Save. The words are in the platform's
/// text boxes.
pub struct Draft {
    /// the tab showing, from `TABS`
    pub tab: usize,
    /// each state's text color as picked so far
    pub colors: [Rgba; STATES.len()],
    /// the state whose words were last focused: the one the swatches color, and the
    /// bubble previews
    pub selected: usize,
    pub show: bool,
    pub autohide: bool,
    pub seconds: u32,
    /// seconds the idle seal naps between plays
    pub idle_every: u32,
    pub hover: Option<Hit>,
    /// the text box with the keyboard focus: a state's, or past those (from `EXTRAS`) the
    /// break's and the feed's
    pub focus: Option<usize>,
}

/// The first of the boxes past the states' (`StatusText::extras`).
pub const EXTRAS: usize = STATES.len();

impl Draft {
    pub fn new(status: &StatusText, show_status: bool, state: &str) -> Draft {
        Draft {
            tab: 0,
            colors: status.colors,
            // idle has no box: the first state's is picked instead
            selected: index(state).unwrap_or(0),
            show: show_status,
            autohide: status.autohide > 0,
            seconds: if status.autohide > 0 { status.autohide } else { DEFAULT_AUTOHIDE },
            idle_every: status.idle_every,
            hover: None,
            focus: None,
        }
    }

    /// Flip a switch, step the seconds, or pick a tab, for a click on `hit`. The other hits
    /// are the platform's to act on; after a tab it shows the boxes `shows` names.
    pub fn act(&mut self, hit: Hit) {
        match hit {
            Hit::Tab(i) => self.tab = i,
            Hit::Show => self.show = !self.show,
            Hit::AutoHide => self.autohide = !self.autohide,
            Hit::Less if self.autohide => self.seconds = step_autohide(self.seconds, false),
            Hit::More if self.autohide => self.seconds = step_autohide(self.seconds, true),
            Hit::IdleLess => self.idle_every = self.idle_every.saturating_sub(IDLE_STEP).max(MIN_IDLE_EVERY),
            Hit::IdleMore => self.idle_every = (self.idle_every + IDLE_STEP).min(MAX_IDLE_EVERY),
            _ => {}
        }
        // a box just hidden loses the keyboard without saying so while the app is busy
        if self.focus.is_some_and(|f| !self.shows(f)) {
            self.focus = None;
        }
    }

    /// Whether text box `field` is shown: it is on the tab showing.
    pub fn shows(&self, field: usize) -> bool {
        (field >= EXTRAS) == (self.tab == 1)
    }

    /// The text box to give the keyboard on the tab showing: the selected state's, or the
    /// first of the others.
    pub fn first_box(&self) -> usize {
        if self.tab == 0 { self.selected } else { EXTRAS }
    }

    /// What under (x, y), in the editor's pixels, takes a click, if anything.
    pub fn hit(&self, ui: f32, x: f32, y: f32) -> Option<Hit> {
        let l = editor_layout(ui);
        let mut targets = vec![(Hit::Close, l.close), (Hit::Tab(0), l.tabs[0]), (Hit::Tab(1), l.tabs[1])];
        if self.tab == 0 {
            targets.extend(l.sets.iter().enumerate().map(|(i, r)| (Hit::Set(i), *r)));
            targets.push((Hit::Custom, l.custom));
            targets.extend(l.swatches.iter().enumerate().map(|(i, r)| (Hit::Swatch(i), *r)));
            targets.extend(l.fields.iter().enumerate().map(|(i, f)| (Hit::State(i), (l.col.0, f.1, f.0 - 4.0 * ui, f.3))));
        } else {
            // the seconds box itself takes no click, though it sits in the auto-hide row
            if inside(l.seconds, x, y) && !inside(l.less, x, y) && !inside(l.more, x, y) {
                return None;
            }
            targets.extend([(Hit::Less, l.less), (Hit::More, l.more), (Hit::Show, l.show_row), (Hit::AutoHide, l.auto_row)]);
            targets.extend([(Hit::IdleLess, l.idle_less), (Hit::IdleMore, l.idle_more)]);
        }
        targets.extend([(Hit::Reset, l.reset), (Hit::Cancel, l.cancel), (Hit::Save, l.save)]);
        targets.into_iter().find(|(_, r)| inside(*r, x, y)).map(|(hit, _)| hit)
    }

    /// What the bubble previews for the selected state: line `roll` of `typed` (its box's
    /// words), or its default words while the box is empty.
    pub fn preview(&self, typed: &str, roll: usize) -> String {
        let lines = split(typed);
        match lines.len() {
            0 => STATES[self.selected].2.to_string(),
            n => lines[roll % n].to_string(),
        }
    }

    /// Note the text box with the keyboard focus. True when that selects another state;
    /// the break's boxes leave the selection be.
    pub fn focus_on(&mut self, focus: Option<usize>) -> bool {
        self.focus = focus;
        match focus {
            Some(i) if i < STATES.len() && i != self.selected => {
                self.selected = i;
                true
            }
            _ => false,
        }
    }

    /// Write the choices, `texts` (each state's box's words) and `extras` (the boxes past
    /// those, as `StatusText::extras`) into `status`.
    pub fn commit(&self, status: &mut StatusText, texts: [String; STATES.len()], extras: [String; 3]) {
        let clean = |text: String| text.replace(['\r', '\n'], " ").trim().to_string();
        for (i, text) in texts.into_iter().enumerate() {
            let text = clean(text);
            status.texts[i] = if text.is_empty() { STATES[i].2.to_string() } else { text };
        }
        status.colors = self.colors;
        status.autohide = if self.autohide { self.seconds.clamp(1, MAX_AUTOHIDE) } else { 0 };
        status.idle_every = self.idle_every.clamp(MIN_IDLE_EVERY, MAX_IDLE_EVERY);
        let [times, message, fed] = extras;
        status.breaks = parse_times(&times);
        let or = |text: String, default: &str| if text.is_empty() { default.to_string() } else { text };
        status.break_message = or(clean(message), DEFAULT_BREAK_MESSAGE);
        status.feed_message = or(clean(fed), DEFAULT_FEED_MESSAGE);
    }

    /// Paint the whole editor, apart from the text in its boxes, into `c`.
    pub fn paint(&self, c: &mut Canvas, fonts: &EditorFonts, ui: f32) {
        let l = editor_layout(ui);
        let s = |v: f32| v * ui;
        let (pad, right) = l.col;
        let hover = self.hover;
        c.reset(l.w, l.h);
        c.fill(WHITE);

        // header: the tabs, as a segmented control, and a close button
        let hovered = (0..TABS.len()).find(|&i| hover == Some(Hit::Tab(i)));
        segmented(c, fonts.button, &l.tabs, &TABS, self.tab, hovered, ui);
        if hover == Some(Hit::Close) {
            pill(c, l.close, s(8.0), FIELD);
        }
        let ink = if hover == Some(Hit::Close) { INK } else { MUTED };
        cross(c, l.close, s(5.0), s(1.6), ink, true);

        if self.tab == 0 {
            self.paint_states(c, fonts, &l, ui);
        } else {
            self.paint_other(c, fonts, &l, ui);
        }

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

    /// The first tab: the message sets' buttons, the words for each state, each beside its
    /// state's dot, and the selected state's text color.
    fn paint_states(&self, c: &mut Canvas, fonts: &EditorFonts, l: &EditorLayout, ui: f32) {
        let s = |v: f32| v * ui;
        let x = l.col;
        let hover = self.hover;
        // a set's button fills every state's box with its lines; nothing is kept until Save
        c.label(fonts.heading, "STARTER MESSAGES", rect(l.set_row), FAINT, Align::Left);
        for (i, r) in l.sets.iter().enumerate() {
            pill(c, *r, s(10.0), if hover == Some(Hit::Set(i)) { FIELD_HOVER } else { FIELD });
            c.label(fonts.button, SETS[i], rect(*r), INK, Align::Center);
        }
        let h = l.set_hint;
        let mid = (h.1 + h.3) / 2.0;
        c.label(fonts.hint, "Loads a starter set of messages, in English or Thai,", rect((h.0, h.1, h.2, mid)), MUTED, Align::Left);
        c.label(fonts.hint, "into every box below. Edit them, then Save.", rect((h.0, mid, h.2, h.3)), MUTED, Align::Left);

        heading(c, fonts, ui, x, l.messages, "MESSAGES", "| between lines, \\n for a new row");
        for (i, f) in l.fields.iter().enumerate() {
            let cy = (f.1 + f.3) / 2.0;
            c.fill_sdf((x.0, cy - s(5.0), x.0 + s(10.0), cy + s(5.0)), accent(STATES[i].0), 1.0, circle(x.0 + s(4.0), cy, s(4.0)));
            let ink = if self.selected == i || hover == Some(Hit::State(i)) { INK } else { MUTED };
            c.label(fonts.label, STATES[i].1, rect((x.0 + s(16.0), f.1, f.0, f.3)), ink, Align::Left);
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
        heading(c, fonts, ui, x, l.colors, &format!("TEXT COLOR  ·  {}", STATES[self.selected].1.to_uppercase()), "");
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
    }

    /// The second tab: the bubble, shown or not and hidden again a few seconds after each
    /// change, how long the idle seal naps between plays, then the break (at these times of day the seal rings its clock and says
    /// this) and what the seal says as it eats.
    fn paint_other(&self, c: &mut Canvas, fonts: &EditorFonts, l: &EditorLayout, ui: f32) {
        let s = |v: f32| v * ui;
        let x = l.col;
        let hover = self.hover;
        heading(c, fonts, ui, x, l.bubble, "STATUS BUBBLE", "The words over the seal");
        c.label(fonts.label, "Show status", rect(l.show_row), INK, Align::Left);
        c.label(fonts.hint, "Says what the AI is doing, in a bubble over the seal", rect(l.show_hint), MUTED, Align::Left);
        let auto = if self.autohide {
            format!("Hides it after {} s; it pops up again on each change", self.seconds)
        } else {
            "Off: the bubble stays up while Show status is on".to_string()
        };
        c.label(fonts.hint, &auto, rect(l.auto_hint), MUTED, Align::Left);
        toggle(c, l.show, self.show);
        c.label(fonts.label, "Auto hide", rect(l.auto_row), INK, Align::Left);
        toggle(c, l.autohide, self.autohide);
        let ends = [(Hit::Less, l.less, self.autohide && self.seconds > 1), (Hit::More, l.more, self.autohide && self.seconds < MAX_AUTOHIDE)];
        stepper(c, fonts, ui, l.seconds, ends, hover, &format!("{} s", self.seconds), self.autohide);

        heading(c, fonts, ui, x, l.idle, "IDLE", "The seal naps, then plays");
        c.label(fonts.label, "Play every", rect(l.idle_row), INK, Align::Left);
        let naps = format!("Naps {} s, then plays a pose for 10 s or more", self.idle_every);
        c.label(fonts.hint, &naps, rect(l.idle_hint), MUTED, Align::Left);
        let ends = [(Hit::IdleLess, l.idle_less, self.idle_every > MIN_IDLE_EVERY), (Hit::IdleMore, l.idle_more, self.idle_every < MAX_IDLE_EVERY)];
        stepper(c, fonts, ui, l.idle_seconds, ends, hover, &format!("{} s", self.idle_every), true);

        heading(c, fonts, ui, x, l.breaks, "BREAK", "The seal rings its alarm clock");
        heading(c, fonts, ui, x, l.feed, "FEED", "Feed in the menu gives a salmon");
        let boxes = [
            ("Break at", "Times of day, like 12:00, 15:30"),
            ("Break text", "Shown for a minute at each break time"),
            ("Fed text", "Shown while the seal eats its salmon"),
        ];
        for (k, (f, (name, hint))) in l.extra_fields.iter().zip(boxes).enumerate() {
            c.label(fonts.label, name, rect((x.0, f.1, f.0, f.3)), INK, Align::Left);
            c.label(fonts.hint, hint, rect(l.extra_hints[k]), MUTED, Align::Left);
            if self.focus == Some(EXTRAS + k) {
                pill(c, grow(*f, s(3.0)), s(13.0), FOCUS_RING);
            }
            pill(c, *f, s(10.0), if self.focus == Some(EXTRAS + k) { BLUE } else { LINE });
            pill(c, grow(*f, -s(1.0)), s(9.0), FIELD);
        }
    }
}

/// A row of choices in a pill, the `selected` one raised: the tabs.
fn segmented(c: &mut Canvas, font: Font, areas: &[Area], labels: &[&str], selected: usize, hovered: Option<usize>, ui: f32) {
    let s = |v: f32| v * ui;
    let track = (areas[0].0, areas[0].1, areas[areas.len() - 1].2, areas[0].3);
    pill(c, track, s(10.0), FIELD);
    for (i, r) in areas.iter().enumerate() {
        let ink = if selected == i {
            let r = grow(*r, -s(3.0));
            pill(c, r, s(8.0), LINE);
            pill(c, grow(r, -s(1.0)), s(7.0), WHITE);
            INK
        } else if hovered == Some(i) {
            INK
        } else {
            MUTED
        };
        c.label(font, labels[i], rect(*r), ink, Align::Center);
    }
}

/// A seconds stepper in `box_`: its minus and plus `ends`, each with its hit, area and
/// whether a click there does anything, and `value` between them, faint unless `on`.
#[allow(clippy::too_many_arguments)]
fn stepper(c: &mut Canvas, fonts: &EditorFonts, ui: f32, box_: Area, ends: [(Hit, Area, bool); 2], hover: Option<Hit>, value: &str, on: bool) {
    let s = |v: f32| v * ui;
    pill(c, box_, (box_.3 - box_.1) / 2.0, FIELD);
    for (i, (hit, r, usable)) in ends.into_iter().enumerate() {
        if usable && hover == Some(hit) {
            pill(c, grow(r, -s(3.0)), (r.3 - r.1) / 2.0 - s(3.0), LINE);
        }
        let ink = if usable { INK } else { TRACK_OFF };
        let (cx, cy, k) = ((r.0 + r.2) / 2.0, (r.1 + r.3) / 2.0, s(4.5));
        c.fill_sdf(r, ink, 1.0, segment((cx - k, cy), (cx + k, cy), s(1.6)));
        // the second end is the plus
        if i == 1 {
            c.fill_sdf(r, ink, 1.0, segment((cx, cy - k), (cx, cy + k), s(1.6)));
        }
    }
    let ink = if on { INK } else { FAINT };
    c.label(fonts.button, value, rect((ends[0].1 .2, box_.1, ends[1].1 .0, box_.3)), ink, Align::Center);
}

/// A section's heading, and a hint at the far end of the column.
fn heading(c: &mut Canvas, fonts: &EditorFonts, ui: f32, (x0, x1): (f32, f32), y: f32, text: &str, hint: &str) {
    let r = rect((x0, y, x1, y + 18.0 * ui));
    c.label(fonts.heading, text, r, FAINT, Align::Left);
    c.label(fonts.hint, hint, r, FAINT, Align::Right);
}
