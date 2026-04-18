# Shared-Core Readiness

Snapshot of which iOS engine-layer files are ready for cross-platform extraction to a shared-core module (iOS ↔ Android). Produced by Phase 11 of the iOS structure refactor (branch `refactor-ios-shared-core-audit`, 2026-04-19).

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

## Candidate roster (33 files, ~2341 LOC)

### Phonetics — 9 files, 609 LOC

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
| `Phonetics/ToneUtilities.swift`                     |  60 | Nasal-marker case adapter.                                         |

### Input — 7 files, 882 LOC

| File                                                | LOC | Notes                                                              |
|-----------------------------------------------------|-----|--------------------------------------------------------------------|
| `Input/CharacterInputPipeline.swift`                |  51 | Keystroke → normalized syllable pipeline.                          |
| `Input/CaseTransformer.swift`                       | 111 | `LetterCase` enum + candidate capitalization. KK-free after Phase 4. |
| `Input/TPS/TPSConverter.swift`                      |  63 | TPS tone mapping façade.                                           |
| `Input/TPS/TPSTables.swift`                         | 242 | TPS initial/final tables + `containsTPS`.                          |
| `Input/TPS/TPSInputAdjuster.swift`                  | 129 | TPS composition order fix-ups.                                     |
| `Input/TPS/TPSToTL.swift`                           | 140 | TPS → TL (numeric tone) converter.                                 |
| `Input/TPS/TLToTPS.swift`                           | 146 | TL → TPS converter.                                                |

### Lexicon — 10 files, 597 LOC

| File                                                | LOC | Notes                                                              |
|-----------------------------------------------------|-----|--------------------------------------------------------------------|
| `Lexicon/Models/TaigiWord.swift`                    |  19 | Core candidate value type.                                         |
| `Lexicon/Models/InputType.swift`                    |  12 | `.romanWithoutTone / .romanWithTone / .hanzi`.                     |
| `Lexicon/Models/DictionarySource.swift`             |  43 | Source enum (ordering is authoritative for bitmask — do not reorder). |
| `Lexicon/Models/CustomDictionaryEntry.swift`        |  27 | CRUD value type for custom-dict rows.                              |
| `Lexicon/Models/LexiconConstants.swift`             |  29 | Constants; logging subsystem name is iOS-bundle-specific but harmless as a string. |
| `Lexicon/Models/LexiconError.swift`                 |  34 | `LocalizedError` over Foundation only.                             |
| `Lexicon/Utils/TaigiUnicode.swift`                  |  27 | `nfdPreprocessed` — mirrors Android `TaigiUnicode.kt`.             |
| `Lexicon/Utils/CandidateProcessor.swift`            | 234 | Classify / capitalize / dedupe / score / sort. Callers inject `inputMode`, `isAutoCap`, `FrequencyData`, `currentTime`. |
| `Lexicon/Trie/InputNormalizer.swift`                |  92 | Mode-agnostic normalization → numeric tones.                       |
| `Lexicon/Database/CustomDictionaryDerivation.swift` |  80 | Pure derivation of `notone` / `abbrev` / `roman_num` search keys.  |

### NextWord — 3 files, 119 LOC

| File                                                | LOC | Notes                                                              |
|-----------------------------------------------------|-----|--------------------------------------------------------------------|
| `NextWord/EnginePrediction.swift`                   |  22 | Engine-side value replacing KK `Autocomplete.Suggestion`.          |
| `NextWord/NextWordScorer.swift`                     |  64 | RIME-style decay + user/dict weighting. Invariant-tagged with Android. |
| `NextWord/AutocompleteContextBooster.swift`         |  33 | Re-orders candidates by predicted first-char bigram set.           |

### Autocomplete — 2 files, 81 LOC

| File                                                | LOC | Notes                                                              |
|-----------------------------------------------------|-----|--------------------------------------------------------------------|
| `Autocomplete/Services/AutocompleteInputClassifier.swift` |  55 | Classifies rawInput → `(inputType, searchKey)`. All deps (InputType, CandidateProcessor, InputNormalizer, TPSTables, TPSToTL) are candidates. |
| `Autocomplete/Services/AutocompleteProviders.swift` |  26 | `ComposingStateProvider`, `SelectionContextProvider`, `AutocompleteContextUpdater` protocols over Foundation + `EnginePrediction`. |

### Settings (engine contracts) — 2 files, 53 LOC

| File                                                | LOC | Notes                                                              |
|-----------------------------------------------------|-----|--------------------------------------------------------------------|
| `Settings/EngineSettings.swift`                     |  33 | Read-only protocol consumed by every injected service.             |
| `Settings/EngineSettingsProvider.swift`             |  20 | `current` accessor; live-read not snapshot (documented invariant). |

---

## Dependency graph

Arrows = "depends on at compile time". Only shared-core candidates shown; leaves are pure.

```
EngineSettings              ←──────── (all services)
    ↑
    └── EngineSettingsProvider

InputType  ──────────────── used by TaigiWord siblings, CandidateProcessor
InputMode (Settings/SettingsModels.swift — NOT marked, see §Blockers)
    ↑
    └── CandidateProcessor, InputNormalizer, CustomDictionaryDerivation,
        CaseTransformer, PhoneticsConverter (via `.poj/.tl`)

TaigiUnicode ─── CandidateProcessor, InputNormalizer,
                 CustomDictionaryDerivation (open-coded equivalent)

PhoneticsTables
    ↑
    ├── SyllableParser ─── TLFormatter, POJFormatter, PhoneticsConverter, RomanizationConverter
    ├── ToneRestoration (reads combining scalars only)
    └── ToneUtilities   (reads nasal-marker tables only)

TaigiPhonetics (facade) ─── InputNormalizer, CandidateProcessor (via ToneUtilities/Tables)

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
CaseTransformer, CharacterInputPipeline — leaves
```

Candidate → non-candidate compile-time edges are limited to the soft dependencies listed in §Blockers (`DebugLogger`, `UserFrequencyService.FrequencyData`, `InputMode` colocation). Every other reference resolves to another candidate.

---

## Blockers (known soft dependencies)

These items satisfy the six mechanical criteria but would need a port strategy before extracting.

| # | Dependency                           | Affected candidates                                         | Mitigation                                                   |
|---|--------------------------------------|-------------------------------------------------------------|---------------------------------------------------------------|
| 1 | `DebugLogger` references             | `CandidateProcessor`, `InputNormalizer`                     | `DebugLogger` itself imports `OSLog` in DEBUG; in shared-core it becomes a thin `protocol LoggerBackend` with iOS + Android implementations. No-op release stub already exists. |
| 2 | `InputMode` lives in `Settings/SettingsModels.swift`, which also defines `FontType` (platform-coupled via `KeyboardFonts`) | everything consuming `InputMode` | Split `InputMode` into its own file before extraction. `FontType` and `KeyboardLayoutType` stay platform-side. |
| 3 | `UserFrequencyService.FrequencyData` referenced by `CandidateProcessor.calculateScore` | `CandidateProcessor` | Move `FrequencyData` out of the non-candidate service into a shared value type (e.g. `Lexicon/Models/FrequencyData.swift`). |
| 4 | `TaigiWord.displayText` string assembly is UTF-8 safe across platforms | `TaigiWord`, downstream | No action; documented invariant. |

---

## Exclusions (evaluated, not marked)

These files are engine-layer but deliberately excluded from the candidate set.

| File                                                 | Reason                                                                                     |
|------------------------------------------------------|--------------------------------------------------------------------------------------------|
| `Phonetics/ToneConverter.swift`                      | Reads `SharedSettings.shared` (`isDoubleTapOOEnabled` / `isDoubleTapNNEnabled`). Parameterize the two booleans to qualify. |
| `Lexicon/Models/EnabledDictionaries.swift`           | Mechanically pure, but bitmask layout and `enabledSources` accessor are coupled to the iOS `dictionary.bin` binary format. Unblock when Android aligns to this shape. |
| `Lexicon/Models/DictionarySearchResult.swift`        | Builds app-specific lookup URL (`chhoe.taigi.info` / `sutian.moe.edu.tw`).                 |
| `Lexicon/Utils/ExternalLookupURLBuilder.swift`       | App-specific URL construction.                                                             |
| `Lexicon/Trie/TrieService.swift`                     | `DispatchQueue` + `static let shared` + C++ MARISA bridge (platform dep).                  |
| `Input/Composing/ComposingManager.swift`             | `Combine`, `@Published`, `ObservableObject`.                                               |
| `Input/Composing/ComposingDelegate.swift`            | Mechanically pure protocol, but methods mirror iOS `UITextDocumentProxy` semantics; Android `InputConnection` has a different contract. |
| `Autocomplete/Views/CandidateViewStyleEnvironment.swift` | `EnvironmentKey` default value depends on `CandidateView.Style.standard` (SwiftUI type). |
| `Lexicon/Services/*` (all)                           | Coordinate side effects (DB, singletons, logging).                                         |
| `Lexicon/Database/*Repository.swift`, `*Schema.swift`, `SQLiteConnectionManager.swift` | Use SQLite3 C API + `FileManager` + `DispatchQueue`. |
| `NextWord/NextWordController.swift`                  | `Timer`, `DispatchQueue.main`, `@MainActor`, `SharedSettings.shared`.                      |
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
ios/Sources/TaigiKeyboard/Phonetics/TaigiPhonetics.swift
ios/Sources/TaigiKeyboard/Phonetics/Tables/PhoneticsTables.swift
ios/Sources/TaigiKeyboard/Phonetics/Parser/SyllableParser.swift
ios/Sources/TaigiKeyboard/Phonetics/Formatter/TLFormatter.swift
ios/Sources/TaigiKeyboard/Phonetics/Formatter/POJFormatter.swift
ios/Sources/TaigiKeyboard/Phonetics/Converter/PhoneticsConverter.swift
ios/Sources/TaigiKeyboard/Phonetics/Converter/RomanizationConverter.swift
ios/Sources/TaigiKeyboard/Phonetics/ToneRestoration.swift
ios/Sources/TaigiKeyboard/Phonetics/ToneUtilities.swift
ios/Sources/TaigiKeyboard/Input/CharacterInputPipeline.swift
ios/Sources/TaigiKeyboard/Input/CaseTransformer.swift
ios/Sources/TaigiKeyboard/Input/TPS/TPSConverter.swift
ios/Sources/TaigiKeyboard/Input/TPS/TPSTables.swift
ios/Sources/TaigiKeyboard/Input/TPS/TPSInputAdjuster.swift
ios/Sources/TaigiKeyboard/Input/TPS/TPSToTL.swift
ios/Sources/TaigiKeyboard/Input/TPS/TLToTPS.swift
ios/Sources/TaigiKeyboard/Lexicon/Models/TaigiWord.swift
ios/Sources/TaigiKeyboard/Lexicon/Models/InputType.swift
ios/Sources/TaigiKeyboard/Lexicon/Models/DictionarySource.swift
ios/Sources/TaigiKeyboard/Lexicon/Models/CustomDictionaryEntry.swift
ios/Sources/TaigiKeyboard/Lexicon/Models/LexiconConstants.swift
ios/Sources/TaigiKeyboard/Lexicon/Models/LexiconError.swift
ios/Sources/TaigiKeyboard/Lexicon/Utils/TaigiUnicode.swift
ios/Sources/TaigiKeyboard/Lexicon/Utils/CandidateProcessor.swift
ios/Sources/TaigiKeyboard/Lexicon/Trie/InputNormalizer.swift
ios/Sources/TaigiKeyboard/Lexicon/Database/CustomDictionaryDerivation.swift
ios/Sources/TaigiKeyboard/NextWord/EnginePrediction.swift
ios/Sources/TaigiKeyboard/NextWord/NextWordScorer.swift
ios/Sources/TaigiKeyboard/NextWord/AutocompleteContextBooster.swift
ios/Sources/TaigiKeyboard/Autocomplete/Services/AutocompleteInputClassifier.swift
ios/Sources/TaigiKeyboard/Autocomplete/Services/AutocompleteProviders.swift
ios/Sources/TaigiKeyboard/Settings/EngineSettings.swift
ios/Sources/TaigiKeyboard/Settings/EngineSettingsProvider.swift
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

## Next steps (not in scope for Phase 11)

1. Move `InputMode` into its own file (`Settings/InputMode.swift`) to unblock candidate-grade file-level isolation.
2. Hoist `UserFrequencyService.FrequencyData` into a shared value type.
3. Introduce a `LoggerBackend` protocol so `CandidateProcessor` / `InputNormalizer` do not reference `DebugLogger` directly in shared core.
4. Parameterize `ToneConverter.preprocessPojInput` to unblock that file.
5. Align Android `enabledSources` onto iOS `EnabledDictionaries` shape before promoting that file.
