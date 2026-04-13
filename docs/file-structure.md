# File Index and Naming Conventions

> **Type**: Index
> **Keywords**: `Files`, `Structure`, `Mapping`, `Naming`
> **Related**: README.md

---

## Summary

- iOS/Android cross-platform file correspondence table
- Covers both IME (keyboard extension) and App (main app) files
- Unified naming conventions and directory structure

---

## Feature Module Correspondence

| Feature Code | iOS Directory | Android Directory |
|--------------|---------------|-------------------|
| `Composing` | `Input/` | `ime/text/composing/` |
| `Autocomplete` | `Autocomplete/` | `ime/text/composing/` |
| `Lexicon` | `Lexicon/` | `ime/dictionary/` |
| `BinaryReader` | `Lexicon/Database/` | `ime/dictionary/` |
| `Trie` | `Lexicon/Trie/` | `ime/dictionary/` |
| `Tone` | `Input/Tone/` | `ime/dictionary/` |
| `UserFrequency` | `Lexicon/` | `ime/text/composing/` |
| `NextWord` | `Lexicon/` | `ime/dictionary/` |
| `Layout` | `Layout/` | `ime/text/layout/` |
| `Smartbar` | `Autocomplete/Views/` | `ime/text/smartbar/` |
| `Settings` | `Settings/` | `settings/` |
| `Styling` | `Styling/` | `ui/theme/` |

---

## IME Core File Correspondence

### Entry Points

| Function | iOS | Android |
|----------|-----|---------|
| Main entry | `KeyboardViewController.swift` | `TaigiKeyboard.kt` |
| Keyboard View | `TaigiKeyboardView.swift` | `KeyboardView.kt` |
| Preferences | `SharedSettings.swift` | `PrefHelper.kt` |

### Composing

| Function | iOS | Android |
|----------|-----|---------|
| Composing manager | `ComposingManager.swift` | `ComposingManager.kt` |
| TPS conversion | `TPSConverter.swift` | `TPSConverter.kt` |
| Case transformation | `CaseTransformationService.swift` | `SuggestionCaseTransformer.kt` |
| Caps state | (KeyboardKit managed) | `CapsStateManager.kt` |
| Candidate coordinator | (inline in AutocompleteService) | `CandidateUpdateCoordinator.kt` |

### Autocomplete

| Function | iOS | Android |
|----------|-----|---------|
| Taigi autocomplete | `AutocompleteService.swift` | `TaigiAutocompleteService.kt` |
| English autocomplete | `EnglishAutocompleteService.swift` | `EnglishAutocompleteService.kt` |
| Candidate View | `CandidateView.swift` | `SmartbarView.kt` |
| Candidate Adapter | - | `CandidateAdapter.kt` |
| Candidate click | (inline in ActionHandler+Suggestions) | `CandidateClickHandler.kt` |
| Expanded overlay | `ExpandedCandidateOverlay.swift` | `CandidateOverlayView.kt` |
| Symbol overlay | `SymbolSelectionOverlay.swift` | `SymbolSelectionOverlayView.kt` |
| Settings overlay | `SettingsSelectionOverlay.swift` | `SettingsSelectionOverlayView.kt` |
| Layout overlay | `LayoutSelectionOverlay.swift` | `LayoutSelectionOverlayView.kt` |
| Symbol data | `SymbolData.swift` | `SymbolData.kt` |
| Toolbar | (inline in CandidateView) | `ToolbarManager.kt` |

### Lexicon

| Function | iOS | Android |
|----------|-----|---------|
| Dictionary service | `LexiconService.swift` | `LexiconService.kt` |
| Dictionary binary reader | `DictionaryBinaryReader.swift` | `DictionaryBinaryReader.kt` |
| Association binary reader | `AssociationBinaryReader.swift` | `AssociationBinaryReader.kt` |
| Enabled dictionaries | `EnabledDictionaries.swift` | `EnabledDictionaries.kt` |
| Trie service | `TrieService.swift` | `TrieService.kt` |
| Input normalization | `InputNormalizer.swift` | `InputNormalizer.kt` |
| Word model | `TaigiWord.swift` | `DictionaryModels.kt` |
| Custom dictionary | `CustomDictionaryService.swift` | `CustomDictionaryService.kt` |
| Custom dict repo | `CustomDictionaryRepository.swift` | (built into Service) |
| Custom dict model | `CustomDictionaryEntry.swift` | (in `DictionaryModels.kt`) |
| Search result | `DictionarySearchResult.swift` | `DictionarySearchResult.kt` |
| Next word | `NextWordService.swift` | `NextWordService.kt` |
| Next word handler | `NextWordController.swift` | `NextWordHandler.kt` |
| Backup | `BackupService.swift` | `BackupService.kt` |

### Tone

| Function | iOS | Android |
|----------|-----|---------|
| Tone conversion | `ToneConverter.swift` | `ToneConverter.kt` |
| Phonetics engine | `TaigiPhonetics.swift` | `TaigiPhonetics.kt` |
| Tone restoration | `ToneRestoration.swift` | `ToneRestoration.kt` |
| Tone utilities | `ToneUtilities.swift` | `ToneUtilities.kt` |
| Tone models | - | `ToneConverterModels.kt` |

### UserFrequency

| Function | iOS | Android |
|----------|-----|---------|
| Frequency service | `UserFrequencyService.swift` | `UserFrequencyService.kt` |
| Frequency repository | `UserFrequencyRepository.swift` | (built-in) |

---

## App Page File Correspondence

### Tab Structure

| iOS File | Android File | Description |
|----------|--------------|-------------|
| `ContentView.swift` | `MainSettingsScreen.kt` | Tab container |
| `Tab1.swift` | `HomeScreen.kt` | Home |
| `Tab2.swift` | `LayoutScreen.kt` | Layout |
| `Tab3.swift` | `DictionarySettingsScreen.kt` | Dictionary |
| `Tab4.swift` | `InputSettingsScreen.kt` | Settings |

### Tab1 Sub-pages

| iOS Page | iOS File | Android File |
|----------|----------|--------------|
| SetupGuide | `SetupGuideView.swift` | `SetupGuideScreen.kt` |
| FeatureDetail | `FeatureDetailView.swift` | `DetailScreen.kt` |
| FAQDetail | `FAQDetailView.swift` | `DetailScreen.kt` |
| FeedbackDetail | `FeedbackDetailView.swift` | `DetailScreen.kt` |
| VersionHistory | `VersionHistoryDetailView.swift` | `DetailScreen.kt` |
| Copyright | `CopyrightView.swift` | `CopyrightScreen.kt` |
| AppearanceSettings | `AppearanceSettingsView.swift` | `AppearanceSettingsScreen.kt` |

### Tab1 Content Models

| iOS File | Android File | Description |
|----------|--------------|-------------|
| `FeatureContent.swift` | `FeatureContent.kt` | Feature data model |
| `FeatureContentLoader.swift` | `FeatureContentLoader.kt` | JSON loader |

### Tab3 Sub-pages

| iOS Page | iOS File | Android File |
|----------|----------|--------------|
| Custom Dictionary | `CustomDictionaryView.swift` | `CustomDictionaryScreen.kt` |
| Custom Dict Edit | `CustomDictionaryEditView.swift` | (inline dialog) |
| Frequency Data | `FrequencyDataView.swift` | `FrequencyDataScreen.kt` |
| Association Data | `AssociationDataView.swift` | `AssociationDataScreen.kt` |
| Data Management | `DataManagementView.swift` | `DataManagementScreen.kt` |
| Backup Document | `BackupDocument.swift` | - |
| CSV Document | `CSVDocument.swift` | - |

### Localization

| iOS File | Android File | Content |
|----------|--------------|---------|
| `LocalizedText.swift` | `LocalizedText.kt` | Core structure |
| - | `DisplayLanguage.kt` | Display language enum |
| - | `LanguageManager.kt` | Language management (StateFlow) |
| `Tab1Texts.swift` | `Tab1Texts.kt` | Tab1 text |
| `Tab2Texts.swift` | `Tab2Texts.kt` | Tab2 text |
| `Tab3Texts.swift` | `Tab3Texts.kt` | Tab3 text |
| `Tab4Texts.swift` | `Tab4Texts.kt` | Tab4 text |

### Localization (continued)

| iOS File | Android File | Content |
|----------|--------------|---------|
| `KeyboardTexts.swift` | (in `Tab4Texts.kt`) | Keyboard UI text (e.g. confirmKey) |

### Shared Components

| iOS Component | Android Component | Use |
|---------------|-------------------|-----|
| `ImageSlideshowView.swift` | - | Auto image slideshow |
| `SettingInfoButton.swift` | `SettingInfoButton.kt` | Info icon button |

### Diagnostics

| iOS File | Android File | Description |
|----------|--------------|-------------|
| `DiagnosticService.swift` | `DiagnosticService.kt` | Device/app diagnostic info |
| `DiagnosticTexts.kt` | `DiagnosticTexts.kt` | Diagnostic string localization |

---

## Directory Structure

### iOS (`ios/Sources/TaigiKeyboard/`)

```
TaigiKeyboard/
├── _Keyboard/       # IME main entry (KeyboardViewController + extensions)
├── Actions/         # Action handlers (ActionHandler + extensions)
├── App/             # Main App UI
│   ├── Components/  # Shared components
│   └── Tabs/        # Tab pages
│       ├── Tab1/    # Home (setup guide, features, FAQ)
│       │   ├── Models/      # FeatureContent, FeatureContentLoader
│       │   ├── DetailViews/ # Feature/FAQ/Feedback/Copyright views
│       │   └── SetupGuide/  # Setup guide views
│       └── Tab3/    # Data management
├── Autocomplete/    # Autocomplete
│   ├── Models/      # CandidateViewModels, SymbolData
│   ├── Services/    # AutocompleteService, SuggestionCaseTransformer
│   └── Views/       # CandidateView, overlays (Layout/Symbol/Settings)
├── Callouts/        # Long-press menus
├── Diagnostics/     # DiagnosticService
├── Emojis/          # Emoji related
├── Input/           # Input and composing
│   └── Tone/        # Tone processing
├── Layout/          # Keyboard layout
├── Lexicon/         # Dictionary query
│   ├── Database/    # Binary readers (DictionaryBinaryReader, AssociationBinaryReader), SQLite repos
│   ├── Models/      # TaigiWord, CustomDictionaryEntry, EnabledDictionaries, etc.
│   ├── Services/    # LexiconService, NextWordService, CustomDictionaryService, BackupService
│   ├── Trie/        # TrieService (handle-based multi-trie), InputNormalizer
│   └── Utils/       # TextProcessor, ResourceBundleResolver
├── Localization/    # Localization
├── Settings/        # SharedSettings, InputMode
└── Styling/         # Button styling & theming
    ├── Helpers/
    └── Providers/
```

### Android (`android/app/src/main/java/.../taigikeyboard/`)

```
taigikeyboard/
├── ime/
│   ├── core/        # TaigiKeyboard, PrefHelper, InputView, Subtype
│   ├── dictionary/  # Trie, Lexicon, Tone, TPS, CustomDictionary, NextWord, Backup
│   ├── keyboard/    # EmojiSkinTone
│   ├── lifecycle/   # LifecycleInputMethodService
│   ├── media/       # MediaInputManager
│   │   └── emoji/   # EmojiKeyboardView, EmojiPaletteView
│   ├── popup/       # Key popups
│   └── text/
│       ├── composing/   # ComposingManager, Autocomplete, UserFrequency
│       ├── key/         # KeyView, KeyData, KeyCode, KeyType
│       ├── keyboard/    # KeyboardView, KeyboardRowView
│       ├── layout/      # LayoutManager, LayoutData
│       └── smartbar/    # SmartbarManager, CandidateAdapter, overlays, ToolbarManager
├── diagnostics/     # DiagnosticService
├── localization/    # LocalizedText, LanguageManager, Tab1-4Texts
├── model/           # FeatureContent, CopyrightData
├── settings/        # Activity wrappers (Compose host)
├── ui/
│   ├── components/  # Reusable Compose components (SwitchRow, ColorRow, etc.)
│   ├── settings/    # Compose settings screens
│   └── theme/       # Theme, Type
└── util/            # AppVersionUtils, FontUtils, etc.
```

---

## Resource Files

### iOS (`ios/Resources/Dictionaries/`)

| File | Size | Description |
|------|------|-------------|
| `dictionary.trie` | ~4.5 MB | MARISA trie (tl:/poj:/hanzi: keys → rowid) |
| `dictionary.bin` | ~4.4 MB | Binary mmap dictionary (rowid → record) |
| `association.bin` | ~3.1 MB | Binary mmap word associations |

Fonts in `ios/Resources/`:
| File | Size | Description |
|------|------|-------------|
| `Iansui-Regular.ttf` | ~9 MB | Iansui font |
| `jf-openhuninn-2.1.ttf` | ~5 MB | jf-openhuninn font |

### Android (`android/app/src/main/assets/`)

| File | Size | Description |
|------|------|-------------|
| `dictionary.trie` | ~4.5 MB | MARISA trie (same as iOS) |
| `dictionary.bin` | ~4.4 MB | Binary mmap dictionary (same as iOS) |
| `association.bin` | ~3.1 MB | Binary mmap word associations (same as iOS) |

Binary files are **platform-independent** — identical files on both platforms.
SQLite only for writable user data: `user_frequency.db`, `user_association.db`, `custom_dictionary.db`.

---

## Naming Conventions

| Type | iOS | Android |
|------|-----|---------|
| Service | `XxxService.swift` | `XxxService.kt` |
| Manager | `XxxManager.swift` | `XxxManager.kt` |
| View | `XxxView.swift` | `XxxView.kt` |
| Models | `XxxModels.swift` | `XxxModels.kt` |
