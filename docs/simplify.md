# Simplify Review Plan

Batch-by-batch codebase review using `/simplify`. Each batch is scoped to fit in one context window. Mark status as you go.

**Convention**: Run `/simplify <path>` for each batch below.

**Scope**: This is not limited to recent changes. Deep-scan all code in each batch for reuse, quality, and efficiency issues.

---

## Android (109 .kt files, 14 batches)

### Batch A1: IME Core
**Path**: `android/app/src/main/java/com/siansiansu/taigikeyboard/ime/core/`
**Files (8)**: TaigiKeyboard, LifecycleInputMethodService, InputView, PrefHelper, PreferenceDataStore, KeyboardColorSettings, NavigationBarManager, Subtype, SubtypeManager
**Focus**: Service lifecycle, preference atomicity, initialization order
**Status**: [x] Done — added serviceScope.cancel() in onDestroy; removed dead initDefaultPreferences(); coroutine leak in PrefHelper.scope noted

### Batch A2: Dictionary — Phonetics & Segmentation
**Path**: `android/.../ime/dictionary/` (subset: SyllableSegmenter, TaigiPhonetics, InputNormalizer, TPSConverter)
**Focus**: Segmentation correctness, normalization pipeline, TPS conversion
**Status**: [x] Done — extracted stopConsonants constant in TPSConverter.kt

### Batch A3: Dictionary — Tone Conversion
**Path**: `android/.../ime/dictionary/` (subset: ToneConverter, ToneConverterModels, ToneCharacterUtils, ToneRestoration, ToneUtilities)
**Focus**: POJ/TL conversion correctness, edge cases, code reuse
**Status**: [x] Done — removed dead pojToneMapping/tlToneMapping maps; unified uppercaseToneLetter/fullUppercaseToneLetter; added early-exit in adjustNasalMarkerCase; vowel check uses Set

### Batch A4: Dictionary — Lookup & Services
**Path**: `android/.../ime/dictionary/` (subset: LexiconService, TrieService, NextWordService, CustomDictionaryService, DictionaryModels, DictionaryConstants, SuggestionCaseTransformer)
**Focus**: Thread safety, query efficiency, atomic reads
**Status**: [x] Done — added @Volatile to LexiconService.database and isInitialized for thread visibility

### Batch A5: Text Input — Core
**Path**: `android/.../ime/text/` (top-level: TextInputManager, CapsStateManager, CandidateUpdateCoordinator)
**Focus**: Concurrency (locks, handlers), input coordination, state management
**Status**: [x] Done — reviewed; composingLock partial coverage noted, design is safe by value-equality guard

### Batch A6: Text Input — Composing
**Path**: `android/.../ime/text/composing/`
**Files (4)**: ComposingManager, TaigiAutocompleteService, EnglishAutocompleteService, UserFrequencyService
**Focus**: Composition state, autocomplete ranking, frequency tracking
**Status**: [x] Done — reviewed with A5; EnglishAutocompleteService pendingSuggestions race noted

### Batch A7: Text Input — Layout & Key
**Path**: `android/.../ime/text/layout/` + `android/.../ime/text/key/`
**Files (8)**: LayoutManager, LayoutType, LayoutData, KeyView, KeyData, KeyCode, KeyType, KeyVariation
**Focus**: Layout switching logic, key rendering, display overrides
**Status**: [x] Done — cached Moshi instance as companion object singleton in LayoutManager

### Batch A8: Text Input — Keyboard Views
**Path**: `android/.../ime/text/keyboard/`
**Files (3)**: KeyboardView, KeyboardRowView, KeyboardMode
**Focus**: View rendering, measurement, mode switching
**Status**: [x] Done — reviewed with A7; full layout rebuild pattern noted (no view recycling)

### Batch A9: Smartbar
**Path**: `android/.../ime/text/smartbar/`
**Files (10)**: SmartbarView, SmartbarManager, CandidateAdapter, CandidateOverlayView, CandidateOverlayAdapter, CandidateClickHandler, NextWordHandler, ToolbarManager, SmartbarQuickActionButton, LayoutSelectionOverlayView
**Focus**: Adapter efficiency, click handling, state coordination
**Status**: [x] Done — reviewed with A7; redundant notifyDataSetChanged after submitList noted

### Batch A10: Popup & Media
**Path**: `android/.../ime/popup/` + `android/.../ime/media/` + `android/.../ime/media/emoji/`
**Files (10)**: KeyPopupManager, KeyPopupExtendedSingleView, MediaInputManager, EmojiKeyboardView, EmojiPaletteView, EmojiCategory, EmojiSet, EmojiKeyData, EmojiLayoutData, EmojiPreferences
**Focus**: Popup lifecycle, emoji rendering, memory
**Status**: [x] Done — reviewed with A7; EmojiKeyView dead code branches noted

### Batch A11: Settings — Compose Screens
**Path**: `android/.../ui/settings/`
**Files (14)**: MainSettingsScreen, DetailScreen, HomeScreen, InputSettingsScreen, InputModeScreen, AppearanceSettingsScreen, LayoutScreen, DictionarySettingsScreen, CustomDictionaryScreen, SetupGuideScreen, CopyrightScreen, KeyboardPreviewPanel, FontPickerContent, ColorPickerDialog
**Focus**: State management, navigation, preview correctness
**Status**: [x] Done — reviewed; good component reuse, no issues

### Batch A12: Settings — Legacy Activities
**Path**: `android/.../settings/`
**Files (8)**: SettingsMainActivity, DetailActivity, AppearanceSettingsActivity, CustomDictionaryActivity, CopyrightActivity, SetupGuideActivity, DebugActivity, DebugListFragment
**Focus**: Dead code candidates, migration to Compose
**Status**: [x] Done — reviewed; DebugActivity Fragment pattern is outlier (rest uses Compose); hardcoded debug strings noted

### Batch A13: UI Components & Theme
**Path**: `android/.../ui/components/` + `android/.../ui/theme/`
**Files (10)**: SettingsCard, SettingsDivider, NavigationRow, ActionRow, SwitchRow, SliderRow, ColorRow, SegmentedButtonRow, Theme, Type
**Focus**: Component reuse, consistency
**Status**: [x] Done — reviewed; excellent component library, no issues

### Batch A14: Localization, Util, Diagnostics
**Path**: `android/.../localization/` + `android/.../util/` + `android/.../diagnostics/` + `android/.../model/`
**Files (16)**: LanguageManager, DisplayLanguage, LocalizedText, Tab1-4Texts, DiagnosticTexts, LocaleUtils, WindowExtensions, PackageManagerUtils, FontUtils, AppVersionUtils, view_utils, DiagnosticService, CopyrightData
**Focus**: String consistency, utility duplication
**Status**: [x] Done — reviewed; no issues found

---

## iOS (90 .swift files, 10 batches)

### Batch I1: Keyboard Controller
**Path**: `ios/Sources/TaigiKeyboard/_Keyboard/`
**Files (7)**: KeyboardViewController (+Setup, +TextInput, +Cleanup, +EmojiDelegate), KeyboardModels, TaigiKeyboardView
**Focus**: Controller lifecycle, memory management, view separation
**Status**: [x] Done — reviewed; EmojiService delegate cycle and unowned capture risk noted

### Batch I2: Input — Composing & Segmentation
**Path**: `ios/Sources/TaigiKeyboard/Input/`
**Files (6)**: SyllableSegmenter, ComposingManager, TPSConverter, CaseTransformationService, KeyboardContext+Composing, KeyboardContext+Translate
**Focus**: DP segmentation, TPS conversion, composing state
**Status**: [x] Done — extracted stopConsonants constant in TPSConverter.swift (aligned with Android A2 fix)

### Batch I3: Input — Tone & Actions
**Path**: `ios/Sources/TaigiKeyboard/Input/Tone/` + `ios/Sources/TaigiKeyboard/Actions/`
**Files (9)**: TaigiPhonetics, ToneConverter, ToneRestoration, ToneUtilities, ActionHandler (+CharacterInput, +Suggestions, +CustomActions, +Utilities)
**Focus**: Tone correctness, action dispatch flow
**Status**: [x] Done — vowel Set in convertNasalDoubleN; early-exit in adjustNasalMarkerCase; static punctuation Set in isPunctuationExceptHyphen

### Batch I4: Lexicon — Models & Services
**Path**: `ios/Sources/TaigiKeyboard/Lexicon/Models/` + `ios/Sources/TaigiKeyboard/Lexicon/Services/`
**Files (9)**: TaigiWord, CustomDictionaryEntry, InputType, DictionaryError, LexiconConstants, LexiconService, AutocompleteService, NextWordService, UserFrequencyService
**Focus**: Data models, lookup logic, frequency tracking
**Status**: [x] Done — reviewed; thread safety issues (recordCounter race, @unchecked Sendable) noted as architectural debt

### Batch I5: Lexicon — Database & Trie
**Path**: `ios/Sources/TaigiKeyboard/Lexicon/Database/` + `ios/Sources/TaigiKeyboard/Lexicon/Trie/` + `ios/Sources/TaigiKeyboard/Lexicon/Utils/`
**Files (8)**: DictionaryRepository, CustomDictionaryRepository, UserFrequencyRepository, SQLiteConnectionManager, TrieService, InputNormalizer, RomanizationConverter, TextProcessor
**Focus**: DB connection safety, trie efficiency, normalization pipeline
**Status**: [x] Done — reviewed with I4; same thread safety findings apply

### Batch I6: Autocomplete
**Path**: `ios/Sources/TaigiKeyboard/Autocomplete/`
**Files (10)**: CandidateViewModels, CandidateExpandState, AutocompleteService, EnglishAutocompleteService, SuggestionCaseTransformer, CandidateView, CandidateViewStyle, CandidateCellHelper, ExpandedCandidateOverlay, LayoutSelectionOverlay
**Focus**: Ranking logic, view efficiency, state management
**Status**: [x] Done — reviewed; ExpandedCandidateOverlay quadratic recomputation noted as optimization target

### Batch I7: Layout
**Path**: `ios/Sources/TaigiKeyboard/Layout/`
**Files (6)**: TaigiLayouts, LayoutConverter, CustomLayoutService, DeviceConfiguration, KeyDef, LayoutConstants
**Focus**: Layout definitions, device adaptation, key mapping
**Status**: [x] Done — reviewed; clean architecture, no issues

### Batch I8: Styling & Callouts
**Path**: `ios/Sources/TaigiKeyboard/Styling/` + `ios/Sources/TaigiKeyboard/Callouts/`
**Files (7)**: TaigiButtonContent, ButtonTextProvider, ButtonImageProvider, ButtonFontProvider, ConfirmKeyTextHelper, Callouts+TaigiCalloutBuilder, Callouts+TaigiCalloutMaps
**Focus**: Display overrides, callout completeness
**Status**: [x] Done — reviewed; good provider pattern, no issues

### Batch I9: App UI
**Path**: `ios/Sources/TaigiKeyboard/App/`
**Files (17)**: TaigiKeyboardApp, ContentView, TabType, Tab1-4, AppearanceSettingsView, FAQType, FeatureType, FeatureDetailView, FAQDetailView, FeedbackDetailView, VersionHistoryDetailView, CopyrightView, SetupGuide*, ImageSlideshowView
**Focus**: Navigation, state, SwiftUI patterns
**Status**: [x] Done — reviewed; no issues

### Batch I10: Settings, Localization, Debug, Diagnostics
**Path**: `ios/Sources/TaigiKeyboard/Settings/` + `ios/Sources/TaigiKeyboard/Localization/` + `ios/Sources/TaigiKeyboard/Debug/` + `ios/Sources/TaigiKeyboard/Diagnostics/` + `ios/Sources/TaigiKeyboard/Emojis/`
**Files (12)**: SharedSettings, InputMode, LocalizedText, KeyboardTexts, Tab1-4Texts, DebugView, DebugAssociationView, DebugFrequencyView, DiagnosticService, EmojiService
**Focus**: Settings consistency, string duplication, debug code in release
**Status**: [x] Done — reviewed; hardcoded debug strings noted (matches Android pattern)

---

## Review Order (recommended)

**Priority 1 — Engine (high complexity, shared logic)**
1. A2 (Phonetics & Segmentation) → I2 (cross-platform alignment)
2. A3 (Tone Conversion) → I3 (Tone & Actions)
3. A4 (Lookup & Services) → I4 + I5 (Lexicon)

**Priority 2 — Input Flow (concurrency, state)**
4. A5 (Text Input Core)
5. A6 (Composing) → I6 (Autocomplete)
6. A1 (IME Core) → I1 (Keyboard Controller)

**Priority 3 — UI (rendering, efficiency)**
7. A9 (Smartbar)
8. A7 (Layout & Key) → I7 (Layout)
9. A8 (Keyboard Views) → I8 (Styling)

**Priority 4 — Settings & Utilities**
10. A11 (Settings Compose) → I9 (App UI)
11. A10 (Popup & Media)
12. A12 (Legacy Activities)
13. A13 (UI Components) + A14 (Localization/Util) → I10
