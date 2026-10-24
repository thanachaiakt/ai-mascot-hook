# มาตรฐาน commit และ release note

Commit ใช้ [Conventional Commits 1.0](https://www.conventionalcommits.org/) และ release note
สร้างจาก commit เหล่านี้ ดังนั้นหัวข้อ (subject) ของ commit คือสิ่งที่ผู้ใช้อ่านในหน้า GitHub release
ข้อความ commit และ release note เขียนเป็นภาษาอังกฤษ ส่วน release note มีภาษาไทยต่อท้าย

## รูปแบบข้อความ commit

```
<type>(<scope>): <subject>

<body ภาษาอังกฤษ>

<body ภาษาไทย>

<footer>
```

Subject เป็นภาษาอังกฤษอย่างเดียว ส่วน body เขียนสองรอบ: อังกฤษก่อน แล้วเว้นบรรทัดและตามด้วยภาษาไทย
เนื้อหาเดียวกัน ภาษาไทยต้องแปลให้ครบ ไม่ใช่สรุป และคงชื่อโค้ด, flag, ชื่อไฟล์, ตัวเลขไว้ตามเดิม
release note นำสองส่วนนี้ไปใช้ต่อ

### Subject

- ใช้ประโยคคำสั่งรูปปัจจุบัน: "add" ไม่ใช่ "added" หรือ "adds"
- ไม่เกิน 72 ตัวอักษร ไม่ลงท้ายด้วยจุด ตัวพิมพ์เล็กหลังเครื่องหมาย colon
- บอกว่าการเปลี่ยนแปลงทำอะไรให้ผู้ใช้หรือโค้ด ไม่ใช่บอกว่าย้ายไฟล์ไหน

### Type

| Type | ใช้กับ | แสดงใน release note |
|------|--------|---------------------|
| `feat` | พฤติกรรมใหม่ที่ผู้ใช้เห็น (state, agent, เมนู, การตั้งค่าใหม่) | ฟีเจอร์ใหม่ |
| `fix` | แก้บั๊ก | แก้ไขข้อผิดพลาด |
| `perf` | เร็วขึ้นหรือเบาลง พฤติกรรมเดิม | ประสิทธิภาพ |
| `refactor` | แก้โค้ดโดยพฤติกรรมไม่เปลี่ยน | ภายใน |
| `docs` | README, SPEC, `docs/` เท่านั้น | เอกสาร |
| `test` | เฉพาะ test | ภายใน |
| `build` | `Cargo.toml`, `build.rs`, dependency, ตัวติดตั้ง | Build |
| `ci` | `.github/workflows` | Build |
| `chore` | เพิ่มเลขรุ่น งานจุกจิก | ไม่แสดง |
| `revert` | ย้อน commit ก่อนหน้า | แก้ไขข้อผิดพลาด |

### Scope (ไม่บังคับ)

ใช้ชื่อส่วนที่แก้: `win`, `mac`, `gfx`, `seal`, `bubble`, `menu`, `editor`, `status`, `store`,
`hooks`, `installer`, `release`, `docs` ถ้าเพิ่ม AI agent ใหม่ใช้ชื่อ agent เช่น
`feat(hooks): add Kiro agent` ถ้าแก้ครอบคลุมหลายส่วนให้ละ scope

### Body (บังคับ ยกเว้น commit เล็กมาก)

ทุก commit ต้องมี body สองภาษา ยกเว้น commit เพิ่มเลขรุ่นหรือแก้คำผิดคำเดียว ตัดบรรทัดที่ 72 ตัวอักษร
(ภาษาไทยไม่มีช่องว่างระหว่างคำ ให้ตัดตรงช่วงวลี) อธิบาย **เหตุผล** ก่อน แล้วตามด้วยสิ่งที่ดูจาก diff ไม่ออก:
พฤติกรรมเก่า พฤติกรรมใหม่ ความต่างระหว่างแพลตฟอร์ม ตัวเลขที่วัดได้ ถ้ามีหลายประเด็นแยกกันให้ใช้ bullet
เขียนบล็อกอังกฤษก่อน เว้นหนึ่งบรรทัด แล้วบล็อกไทยที่มีประเด็นเดียวกันและเรียงเหมือนกัน

### Footer

- Breaking change: `BREAKING CHANGE: <อะไรพัง และต้องปรับอย่างไร>` หรือใส่ `!` หลัง type
  เช่น `feat(hooks)!: rename --agent values`
- Issue: `Closes #12`, `Refs #30`
- บรรทัด attribution ที่เครื่องมือเติมให้ (เช่น `Co-Authored-By`) อยู่ท้ายสุด

### กฎเฉพาะของ repo นี้

- เปลี่ยนพฤติกรรมต้องแก้ `SPEC.md`, `README.md` และ `README.th.md` ใน commit เดียวกัน
- ถ้ากระทบแค่แพลตฟอร์มเดียว ให้ระบุ: `fix(mac): keep menu on top of the Dock`
- commit เพิ่มเลขรุ่น: `chore: bump version to 0.1.5` และแก้ `Cargo.toml` กับ `Cargo.lock` ใน commit เดียว ไม่ต้องมี body
- หนึ่ง commit ต่อหนึ่งการเปลี่ยนแปลงเชิงตรรกะ ห้ามปน `feat` กับ `refactor`

### ตัวอย่าง

```
feat(hooks): add Kiro agent

Kiro reads hook files from .kiro/hooks. Add its Plan to hooks.rs, a box in
installer/windows.iss and the Thai installer file list.

Kiro อ่านไฟล์ hook จาก .kiro/hooks เพิ่ม Plan ของ Kiro ใน hooks.rs
เพิ่มช่องเลือกใน installer/windows.iss และในรายการไฟล์ของตัวติดตั้งภาษาไทย

Closes #21
```

```
fix(win): stop the bubble flickering when the seal moves

The bubble canvas was redrawn before the new position was applied, so one frame
showed at the old spot. Apply placement first, then draw.

bubble ถูกวาดใหม่ก่อนที่ตำแหน่งใหม่จะถูกใช้ ทำให้มีหนึ่งเฟรมโผล่ที่ตำแหน่งเก่า
แก้โดยใช้ตำแหน่งใหม่ก่อน แล้วค่อยวาด
```

```
perf(gfx): unpack only the frame on screen

Memory drops from 48 MB to 21 MB idle.

หน่วยความจำตอนว่างลดจาก 48 MB เหลือ 21 MB
```

## Release note สำหรับ tag

รูปแบบ tag คือ `vMAJOR.MINOR.PATCH` และต้องตรงกับเลขรุ่นใน `Cargo.toml`
(ดู `.github/workflows/release.yml`) ก่อนรุ่น 1.0: `feat` เพิ่ม MINOR, `fix` เพิ่ม PATCH
และ breaking change เพิ่ม MINOR

### วิธีสร้าง release note

1. หา tag ก่อนหน้า: `git describe --tags --abbrev=0 HEAD^` (หรือ tag `v*` ล่าสุด)
2. ดึงรายการ commit: `git log <previous>..HEAD --no-merges --format="%h %s%n%b"`
3. จัดกลุ่มตาม type ในตารางด้านบน ตัด `chore` ทิ้ง และตัด `refactor`/`test` ถ้าไม่กระทบผู้ใช้
4. เขียนใหม่เป็นภาษาผู้ใช้: ผู้ใช้ทำอะไรได้เพิ่ม หรือเลิกเจอปัญหาอะไร
   ห้ามคัดลอก subject ดิบ ๆ เช่น "refactor menu struct"
5. ใส่ breaking change ไว้ก่อน พร้อมบอกวิธีปรับ
6. เขียนอังกฤษก่อน แล้วตามด้วยไทย รายการเหมือนกันและเรียงเหมือนกัน

### แม่แบบ release note

ใช้แม่แบบเดียวกับ [ฉบับภาษาอังกฤษ](../en/commit-convention.md#release-note-template)
ซึ่งมีทั้งส่วนอังกฤษและไทยในบล็อกเดียว หัวข้อที่ไม่มีรายการให้ตัดทิ้ง และเขียนแต่ละ bullet ให้จบในบรรทัดเดียวถ้าทำได้
