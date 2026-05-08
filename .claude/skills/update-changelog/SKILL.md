---
name: update-changelog
description: Release prep on main — diff against a user-given base tag, rebuild dictionary + Rust engine, update changelog + version history, then tag the target version. Args: <base-tag> <target-version>.
disable-model-invocation: false
---

# Release Prep

Run from `main`. User supplies `<base-tag> <target-version>`.
Example: `/update-changelog v3.5.0 v3.5.7`

## 1. Sanity (abort on first failure)

- `git rev-parse --abbrev-ref HEAD` must be `main`
- `git status --short` must be empty (ask user to commit/stash otherwise)
- `git fetch --prune` then `git pull --ff-only origin main`
- `git rev-parse <base-tag>` must succeed
- `git rev-parse <target> 2>/dev/null` must FAIL — abort if the tag already exists

## 2. Pre-release housekeeping

- `gh pr list --state open --json number,title` — if non-empty, show to user and ask whether to proceed
- Delete local branches already merged into main:
  `git branch --merged main | grep -v '^[* ] main$' | xargs -n1 -r git branch -d`

## 3. Diff context (parallel)

- `git log <base-tag>..HEAD --oneline`
- `git diff <base-tag>..HEAD --stat`
- `git diff <base-tag>..HEAD --name-status`
- `git log <base-tag>..HEAD --format="%s%n%b%n---"`
- Read `CHANGELOG.md` and existing `changelog/<target>.md` if present

## 4. Rebuild dictionary + Rust engine

Run sequentially. Abort the skill on any failure:
- `make dict`
- `make build`

`git status --short` afterwards shows the artifact diff that must ride in the release commit.

## 5. Update changelog (4 files)

**`CHANGELOG.md`** — flat index, newest first. Add `- [<target>](changelog/<target>.md)` at the top.

**`changelog/<target>.md`** — create or fill in. Categorize by:
- **iOS** (`ios/`), **Android** (`android/`), **Dictionary** (`dictionary/`), **Shared** (`docs/`, `CLAUDE.md`, root-level)
- Sub-sections: New Features, Bug Fixes, Refactoring, Removed, Changes, New Files
- Dedup against entries already present — never duplicate

**iOS `versionHistoryEntries`** — `ios/Sources/TaigiKeyboard/Strings/HomeTexts.swift`
- `("x.y.z", "YYYY/MM/DD", [LocalizedText(hanji: "…"), …])`

**Android `versionHistoryEntries`** — `android/app/src/main/java/com/siansiansu/taigikeyboard/localization/HomeTexts.kt`
- `VersionEntry("x.y.z", "YYYY/MM/DD", listOf(LocalizedText(hanji = "…"), …))`

Both `versionHistoryEntries`:
- Insert at the top (newest first)
- One short user-facing sentence per entry, no jargon
- **Include**: user-visible features, UI changes, bug fixes users would notice, new layouts, new dictionary sources
- **Exclude**: refactors, internal cleanups, docs, tests, developer tooling, silent dependency bumps
- Group related small fixes into a single entry
- iOS and Android lists may differ — only list what each platform exposes

## 6. Commit + tag + push

```
git add -A
git commit -m "<target>: changelog + dict/engine rebuild"
git tag <target>
git push origin main <target>
```

`git add -A` is safe because step 1 verified the tree was clean — every staged change came from steps 4–5.

## Important

- Hand-edit ONLY the 4 changelog files. Build artifacts go in via `git add -A`.
- NEVER overwrite a previous version's changelog file; only fill in `<target>.md`.
- If steps 4 + 5 produce no diff, tell the user — nothing to release.
- If any rebuild step fails, surface the error and abort before committing.
