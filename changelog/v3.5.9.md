## v3.5.9

Headline release: **TPS 三索引 (TPS three-index, first-class continuous input)** + **POJ continuous first-class** + **v3.5.9 engine maintainability refactor (Tier A + Tier B)**. User-visible surface is a small batch of TPS / POJ continuous-input correctness fixes; the bulk of the diff is engine-internal refactoring that keeps `FetchAtPos` golden-frozen while making the codebase easier to extend.

### Shared (iOS + Android)

#### New Features

- **TPS 三索引 (TPS three-index) — continuous input now first-class for TPS.** TPS joins TL/POJ as a peer in the lexicon read path: dedicated `tps:` key family (toneless / abbrev / variant indices), build-time `er ↔ or` dual-emit so dialectal forms hit the same row id, and the continuous walker / lattice / shadow / commit-key pipeline treat TPS identically to TL/POJ. Typing a whole TPS sentence now produces span-local candidates + min-cost segmentation; partial-prefix and ranking behave the same as TL/POJ. (#333–#340)
- **POJ continuous input first-class.** POJ now emits its own syllable inventory + tagged single FST and is mode-aware end to end; the user-history commit key falls back to canonical TL so POJ and TL hits share the same boost. Continuous-input phrases in POJ mode are no longer downgraded to TL display, and POJ-mode boosts persist correctly across input-mode switches. (#308–#311)

#### Bug Fixes

- **TPS toneless segmentation for medial-led syllables.** Toneless TPS strings whose syllables start with a medial (e.g. `ㄉㄞㄨㄢ` for 台灣, `ㄉㄞㄨㄢㄉㄞㆣㄧ` for 台灣台語) no longer return zero candidates — the segmenter was rewritten as inv-driven BFS mirroring TL, so medial-led syllables segment correctly across all three families. (#344)
- **TPS auto-correct boundary for nasal-coda + nasalized-vowel finals.** Typing `ㄉㄞㄨㄢㄉ…` no longer corrupts the second `ㄉ` to entering-tone `ㆵ` after the precomposed nasal coda `ㄢ`. Five precomposed nasal-coda compound finals (am/an/ang/om/ong) and seven nasalized-vowel finals are now syllable boundaries; `huann-hi` (歡喜) and other cross-syllable `ㄏ`-initial words still segment correctly. (#348)
- **TPS partial-prefix activated for leading initial.** Typing a lone Bopomofo initial like `ㄉ` now returns prefix-matched candidates via `prefix_index.n("tps:ㄉ")` instead of zero results; aligns TPS with TL / POJ / English partial-prefix behavior and matches librime / khiin-rs / McBopomofo leading-prefix UX. (#347)
- **Partial-prefix cap no longer evicts high-frequency short candidates.** Short common words can no longer be silently dropped from the partial-prefix candidate list under cap pressure. (#314)
- **Android ANR fix in `PrefHelper`.** Cache hoisted to the companion to remove the ANR locus on cold-start preference reads. (#316)
- **Android release-build logger compliance.** `LoggerBackend.e()` is now debug-only — release builds no longer risk emitting IME-adjacent text to logcat (security-rules zero-logs compliance, follow-up to the v3.5.8 ProGuard hardening). (already shipped via v3.5.8 hotfix #294, kept here for completeness.)

### iOS

#### Bug Fixes

- Disambiguated `CGFloat.init` reference inside the new typed `SettingsKey<T>` descriptors so Swift type inference picks the intended overload at every call site.

#### Refactoring

- iOS `RustEngineBridge` god-file split by slice (B3, #324) — phonetics / composing extensions live in their own files, mirroring the engine slice boundaries.
- `ComposingManager` migrated to Swift's `@Observable` macro (B9, #328) — replaces the legacy `@Published` / `ObservableObject` wiring on the keyboard-extension side.
- TPS state machine replaces the `SharedSettings` cross-setter cascade (B8a, #319 + #322) — settings dependencies now flow through typed `SettingsKey<T>` descriptors instead of raw-key boilerplate.
- iOS-side autocomplete service renamed `AutocompleteService → TaigiAutocompleteService` (#338) so the Taigi vs English service pair reads symmetrically.

### Android

#### Bug Fixes

- ANR locus in `PrefHelper` removed by hoisting the cache to the companion (#316).

#### Refactoring

- `TextInputManager` split into focused helpers + `TextInputKeyHandler` + `KeyboardUiCoordinator` (B5, #330 / #331 / #332).
- Android `RustEngineBridge` god-file split by slice (B4, #325) — mirrors the iOS split.
- `ComposingManager` caches migrated to `StateFlow` (B9, #329).
- `TpsCascade` becomes a pure state machine with atomic batch persist (B8a Android half, #323).
- `PrefHelper` gains a `PreferenceProperty` delegate (B6, #321) — settings reads are now property-style at call sites.
- `SmartbarContainer` enum replaces raw `R.id` Int state (B7, #320).
- 22 raw `android.util.Log` sites routed through `LoggerBackend` (B12, #317) so all logging respects the shared zero-logs-in-release rule.
- Compose compiler stability config wired in + opt-in stability reports (B10 step 1, #318); `TaigiWord` declared stable so the smartbar candidate strip skips unnecessary recomposition (B10 step 2, #326).

### Engine (Rust shared core)

The v3.5.9 round is a refactor-freeze release for the engine. Every change is gated by a golden `FetchAtPos` snapshot harness (S0, #301) so the public observable behavior is byte-identical to v3.5.8 except for the user-facing fixes above.

- **Tier A — composing-crate maintainability** — `composing::shadow` extracted (A1, #302); `composing::continuous` seam extracted with D1 / D3 fold-ins (A2, #303); walker constants documented as a cluster header (A3, #305); `CORPUS_TOTAL_FREQ` artifact-handshake guard added (A4, #304).
- **Tier B — POJ first-class** — `is_poj: bool` → `phonetics::InputMode` sweep (B-0c, #307); POJ syllable inventory + tagged single FST + mode-aware `contains_in` (B-1, #308); POJ first-class key emission + mode-aware lattice / guard (B-2, #309); canonical TL fallback for the user-history commit key (B-3 / B-4, #310); doc sweep (B-7, #311).
- **D7 / D8** — `ContinuousFetchCtx` 8-arg signature collapsed to a struct; `endings` flagged `#[doc(hidden)]` (#306).
- **Tier D / C-series — TPS three-index** — TPS three-index build pipeline (C-0, #334); TPS read path routed to `tps:` family (C-1, #335); TPS `er ↔ or` build-time dual-emit (C-3a, #336); TPS continuous first-class (C-3b, #337); engine renamed `render_roman_for_mode → recase_tl_as_poj_display` (C-4, #339); TPS golden + parity test coverage (C-5 / D capstone, #340).
- **TPS toneless / nasal-coda / partial-prefix fixes** — see "Bug Fixes" above (#344 / #347 / #348).
- Generated proto + xcframework artifacts resynced post-D7 / D8.

### Dictionary

- Build pipeline gains a TPS three-index stage: `tps_notone`, `tps_abbrev`, `tps_abbrev_var`, plus `tps_notone_var` produced by the `er ↔ or` dual-emit pass. All TPS columns are derived from canonical TL via `taigi_bridge`, so source CSVs stay TL-only.
- `dictionary/tests/test_tps_derivations.py` pins the derivation rules.
- `corpus_total_freq.txt` emitted alongside `dictionary.bin` so the engine's `CORPUS_TOTAL_FREQ` handshake (A4) can fail-fast on artifact drift.
- Dictionary regenerated via `make dict` at release.

### Build / Tooling

- **Round-speedup (PR #345 + #346)** — typical refactor round wall-clock cut ~77% via:
  - `make test-fast` (workspace nextest), `make test-crate CRATE=<name>`, `make test-crate-fast CRATE=<name>` for touched-target test invocations.
  - `make fmt-fast` / `make fmt-check-fast` / `make lint-rust-fast` — skip Gradle / Spotless JVM cold start and `--all-targets` for in-round checks. Canonical `make fmt` / `make lint` / `make test` stay byte-identical.
  - `engine/Cargo.toml` `[profile.dev] debug = "line-tables-only"` — cuts host link wall by dropping full DWARF. Override via `CARGO_PROFILE_DEV_DEBUG=2`.
  - `rust-best-practices.md §7` 4-cmd pre-PR gate reclassified judgment-gated; **commit-first ordering** codified — tests run post-PR in background, parallel to PR-bot review, not as a pre-commit gate.
- **Project rules migrated `rules/ → .claude/rules/`** with `paths:` auto-load frontmatter (Tier A Step 3, #343). 12 of 15 rules now auto-load only when Claude reads matching files; 3 stay always-on (`security-rules.md`, `doc-lookup.md`, `taigi-incidents.md`).
- `AGENTS.md` symlink → `CLAUDE.md` so Codex CLI and Claude Code share the same project conventions.
- `rustfmt 1.95` drift cleared workspace-wide (#342).

### Documentation

- v3.5.8 plan archived to `docs/releases/v3.5.8/plan.md` (was `docs/roadmap.md`).
- Engine spec split into focused files: `continuous-commit-and-display.md` (Model B commit / display contract, §10 extract), `continuous-lexicon-fallback-retire.md` (Item 13 capstone, §15 extract), `keyboard-body-invariants-android.md` (§16 extract from `behavioral-invariants.md`).
- `docs/perf/android-compose-stability.md` — B10 step 2 observability gate re-run procedure.
- `docs/reports/2026-05-20-triple-index-eval.md` + `docs/reports/2026-05-20-v359-b-plan.md` — TPS three-index evaluation + v3.5.9 Tier B plan.

### Removed

- `engine/composing/tests/syllabifier_tps.rs` — replaced by inv-driven BFS coverage in `syllabifier_tl.rs` + new TPS golden cases (#344).
- `ios/Sources/TaigiKeyboard/Settings/TPSSyncCoordinator.swift` — replaced by the new `SettingsKey<T>` typed-descriptor wiring.

### New Files

- Engine: `engine/composing/src/shadow.rs`, `engine/composing/src/continuous.rs`, `engine/composing/tests/golden/fetch_at_pos.golden`, `engine/composing/tests/golden_fetch_at_pos.rs`, `engine/lexicon/tests/{tps_abbrev_parity,tps_notone_parity,poj_notone_parity}.rs`.
- Android: `engine/{ComposingBridge,NextWordBridge,PhoneticsBridge}.kt`, `ime/core/TpsCascade.kt`, `ime/text/{EditorInfoClassifier,KeyboardAppearanceResolver}.kt`, `ime/text/keyboard/{ImeKeyEventDispatcher,KeyboardUiCoordinator,TextInputKeyHandler}.kt`, `ime/text/smartbar/SmartbarContainer.kt`, `app/compose_compiler_config.conf`.
- iOS: `Engine/RustEngineBridge+{Composing,Phonetics}.swift`, `Engine/{RustVec+UInt8,SwiftLoggerSink}.swift`, `Settings/SettingsKey.swift`, test suites `SettingsKeyTests.swift` / `SharedSettingsTests.swift`.
- Dictionary: `dictionary/output/corpus_total_freq.txt`, `dictionary/tests/test_tps_derivations.py`.
- Docs: `docs/perf/android-compose-stability.md`, `docs/engine/{continuous-commit-and-display,continuous-lexicon-fallback-retire}.md`, `docs/architecture/keyboard-body-invariants-android.md`, `docs/releases/v3.5.8/plan.md`, two May-2026 report files under `docs/reports/`.
- `AGENTS.md` (symlink → `CLAUDE.md`).
