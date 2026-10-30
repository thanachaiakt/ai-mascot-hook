//! Where ai-mascot-hook keeps things: the state file the hooks write, the seal's placement,
//! and the frames file beside the exe.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::SystemTime;

use crate::seal::{MAX_H, MIN_H};
use crate::status::parse_state;

// FRAMES_ID and STRIP_NAMES, from build.rs
include!(concat!(env!("OUT_DIR"), "/frames.rs"));

/// The seal's art, which `build.rs` writes beside the exe rather than into it (its header
/// says why, and how the file is laid out).
pub const FRAMES_FILE: &str = "ai-mascot-hook.frames";

/// The frames file, mapped for the rest of the run and checked against this build, or what
/// is wrong with it. Mapped pages are not counted as the app's own memory, just as the
/// frames were not when they were in the exe.
pub fn frames_file() -> Result<&'static [u8], String> {
    static MAPPED: OnceLock<Result<&'static [u8], String>> = OnceLock::new();
    MAPPED
        .get_or_init(|| {
            let path = std::env::current_exe().map_err(|e| format!("cannot find this program's path: {e}"))?.with_file_name(FRAMES_FILE);
            let bytes = map(&path).map_err(|e| format!("cannot open {}: {e}", path.display()))?;
            let word = |i: usize| bytes.get(i..i + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()) as usize);
            let fits = (0..STRIP_NAMES.len()).all(|s| matches!((word(16 + s * 8), word(20 + s * 8)), (Some(at), Some(len)) if at + len <= bytes.len()));
            if bytes.get(..4) != Some(b"AMHF") || bytes.get(4..12) != Some(&FRAMES_ID.to_le_bytes()[..]) || word(12) != Some(STRIP_NAMES.len()) || !fits {
                return Err(format!("{} is not this version's; reinstall ai-mascot-hook", path.display()));
            }
            Ok(bytes)
        })
        .clone()
}

/// Strip `name`'s frames, as `Strip::decode` reads them. `main` has made sure the frames
/// file is good before any window opens.
pub fn strip_frames(name: &str) -> &'static [u8] {
    let bytes = frames_file().expect("frames file checked at start");
    let s = STRIP_NAMES.iter().position(|n| *n == name).expect("strip built by build.rs");
    let word = |i: usize| u32::from_le_bytes(bytes[i..i + 4].try_into().unwrap()) as usize;
    let (at, len) = (word(16 + s * 8), word(20 + s * 8));
    &bytes[at..at + len]
}

/// `path` mapped read-only, never unmapped. Others may read the file meanwhile but not write
/// it, so the mapped bytes cannot change under the seal.
#[cfg(windows)]
fn map(path: &Path) -> std::io::Result<&'static [u8]> {
    use std::os::windows::fs::OpenOptionsExt;
    use std::os::windows::io::AsRawHandle;

    use windows_sys::Win32::Storage::FileSystem::FILE_SHARE_READ;
    use windows_sys::Win32::System::Memory::{CreateFileMappingW, MapViewOfFile, FILE_MAP_READ, PAGE_READONLY};

    let file = fs::OpenOptions::new().read(true).share_mode(FILE_SHARE_READ).open(path)?;
    let len = file.metadata()?.len() as usize;
    if len == 0 {
        return Ok(&[]);
    }
    // the handle stays open with the mapping, as it is what keeps writers out
    let file = std::mem::ManuallyDrop::new(file);
    unsafe {
        let mapping = CreateFileMappingW(file.as_raw_handle(), std::ptr::null(), PAGE_READONLY, 0, 0, std::ptr::null());
        if mapping.is_null() {
            return Err(std::io::Error::last_os_error());
        }
        let view = MapViewOfFile(mapping, FILE_MAP_READ, 0, 0, 0);
        if view.Value.is_null() {
            return Err(std::io::Error::last_os_error());
        }
        Ok(std::slice::from_raw_parts(view.Value as *const u8, len))
    }
}

/// `path` mapped read-only, never unmapped. The installer replaces the file only once the
/// seal is stopped.
#[cfg(not(windows))]
fn map(path: &Path) -> std::io::Result<&'static [u8]> {
    use std::ffi::{c_int, c_void};
    use std::os::fd::AsRawFd;

    extern "C" {
        fn mmap(addr: *mut c_void, len: usize, prot: c_int, flags: c_int, fd: c_int, offset: i64) -> *mut c_void;
    }
    // Darwin's PROT_READ and MAP_PRIVATE
    const PROT_READ: c_int = 1;
    const MAP_PRIVATE: c_int = 2;

    let file = fs::File::open(path)?;
    let len = file.metadata()?.len() as usize;
    if len == 0 {
        return Ok(&[]);
    }
    let at = unsafe { mmap(std::ptr::null_mut(), len, PROT_READ, MAP_PRIVATE, file.as_raw_fd(), 0) };
    // MAP_FAILED is all ones
    if at as usize == usize::MAX {
        return Err(std::io::Error::last_os_error());
    }
    Ok(unsafe { std::slice::from_raw_parts(at as *const u8, len) })
}

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
