---
name: release-helper
description: Prepare a release on main, rebuild generated artifacts, update the detailed changelog, create concise English iOS and Android What's New text, mirror it into both apps' version history, validate it, commit, and push. Use when preparing a version for manual App Store Connect or Google Play release. Never tags, uploads builds, or submits a store release.
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
- Run `python3 tools/release_notes.py check-versions --version <target>`.
- Report open PRs and ask before continuing if any exist.
- Use `<base-tag>..HEAD` as the release range. State it before edits.

The skill may report a required version change but must not edit the user-owned iOS `.pbxproj`.

## 2. Analyze the release

Read the complete release range:

```bash
git log <base-tag>..HEAD --oneline
git diff <base-tag>..HEAD --stat --name-status
git log <base-tag>..HEAD --format="%s%n%b%n---"
```

Trace user-visible behavior by platform. Do not infer release scope from commit subjects alone.

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
| `changelog/<target>.md` | Detailed iOS / Android / Dictionary / Shared record |
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
- Do not mention the other platform in platform-specific notes.
- Keep rendered bullets and newlines within 500 Unicode characters.
- A platform's app history and store text must use exactly the same entries.

Synchronize both apps after the canonical files are final:

```bash
python3 tools/release_notes.py sync --version <target> --date YYYY/MM/DD
```

## 5. Validate and render

```bash
python3 tools/release_notes.py check --version <target>
python3 tools/release_notes.py check-versions --version <target>
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

## Guardrails

- Never edit another version's changelog or store-note files.
- Never hand-edit the generated target history entry.
- Never create, move, or push a tag.
- Never request, store, or use signing certificates, keystores, API keys, or store credentials.
- Never upload a build, edit a live store listing, or submit production.
- Surface the first actionable rebuild or validation failure and stop before commit.
