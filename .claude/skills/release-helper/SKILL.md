---
name: release-helper
description: Cut a release on main. Diff against base tag, rebuild dict + Rust engine, update 4 changelog files, commit, force-retag, push. Args:&nbsp;<base-tag>&nbsp;<target-version>. Re-run safe — overrides existing tag, dedupes entries.
disable-model-invocation: false
---

# Release Helper

Run from `main`. User supplies `<base-tag> <target>`.
Example: `/release-helper v3.5.0 v3.5.7`

Re-run safe: existing `<target>` tag is force-overwritten; existing changelog entries are replaced in place, never appended.

## 1. Sanity (abort on failure)

- HEAD = `main`
- Working tree clean (`git status --short` empty)
- `git fetch --prune && git pull --ff-only origin main`
- `git rev-parse <base-tag>` succeeds
- Open PRs (`gh pr list`): if any, list and ask before continuing

If `<target>` tag exists locally or on origin, note it once; step 6 will overwrite.

## 2. Housekeeping

`git branch --merged main | grep -v '^[* ] main$' | xargs -n1 -r git branch -d`

## 3. Diff context (parallel)

- `git log <base>..HEAD --oneline`
- `git diff <base>..HEAD --stat --name-status`
- `git log <base>..HEAD --format="%s%n%b%n---"`
- Read `CHANGELOG.md`, `changelog/<target>.md` if present

## 4. Rebuild (sequential, abort on failure)

- `make dict`
- `make build`

## 5. Update 4 files (idempotent)

For each file: if a `<target>` block already exists, **replace** its body in place. Otherwise insert at the top (newest first).

| File | Form |
| --- | --- |
| `CHANGELOG.md` | `- [<target>](changelog/<target>.md)` |
| `changelog/<target>.md` | Categories: iOS / Android / Dictionary / Shared. Sub: New Features, Bug Fixes, Refactoring, Removed, Changes, New Files. Regenerate fully from step 3 diff. |
| `ios/Sources/TaigiKeyboard/Strings/HomeTexts.swift` | `("x.y.z", "YYYY/MM/DD", [LocalizedText(hanji: "…"), …])` |
| `android/app/src/main/java/com/siansiansu/taigikeyboard/localization/HomeTexts.kt` | `VersionEntry("x.y.z", "YYYY/MM/DD", listOf(LocalizedText(hanji = "…"), …))` |

`versionHistoryEntries` rules (both platforms):
- Include: user-visible features, UI changes, noticeable bug fixes, new layouts, new dictionary sources
- Exclude: refactors, internal cleanups, docs, tests, dev tooling, silent dep bumps
- Group small related fixes into one line
- iOS and Android lists may diverge — only what each platform exposes

## 6. Commit + force-tag + push

```
git add -A
git commit -m "<target>: changelog + dict/engine rebuild"
git tag -f <target>
git push origin main
git push --force origin refs/tags/<target>
```

If steps 4 + 5 produced no diff: tell the user, do not commit.

## Notes

- Hand-edit ONLY the 4 changelog files. Build artifacts ride via `git add -A`.
- Never modify other versions' `changelog/*.md`.
- Rebuild failure: surface error, abort before committing.
