Write commit messages following Conventional Commits 1.0. The subject is English; the body is English first, then the same content in Thai. Full rules: docs/en/commit-convention.md (Thai: docs/th/commit-convention.md).

Format:

    <type>(<scope>): <subject>

    <body in English>

    <body in Thai>

    <footer>

- type: feat, fix, perf, refactor, docs, test, build, ci, chore, revert.
- scope (optional): win, mac, gfx, seal, bubble, menu, editor, status, store, hooks, installer, release, docs, or an agent name. Omit it when the change spans many areas.
- subject: imperative present tense ("add", not "added"), 72 characters or fewer, lower-case, no trailing period. Describe what the change does for the user or the code, not which file moved.
- body (required except for a version bump or a one-word typo fix): English block, blank line, then a Thai block with the same points in the same order. The Thai block is a faithful translation, not a summary. Keep code names, flags, file names and numbers unchanged. Wrap at 72 characters (at a phrase break in Thai). Explain why, then old versus new behavior, platform differences and measured numbers.
- footer: `BREAKING CHANGE: <what breaks and how to migrate>` (or `!` after the type), `Closes #N`, then attribution lines last.
- One logical change per commit. Do not mix feat with refactor.
- Name the platform when only one is affected, e.g. `fix(mac): ...`.
- Version bump: `chore: bump version to X.Y.Z`, no body.
- Do not invent issue numbers or measurements. Describe only what the diff shows.
