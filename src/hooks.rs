//! `ai-mascot-hook --install-hooks` / `--remove-hooks`: point Claude Code's hooks at this exe, or
//! take them out again, in `~/.claude/settings.json`.
//!
//! Each hook is one plain command line, `<exe> --set <state>`. Earlier hooks (any whose command
//! mentions `ai-mascot-hook`, or its former name `seal-pet`) are replaced; every other hook and
//! setting is kept.
//! The old file is saved beside it as `settings.json.bak-<unix time>` first.

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
pub fn run(install: bool) -> i32 {
    match apply(install) {
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

fn apply(install: bool) -> Result<String, String> {
    let home = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).ok_or("cannot find your home folder")?;
    let path = PathBuf::from(home).join(".claude").join("settings.json");
    let command = if install { Some(exe_command()?) } else { None };

    let mut settings = read(&path)?;
    if path.exists() {
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        let backup = path.with_file_name(format!("settings.json.bak-{stamp}"));
        fs::copy(&path, &backup).map_err(|e| format!("cannot back up {}: {e}", path.display()))?;
    }
    let root = settings.as_object_mut().ok_or("settings.json does not hold an object")?;
    let hooks = root.entry("hooks").or_insert_with(|| Value::Object(Map::new()));
    let hooks = hooks.as_object_mut().ok_or("\"hooks\" in settings.json is not an object")?;
    remove_ours(hooks);
    if let Some(command) = &command {
        for (event, state, matcher) in PLAN {
            let mut group = Map::new();
            if let Some(m) = matcher {
                group.insert("matcher".into(), json!(m));
            }
            group.insert("hooks".into(), json!([{ "type": "command", "command": format!("{command} --set {state}"), "timeout": 10 }]));
            let list = hooks.entry(event).or_insert_with(|| json!([]));
            if let Some(list) = list.as_array_mut() {
                list.push(Value::Object(group));
            }
        }
    }
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    }
    let text = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    fs::write(&path, text + "\n").map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    Ok(match command {
        Some(c) => format!("ai-mascot-hook hooks installed in {} ({c}). Restart Claude Code to load them.", path.display()),
        None => format!("ai-mascot-hook hooks removed from {}.", path.display()),
    })
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

/// Drop every hook whose command runs ai-mascot-hook, and any group or event left empty.
fn remove_ours(hooks: &mut Map<String, Value>) {
    let ours = |handler: &Value| handler.get("command").and_then(Value::as_str).is_some_and(|c| c.contains("ai-mascot-hook") || c.contains("seal-pet"));
    for groups in hooks.values_mut() {
        if let Some(groups) = groups.as_array_mut() {
            for group in groups.iter_mut() {
                if let Some(list) = group.get_mut("hooks").and_then(Value::as_array_mut) {
                    list.retain(|h| !ours(h));
                }
            }
            groups.retain(|g| g.get("hooks").and_then(Value::as_array).is_none_or(|l| !l.is_empty()));
        }
    }
    hooks.retain(|_, groups| groups.as_array().is_none_or(|g| !g.is_empty()));
}

/// This exe, written so the shell Claude Code runs hooks in reads it as one word.
fn exe_command() -> Result<String, String> {
    let exe = std::env::current_exe().map_err(|e| format!("cannot find this program's path: {e}"))?;
    let exe = exe.to_string_lossy().into_owned();
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
}
