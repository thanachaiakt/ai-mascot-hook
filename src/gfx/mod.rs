//! Drawing: a premultiplied BGRA surface, anti-aliased shapes, images and frame strips
//! scaled on the fly. Putting a surface on screen, and text, are each platform's own
//! (`win`, `mac`).

#[cfg(target_os = "macos")]
mod mac;
#[cfg(windows)]
mod win;

#[cfg(target_os = "macos")]
pub use mac::*;
#[cfg(windows)]
pub use win::*;

/// A premultiplied BGRA surface, top row first, kept by the platform's own drawing context
/// (`Surface`) so text can be drawn into it as well.
pub struct Canvas {
    pub w: i32,
    pub h: i32,
    bits: *mut u8,
    sys: Surface,
}

impl Canvas {
    /// Fill the whole surface with an opaque color.
    pub fn fill(&mut self, c: Rgba) {
        for p in self.px().chunks_exact_mut(4) {
            p.copy_from_slice(&[c.2, c.1, c.0, 255]);
        }
    }

    /// Draw `src` over this surface, sampled bilinearly, scaled by `scale` about `pivot` (a
    /// point of `src`), with `src`'s top-left at `at` before scaling. `at` may fall between
    /// pixels: that is what keeps slow movement smooth.
    pub fn draw_canvas(&mut self, src: &Canvas, at: (f32, f32), scale: f32, pivot: (f32, f32)) {
        if scale <= 0.001 {
            return;
        }
        let (sw, sh) = (src.w, src.h);
        let from = unsafe { std::slice::from_raw_parts(src.bits, (sw * sh * 4) as usize) };
        let (ox, oy) = (at.0 + pivot.0, at.1 + pivot.1);
        let x0 = ((ox - pivot.0 * scale).floor() as i32).max(0);
        let x1 = ((ox + (sw as f32 - pivot.0) * scale).ceil() as i32).min(self.w);
        let y0 = ((oy - pivot.1 * scale).floor() as i32).max(0);
        let y1 = ((oy + (sh as f32 - pivot.1) * scale).ceil() as i32).min(self.h);
        let texel = |x: i32, y: i32| -> [f32; 4] {
            if x < 0 || y < 0 || x >= sw || y >= sh {
                return [0.0; 4];
            }
            let i = ((y * sw + x) * 4) as usize;
            [from[i] as f32, from[i + 1] as f32, from[i + 2] as f32, from[i + 3] as f32]
        };
        let w = self.w;
        let px = self.px();
        for y in y0..y1 {
            let sy = pivot.1 + (y as f32 + 0.5 - oy) / scale - 0.5;
            let (ty, fy) = (sy.floor() as i32, sy - sy.floor());
            for x in x0..x1 {
                let sx = pivot.0 + (x as f32 + 0.5 - ox) / scale - 0.5;
                let (tx, fx) = (sx.floor() as i32, sx - sx.floor());
                let (a, b, c, d) = (texel(tx, ty), texel(tx + 1, ty), texel(tx, ty + 1), texel(tx + 1, ty + 1));
                let mut s = [0.0f32; 4];
                for k in 0..4 {
                    s[k] = (a[k] * (1.0 - fx) + b[k] * fx) * (1.0 - fy) + (c[k] * (1.0 - fx) + d[k] * fx) * fy;
                }
                if s[3] < 0.5 {
                    continue;
                }
                let keep = 1.0 - s[3] / 255.0;
                let i = ((y * w + x) * 4) as usize;
                for k in 0..4 {
                    px[i + k] = (s[k] + px[i + k] as f32 * keep).round().min(255.0) as u8;
                }
            }
        }
    }

    pub fn px(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.bits, (self.w * self.h * 4) as usize) }
    }

    /// Paint `color` where `sdf` (signed distance, negative inside) covers, anti-aliased.
    pub fn fill_sdf(&mut self, bbox: (f32, f32, f32, f32), color: Rgba, feather: f32, sdf: impl Fn(f32, f32) -> f32) {
        let (w, h) = (self.w, self.h);
        let x0 = (bbox.0.floor() as i32).max(0);
        let y0 = (bbox.1.floor() as i32).max(0);
        let x1 = (bbox.2.ceil() as i32).min(w);
        let y1 = (bbox.3.ceil() as i32).min(h);
        let px = self.px();
        for y in y0..y1 {
            for x in x0..x1 {
                let d = sdf(x as f32 + 0.5, y as f32 + 0.5);
                let cover = (0.5 - d / feather).clamp(0.0, 1.0);
                if cover > 0.0 {
                    let i = ((y * w + x) * 4) as usize;
                    over(&mut px[i..i + 4], color, cover);
                }
            }
        }
    }

    /// Draw a premultiplied image with its top-left at (x, y).
    pub fn blit(&mut self, img: &Image, x: i32, y: i32) {
        self.blit_faded(img, x, y, 255);
    }

    /// Draw a premultiplied image with its top-left at (x, y), faded by `alpha`.
    pub fn blit_faded(&mut self, img: &Image, x: i32, y: i32, alpha: u8) {
        let (w, h) = (self.w, self.h);
        let fade = alpha as u32;
        let px = self.px();
        for sy in 0..img.h {
            let ty = y + sy as i32;
            if ty < 0 || ty >= h {
                continue;
            }
            for sx in 0..img.w {
                let tx = x + sx as i32;
                if tx < 0 || tx >= w {
                    continue;
                }
                let s = &img.px[(sy * img.w + sx) * 4..][..4];
                let a = s[3] as u32 * fade / 255;
                if a == 0 {
                    continue;
                }
                let i = ((ty * w + tx) * 4) as usize;
                for c in 0..4 {
                    px[i + c] = (s[c] as u32 * fade / 255 + px[i + c] as u32 * (255 - a) / 255) as u8;
                }
            }
        }
    }
}

pub const NAVY: Rgba = Rgba(24, 52, 110, 255);
pub const BLUE: Rgba = Rgba(74, 152, 216, 255);
pub const WHITE: Rgba = Rgba(255, 255, 255, 255);

/// A straight (not premultiplied) RGBA color.
#[derive(Clone, Copy)]
pub struct Rgba(pub u8, pub u8, pub u8, pub u8);

/// Where `Canvas::label` puts text across its rect; it is always centered top to bottom.
#[derive(Clone, Copy)]
pub enum Align {
    Left,
    Center,
    Right,
}





/// Source-over of a straight color with `cover` onto one premultiplied BGRA pixel.
fn over(px: &mut [u8], c: Rgba, cover: f32) {
    let a = c.3 as f32 / 255.0 * cover;
    let keep = 1.0 - a;
    px[0] = (c.2 as f32 * a + px[0] as f32 * keep).round() as u8;
    px[1] = (c.1 as f32 * a + px[1] as f32 * keep).round() as u8;
    px[2] = (c.0 as f32 * a + px[2] as f32 * keep).round() as u8;
    px[3] = (255.0 * a + px[3] as f32 * keep).round() as u8;
}


// ---- signed distance functions ------------------------------------------------

pub fn circle(cx: f32, cy: f32, r: f32) -> impl Fn(f32, f32) -> f32 {
    move |x, y| ((x - cx).powi(2) + (y - cy).powi(2)).sqrt() - r
}

/// A line from `a` to `b`, `w` thick, with round ends.
pub fn segment(a: (f32, f32), b: (f32, f32), w: f32) -> impl Fn(f32, f32) -> f32 {
    move |x, y| {
        let (px, py, dx, dy) = (x - a.0, y - a.1, b.0 - a.0, b.1 - a.1);
        let t = ((px * dx + py * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
        ((px - dx * t).powi(2) + (py - dy * t).powi(2)).sqrt() - w / 2.0
    }
}

pub fn round_rect(x0: f32, y0: f32, x1: f32, y1: f32, r: f32) -> impl Fn(f32, f32) -> f32 {
    let (cx, cy) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
    let (hx, hy) = ((x1 - x0) / 2.0 - r, (y1 - y0) / 2.0 - r);
    move |x, y| {
        let qx = (x - cx).abs() - hx;
        let qy = (y - cy).abs() - hy;
        (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt() + qx.max(qy).min(0.0) - r
    }
}

/// `sdf` seen on a grid of `p`-pixel blocks: each block takes the distance at its center,
/// so drawn with a hard edge the shape comes out as pixel art.
pub fn pixelated(sdf: impl Fn(f32, f32) -> f32, p: f32) -> impl Fn(f32, f32) -> f32 {
    move |x, y| sdf((x / p).floor() * p + p / 2.0, (y / p).floor() * p + p / 2.0)
}

/// A premultiplied BGRA image.
pub struct Image {
    pub w: usize,
    pub h: usize,
    pub px: Vec<u8>,
}

impl Image {
    /// Decode an RGBA PNG.
    pub fn decode(bytes: &[u8]) -> Image {
        let mut decoder = png::Decoder::new(bytes);
        decoder.set_transformations(png::Transformations::normalize_to_color8());
        let mut reader = decoder.read_info().expect("embedded PNG");
        let mut buf = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).expect("embedded PNG decodes");
        let n = info.color_type.samples();
        let (w, h) = (info.width as usize, info.height as usize);
        let mut px = vec![0u8; w * h * 4];
        for y in 0..h {
            for x in 0..w {
                let s = &buf[y * info.line_size + x * n..];
                let a = if n == 4 { s[3] as u32 } else { 255 };
                let o = (y * w + x) * 4;
                px[o] = (s[2] as u32 * a / 255) as u8;
                px[o + 1] = (s[1] as u32 * a / 255) as u8;
                px[o + 2] = (s[0] as u32 * a / 255) as u8;
                px[o + 3] = a as u8;
            }
        }
        Image { w, h, px }
    }

    /// The `index`-th of `count` equal tiles laid side by side, scaled to `size` square.
    pub fn tile(&self, index: usize, count: usize, size: usize) -> Image {
        let tw = self.w / count;
        let mut out = Image { w: size, h: size, px: vec![0; size * size * 4] };
        let ratio = tw as f32 / size as f32;
        let n = (ratio.ceil() as usize).clamp(1, 8);
        for y in 0..size {
            for x in 0..size {
                let mut acc = [0u32; 4];
                for j in 0..n {
                    for i in 0..n {
                        let sx = (((x as f32 + (i as f32 + 0.5) / n as f32) * ratio) as usize).min(tw - 1);
                        let sy = (((y as f32 + (j as f32 + 0.5) / n as f32) * ratio) as usize).min(self.h - 1);
                        let s = &self.px[(sy * self.w + index * tw + sx) * 4..][..4];
                        for c in 0..4 {
                            acc[c] += s[c] as u32;
                        }
                    }
                }
                let o = (y * size + x) * 4;
                for c in 0..4 {
                    out.px[o + c] = (acc[c] / (n * n) as u32) as u8;
                }
            }
        }
        out
    }
}

// ---- frame strips ---------------------------------------------------------------

/// An animation kept as 8-bit palette indices. The frames stay compressed, one PNG each
/// in the frames file (see `build.rs`); only the frame on screen is unpacked, 1 byte a pixel, and
/// scaled when drawn. A whole strip unpacked would take megabytes.
pub struct Strip {
    pngs: Vec<&'static [u8]>,
    /// premultiplied BGRA per palette entry
    pal: [[u8; 4]; 256],
    clear: u8,
    /// the frame size in the PNGs, before any zoom
    sw: usize,
    sh: usize,
    zoom: f32,
    pivot: (f32, f32),
    /// the unpacked frame (`fw` x `fh`), and which one it is; `raw` holds it at the PNG's
    /// size on the way when the strip is zoomed. Both are reused from frame to frame.
    idx: Vec<u8>,
    raw: Vec<u8>,
    unpacked: Option<usize>,
    stride: usize,
    pub frames: usize,
    pub fw: usize,
    pub fh: usize,
    /// room added around the original frame (left, top, right, bottom), so art enlarged
    /// past its edges is kept whole: the frame proper is the rest
    pub pad: (usize, usize, usize, usize),
}

impl Strip {
    /// Read the frame list `build.rs` wrote: the frame count and each frame's length
    /// (u32, little endian), then the frames' PNGs.
    pub fn decode(blob: &'static [u8], frames: usize) -> Strip {
        let word = |i: usize| u32::from_le_bytes(blob[i * 4..i * 4 + 4].try_into().unwrap()) as usize;
        let count = word(0);
        debug_assert_eq!(count, frames, "frame count in seal.rs and in the strip differ");
        let mut pngs = Vec::with_capacity(count);
        let mut at = 4 * (count + 1);
        for f in 0..count {
            pngs.push(&blob[at..at + word(f + 1)]);
            at += word(f + 1);
        }
        // every frame carries the strip's palette: read it from the first
        let reader = png::Decoder::new(pngs[0]).read_info().expect("embedded frame");
        let info = reader.info();
        let (sw, sh) = (info.width as usize, info.height as usize);
        let palette = info.palette.as_deref().unwrap_or_default();
        let trns = info.trns.as_deref().unwrap_or_default();
        let mut pal = [[0u8; 4]; 256];
        for (i, rgb) in palette.chunks(3).enumerate().take(256) {
            let a = *trns.get(i).unwrap_or(&255) as u32;
            pal[i] = [(rgb[2] as u32 * a / 255) as u8, (rgb[1] as u32 * a / 255) as u8, (rgb[0] as u32 * a / 255) as u8, a as u8];
        }
        let clear = pal.iter().position(|c| c[3] == 0).unwrap_or(0) as u8;
        Strip { pngs, pal, clear, sw, sh, zoom: 1.0, pivot: (0.0, 0.0), idx: Vec::new(), raw: Vec::new(), unpacked: None, stride: sw, frames: count, fw: sw, fh: sh, pad: (0, 0, 0, 0) }
    }

    /// Let go of the frames, so a clip about to be decoded does not sit in memory beside
    /// the one it replaces. The strip draws nothing until it is replaced.
    pub fn release(&mut self) {
        self.pngs = Vec::new();
        self.idx = Vec::new();
        self.raw = Vec::new();
        self.unpacked = None;
        self.frames = 0;
    }

    /// Frame `f`'s indices at the PNG's size, into `out`.
    fn unpack(&self, f: usize, out: &mut Vec<u8>) {
        let mut decoder = png::Decoder::new(self.pngs[f]);
        decoder.set_transformations(png::Transformations::IDENTITY);
        let mut reader = decoder.read_info().expect("embedded frame");
        out.resize(reader.output_buffer_size(), self.clear);
        reader.next_frame(out).expect("embedded frame decodes");
        out.truncate(self.sw * self.sh);
    }

    /// The strip with every frame enlarged by `zoom` about `pivot` (in frame pixels), for art
    /// whose subject sits smaller in its frames than the other strips' does. The frames
    /// grow wherever the enlarged art would leave them, so nothing is cut off; `pad` says
    /// by how much. The enlarging itself happens as each frame is unpacked.
    pub fn zoomed(mut self, zoom: f32, pivot: (f32, f32)) -> Strip {
        if (zoom - 1.0).abs() < 1e-3 {
            return self;
        }
        let (fw, fh) = (self.sw, self.sh);
        // where the art reaches, in any frame, once enlarged
        let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
        let mut px = Vec::new();
        for f in 0..self.frames {
            self.unpack(f, &mut px);
            for y in 0..fh {
                for x in 0..fw {
                    if self.pal[px[y * fw + x] as usize][3] > 0 {
                        x0 = x0.min(x as f32);
                        x1 = x1.max(x as f32 + 1.0);
                        y0 = y0.min(y as f32);
                        y1 = y1.max(y as f32 + 1.0);
                    }
                }
            }
        }
        let grown = |v: f32, c: f32| c + (v - c) * zoom;
        let left = (-grown(x0, pivot.0)).ceil().max(0.0) as usize;
        let right = (grown(x1, pivot.0) - fw as f32).ceil().max(0.0) as usize;
        let top = (-grown(y0, pivot.1)).ceil().max(0.0) as usize;
        let bottom = (grown(y1, pivot.1) - fh as f32).ceil().max(0.0) as usize;
        self.fw = fw + left + right;
        self.fh = fh + top + bottom;
        self.stride = self.fw;
        self.pad = (left, top, right, bottom);
        self.zoom = zoom;
        self.pivot = pivot;
        self
    }

    /// Make `idx` hold frame `f`, enlarged if the strip is zoomed.
    fn show(&mut self, f: usize) {
        if self.unpacked == Some(f) {
            return;
        }
        if self.pad == (0, 0, 0, 0) {
            let mut idx = std::mem::take(&mut self.idx);
            self.unpack(f, &mut idx);
            self.idx = idx;
        } else {
            let mut raw = std::mem::take(&mut self.raw);
            self.unpack(f, &mut raw);
            let (sw, sh, nw, nh) = (self.sw, self.sh, self.fw, self.fh);
            let (left, top, _, _) = self.pad;
            let (zoom, pivot) = (self.zoom, self.pivot);
            self.idx.clear();
            self.idx.resize(nw * nh, self.clear);
            for y in 0..nh {
                let sy = pivot.1 + (y as f32 - top as f32 + 0.5 - pivot.1) / zoom;
                if sy < 0.0 || sy >= sh as f32 {
                    continue;
                }
                for x in 0..nw {
                    let sx = pivot.0 + (x as f32 - left as f32 + 0.5 - pivot.0) / zoom;
                    if sx < 0.0 || sx >= sw as f32 {
                        continue;
                    }
                    self.idx[y * nw + x] = raw[sy as usize * sw + sx as usize];
                }
            }
            self.raw = raw;
        }
        self.unpacked = Some(f);
    }

    /// Draw frame `f` as large as fits the canvas, keeping the frame's aspect: centered
    /// across, resting on the bottom edge (a square strip in the taller seal box sits where
    /// the other strips' seal does).
    ///
    /// Source positions are worked out once per row and column, so the per-pixel work is
    /// table lookups and integer sums: this runs every frame, at 20 frames a second.
    pub fn draw(&mut self, f: usize, cv: &mut Canvas) {
        if f >= self.frames {
            return;
        }
        self.show(f);
        let (cw, ch) = (cv.w as usize, cv.h as usize);
        let fit = (cw as f32 / self.fw as f32).min(ch as f32 / self.fh as f32);
        let ow = ((self.fw as f32 * fit).round() as usize).clamp(1, cw);
        let oh = ((self.fh as f32 * fit).round() as usize).clamp(1, ch);
        let (dx, dy) = ((cw - ow) / 2, ch - oh);
        let rx = self.fw as f32 / ow as f32;
        let ry = self.fh as f32 / oh as f32;
        let px = cv.px();
        let out = |x: usize, y: usize| ((y + dy) * cw + x + dx) * 4;
        if rx > 1.25 {
            // shrinking: average an n x n grid of source pixels under each output pixel
            let n = (rx.ceil() as usize).clamp(2, 8);
            let pick = |o: usize, i: usize, r: f32, limit: usize| (((o as f32 + (i as f32 + 0.5) / n as f32) * r) as usize).min(limit - 1);
            let cols: Vec<usize> = (0..ow * n).map(|k| pick(k / n, k % n, rx, self.fw)).collect();
            let rows: Vec<usize> = (0..oh * n).map(|k| pick(k / n, k % n, ry, self.fh) * self.stride).collect();
            let nn = (n * n) as u32;
            for y in 0..oh {
                for x in 0..ow {
                    let mut acc = [0u32; 4];
                    for &row in &rows[y * n..y * n + n] {
                        for &col in &cols[x * n..x * n + n] {
                            let c = &self.pal[self.idx[row + col] as usize];
                            acc[0] += c[0] as u32;
                            acc[1] += c[1] as u32;
                            acc[2] += c[2] as u32;
                            acc[3] += c[3] as u32;
                        }
                    }
                    let o = out(x, y);
                    px[o] = (acc[0] / nn) as u8;
                    px[o + 1] = (acc[1] / nn) as u8;
                    px[o + 2] = (acc[2] / nn) as u8;
                    px[o + 3] = (acc[3] / nn) as u8;
                }
            }
        } else {
            // same size or growing: bilinear between the four nearest source pixels, in 1/256ths
            let at = |o: usize, r: f32, limit: usize| {
                let s = ((o as f32 + 0.5) * r - 0.5).clamp(0.0, (limit - 1) as f32);
                let i0 = s.floor() as usize;
                (i0, (i0 + 1).min(limit - 1), (s.fract() * 256.0) as u32)
            };
            let cols: Vec<(usize, usize, u32)> = (0..ow).map(|x| at(x, rx, self.fw)).collect();
            for y in 0..oh {
                let (y0, y1, wy) = at(y, ry, self.fh);
                let (r0, r1) = (y0 * self.stride, y1 * self.stride);
                for (x, &(x0, x1, wx)) in cols.iter().enumerate() {
                    let c00 = &self.pal[self.idx[r0 + x0] as usize];
                    let c10 = &self.pal[self.idx[r0 + x1] as usize];
                    let c01 = &self.pal[self.idx[r1 + x0] as usize];
                    let c11 = &self.pal[self.idx[r1 + x1] as usize];
                    let o = out(x, y);
                    for k in 0..4 {
                        let top = c00[k] as u32 * (256 - wx) + c10[k] as u32 * wx;
                        let bottom = c01[k] as u32 * (256 - wx) + c11[k] as u32 * wx;
                        px[o + k] = ((top * (256 - wy) + bottom * wy) >> 16) as u8;
                    }
                }
            }
        }
    }
}
