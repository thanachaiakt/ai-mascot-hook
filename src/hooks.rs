//! `ai-mascot-hook --install-hooks` / `--remove-hooks`: point an agent's hooks at this exe, or
//! take them out again. Claude Code (`~/.claude/settings.json`) is the default; `--agent cursor`,
//! `copilot` or `gemini` (or `all`, which skips agents that are not installed) do the same for
//! Cursor (`~/.cursor/hooks.json`), Copilot CLI (`~/.copilot/hooks/ai-mascot-hook.json`, a file
//! of our own), Gemini CLI (`~/.gemini/settings.json`), OpenAI Codex (`~/.codex/hooks.json`),
//! Qwen Code (`~/.qwen/settings.json`), Windsurf (`~/.codeium/windsurf/hooks.json`)
//! and Kiro (`~/.kiro/hooks/ai-mascot-hook.json`, also a file of our own).
//! `--agent` takes one name, several separated by commas, or `all`.
//! ChatGPT on the web has no hooks; Codex is its command-line agent.
//!
//! Each hook is one plain command line, `<exe> --set <state>`. Earlier hooks (any whose command
//! mentions `ai-mascot-hook`, or its former name `seal-pet`) are replaced; everything else in the
//! file is kept as it was: other hooks, other settings, key order, and any group or event of
//! yours that was already empty. Only our own entries are added, changed or removed.
//! The file is rewritten as indented JSON (2 spaces, like Claude Code writes it), through a
//! temporary file so a crash cannot leave half a settings file. The old file is saved beside it
//! as `<name>.bak-<unix time>` first; a removal that finds nothing of ours leaves the file alone.
//! `--exe <path>` names the program the hooks run, for the Windows installer script, which has
//! to hand over a path without spaces.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{json, Map, Value};

/// An agent's hooks: its event, the state it sets, the matcher it needs, if any, and what the
/// agent expects on stdout (`--reply`), if anything.
type Plan = &'static [(&'static str, &'static str, Option<&'static str>, Option<&'static str>)];

#[derive(Clone, Copy, PartialEq)]
enum Agent {
    Claude,
    Cursor,
    Copilot,
    Gemini,
    Codex,
    Qwen,
    Windsurf,
    Kiro,
}

const AGENTS: [(&str, Agent); 8] = [("claude", Agent::Claude), ("cursor", Agent::Cursor), ("copilot", Agent::Copilot), ("gemini", Agent::Gemini), ("codex", Agent::Codex), ("qwen", Agent::Qwen), ("windsurf", Agent::Windsurf), ("kiro", Agent::Kiro)];

const CLAUDE: Plan = &[
    ("SessionStart", "idle", None, None),
    ("UserPromptSubmit", "loading", None, None),
    ("PreToolUse", "working", Some("*"), None),
    ("PostToolUse", "working", Some("*"), None),
    ("PermissionRequest", "asking", Some("*"), None),
    ("Notification", "asking", Some("permission_prompt|idle_prompt|elicitation_dialog"), None),
    ("Stop", "completed", None, None),
    // a turn cut short by an API error (rate limit, auth, server) ends here instead of Stop
    ("StopFailure", "error", None, None),
    ("SessionEnd", "idle", None, None),
];

// Cursor blocks a tool whose preToolUse hook prints nothing, so that one answers "allow"
const CURSOR: Plan = &[
    ("sessionStart", "idle", None, None),
    ("beforeSubmitPrompt", "loading", None, Some("continue")),
    ("preToolUse", "working", None, Some("allow")),
    ("postToolUse", "working", None, None),
    ("stop", "completed", None, None),
    ("sessionEnd", "idle", None, None),
];

// Copilot's permissionRequest runs before every tool, even one it allows on its own, so
// asking waits for the notification that the user is actually being asked
const COPILOT: Plan = &[
    ("sessionStart", "idle", None, None),
    ("userPromptSubmitted", "loading", None, None),
    ("preToolUse", "working", None, None),
    ("postToolUse", "working", None, None),
    ("notification", "asking", Some("permission_prompt|elicitation_dialog"), None),
    ("agentStop", "completed", None, None),
    ("errorOccurred", "error", None, None),
    ("sessionEnd", "idle", None, None),
];

const GEMINI: Plan = &[
    ("SessionStart", "idle", None, None),
    ("BeforeAgent", "loading", None, None),
    ("BeforeTool", "working", None, None),
    ("AfterTool", "working", None, None),
    ("Notification", "asking", None, None),
    ("AfterAgent", "completed", None, None),
    ("SessionEnd", "idle", None, None),
];

// Codex wants JSON from a Stop hook, so that one answers "{}". Its hooks need a review in
// `/hooks` before they run.
const CODEX: Plan = &[
    ("SessionStart", "idle", None, None),
    ("UserPromptSubmit", "loading", None, None),
    ("PreToolUse", "working", Some("*"), None),
    ("PostToolUse", "working", Some("*"), None),
    ("PermissionRequest", "asking", Some("*"), None),
    ("Stop", "completed", None, Some("empty")),
    ("SessionEnd", "idle", None, None),
];

// Windsurf (Cascade) has no events for asking or idle, and times its hooks itself
const WINDSURF: Plan = &[
    ("pre_user_prompt", "loading", None, None),
    ("pre_read_code", "working", None, None),
    ("pre_write_code", "working", None, None),
    ("pre_run_command", "working", None, None),
    ("pre_mcp_tool_use", "working", None, None),
    ("post_cascade_response", "completed", None, None),
];

// Kiro has no event for asking; its SessionEnd is only in the CLI, so it is left out
const KIRO: Plan = &[
    ("SessionStart", "idle", None, None),
    ("UserPromptSubmit", "loading", None, None),
    ("PreToolUse", "working", None, None),
    ("PostToolUse", "working", None, None),
    ("Stop", "completed", None, None),
];

impl Agent {
    fn plan(self) -> Plan {
        match self {
            Agent::Claude | Agent::Qwen => CLAUDE,
            Agent::Windsurf => WINDSURF,
            Agent::Kiro => KIRO,
            Agent::Cursor => CURSOR,
            Agent::Copilot => COPILOT,
            Agent::Gemini => GEMINI,
            Agent::Codex => CODEX,
        }
    }

    fn name(self) -> &'static str {
        AGENTS.iter().find(|(_, a)| *a == self).map_or("", |(n, _)| n)
    }

    /// The agent's own folder in the home folder; `all` only touches agents where it exists.
    fn dir(self, home: &Path) -> PathBuf {
        match self {
            Agent::Claude => home.join(".claude"),
            Agent::Cursor => home.join(".cursor"),
            Agent::Copilot => std::env::var_os("COPILOT_HOME").map(PathBuf::from).unwrap_or_else(|| home.join(".copilot")),
            Agent::Gemini => home.join(".gemini"),
            Agent::Codex => home.join(".codex"),
            Agent::Qwen => home.join(".qwen"),
            Agent::Kiro => home.join(".kiro"),
            Agent::Windsurf => home.join(".codeium").join("windsurf"),
        }
    }

    fn file(self, home: &Path) -> PathBuf {
        let dir = self.dir(home);
        match self {
            Agent::Claude | Agent::Gemini | Agent::Qwen => dir.join("settings.json"),
            Agent::Cursor | Agent::Codex | Agent::Windsurf => dir.join("hooks.json"),
            Agent::Copilot | Agent::Kiro => dir.join("hooks").join("ai-mascot-hook.json"),
        }
    }

    /// How long the agent lets a hook run: Gemini counts milliseconds, the others seconds.
    fn timeout(self) -> u32 {
        if self == Agent::Gemini { 10_000 } else { 10 }
    }
}

/// Install (or with `install` false, remove) the hooks, saying what happened. The exit code.
/// `exe` overrides the path the hooks run; by default it is this program. `agent` names the
/// agents, separated by commas (`claude` when absent), or `all`: on install, Claude Code and
/// every agent found on this computer; on removal, every agent that has a hooks file.
pub fn run(install: bool, exe: Option<&str>, agent: Option<&str>) -> i32 {
    let Some(home) = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).map(PathBuf::from) else {
        return fail("cannot find your home folder");
    };
    let mut agents: Vec<Agent> = Vec::new();
    for name in agent.unwrap_or("claude").split(',').map(str::trim) {
        if name == "all" {
            agents.extend(AGENTS.iter().map(|&(_, a)| a).filter(|a| if install { a.dir(&home).is_dir() || *a == Agent::Claude } else { a.file(&home).exists() }));
        } else if let Some(&(_, a)) = AGENTS.iter().find(|(n, _)| *n == name) {
            agents.push(a);
        } else {
            return fail(&format!("unknown agent \"{name}\"; use claude, cursor, copilot, gemini, codex, qwen, windsurf, kiro or all"));
        }
    }
    agents.sort_by_key(|a| *a as u8);
    agents.dedup();
    let command = if install {
        match exe_command(exe) {
            Ok(c) => Some(c),
            Err(e) => return fail(&e),
        }
    } else {
        None
    };
    let mut code = 0;
    for agent in agents {
        match apply(agent, &home, command.as_deref()) {
            Ok(done) => println!("{done}"),
            Err(e) => code = fail(&format!("{}: {e}", agent.name())),
        }
    }
    code
}

fn fail(message: &str) -> i32 {
    eprintln!("ai-mascot-hook: {message}");
    1
}

fn apply(agent: Agent, home: &Path, command: Option<&str>) -> Result<String, String> {
    let path = agent.file(home);
    let name = agent.name();
    if matches!(agent, Agent::Copilot | Agent::Kiro) {
        // the file is wholly ours, so it is written or deleted, with nothing to merge or back up
        return match command {
            Some(c) => {
                let mut settings = json!({});
                if agent == Agent::Kiro {
                    // Kiro lists its hooks in an array, each naming its trigger
                    let hooks: Vec<Value> = agent.plan().iter().map(|&(trigger, state, _, _)| {
                        json!({ "name": format!("ai-mascot-hook {state}"), "trigger": trigger, "action": { "type": "command", "command": format!("{c} --set {state}") }, "timeout": agent.timeout() })
                    }).collect();
                    settings = json!({ "version": "v1", "hooks": hooks });
                } else {
                    edit(&mut settings, agent, Some(c))?;
                }
                write(&path, &settings)?;
                Ok(format!("ai-mascot-hook hooks installed in {} ({c}). Restart {name} to load them.", path.display()))
            }
            None if path.exists() => {
                fs::remove_file(&path).map_err(|e| format!("cannot delete {}: {e}", path.display()))?;
                Ok(format!("ai-mascot-hook hooks removed from {}.", path.display()))
            }
            None => Ok(format!("No ai-mascot-hook hooks in {}; nothing to remove.", path.display())),
        };
    }

    let mut settings = read(&path)?;
    if !edit(&mut settings, agent, command)? {
        return Ok(format!("No ai-mascot-hook hooks in {}; nothing to remove.", path.display()));
    }
    if path.exists() {
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        let file = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let backup = path.with_file_name(format!("{file}.bak-{stamp}"));
        fs::copy(&path, &backup).map_err(|e| format!("cannot back up {}: {e}", path.display()))?;
    }
    // a removal that leaves nothing (or only Cursor's version number) deletes the file we made
    let bare = settings.as_object().is_some_and(|o| o.keys().all(|k| k == "version"));
    if command.is_none() && bare {
        fs::remove_file(&path).map_err(|e| format!("cannot delete {}: {e}", path.display()))?;
        return Ok(format!("ai-mascot-hook hooks removed from {} (the file is gone, as it held nothing else).", path.display()));
    }
    write(&path, &settings)?;
    Ok(match command {
        Some(c) => format!("ai-mascot-hook hooks installed in {} ({c}). Restart {name} to load them.", path.display()),
        None => format!("ai-mascot-hook hooks removed from {}.", path.display()),
    })
}

/// Write `value` as indented JSON through a temporary file, making its folder first.
fn write(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    }
    let text = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
    let temp = path.with_extension("json.tmp");
    fs::write(&temp, text + "\n").map_err(|e| format!("cannot write {}: {e}", temp.display()))?;
    fs::rename(&temp, path).map_err(|e| format!("cannot replace {}: {e}", path.display()))
}

/// Replace our hooks in `settings` with the current ones (`command` is the exe line), or just
/// remove them when it is `None`. Whether the file needs rewriting: always after an install,
/// after a removal only if something of ours was there.
fn edit(settings: &mut Value, agent: Agent, command: Option<&str>) -> Result<bool, String> {
    let root = settings.as_object_mut().ok_or("settings file does not hold an object")?;
    // a removal must not invent a "hooks" key the file never had
    if command.is_none() && !root.contains_key("hooks") {
        return Ok(false);
    }
    // Cursor and Copilot want a version number in the file
    if command.is_some() && matches!(agent, Agent::Cursor | Agent::Copilot) {
        root.entry("version").or_insert(json!(1));
    }
    let hooks = root.entry("hooks").or_insert_with(|| Value::Object(Map::new()));
    let hooks = hooks.as_object_mut().ok_or("\"hooks\" in the settings file is not an object")?;
    let removed = remove_ours(hooks);
    let Some(command) = command else {
        // a "hooks" key that only held ours goes too, so a file we filled leaves empty-handed
        if removed > 0 && hooks.is_empty() {
            root.remove("hooks");
        }
        return Ok(removed > 0);
    };
    for &(event, state, matcher, reply) in agent.plan() {
        let line = match reply {
            Some(reply) => format!("{command} --set {state} --reply {reply}"),
            None => format!("{command} --set {state}"),
        };
        let entry = match agent {
            // one entry per hook, with no group around it
            Agent::Cursor => json!({ "command": line, "timeout": agent.timeout() }),
            Agent::Windsurf => json!({ "command": line }),
            Agent::Kiro => unreachable!("Kiro's file is built whole in apply"),
            Agent::Copilot => {
                let mut entry = json!({ "type": "command", "command": line, "timeoutSec": agent.timeout() });
                if let Some(m) = matcher {
                    entry["matcher"] = json!(m);
                }
                entry
            }
            // a group of hooks sharing a matcher
            Agent::Claude | Agent::Gemini | Agent::Codex | Agent::Qwen => {
                let mut group = Map::new();
                if let Some(m) = matcher {
                    group.insert("matcher".into(), json!(m));
                }
                group.insert("hooks".into(), json!([{ "type": "command", "command": line, "timeout": agent.timeout() }]));
                Value::Object(group)
            }
        };
        let list = hooks.entry(event).or_insert_with(|| json!([]));
        list.as_array_mut().ok_or(format!("\"hooks.{event}\" in the settings file is not a list"))?.push(entry);
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
            // Cursor and Copilot list the hooks themselves, with no group around them
            if group.get("command").is_some() {
                let mine = ours(group);
                removed += mine as usize;
                return !mine;
            }
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

/// This exe (or `exe`, when given), written so the shell the agent runs hooks in reads it as
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

    use super::Agent;

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
        assert!(super::edit(&mut settings, Agent::Claude, Some("/bin/ai-mascot-hook")).unwrap());
        assert_eq!(settings["model"], before["model"]);
        assert_eq!(settings["permissions"], before["permissions"]);
        let stop = settings["hooks"]["Stop"].as_array().unwrap();
        assert_eq!(stop[0], before["hooks"]["Stop"][0]);
        assert_eq!(stop.len(), 2);
        assert_eq!(settings["hooks"].as_object().unwrap().len(), super::CLAUDE.len());
        // a second install replaces ours instead of stacking
        let once = settings.clone();
        super::edit(&mut settings, Agent::Claude, Some("/bin/ai-mascot-hook")).unwrap();
        assert_eq!(settings, once);
        // and removing puts the file back as it was
        assert!(super::edit(&mut settings, Agent::Claude, None).unwrap());
        assert_eq!(settings, before);
    }

    #[test]
    fn removal_does_not_invent_hooks() {
        let mut settings = json!({ "model": "opus" });
        assert!(!super::edit(&mut settings, Agent::Claude, None).unwrap());
        assert_eq!(settings, json!({ "model": "opus" }));
    }

    #[test]
    fn refuses_a_hooks_entry_it_cannot_extend() {
        let mut settings = json!({ "hooks": { "Stop": "oops" } });
        assert!(super::edit(&mut settings, Agent::Claude, Some("x")).is_err());
    }

    #[test]
    fn cursor_hooks_are_flat_and_answer_where_needed() {
        let mut settings = json!({ "version": 1, "hooks": { "stop": [{ "command": "say done" }] } });
        let before = settings.clone();
        assert!(super::edit(&mut settings, Agent::Cursor, Some("/bin/ai-mascot-hook")).unwrap());
        let tool = &settings["hooks"]["preToolUse"][0];
        assert_eq!(tool["command"], "/bin/ai-mascot-hook --set working --reply allow");
        assert_eq!(settings["hooks"]["stop"][0], before["hooks"]["stop"][0]);
        assert_eq!(settings["hooks"]["stop"].as_array().unwrap().len(), 2);
        assert!(super::edit(&mut settings, Agent::Cursor, None).unwrap());
        assert_eq!(settings, before);
    }

    #[test]
    fn copilot_asks_only_on_a_prompt() {
        let mut settings = json!({});
        super::edit(&mut settings, Agent::Copilot, Some("x")).unwrap();
        assert!(settings["hooks"].get("permissionRequest").is_none());
        let asking = &settings["hooks"]["notification"][0];
        assert_eq!(asking["command"], "x --set asking");
        assert_eq!(asking["matcher"], "permission_prompt|elicitation_dialog");
        assert!(settings["hooks"]["preToolUse"][0].get("matcher").is_none());
    }

    #[test]
    fn removal_drops_a_hooks_key_it_emptied() {
        let mut settings = json!({ "model": "opus" });
        super::edit(&mut settings, Agent::Gemini, Some("/bin/ai-mascot-hook")).unwrap();
        assert!(super::edit(&mut settings, Agent::Gemini, None).unwrap());
        assert_eq!(settings, json!({ "model": "opus" }));
    }

    #[test]
    fn windsurf_is_flat_and_qwen_follows_claude() {
        let mut windsurf = json!({});
        super::edit(&mut windsurf, Agent::Windsurf, Some("x")).unwrap();
        assert_eq!(windsurf["hooks"]["post_cascade_response"][0], json!({ "command": "x --set completed" }));
        assert!(windsurf.get("version").is_none());
        let mut qwen = json!({});
        let mut claude = json!({});
        super::edit(&mut qwen, Agent::Qwen, Some("x")).unwrap();
        super::edit(&mut claude, Agent::Claude, Some("x")).unwrap();
        assert_eq!(qwen, claude);
    }

    #[test]
    fn codex_stop_answers_with_json() {
        let mut settings = json!({});
        super::edit(&mut settings, Agent::Codex, Some("x")).unwrap();
        assert_eq!(settings["hooks"]["Stop"][0]["hooks"][0]["command"], "x --set completed --reply empty");
        assert_eq!(settings["hooks"]["Stop"][0]["hooks"][0]["timeout"], 10);
    }

    #[test]
    fn gemini_counts_milliseconds_and_copilot_names_its_timeout() {
        let mut gemini = json!({});
        super::edit(&mut gemini, Agent::Gemini, Some("x")).unwrap();
        assert_eq!(gemini["hooks"]["AfterAgent"][0]["hooks"][0]["timeout"], 10_000);
        let mut copilot = json!({});
        super::edit(&mut copilot, Agent::Copilot, Some("x")).unwrap();
        assert_eq!(copilot["version"], 1);
        assert_eq!(copilot["hooks"]["agentStop"][0], json!({ "type": "command", "command": "x --set completed", "timeoutSec": 10 }));
    }
}
