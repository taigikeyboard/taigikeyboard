---
name: release-helper
description: Prepare a release on main, rebuild generated artifacts, update the detailed changelog, create concise English iOS and Android What's New text, mirror it into both mobile apps' version history, validate it, commit, and push. Use when preparing a version for manual App Store Connect or Google Play release. Never tags, uploads builds, or submits a store release. Mobile train only (iOS + Android share one version); the desktop train (macOS + Windows) has its own version, its own `changelog/desktop-v<version>.md`, and its own publish scripts, none of which this skill touches.
---

# Release Helper

Run from `main`. Require `<base-tag> <target>` in semantic-version form.
Example: `/release-helper v3.6.4 v3.6.5`

Never create or move a tag, upload a build, edit store metadata, or submit a release. The user handles every store action manually.

## 1. Validate context

- Require a clean worktree and `HEAD = main`.
- Run `git fetch --prune --tags --force` and `git pull --ff-only origin main`.
- Require `<base-tag>` to exist and be an ancestor of `HEAD`.
- Require `<target>` to match `vMAJOR.MINOR.PATCH`.
- Run `python3 tools/release_notes.py check-versions --train mobile --version <target>`.
- Report open PRs and ask before continuing if any exist.
- Use `<base-tag>..HEAD` as the release range. State it before edits.

The skill may report a required version change but must not edit the user-owned iOS `.pbxproj`. When `check-versions` fails because the tree still carries the previous version, give the user the one command that sets both mobile platforms — `make version-mobile <target without the v>` — and continue after they run it. The desktop train (macOS + Windows) is numbered separately and is not this skill's concern.

## 2. Analyze the release

Read the complete release range:

```bash
git log <base-tag>..HEAD --oneline
git diff <base-tag>..HEAD --stat --name-status
git log <base-tag>..HEAD --format="%s%n%b%n---"
```

Trace user-visible behavior by platform. Do not infer release scope from commit subjects alone.

Sort every user-visible change into one of three buckets before writing anything:

- **iOS-only / Android-only** → that platform's store notes, plus its detailed-changelog section.
- **Shared** (engine, dictionary, behavior landing on both mobile platforms) → both store notes, plus the shared detailed-changelog section. Describe it by what a phone user sees, not by the platforms it landed on.
- **macOS-only / Windows-only** → NOT this release. Desktop work is recorded in the desktop train's own `changelog/desktop-v<version>.md` when a desktop release is cut — see § Desktop train. Never either store note.

Run the `upgrade-check` procedure for `<base-tag> → HEAD`:

- `BLOCKED` → stop before rebuild or edits.
- `CLEAN WITH BEHAVIOR CHANGES` → include each behavior change in the detailed changelog and affected platform notes.
- `CLEAN` → continue.

## 3. Rebuild release artifacts

Run sequentially and stop on failure:

```bash
RELEASE_VERSION=<target> make dict
make i18n
make build
```

## 4. Update release content

Update these surfaces idempotently:

| File | Purpose |
| --- | --- |
| `CHANGELOG.md` | Link to the detailed version changelog |
| `changelog/<target>.md` | Detailed Shared / iOS / Android / Dictionary record for the mobile train (desktop work lives in `changelog/desktop-v<version>.md`, not here) |
| `changelog/store/<target>/ios.txt` | Canonical English iOS What's New |
| `changelog/store/<target>/android.txt` | Canonical English Android What's New |
| iOS `VersionHistory.swift` | Generated from `ios.txt` |
| Android `VersionHistory.kt` | Generated from `android.txt` |

If target content already exists, merge new information by topic. Refine an existing line for the same behavior; do not duplicate a topic or add a second target history entry.

Canonical store-note rules:

- English prose; Taigi terms and examples may retain 漢字 / TL / POJ / TPS.
- One to five non-empty entries without source bullet markers.
- Start each entry with `New:`, `Fixed:`, `Improved:`, `Changed:`, or `Updated:`.
- End each entry with punctuation.
- Include only concrete user-visible changes.
- Exclude refactors, tests, tooling, dependencies, issue/PR numbers, URLs, rankings, marketing claims, and future work.
- Do not mention another platform in platform-specific notes: no Android in `ios.txt`, no App Store in `android.txt`, and no macOS in either. `validate_notes` rejects all three.
- Keep rendered bullets and newlines within 500 Unicode characters.
- A platform's app history and store text must use exactly the same entries.

Synchronize both apps after the canonical files are final:

```bash
python3 tools/release_notes.py sync --version <target> --date YYYY/MM/DD
```

## 5. Validate and render

```bash
python3 tools/release_notes.py check --version <target>
python3 tools/release_notes.py check-versions --train mobile --version <target>
python3 tools/release_notes_test.py
git diff --check
```

Render and review both manual-paste values:

```bash
python3 tools/release_notes.py print --version <target> --platform ios
python3 tools/release_notes.py print --version <target> --platform android
```

Passing deterministic validation does not replace the user's factual review.

## 6. Commit and hand off

Commit and push `main`. Do not tag.

```bash
git add -A
git commit -m "<target>: release prep + store notes"
git push origin main
```

Report the commit SHA and both rendered texts. Give the user these clipboard commands:

```bash
python3 tools/release_notes.py print --version <target> --platform ios | pbcopy
python3 tools/release_notes.py print --version <target> --platform android | pbcopy
```

The user pastes iOS text into App Store Connect and Android text into the chosen Google Play release, then manually selects builds and submits releases.

## Desktop train

macOS and Windows are the **desktop train**: one version number shared by the two, moved by `make version-desktop x.y.z`, independent of the mobile number this skill prepares. Each desktop platform announces itself through its own GitHub release on the website repository — `macos/scripts/publish-release.sh` extracts the `### macOS` section and `windows/scripts/publish-release.sh` the `### Windows` section of `changelog/desktop-v<version>.md` as the release body, falling back to a one-line `TaigiKeyboard for <platform> <version>` note when the section is missing. This skill does not write that file, run those scripts, or publish anything for desktop.

What that means while preparing a mobile release:

- Keep macOS and Windows out of `changelog/<target>.md` entirely: that file is the mobile record, and a desktop change in it describes work its readers cannot install. Desktop work waits for its own `changelog/desktop-v<version>.md`.
- Keep macOS out of `ios.txt` and `android.txt`. `validate_notes` forbids the whole words `macOS` and `Mac` in both, so a leak fails `check` rather than reaching a store listing.
- `check-versions --train mobile` verifies iOS + Android only. A macOS or Windows version that differs from `<target>` is expected, not a finding.

## Guardrails

- Never edit another version's changelog or store-note files.
- Never hand-edit the generated target history entry.
- Never create, move, or push a tag.
- Never request, store, or use signing certificates, keystores, API keys, or store credentials.
- Never upload a build, edit a live store listing, or submit production.
- Never run the macOS release or publish scripts, and never put macOS-only work in a store note.
- Surface the first actionable rebuild or validation failure and stop before commit.
