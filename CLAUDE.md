# CLAUDE.md

Guidelines for **Claude Code** in this repo. Two-repo AI environment:

- **Cross-project process rules** (workflow, planning, diagnosis, review, naming, docs authoring) live in `~/.claude/rules/` — managed by the [`configurations`](https://github.com/siansiansu/configurations) dotfiles repo and symlinked in by its `setup.sh`. **Fresh machine**: clone `configurations` + run its `setup.sh` first.
- **Project-specific rules** live in `.claude/rules/` — auto-load via `paths:` glob; three are always-on (`security-rules.md` / `doc-lookup.md` / `taigi-incidents.md`).
- **Personal defaults** (theme, response style, Codex usage) live in `~/.claude/CLAUDE.md` (also from `configurations`).

## Project Overview

**Taigi Keyboard** — cross-platform Taiwanese input method: iOS (Swift + KeyboardKit), Android (Kotlin + FlorisBoard), macOS (IMKit) and Windows (TSF) over a shared Rust engine. POJ/TL romanization, Hanji (漢字), tone variation, autocomplete, continuous input.

## Project Structure

```
taigikeyboard/
├── android/          # Android app (Kotlin + FlorisBoard)
├── ios/              # iOS app (Swift + KeyboardKit)
├── macos/            # macOS input method (Swift + IMKit, SwiftPM)
├── windows/          # Windows input method (Rust TSF, Cargo workspace)
├── engine/           # Shared Rust engine — Cargo workspace, FFI to every platform
├── docs/             # Specs: engine/, architecture/, ui/, references/, reports/, roadmap.md
├── knowledge/        # Taiwanese phonetics reference (TL/POJ/TPS)
├── taigi-converter/  # Canonical TL↔POJ↔TPS converter (git submodule)
├── .claude/rules/    # Project rules (auto-load via paths: glob) — see "Mandatory Rules"
├── dictionary/       # Dictionary sources + build pipeline + output artifacts
├── changelog/        # Per-release changelogs — edit only at release time
└── references/       # Cloned external IME repos (gitignored)
```

## Core Principles (project-specific)

1. **No project-config modification by AI** — `.xcodeproj` / `.pbxproj` / `.xcworkspace` are **user-only** (enforced by `.claude/hooks/block-project-config.sh`). Xcode 16 synchronized groups auto-include new files under most `Sources/TaigiKeyboard/*` subdirs — exceptions in `.claude/rules/ios-guidelines.md`. Android Gradle files **are** editable.
2. **Cross-platform alignment** — align on **intended behavior**, not API calls: define expected behavior, verify each platform independently, document when the same behavior needs different implementations.
3. **Phonetics = authoritative-source-only** — never infer TL/POJ/TPS rules (or "dead" phonetic tables from test/dictionary absence); read `knowledge/taigi-phonetics-reference.md` and consult `taigi-converter/` first. Read-order in `.claude/rules/phonetics.md`.
4. **Bugfix = confirm root cause before fixing** — trace and verify the root cause (cite `file:line`, evidence), present it, and **wait for explicit approval**. No branch, no edit until the user agrees. Diagnosis and fixing are separate, sequential, user-gated steps.
5. **Release scope / timing / tag = user-gated** — never decide what is in/out of vX, never write "deferred / post-vX / known limitation / ready to tag" without the user's explicit dated words. Present work factually (cost, options, trade-offs). Rule: `~/.claude/rules/diagnosis-discipline.md` § No unilateral release scope.
6. **Direction-first over fix-scope** — between two correct fixes, prefer **consistency, best-practice alignment, and correct architectural direction** over the smaller change-set. If the smaller fix preserves a naming inconsistency / anti-pattern / recurring trap, take the larger fix that resolves the root cause. State the trade-off; do NOT default to minimum-change. (USER 2026-05-25: 「考慮一致性、最佳實踐,方向正確會比較重要」)
7. **Taiwanese word identity = (漢字, 羅馬字) pair** — a word is identified by the **combination** of its Hanji AND its canonical-TL reading; **neither alone is a key**. Same Hanji + different reading = different morpheme (`重/tîng` vs `重/tāng`); same reading + different Hanji = different word. Governs **every** dedup / lookup / ranking-merge / accent / variant / substitution decision **project-wide**. Key on the `(hanzi, tl)` pair — never `hanzi` alone, never `tl` alone. POJ/TPS are alternate renderings of the same TL reading. (USER 2026-05-29: 「(羅馬字 + 漢字)的組合視為同一個字」)

## Mandatory Rules

Cross-project process rules auto-load from `~/.claude/rules/` (don't duplicate them here). Project rules in `.claude/rules/` auto-load on matching file reads; this table is the **topic index**.

| Before… | Read |
|---|---|
| a change affecting iOS/Android parity | `.claude/rules/cross-platform-alignment.md` |
| modifying iOS code (structural → also architecture) | `.claude/rules/ios-guidelines.md` (+ `ios-architecture.md`) |
| changing the iOS Shared-Core Candidate roster | `.claude/rules/ios-shared-core-candidates.md` |
| modifying iOS Settings wiring (`SharedSettings`, `EngineSettingsProvider`) | `.claude/rules/ios-settings-injection.md` |
| modifying Android code (architecture, Kotlin idioms, DI, DataStore) | `.claude/rules/android-guidelines.md` |
| modifying Android Compose / IME code, testing, or a refactor-round PR | `.claude/rules/android-ime-patterns.md` |
| modifying app UI | `.claude/rules/ui-style-guide.md` |
| adding logging / SQL / network / storage | `.claude/rules/security-rules.md` |
| adding, renaming, or deleting an `i18n/*.json` key (keys are cross-platform) | `.claude/rules/i18n.md` |
| Rust engine code (hygiene, workspace, errors, crates, tests) | `.claude/rules/rust-best-practices.md` |
| Rust FFI / proto boundary code, `unsafe`, opaque handles | `.claude/rules/rust-ffi-safety.md` |
| a Rust slice migration / platform→engine swap / `.proto` addition | `.claude/rules/rust-migration-policy.md` |
| any TL/POJ/TPS schema, FST key-family, encoding, or canonical-form work | `.claude/rules/phonetics.md` |
| revisiting a global rule and wanting the concrete Taigi "why" | `.claude/rules/taigi-incidents.md` |
| a real-device dogfood pass, or a PR / memory cites an `Sn` item | `docs/architecture/dogfood-checklist.md` |
| adding or changing a framework/OS API call (KeyboardKit, `UITextDocumentProxy`, `InputMethodService`/`InputConnection`, Compose, DataStore) | `.claude/rules/doc-lookup.md` — verify via `find-docs`/`ctx7` **before** coding, never from model memory |
| a 最佳實踐對齊 section, "Project X does Y" claims, or designing / fixing a segmentation / lattice / ranking / user-freq / syllabifier / next-word / continuous-input slice | `docs/references/mainstream-ime-comparison.md` first — do **not** re-explore `references/` from scratch |

## Build & Test

**Commit-first ordering**: commit → push → `gh pr create` with NO pre-commit test gate; post-PR verification fires AFTER `gh pr create` returns. Do NOT block commit / push / PR-open on test results.

| Platform | Build | Test |
|---|---|---|
| iOS | Xcode → keyboard extension | `xcodebuild -project ios/TaigiKeyboard.xcodeproj -scheme TaigiKeyboardTests -destination 'platform=iOS Simulator,id=81ADB050-5242-460C-90DA-F3FAF3F6AAA5' test` (iPhone 17 / iOS 26.1, UDID-pinned) |
| Android | `cd android && ./gradlew :app:assembleDebug` | `cd android && ./gradlew :app:testDebugUnitTest` |
| engine | `cargo build --workspace` | `cargo test --workspace` |
| macOS | `make -C macos build` (`make -C macos install` before a dogfood pass) | `make -C macos test` |
| Windows | `make windows-check` (host-side gate; the TSF DLL builds only on the Windows box — `docs/architecture/windows-release.md`) | included |
| taigi-converter | — | `npm test` from `taigi-converter/` (bare `node --test tests/` fails on Node 26) |

**Bootstrap after cloning** — the engine binaries a platform build links are generated, not committed. Clone with `--recurse-submodules` (the dictionary pipeline needs `taigi-converter/`). One pass per machine, not per build:

| Command | Produces | Needed by |
|---|---|---|
| `make build` (5 s warm; minutes on a cold target dir) | iOS + macOS xcframeworks and their swift-bridge wrappers, Android `jniLibs/*.so`, platform protos | Xcode, SwiftPM, Gradle all link these |

**Stale-artifact gate (mandatory before every iOS/Android build+test)** — the artifacts above are local, untracked build output; building against a stale one gives **false-green tests**. Check `git diff --stat` against this table first:

| If the diff touches… | Run first | Regenerates |
|---|---|---|
| `engine/` (any Rust source, `.proto`, `Cargo.toml`) | `make build` | Platform protos + iOS/macOS xcframeworks + Android jniLibs |
| `dictionary/` (CSV sources, build scripts, syllabifier rules) | `make dict` then `make build` | `dictionary.bin` + `syllables.fst`, then the bundles above |
| platform-only Swift / Kotlin / docs | — | nothing |

Why `dictionaries/` and `fonts/font/` are committed while engine binaries are not, measured timings, and the release-rebuild rule: `docs/architecture/build-artifacts.md`.

**Post-PR parallel verification** (`~/.claude/rules/round-workflow.md` § Codex review sandwich step 6): right after `gh pr create` returns, run build+test for **every platform the diff touches** in the background (one message, parallel `Bash` calls).
Stale-binary gate runs FIRST (sequentially) when `engine/` or `dictionary/` is touched, then platform gates fire in parallel.
On failure: report failing target + first error line, push the fix as a new commit (no `--amend`), re-run only the failing gate. Do NOT close the PR.

## Communication

- Reply in **Taiwanese Mandarin (台灣華語)**; documentation and code comments stay in **English**.
- **Doc-authoring language**: living reference (`.claude/rules/**`, `docs/architecture/**`, `docs/engine/**`, `docs/roadmap.md`) defaults to **English prose**; CJK only for (a) verbatim USER quotes kept as evidence and (b) domain phonetic terms / examples. Dated snapshots (`docs/reports/**`, `docs/releases/**`) are frozen — do **not** retro-translate.
- Concise, bullet-point, key points only — no filler.
- Recommendation first, then 2-4 ranked options with one-line trade-offs and impact scope, before changing code. Output shape rules: global `CLAUDE.md` § Interaction defaults.

## PR numbers before the 2026-09-07 migration

Every `#NNN` written before 2026-09-07 refers to the old repository; numbering restarted at #1. Resolve from git first: `git log --all --oneline --grep="(#NNN)"` — full story in `docs/architecture/pr-number-migration.md`.

## Key References

- `docs/README.md` — full documentation index
- `docs/roadmap.md` + project memory (Claude auto-memory, `~/.claude/projects/-Users-alexsu-Workspace-taigikeyboard/memory/`) — live multi-PR plan & round hand-off (**not** `IMPLEMENTATION_PLAN.md`)
- `references/` — cloned IMEs (azooKey, librime, khiin-rs, McBopomofo, florisboard, …); enter via the comparison doc above, not directly
- Android IME: follow [Creating an Input Method](https://developer.android.com/develop/ui/views/touch-and-input/creating-input-method)
