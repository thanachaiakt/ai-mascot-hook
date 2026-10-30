//! The status bubble, on every platform: a thought bubble drawn in the seal's pixel-art style
//! with a trail of two round bubbles toward the seal, and how it moves. It pops out of the
//! seal one bubble at a time, floats while it is up, bounces when the state changes, sends
//! a ripple out of its dot for the busy states, and fades away when hidden.
//!
//! Also the size readout shown while resizing. Each platform shows what this draws: the
//! canvases, at the screen positions it works out.

use std::time::Instant;

use crate::anim::*;
use crate::gfx::*;
use crate::seal::ripple_period;

/// (left, top, right, bottom), in screen pixels.
pub type Bounds = (i32, i32, i32, i32);

// the bubble's pixel-art palette, taken from the seal's
const OUTLINE: Rgba = Rgba(52, 60, 82, 255);
const ICE: Rgba = Rgba(214, 228, 244, 255);

/// What the bubble says.
pub struct Content<'a> {
    pub text: &'a str,
    /// the state's accent dot, if any
    pub dot: Option<Rgba>,
    pub ink: Rgba,
    /// what the bubble is about (a state, or resizing): a new one bounces
    pub key: &'a str,
}

/// What a frame of the bubble needs from its platform, when the platform has `Bubble::frame`
/// draw it whole.
#[cfg_attr(not(windows), allow(dead_code))]
pub enum Frame {
    /// take the bubble off screen; no more frames until it is shown again
    Hidden,
    /// show `Bubble::shown` at (x, y), faded by `alpha`; `moving` asks for frames at ~60 a
    /// second, otherwise ~30 a second do
    Shown { x: i32, y: i32, alpha: u8, moving: bool },
}

/// Where the bubble's animation is at one moment.
pub struct Pose {
    pub alpha: u8,
    /// the box's scale about `Bubble::anchor`, its bounce included
    pub body: f32,
    /// the big and the small round bubble's scales about their centers
    pub tails: [f32; 2],
    /// still popping, bouncing or fading, which wants frames at ~60 a second; settled, only
    /// the float and the ripple move
    pub moving: bool,
}

/// The float: each part sways this many seconds a round, the round bubbles a beat behind
/// the box (the lags, in radians).
pub const FLOAT_PERIOD: f32 = 2.8;
pub const FLOAT_LAGS: [f32; 3] = [0.0, 0.9, 1.8];

/// How far the float sways each way, in pixels.
pub fn sway(ui: f32) -> f32 {
    2.0 * ui
}

/// The ripple's ring at `phase` (0 .. 1) of its round, for a dot of radius `r`: its radius
/// and its opacity (0 .. 1).
pub fn ripple(phase: f32, r: f32) -> (f32, f32) {
    (r * 2.0 + r * 1.6 * ease_out_cubic(phase), (1.0 - phase).powi(2) * 170.0 / 255.0)
}

/// The ripple ring's width, in pixels.
pub fn ripple_width(ui: f32) -> f32 {
    1.4 * ui
}

pub struct Bubble {
    /// the box as drawn, at full size
    pub drawn: Canvas,
    /// the bubble as shown this frame
    pub shown: Canvas,
    /// the tail's two round bubbles, each drawn once to move on its own
    pub tails: [Canvas; 2],
    /// the size readout
    pub tag: Canvas,
    font: Font,
    tag_font: Font,
    /// whether it should be up
    up: bool,
    /// 0 gone .. 1 fully up
    appear: f32,
    /// 0 a bounce just began .. 1 settled
    pop: f32,
    key: String,
    /// where the drawing sits on screen, and the point of the box nearest the seal, which
    /// it grows out of
    pub at: (i32, i32),
    pub anchor: (f32, f32),
    /// the round bubbles' drawings: top-left in the drawing, and the distance to their centers
    pub tail: [(f32, f32, f32); 2],
    /// the dot: center, radius, color, and seconds per ripple (0 for none)
    pub dot: Option<(f32, f32, f32, Rgba, f32)>,
    last: Instant,
    clock: Instant,
}

impl Bubble {
    pub fn new(font: Font, tag_font: Font) -> Bubble {
        let now = Instant::now();
        Bubble {
            drawn: Canvas::new(),
            shown: Canvas::new(),
            tails: [Canvas::new(), Canvas::new()],
            tag: Canvas::new(),
            font,
            tag_font,
            up: false,
            appear: 0.0,
            pop: 1.0,
            key: String::new(),
            at: (0, 0),
            anchor: (0.0, 0.0),
            tail: [(0.0, 0.0, 0.0); 2],
            dot: None,
            last: now,
            clock: now,
        }
    }

    /// Lay the bubble out over (or, where the screen ends, under) the seal's box `seal`
    /// (x, y, width, height), inside the work area `work`, clear of `avoid` (the size
    /// readout), draw it, and set it on its way up. `ui` is the display's scale.
    pub fn show(&mut self, content: &Content, seal: (i32, i32, i32, i32), work: Bounds, avoid: Option<Bounds>, ui: f32) {
        let (sx, sy, mw, mh) = seal;
        // a typed `\n` breaks the line
        let text = content.text.replace("\\n", "\n");
        let lines: Vec<&str> = text.split('\n').collect();
        let widths: Vec<i32> = lines.iter().map(|l| self.drawn.measure(self.font, l).0).collect();
        let line_h = self.drawn.measure(self.font, "Ag").1;
        let tw = widths.iter().copied().max().unwrap_or(0);
        let th = line_h * lines.len() as i32;
        let pad_x = (16.0 * ui) as i32;
        let pad_y = (11.0 * ui) as i32;
        let dot_r = 3.5 * ui;
        let dot_room = if content.dot.is_some() { (dot_r * 2.0 + 10.0 * ui) as i32 } else { 0 };
        let tail = (24.0 * ui) as i32;
        let halo_r = 7.0 * ui;
        let margin = (16.0 * ui) as i32; // room for the float and the sticker edge
        let (bw, bh) = (tw + dot_room + 2 * pad_x, th + 2 * pad_y);
        let (cw, ch) = (bw + 2 * margin, bh + tail + 2 * margin);
        let (wl, wt, wr, wb) = work;
        let mut bx = sx + mw / 2 - cw / 2;
        // stack clear of the size readout when it sits on the same side
        let (top_edge, bottom_edge) = match avoid {
            Some(t) if t.3 <= sy => (t.1, sy + mh),
            Some(t) => (sy, t.3),
            None => (sy, sy + mh),
        };
        let mut by = top_edge - ch + margin + 2;
        let below = by < wt;
        if below {
            by = bottom_edge - margin - 2;
        }
        bx = bx.clamp(wl, (wr - cw).max(wl));
        by = by.clamp(wt, (wb - ch).max(wt));

        let (x0, x1) = (margin as f32, (margin + bw) as f32);
        let (y0, y1) = if below { ((margin + tail) as f32, (margin + tail + bh) as f32) } else { (margin as f32, (margin + bh) as f32) };
        // one line is a pill; more lines keep softly rounded corners
        let r = if lines.len() == 1 { bh as f32 / 2.0 } else { (bh as f32 / 2.0).min(22.0 * ui) };
        let t = tail as f32;
        let tip_x = ((sx + mw / 2 - bx) as f32).clamp(x0 + r, x1 - r);
        // a thought-bubble tail: two little circles trailing off toward the seal
        let toward = if below { -1.0 } else { 1.0 };
        let edge = if below { y0 } else { y1 };
        let point_y = edge + toward * t;
        let (big_r, small_r) = (4.5 * ui, 2.6 * ui);
        let tail = [(tip_x - 3.0 * ui, edge + toward * (big_r + 4.5 * ui), big_r), (tip_x + 2.0 * ui, point_y - toward * small_r, small_r)];

        // the box, drawn like the seal on the seal's own pixel grid
        let px = art_pixel(mh);
        self.drawn.reset(cw, ch);
        sticker(&mut self.drawn, (0.0, 0.0, cw as f32, ch as f32), round_rect(x0, y0, x1, y1, r), px);
        let mut ripple = None;
        if let Some(dot) = content.dot {
            let (dx, dy) = (x0 + pad_x as f32 + dot_r, y0 + pad_y as f32 + line_h as f32 / 2.0);
            let halo = Rgba(dot.0, dot.1, dot.2, 46);
            self.drawn.fill_sdf((dx - halo_r - 2.0, dy - halo_r - 2.0, dx + halo_r + 2.0, dy + halo_r + 2.0), halo, 1.0, circle(dx, dy, halo_r));
            self.drawn.fill_sdf((dx - dot_r - 2.0, dy - dot_r - 2.0, dx + dot_r + 2.0, dy + dot_r + 2.0), dot, 1.0, circle(dx, dy, dot_r));
            ripple = Some((dx, dy, dot_r, dot, ripple_period(content.key)));
        }
        let text_left = x0 as i32 + pad_x + dot_room;
        for (i, (line, w)) in lines.iter().zip(&widths).enumerate() {
            let top = y0 as i32 + pad_y + i as i32 * line_h;
            let rect = Rect { left: text_left, top, right: text_left + w, bottom: top + line_h };
            self.drawn.text(self.font, line, rect, content.ink);
        }
        // each round bubble of the tail on a canvas of its own, to float on its own
        for (i, &(cx, cy, r)) in tail.iter().enumerate() {
            let reach = (r + 4.0 * px).ceil();
            let side = (2.0 * reach) as i32;
            let c = &mut self.tails[i];
            c.reset(side, side);
            sticker(c, (0.0, 0.0, side as f32, side as f32), circle(reach + (cx - cx.floor()), reach + (cy - cy.floor()), r), px);
            self.tail[i] = (cx.floor() - reach, cy.floor() - reach, reach);
        }
        if self.appear > 0.0 && self.key != content.key {
            self.pop = 0.0;
        }
        if (!self.up && self.appear <= 0.0) || self.settled() {
            self.last = Instant::now();
        }
        self.up = true;
        self.key = content.key.to_string();
        self.at = (bx, by);
        self.anchor = (tip_x, edge);
        self.dot = ripple;
    }

    /// Set the bubble on its way out.
    pub fn hide(&mut self) {
        if self.settled() {
            self.last = Instant::now();
        }
        self.up = false;
    }

    /// Up and done moving: a platform may stop asking for frames until it is shown or
    /// hidden again, so the time since the last frame says nothing.
    fn settled(&self) -> bool {
        self.up && self.appear >= 1.0 && self.pop >= 1.0
    }

    /// Step the animation to now. None once the bubble has faded away.
    pub fn pose(&mut self) -> Option<Pose> {
        let now = Instant::now();
        let dt = now.duration_since(self.last).as_secs_f32().min(0.05);
        self.last = now;
        self.appear = if self.up { self.appear + dt / 0.5 } else { self.appear - dt / 0.18 }.clamp(0.0, 1.0);
        self.pop = (self.pop + dt / 0.45).min(1.0);
        if self.appear <= 0.0 {
            return None;
        }
        // on the way in, each part pops with a little overshoot; on the way out, all shrink a touch
        let (small, big, body) = if self.up {
            let t = self.appear;
            (ease_out_back(stage(t, 0.0, 0.3)), ease_out_back(stage(t, 0.12, 0.35)), ease_out_back(stage(t, 0.28, 0.72)))
        } else {
            let s = 0.94 + 0.06 * self.appear;
            (s, s, s)
        };
        let bounce = 1.0 + 0.14 * (self.pop * std::f32::consts::PI).sin() * (1.0 - self.pop);
        let fade = if self.up { ease_out_cubic((self.appear * 4.0).min(1.0)) } else { ease_out_cubic(self.appear) };
        Some(Pose { alpha: (fade * 255.0).round() as u8, body: body * bounce, tails: [big, small], moving: !self.settled() })
    }

    /// Draw the bubble as it is at this moment of its animation into `shown`. It comes out
    /// of the seal one bubble at a time, the small round one first, then the big one, then
    /// the box; once up, the box and the two round bubbles float smoothly, a beat apart.
    #[cfg_attr(not(windows), allow(dead_code))]
    pub fn frame(&mut self, ui: f32) -> Frame {
        let Some(Pose { alpha, body: scale, tails: [big, small], moving }) = self.pose() else { return Frame::Hidden };

        // the float: a slow sway up and down, the round bubbles a beat behind the box
        let clock = self.clock.elapsed().as_secs_f32();
        let wave = |lag: f32| ((clock / FLOAT_PERIOD) * std::f32::consts::TAU - lag).sin() * sway(ui);
        let [float_box, float_big, float_small] = FLOAT_LAGS.map(wave);

        self.shown.reset(self.drawn.w, self.drawn.h);
        self.shown.draw_canvas(&self.drawn, (0.0, float_box), scale, self.anchor);
        for (i, (grow, float)) in [(big, float_big), (small, float_small)].into_iter().enumerate() {
            let (x, y, reach) = self.tail[i];
            self.shown.draw_canvas(&self.tails[i], (x, y + float), grow, (reach, reach));
        }
        if let Some((x, y, r, color, period)) = self.dot {
            if period > 0.0 {
                let (radius, opacity) = ripple((clock / period).fract(), r);
                let (ax, ay) = self.anchor;
                let (cx, cy) = (ax + (x - ax) * scale, ay + (y - ay) * scale + float_box);
                let radius = radius * scale;
                let w = ripple_width(ui);
                let ring = Rgba(color.0, color.1, color.2, (opacity * 255.0) as u8);
                let reach = radius + w + 2.0;
                self.shown.fill_sdf((cx - reach, cy - reach, cx + reach, cy + reach), ring, 1.0, move |x, y| {
                    circle(cx, cy, radius)(x, y).abs() - w / 2.0
                });
            }
        }
        Frame::Shown { x: self.at.0, y: self.at.1, alpha, moving }
    }

    /// Draw the size readout, a small pill just below the seal's box (above it when the
    /// screen ends there), into `tag`. Returns where to show `tag`, and the pill's bounds,
    /// for the bubble to stack clear of.
    pub fn size_tag(&mut self, height: i32, seal: (i32, i32, i32, i32), work: Bounds, ui: f32) -> ((i32, i32), Bounds) {
        let (sx, sy, mw, mh) = seal;
        let text = format!("{height} px");
        let (tw, th) = self.tag.measure(self.tag_font, &text);
        let pad_x = (10.0 * ui) as i32;
        let pad_y = (3.0 * ui) as i32;
        let gap = (6.0 * ui) as i32;
        let margin = (8.0 * ui) as i32; // room for the soft shadow
        let (bw, bh) = (tw + 2 * pad_x, th + 2 * pad_y);
        let (cw, ch) = (bw + 2 * margin, bh + 2 * margin);
        let mut left = sx + mw / 2 - bw / 2;
        let mut top = sy + mh + gap;
        if top + bh > work.3 {
            top = sy - gap - bh;
        }
        left = left.clamp(work.0, (work.2 - bw).max(work.0));

        let (x0, y0, x1, y1) = (margin as f32, margin as f32, (margin + bw) as f32, (margin + bh) as f32);
        let body = round_rect(x0, y0, x1, y1, bh as f32 / 2.0);
        self.tag.reset(cw, ch);
        let all = (0.0, 0.0, cw as f32, ch as f32);
        let lift = 2.0 * ui;
        self.tag.fill_sdf(all, Rgba(16, 32, 70, 60), 6.0 * ui, |x, y| body(x, y - lift) + 1.0 * ui);
        self.tag.fill_sdf(all, BLUE, 1.0, &body);
        let rect = Rect { left: margin, top: margin, right: margin + bw, bottom: margin + bh };
        self.tag.text(self.tag_font, &text, rect, WHITE);
        ((left - margin, top - margin), (left, top, left + bw, top + bh))
    }
}

/// The size of one pixel of the seal's art on screen, for a seal `height` pixels tall: the
/// bubble is drawn on the same grid.
fn art_pixel(height: i32) -> f32 {
    (height as f32 / 75.0).round().max(1.0)
}

/// `shape` drawn as the seal is drawn: pixel art with a dark outline, a white sticker edge
/// outside that, and a band of icy shade along the bottom.
fn sticker(c: &mut Canvas, area: (f32, f32, f32, f32), shape: impl Fn(f32, f32) -> f32, px: f32) {
    let hard = 0.01;
    let shape = pixelated(shape, px);
    c.fill_sdf(area, WHITE, hard, |x, y| shape(x, y) - 2.5 * px);
    c.fill_sdf(area, OUTLINE, hard, |x, y| shape(x, y) - px);
    c.fill_sdf(area, ICE, hard, &shape);
    c.fill_sdf(area, WHITE, hard, |x, y| shape(x, y + 2.0 * px).max(shape(x, y) + px));
}
