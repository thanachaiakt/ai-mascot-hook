//! Where ai-mascot-hook keeps things: the state file the hooks write, and the seal's placement.

use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

use crate::seal::{MAX_H, MIN_H};
use crate::status::parse_state;

/// `%LOCALAPPDATA%\ai-mascot-hook` on Windows, `~/Library/Application Support/ai-mascot-hook` on a Mac;
/// the system's temporary folder stands in when neither can be found.
pub fn data_dir() -> PathBuf {
    #[cfg(windows)]
    let base = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);
    #[cfg(not(windows))]
    let base = std::env::var_os("HOME").map(|home| PathBuf::from(home).join("Library").join("Application Support"));
    base.unwrap_or_else(std::env::temp_dir).join("ai-mascot-hook")
}

/// Every state file a hook may have written. Hooks run by a packaged (MSIX) app, such as
/// the Claude desktop app, have their `%LOCALAPPDATA%` writes redirected to
/// `%LOCALAPPDATA%\Packages\<app>\LocalCache\Local`, so those copies count too. The
/// packages are looked through again every few seconds, not on every poll.
#[cfg(windows)]
pub fn state_files() -> Vec<PathBuf> {
    use std::cell::RefCell;
    use std::time::Instant;

    use crate::seal::RESCAN;

    thread_local! {
        static FOUND: RefCell<Option<(Instant, Vec<PathBuf>)>> = const { RefCell::new(None) };
    }
    FOUND.with(|found| {
        let mut found = found.borrow_mut();
        if found.as_ref().map_or(true, |(at, _)| at.elapsed() >= RESCAN) {
            let mut files = vec![data_dir().join("state")];
            if let Some(base) = std::env::var_os("LOCALAPPDATA") {
                if let Ok(packages) = fs::read_dir(PathBuf::from(base).join("Packages")) {
                    for package in packages.flatten() {
                        let file = package.path().join("LocalCache").join("Local").join("ai-mascot-hook").join("state");
                        if file.is_file() {
                            files.push(file);
                        }
                    }
                }
            }
            *found = Some((Instant::now(), files));
        }
        found.as_ref().map(|(_, files)| files.clone()).unwrap_or_default()
    })
}

/// The one state file a hook writes.
#[cfg(not(windows))]
pub fn state_files() -> Vec<PathBuf> {
    vec![data_dir().join("state")]
}

/// The state word and modified time of the newest state file. The word is `None` when
/// there is no file or it holds no word yet (caught mid-write), so the caller can look again.
pub fn read_state() -> (Option<String>, Option<SystemTime>) {
    let newest = state_files()
        .into_iter()
        .filter_map(|path| Some((fs::metadata(&path).and_then(|m| m.modified()).ok()?, path)))
        .max_by_key(|(stamp, _)| *stamp);
    let Some((stamp, path)) = newest else { return (None, None) };
    let stamp = Some(stamp);
    let state = fs::read(&path).ok().and_then(|bytes| parse_state(&bytes));
    (state, stamp)
}

/// `x y height show_status` from the last run; older runs saved a scale in place of the height.
pub fn read_placement(default_h: i32) -> Option<(i32, i32, i32, bool)> {
    let text = fs::read_to_string(data_dir().join("placement")).ok()?;
    let mut parts = text.split_whitespace();
    let x = parts.next()?.parse().ok()?;
    let y = parts.next()?.parse().ok()?;
    let size: f32 = parts.next()?.parse().ok()?;
    let height = if size <= 4.0 { (default_h as f32 * size).round() as i32 } else { size as i32 };
    let status = parts.next().map(|s| s == "1").unwrap_or(false);
    Some((x, y, height.clamp(MIN_H, MAX_H), status))
}

/// Remember where the seal is, how big, and whether its bubble shows.
pub fn save_placement(x: i32, y: i32, height: i32, show_status: bool) {
    let _ = fs::create_dir_all(data_dir());
    let _ = fs::write(data_dir().join("placement"), format!("{x} {y} {height} {}", show_status as u8));
}
