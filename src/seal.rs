//! What the seal is: its size limits, timings, and the animation clip for each state.

use std::time::{Duration, SystemTime};

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
// the states clip's completed quadrant, a napping seal
pub const NAPPING: Clip = Clip { png: include_bytes!(concat!(env!("OUT_DIR"), "/completed.frames")), frames: 86, interval: 83, zoom: 1.0, pivot: (0.0, 0.0) };

/// A pose cut from the poses clip by `tools/video_poses.py`, or from a clip of its own by
/// `tools/clip_pose.py`, every 2nd frame at 24 fps.
macro_rules! pose {
    ($name:literal, $frames:literal) => {
        Clip { png: include_bytes!(concat!(env!("OUT_DIR"), "/", $name, ".frames")), frames: $frames, interval: 83, zoom: 1.0, pivot: (0.0, 0.0) }
    };
}

pub const THINKING: Clip = pose!("thinking", 74);
pub const ACTIVE: Clip = pose!("active", 49);
pub const ERROR: Clip = pose!("error", 62);
pub const SLIDE: Clip = pose!("slide", 94);
pub const BREAK_TIME: Clip = pose!("break-time", 26);
pub const TOY_CAR: Clip = pose!("toy-car", 114);
pub const BALL: Clip = pose!("ball", 44);
// its own 20 s clip, from the whole salmon to the seal waving, glowing and content, less
// the first 10 frames, where the seal sits still before it starts to eat
pub const SALMON: Clip = pose!("salmon", 230);

/// The clips a state plays. With more than one, each plays for at least `TURN_MS` and the
/// next takes over as it comes back to its first frame, in order. An act with a `rest`
/// clip plays that instead, and for every `REST_MS` of it one of `clips`, at random, for a
/// turn.
pub struct Act {
    pub clips: &'static [&'static Clip],
    pub rest: Option<&'static Clip>,
}

/// The least time one clip of an act plays before the next takes over.
pub const TURN_MS: u32 = 10_000;
/// How long a resting seal naps before it wakes to play.
pub const REST_MS: u32 = 60_000;

// statics, not consts, so each act has one address to tell it by
static LOADING_ACT: Act = Act { clips: &[&LOADING], rest: None };
// swimming and thinking in turn
static WORKING_ACT: Act = Act { clips: &[&WORKING, &THINKING], rest: None };
static ASKING_ACT: Act = Act { clips: &[&ASKING], rest: None };
static COMPLETED_ACT: Act = Act { clips: &[&ACTIVE], rest: None };
static ERROR_ACT: Act = Act { clips: &[&ERROR], rest: None };
// the seal naps, and wakes now and then to play
static IDLE_ACT: Act = Act { clips: &[&SLIDE, &TOY_CAR, &BALL], rest: Some(&NAPPING) };

pub fn act_for(state: &str) -> &'static Act {
    match state {
        "loading" => &LOADING_ACT,
        "working" => &WORKING_ACT,
        "asking" => &ASKING_ACT,
        "completed" => &COMPLETED_ACT,
        "error" => &ERROR_ACT,
        // idle, or nothing written yet
        _ => &IDLE_ACT,
    }
}

/// How long the seal shows off after a turn ends. No hook says the user has walked away
/// (the VS Code extension sends no idle prompt), so without this it would never nap again.
pub const DONE_FOR: Duration = Duration::from_secs(10);

/// `state`, written at `stamp`, has been shown long enough to settle into idle.
pub fn done_showing(state: &str, stamp: Option<SystemTime>) -> bool {
    state == "completed" && stamp.and_then(|t| t.elapsed().ok()).is_some_and(|age| age >= DONE_FOR)
}

/// Clips played over whatever the state's act is, each for at least its milliseconds,
/// before the act goes on.
pub type CutIn = &'static [(&'static Clip, u32)];
/// Fed a salmon: the seal eats it, the clip played once through.
pub static FEED: CutIn = &[(&SALMON, 19_000)];
/// Break time: the alarm clock rings for a minute, or until the seal is clicked.
pub static BREAK: CutIn = &[(&BREAK_TIME, 60_000)];

/// Which clip of an act is playing, and for how long it has.
pub struct Turns {
    pub act: &'static Act,
    /// the act's clip playing, or none while it rests
    at: Option<usize>,
    played_ms: u32,
    /// a cut-in playing over the act, and which of its clips
    cut_in: Option<(CutIn, usize)>,
}

impl Turns {
    /// An act starts resting, or on its first clip.
    pub fn new(act: &'static Act) -> Turns {
        Turns { act, at: if act.rest.is_some() { None } else { Some(0) }, played_ms: 0, cut_in: None }
    }

    /// Go on with `act`, from its start, but finish any cut-in first.
    pub fn switch(&mut self, act: &'static Act) {
        let (cut_in, played_ms) = (self.cut_in, self.played_ms);
        *self = Turns::new(act);
        if cut_in.is_some() {
            (self.cut_in, self.played_ms) = (cut_in, played_ms);
        }
    }

    pub fn clip(&self) -> &'static Clip {
        match (self.cut_in, self.at, self.act.rest) {
            (Some((seq, i)), _, _) => seq[i].0,
            (None, Some(i), _) => self.act.clips[i],
            (None, None, rest) => rest.unwrap_or(self.act.clips[0]),
        }
    }

    /// Play `seq` over the act, from its first clip; the caller starts it.
    pub fn cut_in(&mut self, seq: CutIn) {
        self.cut_in = Some((seq, 0));
        self.played_ms = 0;
    }

    /// The break is ringing.
    pub fn on_break(&self) -> bool {
        self.cut_in.is_some_and(|(seq, _)| std::ptr::eq(seq, BREAK))
    }

    /// The scene the bubble speaks for, if any: "break" while the clock rings, "feed" while
    /// the seal has its salmon.
    pub fn scene(&self) -> Option<&'static str> {
        match self.cut_in {
            Some((seq, _)) if std::ptr::eq(seq, BREAK) => Some("break"),
            Some((seq, _)) if std::ptr::eq(seq, FEED) => Some("feed"),
            _ => None,
        }
    }

    /// Stop the cut-in playing, if any: true if there was one, and the caller goes back to
    /// the act's clip.
    pub fn end_cut_in(&mut self) -> bool {
        let was = self.cut_in.take().is_some();
        if was {
            self.played_ms = 0;
        }
        was
    }

    /// The clip has come back to its first frame: true when another takes over.
    pub fn looped(&mut self) -> bool {
        let clip = self.clip();
        self.played_ms += clip.frames as u32 * clip.interval;
        if let Some((seq, i)) = self.cut_in {
            if self.played_ms < seq[i].1 {
                return false;
            }
            self.played_ms = 0;
            self.cut_in = (i + 1 < seq.len()).then_some((seq, i + 1));
            return true;
        }
        let n = self.act.clips.len();
        let (due, next) = match (self.at, self.act.rest) {
            // a nap, then a turn at play; a turn at play, then the nap
            (None, Some(_)) => (REST_MS, Some(random() % n)),
            (Some(_), Some(_)) => (TURN_MS, None),
            (Some(i), None) if n > 1 => (TURN_MS, Some((i + 1) % n)),
            _ => return false,
        };
        if self.played_ms < due {
            return false;
        }
        self.played_ms = 0;
        self.at = next;
        true
    }
}

/// A random number: each `RandomState` is keyed apart from the last, from a seed the
/// system picks.
fn random() -> usize {
    use std::hash::BuildHasher;
    std::collections::hash_map::RandomState::new().hash_one(()) as usize
}

/// Minutes since midnight by the computer's clock, for the breaks.
#[cfg(windows)]
pub fn minute_of_day() -> u32 {
    use windows_sys::Win32::System::SystemInformation::GetLocalTime;
    let t = unsafe {
        let mut t = std::mem::zeroed();
        GetLocalTime(&mut t);
        t
    };
    t.wHour as u32 * 60 + t.wMinute as u32
}

/// Minutes since midnight by the computer's clock, for the breaks.
#[cfg(not(windows))]
pub fn minute_of_day() -> u32 {
    // `struct tm` as Darwin lays it out
    #[repr(C)]
    struct Tm {
        sec: i32,
        min: i32,
        hour: i32,
        mday: i32,
        mon: i32,
        year: i32,
        wday: i32,
        yday: i32,
        isdst: i32,
        gmtoff: std::ffi::c_long,
        zone: *const std::ffi::c_char,
    }
    extern "C" {
        fn time(out: *mut i64) -> i64;
        fn localtime_r(clock: *const i64, out: *mut Tm) -> *mut Tm;
    }
    unsafe {
        let now = time(std::ptr::null_mut());
        let mut tm: Tm = std::mem::zeroed();
        localtime_r(&now, &mut tm);
        (tm.hour * 60 + tm.min) as u32
    }
}

/// Seconds per ripple from the dot: the busy states pulse, asking the quickest.
pub fn ripple_period(state: &str) -> f32 {
    match state {
        "asking" | "break" => 1.2,
        "loading" | "working" => 1.8,
        _ => 0.0,
    }
}
