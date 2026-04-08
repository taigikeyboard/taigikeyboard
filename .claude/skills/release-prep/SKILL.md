---
name: release-prep
description: Update changelog/<version>.md + CHANGELOG.md index + Tab1 version history based on commits since main (last release). Run on develop branch when preparing a release.
disable-model-invocation: true
---

# Release Prep

Prepare the current release by updating CHANGELOG.md and version history. Run on `develop` branch, comparing against `main` (last release) to find all new commits.

## Steps

### 1. Gather branch context

Run these in parallel:
- `git log main..HEAD --oneline` — list all commits on the branch
- `git diff main..HEAD --stat` — file-level change summary
- `git diff main..HEAD --name-status` — file changes with A/M/D/R status
- `git log main..HEAD --format="%s%n%b%n---"` — full commit messages
- `git status --short` — verify clean working tree
- Read `CHANGELOG.md` (index file) to find the current version's changelog path, then read that version file (e.g., `changelog/v3.4.8.md`)

If the working tree is not clean (unstaged or uncommitted changes), ask the user to commit or stash first.

### 2. Update CHANGELOG.md and versionHistoryEntries

Compare the current version's changelog file (e.g., `changelog/v3.4.8.md`) against the full diff to find missing entries. Add any changes not already documented. If this is a new version with no existing file, create a new file in `changelog/` and add it to the index in `CHANGELOG.md`.

**Categorization rules** (this is a cross-platform mobile project):
- **iOS** — changes under `ios/`
- **Android** — changes under `android/`
- **Dictionary** — changes under `dictionary/`
- **Shared** — changes affecting both platforms, `docs/`, `CLAUDE.md`, root-level files

**Sub-categories per platform**:
- New Features, Bug Fixes, Refactoring, Removed, Changes, New Files

Keep the existing CHANGELOG style and formatting. Only add missing items — do not rewrite entries that are already correct.

**Also update `versionHistoryEntries` on both platforms**:

- **iOS** — `ios/Sources/TaigiKeyboard/Localization/Tab1Texts.swift`
  - Format: `("x.y.z", "YYYY/MM/DD", [LocalizedText(hanji: "..."), ...])`
- **Android** — `android/app/src/main/java/com/siansiansu/taigikeyboard/localization/Tab1Texts.kt`
  - Format: `VersionEntry("x.y.z", "YYYY/MM/DD", listOf(LocalizedText(hanji = "..."), ...))`

Common rules for both:
- Each change is a `LocalizedText(hanji:)` / `LocalizedText(hanji =)` entry
- Insert at the top of the list (newest version first)
- iOS and Android entries may differ — only include changes relevant to each platform
- **User-facing tone**: Write as if explaining to a non-technical user. Keep each entry to one short sentence
- **Include**: new features, UI changes, bug fixes that users would notice, new keyboard layouts, new dictionary sources
- **Exclude**: refactoring, code cleanup, doc changes, test changes, internal architecture changes, developer tooling
- **Group related changes**: combine small related fixes into one entry (e.g., "Fixed several display issues" instead of listing each one)
- **Examples of good entries**:
  - "Added an English keyboard."
  - "Fixed an issue where candidates did not appear when typing tone 1 or 4."
  - "Updated the app logo."
- **Examples of entries to skip**:
  - "Refactored the codebase for better cleanliness and maintainability." (internal)
  - "Upgraded KeyboardKit to v10." (dependency detail, unless it brings user-visible changes)

### 3. Commit changelog updates

Stage and commit only the changelog-related files:

```
git add CHANGELOG.md changelog/<version>.md ios/Sources/TaigiKeyboard/Localization/Tab1Texts.swift android/app/src/main/java/com/siansiansu/taigikeyboard/localization/Tab1Texts.kt
git commit -m "<version>: Update changelog and version history"
```

### 4. Verify

1. `git log main..HEAD --oneline` — confirm the changelog commit is included
2. `git status --short` — confirm clean working tree

## Important

- Do NOT modify source code files other than the two `Tab1Texts` files — only `CHANGELOG.md` (index), `changelog/<version>.md`, iOS/Android Tab1Texts
- Do NOT alter any functionality
- If the changelog version file is already complete, inform the user — nothing to do
- For new releases: create `changelog/<version>.md` and add it to the `CHANGELOG.md` index
