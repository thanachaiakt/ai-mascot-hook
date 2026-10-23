//! What the seal is: its size limits, timings, and the animation clip for each state.

use crate::gfx::Strip;

/// The frames' own size: every size the seal takes keeps this aspect.
pub const FRAME_W: i32 = 260;
pub const FRAME_H: i32 = 260;
/// The seal fits a 32 x 32 box at its smallest and a 512 x 512 box at its largest.
pub const MIN_H: i32 = 32;
pub const MAX_H: i32 = 512;

pub const POLL_MS: u32 = 400;
/// How often to look for state files redirected into packaged apps' folders.
#[cfg(windows)]
pub const RESCAN: std::time::Duration = std::time::Duration::from_secs(5);
/// How long a press must last to become a drag, and how far it may move before it does.
pub const HOLD_MS: u32 = 220;
pub const DRAG_SLOP: i32 = 4;

/// One animation: its frames, frame count and milliseconds per frame.
pub struct Clip {
    /// the frame strip in `assets/`, cut by `build.rs` into one PNG per frame
    pub png: &'static [u8],
    pub frames: usize,
    pub interval: u32,
    /// enlargement about a point of the frame, so the seal stands as big, and where, it
    /// does in the other clips
    pub zoom: f32,
    pub pivot: (f32, f32),
}

impl Clip {
    pub fn strip(&self) -> Strip {
        Strip::decode(self.png, self.frames).zoomed(self.zoom, self.pivot)
    }
}

pub const LOADING: Clip = Clip { png: include_bytes!(concat!(env!("OUT_DIR"), "/loading.frames")), frames: 74, interval: 83, zoom: 1.0, pivot: (0.0, 0.0) };
// the swimming seal is drawn floating mid-frame and smaller: bring it to the others' size,
// its bottom down to where they stand (the window grows past the seal's box to keep the
// whole swimmer when it reaches an edge)
pub const WORKING: Clip = Clip { png: include_bytes!(concat!(env!("OUT_DIR"), "/working.frames")), frames: 95, interval: 83, zoom: 1.15, pivot: (127.0, 37.0) };
pub const ASKING: Clip = Clip { png: include_bytes!(concat!(env!("OUT_DIR"), "/asking.frames")), frames: 60, interval: 83, zoom: 1.0, pivot: (0.0, 0.0) };
pub const COMPLETED: Clip = Clip { png: include_bytes!(concat!(env!("OUT_DIR"), "/completed.frames")), frames: 86, interval: 83, zoom: 1.0, pivot: (0.0, 0.0) };

pub fn clip_for(state: &str) -> &'static Clip {
    match state {
        "loading" => &LOADING,
        "working" => &WORKING,
        "asking" => &ASKING,
        // completed, idle, or nothing written yet: the seal naps
        _ => &COMPLETED,
    }
}


/// Seconds per ripple from the dot: the busy states pulse, asking the quickest.
pub fn ripple_period(state: &str) -> f32 {
    match state {
        "asking" => 1.2,
        "loading" | "working" => 1.8,
        _ => 0.0,
    }
}
