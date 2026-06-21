---
name: release-helper
description: Cut a release on main. Diff against base tag, rebuild dict + Rust engine, update 4 changelog files, commit, push. User tags manually. Args:&nbsp;<base-tag>&nbsp;<target-version>. Re-run safe — auto picks full vs incremental, dedupes entries by topic.
disable-model-invocation: false
---

# Release Helper

Run from `main`. User supplies `<base-tag> <target>`.
Example: `/release-helper v3.5.0 v3.5.7`

Re-run safe: existing changelog entries are merged in place by topic, never blindly appended. Step 4.5 auto-selects full regenerate vs incremental.

**This skill does NOT tag.** It commits + pushes the release prep to `main`; tagging is user-gated and manual. After the skill finishes, the user runs `git tag <target> && git push origin <target>` themselves.

## 1. Sanity (abort on failure)

- HEAD = `main`
- Working tree clean (`git status --short` empty)
- `git fetch --prune && git pull --ff-only origin main`
- `git rev-parse <base-tag>` succeeds
- Open PRs (`gh pr list`): if any, list and ask before continuing

If `<target>` tag exists locally or on origin, note it once (informational — this skill never creates or moves tags).

## 2. Housekeeping

`git branch --merged main | grep -v '^[* ] main$' | xargs -n1 -r git branch -d`

## 3. Diff context (parallel)

- `git log <base>..HEAD --oneline`
- `git diff <base>..HEAD --stat --name-status`
- `git log <base>..HEAD --format="%s%n%b%n---"`
- Read `CHANGELOG.md`, `changelog/<target>.md` if present

## 4. Rebuild (sequential, abort on failure)

- `RELEASE_VERSION=<target> make dict` — passing the version sets the dict build's diff base to the newest release tag strictly older than `<target>` (so it excludes `<target>` even on a re-run) and prints the build-drop + vs-previous-release diff summary. The previous release's `dictionary.csv` is read via `git show <prev-tag>:…`; no snapshot file is written or committed.
- `make build`

## 4.5 Mode selection (auto)

Default: **full regenerate** (current SKILL behavior, safest).

Switch to **incremental** only if **all** checks pass — otherwise run full:

- `<target>` tag exists locally: `git rev-parse <target>` succeeds
- No history rewrite below tag: `git merge-base --is-ancestor <target> HEAD` succeeds
- Tag not force-pushed apart: local `<target>` SHA == `origin/<target>` SHA (or origin tag absent)
- New range non-empty and clean: `git rev-list <target>..HEAD` has ≥1 commit and **zero** subjects starting with `Revert "`
- `changelog/<target>.md` exists and is non-empty
- `<base>` arg is an ancestor of `<target>`: `git merge-base --is-ancestor <base> <target>` (otherwise base shifted, full regenerate)

State the chosen mode before step 5 ("mode: incremental, N new commits since `<target>`" or "mode: full regenerate, reason: …").

## 5. Update 4 files (idempotent)

**Full mode**: regenerate `<target>` block content from `<base>..HEAD` diff. For each file: if a `<target>` block already exists, **replace** its body in place. Otherwise insert at the top (newest first).

**Incremental mode**: only classify commits in `<target>..HEAD`. Read each existing file's `<target>` block as the base; merge new entries **by topic, not by SHA**:

- If a new commit's topic matches an existing line → extend / refine that line (e.g. "fixed A" → "fixed A and B"); do **not** add a second line
- If a new commit's topic is genuinely new → add a single line under the right category
- Re-classification: if a previously listed item now spans more platforms, extend the existing line's scope rather than duplicate
- Never append below an existing `<target>` block; always edit in place

| File | Form |
| --- | --- |
| `CHANGELOG.md` | `- [<target>](changelog/<target>.md)` |
| `changelog/<target>.md` | Categories: iOS / Android / Dictionary / Shared. Sub: New Features, Bug Fixes, Refactoring, Removed, Changes, New Files. Regenerate fully from step 3 diff. |
| `ios/Sources/TaigiKeyboard/App/Tabs/Home/VersionHistory.swift` | `VersionHistory.entries`: `("x.y.z", "YYYY/MM/DD", ["…", …])` (plain English strings) |
| `android/app/src/main/java/com/siansiansu/taigikeyboard/content/VersionHistory.kt` | `VersionHistory.entries`: `VersionEntry("x.y.z", "YYYY/MM/DD", listOf("…", …))` (plain English strings) |

Version-history entry rules (both platforms — iOS `VersionHistory.entries`, Android `VersionHistory.entries`):
- Include: user-visible features, UI changes, noticeable bug fixes, new layouts, new dictionary sources
- Exclude: refactors, internal cleanups, docs, tests, dev tooling, silent dep bumps
- Group small related fixes into one line
- iOS and Android lists may diverge — only what each platform exposes

## 6. Commit + push (NO tag)

```
git add -A
git commit -m "<target>: changelog + dict/engine rebuild"
git push origin main
```

Do **NOT** tag. After pushing, report the commit SHA and tell the user to tag manually when ready:

```
git tag <target>
git push origin <target>
```

If steps 4 + 5 produced no diff: tell the user, do not commit.

## Notes

- Hand-edit ONLY the 4 changelog files. Build artifacts ride via `git add -A`.
- Never modify other versions' `changelog/*.md`.
- Rebuild failure: surface error, abort before committing.
- Never tag. Tagging is user-gated and manual — the skill stops after `git push origin main`.
