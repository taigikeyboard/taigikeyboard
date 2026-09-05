# Taigi Keyboard - Technical Specifications

> **Type**: Index
> **Keywords**: `spec`, `index`, `specification`

---

## Summary

- Cross-platform reference for iOS/Android implementation alignment
- Quick reference for Claude Code, bullet-point style, concise

---

## Document Index

### `engine/` — IME Core Logic

| File | Description | Status |
|------|-------------|--------|
| `engine/migration-inventory.csv` | Rust slice inventory: every shipped pub item + native pending/keep/wont-migrate (148 rows, 13 cols, zh-TW descriptions) | Canonical |
| `engine/binary-format.md` | `dictionary.fst` + `association.bin` binary spec (mmap-friendly, byte-exact cross-platform) | Active |
| `engine/ffi-safety.md` | Cross-FFI safety contract: panic boundaries, size caps, generation semantics | Active |
| `engine/rust-core-proto.md` | Engine proto envelope + per-slice request/response shapes | Active |
| `engine/composing.md` | Composing state machine (`Phase::Idle` / `Composing { raw }`) — Rust `engine/composing` | Active |
| `engine/continuous-input-ranking.md` | Continuous-input lexicographic SortKey + user_freq_boost + recency_rank spec (v3.5.8 Phase 9 source of truth) | Active |
| `engine/continuous-candidate-display.md` | Continuous candidate dual-line display (roman + hanji) spec — §4 carrier shipped (Items 5+6); §15 fallback retire DONE (Item 13) | Active |
| `engine/continuous-commit-and-display.md` | Continuous mode commit/display contract (Model B) — extracted from `continuous-input-ranking.md` §10 | Active |
| `engine/autocomplete.md` | Candidate search and suggestion pipeline | Active |
| `engine/tone.md` | Tone conversion / restoration / nasal-marker — Rust `engine/phonetics` | Active |
| `engine/sort.md` | Candidate ranking — Rust `engine/ranking` | Active |
| `engine/flow.md` | End-to-end IME data flow (keystroke → candidates → commit) | Active |
| `engine/nextword.md` | Next-word prediction — Rust `engine/nextword` | Active |
| `engine/tps.md` | TPS Taiwanese Phonetic Symbols (方音符號) — Rust `engine/phonetics::tps` | Active |
| `engine/custom-dictionary.md` | User-defined dictionary (CRUD, CSV import/export) — platform SQLite | Active |
| `engine/diagnostics.md` | Device-info collection for bug reporting | Active |
| `engine/syllabifier.md` | Syllable parse primitive + `SyllableInventory` (`syllables.fst`) + lattice consumption | Active |

### `architecture/` — Architectural Contracts

| File | Description | Status |
|------|-------------|--------|
| `architecture/system-overview.md` | Mermaid architecture diagrams: system context, engine crate graph, build pipeline, keystroke flow | Active |
| `architecture/behavioral-invariants.md` | Cross-platform behavior contract (every Rust slice must preserve) | Active |
| `architecture/dogfood-checklist.md` | Real-device acceptance items S1–S28 (type X → expect Y, pins `INVARIANT_*`) — read before a dogfood pass | Active |
| `architecture/composing-state-boundary.md` | Composing engine ↔ platform binding contract (effect enum, race rules) — G4 design record; state machine since moved to Rust | Reference |
| `architecture/nextword-engine-boundary.md` | NextWord engine ↔ platform binding contract (generation, decay, timer) | Active |
| `architecture/keyboard-body-invariants-android.md` | Android Compose keyboard-body geometry / touch-target invariants (refactor-freeze contract) | Active |
| `architecture/data-artifacts-portability.md` | Binary artifacts + SQLite portability contract | Active |
| `architecture/i18n-multilang-plan.md` | App-UI multi-language plan — `i18n/` JSON → codegen resources; shipped, kept as the design record | Reference |
| `architecture/macos-roadmap.md` | macOS desktop IME (InputMethodKit over the shared engine) — design, PR table, dogfood run-book | Active |
| `architecture/macos-candidate-window-port.md` | macOS candidate window port (MacishType-style layouts) — design + not-adopted list | Reference |
| `architecture/manual-release-notes.md` | Canonical English What's New, in-app history sync, validation, and manual store paste workflow | Active |
| `architecture/macos-release.md` | macOS Developer ID signing, notarization, and web-distributed `.pkg` workflow | Active |
| `architecture/windows-roadmap.md` | Windows desktop IME (TSF in Rust over the shared engine, macOS UX parity) — design W1–W16, PR table, reference alignment, dogfood run-book | Active |
| `architecture/windows-release.md` | Windows installer (Inno Setup), Authenticode signing, and web-distributed installer workflow | Active |
| `architecture/ios-exemplar.md` | iOS architectural pattern (alignment target for Android) | Reference |
| `architecture/android-exemplar.md` | Android-specific deviations from iOS exemplar | Reference |

### `ui/` — Presentation & Layout (8)

| File | Description | Status |
|------|-------------|--------|
| `ui/layout.md` | Keyboard layout definitions and conversion | Active |
| `ui/flick.md` | Flick tone keyboard layout and input | Active |
| `ui/case.md` | Case handling (Shift, CapsLock, transformation) | Active |
| `ui/device.md` | Device adaptation for iPhone and iPad | Active |
| `ui/app-ui.md` | Main App UI structure (tabs, settings) | Active |
| `ui/theme.md` | Theme & styling (colors, fonts, user customization) | Active |
| `ui/emoji.md` | Emoji keyboard (ISEmojiView iOS / Compose Android, taigi-emojis data) | Active |
| `ui/callouts.md` | Long-press callouts + tone-variation menus (engine map + platform popups) | Active |

### `references/` — External IME Research (7)

| File | Description | Status |
|------|-------------|--------|
| `references/mainstream-ime-comparison.md` | IME comparison index (TL;DR matrix + per-repo cards) — entry point for best-practice cites | Reference |
| `references/azookey-reference.md` | azooKey research (SwiftUI, Flick, CustardKit) | Reference |
| `references/khiin-reference.md` | khiin-rs research (DPSegment, Bigram, Trie) | Reference |
| `references/moe-taigi-reference.md` | MOE Taigi IME analysis (Segmentation, Nail) | Reference |
| `references/moe-taigi-asr-reference.md` | MOE Taigi IME ASR implementation analysis | Reference |
| `references/rime-reference.md` | librime research (Pipeline, DAG, SpellingAlgebra) | Reference |
| `references/keywords.md` | Standardized keyword mapping for core logic and UI | Reference |

### `reports/` — Audit & Analysis Reports (historical)

One-off snapshots ordered chronologically. Specs cited by engine code (`v358-refactor-design-spec`, `v359-b-plan`) stay here as source-of-truth.

| File | Description | Status |
|------|-------------|--------|
| `reports/2026-04-18-dynamic-font-download-plan.md` | Forward-looking feature plan | Plan |
| `reports/2026-05-18-v358-refactor-design-spec.md` | v3.5.9 refactor implementation design spec (S0/A2/A1) | Historical |
| `reports/2026-05-20-triple-index-eval.md` | 三索引 (POJ+TL+TPS first-class lattice) feasibility eval | Historical |
| `reports/2026-05-20-v359-b-plan.md` | v3.5.9-B dual-index (POJ first-class lattice) plan (Codex-converged v3) | Historical |
| `reports/2026-06-03-user-data-cross-mode-audit.md` | User-data cross-input-mode + single→triple-index compatibility audit (v3.6.1 fix candidates) | Historical |
| `reports/2026-06-22-i18n-content-draft-review.md` | i18n content.json 5-lang draft proofread sheet | Historical |
| `reports/2026-06-22-i18n-poj-draft-review.md` | i18n POJ draft proofread sheet | Historical |
| `reports/2026-06-22-i18n-symbol-draft-review.md` | i18n symbol draft proofread sheet | Historical |
| `reports/2026-06-22-i18n-tl-draft-review.md` | i18n TL draft proofread sheet | Historical |
| `reports/2026-08-30-hanlo-together-mode-research.md` | 候選詞顯示 picker research: 漢羅並排 (default, title/subtitle) / 漢羅濫 (one-label hanji+roman, formerly 漢羅齊出; Part I) / 羅馬字 (roman-only cells in today's candidate UI, all 4 platforms; Part II — 3-column + 台語拼音校正 considered and dropped, kept as future 校正 reference) — research only, not implemented | Plan |

### `releases/` — Per-Release Archives (1)

| File | Description | Status |
|------|-------------|--------|
| `releases/v3.5.8/plan.md` | v3.5.8 continuous-input plan & archive (SHIPPED) | Historical |

### Root — Guides & Planning

| File | Description | Status |
|------|-------------|--------|
| `go-public-checklist.md` | What has to be true before the repository is made public — history audit results, dictionary redistribution blockers, anonymous-clone check, release-chain review, and the settings to turn on afterwards | Active |
| `roadmap.md` | Forward-looking work items not yet scheduled into a release slice | Active |
| `CODE_SIGNING_POLICY.md` | Code-signing policy for released binaries (SignPath Foundation requirement) | Active |

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
| `Ranking` | Candidate dedup / score / sort | `engine/ranking` | `RustEngineBridge.processCandidates*` |
| `Tone` | Tone-mark conversion + restoration + nasal-marker | `engine/phonetics` | `RustEngineBridge.normalizeTone` / `restoreTone` |
| `CaseTransform` | Per-char + per-string case mapping (POJ/TL aware) | `engine/phonetics::case_transform` | `RustEngineBridge+CaseTransform.swift` / `CaseTransformBridge.kt` |
| `NextWord` | Bigram association lookup + decay scoring + ranking | `engine/nextword` (+ `engine/lexicon::assoc_lookup`) | `NextWordController.swift` / `NextWordHandler.kt` (timer/threading) |
| `UserFrequency` | Per-word usage tracking (count + lastUsed) — `wont_migrate` | — | `UserFrequencyService.swift` / `.kt` (SQLite, native-only) |
| ~~`Segmentation`~~ | ~~Syllable segmentation~~ (removed v3.4.6) | — | — |
| `Layout` | Keyboard layout | — | `CustomLayoutService.swift` / `LayoutManager.kt` |
| `Theme` | Theme & styling | — | `Styling/Providers/` / `themes.xml` + `PrefHelper.kt` |
| `CustomDictionary` | User-defined dictionary CRUD — `wont_migrate` | — | `CustomDictionaryRepository.swift` / `CustomDictionaryService.kt` (SQLite) |
| `Diagnostics` | Device info for bug reports | — | `DiagnosticService.swift` / `DiagnosticService.kt` |
| `FFI` | Bytes-in / bytes-out engine entrypoint | `engine/dispatch` + `engine/swift-ffi` + `engine/android-jni` | `RustEngineBridge.process_request_bytes` (both platforms) |
