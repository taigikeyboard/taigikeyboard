# Implementation Plan: v3.4.8 Refactoring

## Overview
Cross-platform refactoring to reduce coupling, extract shared components, and improve code organization.
This refactoring aims to build the right foundation for the future — do the correct thing now, handle it carefully.
Branch: `rel-v3.4.8-bugfix`

---

## iOS Refactoring

### Stage 1: App/ folder cleanup ✅
**Commits**: 893a629, 39e128f, 08e6352
**What was done**:
- Centralized AppStyle font size constants, removed FontManager from main app
- Reorganized Tabs/ folder structure (Tab2/, Tab3/, Tab4/ subfolders)
- Extracted shared components: CSVDocument, SearchBar, DictionaryInfo, KeyboardPreviewPanel
- Removed dead code (unused state vars, methods, SectionHeader struct)
- Merged headlineSize into sectionHeaderSize, removed redundant style modifiers
- Converted all App/ comments to English

### Stage 2: Extract Overlays from Autocomplete ✅
**Goal**: Move overlay views out of Autocomplete/ into a dedicated folder
**Why**: Autocomplete/ mixes candidate search logic with keyboard overlay UI (settings panel, layout picker, symbol grid, expanded candidates). These overlays are not autocomplete — they're keyboard UI panels.
**Files moved**:
- `Autocomplete/Views/SettingsSelectionOverlay.swift` → `Overlays/`
- `Autocomplete/Views/LayoutSelectionOverlay.swift` → `Overlays/`
- `Autocomplete/Views/SymbolSelectionOverlay.swift` → `Overlays/`
- `Autocomplete/Views/ExpandedCandidateOverlay.swift` → `Overlays/`
**Note**: Swift same-target files don't need import updates. User must update Xcode project groups manually.

### Stage 3: Extract Phonetics from Input ✅
**Goal**: Separate pure phonetics logic from input state management
**Why**: Input/ mixes two concerns: (1) composition state (ComposingManager) and (2) phonetics/tone utilities (TaigiPhonetics, ToneConverter, ToneUtilities, ToneRestoration). The phonetics layer is pure logic with no dependencies — good candidate for isolation.
**Files moved**:
- `Input/Tone/TaigiPhonetics.swift` → `Phonetics/`
- `Input/Tone/ToneConverter.swift` → `Phonetics/`
- `Input/Tone/ToneRestoration.swift` → `Phonetics/`
- `Input/Tone/ToneUtilities.swift` → `Phonetics/`
- Removed empty `Input/Tone/` directory
**Note**: User must update Xcode project groups manually.

### Stage 4: Reduce Autocomplete cross-folder coupling ✅
**Goal**: Remove unnecessary imports leaking into Autocomplete/
**Analysis result**:
- `Tab1Texts`–`Tab4Texts` in Autocomplete — **already resolved by Stage 2** (overlays moved to Overlays/, Autocomplete has zero TabNTexts references now)
- `KeyboardModels.Fonts` in Autocomplete — **resolved in Stage 6**. Moved `_Keyboard/KeyboardModels.swift` → `Styling/KeyboardFonts.swift`, flattened `KeyboardModels.Fonts` → `KeyboardFonts` (removed unnecessary double namespace). Updated 30 references across 11 files. Eliminates reverse dependency from 5 folders (Settings, Autocomplete, Overlays, Styling, App) back to `_Keyboard/`.
- Moved `DiagnosticService.swift` from `Diagnostics/` → `App/Tabs/Tab4/` (only consumer); removed empty `Diagnostics/` folder

### Stage 5: Unify DEBUG logging pattern ✅
**Goal**: Replace scattered `#if DEBUG` + `Logger` + `privacy: .public` with unified `DebugLogger` wrapper
**What was done**:
- Created `DebugLogger.swift` — `#if DEBUG` wraps real `os.Logger`, `#else` is no-op with `@autoclosure` (zero cost in release)
- Migrated all 22 files from `Logger` to `DebugLogger`
- Removed 87 `#if DEBUG` blocks → 1 remains (in `DebugLogger.swift` itself)
- Removed 68 `privacy: .public` annotations from call sites (handled internally by wrapper)
- Removed 21 `import OSLog` (only `DebugLogger.swift` imports it)
- Removed instance counting dead code from `KeyboardViewController` and `CandidateExpandState`
- Removed unused debug test methods (`insertTestData`, `testData`, debug `deleteDatabase`) from `UserFrequencyRepository` and `UserFrequencyService`
- Updated `rules/security-rules.md` to reflect new `DebugLogger` convention
- Removed `docs/debug-log.md` (redundant with security-rules.md), updated `docs/README.md`
**New file**: `DebugLogger.swift` (root of TaigiKeyboard/)

### Stage 6: KeyboardViewController review & cleanup ✅
**Branch**: `refactor/ios-review`
**Goal**: Review KeyboardViewController startup flow, fix issues found
**Principle**: Follow clean code — extract repeated expressions, tighten access control, remove dead code
**What was done**:
- Removed boilerplate `init(nibName:bundle:)` and `init?(coder:)` — Swift inherits automatically
- Fixed `setupServices`/`ensureEssentialServicesInitialized` merge — autocomplete service was created twice (KeyboardKit default then replaced); ActionHandler now gets the correct service directly
- Removed redundant `ensureEssentialServicesInitialized()` — lazy var triggers already done in ActionHandler init
- Removed redundant `ensureCleanState()` — state is already clean at `viewDidLoad` time
- Removed redundant `ActionHandler.keyboardViewController` — replaced with inherited `keyboardController` (KeyboardKit's weak ref)
- Removed redundant `setKeyboardCase` override — was pure debug log, no protection logic
- Decoupled AutocompleteService from ActionHandler/ComposingManager — introduced `ComposingStateProvider` and `SelectionContextProvider` protocols
- Migrated `settingsObserver` from NotificationCenter block to Combine — eliminated manual cleanup, removed `removeSettingsObserver()`, simplified `performCleanup()`
- Removed dead code in `syncSettings()` — hardcoded KeyboardKit key read only used in log
- Added FIXME markers on 2-layer auto-capitalization workaround
- Updated deinit TODO (settingsObserver migration done)
- viewDidAppear: guarded `isFullAccessEnabled` write to avoid unnecessary notification → double `syncSettings()`
- syncSettings: removed `syncToKeyboardContext` (constant `spacebarLongPressBehavior` moved to one-time `setupServices()`); initialized `lastInputMode`/`lastKeyboardLayoutType` in `setupCoreServices()` to prevent redundant first-launch service rebuild; merged double `autocompleteContext.reset()` on TPS switch with `needsAutocompleteReset` flag
- Removed `SharedSettings.syncToKeyboardContext` extension (no longer used)
- viewWillSetupKeyboardView: removed redundant `controller` parameter and `as? KeyboardViewController` cast — unified to `[unowned self]` per ios-guidelines
- Renamed `createQwertyKeyboardView` → `createKeyboardView` (handles all layout types, not just QWERTY)
- createCalloutStyle: extracted `FontType.customFontName` computed property, eliminated duplicated `.openHuninn`/`.iansui` switch cases
- Removed `viewDidDisappear` — redundant with `viewWillDisappear` + `deinit` (both call `performCleanup()` with `isCleanedUp` guard)
- textDidChange: unified `actionHandler` access (removed `services.actionHandler as? ActionHandler` cast)
- textDidChangeAsync: removed 6-line NextWord debug trace, kept 1-line auto-cap log
**New file**: `AutocompleteProviders.swift` (ComposingStateProvider + SelectionContextProvider protocols)
**Review criteria**: Remove redundant code, scrutinize necessity, eliminate duplication (logic & variables)
**Review progress** (KeyboardViewController lifecycle):
- [x] Properties
- [x] Initialization (init/deinit)
- [x] viewDidLoad → FontRegistration
- [x] viewDidLoad → setupServices / setupCoreServices
- [x] viewDidLoad → ensureCleanState (removed)
- [x] viewDidLoad → setupSettingsObserver
- [x] viewDidLoad → setupKeyboardCaseProtection (FIXME, workaround)
- [x] viewDidAppear
- [x] viewWillSetupKeyboardView / createKeyboardView / createCalloutStyle
- [x] viewWillDisappear (viewDidDisappear removed)
- [x] textDidChange / textDidChangeAsync
- [x] autocompleteText
- [x] syncSettings (auto-cap section) — clean, no issues
- [x] setupKeyboardCaseProtection (FIXME) — necessarily complex, cannot simplify; removed ActionHandler `.keyboardType` debug trace (11 lines, purely diagnostic)
- [x] EmojiDelegate — clean, no issues
- [x] TextInput — clean; fixed `ComposingManager.selectSuggestion` duplicating `clearMarkedText()` logic
- [x] TaigiKeyboardView — clean, no issues
- [x] `_Keyboard/` folder review — moved `KeyboardModels.swift` → `Styling/KeyboardFonts.swift`, flattened namespace (30 refs updated, 11 files)
- [x] ComposingManager decoupling — introduced `ComposingDelegate` protocol, replaced `weak var keyboardViewController: KeyboardViewController?` with `weak var delegate: (any ComposingDelegate)?`. Input/ no longer depends on _Keyboard/. Removed `deleteBackwardManually()` (replaced by protocol `deleteBackward()`)

### Stage 7: Actions/ review & cleanup
**Branch**: `refactor/ios-review-actions`
**Goal**: Review ActionHandler and extensions — clean code, English comments, remove dead code
**Principle**: Follow clean code — extract repeated expressions, tighten access control, remove dead code
**What was done**:
- Converted all Chinese comments to English (5 Actions/ files + 4 _Keyboard/ files, ~50 comments)
- Removed redundant extension doc comments and inline comments that restated code
- Removed dead constant `contextTimeoutMs` (unused, superseded by `contextTimeoutSeconds`)
- Extracted `currentTimestampMs` computed property (was `Int64(Date()...* 1000)` repeated 4 times across 2 files)
- `private(set)` on `lastSelectedRoman`, `lastSelectionTime`, `isShowingNextWord` — restrict external writes
- Moved hardcoded punctuation string → `NextWordConstants.noisePunctuation`
- Removed unused `rawInput _:` parameter from `handleNextWordPrediction` and `handleEnterNextWordPrediction`
- Fixed `ComposingDelegate` conformance — moved `insertText`/`deleteBackward` overrides to class body (Swift requires override in class, not extension)
- Fixed duplicate `// MARK: - Settings Observer` in KeyboardViewController
- Replaced WHAT comments with WHY comments in TextInput (markedText cursor positioning, two-step UITextInput clear)
- Ran simplify 3-agent review on both Actions/ and _Keyboard/ — fixed safe issues, noted future work
- Build fix: reverted `private(set)` (Swift `private` is file-scoped, blocks cross-file extensions), removed dead `capturedRawInput` and stale caller `rawInput:` argument
**Noted for future** (from simplify review):
- Stringly-typed `additionalInfo` keys → needs separate PR (touches many files)
- Panel bools → enum consolidation in TaigiKeyboardView
- Prediction filtering → move from ActionHandler to NextWordService
**Status**: ✅

### Stage 8: NextWord State deduplication
**Branch**: `refactor/ios-review-actions`
**Goal**: Eliminate duplicated NextWord logic across 3 entry points
**Principle**: One flow, one method — use parameters to control behavior variants
**Rollback**: Each step = 1 commit, can `git revert` individually

**Problem analysis** (5 issues identified):
1. `handleNextWordPrediction` ≈ `handleEnterNextWordPrediction` (~90% identical code)
2. `hanzi` parameter in `handleNextWordPrediction` is dead (unused in body)
3. `updateNextWordState` vs `updateLastSelectedWord` — overlapping state updates
4. `recordCompoundWordAssociations` called from 3 scattered locations
5. Noise filtering logic duplicated across 3 entry points with slight variations

**Steps**:

- [ ] **Step A**: Remove dead `hanzi` parameter from `handleNextWordPrediction`
  - Files: `ActionHandler+Suggestions.swift` (declaration + call site)
  - Risk: Zero — parameter unused in body
  - Commit separately

- [ ] **Step B**: Unify `handleNextWordPrediction` + `handleEnterNextWordPrediction` into single method
  - Signature: `processNextWord(text:roman:triggerPrediction:requireRomanMode:)`
  - `handleNextWordPrediction` → `processNextWord(triggerPrediction: true, requireRomanMode: false)`
  - `handleEnterNextWordPrediction` → `processNextWord(triggerPrediction: true, requireRomanMode: true)`
  - Preserves sentence-end punctuation reset from original `handleNextWordPrediction`
  - Files: `ActionHandler+Suggestions.swift`
  - Risk: Low — same logic, just merged

- [ ] **Step C**: Absorb `updateLastSelectedWord` into `processNextWord`
  - Space path → `processNextWord(triggerPrediction: false, requireRomanMode: false)`
  - Remove `updateLastSelectedWord`, remove `updateNextWordState` (inlined into `processNextWord`)
  - Files: `ActionHandler.swift`, `ActionHandler+Suggestions.swift`, `ActionHandler+CharacterInput.swift`
  - Risk: Medium — Space path semantics change slightly (adds TL normalization, consistent noise filtering)
  - Verify: Space commit still records associations correctly

**After completion**: 3 entry points → 1 method, ~50 lines removed

**What was done**:
- Removed dead `hanzi` parameter from `handleNextWordPrediction` (declaration + call site)
- Replaced `handleNextWordPrediction` + `handleEnterNextWordPrediction` with unified `processNextWord(text:roman:requireRomanMode:triggerPrediction:)` in `ActionHandler+Suggestions.swift`
- Replaced `updateLastSelectedWord` (Space path) call with `processNextWord(triggerPrediction: false)`
- Removed `updateNextWordState` and `updateLastSelectedWord` from `ActionHandler.swift` (state update inlined into `processNextWord`)
- Call sites updated: `ActionHandler+Suggestions.swift:86`, `ActionHandler+CharacterInput.swift:151,228`
- AI-friendliness improvements:
  - Added action flow overview to `ActionHandler` class doc (gesture → dispatch → handler → processNextWord → predict)
  - Added file-level overview comments to all 4 extension files
  - Added WHY comment on `handleBackspaceForNextWord` explaining why it bypasses `processNextWord` (backspace is not a word selection — no association/compound recording)
  - Made FIXME cross-reference precise: "see KeyboardViewController.setupKeyboardCaseProtection() (Layer 2)"

**Revert guide**:
- All changes are in one commit on `refactor/ios-review-actions`
- To revert NextWord dedup only: restore these 3 deleted methods and their call sites:
  - `handleNextWordPrediction(displayText:roman:)` → private in `ActionHandler+Suggestions.swift`, called from `handleSuggestionSelection`
  - `handleEnterNextWordPrediction(committedText:)` → func in `ActionHandler+Suggestions.swift`, called from `handleReturnAction`
  - `updateNextWordState(selectedWord:roman:)` → func in `ActionHandler.swift`, called by the above two
  - `updateLastSelectedWord(_:roman:)` → func in `ActionHandler.swift`, called from `handleSpaceAction`
- Then remove `processNextWord` and update call sites back
- Comment/doc changes are independent and safe to keep even if logic is reverted

**Manual test results**: All 9 scenarios passed (candidate selection → predict, Enter → predict, Space → no predict, new letter clears, punctuation resets, digit clears, backspace re-predicts, "-" preserves, 30s timeout clears)
**Status**: ✅

### Stage 8b: App/ simplify scan fixes
**Branch**: `refactor/ios-review-actions`
**What was done**:
- Unified 3 slider row methods → single `sliderRow(label:value:in:step:defaultValue:onChanged:)` in AppearanceSettingsView
- Added spacing/cornerRadius constants to AppStyle: `horizontalPadding`(16), `innerHorizontalPadding`(12), `verticalPadding`(8), `cardCornerRadius`(12), `previewCornerRadius`(10), `smallCornerRadius`(8)
- Replaced hardcoded values across 6 files: SearchBar, Tab2, Tab3, CustomDictionaryView, FAQDetailView, SetupGuideView
- Fixed 3 empty `catch {}` blocks in Tab4 and FrequencyDataView → DebugLogger error logging
**Status**: ✅

### Stage 9: Tab3 data view deduplication
**Branch**: `refactor/ios-review-actions`
**Goal**: Extract shared import/export pattern from CustomDictionaryView, FrequencyDataView, AssociationDataView
**Why**: 3 views share identical import/export state vars (8 each), alert chain (3 modifiers), file importer/exporter, export filename pattern, and import handler boilerplate
**Approach**: Option B — `ImportExportHandler` (ObservableObject) + `ImportExportModifiers` (ViewModifier)

**What was done**:
- Created `ImportExportHandler.swift` — `@MainActor ObservableObject` holding 8 shared `@Published` state vars + `performExport()` + `handleFileImport()` + `exportFilename(prefix:)` utility
- Created `ImportExportModifiers` ViewModifier — attaches `.fileImporter`, `.fileExporter`, and 3 `.alert` modifiers (import result, export success, error) in one call
- Added `View.importExportModifiers(handler:...)` convenience extension
- Updated `FrequencyDataView`: 8 `@State` → 1 `@StateObject`, 5 modifiers → 1, removed `exportFilename()` + `exportFrequencyCSV()` + `handleFrequencyImport()`, added `exportCSV()` (returns String) + `handleImport()` (delegates to handler)
- Updated `AssociationDataView`: same pattern — 8 `@State` → 1 `@StateObject`, removed 3 methods, added 2 simplified methods
- Updated `CustomDictionaryView`: same pattern — 8 `@State` → 1 `@StateObject`, removed `customDictExportFilename()` + `exportCSV()` + `handleFileImport()`, export now inlines `service.exportCSV()` directly
**New file**: `ImportExportHandler.swift` (App/Tabs/Tab3/)
**Net change**: ~−120 lines across 3 views, +120 lines in new file (zero duplication vs 3× duplication)
**Status**: ✅

---

## Android Refactoring

### Stage 1: Cleanup & extract shared UI components ✅
**What was done**:
- Removed unused imports (DetailScreen, InputModeScreen, FontPickerContent, DictionarySettingsScreen)
- Removed unused `total` state var from AssociationDataScreen
- Extracted `CsvUtils` to `util/` (shared CSV parse/escape)
- Extracted `ConfirmationDialog`, `ResultDialog` to `ui/components/`
- Fixed deprecated APIs (`largeTopAppBarColors`, `OpenInNew`)
- Unified `SectionHeader` composable with default padding (13 inline patterns replaced)
- Added spacing constants to `AppStyle`
- Converted Chinese comments to English (Type.kt, InputSettingsScreen, DictionarySettingsScreen)
**New files**: `CsvUtils.kt`, `ConfirmationDialog.kt`, `ResultDialog.kt`

### Stage 2: (TBD — depends on Stage 1 completion)

---

## Future Work (post-v3.4.8)

### NextWord decoupling from ActionHandler
**Problem**: ActionHandler 同時負責「鍵盤動作分派」和「NextWord 控制層」兩個職責。NextWord 在 ActionHandler 中佔 ~14 methods/properties + 1 enum（超過一半程式碼）。`NextWordService` 只負責資料層（DB query），控制層邏輯全部散在 ActionHandler。

**耦合點**:
1. NextWord 狀態（4 vars + timer）住在 ActionHandler
2. NextWord 需讀 `settings`（inputMode, isTranslateSwapped）
3. NextWord 需寫 `keyboardController.state.autocompleteContext` 更新 UI
4. `shouldSkipAutocomplete()` 讀取 `isShowingNextWord`
5. 四個 action handler（select、space、enter、backspace）都呼叫 NextWord

**方向**: 提取 `NextWordController`，持有狀態 + 預測 + 關聯記錄邏輯。ActionHandler 只在動作發生時呼叫 `nextWordController.process(...)`。

**涉及檔案**: `ActionHandler.swift`（狀態 + 預測）、`ActionHandler+Suggestions.swift`（processNextWord + compound word）、`ActionHandler+KeyActions.swift`（呼叫端）

---

## Session Log
- **2026-04-06** (iOS): Completed Stage 1 — App/ folder cleanup (3 commits)
- **2026-04-07** (Android): Stage 1 completed — cleanup, shared components, deprecated API fixes, SectionHeader unification
- **2026-04-07** (iOS): Dependency analysis completed — identified Stages 2–5
- **2026-04-07** (iOS): Stage 2 completed — moved 4 overlay files to Overlays/
- **2026-04-07** (iOS): Stage 3 completed — moved 4 phonetics files to Phonetics/, removed Input/Tone/
- **2026-04-07** (iOS): Stage 4 analyzed — Tab1-4Texts coupling already resolved by Stage 2; KeyboardModels.Fonts rename deferred (16 files, no coupling benefit); DiagnosticService moved to Tab4
- **2026-04-08** (iOS): Stage 5 completed — DebugLogger wrapper, 87→1 #if DEBUG, removed instance counting & dead test code, updated docs
- **2026-04-08** (iOS): Stage 6 in progress — KeyboardViewController review, fixed service init order, protocol decoupling, Combine migration, dead code removal
- **2026-04-09** (iOS): Stage 6 continued — reviewed syncSettings auto-cap (clean), setupKeyboardCaseProtection (cannot simplify), removed ActionHandler `.keyboardType` debug trace; reviewed EmojiDelegate (clean), TextInput (fixed duplicated clearMarkedText in ComposingManager.selectSuggestion), TaigiKeyboardView (clean); `_Keyboard/` folder review: moved `KeyboardModels.swift` → `Styling/KeyboardFonts.swift` (flattened namespace, 30 refs across 11 files); decoupled ComposingManager from KeyboardViewController via `ComposingDelegate` protocol (Input/ no longer depends on _Keyboard/)
- **2026-04-09** (iOS): _Keyboard/ comment & naming review — converted all Chinese comments to English (4 files), renamed `emojiSvc` → `emojiServiceStorage`, removed redundant doc comments that restated function names, trimmed verbose comments to keep only "why" context (net −29 lines)
- **2026-04-09** (iOS): Stage 7 — Actions/ + _Keyboard/ review with simplify 3-agent scan. English comments, dead code removal (`contextTimeoutMs`, unused `rawInput` params), `private(set)` access control, `currentTimestampMs` helper, punctuation constant extraction, ComposingDelegate override fix, duplicate MARK fix
- **2026-04-09** (iOS): Stage 8 — NextWord State deduplication. Merged 3 entry points (`handleNextWordPrediction`, `handleEnterNextWordPrediction`, `updateLastSelectedWord`) into unified `processNextWord`. Removed `updateNextWordState`. Added AI-friendly docs (action flow overview, extension file headers, WHY comments, precise FIXME refs). 9/9 manual tests passed
- **2026-04-09** (iOS): Stage 8b — App/ simplify scan. Unified 3 slider rows in AppearanceSettingsView, added AppStyle spacing/cornerRadius constants (6 values), replaced hardcoded values across 6 files, fixed 3 empty catch blocks → DebugLogger. Recorded Stage 9 (Tab3 data view dedup) for future session
- **2026-04-09** (iOS): Stage 9 — Tab3 data view deduplication. Created `ImportExportHandler` (ObservableObject + ViewModifier) extracting 8 shared @State vars, 5 shared modifiers, export/import flow. Updated CustomDictionaryView, FrequencyDataView, AssociationDataView. New file: `ImportExportHandler.swift`
