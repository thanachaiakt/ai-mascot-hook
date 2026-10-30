//! The words the status bubble says for each state, and their color.
//!
//! Kept in `%LOCALAPPDATA%\ai-mascot-hook\status.txt` as `key=value` lines:
//! `loading`, `working`, `asking`, `completed`, `error`, a text color for each as
//! `color.<state>` (`#rrggbb`), `autohide` (seconds the bubble stays up after each state
//! change, 0 to keep it up), `idle.every` (seconds the idle seal naps between plays),
//! `break.times` (`12:00, 15:30`) and `break.message` for the
//! break reminders, and `feed.message`, what the seal says as it eats. A state's words may
//! hold several lines split by `|`, one said at random. A plain `color`, from older
//! versions, colors every state. Idle has no words: the bubble stays hidden while the seal
//! naps.

use std::fs;
use std::path::Path;

use crate::gfx::Rgba;
use crate::phrases::split;

/// The states the bubble has words for, in the order the editor lists them.
pub const STATES: [(&str, &str, &str); 5] = [
    ("loading", "Thinking", "Thinking…"),
    ("working", "Working", "Working…"),
    ("asking", "Asking", "Need your input!"),
    ("completed", "Done", "All done!"),
    ("error", "Error", "Something went wrong"),
];

/// The state the seal naps in, which the bubble has no words for.
pub const IDLE: &str = "idle";

pub const DEFAULT_COLOR: Rgba = Rgba(24, 52, 110, 255);

/// What the bubble says at break time, unless set.
pub const DEFAULT_BREAK_MESSAGE: &str = "Time for a break!";
/// What the bubble says while the seal eats, unless set.
pub const DEFAULT_FEED_MESSAGE: &str = "Yum, thank you!";

/// The most characters a state's words may have: room for a set's ten lines.
pub const MAX_CHARS: usize = 400;

/// The longest the bubble may stay up before it hides itself, in seconds.
pub const MAX_AUTOHIDE: u32 = 60;

/// How long the idle seal naps between plays, in seconds, unless set, and the range the
/// editor allows.
pub const DEFAULT_IDLE_EVERY: u32 = 60;
pub const MIN_IDLE_EVERY: u32 = 10;
pub const MAX_IDLE_EVERY: u32 = 120;

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
    pub texts: [String; STATES.len()],
    /// each state's text color, in the order of `STATES`
    pub colors: [Rgba; STATES.len()],
    /// seconds the bubble stays up after each state change; 0 keeps it up
    pub autohide: u32,
    /// seconds the idle seal naps before it wakes to play
    pub idle_every: u32,
    /// the break times, in minutes since midnight, in order
    pub breaks: Vec<u32>,
    pub break_message: String,
    pub feed_message: String,
}

impl StatusText {
    pub fn defaults() -> StatusText {
        StatusText {
            texts: STATES.map(|(_, _, text)| text.to_string()),
            colors: [DEFAULT_COLOR; STATES.len()],
            autohide: 0,
            idle_every: DEFAULT_IDLE_EVERY,
            breaks: Vec::new(),
            break_message: DEFAULT_BREAK_MESSAGE.to_string(),
            feed_message: DEFAULT_FEED_MESSAGE.to_string(),
        }
    }

    pub fn load(path: &Path) -> StatusText {
        let mut status = StatusText::defaults();
        if let Ok(file) = fs::read_to_string(path) {
            // a file saved by hand may start with a BOM (PowerShell's Set-Content does)
            let file = file.trim_start_matches('\u{feff}');
            for line in file.lines() {
                let Some((key, value)) = line.split_once('=') else { continue };
                let value = value.trim();
                if key == "color" {
                    // older files: one color for every state, unless a state names its own
                    if let Some(c) = parse_hex(value) {
                        for (i, (k, _, _)) in STATES.iter().enumerate() {
                            if !file.lines().any(|l| l.trim_start().starts_with(&format!("color.{k}="))) {
                                status.colors[i] = c;
                            }
                        }
                    }
                } else if let Some(state) = key.strip_prefix("color.") {
                    if let (Some(i), Some(c)) = (STATES.iter().position(|(k, _, _)| *k == state), parse_hex(value)) {
                        status.colors[i] = c;
                    }
                } else if key == "break.times" {
                    status.breaks = parse_times(value);
                } else if key == "break.message" {
                    if !value.is_empty() {
                        status.break_message = value.to_string();
                    }
                } else if key == "feed.message" {
                    if !value.is_empty() {
                        status.feed_message = value.to_string();
                    }
                } else if key == "autohide" {
                    if let Ok(secs) = value.parse::<u32>() {
                        status.autohide = secs.min(MAX_AUTOHIDE);
                    }
                } else if key == "idle.every" {
                    if let Ok(secs) = value.parse::<u32>() {
                        status.idle_every = secs.clamp(MIN_IDLE_EVERY, MAX_IDLE_EVERY);
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
        for (i, (key, _, _)) in STATES.iter().enumerate() {
            let c = self.colors[i];
            out.push_str(&format!("color.{key}=#{:02x}{:02x}{:02x}\n", c.0, c.1, c.2));
        }
        out.push_str(&format!("autohide={}\n", self.autohide));
        out.push_str(&format!("idle.every={}\n", self.idle_every));
        out.push_str(&format!("break.times={}\n", format_times(&self.breaks)));
        out.push_str(&format!("break.message={}\n", self.break_message));
        out.push_str(&format!("feed.message={}\n", self.feed_message));
        let _ = fs::write(path, out);
    }

    /// The words in the editor's boxes past the states': the break's times and message,
    /// then what the seal says as it eats.
    pub fn extras(&self) -> [String; 3] {
        [format_times(&self.breaks), self.break_message.clone(), self.feed_message.clone()]
    }

    /// What the bubble says through a scene the seal plays (see `Turns::scene`).
    pub fn scene_text(&self, scene: &str) -> &str {
        if scene == "feed" { &self.feed_message } else { &self.break_message }
    }

    /// The words for `state`: line `roll` of its lines, nothing for idle or anything unknown.
    pub fn text(&self, state: &str, roll: usize) -> &str {
        let lines = self.lines(state);
        lines.get(roll % lines.len().max(1)).copied().unwrap_or("")
    }

    /// How many lines `state` has to pick from; none while it naps.
    pub fn count(&self, state: &str) -> usize {
        self.lines(state).len()
    }

    fn lines(&self, state: &str) -> Vec<&str> {
        index(state).map(|i| split(&self.texts[i])).unwrap_or_default()
    }

    /// The text color for `state`.
    pub fn color(&self, state: &str) -> Rgba {
        index(state).map_or(DEFAULT_COLOR, |i| self.colors[i])
    }
}

/// The times in `text` (`12:00, 15:30`, `9:05 17:00`), as minutes since midnight, in order
/// and once each. Anything that is not a time of day is left out.
pub fn parse_times(text: &str) -> Vec<u32> {
    let mut times: Vec<u32> = text
        .split(|c: char| c == ',' || c == ';' || c.is_whitespace())
        .filter_map(|t| {
            let (h, m) = t.split_once([':', '.'])?;
            let (h, m) = (h.parse::<u32>().ok()?, m.parse::<u32>().ok()?);
            (h < 24 && m < 60).then_some(h * 60 + m)
        })
        .collect();
    times.sort_unstable();
    times.dedup();
    times
}

/// Break times as the editor shows them and `status.txt` keeps them: `12:00, 15:30`.
pub fn format_times(times: &[u32]) -> String {
    times.iter().map(|t| format!("{}:{:02}", t / 60, t % 60)).collect::<Vec<_>>().join(", ")
}

/// Where `state` sits in `STATES`; idle and anything unknown sit nowhere.
pub fn index(state: &str) -> Option<usize> {
    STATES.iter().position(|(k, _, _)| *k == state)
}

/// The state word in the bytes of the state file, whatever wrote it: UTF-8 with or without
/// a BOM, or UTF-16 (PowerShell's `>` and `Out-File`), with surrounding whitespace, quotes
/// (cmd's `echo "working" >`) or a trailing newline, in any case. `None` when there is no
/// word, as when the file is read mid-write.
pub fn parse_state(bytes: &[u8]) -> Option<String> {
    let text = match bytes {
        [0xFF, 0xFE, rest @ ..] => utf16(rest, u16::from_le_bytes),
        [0xFE, 0xFF, rest @ ..] => utf16(rest, u16::from_be_bytes),
        [0xEF, 0xBB, 0xBF, rest @ ..] => String::from_utf8_lossy(rest).into_owned(),
        // UTF-16 without a BOM still shows itself by its zero bytes
        _ if bytes.len() >= 2 && bytes[1] == 0 => utf16(bytes, u16::from_le_bytes),
        _ if bytes.len() >= 2 && bytes[0] == 0 => utf16(bytes, u16::from_be_bytes),
        _ => String::from_utf8_lossy(bytes).into_owned(),
    };
    let word = text.trim_matches(|c: char| c.is_whitespace() || c == '"' || c == '\'' || c == '\0');
    let word = word.split_whitespace().next()?.to_ascii_lowercase();
    Some(word)
}

/// Write `word` as the state file `dir\state`, as plain UTF-8, creating `dir` if needed.
/// The word goes to a temporary file first and is then moved over the state file, so a
/// reader never sees it half written.
pub fn write_state(dir: &Path, word: &str) -> std::io::Result<()> {
    let word = word.trim().to_ascii_lowercase();
    if word != IDLE && index(&word).is_none() {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("unknown state {word:?}")));
    }
    fs::create_dir_all(dir)?;
    let target = dir.join("state");
    let temp = dir.join(format!("state.{}.tmp", std::process::id()));
    fs::write(&temp, &word)?;
    if fs::rename(&temp, &target).is_err() {
        // the file is held open elsewhere: write it in place instead
        let _ = fs::remove_file(&temp);
        fs::write(&target, &word)?;
    }
    Ok(())
}

fn utf16(bytes: &[u8], unit: fn([u8; 2]) -> u16) -> String {
    let units: Vec<u16> = bytes.chunks_exact(2).map(|p| unit([p[0], p[1]])).collect();
    String::from_utf16_lossy(&units)
}

/// The dot beside the words: one color per state, so it reads at a glance.
pub fn accent(state: &str) -> Rgba {
    match state {
        "loading" => Rgba(74, 152, 216, 255),
        "working" => Rgba(240, 162, 2, 255),
        "asking" => Rgba(226, 85, 122, 255),
        "completed" => Rgba(46, 158, 68, 255),
        "error" => Rgba(220, 53, 53, 255),
        "break" => Rgba(224, 123, 0, 255),
        "feed" => Rgba(236, 112, 72, 255),
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

#[cfg(test)]
mod tests {
    use super::parse_state;

    fn utf16le(s: &str, bom: bool) -> Vec<u8> {
        let mut out = if bom { vec![0xFF, 0xFE] } else { vec![] };
        out.extend(s.encode_utf16().flat_map(|u| u.to_le_bytes()));
        out
    }

    #[test]
    fn plain_words() {
        assert_eq!(parse_state(b"working").as_deref(), Some("working"));
        assert_eq!(parse_state(b"  asking\r\n").as_deref(), Some("asking"));
        assert_eq!(parse_state(b"Completed\n").as_deref(), Some("completed"));
    }

    #[test]
    fn what_shells_write() {
        // cmd: echo "loading" > state
        assert_eq!(parse_state(b"\"loading\" \r\n").as_deref(), Some("loading"));
        // PowerShell 5: "working" > state  /  Out-File
        assert_eq!(parse_state(&utf16le("working\r\n", true)).as_deref(), Some("working"));
        assert_eq!(parse_state(&utf16le("asking", false)).as_deref(), Some("asking"));
        // Set-Content -Encoding UTF8 (with a BOM)
        assert_eq!(parse_state(b"\xEF\xBB\xBFidle\r\n").as_deref(), Some("idle"));
    }

    #[test]
    fn set_writes_what_the_seal_reads() {
        let dir = std::env::temp_dir().join(format!("ai-mascot-hook-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        super::write_state(&dir, "Working\n").unwrap();
        assert_eq!(std::fs::read(dir.join("state")).unwrap(), b"working");
        super::write_state(&dir, "idle").unwrap();
        assert_eq!(std::fs::read(dir.join("state")).unwrap(), b"idle");
        super::write_state(&dir, "asking").unwrap();
        assert_eq!(parse_state(&std::fs::read(dir.join("state")).unwrap()).as_deref(), Some("asking"));
        assert!(super::write_state(&dir, "dancing").is_err());
        assert_eq!(std::fs::read(dir.join("state")).unwrap(), b"asking");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn colors_per_state() {
        let dir = std::env::temp_dir().join(format!("ai-mascot-hook-colors-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("status.txt");
        // an older file: one color for all, except where a state has its own
        std::fs::write(&path, "color=#112233\ncolor.asking=#d6336c\n").unwrap();
        let status = super::StatusText::load(&path);
        assert_eq!(rgb(status.color("loading")), (0x11, 0x22, 0x33));
        assert_eq!(rgb(status.color("asking")), (0xd6, 0x33, 0x6c));
        status.save(&path);
        let again = super::StatusText::load(&path);
        assert_eq!(rgb(again.color("error")), (0x11, 0x22, 0x33));
        assert_eq!(rgb(again.color("asking")), (0xd6, 0x33, 0x6c));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn several_lines() {
        let dir = std::env::temp_dir().join(format!("ai-mascot-hook-lines-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("status.txt");
        let mut status = super::StatusText::defaults();
        assert_eq!(status.text("loading", 7), "Thinking…");
        assert_eq!((status.text("idle", 0), status.count("idle")), ("", 0));
        status.texts[0] = "One | two |  | three".into();
        status.save(&path);
        let again = super::StatusText::load(&path);
        assert_eq!(again.count("loading"), 3);
        assert_eq!(again.text("loading", 1), "two");
        assert_eq!(again.text("loading", 5), "three");
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn rgb(c: crate::gfx::Rgba) -> (u8, u8, u8) {
        (c.0, c.1, c.2)
    }

    #[test]
    fn break_times() {
        assert_eq!(super::parse_times("12:00, 15:30"), vec![720, 930]);
        assert_eq!(super::parse_times("17:00 9.05;9:05"), vec![545, 1020]);
        assert_eq!(super::parse_times("noon, 25:00, 12:60, 7"), Vec::<u32>::new());
        assert_eq!(super::format_times(&[545, 1020]), "9:05, 17:00");
        let dir = std::env::temp_dir().join(format!("ai-mascot-hook-breaks-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("status.txt");
        let mut status = super::StatusText::defaults();
        status.breaks = vec![720];
        status.break_message = "Stretch!".into();
        status.feed_message = "Yum!".into();
        status.idle_every = 30;
        status.save(&path);
        let again = super::StatusText::load(&path);
        assert_eq!(again.breaks, vec![720]);
        assert_eq!(again.break_message, "Stretch!");
        assert_eq!(again.feed_message, "Yum!");
        assert_eq!(again.idle_every, 30);
        std::fs::write(&path, "idle.every=5\nautohide=90\n").unwrap();
        let clamped = super::StatusText::load(&path);
        assert_eq!((clamped.idle_every, clamped.autohide), (super::MIN_IDLE_EVERY, super::MAX_AUTOHIDE));
        std::fs::write(&path, "\u{feff}break.times=9:00\r\n").unwrap();
        assert_eq!(super::StatusText::load(&path).breaks, vec![540]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn nothing_yet() {
        assert_eq!(parse_state(b""), None);
        assert_eq!(parse_state(b" \r\n"), None);
        assert_eq!(parse_state(&[0xFF, 0xFE]), None);
    }
}
