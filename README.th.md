# Seal Pet (ภาษาไทย)

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

## สิ่งที่ต้องมี

- Windows 10 หรือใหม่กว่า แบบ 64 บิต หรือ macOS 11 ขึ้นไป (ชิป Apple หรือ Intel)
  สำหรับ Mac ดูหัวข้อ [ใช้บน Mac](#ใช้บน-mac) ด้านล่าง
- ไฟล์ `seal-pet.exe` จากหน้า Releases ไม่ต้องติดตั้งโปรแกรมอื่นเพิ่ม
- Claude Code (ไม่บังคับ) ถ้าต้องการให้แมวน้ำเปลี่ยนท่าตาม Claude อัตโนมัติ
  ใช้ได้ทั้ง Claude Code แบบ command line และใน Claude desktop app
- ถ้าจะ build เอง ต้องมี Rust (stable)

## ขั้นตอนติดตั้ง

### 1. ดาวน์โหลดหรือ build โปรแกรม

**แบบดาวน์โหลด:** ดาวน์โหลด `seal-pet.exe` จาก
[GitHub Releases](https://github.com/thanachaiakt/ai-mascot-status/releases)
แล้ววางไว้ในโฟลเดอร์ที่จะเก็บถาวร เช่น `C:\Tools\seal-pet\seal-pet.exe`
ไม่ควรวางไว้ใน Downloads เพราะ hook จะเรียกไฟล์จาก path นี้ตลอด

**แบบ build เอง:**

```powershell
cargo build --release
```

ได้ไฟล์ที่ `target\release\seal-pet.exe`

### 2. เปิดแมวน้ำ

ดับเบิลคลิก `seal-pet.exe` แมวน้ำจะขึ้นที่มุมขวาล่างของจอ
เปิดได้ทีละตัวเท่านั้น ถ้าเปิดซ้ำจะไม่มีตัวที่สองขึ้นมา

### 3. เชื่อมกับ Claude Code (ติดตั้ง hook)

เปิด PowerShell ในโฟลเดอร์โปรเจกต์ แล้วรัน

```powershell
powershell -ExecutionPolicy Bypass -File tools\install-hooks.ps1
```

ถ้า `seal-pet.exe` ไม่ได้อยู่ที่ `target\release\` ให้บอก path ด้วย `-Exe`

```powershell
powershell -ExecutionPolicy Bypass -File tools\install-hooks.ps1 -Exe C:\Tools\seal-pet\seal-pet.exe
```

สคริปต์จะ:

- สำรอง `~\.claude\settings.json` ไว้เป็น `settings.json.bak-<เวลา>` ก่อน
- เพิ่ม hook ที่สั่ง `seal-pet.exe --set <สถานะ>` ตามเหตุการณ์ของ Claude Code
- เก็บ hook และค่าอื่นๆ ที่คุณมีอยู่ไว้ครบ

| เหตุการณ์ของ Claude Code | สถานะของแมวน้ำ |
|---|---|
| เริ่ม/จบ session | `idle` |
| ส่งข้อความ | `loading` |
| ก่อน/หลังใช้ tool | `working` |
| ขออนุญาต หรือรอคำตอบ | `asking` |
| ตอบเสร็จ | `completed` |

### 4. เริ่ม Claude Code ใหม่

ปิดแล้วเปิด Claude Code ใหม่ (หรือเปิด session ใหม่ใน Claude desktop app)
**session ที่เปิดไว้ก่อนติดตั้ง hook จะไม่เรียก hook** ต้องเปิดใหม่เท่านั้น

ลองส่งข้อความหา Claude แมวน้ำจะเปลี่ยนเป็นท่าตบมือ (loading) ทันที

### 5. ให้เปิดเองตอนเข้า Windows (ไม่บังคับ)

กด `Win + R` พิมพ์ `shell:startup` แล้วกด Enter
จากนั้นสร้าง shortcut ของ `seal-pet.exe` ไว้ในโฟลเดอร์ที่เปิดขึ้นมา

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

```powershell
seal-pet.exe --set working
```

สถานะที่ใช้ได้: `loading`, `working`, `asking`, `completed`, `idle`

## ใช้บน Mac

เวอร์ชัน Mac ใช้งานได้เหมือน Windows: แมวน้ำ ท่าทางตามสถานะ กล่องข้อความพร้อม animation
การลาก การปรับขนาด เมนูปุ่มกลม หน้าตั้งค่าข้อความ และ hook ของ Claude Code
ปุ่ม **+** ในหน้าตั้งค่าจะเปิดหน้าเลือกสีของ macOS
การตั้งค่าเก็บไว้ใน `~/Library/Application Support/seal-pet/` (`status.txt` รูปแบบเดียวกับ
Windows คัดลอกไฟล์จาก Windows มาใช้ได้)

### ติดตั้งบน Mac

ต้องมี Rust ก่อน (ติดตั้งจาก https://rustup.rs) แล้วเปิด Terminal ในโฟลเดอร์โปรเจกต์

```bash
cargo build --release
./target/release/seal-pet --install-hooks
./target/release/seal-pet &
```

จากนั้นเปิด Claude Code ใหม่ แมวน้ำไม่มีไอคอนใน Dock ปิดได้จากปุ่ม **Close** ในเมนูของแมวน้ำ

ถ้าย้ายไฟล์ `seal-pet` ไปไว้ที่อื่น ให้รัน `--install-hooks` จากที่ใหม่อีกครั้ง เพราะ hook เรียกโปรแกรมตาม path

### ให้เปิดเองตอน login (ไม่บังคับ)

สร้างไฟล์ `~/Library/LaunchAgents/com.seal-pet.plist` (แก้ path ให้ตรงกับเครื่องคุณ)

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key><string>com.seal-pet</string>
  <key>ProgramArguments</key><array><string>/Users/you/seal-pet/target/release/seal-pet</string></array>
  <key>RunAtLoad</key><true/>
</dict>
</plist>
```

แล้วรัน `launchctl load ~/Library/LaunchAgents/com.seal-pet.plist`

### ถอนการติดตั้งบน Mac

```bash
./target/release/seal-pet --remove-hooks
launchctl unload ~/Library/LaunchAgents/com.seal-pet.plist   # ถ้าตั้งให้เปิดตอน login ไว้
rm -rf ~/Library/Application\ Support/seal-pet
```

## ไฟล์ที่โปรแกรมเก็บ

ทั้งหมดอยู่ใน `%LOCALAPPDATA%\seal-pet\` (Windows) หรือ `~/Library/Application Support/seal-pet/` (Mac)

| ไฟล์ | เก็บอะไร |
|---|---|
| `state` | สถานะปัจจุบัน (hook เป็นคนเขียน) |
| `placement` | ตำแหน่ง ขนาด และการเปิด/ปิดกล่องข้อความ |
| `status.txt` | ข้อความ สีของแต่ละสถานะ และเวลา auto hide |

## ถอนการติดตั้ง

1. ปิดแมวน้ำ: คลิกที่ตัวแมวน้ำ แล้วกด **Close**
2. ลบ hook ออกจาก Claude Code:

   ```powershell
   powershell -ExecutionPolicy Bypass -File tools\install-hooks.ps1 -Remove
   ```

3. ลบ `seal-pet.exe` shortcut ใน `shell:startup` (ถ้ามี) และโฟลเดอร์ `%LOCALAPPDATA%\seal-pet`

## แก้ปัญหา

**แมวน้ำไม่เปลี่ยนท่าตาม Claude**

- เปิด Claude Code หรือ session ใหม่หรือยัง session ที่เปิดก่อนติดตั้ง hook จะไม่เรียก hook
- ดูว่า hook ชี้ไปที่ `seal-pet.exe` ถูกที่ไหม ถ้าย้ายไฟล์ exe ให้รัน `install-hooks.ps1 -Exe <path ใหม่>` อีกครั้ง
- ลองสั่ง `seal-pet.exe --set asking` ถ้าแมวน้ำเปลี่ยนท่า แปลว่าตัวโปรแกรมทำงานปกติ ปัญหาอยู่ที่ hook

**ใช้ Claude desktop app แล้วไม่เปลี่ยน**

Claude desktop app เป็นแอปแบบ MSIX Windows จะย้ายไฟล์ที่ hook เขียนไปไว้ใน
`%LOCALAPPDATA%\Packages\<แอป>\LocalCache\Local\seal-pet\state`
Seal Pet เวอร์ชันนี้อ่านไฟล์ตรงนั้นให้แล้ว ถ้ายังไม่เปลี่ยน ให้ใช้ `seal-pet.exe` ตัวล่าสุด
แล้วเปิด session ใหม่

**build ไม่ผ่าน: `Access is denied` ตอน build**

Windows ล็อกไฟล์ exe ที่กำลังรันอยู่ ปิดแมวน้ำก่อน build ใหม่

**ไม่เห็นกล่องข้อความ**

- เปิด **Show status** ในเมนู Settings หรือในหน้าตั้งค่า
- ถ้าเปิด **Auto hide** ไว้ กล่องจะซ่อนเองหลังครบเวลา และขึ้นใหม่เมื่อสถานะเปลี่ยน
- ตอนเปิดเมนูของแมวน้ำ กล่องข้อความจะซ่อนชั่วคราว

รายละเอียดการทำงานทั้งหมดดูได้ที่ [SPEC.md](SPEC.md)
และแผนภาพการเชื่อมต่อของโค้ดดูได้ที่ [docs/th/architecture.md](docs/th/architecture.md)
ส่วนเรื่อง token, เวลาที่ hook ใช้, CPU และหน่วยความจำ ดูได้ที่ [docs/th/technical.md](docs/th/technical.md)
