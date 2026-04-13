# Architecture Review Report

**Date**: 2026-04-03
**Scope**: Full cross-platform architecture review (iOS + Android)
**Reviewers**: Claude Opus 4.6, Google Gemini
**iOS**: 97 Swift files, ~16K LOC | **Android**: 114 Kotlin files, ~23K LOC

---

## Executive Summary

Overall architecture quality: **B+ (8/10)**

The Taigi Keyboard has a well-structured, layered architecture with clear data flow from key press to output. Both platforms follow consistent domain naming and maintain a clean separation between dictionary/linguistic engine, input processing, and UI rendering. The main areas for improvement are a few oversized orchestrator classes (SRP violations), some folder hierarchy inconsistencies, and opportunities for better cross-platform naming alignment.

### Key Findings at a Glance

| Area | Rating | Summary |
|------|--------|---------|
| Data Flow Clarity | A | Clean, traceable pipeline on both platforms |
| Layer Separation | A- | Good separation; minor mixing in View layer |
| Single Responsibility | B | 5-6 files exceed scope; most are well-focused |
| Folder Structure | B+ | iOS well-organized; Android `ime/text/` cluttered |
| File Naming | B+ | Domain terms consistent; some generic/misleading names |
| Cross-Platform Alignment | B | Behavioral alignment good; naming conventions diverge |
| Duplicate Implementations | B+ | Platform-specific duplication is mostly justified |

---

## 1. Data Flow: Input to Output

### Pipeline Overview

Both platforms follow the same logical pipeline:

```
Key Press
  --> Action Dispatch (ActionHandler / TextInputManager)
    --> Composing State (ComposingManager: rawInput + composingText)
      --> Display Derivation (ToneConverter --> TaigiPhonetics)
      --> Candidate Search (AutocompleteService --> LexiconService)
        --> Trie Prefix Search (TrieService: MARISA C++/JNI)
        --> Binary mmap Lookup + Bitmask Filter (DictionaryBinaryReader)
        --> Custom Dictionary Merge (CustomDictionaryService)
        --> User Frequency Scoring (UserFrequencyService)
        --> Next Word Context Boost (NextWordService)
      --> Candidate Display (CandidateView / SmartbarView)
        --> Case Transformation (SuggestionCaseTransformer)
    --> User Selection
      --> Text Commit (insertText / commitText)
      --> Frequency Recording (UserFrequencyService)
      --> Next Word Prediction (NextWordService)
    --> Reset to Idle
```

### Assessment

**Strengths:**
- Dual-state composing model (`rawInput` for search, `composingText` for display) is elegant and consistent across platforms
- Async candidate search prevents UI blocking (iOS: async/await, Android: Coroutines + 50ms debounce)
- Clear separation between trie prefix search and binary record lookup

**Concerns:**
- iOS custom dictionary is queried synchronously — potential bottleneck for large custom dictionaries
- NextWord prediction fires on every candidate selection

> **Note (2026-04)**: SQLite batch lookup replaced by binary mmap + bitmask filter on both platforms. N+1 frequency lookup concern resolved.

---

## 2. Layer Separation

### Layer Map

```
Layer 5: UI / Presentation
  iOS:     App/, _Keyboard/, Autocomplete/Views/, Styling/
  Android: settings/, ui/, ime/text/smartbar/, ime/text/keyboard/

Layer 4: Input Processing / Business Logic
  iOS:     Actions/, Input/, Autocomplete/Services/
  Android: ime/text/TextInputManager, ime/text/composing/

Layer 3: Linguistic Engine
  iOS:     Input/Tone/, Lexicon/Trie/
  Android: ime/dictionary/ (TaigiPhonetics, ToneConverter, TPSConverter, InputNormalizer)

Layer 2: Data Access
  iOS:     Lexicon/Database/ (BinaryReaders + SQLite repos), Lexicon/Services/
  Android: ime/dictionary/ (BinaryReaders, LexiconService, TrieService, NextWordService)

Layer 1: Platform / Lifecycle
  iOS:     _Keyboard/KeyboardViewController
  Android: ime/core/TaigiKeyboard, ime/lifecycle/
```

### Assessment

| Layer | iOS | Android |
|-------|-----|---------|
| UI / Presentation | Well-separated. SwiftUI views don't directly access DB. | Mostly good. **KeyView has tone logic** (SRP leak from Layer 3 into Layer 5). |
| Business Logic | Good. ActionHandler extensions keep concerns modular. | **TextInputManager mixes too many concerns** (dispatch, caps, candidates, layout reload). |
| Linguistic Engine | Excellent. Pure functions, no side effects. | Good. Same pure logic, but **InputNormalizer does both NFD and diacritics→digits**. |
| Data Access | Good. Binary mmap readers for read-only data, SQLite for user data. | Good. Same pattern — binary readers + SQLite for user data. |
| Platform | Good. Clean delegation to services. | Good. LifecycleInputMethodService enables Compose/ViewModel. |

### Critical Layer Violations

1. **Android KeyView** — contains `adjustNasalMarkerCase()` tone logic (Layer 3 leaking into Layer 5)
2. **Android TextInputManager** — acts as both key dispatcher (Layer 4) and layout reloader (Layer 5)

> **Note (2026-04)**: iOS NextWordService dual SQLite connection pools issue resolved — dict queries now use `AssociationBinaryReader` (binary mmap). Only user_association.db remains SQLite.

---

## 3. Single Responsibility Audit

### Files Exceeding Single Responsibility

| File | Platform | LOC | Responsibilities | Severity |
|------|----------|-----|------------------|----------|
| **TextInputManager** | Android | 830 | Key dispatch, caps logic, candidate coordination, layout reload, subtype handling, text input | **High** |
| **NextWordService** | iOS | 705 | Prediction logic, DB connection pooling, bigram scoring, user association learning | **High** |
| **PrefHelper** | Android | 600+ | DataStore caching, SharedPrefs migration, 100+ getters/setters, flow observation | **High** |
| **SharedSettings** | iOS | 568 | 40+ UserDefaults properties, container URL management, biometric auth | **Medium** |
| **ActionHandler** (total) | iOS | 947 | Gestures, composing, NextWord, case, suggestions (split across 5 files) | **Medium** |
| **KeyView** | Android | 500+ | Key rendering, touch events, tone hints, popup trigger, case transformation | **Medium** |
| **KeyboardView** | Android | 600+ | Layout rendering, touch processing, popup management, color/font caching | **Medium** |
| **CandidateView** | iOS | 481 | Candidate bar, tool toolbar, input mode switcher, expansion toggle | **Low** |
| **Tab3** | iOS | 410 | Custom dict, frequency data, association data, backup/restore | **Low** |

### Recommended Extractions

**High Priority:**

| Current | Extract To | Estimated LOC |
|---------|-----------|---------------|
| TextInputManager (Android) | KeyDispatcher + TaigiInputProcessor + LayoutReloadManager | 250 + 350 + 150 |
| NextWordService (iOS) | NextWordPredictor (logic) + NextWordRepository (DB) | 200 + 400 |
| PrefHelper (Android) | InputPrefs + AppearancePrefs + DictionaryPrefs (data classes) | 200 + 200 + 200 |

**Medium Priority:**

| Current | Extract To | Estimated LOC |
|---------|-----------|---------------|
| KeyView tone logic (Android) | Move `adjustNasalMarkerCase()` to ToneUtilities | ~10 lines |
| SharedSettings (iOS) | Group into InputSettings + AppearanceSettings + DictionarySettings | 200 + 200 + 150 |
| KeyboardView caching (Android) | ColorCache + FontCache | ~100 each |

### Files with Excellent SRP (Worth Preserving)

| File | Platform | Why It Works |
|------|----------|-------------|
| ComposingManager | Both | Pure state machine, single source of truth |
| TaigiPhonetics | Both | Pure conversion logic, no side effects |
| TPSConverter | Both | Pure data transformation |
| TrieService | Both | Clean native binding wrapper |
| DiagnosticService | Both | Single purpose, minimal |
| CandidateClickHandler | Android | Extracted click logic from SmartbarManager |
| CandidateUpdateCoordinator | Android | Extracted debounce logic from TextInputManager |
| CapsStateManager | Android | Extracted caps state from TextInputManager |

---

## 4. Folder Structure Analysis

### iOS Structure Assessment

```
ios/Sources/TaigiKeyboard/
├── _Keyboard/          [!] Unconventional name (leading underscore)
├── Actions/            [OK] Clear purpose
├── Input/              [OK] Well-organized with Tone/ subfolder
├── Autocomplete/       [OK] Good Models/Services/Views split
├── Lexicon/            [OK] Excellent Database/Services/Trie/Models/Utils split
├── Layout/             [OK] Cohesive
├── Settings/           [OK] Small and focused
├── Styling/            [OK] Providers pattern is clean
├── App/                [!] Tab3/ subfolder has 7 files mixing 4 concerns
│   └── Tabs/
│       ├── Tab1/       [OK] Has Models/ and DetailViews/ subfolders
│       └── Tab3/       [!] Flat folder with 7 unrelated files
├── Callouts/           [OK] Small, focused
├── Localization/       [OK] Consistent
├── Diagnostics/        [OK] Single file
└── Emojis/             [OK] Single file
```

**iOS Issues:**
1. `_Keyboard/` — leading underscore is unconventional. Rename to `Core/` or `Controller/`
2. `App/Tabs/Tab3/` — 7 files mixing custom dictionary, frequency data, association data, and backup. Should split by feature
3. `Tab1`, `Tab2`, `Tab3`, `Tab4` — generic names that don't convey purpose

### Android Structure Assessment

```
android/.../taigikeyboard/
├── ime/
│   ├── core/           [OK] IME lifecycle
│   ├── text/           [!] Cluttered — mixes business logic with view components
│   │   ├── composing/  [OK] Clear purpose
│   │   ├── key/        [OK] Key model + view
│   │   ├── keyboard/   [!] Should be under a view/ hierarchy, not text/
│   │   ├── layout/     [OK] Data model + manager
│   │   └── smartbar/   [!] 14 files — too many responsibilities in one folder
│   ├── dictionary/     [OK] All linguistic services grouped
│   ├── media/          [OK] Emoji subsystem
│   ├── popup/          [OK] Small, focused
│   └── lifecycle/      [OK] Single file
├── settings/           [!] Activity classes only — screens are in ui/settings/
├── ui/
│   ├── components/     [OK] Reusable Compose components
│   ├── settings/       [!] 18 files — every settings screen in one flat folder
│   └── theme/          [OK] Small
├── localization/       [OK] Consistent with iOS
├── model/              [OK] Small
├── util/               [OK] Small
└── diagnostics/        [OK] Single file
```

**Android Issues:**
1. `ime/text/` — mixes business logic (`TextInputManager`, `CapsStateManager`) with UI concerns (`keyboard/`, `smartbar/`). Should split into `ime/text/` (logic) and `ime/view/` or `ime/ui/` (rendering)
2. `ime/text/smartbar/` — 14 files in one folder covering candidates, overlays, toolbar, and next word. Should split:
   - `smartbar/` → candidate bar + adapter
   - `overlay/` → layout/symbol/settings overlays
   - `toolbar/` → toolbar management
3. `settings/` vs `ui/settings/` — Activity wrappers separate from Compose screens creates confusion. Consider merging or making the relationship explicit
4. `ui/settings/` — 18 Compose screens in a flat folder. Group by feature area

### Recommended Folder Restructuring

**iOS:**
```
_Keyboard/ --> Core/    (rename only)
App/Tabs/Tab1/ --> App/Tabs/Home/
App/Tabs/Tab2.swift --> App/Tabs/Layout/Tab2.swift
App/Tabs/Tab3/ --> App/Tabs/Data/
  (split into: CustomDictionary/, FrequencyData/, Backup/)
App/Tabs/Tab4.swift --> App/Tabs/Settings/Tab4.swift
```

**Android:**
```
ime/text/ --> split into:
  ime/input/       (TextInputManager, CapsStateManager, CandidateUpdateCoordinator, composing/)
  ime/view/        (keyboard/, key/, smartbar/ views)

ime/text/smartbar/ --> split into:
  ime/view/smartbar/    (SmartbarView, CandidateAdapter, CandidateOverlayView)
  ime/view/overlay/     (LayoutSelection, SymbolSelection, SettingsSelection)
  ime/input/nextword/   (NextWordHandler)
  ime/view/toolbar/     (ToolbarManager)

settings/ + ui/settings/ --> consider:
  ui/settings/activities/  (Activity wrappers)
  ui/settings/screens/     (Compose screens)
  -- or simply merge Activity logic into Compose screens --
```

---

## 5. File Naming Analysis

### Cross-Platform Naming Alignment

| Concept | iOS Name | Android Name | Aligned? | Recommendation |
|---------|----------|-------------|----------|----------------|
| IME Entry | KeyboardViewController | TaigiKeyboard | Divergent (platform convention) | OK — follows platform norms |
| Action Dispatch | ActionHandler | TextInputManager | **Misaligned** | Standardize concept name in docs |
| Composing | ComposingManager | ComposingManager | Aligned | -- |
| Tone Core | TaigiPhonetics | TaigiPhonetics | Aligned | -- |
| Tone Orchestrator | ToneConverter | ToneConverter | Aligned | -- |
| TPS | TPSConverter | TPSConverter | Aligned | -- |
| Dictionary | LexiconService | LexiconService | Aligned | -- |
| Trie | TrieService | TrieService | Aligned | -- |
| Autocomplete | AutocompleteService | TaigiAutocompleteService | **Slight divergence** | Consider aligning |
| Case Transform | CaseTransformationService | CapsStateManager | **Misaligned** | Different scope, acceptable |
| Suggestion Case | SuggestionCaseTransformer | SuggestionCaseTransformer | Aligned | -- |
| Next Word | NextWordService | NextWordService | Aligned | -- |
| User Frequency | UserFrequencyService | UserFrequencyService | Aligned | -- |
| Custom Dictionary | CustomDictionaryService | CustomDictionaryService | Aligned | -- |
| Backup | BackupService | BackupService | Aligned | -- |
| Settings | SharedSettings | PrefHelper | **Misaligned** | Different pattern, acceptable |
| Input Mode | InputMode | ToneConverterModels.InputMode | **Slight divergence** | Consider aligning |
| Localization | LocalizedText | LocalizedText | Aligned | -- |

**Alignment rate**: 14/20 fully aligned, 3 acceptably divergent (platform convention), 3 worth aligning

### Misleading or Generic Names

| File | Platform | Issue | Suggested Name |
|------|----------|-------|---------------|
| `_Keyboard/` | iOS | Leading underscore is unconventional | `Core/` or `Controller/` |
| `Tab1`, `Tab2`, `Tab3`, `Tab4` | iOS | Generic; don't convey purpose | `HomeTab`, `LayoutTab`, `DataTab`, `SettingsTab` |
| `DetailActivity` | Android | Too generic | `FeatureDetailActivity` |
| `LocalizedText` | Both | Sounds like a single instance; is a collection | `LocalizedStrings` |
| `PreferenceDataStore` | Android | Sounds like a preference class; is a factory | `DataStoreProvider` |
| `LayoutSelectionOverlayView` | Android | Redundant suffix | `LayoutPickerView` |
| `SettingsOverlayContent` | Android | "Overlay" misleading; is a content builder | `SettingsMenuContent` (already used, inconsistent) |
| `KeyboardModels` | iOS | Vague | `KeyboardStateModels` or specific model names |
| `DictionaryModels` | Android | Same vagueness | `DictionaryDataModels` or split into individual files |
| `view_utils.kt` | Android | Snake_case breaks Kotlin convention | `ViewUtils.kt` |

### Naming Convention Issues

1. **Manager vs Service inconsistency** — Android uses both patterns without clear distinction:
   - "Manager": TextInputManager, SmartbarManager, LayoutManager, SubtypeManager
   - "Service": LexiconService, NextWordService, CustomDictionaryService
   - **Rule proposal**: "Manager" = lifecycle/state management, "Service" = stateless operations

2. **iOS extension file naming** — `ActionHandler+CharacterInput.swift` pattern is clear and consistent

3. **`view_utils.kt`** — only snake_case file in the Android project; should be `ViewUtils.kt`

---

## 6. Duplicate Implementation Analysis

### Justified Platform-Specific Duplication

These components are duplicated across iOS/Android but with **good reason** (different platform APIs):

| Component | Why Separate | Risk |
|-----------|-------------|------|
| KeyboardViewController / TaigiKeyboard | UIKit vs InputMethodService lifecycle | Low — platform-mandated |
| CandidateView / SmartbarView+RecyclerView | SwiftUI vs Android Views | Low — UI framework difference |
| SharedSettings / PrefHelper | UserDefaults vs DataStore | Low — platform storage APIs |
| TrieService (C bridge / JNI) | Different FFI mechanisms | Low — same MARISA C++ underneath |
| Layout system (TaigiLayouts / JSON assets) | Code-defined vs JSON-loaded | Medium — behavioral drift possible |

### Phonetic Engine Duplication (Flagged by Gemini)

The most significant duplication is the **phonetic conversion pipeline** (TaigiPhonetics, ToneConverter, TPSConverter, InputNormalizer), which is fully reimplemented on both platforms.

**Current state:**
- Both implementations reference `taigi-converter/` (Node.js) as canonical source
- Both have parallel test suites validating against the same reference
- Logic is stable (phonetic rules don't change often)

**Risk:** Behavioral drift when one platform is updated but not the other.

**Mitigation options (ascending effort):**
1. **Status quo + tests** — Keep separate implementations, ensure test parity. *(Current approach, working well)*
2. **Shared test data** — Generate test vectors from `taigi-converter/` that both platforms validate against
3. **KMP module** — Move phonetic logic to Kotlin Multiplatform. *(High effort, benefits unclear given stability)*
4. **C++ core** — Implement phonetics in C++ alongside MARISA trie. *(Highest effort)*

**Recommendation:** Option 2 (shared test vectors) provides the best risk/effort ratio. The phonetic logic is stable enough that full code sharing is not urgent.

### Cross-Platform Overlap in Non-Engine Code

| Component | Overlap | Recommendation |
|-----------|---------|----------------|
| Localization (Tab1-4Texts) | Identical string content, different syntax | Share via JSON → generate platform code |
| FeatureContent / FeatureContentLoader | Both load `tab1-features.json` | Already shared via JSON — good |
| SymbolData | Both define same symbol sets | Share via JSON data file |
| CopyrightData | Both define same copyright info | Share via JSON data file |

---

## 7. Cross-Platform Comparison Table

### Structural Correspondence

| iOS Folder | Android Folder | Alignment |
|------------|---------------|-----------|
| `_Keyboard/` | `ime/core/` | Conceptually aligned |
| `Actions/` | `ime/text/` (TextInputManager) | Conceptually aligned, different granularity |
| `Input/` | `ime/text/composing/` | Aligned |
| `Input/Tone/` | `ime/dictionary/` (Tone*.kt) | **Misaligned** — Android puts tone logic in dictionary folder |
| `Autocomplete/Services/` | `ime/text/composing/` (TaigiAutocompleteService) | **Misaligned** — different parent folders |
| `Autocomplete/Views/` | `ime/text/smartbar/` | Conceptually aligned |
| `Lexicon/` | `ime/dictionary/` | Aligned |
| `Lexicon/Database/` | *(mixed into ime/dictionary/)* | **iOS has better separation** |
| `Lexicon/Trie/` | *(mixed into ime/dictionary/)* | **iOS has better separation** |
| `Layout/` | `ime/text/layout/` | Aligned |
| `Settings/` | `settings/` + `ui/settings/` | Both need cleanup |
| `Styling/` | `ime/text/key/` + `ime/core/KeyboardColorSettings` | Divergent |
| `App/` | `settings/` + `ui/` | Divergent (platform convention) |
| `Localization/` | `localization/` | Aligned |
| `Diagnostics/` | `diagnostics/` | Aligned |

### Notable Structural Differences

1. **iOS `Lexicon/` is better organized** than Android `ime/dictionary/`:
   - iOS separates: `Database/`, `Services/`, `Trie/`, `Models/`, `Utils/`
   - Android: 16 files in a flat `dictionary/` folder
   - **Recommendation**: Android should adopt iOS's subfolder pattern

2. **Android `ime/text/smartbar/` (14 files)** has no iOS equivalent as a single folder:
   - iOS distributes this across: `Autocomplete/Views/`, `Actions/ActionHandler+Suggestions.swift`
   - iOS approach is cleaner

3. **Tone logic placement differs**:
   - iOS: `Input/Tone/` (part of input processing layer)
   - Android: `ime/dictionary/` (part of dictionary layer)
   - **Recommendation**: Tone conversion is input processing, not dictionary. Android should move to `ime/input/tone/` or `ime/text/tone/`

---

## 8. Prioritized Recommendations

### P0 — High Impact, Actionable Now

| # | Action | Platform | Effort | Impact |
|---|--------|----------|--------|--------|
| 1 | Extract `KeyDispatcher` + `TaigiInputProcessor` from `TextInputManager` | Android | High | Testability, readability |
| 2 | Move `adjustNasalMarkerCase()` from `KeyView` to `ToneUtilities` | Android | Low | Layer separation |
| 3 | Extract `NextWordRepository` from `NextWordService` | iOS | Medium | SRP, consistent DB pattern |

### P1 — Important, Plan for Next Version

| # | Action | Platform | Effort | Impact |
|---|--------|----------|--------|--------|
| 4 | Rename `_Keyboard/` to `Core/` | iOS | Low | Naming convention |
| 5 | Split `ime/text/` into `ime/input/` + `ime/view/` | Android | Medium | Folder clarity |
| 6 | Split `ime/text/smartbar/` into `smartbar/`, `overlay/`, `toolbar/` | Android | Medium | Folder clarity |
| 7 | Add subfolders to `ime/dictionary/`: `tone/`, `services/`, `models/` | Android | Medium | Match iOS structure |
| 8 | Split `PrefHelper` into domain-specific settings data classes | Android | High | SRP, maintainability |
| 9 | Rename `Tab1`-`Tab4` to descriptive names | iOS | Low | Readability |
| 10 | Rename `view_utils.kt` to `ViewUtils.kt` | Android | Low | Convention |

### P2 — Nice to Have

| # | Action | Platform | Effort | Impact |
|---|--------|----------|--------|--------|
| 11 | Share `SymbolData` / `CopyrightData` via JSON | Both | Low | Reduce duplication |
| 12 | Generate shared test vectors from `taigi-converter/` | Both | Medium | Prevent behavioral drift |
| 13 | Split `App/Tabs/Tab3/` into feature subfolders | iOS | Low | Folder clarity |
| 14 | Merge `settings/` Activities into Compose screens | Android | Medium | Simplify navigation |
| 15 | Standardize Manager vs Service naming convention | Both | Low | Consistency |

---

## Appendix A: File Count by Folder

### iOS

| Folder | Files | Notes |
|--------|-------|-------|
| _Keyboard/ | 8 | Controller + setup extensions |
| Actions/ | 5 | ActionHandler + 4 extensions |
| Input/ | 7 | Including Tone/ (4 files) |
| Autocomplete/ | 13 | Models (3) + Services (3) + Views (7) |
| Lexicon/ | 20 | Database (4) + Services (5) + Trie (3) + Models (6) + Utils (2) |
| Layout/ | 6 | Layout definitions + services |
| Settings/ | 2 | SharedSettings + InputMode |
| Styling/ | 5 | Button providers + helpers |
| App/ | 21 | Tabs, detail views, setup guide |
| Callouts/ | 2 | Action + tone callouts |
| Localization/ | 6 | Per-tab strings |
| Diagnostics/ | 1 | DiagnosticService |
| Emojis/ | 1 | EmojiService |
| **Total** | **97** | |

### Android

| Folder | Files | Notes |
|--------|-------|-------|
| ime/core/ | 9 | TaigiKeyboard + config |
| ime/text/ | 3+12 | Manager + composing(4) + key(5) + keyboard(3) |
| ime/text/smartbar/ | 14 | Candidates + overlays + toolbar |
| ime/text/layout/ | 3 | Layout data + manager |
| ime/dictionary/ | 16 | All linguistic services (flat) |
| ime/media/ | 9 | Emoji subsystem |
| ime/popup/ | 2 | Key popup |
| ime/lifecycle/ | 1 | Lifecycle base |
| settings/ | 8 | Activity wrappers |
| ui/components/ | 10 | Reusable Compose |
| ui/settings/ | 18 | Settings screens |
| ui/theme/ | 2 | Theme + typography |
| localization/ | 8 | Language management |
| model/ | 3 | Data models |
| util/ | 6 | Utilities |
| diagnostics/ | 1 | DiagnosticService |
| **Total** | **114** | |

## Appendix B: Reviewers' Notes

### Gemini Assessment (Independent)

> "The architecture is professionally structured and follows mobile best practices. However, the project is currently paying a maintenance tax by duplicating the most difficult parts of its logic (phonetics and dictionary searching) across two languages."

Key Gemini findings aligned with this review:
- TextInputManager (830 LOC) and ActionHandler (947 LOC) are oversized orchestrators
- `_Keyboard/` naming is unconventional
- `ime/text/` mixes business logic with view components
- Phonetic engine duplication is the biggest cross-platform risk
- Data flow pipeline is "exceptionally clear and consistent"

Gemini confidence score: **0.9/1.0**
