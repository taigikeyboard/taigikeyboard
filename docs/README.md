# TaigiKeyboard - Technical Specifications

> **Type**: Index
> **Keywords**: `spec`, `index`, `specification`

---

## Summary

- New contributor: build and test setup is in `BUILDING.md`
- Cross-platform reference for iOS / Android / macOS / Windows / Linux implementation alignment — start at `architecture/system-overview.md`
- Style guides and per-platform rules: `contributing/README.md`

---

## Document Index

### `engine/` — IME Core Logic

| File | Description | Status |
|------|-------------|--------|
| `engine/migration-inventory.csv` | Rust slice inventory: every shipped pub item + native pending/keep/wont-migrate (146 rows, 13 cols, zh-TW descriptions) | Canonical |
| `engine/binary-format.md` | `dictionary.fst` + `association.bin` binary spec (mmap-friendly, byte-exact cross-platform) | Active |
| `engine/rust-core-proto.md` | Engine proto envelope + per-slice request/response shapes | Active |
| `engine/composing.md` | Composing state machine (`Phase::Idle` / `Continuous { raw, caret, nailed }`) — Rust `engine/composing` | Active |
| `engine/continuous-input-ranking.md` | Continuous-input ranking design record (v3.5.8): lexicographic CandidateSortKey + user_freq_boost + recency_rank; the code's module docs win where they differ | Reference |
| `engine/continuous-candidate-display.md` | Continuous candidate dual-line display (roman + hanji) spec — §4 carrier shipped (Items 5+6); §15 fallback retire DONE (Item 13) | Active |
| `engine/continuous-commit-and-display.md` | Continuous mode commit/display contract (Model B) — extracted from `continuous-input-ranking.md` §10 | Active |
| `engine/tone.md` | Tone conversion / restoration / nasal-marker — Rust `engine/phonetics` | Active |
| `engine/sort.md` | Candidate ranking — Rust `engine/ranking` | Active |
| `engine/nextword.md` | Next-word prediction — Rust `engine/nextword` | Active |
| `engine/tps.md` | TPS Taiwanese Phonetic Symbols (方音符號) — Rust `engine/phonetics::tps` | Active |
| `engine/custom-dictionary.md` | User-defined dictionary (CRUD, CSV import/export) — engine-owned SQLite (`engine/userdata`) | Active |
| `engine/diagnostics.md` | Device-info collection for bug reporting | Active |
| `engine/syllabifier.md` | Syllable parse primitive + `SyllableInventory` (`syllables.fst`) + lattice consumption | Active |

### `architecture/` — Architectural Contracts

| File | Description | Status |
|------|-------------|--------|
| `architecture/system-overview.md` | Five-platform architecture entry point: system context, engine crate graph, build pipeline, keystroke flow + per-platform glue chain | Active |
| `architecture/behavioral-invariants.md` | Cross-platform behavior contract (every Rust slice must preserve) | Active |
| `architecture/dogfood-checklist.md` | Real-device acceptance items `Sn` (type X → expect Y, pins `INVARIANT_*`, per-item `Status` line) — read before a dogfood pass | Active |
| `architecture/incident-log.md` | Dated incident narratives behind the rules in `docs/contributing/known-pitfalls.md` (append-only) | Reference |
| `architecture/build-artifacts.md` | What is committed (`assets/dictionaries/`, `assets/fonts/font/`) vs generated (`make build`), why, measured timings, release-rebuild rule | Active |
| `architecture/pr-number-migration.md` | Resolving pre-2026-09-07 `#NNN` PR numbers (archive repository, numbering restarted at #1) | Reference |
| `architecture/composing-state-boundary.md` | Composing engine ↔ platform binding contract (§2.2 Effect table, ordering, §11 Android `InputConnection` binding); section numbers frozen | Reference |
| `architecture/nextword-engine-boundary.md` | NextWord engine ↔ platform binding contract (generation, decay, timer, §13 Android binding); section numbers frozen | Reference |
| `architecture/keyboard-body-invariants-android.md` | Android Compose keyboard-body geometry / touch-target invariants (refactor-freeze contract) | Active |
| `architecture/data-artifacts-portability.md` | Binary artifacts + SQLite portability contract (iOS / Android §1–7, macOS / Windows stores §8, decision register) | Active |
| `architecture/macos-roadmap.md` | macOS desktop IME (InputMethodKit over the shared engine) — design decisions D1–D11 incl. candidate-window port, PR table; shipped desktop v3.6.7/v3.6.8 | Reference |
| `architecture/manual-release-notes.md` | Canonical English What's New, validation, and manual store paste workflow | Active |
| `architecture/desktop-release.md` | How a desktop version reaches a user: staged on a draft, tested, published by hand, announced automatically | Active |
| `architecture/macos-release.md` | macOS Developer ID signing, notarization, and what only a Mac asserts about its `.pkg` | Active |
| `architecture/windows-roadmap.md` | Windows desktop IME (TSF in Rust over the shared engine, macOS UX parity) — design W1–W17, PR table, reference alignment, dogfood run-book; shipped desktop v3.6.7/v3.6.8 | Reference |
| `architecture/linux-roadmap.md` | Linux desktop IME (Fcitx5 addon primary + IBus engine second over one Rust core, GTK 4 / libadwaita settings window over the shared `desktop/` crates) — design L1–L13, PR table, named divergences, dogfood run-book; PR0–PR9 merged, device dogfood pending | Done |
| `architecture/e2e-testing-roadmap.md` | End-to-end test system — AI-driven simulator / emulator / VM / container runs, test-build-only JSONL trace, analyzer for bugs + perf, per-platform drivers, capability spike results, PR table; Linux live (`make e2e PLATFORM=linux` / `linux-desktop`), macOS / Windows / mobile manual | Active |
| `architecture/user-data-engine-roadmap.md` | The four user-data SQLite stores moved from four platform implementations into one engine crate (`engine/userdata`) — audit of today's stores and their drift, design U1–U10, PR table, reference alignment | Done |
| `architecture/bigram-lm-roadmap.md` | Cross-word bigram language model from `corpus/taigi-corpus` — today's unigram walker + intra-word association table, corpus alignment survey, design D1–D7 (`association.bin` v2 word keys, next-word backoff, composing re-rank, gated walker term), phases P0–P8, reference survey of every bigram implementation in `references/`, corpus-expansion survey (ten more Taiwanese sources + the 意傳 aligner); P0–P5 merged, P6/P7 not adopted | Closed (2026-09-30) |
| `architecture/maintainability-roadmap.md` | Follow-up of the 2026-09-30 maintainability audit — mandate (settings may change, user data never lost), PR table R1–R12 + docs drift, decisions recorded while running; complete 2026-10-01 | Done |
| `architecture/maintainability-roadmap-2026-10.md` | Follow-up of the 2026-10-09 maintainability audit — mandate, phase table H + A–G (dead wire, mobile custom-dictionary parity, one `DictionarySearch` op, prediction taps + TPS keys into the engine, engine cohesion, desktop-core twins, mobile layering), best-practices alignment, not-adopted list; nothing started | Proposed |
| `architecture/macos-desktop-core-roadmap.md` | macOS links the shared Rust `taigi-desktop-core` for its key path instead of re-implementing it in Swift — scope from the measured inventory, one static library, the bytes seam, `DesktopPlatform`, parity decisions, phases P0–P15, the rule that replaces "macOS is the behaviour oracle" | Planning |
| `architecture/e2e-trace-schema.md` | Test-build-only JSONL trace contract — how it stays out of release, `trace_open` / `engine_request` / `engine_panic` / `adapter_reject` events | Reference |
| `architecture/linux-release.md` | The Linux half of a desktop release: the `.deb` / `.rpm` / Arch packages (both shells, dictionaries, settings window), how `make -C linux deb` and `linux-build.yml` build and attach them, no in-app update | Reference |
| `architecture/windows-release.md` | Windows installer (Inno Setup), Authenticode signing, and web-distributed installer workflow | Active |
| `architecture/ios-exemplar.md` | Cross-platform architectural pattern (layers, DI, live-read settings, markers) + §9 Android deviations | Reference |

### `ui/` — Presentation & Layout (7)

| File | Description | Status |
|------|-------------|--------|
| `ui/layout.md` | Keyboard layout definitions and conversion | Active |
| `ui/case.md` | Case handling (Shift, CapsLock, transformation) | Active |
| `ui/device.md` | Device adaptation for iPhone and iPad | Active |
| `ui/app-ui.md` | Main App UI structure (tabs, settings) | Active |
| `ui/theme.md` | Theme & styling (colors, fonts, user customization) | Active |
| `ui/emoji.md` | Emoji keyboard (ISEmojiView iOS / Compose Android, `emoji/` data) | Active |
| `ui/callouts.md` | Long-press callouts + tone-variation menus (engine map + platform popups) | Active |

### `references/` — External IME Research (7)

| File | Description | Status |
|------|-------------|--------|
| `references/mainstream-ime-comparison.md` | IME comparison index (TL;DR matrix + per-repo cards) — entry point for best-practice cites | Reference |
| `references/azookey-reference.md` | azooKey research (SwiftUI, Flick, CustardKit) | Reference |
| `references/chiakey-reference.md` | ChiaKey research (lexicon release contract, Runtime/Engine facade, learning store, IME↔helper coordination, updater, release workflow) | Reference |
| `references/khiin-reference.md` | khiin-rs research (DPSegment, Bigram, Trie) | Reference |
| `references/moe-taigi-reference.md` | MOE Taigi IME analysis (Segmentation, Nail) | Reference |
| `references/rime-reference.md` | librime research (Pipeline, DAG, SpellingAlgebra) | Reference |
| `references/keywords.md` | Standardized keyword mapping for core logic and UI | Reference |

### `reports/` — Audit & Analysis Reports (historical)

One-off snapshots ordered chronologically. A report stays while live code cites it, it holds design rationale found nowhere else, or its research is still open; otherwise it is deleted once its facts live in `architecture/behavioral-invariants.md`, an `engine/` spec or a roadmap (git history keeps the text).

| File | Description | Status |
|------|-------------|--------|
| `reports/2026-08-30-hanlo-together-mode-research.md` | Candidate Display picker research: Hanji and romanization side by side (default, title/subtitle) / Hanji with Romanization (one-label hanji+roman, formerly "Hanji and romanization together"; Part I) / Romanization Only (roman-only cells in today's candidate UI, all 4 platforms; Part II — 3-column + Taigi spelling correction considered and dropped, kept as future correction reference) — the picker shipped (all platforms); Taigi spelling correction stays open research | Historical |
| `reports/desktop-3.6.x-design-notes.md` | Frozen design bodies of the seven desktop 3.6.8 sections collapsed out of `roadmap.md` (installed typefaces, Telex, symbol picker, composing caret, ⇧+slot, Shortcuts pane, custom fonts) | Historical |
| `reports/2026-09-11-windows-candidate-window-paint-latency.md` | Windows candidate window frame-before-content latency: measured on the box (2-5 ms steady, one 171 ms first-show in Chrome), mechanism (in-proc `ShowWindow` before `WM_PAINT`), what mozc / PIME / khiin do, options B (in-proc sync paint) / C (renderer process) costed — evaluation only, nothing decided | Plan |
| `reports/2026-09-21-taile-mode-tone-commit.md` | TL mode — tone key commits without a candidate window (MOE Mac IME parity): why marked text stays, desktop-only Enter cost under Candidate Display = Romanization Only, options A (Space commits when `.ignored`) / B (real TL mode) costed, open USER decisions — research only, low priority (USER 2026-09-21) | Plan |
| `reports/2026-09-24-mobile-smart-suggestions-brainstorm.md` | Mobile smart suggestions brainstorm: today's next-word = dictionary-word completion keyed on the last character (gaps ranked), references survey (mozc / McBopomofo / vChewing / librime-predict / ChiaKey / azooKey) ranked for Taigi, engine + mobile maintainability audit (dead ops, Swift/Kotlin prediction-assembly duplication), draft R1 → R3 → batches A/B + corpus track, open USER decisions — R1–R4 and batches A/B delivered (smart suggestions + bigram LM); leftovers listed in `roadmap.md` § Open candidates | Historical |
| `reports/2026-09-24-open-source-readiness-and-layout.md` | Open-source readiness + repository layout audit: community files, fresh-clone blockers (build doc only in CLAUDE.md, macOS-only `make build`, unpinned protoc), privacy scrub, dictionary licence position, four Cargo workspaces / CI coverage gaps, non-reproducible `build_ts`, proposed target tree (platforms stay top-level) + phased rounds 0–10 — rounds 0–9 executed (#170–#335); round 10 closed unbuilt 2026-10-04 (`roadmap.md` § Closed phases); optional items in `roadmap.md` § Open candidates | Historical |
| `reports/2026-09-30-audit-all.md` | Whole-repo maintainability audit (frozen on `30a79c16`): findings, justified-looking layers not to re-flag, draft rounds — executed as `architecture/maintainability-roadmap.md` | Historical |
| `reports/2026-10-09-audit-all.md` | Whole-repo maintainability audit (frozen on `68f2481b`): findings, justified-looking layers, draft rounds, open decisions — planned as `architecture/maintainability-roadmap-2026-10.md` | Historical |
| `reports/2026-10-02-macos-desktop-core-inventory.md` | macOS ↔ `desktop/crates` duplication inventory (frozen on `e330cd30`): per-file Swift ↔ Rust rows with `file:line`, measured LOC (20,389 hand-written Swift, ~8,070 duplicate), every behavioural divergence, Swift tests with no Rust equivalent — the basis of `architecture/macos-desktop-core-roadmap.md` | Historical |
| `reports/2026-09-26-shipped-roadmap-design-notes.md` | Frozen design bodies of the roadmap items collapsed on 2026-09-26 after they merged (identical desktop menus + Linux update check added then removed, learned phrases, mobile custom theme, Telex tone 1 / 4 keys, input-source menu) | Historical |
| `reports/2026-09-28-bigram-corpus-spike.md` | Bigram LM roadmap P1 spike — per-source hanji↔TL alignment of eight `taigi-corpus` sources, pair counts / top-K coverage / `association.bin` v2 size projection, romanized function-word recovery options, dictionary reading-prior accuracy vs iCorpus gold, 意傳 aligner comparison; opens the P2 decision | Historical |

### `releases/` — Per-Release Archives (1)

| File | Description | Status |
|------|-------------|--------|
| `releases/v3.5.8/plan.md` | v3.5.8 continuous-input plan & archive (SHIPPED) | Historical |

### Root — Guides & Planning

| File | Description | Status |
|------|-------------|--------|
| `go-public-checklist.md` | Go-public audit record — repository PUBLIC since 2026-09-07 (flip record, GitHub scanning state), re-run procedure per section, dictionary-licence position still open | Reference |
| `INSTALL.md` | End-user install and setup per platform and Linux distribution | Active |
| `BUILDING.md` | Contributor build guide: host matrix, prerequisites (`mise.toml`), build + test command per platform, stale-artifact rule, troubleshooting | Active |
| `roadmap.md` | Forward-looking work items, released-versions index (mobile + desktop trains), closed phases | Active |
| `CODE_SIGNING_POLICY.md` | Code-signing policy for released binaries (SignPath Foundation requirement) | Active |
| `contributing/` | Per-platform style guides + cross-cutting rules (phonetics, i18n, cross-platform alignment, known pitfalls); index in `contributing/README.md` | Active |
| `phonetics/` | Authoritative TL / POJ / TPS reference (`taigi-phonetics-reference.md`), TPS auto-correct rules + test checklist | Reference |

---

## Naming Conventions

| Category | Pattern | Example |
|----------|---------|---------|
| Engine spec | `engine/{module}.md` | `engine/composing.md` |
| UI spec | `ui/{module}.md` | `ui/layout.md` |
| External reference | `references/{project}-reference.md` | `references/khiin-reference.md` |
| Guide / Planning | `{descriptive-name}.md` (root) | `roadmap.md` |

---

## Format Guidelines

### Document Structure

```markdown
# [Title]

> **Type**: [Feature|Reference|Planning|Index]
> **Keywords**: `keyword1`, `keyword2`
> **Related**: file1.md, file2.md

---

## Summary
- 1-3 sentence core purpose

## Core Concepts
- Bullet point highlights

## Platform Comparison
| Item | iOS | Android |

## Related Files
| File | Description |

## Notes
- Key decisions or pitfalls
```

### Writing Principles

- Bullet points preferred, avoid long paragraphs
- Use English for keywords
- Code snippets should be key fragments only (< 10 lines)
- Keep it concise to reduce token consumption

---

## Feature Module Codes

Authoritative ownership map (Rust crate vs platform glue) — see `engine/migration-inventory.csv` for the full row-level inventory.

| Code | Description | Rust crate | Platform glue (iOS / Android) |
|------|-------------|------------|--------------------------------|
| `Phonetics` | POJ/TL/TPS conversion, normalize, case-transform | `engine/phonetics` | `RustEngineBridge.swift` / `RustEngineBridge.kt` |
| `Composing` | Composing state machine (Phase × Intent → Effect) | `engine/composing` | `ComposingManager.swift` / `ComposingManager.kt` (effect interpreter) |
| `Autocomplete` | Continuous-engine candidate pipeline (single source after v3.5.8 Item 13) | `engine/composing` (Continuous dispatch) + `engine/lexicon` + `engine/ranking` | `TaigiAutocompleteService.swift` / `TaigiAutocompleteService.kt` |
| `Lexicon` | fst prefix index + dictionary/association mmap readers | `engine/lexicon` + `engine/mmap-host` | iOS Tab3 `DictionarySearchService.swift`; Android `LexiconService.kt` (Tab3 + asset lifecycle) |
| `Ranking` | Continuous-input score, source rank, user-frequency boost | `engine/ranking` | via `FetchAtPos` (`RustEngineBridge+Composing`) |
| `Tone` | Tone-mark conversion + nasal-marker | `engine/phonetics` | no direct bridge — applied inside composing ops (`engine/composing`) |
| `CaseTransform` | Per-char + per-string case mapping (POJ/TL aware) | `engine/phonetics::case_transform` | `RustEngineBridge+CaseTransform.swift` / `CaseTransformBridge.kt` |
| `NextWord` | Bigram association lookup + decay scoring + ranking | `engine/nextword` (+ `engine/lexicon::lookup_associations`) | `NextWordController.swift` / `NextWordController.kt` (timer/threading) |
| `UserFrequency` | Per-word usage tracking (count + lastUsed) | `engine/userdata` (read in `FetchAtPos`, written by `RecordUsage`) | mobile `UsageRecorder.swift` / `UsageRecorder.kt` (picks → `RecordUsage`); macOS / Windows / Linux: the engine counts each commit it resolves (R5) |
| ~~`Segmentation`~~ | ~~Syllable segmentation~~ (removed v3.4.6) | — | — |
| `Layout` | Keyboard layout | — | `CustomLayoutService.swift` / `LayoutManager.kt` |
| `Theme` | Theme & styling | — | `Styling/Providers/` / `themes.xml` + `PrefHelper.kt` |
| `CustomDictionary` | User-defined dictionary CRUD, CSV, `.taigi` backup | `engine/userdata` (`UserDataRequest` ops) | `UserDataClient.swift` / `UserDataClient.kt` |
| `Diagnostics` | Device info for bug reports | — | `DiagnosticService.swift` / `DiagnosticService.kt` |
| `FFI` | Bytes-in / bytes-out engine entrypoint | `engine/dispatch` + `engine/swift-ffi` + `engine/android-jni` | iOS `process_request_bytes` (swift-bridge) / Android `RustEngineBridge.processRequestBytes` (JNI) |
