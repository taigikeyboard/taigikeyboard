---
name: release-mobile
description: Prepare a MOBILE release (iOS + Android, one shared version) on main - rebuild generated artifacts, update the detailed changelog, write concise English iOS and Android What's New text, mirror it into both apps' version history, validate, commit, and push. Use when preparing a version for manual App Store Connect or Google Play release. Never tags, uploads builds, or submits a store release. Takes no version argument - it releases the version already in the tree (set beforehand with `make version-mobile x.y.z`); an optional argument only overrides the release base. Mobile train only; the desktop train (macOS + Windows) is `release-desktop`.
---

# Release Mobile

The mobile train is iOS + Android, sharing one version. The desktop train is a
separate skill, `release-desktop`; a release never mixes the two.

Run from `main`. Takes no version: **`<target>` is whatever version the tree
already carries** — the maintainer sets it with `make version-mobile x.y.z`
before invoking. Example: `/release-mobile`

One optional argument, `<base-tag>`, overrides the derived release base.
Example: `/release-mobile v3.6.4`

Never create or move a tag, upload a build, edit store metadata, or submit a release. The user handles every store action manually.

## 1. Validate context

- Require a clean worktree and `HEAD = main`.
- Run `git fetch --prune --tags --force` and `git pull --ff-only origin main`.

Derive `<target>` from the tree, never from an argument:

```bash
grep -m1 'versionName = ' android/app/build.gradle.kts    # 3.6.6 -> target v3.6.6
python3 tools/release_notes.py check-versions --train mobile --version <target>
```

That check is what proves the iOS `MARKETING_VERSION` carries the same number,
so a half-applied bump stops here instead of shipping. The skill must not edit
the user-owned iOS `.pbxproj`: when the two disagree, hand the user the one
command that sets both — `make version-mobile <target without the v>` — and
continue after they run it. The desktop train is numbered separately and is not
this skill's concern.

Derive the release base unless one was passed:

```bash
git describe --tags --abbrev=0 --match 'v*' --match 'mobile-*' HEAD
```

Older mobile tags are the bare `v<version>` form, newer ones `mobile-<version>`.
When the derived base does not exist or is not an ancestor of `HEAD`, stop and
ask for one rather than guessing a commit.

Then, before any edit:

- Require `<target>` to match `vMAJOR.MINOR.PATCH`, and
  `changelog/store/<target>/` to be absent or unreleased — never re-prepare a
  version already tagged.
- Report open PRs and ask before continuing if any exist.
- **State the derived version and range — `preparing mobile <target>, range
  <base>..HEAD, N commits` — and wait for the user to confirm.** Nothing is
  derived silently, because nothing was passed in.

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

**The skill runs these itself.** `CLAUDE.md` § Build & Test reserves builds for
the user mid-round; a release is a named exception, like post-PR verification —
the whole point of this step is that the artifact being shipped was built from
the commit being released, and asking the user to remember it is what the
conditional version already got wrong.

**Always**, not only when the range touched them. The engine binaries iOS and
Android link are generated and gitignored, so a clean tree says nothing about
which commit the local copies came from; a machine that last built on another
branch would ship that.

```bash
make i18n
make build
```

`make dict` is the exception: run it only when the range touched `dictionary/`
sources. Its outputs are committed and rebuild byte-differently every run, so an
unconditional pass would put noise in the release commit.

```bash
RELEASE_VERSION=<target> make dict   # only if dictionary/ sources moved
```

Stop on the first failure, and commit whatever the rebuild changed.

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
- One to eight non-empty entries without source bullet markers. The 500-character
  ceiling usually binds first.
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

macOS and Windows are the **desktop train**: one version number shared by the two, moved by `make version-desktop x.y.z`, independent of the mobile number this skill prepares. It has its own skill (`release-desktop`), its own record (`changelog/desktop-v<version>.md`), and its own publish scripts. This skill never writes that file, runs those scripts, or publishes anything for desktop.

What that means while preparing a mobile release:

- Keep macOS and Windows out of `changelog/<target>.md` entirely: that file is the mobile record, and a desktop change in it describes work its readers cannot install. Desktop work waits for its own `changelog/desktop-v<version>.md`.
- Keep macOS out of `ios.txt` and `android.txt`. `validate_notes` forbids the whole words `macOS` and `Mac` in both, so a leak fails `check` rather than reaching a store listing.
- `check-versions --train mobile` verifies iOS + Android only. A macOS or Windows version that differs from `<target>` is expected, not a finding.

## Guardrails

- Never edit another version's changelog or store-note files.
- Never touch `changelog/desktop-v<version>.md`, `macos/`, or `windows/` — that is `release-desktop`'s surface.
- Never hand-edit the generated target history entry.
- Never create, move, or push a tag. When the user tags a mobile release themselves, the name is `mobile-<version>` (the pre-2026-09 tags are the bare `v<version>` form).
- Never request, store, or use signing certificates, keystores, API keys, or store credentials.
- Never upload a build, edit a live store listing, or submit production.
- Never run the macOS release or publish scripts, and never put macOS-only work in a store note.
- Surface the first actionable rebuild or validation failure and stop before commit.
