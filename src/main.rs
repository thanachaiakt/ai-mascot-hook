//! ai-mascot-hook: a small seal that floats on the desktop and acts out what Claude Code is doing.
//!
//! Claude Code hooks run `ai-mascot-hook --set <state>` (loading, working, asking, completed,
//! idle), which writes the word to a state file; the running seal polls that file and plays
//! the matching animation in a borderless, transparent, always-on-top window, with an
//! optional status bubble. `ai-mascot-hook --install-hooks` sets those hooks up.
//!
//! The parts every platform shares live here (the clips, the state file, the bubble, the
//! hooks); `win` and `mac` hold each platform's windows.
#![cfg_attr(windows, windows_subsystem = "windows")]

mod anim;
mod bubble;
mod editor;
mod gfx;
mod hooks;
mod menu;
mod seal;
mod status;
mod store;

#[cfg(target_os = "macos")]
mod mac;
#[cfg(windows)]
mod win;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        // what the Claude Code hooks run: write the state and leave; the running seal picks
        // it up on its next poll
        Some("--set") => {
            let word = args.get(1).map(String::as_str).unwrap_or("");
            std::process::exit(if status::write_state(&store::data_dir(), word).is_ok() { 0 } else { 2 });
        }
        // for the Mac installer, which tells an update from a fresh install by this
        Some("--version") => {
            println!("{}", env!("CARGO_PKG_VERSION"));
            return;
        }
        Some("--install-hooks") => {
            let exe = args.iter().position(|a| a == "--exe").and_then(|i| args.get(i + 1));
            std::process::exit(hooks::run(true, exe.map(String::as_str)))
        }
        Some("--remove-hooks") => std::process::exit(hooks::run(false, None)),
        _ => {}
    }
    #[cfg(windows)]
    win::run();
    #[cfg(target_os = "macos")]
    mac::run();
}
