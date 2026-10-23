# ai-mascot-hook (ภาษาไทย)

[English](README.md)

แมวน้ำตัวเล็กที่ลอยอยู่บนหน้าจอ Windows หรือ macOS และแสดงท่าทางตามสิ่งที่ Claude Code กำลังทำ
ตบมือตอนคิด ว่ายน้ำใส่หมวกนิรภัยตอนทำงาน โบกมือบนก้อนน้ำแข็งตอนรอคำตอบ
และนอนหลับตอนเสร็จงานหรือว่างอยู่ พร้อมกล่องข้อความบอกสถานะแบบ pixel art เข้าชุดกับตัวแมวน้ำ

| สถานะ | ภาพตัวอย่าง | ท่าทาง | ข้อความเริ่มต้น |
|---|---|---|---|
| `loading` | <img src="assets/gif/loading.gif" width="120" alt="แมวน้ำตบมือ"> | ตบมือ กำลังคิด | Thinking… |
| `working` | <img src="assets/gif/working.gif" width="120" alt="แมวน้ำว่ายน้ำใส่หมวกนิรภัย"> | ว่ายน้ำใส่หมวกนิรภัย ถือพลั่ว | Working… |
| `asking` | <img src="assets/gif/asking.gif" width="120" alt="แมวน้ำโบกมือบนก้อนน้ำแข็ง"> | โบกมือบนก้อนน้ำแข็ง รอคำตอบจากคุณ | Need your input! |
| `completed` | <img src="assets/gif/completed.gif" width="120" alt="แมวน้ำนอนหลับ"> | นอนหลับ เสร็จงานแล้ว | All done! |
| `idle` | <img src="assets/gif/completed.gif" width="120" alt="แมวน้ำนอนหลับ"> | นอนหลับ ว่างอยู่ (ท่าเดียวกับ completed) | Napping… zZ |

ภาพตัวอย่างเป็น GIF เคลื่อนไหวจาก `assets/gif/`

## สารบัญ

- [โปรแกรมทำงานอย่างไร](#โปรแกรมทำงานอย่างไร)
- [ติดตั้งบน Windows](#ติดตั้งบน-windows)
- [ติดตั้งบน macOS](#ติดตั้งบน-macos)
- [วิธีใช้งาน](#วิธีใช้งาน)
- [การเก็บ setting](#การเก็บ-setting)
- [ถอนการติดตั้ง](#ถอนการติดตั้ง)
- [แก้ปัญหา](#แก้ปัญหา)

## โปรแกรมทำงานอย่างไร

มีสองส่วนที่ทำงานร่วมกัน โดยไม่ได้คุยกันตรงๆ แต่ใช้ไฟล์เล็กๆ ไฟล์เดียวเป็นตัวกลาง
เหมือนการฝากโน้ตไว้บนโต๊ะ

```text
Claude Code ──(เรียก hook)──▶ ai-mascot-hook --set working ──▶ เขียนคำว่า "working"
                                                               ลงในไฟล์ `state`
                                                                      │
แมวน้ำบนหน้าจอ ◀──(อ่านไฟล์ทุก 0.4 วินาที)────────────────────────────┘
      └─ เห็นคำใหม่ แล้วเล่นท่าทางนั้น พร้อมแสดงกล่องข้อความ
```

1. **Hook (ผู้ส่งโน้ต)** Claude Code สั่งให้รันคำสั่งได้เมื่อมีเหตุการณ์เกิดขึ้น
   เช่น คุณส่งข้อความ, Claude ใช้ tool, ขออนุญาต, หรือตอบเสร็จ
   ตอนติดตั้ง โปรแกรมจะเพิ่มคำสั่งให้แต่ละเหตุการณ์ลงใน `~/.claude/settings.json`
   คำสั่งนั้นคือ `ai-mascot-hook --set <สถานะ>` เขียนคำบอกสถานะหนึ่งคำลงไฟล์ `state` แล้วจบทันที
2. **แมวน้ำ (ผู้อ่านโน้ต)** โปรแกรมเดียวกัน ถ้าเปิดโดยไม่ใส่อะไรต่อท้าย จะเป็นแมวน้ำบนหน้าจอ
   มันเปิดไฟล์ `state` ดูทุก 400 มิลลิวินาที เจอคำใหม่ก็เปลี่ยนท่าตามนั้น
   เปิดได้ทีละตัวเท่านั้น

| เหตุการณ์ของ Claude Code | สถานะ | แมวน้ำทำอะไร |
|---|---|---|
| เริ่ม/จบ session (`SessionStart`, `SessionEnd`) | `idle` | นอนหลับ |
| ส่งข้อความ (`UserPromptSubmit`) | `loading` | ตบมือ กำลังคิด |
| ก่อน/หลังใช้ tool (`PreToolUse`, `PostToolUse`) | `working` | ว่ายน้ำใส่หมวกนิรภัย |
| ขออนุญาต หรือรอคำตอบ (`PermissionRequest`, `Notification`) | `asking` | โบกมือเรียกคุณ |
| ตอบเสร็จ (`Stop`) | `completed` | นอนหลับ เสร็จแล้ว |

Hook ทำงานเฉพาะ Claude Code ที่รันบนเครื่องนี้ และโปรแกรมไม่ส่งข้อมูลอะไรออกทางเน็ตเลย

## ติดตั้งบน Windows

ต้องใช้ Windows 10 ขึ้นไปแบบ 64 บิต ตอนนี้ยังไม่มีไฟล์สำเร็จรูปให้ดาวน์โหลด
จึงต้อง build เอง (ประมาณ 1 นาที) ไฟล์ exe ที่ได้ไม่ต้องติดตั้งโปรแกรมอื่นเพิ่ม
Claude Code ไม่บังคับ (ถ้าไม่มี แมวน้ำก็ยังรันได้ และสั่งเองด้วย `--set` ได้)

1. **Build** ติดตั้ง Rust จาก <https://rustup.rs> (ข้ามได้ถ้าสั่ง `cargo --version` ได้อยู่แล้ว)
   แล้วเปิด PowerShell ในโฟลเดอร์โปรเจกต์ และรัน

   ```powershell
   cargo build --release
   ```

   จากนั้น copy `target\release\ai-mascot-hook.exe` ไปวางในโฟลเดอร์ที่จะเก็บถาวร
   และ **path ต้องไม่มีช่องว่าง** เช่น `C:\Tools\ai-mascot-hook\ai-mascot-hook.exe`
   ไม่ควรวางไว้ใน Downloads เพราะ hook จะเรียกไฟล์จาก path นี้ตลอด ถ้าย้ายทีหลัง hook จะใช้ไม่ได้
2. **เปิดแมวน้ำ** ดับเบิลคลิก `ai-mascot-hook.exe` แมวน้ำจะขึ้นบนหน้าจอ
3. **ติดตั้ง hook** เปิด PowerShell แล้วรัน

   ```powershell
   C:\Tools\ai-mascot-hook\ai-mascot-hook.exe --install-hooks
   ```

   โปรแกรมจะสำรอง `~\.claude\settings.json` ไว้เป็น `settings.json.bak-<เวลา>` ก่อน
   แล้วเพิ่ม hook ตามตารางด้านบน โดยเก็บ hook และค่าอื่นๆ ที่คุณมีอยู่ไว้ครบ
   (ถ้ามี repo อยู่ในเครื่อง ใช้
   `powershell -ExecutionPolicy Bypass -File tools\install-hooks.ps1 -Exe <path>` แทนได้ ผลเหมือนกัน)
4. **เปิด Claude Code ใหม่** ปิดแล้วเปิดใหม่ (หรือเปิด session ใหม่ใน Claude desktop app)
   **session ที่เปิดไว้ก่อนติดตั้ง hook จะไม่เรียก hook** ลองส่งข้อความหา Claude
   แมวน้ำจะเปลี่ยนเป็นท่าตบมือ (loading) ทันที
5. **ให้เปิดเองตอนเข้า Windows (ไม่บังคับ)** กด `Win + R` พิมพ์ `shell:startup` แล้วกด Enter
   จากนั้นสร้าง shortcut ของ `ai-mascot-hook.exe` ไว้ในโฟลเดอร์ที่เปิดขึ้นมา

Claude desktop app ใช้ได้เช่นกัน แต่เป็นแอปแบบ MSIX ซึ่ง Windows จะย้ายไฟล์ที่ hook เขียนไปไว้ใน
`%LOCALAPPDATA%\Packages\<แอป>\LocalCache\Local\ai-mascot-hook\state`
แมวน้ำอ่านไฟล์ตรงนั้นให้ด้วย และตามไฟล์ที่ถูกเขียนล่าสุด

## ติดตั้งบน macOS

ต้องใช้ macOS 11 ขึ้นไป (ชิป Apple หรือ Intel) ตอนนี้ยังไม่มีไฟล์สำเร็จรูปให้ดาวน์โหลดสำหรับ Mac
จึงต้อง build เอง (ประมาณ 1 นาที) แมวน้ำไม่มีไอคอนใน Dock

1. **ติดตั้ง Rust** จาก <https://rustup.rs> (ข้ามได้ถ้าสั่ง `cargo --version` ได้อยู่แล้ว)
2. **Build** เปิด Terminal ในโฟลเดอร์โปรเจกต์ แล้วรัน

   ```bash
   cargo build --release
   ```

3. **ย้ายโปรแกรมไปไว้ที่ถาวร** เพราะ hook เรียกโปรแกรมตาม path

   ```bash
   mkdir -p ~/bin && cp target/release/ai-mascot-hook ~/bin/
   ```

4. **ติดตั้ง hook** (รันจากไฟล์ที่ copy ไปไว้)

   ```bash
   ~/bin/ai-mascot-hook --install-hooks
   ```

5. **เปิดแมวน้ำ**

   ```bash
   ~/bin/ai-mascot-hook &
   ```

6. **เปิด Claude Code ใหม่** แล้วลองส่งข้อความ แมวน้ำจะเปลี่ยนเป็นท่าตบมือ
7. **ให้เปิดเองตอน login (ไม่บังคับ)** สร้างไฟล์ `~/Library/LaunchAgents/com.ai-mascot-hook.plist`
   (แก้ path ให้ตรงกับเครื่องคุณ) แล้วรัน
   `launchctl load ~/Library/LaunchAgents/com.ai-mascot-hook.plist`

   ```xml
   <?xml version="1.0" encoding="UTF-8"?>
   <!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
   <plist version="1.0">
   <dict>
     <key>Label</key><string>com.ai-mascot-hook</string>
     <key>ProgramArguments</key><array><string>/Users/you/bin/ai-mascot-hook</string></array>
     <key>RunAtLoad</key><true/>
   </dict>
   </plist>
   ```

ส่วนที่เหลือใช้งานเหมือน Windows: การลาก การปรับขนาด เมนูปุ่มกลม หน้าตั้งค่าข้อความ
และกล่องข้อความ (ปุ่ม **+** ในหน้าตั้งค่าจะเปิดหน้าเลือกสีของ macOS)

## วิธีใช้งาน

### จัดการตัวแมวน้ำ

- **กดค้างแล้วลาก** (หรือกดแล้วขยับ): ย้ายตำแหน่งแมวน้ำ
- **คลิก** (หรือคลิกขวา): เปิดเมนูปุ่มกลมสามปุ่ม
  1. ลูกศร (**Resize**): ปรับขนาด หมุนล้อเมาส์บนตัวแมวน้ำ หรือลากมุมขวาล่าง
     ได้ตั้งแต่ 32 ถึง 512 px คลิกที่แมวน้ำเมื่อเสร็จ
  2. เฟือง (**Settings**): เปิดแถวที่สอง
     - **Show status / Hide status**: เปิดหรือปิดกล่องข้อความสถานะ
     - **Edit status**: เปิดหน้าตั้งค่าข้อความ
  3. กากบาท (**Close**): ปิดโปรแกรม
- กด Esc เพื่อปิดเมนู
- ตำแหน่ง ขนาด และการเปิด/ปิดกล่องข้อความจะถูกจำไว้ เปิดครั้งหน้าจะอยู่ที่เดิม

### กล่องข้อความสถานะ

- ค่อยๆ ออกมาจากตัวแมวน้ำทีละส่วน วงกลมเล็ก วงกลมใหญ่ แล้วกล่องข้อความ
- ลอยขึ้นลงเบาๆ ตอนแสดงอยู่ เด้งหนึ่งครั้งเมื่อสถานะเปลี่ยน และค่อยๆ จางหายตอนซ่อน
- ตอน Claude กำลังคิด ทำงาน หรือรอคำตอบ จะมีวงแหวนกระจายออกจากจุดสี
  ตอนรอคำตอบ (asking) จะกระจายถี่กว่า

### หน้าตั้งค่าข้อความ (Edit status)

| ส่วน | ใช้ทำอะไร |
|---|---|
| **MESSAGES** | ข้อความของแต่ละสถานะ ยาวได้ไม่เกิน 60 ตัวอักษร |
| ชื่อสถานะ (Thinking, Working, …) | คลิกเพื่อเลือกและ preview แมวน้ำจะเล่นท่าของสถานะนั้น และกล่องข้อความจะแสดงตัวอย่าง |
| **TEXT COLOR** | สีตัวอักษรของสถานะที่เลือกอยู่ แต่ละสถานะมีสีของตัวเองได้ กด **+** เพื่อเลือกสีอื่นนอกจาก 8 สี |
| **Show status** | เปิด/ปิดกล่องข้อความ (เหมือนปุ่มในเมนู) |
| **Auto hide** | ซ่อนกล่องข้อความเองหลังสถานะเปลี่ยน ตั้งได้ 1–20 วินาทีด้วยปุ่ม − / + |
| **Reset** | คืนข้อความและสีทั้งหมดเป็นค่าเริ่มต้น (ยังไม่บันทึกจนกว่าจะกด Save) |
| **Save** / **Cancel** | บันทึก / ยกเลิก |

**วิธีขึ้นบรรทัดใหม่:** พิมพ์ `\n` ตรงที่ต้องการตัดบรรทัด เช่น

```text
กำลังทำงาน\nรอแป๊บนะ
```

กล่องข้อความจะแสดงเป็นสองบรรทัด

คีย์ลัด: **Enter** = Save, **Esc** = Cancel, **Tab** = ไปช่องถัดไป
ลากย้ายหน้าตั้งค่าได้โดยจับที่หัวหน้าต่าง

### สั่งสถานะเอง

ใช้ทดสอบ หรือเชื่อมกับโปรแกรมอื่นได้

```bash
ai-mascot-hook --set working
```

สถานะที่ใช้ได้: `loading`, `working`, `asking`, `completed`, `idle`

## การเก็บ setting

ทุกอย่างอยู่ในโฟลเดอร์เดียว ไม่มีการเก็บที่อื่น (ไม่ใช้ registry ไม่ส่งขึ้น cloud)

| ระบบ | โฟลเดอร์ |
|---|---|
| Windows | `%LOCALAPPDATA%\ai-mascot-hook\` |
| macOS | `~/Library/Application Support/ai-mascot-hook/` |

| ไฟล์ | เก็บอะไร | ใครเขียน |
|---|---|---|
| `state` | สถานะปัจจุบัน (`working` ฯลฯ) | hook (คำสั่ง `--set`) |
| `placement` | ตำแหน่ง ขนาด และการเปิด/ปิดกล่องข้อความ | แมวน้ำ เมื่อคุณย้าย ปรับขนาด หรือสลับกล่องข้อความ |
| `status.txt` | ข้อความ สีของแต่ละสถานะ และเวลา auto hide | ปุ่ม **Save** ในหน้าตั้งค่าข้อความ |

`status.txt` เป็นข้อความธรรมดา รูปแบบเดียวกันทั้งสองระบบ คัดลอกข้าม Windows กับ Mac ได้
ลบไฟล์ไหนทิ้ง ค่านั้นจะกลับเป็นค่าเริ่มต้น
ส่วน hook เองอยู่ใน `~/.claude/settings.json` ของ Claude Code

## ถอนการติดตั้ง

1. ปิดแมวน้ำ: คลิกที่ตัวแมวน้ำ แล้วกดกากบาท (**Close**)
2. ลบ hook ออกจาก Claude Code (สำรอง `settings.json` ให้ก่อน)
   - Windows: `C:\Tools\ai-mascot-hook\ai-mascot-hook.exe --remove-hooks`
   - macOS: `~/bin/ai-mascot-hook --remove-hooks`
3. ลบส่วนที่เหลือ
   - Windows: shortcut ใน `shell:startup` (ถ้ามี) ไฟล์ exe และโฟลเดอร์ `%LOCALAPPDATA%\ai-mascot-hook`
   - macOS: `launchctl unload ~/Library/LaunchAgents/com.ai-mascot-hook.plist` แล้วลบไฟล์นั้น
     (ถ้าตั้งให้เปิดตอน login ไว้) ลบตัวโปรแกรม และรัน
     `rm -rf ~/Library/Application\ Support/ai-mascot-hook`

## แก้ปัญหา

**แมวน้ำไม่เปลี่ยนท่าตาม Claude**

- เปิด Claude Code หรือ session ใหม่หรือยัง session ที่เปิดก่อนติดตั้ง hook จะไม่เรียก hook
- ลองสั่ง `ai-mascot-hook --set asking` ถ้าแมวน้ำเปลี่ยนท่า แปลว่าตัวโปรแกรมทำงานปกติ ปัญหาอยู่ที่ hook
  ถ้าย้ายไฟล์โปรแกรม ให้รัน `--install-hooks` จากที่ใหม่อีกครั้ง

**ใช้ Claude desktop app แล้วไม่เปลี่ยน**

ใช้ `ai-mascot-hook.exe` ตัวล่าสุดแล้วเปิด session ใหม่ (ดูเรื่อง MSIX ในหัวข้อ
[ติดตั้งบน Windows](#ติดตั้งบน-windows))

**อัปเกรดมาจากชื่อเดิม `seal-pet`**

รัน `--install-hooks` หนึ่งครั้ง โปรแกรมจะแทนที่ hook `seal-pet` เดิมด้วยของใหม่
ส่วน setting ตอนนี้อยู่ในโฟลเดอร์ `ai-mascot-hook` ถ้าอยากเก็บค่าเดิมไว้
ให้คัดลอก `status.txt` และ `placement` จากโฟลเดอร์ `seal-pet` เดิมมาใส่

**build ไม่ผ่าน: `Access is denied` ตอน build (Windows)**

Windows ล็อกไฟล์ exe ที่กำลังรันอยู่ ปิดแมวน้ำก่อน build ใหม่

**ไม่เห็นกล่องข้อความ**

- เปิด **Show status** ในเมนู Settings หรือในหน้าตั้งค่า
- ถ้าเปิด **Auto hide** ไว้ กล่องจะซ่อนเองหลังครบเวลา และขึ้นใหม่เมื่อสถานะเปลี่ยน
- ตอนเปิดเมนูของแมวน้ำ กล่องข้อความจะซ่อนชั่วคราว

## Build เอง

ต้องมี Rust (stable) แล้วรัน `cargo build --release` ได้ไฟล์ `target\release\ai-mascot-hook.exe` (Windows)
หรือ `target/release/ai-mascot-hook` (macOS)

รายละเอียดการทำงานทั้งหมดดูได้ที่ [SPEC.md](SPEC.md)
และแผนภาพการเชื่อมต่อของโค้ดดูได้ที่ [docs/th/architecture.md](docs/th/architecture.md)
ส่วนเรื่อง token, เวลาที่ hook ใช้, CPU และหน่วยความจำ ดูได้ที่ [docs/th/technical.md](docs/th/technical.md)
