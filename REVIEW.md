# REVIEW.md

What a reviewer — human, `/code-review`, or a fresh-context subagent — checks in a TaigiKeyboard diff. Rule detail lives in `AGENTS.md` and `docs/contributing/`; this file names what counts as a defect and how to report it.

## What counts as a defect

- **Word identity broken** — a dedup / lookup / ranking merge / variant decision keyed on 漢字 alone or TL alone instead of the `(漢字, canonical-TL)` pair (`AGENTS.md` Core Principle 6).
- **Phonetics inferred** — a TL / POJ / TPS rule added, removed or changed without a cite from `docs/phonetics/taigi-phonetics-reference.md` or `taigi-converter/` (`docs/contributing/phonetics.md`). Absence from the dictionary or tests is no evidence.
- **Cross-platform drift** — shared behaviour changed on one platform without its counterparts, or the divergence not recorded in `docs/architecture/behavioral-invariants.md` (`docs/contributing/cross-platform-alignment.md`). Logic every platform needs written outside `engine/`.
- **User data lost** — a schema / backup / settings change that drops or silently resets learned words, custom dictionary entries or `.taigi` backups on upgrade. Settings may change; data may not.
- **FFI boundary** — new `unsafe`, a panic that can cross the seam, or a handle crossing it (`docs/contributing/rust-ffi-safety.md`).
- **Redundant fallback** — "A fails → fall back to B" in a primary data flow, unless marked "temporary, remove after X".
- **Docs that lie** — a doc, comment or invariant the diff makes untrue.
- **Personal identifier** — an address, account ID or real name in the diff, commit message or PR body (`CONTRIBUTING.md` § Secrets).
- **iOS keyboard extension** — anything that risks the 64 MB memory cap or a keyboard dismiss.

## Risk tiers (set by the paths touched, not the author)

| Tier | Paths | Review |
|---|---|---|
| High | `engine/userdata/`, `engine/swift-ffi/`, `engine/android-jni/`, `macos/crates/taigi-macos-ffi/`, `desktop/crates/taigi-desktop-update/`, `windows/installer/`, `.github/workflows/`, `tools/release/`, `dictionary/build/` | Line-by-line; maintainer approval; state the revert path in the PR |
| Normal | other source under `engine/`, `ios/`, `android/`, `macos/`, `windows/`, `linux/`, `desktop/`, `web/` | One review pass on the diff |
| Low | `docs/**`, `i18n/*.json` string-only edits, comments | Skim for the defects above |

A diff with numeric / geometry fidelity risk (unit conversion, wide caller surface) is High whatever its paths (`docs/contributing/known-pitfalls.md` § Review and device gates).

## Skip

Generated files — regenerated, never hand-edited (`.gitattributes` lists them as `linguist-generated`): `*/Engine/Generated/**`, `*/Strings/Generated/**`, `*/DesktopCore/Generated/**`, Android `engine/proto/**` and `i18n/generated/**`, `strings_i18n.xml`, `ios/Localizable.xcstrings`, `InfoPlist.strings`, `desktop/crates/taigi-desktop-core/src/strings/generated.rs`, `windows/build-support/product_name_strings.rs`, `windows/installer/Messages.iss`, `web/src/generated/**`. Binary artifacts: `assets/dictionaries/*.bin`, `*.fst` (they differ every build). `changelog/` outside a release. `references/` (cloned external repos). Dated snapshots under `docs/reports/` and `docs/releases/`.

Check instead that a generated file changed together with its source (`make protos`, `make i18n`, `make dict`).

## Not a finding

- Formatting — `make lint` gates it in CI.
- Style preferences that match the surrounding code (`docs/contributing/` platform guides decide).
- Pre-existing issues outside the diff — report under a separate "Pre-existing" heading, never as a blocker.

## Report format

- Open with a one-line summary: the finding count, or exactly `No issues found.`
- One finding per item: `path:line` — the defect — the concrete input or state that triggers it. Name only files you opened; never guess a filename.
- Group by severity: Blocking, then Nit, then Pre-existing.
- Bug fix PRs: check the PR states the root cause at `file:line` and the fix addresses it, not a symptom.

## Convergence

- Re-reviews look at the new commits only, and do not re-report a finding already raised.
- Report the class of a defect ("every caller of X skips Y"), so one fix pass covers all of it.
- After three review cycles on one PR, stop and question the approach instead of starting a fourth.
