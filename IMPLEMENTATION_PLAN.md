# Implementation Plan: v3.4.8 Refactoring

## Overview
Cross-platform refactoring to reduce coupling, extract shared components, and improve code organization.
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

### Stage 4: Reduce Autocomplete cross-folder coupling ✅ (partially resolved)
**Goal**: Remove unnecessary imports leaking into Autocomplete/
**Analysis result**:
- `Tab1Texts`–`Tab4Texts` in Autocomplete — **already resolved by Stage 2** (overlays moved to Overlays/, Autocomplete has zero TabNTexts references now)
- `KeyboardModels.Fonts` in Autocomplete (12 refs) — **deferred**. `KeyboardModels` is used across 16 files in 7 folders; it's a global font utility that happens to live in `_Keyboard/`. Moving to `Styling/` would be semantically cleaner but touches 16 files for a pure rename with no coupling benefit (no circular deps). Not worth the risk.
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

### Stage 6: Non-App comments to English
**Status**: Not Started
**Goal**: Convert remaining Chinese comments in non-App folders to English (matching Stage 1 pattern)
**Scope**: _Keyboard/, Actions/, Autocomplete/, Callouts/, Input/, Layout/, Lexicon/, Styling/
**Rule**: English primary; Taiwanese Mandarin in parentheses only for proper nouns

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

## Session Log
- **2026-04-06** (iOS): Completed Stage 1 — App/ folder cleanup (3 commits)
- **2026-04-07** (Android): Stage 1 completed — cleanup, shared components, deprecated API fixes, SectionHeader unification
- **2026-04-07** (iOS): Dependency analysis completed — identified Stages 2–5
- **2026-04-07** (iOS): Stage 2 completed — moved 4 overlay files to Overlays/
- **2026-04-07** (iOS): Stage 3 completed — moved 4 phonetics files to Phonetics/, removed Input/Tone/
- **2026-04-07** (iOS): Stage 4 analyzed — Tab1-4Texts coupling already resolved by Stage 2; KeyboardModels.Fonts rename deferred (16 files, no coupling benefit); DiagnosticService moved to Tab4
- **2026-04-08** (iOS): Stage 5 completed — DebugLogger wrapper, 87→1 #if DEBUG, removed instance counting & dead test code, updated docs
