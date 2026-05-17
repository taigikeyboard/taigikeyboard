# CLAUDE.md

Guidelines for **Claude Code** in this repo. Generic agent discipline — incremental progress, confirm-before-acting, YAGNI, reasoning depth, subagent use, Codex usage, commit authorship — lives in global `~/.claude/CLAUDE.md` and is **not duplicated here**. This file holds only what is specific to Taigi Keyboard.

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

1. **No project-config modification by AI** — `.xcodeproj` / `.pbxproj` are **user-only** (manual Xcode edits; new iOS files need the user to add the Xcode target). Android Gradle (`build.gradle`, `*.gradle.kts`) **is** editable by Claude.
2. **Cross-platform alignment** — align on **intended behavior**, not API calls: define expected behavior, verify each platform independently, document when the same behavior needs different implementations.
3. **Phonetics = authoritative-source-only** — never infer TL/POJ/TPS rules (or "dead" phonetic tables from test/dictionary absence); read `knowledge/taigi-phonetics-reference.md` and consult `taigi-converter/` first.

## Mandatory Rules

Read the listed file **before** the matching work — these override defaults.

| Before… | Read |
|---|---|
| any non-trivial task | `rules/claude-workflow.md` |
| writing/modifying any code | `rules/ai-friendly-code.md` |
| reviewing code | `rules/code-review-rules.md` |
| a change affecting iOS/Android parity | `rules/cross-platform-alignment.md` |
| modifying iOS code (structural → also architecture) | `rules/ios-guidelines.md` (+ `ios-architecture.md`) |
| modifying Android code | `rules/android-guidelines.md` |
| modifying app UI | `rules/ui-style-guide.md` |
| adding logging / SQL / network / storage | `rules/security-rules.md` |
| Rust engine code or FFI/proto docs | `rules/rust-best-practices.md` |
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
