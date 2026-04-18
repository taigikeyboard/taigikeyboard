# iOS Exemplar Refactor — Phase I Plan

Pre-work for the shared-core extraction roadmap. Produced 2026-04-19 after PR #133 closed shared-core soft deps #1–3.

**Purpose**: make iOS the architectural template Android will copy in Phase II. Clean UI ↔ engine separation, DI-based composition, no engine-layer ObservableObject / singletons. Once both platforms match this shape, the Phase III confidence gate (≥95%) becomes achievable.

**Scope boundary**: this plan does NOT extract the shared core or touch FFI. It only moves iOS into a state where the candidate roster can leave the iOS target cleanly.

---

## Current state snapshot (2026-04-19)

- **Shared-core candidates**: 36 files marked, 0 compile-time soft deps, all 5 verification greps green.
- **Non-candidate singletons (`static let shared`)**: 10 — `SharedSettings`, `LexiconService`, `UserFrequencyService`, `BackupService`, `CustomDictionaryService`, `TrieService`, `DictionaryRepository`, `UserFrequencyRepository`, `CustomDictionaryRepository`, `NextWordService`.
- **Engine-layer services already accept DI** (optional `= .shared` default): `LexiconService`, `DictionarySearchService`, `NextWordService`, `NextWordController`, `ComposingManager`, `CustomDictionaryService`, `UserFrequencyService`, `DictionaryRepository`. Backup service is the outlier.
- **Engine-layer `ObservableObject` / `@Published`**: only `ComposingManager` (already in Exclusions for a reason — needs platform split).
- **UI layer directly calls repositories / `SharedSettings.shared`**: ~40+ sites across `App/Tabs/*`, `KeyboardExtension/*`, `Autocomplete/Models/CandidateViewModels.swift`.

---

## Task groups

Each group is independently mergeable. Estimates assume single-phase focus.

### G1 · BackupService DI-ification (S — ~1 hr)

**Problem**: `BackupService.swift` directly calls `CustomDictionaryService.shared`, `UserFrequencyRepository.shared`, `NextWordService.shared` at 7+ sites. Every other engine-layer service accepts dependencies via init. This one is the outlier.

**Deliverable**: `BackupService` takes `customDictionaryService`, `userFrequencyRepository`, `nextWordService` as init params with `= .shared` defaults (matching the rest of the services). All inline `.shared` calls route through instance properties.

**Risk**: low. No behavior change; call sites still use `BackupService.shared` by default.

---

### G2 · Keyboard extension settings snapshot (M — ~2–3 hr)

**Problem**: `TaigiKeyboardView.swift` reads `SharedSettings.shared` directly 8 times (State init + live re-reads in `.onChange`). `KeyboardViewController+Setup.swift` scatters 4 more. `KeyboardViewController.swift` mutates `isFullAccessEnabled` on the singleton.

**Deliverable**: introduce a `KeyboardEnvironment` (or reuse `EngineSettingsProvider`) passed through `@Environment` / init. Views take the protocol; only the extension's composition root binds to `SharedSettings.shared`.

**Risk**: medium — touches keyboard extension hot path. Need careful testing of settings live-sync via Darwin notifications.

---

### G3 · Dictionary tab Views → ViewModel pattern (M — ~3–4 hr)

**Problem**: `FrequencyDataView`, `AssociationDataView`, `CustomDictionaryView`, `DataManagementView` directly invoke repository methods (`UserFrequencyRepository.shared.deleteWord(...)`, `NextWordService.shared.allAssociations()`, etc.). SwiftUI Views should not own DB logic.

**Deliverable**: one ViewModel per view (`FrequencyDataViewModel`, `AssociationDataViewModel`, etc.) implementing `ObservableObject`. Repository/service references moved to the VM; View only reads `@Published` state + calls VM actions.

**Risk**: medium — need to preserve async behavior and loading states.

---

### G4 · ComposingManager engine/platform split (L — ~4–6 hr)

**Problem**: `ComposingManager` is `public class: ObservableObject` with `@Published` properties. Mirrors iOS `UITextDocumentProxy` semantics in its protocol. Listed in Exclusions because:
1. `@Published` / `ObservableObject` (Combine on an engine type).
2. `ComposingDelegate` protocol signatures mirror iOS conventions; Android `InputConnection` has different semantics.

**Deliverable**: split into two types:
- `ComposingState` — Foundation-only value type / actor holding the state machine (shared-core candidate material).
- `ComposingManager` — iOS-only wrapper that translates `ComposingState` changes into `@Published` + `UITextDocumentProxy` calls.

**Risk**: high — this is on the keystroke hot path. Needs golden-text tests before/after to verify no regression in IME composition.

---

### G5 · NextWordController engine/platform split (L — ~3–4 hr)

**Problem**: `NextWordController` uses `@MainActor`, `Timer`, `DispatchQueue.main`, and reads `SharedSettings.shared`. Listed in Exclusions.

**Deliverable**: extract the RIME-style decay scheduler into `NextWordEngine` (Foundation-only; caller provides `currentTime` + fires `tick()`). `NextWordController` keeps only the Timer-driven platform executor wrapping the engine.

**Risk**: medium — decay math is subtle; tests must verify scheduling parity before/after.

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

### G9 · Engine test coverage baseline (M — ~3–4 hr, recurring)

**Problem**: `/shared-core-confidence` scoring dimension D8 (test coverage) is unknown. Phase III requires a number.

**Deliverable**: add or audit unit tests for the top 10 most-depended-on candidates: `PhoneticsConverter`, `SyllableParser`, `TPSToTL`, `TLToTPS`, `InputNormalizer`, `CandidateProcessor`, `NextWordScorer`, `AutocompleteContextBooster`, `CaseTransformer`, `CustomDictionaryDerivation`. Target: ≥ 70% line coverage, with named invariant tests (`// INVARIANT: tl_to_poj_roundtrip_is_lossless`).

**Risk**: low — tests only.

---

## Recommended order

```
G8 (doc skeleton) ─── gives direction
   ↓
G1 (BackupService DI) ─── quick unblock
   ↓
G6 (Autocomplete + CandidateVM)
   ↓
G3 (Dictionary tab VMs) ─── UI pattern template
   ↓
G2 (Keyboard extension env) ─── hottest path
   ↓
G5 (NextWordController split) ─── unblocks one more candidate
   ↓
G4 (ComposingManager split) ─── highest risk, do with fresh context
   ↓
G7 (singleton stripping) ─── mechanical cleanup
   ↓
G9 (test baseline) ─── recurring, can interleave
```

Total estimate: **20–30 hours focused work** if done in one uninterrupted arc. Split across 4–6 sessions is realistic.

---

## Phase I gating signals

Advance to Phase II when ALL of:

1. **Engine purity**: `grep -r "SharedSettings.shared\|\\.shared\b" ios/Sources/TaigiKeyboard/{Lexicon,Phonetics,Input,NextWord,Autocomplete,Settings,Common,Actions}` returns only `SharedSettings`'s own definition + init defaults. No engine logic reaches a global.
2. **UI purity**: `App/Tabs/*` Views contain no direct repository or service calls. All DB work lives in a ViewModel.
3. **Exclusions shrunk**: `ComposingManager`, `NextWordController` are split; the shared-core roster grows to ≥40 candidates.
4. **Doc parity**: `docs/architecture/ios-exemplar.md` exists and describes the pattern Android will copy.
5. **Test baseline**: G9's 10-candidate coverage ≥ 70%.

---

## Out of scope for Phase I

- Shared-core module extraction (Phase IV).
- Rust FFI design (Phase IV).
- Android work (Phase II).
- `/shared-core-confidence` skill build (Phase II/III).
- `SharedSettings.resetAppearanceToDefaults()` abstraction (previously flagged Phase 10 MED — addressable in G2 if convenient, but not blocking).
- `ToneConverter.preprocessPojInput` parameterization — tiny follow-up from the soft-dep session; do it any time, independent of this plan.
