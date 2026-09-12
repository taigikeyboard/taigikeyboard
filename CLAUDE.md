# CLAUDE.md

**Taigi Keyboard** — cross-platform Taiwanese input method: iOS (Swift + KeyboardKit), Android (Kotlin + FlorisBoard), macOS (IMKit), Windows (TSF, Rust) over a shared Rust engine. POJ/TL/TPS romanization, Hanji, tone variation, autocomplete, continuous input.

Rule layers:
- **Cross-project process rules** — `~/.claude/rules/` (from the [`configurations`](https://github.com/siansiansu/configurations) dotfiles repo; run its `setup.sh` on a fresh machine).
- **Project rules** — `.claude/rules/`; most auto-load via `paths:` glob when a matching file is read. Always-on: `security-rules.md`, `doc-lookup.md`, `taigi-incidents.md`.

## Project Structure

```
android/  ios/  macos/  windows/   # platform apps
engine/            # Shared Rust engine — Cargo workspace, FFI to every platform
dictionary/        # Dictionary sources + build pipeline + output artifacts
docs/              # engine/, architecture/, ui/, references/, reports/, roadmap.md
knowledge/         # Taiwanese phonetics reference (TL/POJ/TPS)
taigi-converter/   # Canonical TL↔POJ↔TPS converter (git submodule)
changelog/         # Per-release changelogs — edit only at release time
references/        # Cloned external IME repos (gitignored)
```

## Core Principles

1. **Project config is user-only** — `.xcodeproj` / `.pbxproj` / `.xcworkspace` never edited by AI (enforced by `.claude/hooks/block-project-config.sh`). Xcode synchronized groups auto-include new files under most `Sources/TaigiKeyboard/*` subdirs — exceptions in `.claude/rules/ios-guidelines.md`. Android Gradle files are editable.
2. **Cross-platform alignment** — align on **intended behavior**, not API calls; verify each platform independently; document when the same behavior needs different implementations.
3. **Phonetics = authoritative-source-only** — never infer TL/POJ/TPS rules from test/dictionary absence; read `knowledge/taigi-phonetics-reference.md` and `taigi-converter/` first (`.claude/rules/phonetics.md`).
4. **Bugfix = confirm root cause, then wait for approval** before any branch or edit. Release scope / timing / tag = user-gated. Both per `~/.claude/rules/diagnosis-discipline.md`.
5. **Direction-first over fix-scope** — between two correct fixes prefer consistency and correct architectural direction over the smaller diff; state the trade-off. (USER 2026-05-25: 「考慮一致性、最佳實踐，方向正確會比較重要」)
6. **Word identity = (漢字, canonical-TL) pair** — neither alone is a key (`重/tîng` ≠ `重/tāng`). Governs every dedup / lookup / ranking-merge / variant decision project-wide. POJ/TPS are alternate renderings of the same TL reading. (USER 2026-05-29: 「(羅馬字 + 漢字)的組合視為同一個字」)

## Read before…

Path-scoped rules load themselves; these do not:

| Before… | Read |
|---|---|
| adding / renaming / deleting an `i18n/*.json` key from platform code | `.claude/rules/i18n.md` |
| a real-device dogfood pass, or citing an `Sn` item | `docs/architecture/dogfood-checklist.md` |
| a 最佳實踐對齊 section, or any segmentation / lattice / ranking / next-word / continuous-input design | `docs/references/mainstream-ime-comparison.md` — never re-explore `references/` from scratch |
| a framework/OS API call (KeyboardKit, `UITextDocumentProxy`, `InputMethodService`, Compose, DataStore) | `.claude/rules/doc-lookup.md` — verify via `find-docs` first, never from model memory |

## Build & Test

**Commit-first ordering**: commit → push → `gh pr create`, no pre-commit test gate. Post-PR verification (`~/.claude/rules/round-workflow.md` § Codex review sandwich step 6) runs build+test for every touched platform in parallel, in the background, after the PR URL returns.

| Platform | Build | Test |
|---|---|---|
| engine | `cargo build --workspace` | `cargo test --workspace` |
| iOS | Xcode → keyboard extension | `xcodebuild -project ios/TaigiKeyboard.xcodeproj -scheme TaigiKeyboardTests -destination 'platform=iOS Simulator,id=81ADB050-5242-460C-90DA-F3FAF3F6AAA5' test` (iPhone 17 / iOS 26.1) |
| Android | `android/gradlew -p android :app:assembleDebug` | `android/gradlew -p android :app:testDebugUnitTest` |
| macOS | `make -C macos build` (`install` before dogfood) | `make -C macos test` |
| Windows | `make windows-check` (host gate; TSF DLL builds only on the Windows box — `docs/architecture/windows-release.md`) | included |
| taigi-converter | — | `npm test` in `taigi-converter/` |

**Bootstrap**: clone with `--recurse-submodules`, then `make build` once per machine — generates the xcframeworks, `jniLibs/*.so` and platform protos the app builds link (not committed).

**Stale-artifact gate — run before every iOS/Android build+test** (stale binaries give false-green tests):

| Diff touches… | Run first |
|---|---|
| `engine/` (Rust, `.proto`, `Cargo.toml`) | `make build` |
| `dictionary/` | `make dict` then `make build` |
| platform-only Swift / Kotlin / docs | nothing |

Details and timings: `docs/architecture/build-artifacts.md`.

## Communication

- Reply in **Taiwanese Mandarin (台灣華語)**; code, comments and docs in **English**. Living reference docs use English prose; CJK only for verbatim USER quotes and phonetic examples. Dated snapshots (`docs/reports/**`, `docs/releases/**`) are frozen.
- Recommendation first, then ranked options with one-line trade-offs, before changing code.

## References

- `docs/architecture/system-overview.md` — read first for architecture; LSP plugins (rust-analyzer / swift / kotlin) installed — prefer go-to-definition / references over grep+Read chains
- `docs/README.md` — documentation index · `docs/roadmap.md` + project memory — live multi-PR plan and round hand-off
- `#NNN` written before 2026-09-07 = old repository; resolve via `git log --all --oneline --grep="(#NNN)"` (`docs/architecture/pr-number-migration.md`)
