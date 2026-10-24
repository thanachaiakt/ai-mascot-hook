#!/bin/bash
# Installs ai-mascot-hook from the unpacked release archive: copies the binary to ~/bin,
# points the hooks of the AI assistants you pick at it, starts it at login and starts it now.
#   ./install.sh                   install or upgrade; asks the language and which assistants
#   ./install.sh --yes             the same without asking (English unless --lang th or a Thai
#                                  locale; hooks for Claude Code and every assistant found)
#   ./install.sh --lang en|th      the language of the messages
#   ./install.sh --agents LIST     which assistants to hook: claude,cursor,copilot,gemini,codex,
#                                  qwen,windsurf,kiro (comma separated), all, or none
#   ./install.sh --uninstall       remove the hooks of every assistant, the login item, the
#                                  binary and the seal's settings (--keep keeps the settings)
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
bin="$HOME/bin/ai-mascot-hook"
plist="$HOME/Library/LaunchAgents/com.ai-mascot-hook.plist"
data="$HOME/Library/Application Support/ai-mascot-hook"

mode=install yes=n keep=n lang="" agents=""
while [ $# -gt 0 ]; do
    case "$1" in
        --uninstall) mode=uninstall ;;
        --yes) yes=y ;;
        --keep) keep=y ;;
        --lang) lang="${2:-}"; shift ;;
        --agents) agents="${2:-}"; shift ;;
        *) echo "unknown option: $1" >&2; exit 2 ;;
    esac
    shift
done

# the language: asked on a terminal, else from the locale
if [ -z "$lang" ]; then
    case "${LC_ALL:-${LANG:-}}" in th*) lang=th ;; *) lang=en ;; esac
    if [ "$yes" != y ] && [ "$mode" = install ]; then
        read -r -p "Language / ภาษา: [1] English  [2] ไทย (Enter = $([ "$lang" = th ] && echo 2 || echo 1)): " pick </dev/tty || pick=""
        case "$pick" in 1) lang=en ;; 2) lang=th ;; esac
    fi
fi
[ "$lang" = th ] || lang=en

# say KEY: a message in the chosen language
say() {
    case "$lang:$1" in
        en:found) printf 'ai-mascot-hook is already installed; updating to %s (settings are kept, the seal restarts by itself)\n' "$2" ;;
        th:found) printf 'พบ ai-mascot-hook ในเครื่องแล้ว จะอัปเดตเป็นรุ่น %s (การตั้งค่าอยู่ครบ แมวน้ำจะเปิดใหม่ให้เอง)\n' "$2" ;;
        en:intro) cat <<'EOF'

AI coding assistants run small commands called "hooks" when something happens, such as you
sending a message, a tool starting, or a turn ending, so the seal can follow along. Setup adds
one hook per event to the settings file of each assistant you pick:
    claude   ~/.claude/settings.json            cursor    ~/.cursor/hooks.json
    copilot  ~/.copilot/hooks/ai-mascot-hook.json   gemini   ~/.gemini/settings.json
    codex    ~/.codex/hooks.json                qwen      ~/.qwen/settings.json
    windsurf ~/.codeium/windsurf/hooks.json     kiro      ~/.kiro/hooks/ai-mascot-hook.json
Each hook runs "ai-mascot-hook --set <state>", which writes one word to a file in
~/Library/Application Support/ai-mascot-hook and exits. Nothing is sent over the internet.
- An existing file is backed up first, as <name>.bak-<time>
- Your other hooks and settings are kept; older ai-mascot-hook hooks are replaced
- ./install.sh --uninstall removes the hooks from every assistant
EOF
        ;;
        th:intro) cat <<'EOF'

AI เขียนโค้ดเหล่านี้รันคำสั่งเล็ก ๆ ที่เรียกว่า "ฮุก" เมื่อมีเหตุการณ์เกิดขึ้น เช่น คุณส่งข้อความ
เครื่องมือเริ่มทำงาน หรือจบหนึ่งรอบ เพื่อให้แมวน้ำขยับตาม ตัวติดตั้งจะเพิ่มฮุกหนึ่งตัวต่อหนึ่ง
เหตุการณ์ ลงในไฟล์ตั้งค่าของ AI แต่ละตัวที่คุณเลือก:
    claude   ~/.claude/settings.json            cursor    ~/.cursor/hooks.json
    copilot  ~/.copilot/hooks/ai-mascot-hook.json   gemini   ~/.gemini/settings.json
    codex    ~/.codex/hooks.json                qwen      ~/.qwen/settings.json
    windsurf ~/.codeium/windsurf/hooks.json     kiro      ~/.kiro/hooks/ai-mascot-hook.json
แต่ละฮุกรันคำสั่ง "ai-mascot-hook --set <สถานะ>" ซึ่งเขียนคำเดียวลงไฟล์ใน
~/Library/Application Support/ai-mascot-hook แล้วจบทันที ไม่มีการส่งข้อมูลผ่านอินเทอร์เน็ต
- ไฟล์เดิมจะถูกสำรองไว้ก่อน เป็น <ชื่อไฟล์>.bak-<เวลา>
- ฮุกและการตั้งค่าอื่นของคุณอยู่ครบ ฮุก ai-mascot-hook รุ่นเก่าจะถูกแทนที่
- ./install.sh --uninstall จะเอาฮุกออกจาก AI ทุกตัวให้
EOF
        ;;
        en:ask) printf 'Hook which assistants? Comma separated, "all" or "none" (Enter = %s): ' "$2" ;;
        th:ask) printf 'จะเพิ่มฮุกให้ AI ตัวไหน? คั่นด้วยจุลภาค หรือพิมพ์ all / none (Enter = %s): ' "$2" ;;
        en:skipped) printf 'No hooks added. The seal still runs but follows no assistant until you run: %s --install-hooks --agent <name>\n' "$2" ;;
        th:skipped) printf 'ไม่ได้เพิ่มฮุก แมวน้ำยังทำงานได้ แต่จะไม่ขยับตาม AI จนกว่าจะรัน: %s --install-hooks --agent <ชื่อ>\n' "$2" ;;
        en:done) printf 'Installed %s. Restart the assistants you hooked so they load the hooks.\n' "$2" ;;
        th:done) printf 'ติดตั้ง %s เรียบร้อย เปิด AI ที่เพิ่มฮุกให้ใหม่เพื่อโหลดฮุก\n' "$2" ;;
        en:removed) printf 'Uninstalled; the hooks and the settings in %s are removed.\n' "$2" ;;
        th:removed) printf 'ถอนการติดตั้งแล้ว ลบฮุกและการตั้งค่าใน %s เรียบร้อย\n' "$2" ;;
        en:kept) printf 'Uninstalled; the hooks are removed. The settings stay in %s (delete it if you do not need it).\n' "$2" ;;
        th:kept) printf 'ถอนการติดตั้งแล้ว ลบฮุกแล้ว การตั้งค่ายังอยู่ใน %s (ลบเองได้ถ้าไม่ใช้แล้ว)\n' "$2" ;;
    esac
}

stop_seal() {
    launchctl unload "$plist" 2>/dev/null || true
    pkill -x ai-mascot-hook 2>/dev/null || true
}

if [ "$mode" = uninstall ]; then
    stop_seal
    # every assistant that has our hooks, not only the ones chosen at install time
    [ -x "$bin" ] && "$bin" --remove-hooks --agent all
    rm -f "$plist" "$bin"
    # the seal's own settings (placement, status.txt, state) sit outside the binary's folder
    if [ "$keep" = y ]; then
        say kept "$data"
    else
        rm -rf "$data"
        say removed "$data"
    fi
    exit 0
fi

# an update keeps everything: the settings stay, the hooks are refreshed, the seal restarts.
# The installed binary is not asked for its version: builds before 0.1.2 would start the seal
# instead of answering.
new="$("$here/ai-mascot-hook" --version 2>/dev/null || true)"
hooked=""
if [ -x "$bin" ]; then
    say found "${new:-new}"
    # the assistants already hooked were agreed to once: they stay the default
    for pair in claude:.claude/settings.json cursor:.cursor/hooks.json copilot:.copilot/hooks/ai-mascot-hook.json \
        gemini:.gemini/settings.json codex:.codex/hooks.json qwen:.qwen/settings.json \
        windsurf:.codeium/windsurf/hooks.json kiro:.kiro/hooks/ai-mascot-hook.json; do
        if grep -q "ai-mascot-hook" "$HOME/${pair#*:}" 2>/dev/null; then hooked="${hooked:+$hooked,}${pair%%:*}"; fi
    done
fi

stop_seal
mkdir -p "$HOME/bin" "$HOME/Library/LaunchAgents"
cp "$here/ai-mascot-hook" "$bin"
chmod +x "$bin"
# the binary is not notarized: a browser download marks it, and Gatekeeper would block it
xattr -d com.apple.quarantine "$bin" 2>/dev/null || true

# editing other programs' settings needs the user's say-so: say what changes, then ask. The
# default is what is already hooked, else Claude Code plus every assistant found here.
if [ -z "$agents" ]; then
    if [ -n "$hooked" ]; then
        default="$hooked"
    else
        default=claude
        for pair in cursor:.cursor copilot:.copilot gemini:.gemini codex:.codex qwen:.qwen windsurf:.codeium/windsurf kiro:.kiro; do
            [ -d "$HOME/${pair#*:}" ] && default="$default,${pair%%:*}"
        done
    fi
    agents="$default"
    if [ "$yes" != y ]; then
        say intro
        echo
        say ask "$default"
        read -r answer </dev/tty || answer=none
        [ -n "$answer" ] && agents="$answer"
    fi
fi
if [ "$agents" = none ] || [ -z "$agents" ]; then
    say skipped "$bin"
else
    "$bin" --install-hooks --agent "$agents"
fi

cat > "$plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key><string>com.ai-mascot-hook</string>
  <key>ProgramArguments</key><array><string>$bin</string></array>
  <key>RunAtLoad</key><true/>
</dict>
</plist>
EOF
launchctl load "$plist"

say done "$bin"
