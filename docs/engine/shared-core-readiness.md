# Shared-Core Readiness

Snapshot of which iOS engine-layer files are ready for cross-platform extraction to a shared-core module (iOS ↔ Android). Produced by Phase 11 of the iOS structure refactor (branch `refactor-ios-shared-core-audit`, 2026-04-19); soft-dep follow-ups #1–3 cleared on branch `refactor-ios-shared-core-softdeps` (2026-04-19).

**Marking contract** — every candidate file begins with:

```swift
// MARK: - Shared-Core Candidate
// Pure logic, Foundation-only. Eligible for cross-platform extraction.
```

**Criteria** (all must hold; see `rules/ios-architecture.md` §4 for the authoritative spec):

1. Imports `Foundation` only (no `UIKit`, `SwiftUI`, `KeyboardKit`, `Combine`, `OSLog`).
2. No global singleton read (no `SharedSettings.shared`, `KeyboardSettings.store`, `*.shared`).
3. No DB / App Group / `FileManager` / file-system access.
4. No app-specific URL construction or external service integration.
5. No platform side effects (`NotificationCenter`, `Timer`, `DispatchQueue`, `OperationQueue`).
6. No Combine primitives (`@Published`, `ObservableObject`, `@MainActor` on type).

---

## Candidate roster (41 files, ~2840 LOC after case-transform slice)

G5-impl (2026-04-19, PR #137) added three NextWord engine files. G4-impl
(same day, this PR) added three composing-engine files plus promoted
`Phonetics/ToneConverter.swift` once `SharedSettings.shared` was removed
behind a `ToneToggles` parameter. The case-transform slice (post-v3.5.7)
removed `Phonetics/ToneUtilities.swift` and `Input/CaseTransformer.swift`
from the roster — both now live in `engine/phonetics/src/case_transform.rs`.

### Phonetics — 10 files, 700 LOC

| File                                                | LOC | Notes                                                              |
|-----------------------------------------------------|-----|--------------------------------------------------------------------|
| `Phonetics/TaigiPhonetics.swift`                    |  81 | Facade re-exporting the split layers below.                        |
| `Phonetics/Tables/PhoneticsTables.swift`            |  86 | Tone-mark tables, combining scalars — data only.                   |
| `Phonetics/Parser/SyllableParser.swift`             |  82 | `stripToneMark`, `splitInitialFinal`, `normalizeToTL`, `parseSyllable`. |
| `Phonetics/Formatter/TLFormatter.swift`             |  36 | TL tone-mark placement.                                            |
| `Phonetics/Formatter/POJFormatter.swift`            |  87 | POJ tone-mark placement, `tlFinalToPOJ`.                           |
| `Phonetics/Converter/PhoneticsConverter.swift`      | 121 | High-level display round-trips (POJ↔TL).                           |
| `Phonetics/Converter/RomanizationConverter.swift`   |  20 | Thin wrapper around the formatter/parser pair.                     |
| `Phonetics/ToneRestoration.swift`                   |  36 | NFD-based tone-mark stripping for backspace.                       |
| ~~`Phonetics/ToneUtilities.swift`~~ (DELETED — case-transform slice) | — | **MIGRATED** to `engine/phonetics/src/case_transform.rs` (`adjust_nasal_marker_case`, `uppercase_tone_char`, `lowercase_tone_char`, `full_uppercase_tone_string`). PR #187 platform-stays decision SUPERSEDED by Path G. |
| `Phonetics/ToneConverter.swift`                     |  95 | POJ/TL tone conversion. POJ preprocessing toggles passed in as `ToneToggles` (G4-impl). Logs via `LoggerBackend` — no `SharedSettings.shared` / OSLog. |

### Input — 9 files, ~1160 LOC

| File                                                | LOC | Notes                                                              |
|-----------------------------------------------------|-----|--------------------------------------------------------------------|
| `Input/CharacterInputPipeline.swift`                |  51 | Keystroke → normalized syllable pipeline.                          |
| ~~`Input/CaseTransformer.swift`~~ (DELETED — case-transform slice) | — | **MIGRATED** to `engine/phonetics/src/case_transform.rs` (`transform_input_case`, `capitalize_candidate`, `LetterCase` enum). Bridged via `RustEngineBridge.transformInputCase` / `.capitalizeCandidate`. |
| `Input/TPS/TPSConverter.swift`                      |  63 | TPS tone mapping façade.                                           |
| `Input/TPS/TPSTables.swift`                         | 242 | TPS initial/final tables + `containsTPS`.                          |
| `Input/TPS/TPSInputAdjuster.swift`                  | 129 | TPS composition order fix-ups.                                     |
| `Input/TPS/TPSToTL.swift`                           | 140 | TPS → TL (numeric tone) converter.                                 |
| `Input/TPS/TLToTPS.swift`                           | 146 | TL → TPS converter.                                                |
| `Input/Composing/ComposingState.swift`              | 230 | Pure state machine (G4-impl). Intent API + `apply(...)` returning `ComposingTransition`. |
| `Input/Composing/ComposingTransition.swift`         |  50 | Platform-neutral `Effect` enum + `Transition` value (G4-impl).      |

### Lexicon — 11 files, 615 LOC

| File                                                | LOC | Notes                                                              |
|-----------------------------------------------------|-----|--------------------------------------------------------------------|
| `Lexicon/Models/TaigiWord.swift`                    |  19 | Core candidate value type.                                         |
| `Lexicon/Models/InputType.swift`                    |  12 | `.romanWithoutTone / .romanWithTone / .hanzi`.                     |
| `Lexicon/Models/DictionarySource.swift`             |  43 | Source enum (ordering is authoritative for bitmask — do not reorder). |
| `Lexicon/Models/CustomDictionaryEntry.swift`        |  27 | CRUD value type for custom-dict rows.                              |
| `Lexicon/Models/FrequencyData.swift`                |  18 | Per-word usage snapshot (count + lastUsedMillis). Consumed by ranking; hoisted out of `UserFrequencyRepository`. |
| `Lexicon/Models/LexiconConstants.swift`             |  29 | Constants; logging subsystem name is iOS-bundle-specific but harmless as a string. |
| `Lexicon/Models/LexiconError.swift`                 |  34 | `LocalizedError` over Foundation only.                             |
| `Lexicon/Utils/TaigiUnicode.swift`                  |  27 | `nfdPreprocessed` — mirrors Android `TaigiUnicode.kt`. **Platform-stays (PR #187)** — Android JVM unit tests can't load `.so`; helper kept on platform. Rust crate retains canonical implementation but is not invoked at search-key build sites. |
| `Lexicon/Utils/CandidateProcessor.swift`            |  28 | **MIGRATED** body. Was 267 LOC (classify/capitalize/dedupe/score/sort). Now: `capitalize` 1-line bridge to `RustEngineBridge.capitalizeCandidate` (case-transform slice) + `startsWithRomanLetter` 3-line predicate. Score/sort/dedupe in v3.5.2 ranking; classify in v3.5.7 lexicon classification. |
| `Lexicon/Trie/InputNormalizer.swift`                |  92 | Mode-agnostic normalization → numeric tones. Logs via `LoggerBackend`. |
| `Lexicon/Database/CustomDictionaryDerivation.swift` |  80 | Pure derivation of `notone` / `abbrev` / `roman_num` search keys.  |

### NextWord — 6 files, ~530 LOC

| File                                                | LOC | Notes                                                              |
|-----------------------------------------------------|-----|--------------------------------------------------------------------|
| `NextWord/EnginePrediction.swift`                   |  22 | Engine-side value replacing KK `Autocomplete.Suggestion`.          |
| `NextWord/NextWordScorer.swift`                     |  64 | RIME-style decay + user/dict weighting. Invariant-tagged with Android. |
| `NextWord/AutocompleteContextBooster.swift`         |  33 | Re-orders candidates by predicted first-char bigram set.           |
| `NextWord/NextWordEngine.swift`                     | 290 | Pure decide/filter pipeline (G5-impl). Enum namespace, no time/timer reads. |
| `NextWord/NextWordOutcome.swift`                    |  96 | Intent / PersistedState / DecisionInput / Outcome / Effect / AssociationPair / EngineSettings DTOs (G5-impl). |
| `NextWord/RawNextWordPrediction.swift`              |  26 | Service-boundary DTO (hanzi/tl/score) decoupled from `NextWordService` (G5-impl). |

### Autocomplete — 2 files, 81 LOC

| File                                                | LOC | Notes                                                              |
|-----------------------------------------------------|-----|--------------------------------------------------------------------|
| `Autocomplete/Services/AutocompleteInputClassifier.swift` |  55 | Classifies rawInput → `(inputType, searchKey)`. All deps (InputType, CandidateProcessor, InputNormalizer, TPSTables, TPSToTL) are candidates. |
| `Autocomplete/Services/AutocompleteProviders.swift` |  26 | `ComposingStateProvider`, `SelectionContextProvider`, `AutocompleteContextUpdater` protocols over Foundation + `EnginePrediction`. |

### Settings (engine contracts) — 4 files, ~93 LOC

| File                                                | LOC | Notes                                                              |
|-----------------------------------------------------|-----|--------------------------------------------------------------------|
| `Settings/EngineSettings.swift`                     |  33 | Read-only protocol consumed by every injected service.             |
| `Settings/EngineSettingsProvider.swift`             |  20 | `current` accessor; live-read not snapshot (documented invariant). |
| `Settings/InputMode.swift`                          |  19 | `.poj / .tl / .english / .tps`. Pure enum; `displayName` localization lives platform-side in `SettingsModels.swift`. |
| `Settings/ToneToggles.swift`                        |  21 | POJ preprocessing toggles as a value type (G4-impl). Consumed by `ComposingState` / `ToneConverter`. |

### Common (shared-core infrastructure) — 1 file, 59 LOC

| File                                                | LOC | Notes                                                              |
|-----------------------------------------------------|-----|--------------------------------------------------------------------|
| `Common/LoggerBackend.swift`                        |  59 | `LoggerBackend` protocol + `NullLoggerBackend` default + thread-safe `LoggerFactory`. iOS installs `DebugLogger` at startup via `LoggerFactory.install(_:)`. |

---

## Dependency graph

Arrows = "depends on at compile time". Only shared-core candidates shown; leaves are pure.

```
EngineSettings              ←──────── (all services)
    ↑
    └── EngineSettingsProvider

InputType  ──────────────── used by TaigiWord siblings, CandidateProcessor
InputMode  ──────────────── CandidateProcessor, InputNormalizer, CustomDictionaryDerivation,
                            PhoneticsConverter (via `.poj/.tl`)

LoggerBackend (+ LoggerFactory) ─── CandidateProcessor, InputNormalizer
FrequencyData ─── CandidateProcessor (ranking input)

TaigiUnicode ─── CandidateProcessor, InputNormalizer,
                 CustomDictionaryDerivation (open-coded equivalent)

PhoneticsTables
    ↑
    ├── SyllableParser ─── TLFormatter, POJFormatter, PhoneticsConverter, RomanizationConverter
    └── ToneRestoration (reads combining scalars only)

TaigiPhonetics (facade) ─── InputNormalizer, CandidateProcessor (via Tables)

TPSTables ──── TPSToTL, TLToTPS, TPSInputAdjuster, TPSConverter, InputNormalizer
TPSToTL  ───── InputNormalizer, CustomDictionaryDerivation.generateRomanNum
TLToTPS  ───── TPSConverter

InputNormalizer ─── CustomDictionaryDerivation

TaigiWord ──── CandidateProcessor, AutocompleteContextBooster, (engine services)

DictionarySource ──── (engine services — not consumed by any shared-core candidate except transitively)

LexiconError, LexiconConstants, CustomDictionaryEntry — leaves (no inbound candidate edges)

EnginePrediction ──── AutocompleteProviders (protocol signature), (NextWord controller consumer)
AutocompleteInputClassifier ── consumes: InputType, CandidateProcessor, InputNormalizer, TPSTables, TPSToTL

NextWordScorer, AutocompleteContextBooster — leaves
CharacterInputPipeline — leaf (CaseTransformer migrated to Rust in case-transform slice)
```

Every candidate's compile-time references now resolve to another candidate. The former soft dependencies (`DebugLogger` references, `InputMode` colocation with `FontType`, `FrequencyData` nested inside `UserFrequencyRepository`) were cleared in the 2026-04-19 follow-up; see §Blockers → Resolved for details.

---

## Blockers (known soft dependencies)

These items satisfy the six mechanical criteria but would need a port strategy before extracting.

| # | Dependency                           | Affected candidates                                         | Mitigation                                                   |
|---|--------------------------------------|-------------------------------------------------------------|---------------------------------------------------------------|
| 1 | `TaigiWord.displayText` string assembly is UTF-8 safe across platforms | `TaigiWord`, downstream | No action; documented invariant. |

### Resolved (2026-04-19, branch `refactor-ios-shared-core-softdeps`)

| Previous # | Dependency | Resolution |
|---|---|---|
| ~~1~~ | `DebugLogger` references in `CandidateProcessor`, `InputNormalizer` | Added `LoggerBackend` protocol + `LoggerFactory` in `Common/LoggerBackend.swift`. `DebugLogger` now conforms; candidates consume the protocol. iOS entry points install the factory at startup. |
| ~~2~~ | `InputMode` colocated with `FontType` in `Settings/SettingsModels.swift` | `InputMode` moved to `Settings/InputMode.swift`. `displayName` localization stays platform-side as a `SettingsModels.swift` extension. |
| ~~3~~ | `FrequencyData` nested inside `UserFrequencyRepository` | Hoisted to top-level `Lexicon/Models/FrequencyData.swift`. Nested struct + `UserFrequencyService.FrequencyData` typealias removed; call sites updated to reference the top-level type directly. |

---

## Exclusions (evaluated, not marked)

These files are engine-layer but deliberately excluded from the candidate set.

| File                                                 | Reason                                                                                     |
|------------------------------------------------------|--------------------------------------------------------------------------------------------|
| `Lexicon/Models/EnabledDictionaries.swift`           | Mechanically pure, but bitmask layout and `enabledSources` accessor are coupled to the iOS `dictionary.bin` binary format. Unblock when Android aligns to this shape. |
| `Lexicon/Models/DictionarySearchResult.swift`        | Builds app-specific lookup URL (`chhoe.taigi.info` / `sutian.moe.edu.tw`).                 |
| `Lexicon/Utils/ExternalLookupURLBuilder.swift`       | App-specific URL construction.                                                             |
| `Lexicon/Trie/TrieService.swift`                     | `DispatchQueue` + `static let shared` + C++ MARISA bridge (platform dep).                  |
| `Input/Composing/ComposingManager.swift`             | iOS platform wrapper over `ComposingState` — `Combine`, `@Published`, `ObservableObject`, delegate / context-sink wiring. Shrunk to ~150 LOC by G4-impl. |
| `Input/Composing/ComposingDelegate.swift`            | iOS `Effect` interpreter (single `execute(_:)` method). Android Phase II mirror implements the same contract against `InputConnection`. |
| `Autocomplete/Views/CandidateViewStyleEnvironment.swift` | `EnvironmentKey` default value depends on `CandidateView.Style.standard` (SwiftUI type). |
| `Lexicon/Services/*` (all)                           | Coordinate side effects (DB, singletons, logging).                                         |
| `Lexicon/Database/*Repository.swift`, `*Schema.swift`, `SQLiteConnectionManager.swift` | Use SQLite3 C API + `FileManager` + `DispatchQueue`. |
| `NextWord/NextWordController.swift`                  | iOS platform executor for `NextWordEngine` (G5-impl). `Timer`, `DispatchQueue.main`, `@MainActor`. |
| `NextWord/Services/NextWordService.swift`            | SQLite + `FileManager` + `SharedSettings.shared`.                                          |
| `NextWord/Repository/*`                              | SQLite.                                                                                    |
| `App/Tabs/Layout/AppearanceSettingsViewModel.swift`  | SwiftUI `Color` + KK `Color.keyboardBackground` + SharedSettings write path.               |
| `Lexicon/Services/DictionarySearchService.swift`     | Depends on `DictionaryRepository`, `CustomDictionaryRepository`, `EngineSettingsProvider`; iOS-only `_ = LexiconService.shared` bootstrap. |

---

## Verification

Run from the repo root; each grep must return empty (no filename output).

```sh
# Build the candidate list
cat <<'EOF' > /tmp/shared-core-list.txt
ios/Sources/TaigiKeyboard/Common/LoggerBackend.swift
ios/Sources/TaigiKeyboard/Phonetics/TaigiPhonetics.swift
ios/Sources/TaigiKeyboard/Phonetics/Tables/PhoneticsTables.swift
ios/Sources/TaigiKeyboard/Phonetics/Parser/SyllableParser.swift
ios/Sources/TaigiKeyboard/Phonetics/Formatter/TLFormatter.swift
ios/Sources/TaigiKeyboard/Phonetics/Formatter/POJFormatter.swift
ios/Sources/TaigiKeyboard/Phonetics/Converter/PhoneticsConverter.swift
ios/Sources/TaigiKeyboard/Phonetics/Converter/RomanizationConverter.swift
ios/Sources/TaigiKeyboard/Phonetics/ToneRestoration.swift
ios/Sources/TaigiKeyboard/Input/CharacterInputPipeline.swift
ios/Sources/TaigiKeyboard/Input/TPS/TPSConverter.swift
ios/Sources/TaigiKeyboard/Input/TPS/TPSTables.swift
ios/Sources/TaigiKeyboard/Input/TPS/TPSInputAdjuster.swift
ios/Sources/TaigiKeyboard/Input/TPS/TPSToTL.swift
ios/Sources/TaigiKeyboard/Input/TPS/TLToTPS.swift
ios/Sources/TaigiKeyboard/Lexicon/Models/TaigiWord.swift
ios/Sources/TaigiKeyboard/Lexicon/Models/InputType.swift
ios/Sources/TaigiKeyboard/Lexicon/Models/DictionarySource.swift
ios/Sources/TaigiKeyboard/Lexicon/Models/CustomDictionaryEntry.swift
ios/Sources/TaigiKeyboard/Lexicon/Models/FrequencyData.swift
ios/Sources/TaigiKeyboard/Lexicon/Models/LexiconConstants.swift
ios/Sources/TaigiKeyboard/Lexicon/Models/LexiconError.swift
ios/Sources/TaigiKeyboard/Lexicon/Utils/TaigiUnicode.swift
ios/Sources/TaigiKeyboard/Lexicon/Utils/CandidateProcessor.swift
ios/Sources/TaigiKeyboard/Lexicon/Trie/InputNormalizer.swift
ios/Sources/TaigiKeyboard/Lexicon/Database/CustomDictionaryDerivation.swift
ios/Sources/TaigiKeyboard/NextWord/EnginePrediction.swift
ios/Sources/TaigiKeyboard/NextWord/NextWordScorer.swift
ios/Sources/TaigiKeyboard/NextWord/AutocompleteContextBooster.swift
ios/Sources/TaigiKeyboard/NextWord/NextWordEngine.swift
ios/Sources/TaigiKeyboard/NextWord/NextWordOutcome.swift
ios/Sources/TaigiKeyboard/NextWord/RawNextWordPrediction.swift
ios/Sources/TaigiKeyboard/Autocomplete/Services/AutocompleteInputClassifier.swift
ios/Sources/TaigiKeyboard/Autocomplete/Services/AutocompleteProviders.swift
ios/Sources/TaigiKeyboard/Settings/EngineSettings.swift
ios/Sources/TaigiKeyboard/Settings/EngineSettingsProvider.swift
ios/Sources/TaigiKeyboard/Settings/InputMode.swift
ios/Sources/TaigiKeyboard/Settings/ToneToggles.swift
ios/Sources/TaigiKeyboard/Input/Composing/ComposingState.swift
ios/Sources/TaigiKeyboard/Input/Composing/ComposingTransition.swift
ios/Sources/TaigiKeyboard/Phonetics/ToneConverter.swift
EOF

# 1. Every candidate must import Foundation (and nothing else heavy)
grep -L "^import Foundation" $(cat /tmp/shared-core-list.txt)

# 2. No disallowed imports (anchor to line start so doc-comment mentions do not match)
grep -lE "^import (KeyboardKit|UIKit|SwiftUI|Combine|OSLog)" $(cat /tmp/shared-core-list.txt)

# 3. No singleton reads (exclude `///` doc-comment mentions)
grep -En "SharedSettings\.shared|KeyboardSettings\.store|\b[A-Z][A-Za-z]+\.shared\b" $(cat /tmp/shared-core-list.txt) | grep -v '///'

# 4. No platform side effects / URL / file system (exclude `///` doc-comment mentions)
grep -En "URL\(string:|FileManager|DispatchQueue|@Published|ObservableObject|NotificationCenter|Timer\.scheduledTimer|@MainActor" $(cat /tmp/shared-core-list.txt) | grep -v '///'

# 5. Every candidate carries the marker
grep -L "Shared-Core Candidate" $(cat /tmp/shared-core-list.txt)
```

Each of the five greps should produce no output. The import check (grep 2) is anchored to line start, so doc-comment text like `EnginePrediction`'s note that it does *not* `import KeyboardKit` is excluded. Greps 3 and 4 pipe through `grep -v '///'` for the same reason on singleton/side-effect text mentioned inside Swift doc comments.

---

## Next steps

1. ~~Move `InputMode` into its own file (`Settings/InputMode.swift`).~~ Done 2026-04-19.
2. ~~Hoist `UserFrequencyService.FrequencyData` into a shared value type.~~ Done 2026-04-19 → `Lexicon/Models/FrequencyData.swift`.
3. ~~Introduce a `LoggerBackend` protocol so `CandidateProcessor` / `InputNormalizer` do not reference `DebugLogger` directly in shared core.~~ Done 2026-04-19 → `Common/LoggerBackend.swift`.
4. ~~Parameterize `ToneConverter.preprocessPojInput` to unblock that file.~~ Done 2026-04-19 (G4-impl) — now consumes `ToneToggles`.
5. Align Android `enabledSources` onto iOS `EnabledDictionaries` shape before promoting that file.
