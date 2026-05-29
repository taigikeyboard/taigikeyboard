# CLAUDE.md

Guidelines for **Claude Code** in this repo. Two-repo AI environment:

- **Cross-project process rules** (workflow, planning, diagnosis, review, naming, docs authoring) live in `~/.claude/rules/` — managed by the [`configurations`](https://github.com/siansiansu/configurations) dotfiles repo and symlinked in by **that repo's `setup.sh`**. **Required external dependency**: clone `configurations` + run its `setup.sh` before working in this repo on a fresh machine.
- **Project-specific rules** (Taigi phonetics, iOS/Android/Rust engine specifics, cross-platform parity, dual-platform UI, Rust migration policy, Taigi incident appendix) live in `.claude/rules/` here — auto-load via `paths:` glob when Claude reads matching files; three (`security-rules.md` / `doc-lookup.md` / `taigi-incidents.md`) are always-on.
- **Personal defaults** (theme, response style, Codex usage, Opus 4.7 tuning) live in `~/.claude/CLAUDE.md` (also from `configurations`).

## Project Overview

**Taigi Keyboard** — cross-platform Taiwanese input method. iOS (Swift + KeyboardKit) and Android (Kotlin + FlorisBoard) over a shared Rust engine. Supports POJ/TL romanization, Hanji (漢字), tone variation, autocomplete, and continuous input.

## Project Structure

```
taigikeyboard/
├── android/          # Android app (Kotlin + FlorisBoard)
├── ios/              # iOS app (Swift + KeyboardKit)
├── engine/           # Shared Rust engine — Cargo workspace, FFI to both platforms
├── docs/             # Specs: engine/, architecture/, ui/, references/, reports/, roadmap.md
├── knowledge/        # Taiwanese phonetics reference (TL/POJ/TPS)
├── taigi-converter/  # Canonical TL↔POJ↔TPS converter (git submodule)
├── .claude/rules/    # Mandatory rules (auto-load via paths: glob) — see "Mandatory Rules" table
├── dictionary/       # Dictionary data files
├── changelog/        # Per-release changelogs — edit only at release time
├── content/          # In-app content (FAQ / feature JSON)
└── references/       # Cloned external IME repos (gitignored — no tracked content)
```

## Core Principles (project-specific)

1. **No project-config modification by AI** — `.xcodeproj` / `.pbxproj` / `.xcworkspace` are **user-only** (enforced by `.claude/hooks/block-project-config.sh`). Xcode 16 synchronized groups auto-include new files under most `Sources/TaigiKeyboard/*` subdirs — see `.claude/rules/ios-guidelines.md` for the synced-group rules + exceptions. Android Gradle (`build.gradle`, `*.gradle.kts`) **is** editable by Claude (lifted 2026-05-09).
2. **Cross-platform alignment** — align on **intended behavior**, not API calls: define expected behavior, verify each platform independently, document when the same behavior needs different implementations.
3. **Phonetics = authoritative-source-only** — never infer TL/POJ/TPS rules (or "dead" phonetic tables from test/dictionary absence); read `knowledge/taigi-phonetics-reference.md` and consult `taigi-converter/` first. Full read-order in `.claude/rules/phonetics.md`.
4. **Bugfix = confirm root cause before fixing** — for any bug fix, first carefully trace and verify the root cause (cite `file:line`, evidence), present it to the user, and **wait for explicit approval**. Do NOT create a branch, edit code, or implement until the user agrees the root cause is correct. Diagnosis and fixing are separate, sequential, user-gated steps.
5. **Release scope / timing / tag = user-gated** — never decide what is in/out of vX, never tag something "deferred / post-vX / known limitation / ready to tag" without the user's explicit dated word. Full rule in `~/.claude/rules/diagnosis-discipline.md` § No unilateral release scope; Taigi incident in `.claude/rules/taigi-incidents.md`. Present work factually (cost, options, trade-offs); never assign or exclude scope yourself.
6. **Direction-first over fix-scope** — when choosing between two correct fixes, prioritize **consistency, best-practice alignment, and correct architectural direction** over minimizing the change-set. Scope minimality is NOT the top criterion: if the smaller fix (e.g. per-call-site qualification, one-line workaround) preserves a naming inconsistency / architectural anti-pattern / recurring trap, prefer the larger fix that resolves the root cause and aligns the codebase. State the trade-off when presenting options; do NOT default to the minimum-change option. **Why**: USER explicit preference 2026-05-25 — "比起修復範圍，我認為考慮一致性、最佳實踐，方向正確會比較重要" (during iOS `AutocompleteService` ambiguity round; per-call-site qualification was the smallest fix, full class rename was the consistent-with-`EnglishAutocompleteService`-sibling root-cause fix).
7. **Taiwanese word identity = (漢字, 羅馬字) pair** — a Taiwanese word is uniquely identified by the **combination** of its Hanji AND its romanization (canonical TL); **neither field alone is a key**. 台語一字多音: same Hanji + different reading = **different** morpheme (e.g. `重/tîng` 重複 vs `重/tāng` 重量; `八/pat` 文讀 vs `八/pueh` 白讀); homophones: same reading + different Hanji = **different** word. Two entries are "the same word" **only when both 漢字 and 羅馬字 match**. This governs **every** dedup / lookup / ranking-merge / accent / variant / substitution decision **project-wide** (engine `lexicon`/`ranking`, dictionary build `merge`/`merge_csv`/`cleanup`, accent-substitution generation), NOT just the dictionary pipeline. When keying, grouping, or matching Taiwanese entries, key on the `(hanzi, tl)` pair — never `hanzi` alone, never `tl` alone. POJ/TPS are alternate renderings of the same TL reading and do not create separate identities. **Why**: USER 2026-05-29 — "針對台語羅馬字,(羅馬字 + 漢字)的組合視為同一個字".

## Mandatory Rules

**Cross-project process rules** (workflow, planning, diagnosis, code review, naming, docs authoring, Claude interaction) auto-load from `~/.claude/rules/` via the `configurations` repo symlinks. Don't duplicate them here.

**Project-specific rules** live in `.claude/rules/` and auto-load via `paths:` glob when Claude reads matching files (e.g. editing `ios/**/*.swift` auto-loads `ios-guidelines.md` / `ios-architecture.md`). The table below is the **conceptual discovery index** — use it when a task is framed by topic ("phonetics work", "cross-platform parity") rather than by a specific file path, since `paths:` triggers on file reads, not topic mentions. Three rules are **always-on** (no `paths:` field): `security-rules.md`, `doc-lookup.md`, `taigi-incidents.md`.

| Before… | Read |
|---|---|
| a change affecting iOS/Android parity | `.claude/rules/cross-platform-alignment.md` |
| modifying iOS code (structural → also architecture) | `.claude/rules/ios-guidelines.md` (+ `ios-architecture.md`) |
| marking a file as iOS Shared-Core Candidate or changing the candidate roster | `.claude/rules/ios-shared-core-candidates.md` |
| modifying iOS Settings wiring (`SharedSettings`, `EngineSettingsProvider`, live-read regressions) | `.claude/rules/ios-settings-injection.md` |
| modifying Android code (core architecture, Kotlin idioms, DI, DataStore) | `.claude/rules/android-guidelines.md` |
| modifying Android Compose / IME-specific code, testing, or a refactor-round PR | `.claude/rules/android-ime-patterns.md` |
| modifying app UI | `.claude/rules/ui-style-guide.md` |
| adding logging / SQL / network / storage | `.claude/rules/security-rules.md` |
| Rust engine code (general hygiene, workspace, errors, crates, tests) | `.claude/rules/rust-best-practices.md` |
| Rust FFI / proto boundary code, `unsafe` blocks, opaque handles, enforcement | `.claude/rules/rust-ffi-safety.md` |
| starting a Rust slice migration / platform→engine swap / `.proto` addition / mirror-source delete | `.claude/rules/rust-migration-policy.md` |
| any TL/POJ/TPS schema, FST key-family, encoding, or canonical-form work | `.claude/rules/phonetics.md` |
| revisiting a global rule and wanting the concrete Taigi "why" | `.claude/rules/taigi-incidents.md` |
| adding or changing a call to / contract with a framework/OS API (KeyboardKit, `UIInputViewController`/`UITextDocumentProxy`, `InputMethodService`/`InputConnection`/`EditorInfo`, Jetpack Compose, DataStore) — not trivial edits to framework-adjacent code | `.claude/rules/doc-lookup.md` — verify the current API via `find-docs`/`ctx7` (or local `references/KeyboardKit-Documentation/`) **before** coding; never from model memory |
| writing a 最佳實踐對齊 section, claiming "Project X does Y", or designing a segmentation / lattice / ranking / user-freq / syllabifier / predictive / next-word / continuous-input slice | `docs/references/mainstream-ime-comparison.md` first (TL;DR matrix + topic index → drill into per-repo cards; do **not** re-explore `references/` from scratch) |

## Build & Test

The **user runs all builds/tests manually mid-round** — never invoke these or add build hooks/reminders mid-round. **Commit-first ordering**: commit → push → `gh pr create` runs WITHOUT a pre-commit test gate; the post-PR parallel verification (§ EXCEPTION below + `~/.claude/rules/round-workflow.md` sandwich step 6) fires tests in the background AFTER `gh pr create` returns the URL, in parallel with the PR-bot review. Total wall-clock = max(test, PR-bot) instead of sum. Do NOT block commit / push / PR-open on test results. Aligns with `.claude/rules/rust-best-practices.md §7` (judgment-gated, not mandatory).

Reference:

| Platform | Build | Test |
|---|---|---|
| iOS | Xcode → keyboard extension | Xcode / `xcodebuild -project ios/TaigiKeyboard.xcodeproj -scheme TaigiKeyboardTests -destination 'platform=iOS Simulator,id=81ADB050-5242-460C-90DA-F3FAF3F6AAA5' test` (iPhone 17 / iOS 26.1; UDID-pinned for derived-data + sim-runtime cache reuse — project `IPHONEOS_DEPLOYMENT_TARGET = 26.1`, so iOS 26.1+ sim required) |
| Android | `cd android && ./gradlew :app:assembleDebug` | `cd android && ./gradlew :app:testDebugUnitTest` |
| engine | `cargo build --workspace` | `cargo test --workspace` |
| taigi-converter | — | `node --test tests/` |

**Stale-binary gate (mandatory upstream of every iOS/Android build+test)**:

iOS and Android link against pre-built artifacts (`ios/RustEngine/RustTaigi.xcframework`, `android/app/src/main/jniLibs/`, `dictionary/output/dictionary.bin`, `dictionary/output/syllables.fst`). Running `xcodebuild` / `./gradlew` against stale artifacts gives **false-green test results** — Swift/Kotlin builds against the OLD engine binary even though `engine/src/*.rs` changed on disk.

Before any iOS or Android build/test/dogfood when the diff touches **upstream** sources, regenerate artifacts first:

| If the diff touches… | Run first | Regenerates |
|---|---|---|
| `engine/` (any Rust source, `.proto`, `Cargo.toml`) | `make build` | Platform protos + iOS xcframework + Android jniLibs |
| `dictionary/` (CSV sources, build scripts, syllabifier rules) | `make dict` then `make build` | `dictionary.bin` + `syllables.fst` (then xcframework/jniLibs that bundle them) |
| iOS-only Swift / Android-only Kotlin / docs only | — | No regen needed |

Skipping this gate is the #1 source of "tests pass locally but Continuous-input behaves wrong on device" bugs. Always check `git diff --stat` against the table above before any iOS/Android invocation.

`make build` is **sequential, ~3-5 min** (cargo + xcframework + jniLibs) — it cannot run in parallel with the iOS/Android gates it feeds. `make dict` is a separate ~30s pass that must complete before `make build`.

**EXCEPTION — post-PR parallel verification** per `~/.claude/rules/round-workflow.md` § Codex review sandwich step 6: immediately after `gh pr create` returns the URL, kick off **every platform the diff touches** (iOS, Android, engine) build+test in the background (single message, parallel `Bash` calls with `run_in_background: true`) so total wall-clock = max(build, PR-bot review) instead of sum. Single-platform refactor → run only that platform's gate. Multi-platform diff → run all touched platforms. **If the diff touches `engine/` or `dictionary/`, the stale-binary gate above runs FIRST (sequentially), then the platform gates fire in parallel.** On failure: notify user with the failing target + first error line, push fix as a new commit on the same branch (no `--amend`), re-run only the failing gate. Do NOT close the PR.

## Communication

- Reply in **Taiwanese Mandarin (台灣華語)**; documentation and code comments stay in **English**.
- Concise, bullet-point, key points only — no filler.
- Analyze first and present options; explain scope of impact before changing code.

## Key References

- `docs/README.md` — full documentation index
- `docs/roadmap.md` + `memory/project_*.md` — live multi-PR plan & round hand-off (this project uses these, **not** `IMPLEMENTATION_PLAN.md`)
- `references/` — cloned IMEs (azooKey, librime, khiin-rs, McBopomofo, florisboard, …); enter via the comparison doc above, not directly
- Android IME: follow [Creating an Input Method](https://developer.android.com/develop/ui/views/touch-and-input/creating-input-method)
