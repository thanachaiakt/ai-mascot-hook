//! The words the status bubble says for each state, and their color.
//!
//! Kept in `%LOCALAPPDATA%\seal-pet\status.txt` as `key=value` lines:
//! `loading`, `working`, `asking`, `completed`, `idle` and `color` (`#rrggbb`).

use std::fs;
use std::path::Path;

use crate::gfx::Rgba;

/// The states the bubble has words for, in the order the editor lists them.
pub const STATES: [(&str, &str, &str); 5] = [
    ("loading", "Thinking", "Thinking…"),
    ("working", "Working", "Working…"),
    ("asking", "Asking", "Need your input!"),
    ("completed", "Done", "All done!"),
    ("idle", "Idle", "Napping… zZ"),
];

pub const DEFAULT_COLOR: Rgba = Rgba(24, 52, 110, 255);

/// The swatches the editor offers.
pub const SWATCHES: [Rgba; 8] = [
    Rgba(24, 52, 110, 255),
    Rgba(47, 128, 237, 255),
    Rgba(15, 157, 138, 255),
    Rgba(46, 158, 68, 255),
    Rgba(224, 123, 0, 255),
    Rgba(214, 51, 108, 255),
    Rgba(112, 72, 232, 255),
    Rgba(51, 51, 51, 255),
];

pub struct StatusText {
    pub texts: [String; 5],
    pub color: Rgba,
}

impl StatusText {
    pub fn defaults() -> StatusText {
        StatusText { texts: STATES.map(|(_, _, text)| text.to_string()), color: DEFAULT_COLOR }
    }

    pub fn load(path: &Path) -> StatusText {
        let mut status = StatusText::defaults();
        if let Ok(file) = fs::read_to_string(path) {
            for line in file.lines() {
                let Some((key, value)) = line.split_once('=') else { continue };
                let value = value.trim();
                if key == "color" {
                    if let Some(c) = parse_hex(value) {
                        status.color = c;
                    }
                } else if let Some(i) = STATES.iter().position(|(k, _, _)| *k == key) {
                    if !value.is_empty() {
                        status.texts[i] = value.to_string();
                    }
                }
            }
        }
        status
    }

    pub fn save(&self, path: &Path) {
        let mut out = String::new();
        for (i, (key, _, _)) in STATES.iter().enumerate() {
            out.push_str(&format!("{key}={}\n", self.texts[i]));
        }
        out.push_str(&format!("color=#{:02x}{:02x}{:02x}\n", self.color.0, self.color.1, self.color.2));
        let _ = fs::write(path, out);
    }

    /// The words for `state`; anything unknown naps.
    pub fn text(&self, state: &str) -> &str {
        let i = STATES.iter().position(|(k, _, _)| *k == state).unwrap_or(4);
        &self.texts[i]
    }
}

/// The dot beside the words: one color per state, so it reads at a glance.
pub fn accent(state: &str) -> Rgba {
    match state {
        "loading" => Rgba(74, 152, 216, 255),
        "working" => Rgba(240, 162, 2, 255),
        "asking" => Rgba(226, 85, 122, 255),
        "completed" => Rgba(46, 158, 68, 255),
        _ => Rgba(154, 165, 177, 255),
    }
}

fn parse_hex(s: &str) -> Option<Rgba> {
    let s = s.strip_prefix('#')?;
    if s.len() != 6 {
        return None;
    }
    let v = u32::from_str_radix(s, 16).ok()?;
    Some(Rgba((v >> 16) as u8, (v >> 8) as u8, v as u8, 255))
}
