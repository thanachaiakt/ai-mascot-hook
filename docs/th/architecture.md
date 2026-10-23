# โครงสร้างและการเชื่อมต่อของโค้ด

[English](../en/architecture.md)

Seal Pet เป็นโปรแกรม Rust ไฟล์เดียว ทำสองหน้าที่:

- **คำสั่งสำหรับ hook:** `seal-pet --set <สถานะ>` เขียนคำหนึ่งคำลงไฟล์แล้วจบทันที
- **ตัวแมวน้ำ:** ถ้ารันโดยไม่ใส่ argument จะเปิดหน้าต่าง คอยดูไฟล์นั้น แล้วเล่นท่าทางและแสดงกล่องข้อความตามสถานะ

โค้ดส่วนใหญ่ใช้ร่วมกันทั้งสองระบบ มีแค่ส่วนจัดการหน้าต่างที่เขียนแยกสำหรับ Windows และ macOS

## 1. จาก Claude Code ถึงหน้าจอ

```mermaid
flowchart LR
    CC["Claude Code<br/>(CLI หรือ desktop app)"] -- "เหตุการณ์ hook<br/>UserPromptSubmit, PreToolUse, Stop, …" --> H["seal-pet --set &lt;สถานะ&gt;"]
    H -- "status::write_state<br/>เขียนไฟล์ชั่วคราวแล้ว rename" --> F[("ไฟล์ state<br/>%LOCALAPPDATA%\seal-pet\state<br/>~/Library/Application Support/seal-pet/state")]
    F -- "อ่านทุก 400 ms<br/>store::read_state" --> S["แมวน้ำที่รันอยู่<br/>win::App / mac::App"]
    S -- "seal::clip_for" --> A["ภาพเคลื่อนไหว"]
    S -- "bubble::Bubble" --> B["กล่องข้อความสถานะ"]
    A --> W["หน้าต่างแมวน้ำ"]
    B --> BW["หน้าต่างกล่องข้อความ"]
```

| เหตุการณ์ของ Claude Code | สถานะ | ท่าทาง |
|---|---|---|
| `SessionStart`, `SessionEnd` | `idle` | นอนหลับ |
| `UserPromptSubmit` | `loading` | ตบมือ |
| `PreToolUse`, `PostToolUse` | `working` | ว่ายน้ำใส่หมวกนิรภัย |
| `PermissionRequest`, `Notification` | `asking` | โบกมือบนก้อนน้ำแข็ง |
| `Stop` | `completed` | นอนหลับ |

Claude desktop app เป็นแอปแบบ MSIX Windows จะย้ายไฟล์ state ที่ hook ของแอปนี้เขียน ไปไว้ที่
`%LOCALAPPDATA%\Packages\<แอป>\LocalCache\Local\seal-pet\state`
`store::state_files` หาไฟล์ที่ถูกย้ายพวกนี้ด้วย และ `store::read_state` จะใช้ไฟล์ที่เขียนล่าสุด

## 2. แผนผัง module

```mermaid
flowchart TB
    main["main.rs<br/>จุดเริ่มต้น"]

    subgraph shared["ใช้ร่วมกัน"]
        seal["seal.rs<br/>ขนาด เวลา ท่าของแต่ละสถานะ"]
        store["store.rs<br/>folder ข้อมูล ไฟล์ state ตำแหน่ง"]
        status["status.rs<br/>ข้อความ สี auto hide<br/>อ่านไฟล์ state"]
        bubble["bubble.rs<br/>จัดวาง วาด และ animate กล่องข้อความ"]
        menu["menu.rs<br/>จัดวาง วาด และ animate เมนูปุ่มกลม"]
        editor["editor.rs<br/>จัดวาง วาด และรับคลิกหน้าตั้งค่า"]
        hooks["hooks.rs<br/>~/.claude/settings.json"]
        anim["anim.rs<br/>เส้นโค้งความเร็ว"]
        gfx["gfx/mod.rs<br/>Canvas รูปทรง Strip"]
    end

    subgraph backends["gfx แยกตามระบบ"]
        gwin["gfx/win.rs<br/>DIB + ตัวอักษร GDI"]
        gmac["gfx/mac.rs<br/>CGBitmapContext + ตัวอักษร AppKit"]
    end

    subgraph win["Windows: win/"]
        wmod["win/mod.rs<br/>App หน้าต่างแมวน้ำ เมาส์<br/>timer และ message loop"]
        wmenu["win/menu.rs<br/>หน้าต่างเมนูปุ่มกลม"]
        wed["win/editor.rs<br/>หน้าต่างตั้งค่า ช่อง EDIT<br/>หน้าเลือกสี"]
    end

    subgraph mac["macOS: mac/"]
        mmod["mac/mod.rs<br/>App, NSPanel, SealView,<br/>NSTimer"]
        mmenu["mac/menu.rs<br/>panel เมนูปุ่มกลม"]
        med["mac/editor.rs<br/>panel ตั้งค่า NSTextField<br/>NSColorPanel"]
    end

    main --> hooks
    main --> status
    main --> store
    main -- "windows" --> wmod
    main -- "macos" --> mmod

    wmod --> wmenu
    wmod --> wed
    mmod --> mmenu
    mmod --> med
    wmenu & mmenu --> menu
    wed & med --> editor
    wmod & mmod --> bubble
    wmod & mmod --> seal
    wmod & mmod --> store
    wmod & mmod --> status
    wmod & mmod --> gfx

    bubble --> anim
    bubble --> gfx
    menu & editor --> gfx
    editor --> status
    bubble --> seal
    store --> seal
    store --> status
    seal --> gfx
    status --> gfx
    gfx -. "cfg(windows)" .-> gwin
    gfx -. "cfg(macos)" .-> gmac
```

- **โค้ดที่ใช้ร่วมกัน** ไม่เรียกระบบปฏิบัติการตรงๆ ยกเว้น `gfx` ซึ่งเลือกส่วนของแต่ละระบบตอน compile
- **แอปฝั่ง `win` และ `mac`** ทำงานเหมือนกันและเรียงลำดับเดียวกัน ที่ต้องเขียนแยกเพราะแต่ละระบบใช้หน้าต่าง event และ timer ของตัวเอง

## 3. ตอนเริ่มโปรแกรม

```mermaid
flowchart TD
    start(["seal-pet"]) --> args{"argument แรก"}
    args -- "--set สถานะ" --> write["status::write_state"] --> exit0(["จบ"])
    args -- "--install-hooks / --remove-hooks" --> hk["hooks::run<br/>สำรองไฟล์ ลบ hook เก่าของ Seal Pet<br/>เพิ่ม hook 8 ตัว"] --> exit1(["จบ"])
    args -- "ไม่มี" --> plat{"ระบบ"}
    plat -- Windows --> wrun["win::run"]
    plat -- macOS --> mrun["mac::run"]
    wrun & mrun --> one{"มีตัวอื่นรันอยู่แล้ว?<br/>(mutex / socket)"}
    one -- ใช่ --> quit(["จบ"])
    one -- ไม่ --> init["สร้างหน้าต่าง<br/>read_state, read_placement,<br/>StatusText::load"]
    init --> loop["event loop + timer"]
```

## 4. ระหว่างแมวน้ำทำงาน

ทั้งสองแอปเก็บ `App` ไว้ตัวเดียวในตัวแปรของ thread และเข้าถึงผ่าน `with(|app| ...)`
timer และ event ของเมาส์จะเรียกเข้ามาที่ตัวนี้:

```mermaid
flowchart LR
    subgraph sources["เวลาและการกด"]
        t1["timer เปลี่ยนภาพ<br/>(ตามท่า 83 ms)"]
        t2["timer อ่านไฟล์<br/>400 ms"]
        t3["timer กล่องข้อความ<br/>15 ms ตอนขยับ / 33 ms ตอนลอย"]
        t4["timer auto hide"]
        t5["timer กดค้าง<br/>220 ms"]
        mouse["เมาส์:<br/>กด ขยับ ปล่อย ล้อ<br/>คลิกขวา"]
    end

    t1 --> fr["ภาพถัดไป<br/>draw_mascot"]
    t2 --> poll["poll<br/>read_state"]
    poll -- "สถานะเปลี่ยน" --> sync["sync_clip<br/>(โหลดท่าใหม่)"] --> rah["restart_autohide"]
    rah --> ub["update_bubble"]
    t3 --> bf["bubble_frame"]
    t4 --> hide["auto_hidden = true"] --> ub
    t5 --> drag["เริ่มลาก"]
    mouse --> press["press_down / press_move / press_up"]
    press -- "กดค้าง หรือขยับเกิน 4 px" --> drag --> mv["move_to + บันทึกตำแหน่ง"]
    press -- "คลิก" --> menu["เมนูปุ่มกลม"]
    press -- "โหมดปรับขนาด" --> rs["set_height + บันทึก"]
    menu --> act["ปรับขนาด / แสดงสถานะ /<br/>แก้ข้อความ / ปิด"]
```

บน Windows แต่ละอย่างเป็น timer ของ Win32 (`TIMER_FRAME`, `TIMER_POLL`, `TIMER_BUBBLE`,
`TIMER_AUTOHIDE`, `TIMER_HOLD` และ `TIMER_MENU` สำหรับ animation ของเมนู)
บน macOS ใช้ `NSTimer` ตัวเดียวที่ 60 ครั้งต่อวินาที เรียก `App::tick` แล้วเช็คว่าถึงเวลาทำอะไรบ้าง

## 5. การวาดแมวน้ำ

```mermaid
flowchart LR
    png["assets/*.png<br/>แถบภาพ (PNG แบบ palette)"] -- "build.rs<br/>PNG ทีละเฟรม" --> frames["$OUT_DIR/*.frames<br/>(ฝังใน exe)"]
    frames -- "Strip::decode" --> strip["Strip<br/>เฟรมที่ยังบีบอัดอยู่"]
    strip -- "Strip::zoomed<br/>(working ขยาย 1.15 เท่า ขยายกรอบตาม)" --> strip2["Strip + ระยะที่ขยาย"]
    strip2 -- "Strip::draw<br/>ถอดหนึ่งเฟรม ย่อขยายเท่าขนาดแมวน้ำ" --> mc["Canvas mc"]
    mc -- "โหมดปรับขนาด:<br/>กรอบ + ปุ่มมุม" --> mc
    mc -- "Windows: UpdateLayeredWindow<br/>macOS: CGImage บน layer" --> win["หน้าต่างแมวน้ำ<br/>(กรอบ + พื้นที่โปร่งใสรอบนอก)"]
```

**กรอบ** ของแมวน้ำ (x, y, ความสูง) คือส่วนที่ถูกบันทึก ถูกลาก ถูกปรับขนาด และใช้วาง bubble
สำหรับท่า `working` ที่ขยาย **หน้าต่าง** จะใหญ่เกินกรอบออกไป (`overhang`)
ตำแหน่งเมาส์จะถูกแปลงกลับมาเทียบกับกรอบเสมอ (`in_box`)

## 6. กล่องข้อความสถานะ

`bubble::Bubble` ทำทุกอย่าง ยกเว้นการเอาภาพขึ้นจอ:

```mermaid
flowchart TD
    app["App::update_bubble<br/>เลือกข้อความ จุดสี สีตัวอักษร key<br/>(preview ในหน้าตั้งค่า คำแนะนำตอนปรับขนาด หรือสถานะ)"]
    app -- "แสดง" --> show["Bubble::show<br/>วัดข้อความ วางเหนือหรือใต้แมวน้ำ<br/>ไม่ให้ล้นจอ หลบป้ายบอกขนาด"]
    app -- "ซ่อน" --> hidebox["Bubble::hide"]
    show --> drawn["drawn: กล่องแบบ pixel art สติกเกอร์<br/>+ จุดสีแบบเรียบ + ข้อความ"]
    show --> tails["tails: วงกลมหางสองวง<br/>แต่ละวงมี canvas ของตัวเอง"]
    frame["App::bubble_frame (timer)"] --> bfr["Bubble::frame"]
    hidebox --> bfr
    bfr --> comp["shown = drawn + tails รวมกัน<br/>พร้อมเด้งออก เด้ง ลอย วงแหวน"]
    comp -- "Frame::Shown {x, y, alpha, moving}" --> present["แต่ละระบบเอา canvas 'shown' ขึ้นจอ<br/>แล้วตั้งเวลาภาพถัดไป"]
    bfr -- "Frame::Hidden" --> off["แต่ละระบบซ่อนหน้าต่าง<br/>หยุด timer"]
```

ค่าที่ใช้คุม animation:

- `appear` (0 ถึง 1): เด้งออกภายใน 0.5 วินาที จางหายภายใน 0.18 วินาที
- `pop`: การเด้งตอน `key` เปลี่ยน
- นาฬิกา: ใช้กับการลอยรอบละ 2.8 วินาที และวงแหวนจากจุดสี

เส้นโค้งความเร็วมาจาก `anim.rs`

## 7. ไฟล์ที่เกี่ยวข้อง

```mermaid
flowchart LR
    hooks2["hooks::run"] -- "แก้ไข" --> settings[("~/.claude/settings.json")]
    setcmd["seal-pet --set"] -- "เขียน" --> state[("state")]
    state -- "อ่าน" --> app2["App::poll"]
    app2 -- "save_placement" --> placement[("placement<br/>x y ความสูง show_status")]
    placement -- "read_placement ตอนเริ่ม" --> app2
    editor2["หน้าตั้งค่า กด Save"] -- "StatusText::save" --> statustxt[("status.txt<br/>ข้อความ color.&lt;สถานะ&gt; autohide")]
    statustxt -- "StatusText::load" --> app2
```

ทุกไฟล์ยกเว้น `settings.json` อยู่ใน folder ข้อมูล (`store::data_dir`)
คือ `%LOCALAPPDATA%\seal-pet` บน Windows และ `~/Library/Application Support/seal-pet` บน macOS

## จะแก้อะไร ดูที่ไหน

| อยากแก้ | ดูที่ |
|---|---|
| เหตุการณ์ไหนตั้งสถานะอะไร | `hooks.rs` (`PLAN`), `tools/install-hooks.ps1` |
| ท่า ความเร็ว หรือขนาดของแต่ละสถานะ | `seal.rs` (`Clip`, `clip_for`) |
| หน้าตาหรือการเคลื่อนไหวของกล่องข้อความ | `bubble.rs` |
| ข้อความและสีเริ่มต้น | `status.rs` (`STATES`, `DEFAULT_COLOR`) |
| หน้าตาเมนูปุ่มกลม หรือหน้าตั้งค่า | `menu.rs` / `editor.rs` |
| หน้าต่างเมนู / หน้าตั้งค่าบน Windows | `win/menu.rs` / `win/editor.rs` |
| หน้าต่างและการกดบน Mac | `mac/mod.rs`, `mac/menu.rs`, `mac/editor.rs` |
| การวาดตัวอักษร หรือเอาภาพขึ้นจอ | `gfx/win.rs`, `gfx/mac.rs` |
