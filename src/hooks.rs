//! `ai-mascot-hook --install-hooks` / `--remove-hooks`: point Claude Code's hooks at this exe, or
//! take them out again, in `~/.claude/settings.json`.
//!
//! Each hook is one plain command line, `<exe> --set <state>`. Earlier hooks (any whose command
//! mentions `ai-mascot-hook`, or its former name `seal-pet`) are replaced; everything else in the
//! file is kept as it was: other hooks, other settings, key order, and any group or event of
//! yours that was already empty. Only our own entries are added, changed or removed.
//! The file is rewritten as indented JSON (2 spaces, like Claude Code writes it), through a
//! temporary file so a crash cannot leave half a settings file. The old file is saved beside it
//! as `settings.json.bak-<unix time>` first; a removal that finds nothing of ours leaves the
//! file alone.
//! `--exe <path>` names the program the hooks run, for the Windows installer script, which has
//! to hand over a path without spaces.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{json, Map, Value};

/// Which Claude Code event sets which state, and the matcher it needs, if any.
const PLAN: [(&str, &str, Option<&str>); 8] = [
    ("SessionStart", "idle", None),
    ("UserPromptSubmit", "loading", None),
    ("PreToolUse", "working", Some("*")),
    ("PostToolUse", "working", Some("*")),
    ("PermissionRequest", "asking", Some("*")),
    ("Notification", "asking", Some("permission_prompt|idle_prompt|elicitation_dialog")),
    ("Stop", "completed", None),
    ("SessionEnd", "idle", None),
];

/// Install (or with `install` false, remove) the hooks, saying what happened. The exit code.
/// `exe` overrides the path the hooks run; by default it is this program.
pub fn run(install: bool, exe: Option<&str>) -> i32 {
    match apply(install, exe) {
        Ok(done) => {
            println!("{done}");
            0
        }
        Err(e) => {
            eprintln!("ai-mascot-hook: {e}");
            1
        }
    }
}

fn apply(install: bool, exe: Option<&str>) -> Result<String, String> {
    let home = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).ok_or("cannot find your home folder")?;
    let path = PathBuf::from(home).join(".claude").join("settings.json");
    let command = if install { Some(exe_command(exe)?) } else { None };

    let mut settings = read(&path)?;
    if !edit(&mut settings, command.as_deref())? {
        return Ok(format!("No ai-mascot-hook hooks in {}; nothing to remove.", path.display()));
    }
    if path.exists() {
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        let backup = path.with_file_name(format!("settings.json.bak-{stamp}"));
        fs::copy(&path, &backup).map_err(|e| format!("cannot back up {}: {e}", path.display()))?;
    }
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    }
    let text = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    let temp = path.with_file_name("settings.json.tmp");
    fs::write(&temp, text + "\n").map_err(|e| format!("cannot write {}: {e}", temp.display()))?;
    fs::rename(&temp, &path).map_err(|e| format!("cannot replace {}: {e}", path.display()))?;
    Ok(match command {
        Some(c) => format!("ai-mascot-hook hooks installed in {} ({c}). Restart Claude Code to load them.", path.display()),
        None => format!("ai-mascot-hook hooks removed from {}.", path.display()),
    })
}

/// Replace our hooks in `settings` with the current ones (`command` is the exe line), or just
/// remove them when it is `None`. Whether the file needs rewriting: always after an install,
/// after a removal only if something of ours was there.
fn edit(settings: &mut Value, command: Option<&str>) -> Result<bool, String> {
    let root = settings.as_object_mut().ok_or("settings.json does not hold an object")?;
    // a removal must not invent a "hooks" key the file never had
    if command.is_none() && !root.contains_key("hooks") {
        return Ok(false);
    }
    let hooks = root.entry("hooks").or_insert_with(|| Value::Object(Map::new()));
    let hooks = hooks.as_object_mut().ok_or("\"hooks\" in settings.json is not an object")?;
    let removed = remove_ours(hooks);
    let Some(command) = command else { return Ok(removed > 0) };
    for (event, state, matcher) in PLAN {
        let mut group = Map::new();
        if let Some(m) = matcher {
            group.insert("matcher".into(), json!(m));
        }
        group.insert("hooks".into(), json!([{ "type": "command", "command": format!("{command} --set {state}"), "timeout": 10 }]));
        let list = hooks.entry(event).or_insert_with(|| json!([]));
        list.as_array_mut().ok_or(format!("\"hooks.{event}\" in settings.json is not a list"))?.push(Value::Object(group));
    }
    Ok(true)
}

fn read(path: &Path) -> Result<Value, String> {
    match fs::read_to_string(path) {
        Ok(text) if !text.trim().is_empty() => {
            let text = text.trim_start_matches('\u{feff}');
            serde_json::from_str(text).map_err(|e| format!("cannot read {}: {e}", path.display()))
        }
        _ => Ok(json!({})),
    }
}

/// Drop every hook whose command runs ai-mascot-hook, and any group or event that dropping
/// emptied. Groups and events that were already empty are yours and stay. How many were dropped.
fn remove_ours(hooks: &mut Map<String, Value>) -> usize {
    let ours = |handler: &Value| handler.get("command").and_then(Value::as_str).is_some_and(|c| c.contains("ai-mascot-hook") || c.contains("seal-pet"));
    let mut removed = 0;
    let mut emptied = Vec::new();
    for (event, groups) in hooks.iter_mut() {
        let Some(groups) = groups.as_array_mut() else { continue };
        let before = removed;
        groups.retain_mut(|group| {
            let Some(list) = group.get_mut("hooks").and_then(Value::as_array_mut) else { return true };
            let len = list.len();
            list.retain(|h| !ours(h));
            removed += len - list.len();
            len == list.len() || !list.is_empty()
        });
        if removed > before && groups.is_empty() {
            emptied.push(event.clone());
        }
    }
    hooks.retain(|event, _| !emptied.contains(event));
    removed
}

/// This exe (or `exe`, when given), written so the shell Claude Code runs hooks in reads it as
/// one word.
fn exe_command(exe: Option<&str>) -> Result<String, String> {
    let exe = match exe {
        Some(exe) => exe.to_string(),
        None => std::env::current_exe().map_err(|e| format!("cannot find this program's path: {e}"))?.to_string_lossy().into_owned(),
    };
    if cfg!(windows) {
        // Git Bash and PowerShell quote differently, so the path goes unquoted, with forward
        // slashes, which both read the same way (tools\install-hooks.ps1 also handles spaces)
        if exe.contains(' ') {
            return Err(format!("the path has spaces: {exe}\nMove ai-mascot-hook.exe to a folder without spaces, or use tools\\install-hooks.ps1."));
        }
        Ok(exe.replace('\\', "/"))
    } else {
        Ok(format!("'{}'", exe.replace('\'', "'\\''")))
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    #[test]
    fn keeps_other_hooks() {
        let mut hooks = json!({
            "Stop": [{ "hooks": [{ "type": "command", "command": "/old/ai-mascot-hook --set completed" }, { "type": "command", "command": "say done" }] }],
            "SessionStart": [{ "hooks": [{ "type": "command", "command": "ai-mascot-hook.exe --set idle" }] }]
        });
        super::remove_ours(hooks.as_object_mut().unwrap());
        assert_eq!(hooks, json!({ "Stop": [{ "hooks": [{ "type": "command", "command": "say done" }] }] }));
    }

    #[test]
    fn keeps_your_empty_groups_and_events() {
        let mut hooks = json!({
            "Stop": [],
            "PreToolUse": [{ "matcher": "Bash", "hooks": [] }, { "hooks": [{ "type": "command", "command": "ai-mascot-hook --set working" }] }]
        });
        assert_eq!(super::remove_ours(hooks.as_object_mut().unwrap()), 1);
        assert_eq!(hooks, json!({ "Stop": [], "PreToolUse": [{ "matcher": "Bash", "hooks": [] }] }));
    }

    #[test]
    fn install_adds_ours_and_keeps_the_rest() {
        let mut settings = json!({
            "model": "opus",
            "permissions": { "allow": ["Bash(ls)"] },
            "hooks": { "Stop": [{ "hooks": [{ "type": "command", "command": "say done" }] }] }
        });
        let before = settings.clone();
        assert!(super::edit(&mut settings, Some("/bin/ai-mascot-hook")).unwrap());
        assert_eq!(settings["model"], before["model"]);
        assert_eq!(settings["permissions"], before["permissions"]);
        let stop = settings["hooks"]["Stop"].as_array().unwrap();
        assert_eq!(stop[0], before["hooks"]["Stop"][0]);
        assert_eq!(stop.len(), 2);
        assert_eq!(settings["hooks"].as_object().unwrap().len(), super::PLAN.len());
        // a second install replaces ours instead of stacking
        let once = settings.clone();
        super::edit(&mut settings, Some("/bin/ai-mascot-hook")).unwrap();
        assert_eq!(settings, once);
        // and removing puts the file back as it was
        assert!(super::edit(&mut settings, None).unwrap());
        assert_eq!(settings, before);
    }

    #[test]
    fn removal_does_not_invent_hooks() {
        let mut settings = json!({ "model": "opus" });
        assert!(!super::edit(&mut settings, None).unwrap());
        assert_eq!(settings, json!({ "model": "opus" }));
    }

    #[test]
    fn refuses_a_hooks_entry_it_cannot_extend() {
        let mut settings = json!({ "hooks": { "Stop": "oops" } });
        assert!(super::edit(&mut settings, Some("x")).is_err());
    }
}
