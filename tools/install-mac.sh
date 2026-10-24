#!/bin/bash
# Installs ai-mascot-hook from the unpacked release archive: copies the binary to ~/bin,
# points Claude Code's hooks at it, starts it at login and starts it now.
#   ./install.sh             install or upgrade, asking before it edits ~/.claude/settings.json
#   ./install.sh --yes       the same without asking
#   ./install.sh --uninstall remove the hooks, the login item and the binary (settings stay)
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
bin="$HOME/bin/ai-mascot-hook"
plist="$HOME/Library/LaunchAgents/com.ai-mascot-hook.plist"

stop_seal() {
    launchctl unload "$plist" 2>/dev/null || true
    pkill -x ai-mascot-hook 2>/dev/null || true
}

if [ "${1:-}" = "--uninstall" ]; then
    stop_seal
    [ -x "$bin" ] && "$bin" --remove-hooks
    rm -f "$plist" "$bin"
    echo "Removed. Settings are left in ~/Library/Application Support/ai-mascot-hook/."
    exit 0
fi

stop_seal
mkdir -p "$HOME/bin" "$HOME/Library/LaunchAgents"
cp "$here/ai-mascot-hook" "$bin"
chmod +x "$bin"
# the binary is not notarized: a browser download marks it, and Gatekeeper would block it
xattr -d com.apple.quarantine "$bin" 2>/dev/null || true

# editing another program's settings needs the user's say-so: say what changes, then ask
cat <<EOF

Claude Code runs small commands, called hooks, when something happens: you send a prompt,
a tool runs, Claude asks for permission, or a turn finishes. To make the seal follow Claude,
this adds one hook per event to:
    $HOME/.claude/settings.json

Each hook runs "$bin --set <state>", which writes one word to a file in
~/Library/Application Support/ai-mascot-hook and exits. Nothing is sent over the network.
- The current file is backed up first as settings.json.bak-<time>.
- Your other hooks and settings are kept; older ai-mascot-hook hooks are replaced.
- ./install.sh --uninstall removes these hooks again.

EOF
answer=y
if [ "${1:-}" != "--yes" ]; then
    read -r -p "Add the hooks to settings.json? [Y/n] " answer </dev/tty || answer=n
fi
case "$answer" in
    ""|[Yy]*) "$bin" --install-hooks ;;
    *) echo "Skipped. The seal runs, but does not follow Claude until you run: $bin --install-hooks" ;;
esac

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

echo "Installed $bin. Restart Claude Code so it loads the new hooks."
