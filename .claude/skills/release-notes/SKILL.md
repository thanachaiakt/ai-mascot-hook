---
name: release-notes
description: Write bilingual (English, then Thai) release notes for a version tag from the git history. Use when the user asks for release notes, a changelog, or "what changed since the last tag".
---

Follow the "Release notes for a tag" section of `docs/en/commit-convention.md`.

1. Take the target tag from the user, or from `Cargo.toml`'s version as `vX.Y.Z`. Find the
   previous tag with `git describe --tags --abbrev=0 HEAD^` (or the last `v*` tag).
2. Read `git log <previous>..HEAD --no-merges --format="%h %s%n%b"`. If a subject is vague,
   read its diff with `git show <hash> --stat` before describing it.
3. Group by Conventional Commit type: Breaking changes, Features, Fixes, Performance, Build
   and installer, Docs. Drop `chore`; drop `refactor` and `test` unless users feel them.
4. Rewrite each entry in user words. Name the platform when only one is affected. Commit
   bodies carry an English and a Thai block; reuse the Thai wording for the Thai section.
5. Output the template from the doc: English section, `---`, then Thai section with the same
   items in the same order. Omit empty sections. Fill the compare link from `git remote get-url origin`.
6. Every line must trace to a commit. Do not invent changes or numbers.
7. Print the notes. Write a file or run `gh release create/edit` only if the user asks.
