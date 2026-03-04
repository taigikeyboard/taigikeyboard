---
name: squash-commit
description: Update CHANGELOG.md + Tab1 version history, then squash all branch commits into a single commit with a structured message. Use when the user wants to squash commits before merge.
disable-model-invocation: true
---

# Release Prep

Prepare the current branch for merge by updating CHANGELOG.md and squashing all commits into one.

## Steps

### 1. Gather branch context

Run these in parallel:
- `git log main..HEAD --oneline` — list all commits on the branch
- `git diff main..HEAD --stat` — file-level change summary
- `git diff main..HEAD --name-status` — file changes with A/M/D/R status
- `git log main..HEAD --format="%s%n%b%n---"` — full commit messages
- `git status --short` — verify clean working tree
- Read the current `CHANGELOG.md`

If the working tree is not clean, warn the user and stop.

### 2. Update CHANGELOG.md and versionHistoryEntries

Compare the existing CHANGELOG.md against the full diff to find missing entries. Add any changes not already documented.

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

### 3. Squash commits

1. Find merge base: `git merge-base main HEAD`
2. Stage updated files: `git add CHANGELOG.md ios/Sources/TaigiKeyboard/Localization/Tab1Texts.swift android/app/src/main/java/com/siansiansu/taigikeyboard/localization/Tab1Texts.kt`
3. Soft reset: `git reset --soft <merge-base>`
4. Create a single commit with a structured message:

```
<version-tag>: <concise summary>

### iOS
- bullet points of iOS changes

### Android
- bullet points of Android changes

### Dictionary
- bullet points of dictionary changes (if any)

### Shared
- bullet points of shared changes (if any)

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
```

Extract the version tag from the branch name or CHANGELOG header (e.g., `v3.4.1` from `bugfix-v3.4.1`).

### 4. Verify and push

1. `git log main..HEAD --oneline` — confirm exactly 1 commit
2. `git status --short` — confirm clean working tree
3. Ask the user for confirmation, then: `git push --force-with-lease origin <branch>`

## Important

- Do NOT modify source code files other than the two `Tab1Texts` files — only CHANGELOG.md, iOS/Android Tab1Texts, and commit history
- Do NOT alter any functionality
- If the CHANGELOG.md is already complete, skip to step 3
- Always use `--force-with-lease` (not `--force`) for safety
