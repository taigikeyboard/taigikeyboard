# Desktop changelog fragments

A pull request that changes what a desktop user sees (macOS, Windows, Linux,
ChromeOS) adds one file here. The monthly release assembles them into
`changelog/desktop-v<version>.md`, the body of the GitHub release, and deletes
them:

```bash
python3 tools/release_notes.py assemble --train desktop --version 2026.12.0
python3 tools/release_notes.py assemble --train desktop --version 2026.12.1 --platform linux   # patch
```

Nobody edits the assembled file afterwards, so the fragment is the release
note.

## Format

`changelog/unreleased/<short-slug>.md`:

```markdown
---
platforms: [macos, windows, linux]
section: Candidates
---
- **Whole-input candidates come first.** A candidate covering everything you
  typed now comes before ones covering only the first syllables: `taiuanl`
  gives 台灣人 first.
```

- `platforms` — any of `macos`, `windows`, `linux`, `chromeos`, as a `[…]`
  list. The bullet appears under each one's `###` heading.
- `section` — one of `Input`, `Phonetic Symbols`, `Candidates`,
  `Candidate Window`, `Settings`, `Appearance`, `Updates`, `Install`.
- Body — one bullet: a bold short claim, then what a user sees. Continuation
  lines are indented two spaces. Writing rules: `.claude/skills/release-desktop/SKILL.md`
  § 4 (English, UI labels by their i18n `en` value, no refactors or tooling).
- The pull request number is added when the release is assembled, from the
  merge that added the file. Write `(#123)` at the end yourself only for a
  fragment that lands without its own pull request, or that covers several.
- Wording differs per platform (`⌃⌘H` on macOS, `Ctrl+Alt+H` elsewhere) →
  one fragment per wording.

## When none is needed

CI (`.github/workflows/changelog.yml`) fails a pull request that changes
desktop-shipping files (`DESKTOP_SHIPPING_PREFIXES` in
`tools/release_notes.py`) without a fragment. Label it `no-changelog` when no
desktop user can see the change: a refactor, a test, a mobile-only engine
change. Dependabot pull requests are exempt.
