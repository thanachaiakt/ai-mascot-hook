# Commit and release-note convention

Commits follow [Conventional Commits 1.0](https://www.conventionalcommits.org/). Release notes
are built from them, so the commit subject is what users read on the GitHub release page.

## Commit message format

```
<type>(<scope>): <subject>

<body in English>

<body in Thai>

<footer>
```

The subject is English only. The body is written twice: English first, then the same content
in Thai after a blank line. The Thai part is a faithful translation, not a summary, and keeps
code names, flags, file names and numbers unchanged. Release notes reuse these two parts.

### Subject

- Imperative mood, present tense: "add", not "added" or "adds".
- 72 characters or fewer, no trailing period, lower-case after the colon.
- Say what the change does for the user or the code, not which file moved.

### Type

| Type | Use for | Shown in release notes |
|------|---------|------------------------|
| `feat` | New user-visible behavior (new state, agent, menu item, setting) | Features |
| `fix` | Bug fix | Fixes |
| `perf` | Faster or lighter, same behavior | Performance |
| `refactor` | Code change with no behavior change | Internal |
| `docs` | README, SPEC, `docs/` only | Docs |
| `test` | Tests only | Internal |
| `build` | `Cargo.toml`, `build.rs`, dependencies, installers | Build |
| `ci` | `.github/workflows` | Build |
| `chore` | Version bump, housekeeping | Hidden |
| `revert` | Undo an earlier commit | Fixes |

### Scope (optional)

Use the area the change touches: `win`, `mac`, `gfx`, `seal`, `bubble`, `menu`, `editor`,
`status`, `store`, `hooks`, `installer`, `release`, `docs`. For a new AI agent use the
agent name: `feat(hooks): add Kiro agent`. Omit the scope when a change spans many areas.

### Body (required except for trivial commits)

Every commit except a version bump or a one-word typo fix has a bilingual body. Wrap at 72
characters (Thai has no spaces between words, so wrap at a phrase break). Explain **why**,
then anything a reviewer cannot see in the diff: the old behavior, the new behavior,
platform differences, measured numbers. Use a bulleted list for several separate points.
Write the English block first, a blank line, then the Thai block with the same points in the
same order.

### Footer

- Breaking change: `BREAKING CHANGE: <what breaks and how to migrate>`, or add `!` after
  the type: `feat(hooks)!: rename --agent values`.
- Issues: `Closes #12`, `Refs #30`.
- Attribution lines the tooling adds (such as `Co-Authored-By`) go last.

### Rules specific to this repo

- A change to behavior updates `SPEC.md`, `README.md` and `README.th.md` in the same commit.
- Mention the platform when only one is affected: `fix(mac): keep menu on top of the Dock`.
- Version bump commit: `chore: bump version to 0.1.5`. Keep `Cargo.toml` and `Cargo.lock` in one commit. No body needed.
- One logical change per commit. Do not mix `feat` and `refactor`.

### Examples

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

## Release notes for a tag

Tag format is `vMAJOR.MINOR.PATCH` and must equal the version in `Cargo.toml`
(see `.github/workflows/release.yml`). Pre-1.0: a `feat` raises MINOR, a `fix` raises PATCH,
and a breaking change also raises MINOR.

### How to build the notes

1. Find the previous tag: `git describe --tags --abbrev=0 HEAD^` (or the last `v*` tag).
2. List commits: `git log <previous>..HEAD --no-merges --format="%h %s%n%b"`.
3. Group by type using the table above. Drop `chore` and `refactor`/`test` unless they matter to users.
4. Rewrite each entry in user language: what the user can now do or no longer suffers.
   Do not paste raw subjects such as "refactor menu struct".
5. Put breaking changes first and say how to migrate.
6. Write English first, then Thai. Both carry the same items in the same order.

### Release note template

```markdown
## ai-mascot-hook vX.Y.Z

One or two sentences: the headline change of this release.

### Breaking changes
- <what changed>. Migrate by <step>.

### Features
- <feature in user words> (#issue)

### Fixes
- <fix in user words>; affects <Windows|macOS|both>

### Performance
- <measured gain>

### Build and installer
- <change>

### Docs
- <change>

### Upgrade
Run the installer again. Settings (`status.txt`, placement) are kept and hooks are refreshed.

**Full changelog**: https://github.com/<owner>/ai-mascot-hook/compare/vPREV...vX.Y.Z

---

## ไทย: ai-mascot-hook vX.Y.Z

<สรุปการเปลี่ยนแปลงหลักของรุ่นนี้ 1-2 ประโยค>

### การเปลี่ยนแปลงที่ไม่เข้ากับรุ่นเดิม
- <สิ่งที่เปลี่ยน> วิธีปรับ: <ขั้นตอน>

### ฟีเจอร์ใหม่
- <ฟีเจอร์ ด้วยภาษาผู้ใช้> (#issue)

### แก้ไขข้อผิดพลาด
- <สิ่งที่แก้>; มีผลกับ <Windows|macOS|ทั้งสอง>

### ประสิทธิภาพ
- <ผลที่วัดได้>

### Build และตัวติดตั้ง
- <การเปลี่ยนแปลง>

### เอกสาร
- <การเปลี่ยนแปลง>

### การอัปเดต
รันตัวติดตั้งอีกครั้ง การตั้งค่า (`status.txt`, ตำแหน่ง) จะยังอยู่ และ hook จะถูกรีเฟรช

**รายการเปลี่ยนแปลงทั้งหมด**: https://github.com/<owner>/ai-mascot-hook/compare/vPREV...vX.Y.Z
```

Leave out any section with no items. Keep each bullet to one line when possible.
