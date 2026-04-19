# iOS Exemplar Refactor — Phase I Plan (post-Codex review)

Pre-work for the shared-core extraction roadmap. Produced 2026-04-19 after PR #133 closed shared-core soft deps #1–3; revised the same day after Codex strategic review (see `codex-review-2026-04-19.md`).

**Purpose**: make iOS the architectural template Android will copy in Phase II. Clean UI ↔ engine separation, DI-based composition, no engine-layer ObservableObject / singletons, plus measurable latency/memory parity. Once both platforms match this shape and the Phase 0 behavioral invariants hold on both, the Phase III confidence gate (≥95% + FFI POC) becomes achievable.

**Scope boundary**: this plan does NOT extract the shared core or touch FFI. It only moves iOS into a state where the candidate roster can leave the iOS target cleanly AND proves Android has a well-defined copy target.

**Prerequisite**: Phase 0 (`docs/architecture/behavioral-invariants.md`) must exist before Phase I work begins. That doc defines the IME behavior Phase I must not regress.

---

## Current state snapshot (2026-04-19)

- **Shared-core candidates**: 36 files marked, 0 compile-time soft deps, all 5 verification greps green.
- **Non-candidate singletons (`static let shared`)**: 10 — `SharedSettings`, `LexiconService`, `UserFrequencyService`, `BackupService`, `CustomDictionaryService`, `TrieService`, `DictionaryRepository`, `UserFrequencyRepository`, `CustomDictionaryRepository`, `NextWordService`.
- **Engine-layer services already accept DI** (optional `= .shared` default): `LexiconService`, `DictionarySearchService`, `NextWordService`, `NextWordController`, `ComposingManager`, `CustomDictionaryService`, `UserFrequencyService`, `DictionaryRepository`. `BackupService` is the outlier.
- **Engine-layer `ObservableObject` / `@Published`**: only `ComposingManager` (already in Exclusions — needs platform split).
- **UI layer directly calls repositories / `SharedSettings.shared`**: ~40+ sites across `App/Tabs/*`, `KeyboardExtension/*`, `Autocomplete/Models/CandidateViewModels.swift`.
- **Keystroke latency baseline**: not yet measured (G0).
- **Extension memory baseline**: not yet measured (G0).

---

## Task groups

Each group is independently mergeable. Estimates assume single-phase focus.

### G0 · Phase 0 invariants doc + qualitative perf gate (S — ~1–2 hr)

**Problem**: Phase I must not regress perceived latency or cause memory-related extension termination. The cost of a full quantitative baseline (Instruments signposts, P50/P95 capture, per-refactor re-measurement) is not justified for a solo-dev IME — the user is the QA, and perceptible regression on S1/S2/S3 sequences is the acceptance criterion.

**Deliverable (revised 2026-04-19)**:
- Phase 0 doc `docs/architecture/behavioral-invariants.md` — enumerate the cross-platform invariants the 36 candidates must uphold (TL↔POJ round-trip, segmentation tie rules, NFD normalization, candidate dedup, scoring determinism, decay math). Each invariant references a named test case (may be TODO in G9). **Status: authored 2026-04-19.**
- `docs/perf/keyboard-baseline-2026-04.md` and `docs/perf/extension-memory-2026-04.md` — quantitative methodology **deferred** (kept as optional future work for when CI perf lane / team workflow makes numbers worth the overhead). Files retained; header annotated `deferred`.
- **Qualitative gate** in place of quantitative baseline: after G2 / G4-impl / G5-impl, user dogfoods S1/S2/S3 sequences (same three defined in the perf methodology docs) on real device. Pass = no perceptible typing latency regression, no keyboard dismiss (64 MB termination signal), no progressive memory growth during extended typing session (sanity check for leaks only — refactor does not add features, so steady-state capacity is unchanged).

**Why qualitative**: refactor-only scope means total memory footprint should not grow. The dogfooding gate catches leaks and architectural regressions that actually affect users; Instruments numbers catch microsecond-level regressions that would not.

**Risk**: low (doc only). Risk of missing a microsecond-level regression is accepted as a known trade-off.

---

### G1 · BackupService DI-ification (S — ~1 hr)

**Problem**: `BackupService.swift` directly calls `CustomDictionaryService.shared`, `UserFrequencyRepository.shared`, `NextWordService.shared` at 7+ sites. Every other engine-layer service accepts dependencies via init. This one is the outlier.

**Deliverable**: `BackupService` takes `customDictionaryService`, `userFrequencyRepository`, `nextWordService` as init params with `= .shared` defaults (matching the rest of the services). All inline `.shared` calls route through instance properties.

**Risk**: low. No behavior change; call sites still use `BackupService.shared` by default.

---

### G2 · Keyboard extension settings snapshot (M — ~2–3 hr)

**Problem**: `TaigiKeyboardView.swift` reads `SharedSettings.shared` directly 8 times (State init + live re-reads in `.onChange`). `KeyboardViewController+Setup.swift` scatters 4 more. `KeyboardViewController.swift` mutates `isFullAccessEnabled` on the singleton.

**Deliverable**: introduce a `KeyboardEnvironment` (or reuse `EngineSettingsProvider`) passed through `@Environment` / init. Views take the protocol; only the extension's composition root binds to `SharedSettings.shared`.

**Risk**: medium — touches keyboard extension hot path. Need careful testing of settings live-sync via Darwin notifications. Dogfooding pass required (S1/S2/S3: no perceptible latency regression, no keyboard dismiss, no memory leak during extended session).

---

### G3 · Dictionary tab Views → ViewModel pattern (M — ~3–4 hr)

**Problem**: `FrequencyDataView`, `AssociationDataView`, `CustomDictionaryView`, `DataManagementView` directly invoke repository methods (`UserFrequencyRepository.shared.deleteWord(...)`, `NextWordService.shared.allAssociations()`, etc.). SwiftUI Views should not own DB logic.

**Deliverable**: one ViewModel per view (`FrequencyDataViewModel`, `AssociationDataViewModel`, etc.) implementing `ObservableObject`. Repository/service references moved to the VM; View only reads `@Published` state + calls VM actions.

**Risk**: medium — need to preserve async behavior and loading states.

**Status 2026-04-19**: Done (branch `refactor/ios-g3-dict-tab-viewmodels`). 4 VMs added under `App/Tabs/Dictionary/Models/` (`FrequencyDataViewModel`, `AssociationDataViewModel`, `CustomDictionaryViewModel`, `DataManagementViewModel`), each `@MainActor final class … : ObservableObject` with init DI defaulting to `.shared` (matches G1/G6 pattern). CSV parse/build extracted to `CSVDocument` extension (`App/Tabs/Dictionary/Utilities/CSVParsers.swift`) to keep VMs lean; `NextWordService.AssociationEntry: Identifiable` extension relocated to `Models/AssociationEntry+Identifiable.swift`. `DataManagementViewModel` returns a named `BackupExportPayload` struct instead of a bare tuple. Views retain form state (`romanInput`/`hanziInput`/`editingEntry`), alert flags, `ImportExportHandler` (@StateObject), `UIImpactFeedbackGenerator`, and the SwiftUI `fileExporter`/`fileImporter` bindings — only repository/service calls and data state migrated. No API changes to repositories/services.

**Cross-platform format notes (for Phase II Android)**:
- **Frequency CSV**: 2 columns — `word,count`. Count is a positive integer; rows with empty word or non-positive count are dropped silently. Fields wrapping comma/quote/newline use RFC4180 double-quote escaping (`CSVDocument.escape`). Parser toggles `inQuotes` on any `"` — does NOT unescape `""` back to `"` (known short-term simplification; file as follow-up before Phase II parser is written).
- **Association CSV**: 5 columns — `prevWord,prevTl,nextWord,nextTl,count`. Same escaping/parsing caveats. `prevWord`/`prevTl` may be empty (unigram start-of-sentence); `nextWord` must be non-empty; `count` must be > 0.
- **Backup JSON (`.taigi`)**: `BackupService.BackupData` — `{ version, exportedAt, platform, appVersion, customDictionary[], userFrequency[], userAssociation[] }`. `platform: "ios"` field is present in iOS exports; Android import must accept `"ios"` / `"android"` without branching on it (data payload is platform-neutral). `prevTl` in `AssociationBackupEntry` is `String?` (optional), `nextTl` is non-optional `String`.

---

### G4 · ComposingManager engine/platform split (L — ~4–6 hr, **boundary design front-loaded**)

**Problem**: `ComposingManager` is `public class: ObservableObject` with `@Published` properties. Mirrors iOS `UITextDocumentProxy` semantics in its protocol. Listed in Exclusions because:
1. `@Published` / `ObservableObject` (Combine on an engine type).
2. `ComposingDelegate` protocol signatures mirror iOS conventions; Android `InputConnection` has different semantics.
3. Behavior may be emergent from SwiftUI scheduling; decoupling risks perceptible UX changes.

**Deliverable (two stages)**:
- **G4-design** (front-loaded, ~1 hr, blocks G2/G3/G5): produce `docs/architecture/composing-state-boundary.md` sketching the pure `ComposingState` shape and how the platform wrapper translates it back to SwiftUI + `UITextDocumentProxy`. Other task groups must not lock in assumptions that contradict this sketch.
- **G4-impl** (late, ~3–5 hr): split into `ComposingState` (Foundation-only state machine, shared-core candidate) + `ComposingManager` (iOS platform wrapper over `@Published` + `UITextDocumentProxy`).

**Risk**: high (implementation). Mitigation: boundary design early, implementation late, golden-text regression tests via G9.

---

### G5 · NextWordController engine/platform split (L — ~3–4 hr, **boundary design front-loaded**)

**Problem**: `NextWordController` uses `@MainActor`, `Timer`, `DispatchQueue.main`, and reads `SharedSettings.shared`. Listed in Exclusions. Timing of the decay algorithm may be emergent from Timer scheduling.

**Deliverable (two stages)**:
- **G5-design** (front-loaded, ~30 min, alongside G4-design): sketch `NextWordEngine` — caller supplies `currentTime` + fires `tick(at:)`. Document the expected scheduling contract.
- **G5-impl** (late, ~2.5–3.5 hr): extract `NextWordEngine` (Foundation-only). `NextWordController` keeps only the Timer-driven platform executor wrapping the engine.

**Risk**: medium — decay math is subtle; G9 tests must verify scheduling parity before/after.

---

### G6 · AutocompleteService + CandidateViewModels cleanup (S — ~1–2 hr)

**Problem**: `AutocompleteService` holds `private let lexiconService = LexiconService.shared` and `settingsProvider = SharedSettings.shared` at property-init (non-DI). `CandidateViewModels` (5 sites) inline `SharedSettings.shared.candidateTextSizeScale` / `colorSettings` inside view-adjacent model types.

**Deliverable**:
- `AutocompleteService` takes DI via init (matches other services).
- `CandidateViewModels` accepts settings via computed properties or SwiftUI environment, so the Models stop reaching out to `SharedSettings.shared`.

**Risk**: low.

---

### G7 · Singleton stripping (S — ~1 hr)

**Problem**: with G1–G6 done, most singletons exist only as default-value sugar in init params. Keeping `static let shared` invites future regressions (someone bypasses DI again).

**Deliverable**: move composition to a single `CompositionRoot` (or per-target bootstrap in `TaigiKeyboardApp.init` + `KeyboardViewController.viewDidLoad`). Drop `static let shared` from services where no call site remains outside init defaults. Keep `SharedSettings.shared` — that one is legitimately a cross-process store.

**Risk**: low (mechanical), but catches any missed call sites.

---

### G8 · Write `docs/architecture/ios-exemplar.md` (S — ~1 hr)

**Problem**: Android Phase II needs a concrete target. "Look at iOS and copy it" is not enough.

**Deliverable**: a doc describing the target pattern:
- Layer map (View → ViewModel → Service → Repository → SQLite).
- DI composition root.
- Settings access pattern (`EngineSettingsProvider` protocol).
- Engine vs platform split (for `ComposingState` / `NextWordEngine`).
- Naming / file layout rules.
- List of 36 (+) shared-core candidates as the immutable contract surface.

**Risk**: none.

---

### G9 · Engine test coverage baseline (M — ~4–6 hr, **hard Phase I gate, not recurring**)

**Problem**: `/shared-core-confidence` D8 requires a number, and the Phase 0 invariants need test references. Marking this as "recurring" previously effectively deferred it.

**Deliverable**:
- Add or audit unit tests for the top 10 most-depended-on candidates: `PhoneticsConverter`, `SyllableParser`, `TPSToTL`, `TLToTPS`, `InputNormalizer`, `CandidateProcessor`, `NextWordScorer`, `AutocompleteContextBooster`, `CaseTransformer`, `CustomDictionaryDerivation`.
- Target ≥ 70% line coverage with named invariant tests matching Phase 0 labels (e.g. `INVARIANT_tl_to_poj_roundtrip_is_lossless`, `INVARIANT_segmentation_tie_break_is_deterministic`).
- Tests must be runnable via `xcodebuild test` (i.e., wired to the test target).

**Risk**: low (tests only) but can expose pre-existing bugs.

---

### G10 · Dictionary / MARISA / SQLite binary portability audit (NEW, M — ~2–3 hr)

**Problem**: Codex review flagged a blind spot: the shared-core roster proves Swift/Kotlin code can align, but the *data artifacts* beneath it (MARISA trie format, `dictionary.bin`, user-frequency SQLite schema) have their own portability story. A below-the-architecture incompatibility would only surface in Phase IV.

**Deliverable**: `docs/architecture/data-artifacts-portability.md` covering:
- MARISA trie format: does the C++ library produce identical bytes across build toolchains? Android currently uses how? Can Rust bind the same library or does it need its own trie?
- `dictionary.bin`: binary layout, endian-ness, version field, load-time cost on both platforms.
- User-frequency SQLite: schema, migration policy, whether Rust-side would use `rusqlite` or call through platform SQLite.
- Update strategy: how does a shipped core handle dictionary updates without app re-release?

**Output**: decision register (not blocking Phase I, but must be resolved before Phase IV-A design).

**Risk**: analysis only, no code.

---

## Recommended order

```
G0 (baselines + invariants)
   ↓
G4-design + G5-design (boundary sketches, ~1.5 hr total)
   ↓
G8 (doc skeleton) ─── gives direction
   ↓
G1 (BackupService DI) ─── quick unblock
   ↓
G6 (Autocomplete + CandidateVM)
   ↓
G3 (Dictionary tab VMs) ─── UI pattern template
   ↓
G2 (Keyboard extension env) ─── hottest path; dogfooding pass required
   ↓
G5-impl (NextWordController split) ─── unlocks one more candidate
   ↓
G4-impl (ComposingManager split) ─── highest risk, do with fresh context
   ↓
G7 (singleton stripping) ─── mechanical cleanup
   ↓
G9 (test baseline) ─── hard gate; can interleave but must close before Phase II
   ↓
G10 (data artifact portability audit) ─── can parallelize with G9
```

Total estimate: **25–35 hours focused work**. Split across 5–7 sessions is realistic.

---

## Phase I gating signals

Advance to Phase II when ALL of:

1. **Engine purity**: `grep -r "SharedSettings.shared\|\\.shared\b" ios/Sources/TaigiKeyboard/{Lexicon,Phonetics,Input,NextWord,Autocomplete,Settings,Common,Actions}` returns only `SharedSettings`'s own definition + init defaults. No engine logic reaches a global.
2. **UI purity**: `App/Tabs/*` Views contain no direct repository or service calls. All DB work lives in a ViewModel.
3. **Exclusions shrunk**: `ComposingManager`, `NextWordController` are split; shared-core roster grows to ≥40 candidates.
4. **Doc parity**: `docs/architecture/ios-exemplar.md` AND `docs/architecture/behavioral-invariants.md` exist.
5. **Test baseline**: G9's 10-candidate coverage ≥ 70% with named invariant tests referenced from Phase 0 doc.
6. **Latency dogfooding pass** (revised 2026-04-19): S1 (POJ diacritics) / S2 (TPS composition) / S3 (Hanji candidate scroll) on real device show no user-perceptible typing latency regression after G2 / G4-impl / G5-impl land.
7. **Memory leak / stability pass** (revised 2026-04-19): no keyboard extension dismiss observed (64 MB termination signal), no progressive memory growth during extended typing session. Refactor does not add features, so steady-state capacity is unchanged; gate is leak-free, not a number.
8. **Data artifact audit** (NEW): `docs/architecture/data-artifacts-portability.md` exists and lists open decisions (not all need resolution yet — but they must be catalogued).

---

## Out of scope for Phase I

- Shared-core module extraction (Phase IV-A / IV-B).
- Rust FFI design (Phase IV-A).
- FFI POC build (Phase III).
- Android work (Phase II).
- `/shared-core-confidence` skill build (Phase II/III).
- `SharedSettings.resetAppearanceToDefaults()` abstraction (previously flagged Phase 10 MED — addressable in G2 if convenient, but not blocking).
- `ToneConverter.preprocessPojInput` parameterization — **upgraded to G4-impl precondition** after the 2026-04-19 docs-review cycle (Codex C1). `ComposingState.derivedDisplay` needs Foundation-pure tone conversion; G4-impl folds the parameterization in or lands it as a separate prerequisite PR. See `composing-state-boundary.md` §Precondition.
