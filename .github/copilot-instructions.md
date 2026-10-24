# Copilot instructions for ai-mascot-hook

Read `CLAUDE.md` for the project layout, commands and conventions; it applies to Copilot too.

## Commits

Follow `.github/instructions/commit-message.instructions.md` (Conventional Commits). Full
rules and examples: `docs/en/commit-convention.md`, Thai in `docs/th/commit-convention.md`.

## Release notes

When asked for release notes of a tag (`vX.Y.Z`, which must match `Cargo.toml`'s version):

1. Find the previous `v*` tag and list commits with `git log <previous>..HEAD --no-merges`.
2. Group them: Breaking changes, Features, Fixes, Performance, Build and installer, Docs.
   Drop `chore`, and drop `refactor` and `test` unless users feel them.
3. Rewrite each entry in user words (what they can now do, what no longer breaks) and name
   the platform when only Windows or only macOS is affected.
4. Write English first, then Thai, with the same items in the same order. Use the template in
   `docs/en/commit-convention.md`. Omit empty sections.
5. Do not invent changes. Every line must trace to a commit.
