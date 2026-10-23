//! The Mac surface: our own pixel buffer, wrapped in a CoreGraphics bitmap context of the
//! same layout (premultiplied BGRA, top row first) so AppKit can draw text into it, and
//! turned into a `CGImage` for a window's layer to show.

use std::ffi::c_void;
use std::ptr::{null_mut, NonNull};

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_app_kit::{
    NSColor, NSFont, NSFontAttributeName, NSFontWeightBold, NSFontWeightRegular, NSFontWeightSemibold, NSForegroundColorAttributeName, NSGraphicsContext, NSStringDrawing,
};
use objc2_core_foundation::{CFRetained, CGPoint, CGRect, CGSize};
use objc2_core_graphics::{CGBitmapContextCreateImage, CGColorSpace, CGContext, CGImage};
use objc2_foundation::{NSDictionary, NSString};

use super::{Align, Canvas, Rgba};

/// A font, kept for the life of the program (there are only a few).
pub type Font = *const NSFont;

/// (left, top, right, bottom) in pixels, as on Windows.
#[derive(Clone, Copy)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

/// The system font, `px` pixels tall, at a CSS-style `weight` (400 regular, 600 semibold,
/// 700 bold), as on Windows. It covers Thai too.
pub fn font(px: f32, weight: i32) -> Font {
    let weight = unsafe {
        match weight {
            ..=500 => NSFontWeightRegular,
            501..=650 => NSFontWeightSemibold,
            _ => NSFontWeightBold,
        }
    };
    Retained::into_raw(NSFont::systemFontOfSize_weight(px as f64, weight)) as Font
}

pub struct Surface {
    buf: Vec<u8>,
    ctx: Option<CFRetained<CGContext>>,
}

#[link(name = "CoreGraphics", kind = "framework")]
extern "C-unwind" {
    fn CGBitmapContextCreate(data: *mut c_void, width: usize, height: usize, bits_per_component: usize, bytes_per_row: usize, space: Option<&CGColorSpace>, bitmap_info: u32) -> Option<NonNull<CGContext>>;
}

/// kCGImageAlphaPremultipliedFirst | kCGBitmapByteOrder32Little: BGRA in memory.
const BGRA_PREMULTIPLIED: u32 = 2 | (2 << 12);

impl Canvas {
    pub fn new() -> Canvas {
        Canvas { w: 0, h: 0, bits: null_mut(), sys: Surface { buf: Vec::new(), ctx: None } }
    }

    /// Size the surface to w x h and clear it.
    pub fn reset(&mut self, w: i32, h: i32) {
        let (w, h) = (w.max(1), h.max(1));
        if w != self.w || h != self.h || self.sys.ctx.is_none() {
            self.sys.buf = vec![0; (w * h * 4) as usize];
            self.bits = self.sys.buf.as_mut_ptr();
            self.w = w;
            self.h = h;
            let space = CGColorSpace::new_device_rgb();
            let ctx = unsafe { CGBitmapContextCreate(self.bits as *mut c_void, w as usize, h as usize, 8, (w * 4) as usize, space.as_deref(), BGRA_PREMULTIPLIED) };
            self.sys.ctx = ctx.map(|c| unsafe { CFRetained::from_raw(c) });
            if let Some(ctx) = &self.sys.ctx {
                // top-down, as the pixels are laid out and as Windows draws
                CGContext::translate_ctm(Some(ctx), 0.0, h as f64);
                CGContext::scale_ctm(Some(ctx), 1.0, -1.0);
            }
        } else {
            self.px().fill(0);
        }
    }

    /// The surface as an image, for a layer to show.
    pub fn image(&self) -> Option<CFRetained<CGImage>> {
        CGBitmapContextCreateImage(self.sys.ctx.as_deref())
    }

    /// Size of `text` in `font`, in pixels.
    pub fn measure(&self, font: Font, text: &str) -> (i32, i32) {
        let size = unsafe { NSString::from_str(text).sizeWithAttributes(Some(&attributes(font, Rgba(0, 0, 0, 255)))) };
        (size.width.ceil() as i32, size.height.ceil() as i32)
    }

    /// Draw `text` centered in `rect`.
    pub fn text(&mut self, font: Font, text: &str, rect: Rect, color: Rgba) {
        self.label(font, text, rect, color, Align::Center);
    }

    /// Draw `text` on one line in `rect`, placed by `align`.
    pub fn label(&mut self, font: Font, text: &str, rect: Rect, color: Rgba, align: Align) {
        let Some(ctx) = &self.sys.ctx else { return };
        let attrs = attributes(font, color);
        let s = NSString::from_str(text);
        let size = unsafe { s.sizeWithAttributes(Some(&attrs)) };
        let x = match align {
            Align::Left => rect.left as f64,
            Align::Center => rect.left as f64 + ((rect.right - rect.left) as f64 - size.width) / 2.0,
            Align::Right => rect.right as f64 - size.width,
        };
        let y = rect.top as f64 + ((rect.bottom - rect.top) as f64 - size.height) / 2.0;
        let gc = NSGraphicsContext::graphicsContextWithCGContext_flipped(ctx, true);
        NSGraphicsContext::saveGraphicsState_class();
        NSGraphicsContext::setCurrentContext(Some(&gc));
        unsafe { s.drawInRect_withAttributes(CGRect::new(CGPoint::new(x, y), CGSize::new(size.width, size.height)), Some(&attrs)) };
        gc.flushGraphics();
        NSGraphicsContext::restoreGraphicsState_class();
    }
}

fn attributes(font: Font, color: Rgba) -> Retained<NSDictionary<objc2_foundation::NSAttributedStringKey, AnyObject>> {
    let font: &NSFont = unsafe { &*font };
    let color = NSColor::colorWithSRGBRed_green_blue_alpha(color.0 as f64 / 255.0, color.1 as f64 / 255.0, color.2 as f64 / 255.0, color.3 as f64 / 255.0);
    let keys = unsafe { [NSFontAttributeName, NSForegroundColorAttributeName] };
    let values: [&AnyObject; 2] = [font.as_ref(), color.as_ref()];
    NSDictionary::from_slices(&keys, &values)
}
