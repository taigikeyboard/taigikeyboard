# CLAUDE.md

Guidelines for **Claude Code** in this repo. Two-repo AI environment:

- **Cross-project process rules** (workflow, planning, diagnosis, review, naming, docs authoring) live in `~/.claude/rules/` — managed by the [`configurations`](https://github.com/siansiansu/configurations) dotfiles repo and symlinked in by **that repo's `setup.sh`**. **Required external dependency**: clone `configurations` + run its `setup.sh` before working in this repo on a fresh machine.
- **Project-specific rules** (Taigi phonetics, iOS/Android/Rust engine specifics, cross-platform parity, dual-platform UI, Rust migration policy, Taigi incident appendix) live in `rules/` here.
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
├── rules/            # Mandatory rules — see "Mandatory Rules" table
├── dictionary/       # Dictionary data files
├── changelog/        # Per-release changelogs — edit only at release time
├── content/          # In-app content (FAQ / feature JSON)
└── references/       # Cloned external IME repos (gitignored — no tracked content)
```

## Core Principles (project-specific)

1. **No project-config modification by AI** — `.xcodeproj` / `.pbxproj` / `.xcworkspace` are **user-only** (enforced by `.claude/hooks/block-project-config.sh`). Xcode 16 synchronized groups auto-include new files under most `Sources/TaigiKeyboard/*` subdirs — see `rules/ios-guidelines.md` for the synced-group rules + exceptions. Android Gradle (`build.gradle`, `*.gradle.kts`) **is** editable by Claude (lifted 2026-05-09).
2. **Cross-platform alignment** — align on **intended behavior**, not API calls: define expected behavior, verify each platform independently, document when the same behavior needs different implementations.
3. **Phonetics = authoritative-source-only** — never infer TL/POJ/TPS rules (or "dead" phonetic tables from test/dictionary absence); read `knowledge/taigi-phonetics-reference.md` and consult `taigi-converter/` first. Full read-order in `rules/phonetics.md`.
4. **Bugfix = confirm root cause before fixing** — for any bug fix, first carefully trace and verify the root cause (cite `file:line`, evidence), present it to the user, and **wait for explicit approval**. Do NOT create a branch, edit code, or implement until the user agrees the root cause is correct. Diagnosis and fixing are separate, sequential, user-gated steps.
5. **Release scope / timing / tag = user-gated** — never decide what is in/out of vX, never tag something "deferred / post-vX / known limitation / ready to tag" without the user's explicit dated word. Full rule in `~/.claude/rules/diagnosis-discipline.md` § No unilateral release scope; Taigi incident in `rules/taigi-incidents.md`. Present work factually (cost, options, trade-offs); never assign or exclude scope yourself.

## Mandatory Rules

**Cross-project process rules** (workflow, planning, diagnosis, code review, naming, docs authoring, Claude interaction) auto-load from `~/.claude/rules/` via the `configurations` repo symlinks. Don't duplicate them here.

**Project-specific rules** below override defaults — read the listed file **before** the matching work.

| Before… | Read |
|---|---|
| a change affecting iOS/Android parity | `rules/cross-platform-alignment.md` |
| modifying iOS code (structural → also architecture) | `rules/ios-guidelines.md` (+ `ios-architecture.md`) |
| marking a file as iOS Shared-Core Candidate or changing the candidate roster | `rules/ios-shared-core-candidates.md` |
| modifying iOS Settings wiring (`SharedSettings`, `EngineSettingsProvider`, live-read regressions) | `rules/ios-settings-injection.md` |
| modifying Android code (core architecture, Kotlin idioms, DI, DataStore) | `rules/android-guidelines.md` |
| modifying Android Compose / IME-specific code, testing, or a refactor-round PR | `rules/android-ime-patterns.md` |
| modifying app UI | `rules/ui-style-guide.md` |
| adding logging / SQL / network / storage | `rules/security-rules.md` |
| Rust engine code (general hygiene, workspace, errors, crates, tests) | `rules/rust-best-practices.md` |
| Rust FFI / proto boundary code, `unsafe` blocks, opaque handles, enforcement | `rules/rust-ffi-safety.md` |
| starting a Rust slice migration / platform→engine swap / `.proto` addition / mirror-source delete | `rules/rust-migration-policy.md` |
| any TL/POJ/TPS schema, FST key-family, encoding, or canonical-form work | `rules/phonetics.md` |
| revisiting a global rule and wanting the concrete Taigi "why" | `rules/taigi-incidents.md` |
| adding or changing a call to / contract with a framework/OS API (KeyboardKit, `UIInputViewController`/`UITextDocumentProxy`, `InputMethodService`/`InputConnection`/`EditorInfo`, Jetpack Compose, DataStore) — not trivial edits to framework-adjacent code | `rules/doc-lookup.md` — verify the current API via `find-docs`/`ctx7` (or local `references/KeyboardKit-Documentation/`) **before** coding; never from model memory |
| writing a 最佳實踐對齊 section, claiming "Project X does Y", or designing a segmentation / lattice / ranking / user-freq / syllabifier / predictive / next-word / continuous-input slice | `docs/references/mainstream-ime-comparison.md` first (TL;DR matrix + topic index → drill into per-repo cards; do **not** re-explore `references/` from scratch) |

## Build & Test

The **user runs all builds/tests manually** — never invoke these or add build hooks/reminders. Reference only:

| Platform | Build | Test |
|---|---|---|
| iOS | Xcode → keyboard extension | Xcode / `xcodebuild test` |
| Android | `./gradlew assembleDebug` | `./gradlew test` |
| engine | `cargo build` (workspace) | `cargo test --workspace` |
| taigi-converter | — | `node --test tests/` |

## Communication

- Reply in **Taiwanese Mandarin (台灣華語)**; documentation and code comments stay in **English**.
- Concise, bullet-point, key points only — no filler.
- Analyze first and present options; explain scope of impact before changing code.

## Key References

- `docs/README.md` — full documentation index
- `docs/roadmap.md` + `memory/project_*.md` — live multi-PR plan & round hand-off (this project uses these, **not** `IMPLEMENTATION_PLAN.md`)
- `references/` — cloned IMEs (azooKey, librime, khiin-rs, McBopomofo, florisboard, …); enter via the comparison doc above, not directly
- Android IME: follow [Creating an Input Method](https://developer.android.com/develop/ui/views/touch-and-input/creating-input-method)
