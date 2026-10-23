//! The Windows surface: a 32-bit top-down DIB selected into its own memory DC, shown with
//! `UpdateLayeredWindow` or painted with `BitBlt`, and text drawn by GDI.

use std::ptr::null_mut;

use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

use super::{Canvas, Rgba};

pub type Font = HFONT;
pub type Rect = RECT;

pub struct Surface {
    pub dc: HDC,
    bitmap: HBITMAP,
}

impl Canvas {
    pub fn new() -> Canvas {
        let dc = unsafe {
            let screen = GetDC(null_mut());
            let dc = CreateCompatibleDC(screen);
            ReleaseDC(null_mut(), screen);
            dc
        };
        Canvas { sys: Surface { dc, bitmap: null_mut() }, bits: null_mut(), w: 0, h: 0 }
    }

    /// Size the surface to w x h and clear it.
    pub fn reset(&mut self, w: i32, h: i32) {
        let (w, h) = (w.max(1), h.max(1));
        if w != self.w || h != self.h || self.sys.bitmap.is_null() {
            unsafe {
                if !self.sys.bitmap.is_null() {
                    DeleteObject(self.sys.bitmap);
                }
                let mut info: BITMAPINFO = std::mem::zeroed();
                info.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
                info.bmiHeader.biWidth = w;
                info.bmiHeader.biHeight = -h;
                info.bmiHeader.biPlanes = 1;
                info.bmiHeader.biBitCount = 32;
                info.bmiHeader.biCompression = BI_RGB;
                let mut bits: *mut core::ffi::c_void = null_mut();
                self.sys.bitmap = CreateDIBSection(self.sys.dc, &info, DIB_RGB_COLORS, &mut bits, null_mut(), 0);
                self.bits = bits as *mut u8;
                SelectObject(self.sys.dc, self.sys.bitmap);
            }
            self.w = w;
            self.h = h;
        }
        self.px().fill(0);
    }

    /// Copy the surface onto `hdc` at (x, y), ignoring alpha.
    pub fn paint_to(&self, hdc: HDC, x: i32, y: i32) {
        unsafe { BitBlt(hdc, x, y, self.w, self.h, self.sys.dc, 0, 0, SRCCOPY) };
    }

    /// Put the surface on screen as `hwnd`'s content, at (x, y), faded by `alpha`.
    pub fn present(&self, hwnd: HWND, x: i32, y: i32, alpha: u8) {
        unsafe {
            let at = POINT { x, y };
            let size = SIZE { cx: self.w, cy: self.h };
            let origin = POINT { x: 0, y: 0 };
            let blend = BLENDFUNCTION { BlendOp: AC_SRC_OVER as u8, BlendFlags: 0, SourceConstantAlpha: alpha, AlphaFormat: AC_SRC_ALPHA as u8 };
            let screen = GetDC(null_mut());
            UpdateLayeredWindow(hwnd, screen, &at, &size, self.sys.dc, &origin, 0, &blend, ULW_ALPHA);
            ReleaseDC(null_mut(), screen);
        }
    }

    /// Size of `text` in `font`, in pixels.
    pub fn measure(&self, font: HFONT, text: &str) -> (i32, i32) {
        let wide = wide(text);
        let mut rect = RECT { left: 0, top: 0, right: 0, bottom: 0 };
        unsafe {
            SelectObject(self.sys.dc, font);
            DrawTextW(self.sys.dc, wide.as_ptr(), -1, &mut rect, DT_CALCRECT | DT_SINGLELINE | DT_NOPREFIX);
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
            SelectObject(self.sys.dc, font);
            SetBkMode(self.sys.dc, TRANSPARENT as _);
            SetTextColor(self.sys.dc, color.0 as u32 | (color.1 as u32) << 8 | (color.2 as u32) << 16);
            DrawTextW(self.sys.dc, wide.as_ptr(), -1, &mut r, DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
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

    /// Draw `text` into `rect` with DrawText `flags`, leaving alpha as it is: for surfaces
    /// shown with `paint_to`, which ignores alpha.
    pub fn draw_text(&mut self, font: HFONT, text: &str, rect: RECT, color: Rgba, flags: u32) {
        let wide = wide(text);
        let mut r = rect;
        unsafe {
            SelectObject(self.sys.dc, font);
            SetBkMode(self.sys.dc, TRANSPARENT as _);
            SetTextColor(self.sys.dc, color.0 as u32 | (color.1 as u32) << 8 | (color.2 as u32) << 16);
            DrawTextW(self.sys.dc, wide.as_ptr(), -1, &mut r, flags | DT_SINGLELINE | DT_NOPREFIX);
            GdiFlush();
        }
    }
}

impl Drop for Canvas {
    fn drop(&mut self) {
        unsafe {
            if !self.sys.bitmap.is_null() {
                DeleteObject(self.sys.bitmap);
            }
            DeleteDC(self.sys.dc);
        }
    }
}

pub fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}
