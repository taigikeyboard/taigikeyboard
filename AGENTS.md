# AGENTS.md

Working instructions for coding agents — and for humans — in this repository. Tool-specific additions live in each tool's own file (`CLAUDE.md` for Claude Code). Licensing, secrets and commit conventions: [`CONTRIBUTING.md`](CONTRIBUTING.md).

**TaigiKeyboard** — cross-platform Taiwanese input method: iOS (Swift + KeyboardKit), Android (Kotlin + FlorisBoard), macOS (IMKit), Windows (TSF, Rust), Linux (Fcitx5 + IBus, Rust) over a shared Rust engine. POJ/TL/TPS romanization, Hanji, tone variation, autocomplete, continuous input.

## Project Structure

```
android/  ios/  macos/  windows/  linux/   # platform apps
                   # macos/ links one Rust archive, macos/crates/taigi-macos-ffi (engine seam + taigi-desktop-core)
engine/            # Shared Rust engine — Cargo workspace, FFI to every platform
desktop/           # Rust crates shared by the desktops — taigi-desktop-core (all three), -storage (Windows + Linux), -update (Windows)
dictionary/        # Dictionary sources + build pipeline + output artifacts
assets/            # Committed shipped data every platform packages — dictionaries/ (pipeline output), fonts/, symbols/
docs/              # engine/, architecture/, contributing/, phonetics/ (TL/POJ/TPS reference), ui/, references/, reports/, roadmap.md
taigi-converter/   # Canonical TL↔POJ↔TPS converter (git submodule)
tools/             # Dev tooling — test selection, i18n codegen, release/, secret-scan/
e2e/               # End-to-end typing tests — scenarios/, analyzer/, drivers/
corpus/            # Real Taiwanese text — taigi-typing (manual-test sentences) + taigi-corpus (opt-in, future LM data); never a build input
changelog/         # Per-release changelogs — edit only at release time
references/        # Cloned external IME repos (gitignored)
emoji/             # Emoji data generator (own instructions file)
```

## Core Principles

1. **Xcode project files are maintainer-only** — agents never edit `.xcodeproj` / `.pbxproj` / `.xcworkspace`. Xcode synchronized groups auto-include new files under most `Sources/TaigiKeyboard/*` subdirs — exceptions in `docs/contributing/ios-guidelines.md`. Android Gradle files are editable.
2. **Cross-platform alignment** — align on **intended behavior**, not API calls; verify each platform independently; document when the same behavior needs different implementations (`docs/contributing/cross-platform-alignment.md`).
3. **Phonetics = authoritative-source-only** — never infer TL/POJ/TPS rules from test/dictionary absence; read `docs/phonetics/taigi-phonetics-reference.md` and `taigi-converter/` first (`docs/contributing/phonetics.md`).
4. **Bugfix = root cause first** — reproduce the failure and pin the cause to `file:line` before writing the fix; the PR states both.
5. **Direction-first over fix-scope** — between two correct fixes prefer consistency and correct architectural direction over the smaller diff; state the trade-off.
6. **Word identity = (漢字, canonical-TL) pair** — neither alone is a key (`重/tîng` ≠ `重/tāng`). Governs every dedup / lookup / ranking-merge / variant decision project-wide. POJ/TPS are alternate renderings of the same TL reading.

## Design principles

- **No redundant fallback** — "A fails → fall back to B" is an anti-pattern for primary data flow (fetch, query, transform, dispatch). Push B's capability into A and keep the flow one-directional. Defensive guards (boundary checks, empty-state defaults) are not fallbacks; a temporary compatibility fallback says "temporary, remove after X" where it is introduced.
- **Shared logic lives in the engine** — behavior every platform needs is written once in Rust (`docs/contributing/rust-migration-policy.md`); platforms keep UI and OS integration.

## Read before…

| Before… | Read |
|---|---|
| your first change | `docs/contributing/known-pitfalls.md` |
| changing files on a platform | that platform's guide in `docs/contributing/` (index: `docs/contributing/README.md`) |
| adding / renaming / deleting an `i18n/*.json` key from platform code | `docs/contributing/i18n.md` |
| adding or changing a framework / OS API call (KeyboardKit, `UITextDocumentProxy`, `InputMethodService`, Compose, DataStore) | `docs/contributing/doc-lookup.md` — verify against current docs, never from memory |
| a real-device dogfood pass, or citing an `Sn` item | `docs/architecture/dogfood-checklist.md`; draw test sentences from `corpus/README.md` |
| a best-practices section, or any segmentation / lattice / ranking / next-word / continuous-input design | `docs/references/mainstream-ime-comparison.md` — never re-explore `references/` from scratch |

## Build & Test

Full setup: [`docs/BUILDING.md`](docs/BUILDING.md). **Bootstrap**: clone with `--recurse-submodules`, then `make build` once per machine — generates the xcframeworks and `jniLibs/*.so` the app builds link (not committed) and regenerates the committed platform protos.

**Test what you touched**: `python3 tools/test_select.py --run --platform <p>` per touched platform (no args = print the selection; `make test-changed` runs the whole selection). The full suites below are for release prep or a deliberate catch-net. `make lint` runs every formatter / linter gate CI enforces.

| Platform | Build | Test |
|---|---|---|
| engine | `cargo build --workspace` | `cargo test --workspace` |
| iOS | Xcode → keyboard extension | `xcodebuild -project ios/TaigiKeyboard.xcodeproj -scheme TaigiKeyboardTests -destination 'platform=iOS Simulator,name=<device>' test` |
| Android | `android/gradlew -p android :app:assembleDebug` | `android/gradlew -p android :app:testDebugUnitTest` |
| macOS | `make -C macos build` (`install` before dogfood) | `make -C macos test` |
| Windows | `make windows-check` (host gate; TSF DLL builds only on Windows — `docs/architecture/windows-release.md`) | included |
| Linux | `make linux-check` (host gate; `.deb` via `make -C linux deb` — `docs/architecture/linux-release.md`) | included |
| taigi-converter | — | `npm test` in `taigi-converter/` |

**Stale-artifact gate — run before every iOS / Android / macOS build+test** (stale binaries give false-green tests):

| Diff touches… | Run first |
|---|---|
| `engine/` (Rust, `.proto`, `Cargo.toml`) | `make build` |
| `dictionary/` | `make dict` then `make build` |
| `desktop/`, `macos/crates/`, `macos/Cargo.*` (the archive macOS links) | `make build` before a macOS build or test |
| platform-only Swift / Kotlin / docs | nothing |

Details and timings: `docs/architecture/build-artifacts.md`.

## Conventions

- Code, comments and docs in **English**. Living docs name UI labels by their i18n `en` value. CJK only for Taigi content (phonetic / Hanji examples, test data, proper names of dictionaries and fonts). Dated snapshots (`docs/reports/**`, `docs/releases/**`) are frozen.
- Conventional Commits `type(scope): subject`; one PR per logical change; say which platforms you built and tested.
- `changelog/` is edited only at release time.
- No personal identifiers in tracked files, commits or PRs (`docs/contributing/known-pitfalls.md` § Privacy).

## References

- `docs/architecture/system-overview.md` — read first for architecture
- `docs/README.md` — documentation index · `docs/roadmap.md` — live multi-PR plans
- `#NNN` written before 2026-09-07 = old repository; resolve via `git log --all --oneline --grep="(#NNN)"` (`docs/architecture/pr-number-migration.md`)
