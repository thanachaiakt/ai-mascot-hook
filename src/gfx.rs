//! Drawing for layered windows: a premultiplied BGRA surface, anti-aliased shapes,
//! images, GDI text with its alpha repaired, and frame strips scaled on the fly.

use std::ptr::null_mut;

use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

/// A straight (not premultiplied) RGBA color.
#[derive(Clone, Copy)]
pub struct Rgba(pub u8, pub u8, pub u8, pub u8);

/// A 32-bit top-down DIB selected into its own memory DC.
pub struct Canvas {
    pub dc: HDC,
    bitmap: HBITMAP,
    bits: *mut u8,
    pub w: i32,
    pub h: i32,
}

impl Canvas {
    pub fn new() -> Canvas {
        let dc = unsafe {
            let screen = GetDC(null_mut());
            let dc = CreateCompatibleDC(screen);
            ReleaseDC(null_mut(), screen);
            dc
        };
        Canvas { dc, bitmap: null_mut(), bits: null_mut(), w: 0, h: 0 }
    }

    /// Size the surface to w x h and clear it.
    pub fn reset(&mut self, w: i32, h: i32) {
        let (w, h) = (w.max(1), h.max(1));
        if w != self.w || h != self.h || self.bitmap.is_null() {
            unsafe {
                if !self.bitmap.is_null() {
                    DeleteObject(self.bitmap);
                }
                let mut info: BITMAPINFO = std::mem::zeroed();
                info.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
                info.bmiHeader.biWidth = w;
                info.bmiHeader.biHeight = -h;
                info.bmiHeader.biPlanes = 1;
                info.bmiHeader.biBitCount = 32;
                info.bmiHeader.biCompression = BI_RGB;
                let mut bits: *mut core::ffi::c_void = null_mut();
                self.bitmap = CreateDIBSection(self.dc, &info, DIB_RGB_COLORS, &mut bits, null_mut(), 0);
                self.bits = bits as *mut u8;
                SelectObject(self.dc, self.bitmap);
            }
            self.w = w;
            self.h = h;
        }
        self.px().fill(0);
    }

    /// Fill the whole surface with an opaque color.
    pub fn fill(&mut self, c: Rgba) {
        for p in self.px().chunks_exact_mut(4) {
            p.copy_from_slice(&[c.2, c.1, c.0, 255]);
        }
    }

    /// Copy the surface onto `hdc` at (x, y), ignoring alpha.
    pub fn paint_to(&self, hdc: HDC, x: i32, y: i32) {
        unsafe { BitBlt(hdc, x, y, self.w, self.h, self.dc, 0, 0, SRCCOPY) };
    }

    pub fn px(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.bits, (self.w * self.h * 4) as usize) }
    }

    /// Put the surface on screen as `hwnd`'s content, at (x, y), faded by `alpha`.
    pub fn present(&self, hwnd: HWND, x: i32, y: i32, alpha: u8) {
        unsafe {
            let at = POINT { x, y };
            let size = SIZE { cx: self.w, cy: self.h };
            let origin = POINT { x: 0, y: 0 };
            let blend = BLENDFUNCTION { BlendOp: AC_SRC_OVER as u8, BlendFlags: 0, SourceConstantAlpha: alpha, AlphaFormat: AC_SRC_ALPHA as u8 };
            let screen = GetDC(null_mut());
            UpdateLayeredWindow(hwnd, screen, &at, &size, self.dc, &origin, 0, &blend, ULW_ALPHA);
            ReleaseDC(null_mut(), screen);
        }
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
        let (w, h) = (self.w, self.h);
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
                let a = s[3] as u32;
                if a == 0 {
                    continue;
                }
                let i = ((ty * w + tx) * 4) as usize;
                for c in 0..4 {
                    px[i + c] = (s[c] as u32 + px[i + c] as u32 * (255 - a) / 255) as u8;
                }
            }
        }
    }

    /// Size of `text` in `font`, in pixels.
    pub fn measure(&self, font: HFONT, text: &str) -> (i32, i32) {
        let wide = wide(text);
        let mut rect = RECT { left: 0, top: 0, right: 0, bottom: 0 };
        unsafe {
            SelectObject(self.dc, font);
            DrawTextW(self.dc, wide.as_ptr(), -1, &mut rect, DT_CALCRECT | DT_SINGLELINE | DT_NOPREFIX);
        }
        (rect.right - rect.left, rect.bottom - rect.top)
    }

    /// Draw `text` centered in `rect`, on an area already painted opaque.
    ///
    /// GDI writes no alpha, so pixels it touched are made opaque again afterwards.
    pub fn text(&mut self, font: HFONT, text: &str, rect: RECT, color: Rgba) {
        let saved: Vec<u8> = self.px().to_vec();
        let wide = wide(text);
        let mut r = rect;
        unsafe {
            SelectObject(self.dc, font);
            SetBkMode(self.dc, TRANSPARENT as _);
            SetTextColor(self.dc, color.0 as u32 | (color.1 as u32) << 8 | (color.2 as u32) << 16);
            DrawTextW(self.dc, wide.as_ptr(), -1, &mut r, DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
            GdiFlush();
        }
        let px = self.px();
        for i in (0..px.len()).step_by(4) {
            if saved[i + 3] == 255 {
                px[i + 3] = 255;
            } else {
                px[i..i + 4].copy_from_slice(&saved[i..i + 4]);
            }
        }
    }
}

impl Drop for Canvas {
    fn drop(&mut self) {
        unsafe {
            if !self.bitmap.is_null() {
                DeleteObject(self.bitmap);
            }
            DeleteDC(self.dc);
        }
    }
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

pub fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

// ---- signed distance functions ------------------------------------------------

pub fn circle(cx: f32, cy: f32, r: f32) -> impl Fn(f32, f32) -> f32 {
    move |x, y| ((x - cx).powi(2) + (y - cy).powi(2)).sqrt() - r
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

/// Smooth minimum of two distances: shapes melt into each other over `k` pixels.
pub fn smin(a: f32, b: f32, k: f32) -> f32 {
    let h = (k - (a - b).abs()).max(0.0) / k;
    a.min(b) - h * h * k * 0.25
}

pub fn triangle(a: (f32, f32), b: (f32, f32), c: (f32, f32)) -> impl Fn(f32, f32) -> f32 {
    move |x, y| {
        let edge = |p: (f32, f32), q: (f32, f32)| {
            let (ex, ey) = (q.0 - p.0, q.1 - p.1);
            let (wx, wy) = (x - p.0, y - p.1);
            let t = ((wx * ex + wy * ey) / (ex * ex + ey * ey)).clamp(0.0, 1.0);
            let (dx, dy) = (wx - ex * t, wy - ey * t);
            (dx * dx + dy * dy, ex * wy - ey * wx)
        };
        let (d0, s0) = edge(a, b);
        let (d1, s1) = edge(b, c);
        let (d2, s2) = edge(c, a);
        let d = d0.min(d1).min(d2).sqrt();
        let inside = (s0 >= 0.0 && s1 >= 0.0 && s2 >= 0.0) || (s0 <= 0.0 && s1 <= 0.0 && s2 <= 0.0);
        if inside { -d } else { d }
    }
}

// ---- images -------------------------------------------------------------------

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

/// An animation kept as 8-bit palette indices: 1 byte a pixel, scaled when drawn.
pub struct Strip {
    idx: Vec<u8>,
    stride: usize,
    /// premultiplied BGRA per palette entry
    pal: [[u8; 4]; 256],
    pub frames: usize,
    pub fw: usize,
    pub fh: usize,
}

impl Strip {
    pub fn decode(bytes: &[u8], frames: usize) -> Strip {
        let mut decoder = png::Decoder::new(bytes);
        decoder.set_transformations(png::Transformations::IDENTITY);
        let mut reader = decoder.read_info().expect("embedded strip");
        let (palette, trns) = {
            let info = reader.info();
            (info.palette.as_ref().map(|p| p.to_vec()).unwrap_or_default(), info.trns.as_ref().map(|t| t.to_vec()).unwrap_or_default())
        };
        let mut buf = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).expect("embedded strip decodes");
        let (w, h) = (info.width as usize, info.height as usize);
        let bits = info.bit_depth as usize;
        let mut idx = vec![0u8; w * h];
        for y in 0..h {
            let row = &buf[y * info.line_size..];
            for x in 0..w {
                idx[y * w + x] = if bits == 8 {
                    row[x]
                } else {
                    let per = 8 / bits;
                    (row[x / per] >> (8 - bits * (x % per + 1))) & ((1 << bits) - 1) as u8
                };
            }
        }
        let mut pal = [[0u8; 4]; 256];
        for (i, rgb) in palette.chunks(3).enumerate().take(256) {
            let a = *trns.get(i).unwrap_or(&255) as u32;
            pal[i] = [(rgb[2] as u32 * a / 255) as u8, (rgb[1] as u32 * a / 255) as u8, (rgb[0] as u32 * a / 255) as u8, a as u8];
        }
        Strip { idx, stride: w, pal, frames, fw: w / frames, fh: h }
    }

    /// Draw frame `f` scaled to fill the canvas.
    ///
    /// Source positions are worked out once per row and column, so the per-pixel work is
    /// table lookups and integer sums: this runs every frame, at 20 frames a second.
    pub fn draw(&self, f: usize, cv: &mut Canvas) {
        let (ow, oh) = (cv.w as usize, cv.h as usize);
        let rx = self.fw as f32 / ow as f32;
        let ry = self.fh as f32 / oh as f32;
        let base = f * self.fw;
        let px = cv.px();
        if rx > 1.25 {
            // shrinking: average an n x n grid of source pixels under each output pixel
            let n = (rx.ceil() as usize).clamp(2, 8);
            let pick = |o: usize, i: usize, r: f32, limit: usize| (((o as f32 + (i as f32 + 0.5) / n as f32) * r) as usize).min(limit - 1);
            let cols: Vec<usize> = (0..ow * n).map(|k| base + pick(k / n, k % n, rx, self.fw)).collect();
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
                    let o = (y * ow + x) * 4;
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
            let cols: Vec<(usize, usize, u32)> = (0..ow).map(|x| at(x, rx, self.fw)).map(|(a, b, w)| (base + a, base + b, w)).collect();
            for y in 0..oh {
                let (y0, y1, wy) = at(y, ry, self.fh);
                let (r0, r1) = (y0 * self.stride, y1 * self.stride);
                for (x, &(x0, x1, wx)) in cols.iter().enumerate() {
                    let c00 = &self.pal[self.idx[r0 + x0] as usize];
                    let c10 = &self.pal[self.idx[r0 + x1] as usize];
                    let c01 = &self.pal[self.idx[r1 + x0] as usize];
                    let c11 = &self.pal[self.idx[r1 + x1] as usize];
                    let o = (y * ow + x) * 4;
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
