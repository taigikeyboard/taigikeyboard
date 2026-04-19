# Refactor Backlog

> **Type**: Report
> **Created**: 2026-03-21
> **Source**: Full codebase deep scan (iOS + Android)
> **Constraint**: All items carry refactoring risk — changes touch core logic, state management, or high-traffic code paths. Must be done carefully with testing.

---

## iOS

### HIGH — Core Logic Risk

#### ~~1. SyllableSegmenter~~ — REMOVED (v3.4.6)

#### 2. SharedSettings: Bidirectional TPS ↔ InputMode Sync
- **File**: `ios/Sources/TaigiKeyboard/Settings/SharedSettings.swift` (lines 136-163, 234-257)
- **Problem**: Setting `inputMode` triggers `keyboardLayoutType` setter, which may trigger `inputMode` setter again. Hidden cascading state dependencies.
- **Risk**: Silent recursive side effects; future modifications can easily break this
- **Fix direction**: Explicit state transition methods (`enterTPSMode()`, `exitTPSMode()`) that document all side effects
- **Why risky**: Settings are read on every keystroke; incorrect sync = wrong keyboard layout

#### 3. TPSConverter: Bidirectional Tables Without Single Source of Truth
- **File**: `ios/Sources/TaigiKeyboard/Input/TPSConverter.swift` (lines 15-82 vs 284-343)
- **Problem**: TPS→TL and TL→TPS are maintained as two separate arrays. If phonetics change, both must be updated. No shared mapping.
- **Risk**: One-sided update causes silent conversion errors
- **Fix direction**: Single mapping table, generate reverse direction at init time
- **Why risky**: Affects all TPS input/output; wrong mapping = garbled text

### MEDIUM — State & Structure

#### 4. ActionHandler: NextWord State Coupling
- **File**: `ios/Sources/TaigiKeyboard/Actions/ActionHandler.swift` (lines 31-46)
- **Problem**: NextWord state split across 4 loose properties: `lastSelectedWord`, `lastSelectionTime`, `isShowingNextWord`, `contextTimeoutTimer`. Reset requires updating all 4 consistently.
- **Fix direction**: Extract `NextWordState` struct
- **Why risky**: Inconsistent reset = stale next-word suggestions or UI glitches

#### 5. ActionHandler: Long Functions
- **File**: `ios/Sources/TaigiKeyboard/Actions/ActionHandler.swift` (lines 81-152) and `ActionHandler+CharacterInput.swift` (lines 12-104)
- **Problem**: `handle(_ gesture, on action)` is 71 lines, `handleCharacterInput()` is 93 lines — both mix multiple concerns
- **Fix direction**: Extract `handleSpacebarGesture()`, punctuation handling, TPS adjustment into separate functions
- **Why risky**: ActionHandler is the central input dispatch; incorrect extraction = dropped keystrokes

#### 6. SharedSettings: Boolean Naming Inconsistency
- **File**: `ios/Sources/TaigiKeyboard/Settings/SharedSettings.swift` (lines 165-335)
- **Problem**: Mixed patterns: `enableDoubleTapOO` (verb), `phahTaigiLayoutEnabled` (adjective suffix), `moeDictEnabled` (past participle). Should use `is-` prefix per Swift convention.
- **Why risky**: Property names may map to UserDefaults keys. Renaming requires migration logic to preserve existing user settings. Must verify key mapping before renaming.

#### 7. ~~Callouts+TaigiToneMaps: buildToneMap Complexity~~ ✅ Fixed (Stage 12)
- **File**: `ios/Sources/TaigiKeyboard/Callouts/Callouts+TaigiCalloutMaps.swift`
- **Resolution**: Extracted `combiningMark(for:mode:)` and `buildVariations(base:suffix:toneNumbers:mode:)` helpers; `buildToneMap` reduced from 93→42 lines

#### 8. CaseTransformationService: Unused Parameter
- **File**: `ios/Sources/TaigiKeyboard/Input/CaseTransformationService.swift` (lines 24-32)
- **Problem**: `transformForInput(isAutoCapitalizationEnabled:)` parameter never used in function body
- **Fix direction**: Remove parameter; update all call sites
- **Why risky**: Must find and update all callers; if a caller relies on the parameter for future logic, removing it loses the intent

#### 9. TaigiButtonContent: Deep Nesting
- **File**: `ios/Sources/TaigiKeyboard/Styling/TaigiButtonContent.swift` (lines 31-98)
- **Problem**: 4+ levels of nested if/else for image/text/hint/mode rendering
- **Fix direction**: Extract each branch into private computed properties
- **Why risky**: Button rendering is per-key, per-frame; incorrect extraction = visual glitches

---

## Android

### HIGH — Boilerplate & Safety

#### 1. PrefHelper: 46-Property Boilerplate
- **File**: `android/.../ime/core/PrefHelper.kt` (778 lines)
- **Problem**: 46 properties repeat identical get/set boilerplate with DataStore (~600 lines). Adding a new preference requires copy-pasting 12 lines.
- **Fix direction**: Kotlin property delegate (`PreferenceProperty<T>`) that encapsulates get/set/cache pattern
- **Why risky**: PrefHelper is read on every keystroke and UI update. Incorrect delegate = settings not persisted or wrong defaults. Must preserve migration from SharedPreferences.

#### 2. PrefHelper: TPS ↔ InputMode Sync (mirrors iOS issue)
- **File**: `android/.../ime/core/PrefHelper.kt` (lines 163-213, 298-344)
- **Problem**: Same bidirectional sync issue as iOS SharedSettings
- **Fix direction**: Same as iOS — explicit transition methods
- **Why risky**: Same as iOS — affects keyboard layout selection

### MEDIUM — File Size & Architecture

#### 3. LexiconService.kt Decomposition
- **File**: `android/.../ime/dictionary/LexiconService.kt`
- **Problem**: Single file handles trie queries, binary reader calls, deduplication, sorting, filtering
- **Fix direction**: Split into `TrieQueryService`, `BinaryReaderService`, `DictionaryResultProcessor`
- **Why risky**: Central dictionary lookup path; incorrect split = broken autocomplete

#### 4. TextInputManager.kt Decomposition
- **File**: `android/.../ime/text/TextInputManager.kt` (794 lines)
- **Problem**: Core logic mixed with 6 handler delegations
- **Fix direction**: Extract `KeyEventDispatcher`, move handler init to factory
- **Why risky**: All key input flows through this; incorrect extraction = input drops

#### 5. KeyView.kt Touch Refactor
- **File**: `android/.../ime/text/key/KeyView.kt` (924 lines)
- **Problem**: Touch handling is 80+ lines of nested when-block with `osHandler?.postDelayed()` calls
- **Fix direction**: Extract `KeyTouchHandler` with state machine (DOWN/MOVE/UP)
- **Why risky**: Touch timing is critical for repeat keys, long-press, flick; wrong timing = missed inputs

#### 6. SmartbarManager: Boolean State → Enum
- **File**: `android/.../ime/text/smartbar/SmartbarManager.kt` (lines 57-70)
- **Problem**: 4 boolean flags (`isExpanded`, `hasCandidates`, `cachedIsTranslateSwapped`, `cachedOutputBothScripts`) interact with complex visibility logic
- **Fix direction**: Replace with `SmartbarState` enum (EMPTY, COLLAPSED, EXPANDED)
- **Why risky**: Smartbar visibility is user-facing; wrong state = hidden candidates or broken UI

#### 7. PrefHelper: Infinite Flow Collection
- **File**: `android/.../ime/core/PrefHelper.kt` (lines 37-44)
- **Problem**: `warmUp()` spawns infinite `collect` on DataStore. Race condition possible if cache checked before first emission.
- **Fix direction**: Use `shareIn(scope, SharingStarted.Eagerly)` or `stateIn()`
- **Why risky**: Affects all preference reads during app startup

---

## Recommended Approach

1. **One module per session** — each item above is a self-contained refactor
2. **Write tests first** for the affected behavior before changing structure
3. **iOS and Android in parallel** for items that mirror each other (TPS sync, PrefHelper/SharedSettings)
4. **Prioritize by user impact**: PrefHelper boilerplate (A1) > ActionHandler long functions (5) > rest. (Item 1, Segmenter trie, no longer applies — `SyllableSegmenter` removed in v3.4.6.)
