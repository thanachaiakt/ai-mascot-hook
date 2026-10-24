#!/bin/bash
# Installs ai-mascot-hook from the unpacked release archive: copies the binary to ~/bin,
# points Claude Code's hooks at it, starts it at login and starts it now.
#   ./install.sh             install or upgrade, asking before it edits ~/.claude/settings.json
#   ./install.sh --yes       the same without asking
#   ./install.sh --uninstall remove the hooks, the login item and the binary; asks whether to
#                            also delete the seal's settings (--uninstall --purge deletes them
#                            without asking, --uninstall --keep never does)
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
bin="$HOME/bin/ai-mascot-hook"
plist="$HOME/Library/LaunchAgents/com.ai-mascot-hook.plist"
data="$HOME/Library/Application Support/ai-mascot-hook"

stop_seal() {
    launchctl unload "$plist" 2>/dev/null || true
    pkill -x ai-mascot-hook 2>/dev/null || true
}

if [ "${1:-}" = "--uninstall" ]; then
    stop_seal
    [ -x "$bin" ] && "$bin" --remove-hooks
    rm -f "$plist" "$bin"
    # the seal's own settings (placement, status.txt, state) sit outside the binary's folder,
    # so they go only if the user says so
    purge=n
    case "${2:-}" in
        --purge) purge=y ;;
        --keep) ;;
        *) read -r -p "ลบการตั้งค่าของ ai-mascot-hook ด้วยหรือไม่ (ตำแหน่ง ข้อความและสีของฟองสถานะ)? [y/N] " purge </dev/tty || purge=n ;;
    esac
    case "$purge" in
        [Yy]*) rm -rf "$data"; echo "ถอนการติดตั้งแล้ว และลบการตั้งค่าใน $data เรียบร้อย" ;;
        *) echo "ถอนการติดตั้งแล้ว การตั้งค่ายังอยู่ใน $data (ลบเองได้ถ้าไม่ใช้แล้ว)" ;;
    esac
    exit 0
fi

# an update keeps everything: the settings stay, the hooks are refreshed, the seal restarts.
# The installed binary is not asked for its version: builds before 0.1.2 would start the seal
# instead of answering.
new="$("$here/ai-mascot-hook" --version 2>/dev/null || true)"
if [ -x "$bin" ]; then
    echo "พบ ai-mascot-hook ในเครื่องแล้ว จะอัปเดตเป็นรุ่น ${new:-ใหม่} (การตั้งค่าอยู่ครบ แมวน้ำจะเปิดใหม่ให้เอง)"
    # hooks already in place were agreed to once: refresh them without asking again
    if grep -q "ai-mascot-hook" "$HOME/.claude/settings.json" 2>/dev/null; then
        set -- --yes
    fi
fi

stop_seal
mkdir -p "$HOME/bin" "$HOME/Library/LaunchAgents"
cp "$here/ai-mascot-hook" "$bin"
chmod +x "$bin"
# the binary is not notarized: a browser download marks it, and Gatekeeper would block it
xattr -d com.apple.quarantine "$bin" 2>/dev/null || true

# editing another program's settings needs the user's say-so: say what changes, then ask
cat <<EOF

Claude Code จะรันคำสั่งเล็ก ๆ ที่เรียกว่า "ฮุก" เมื่อมีเหตุการณ์เกิดขึ้น เช่น คุณส่งข้อความ
เครื่องมือเริ่มทำงาน Claude ขออนุญาต หรือจบหนึ่งรอบ เพื่อให้แมวน้ำขยับตาม Claude
ตัวติดตั้งจะเพิ่มฮุกหนึ่งตัวต่อหนึ่งเหตุการณ์ ลงในไฟล์:
    $HOME/.claude/settings.json

แต่ละฮุกรันคำสั่ง "$bin --set <สถานะ>" ซึ่งเขียนคำเดียวลงไฟล์ใน
~/Library/Application Support/ai-mascot-hook แล้วจบทันที ไม่มีการส่งข้อมูลผ่านอินเทอร์เน็ต
- ไฟล์เดิมจะถูกสำรองไว้ก่อน เป็น settings.json.bak-<เวลา>
- ฮุกและการตั้งค่าอื่นของคุณอยู่ครบ ฮุก ai-mascot-hook รุ่นเก่าจะถูกแทนที่
- ./install.sh --uninstall จะเอาฮุกเหล่านี้ออกให้

EOF
answer=y
if [ "${1:-}" != "--yes" ]; then
    read -r -p "เพิ่มฮุกลงใน settings.json หรือไม่? [Y/n] " answer </dev/tty || answer=n
fi
case "$answer" in
    ""|[Yy]*) "$bin" --install-hooks ;;
    *) echo "ข้ามขั้นตอนนี้ แมวน้ำยังทำงานได้ แต่จะไม่ขยับตาม Claude จนกว่าจะรัน: $bin --install-hooks" ;;
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

echo "ติดตั้ง $bin เรียบร้อย เปิด Claude Code ใหม่เพื่อโหลดฮุก"
