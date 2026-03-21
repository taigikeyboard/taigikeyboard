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
| TPS conversion | `TPSConverter.swift` | - |
| Case transformation | `CaseTransformationService.swift` | `SuggestionCaseTransformer.kt` |

### Autocomplete

| Function | iOS | Android |
|----------|-----|---------|
| Taigi autocomplete | `AutocompleteService.swift` | `TaigiAutocompleteService.kt` |
| English autocomplete | - | `EnglishAutocompleteService.kt` |
| Candidate View | `CandidateView.swift` | `SmartbarView.kt` |
| Candidate Adapter | - | `CandidateAdapter.kt` |
| Expanded overlay | `ExpandedCandidateOverlay.swift` | `CandidateOverlayView.kt` |

### Lexicon

| Function | iOS | Android |
|----------|-----|---------|
| Dictionary service | `LexiconService.swift` | `LexiconService.kt` |
| Trie service | `TrieService.swift` | `TrieService.kt` |
| Input normalization | `InputNormalizer.swift` | `InputNormalizer.kt` |
| Word model | `TaigiWord.swift` | `DictionaryModels.kt` |

### Segmentation

| Function | iOS | Android |
|----------|-----|---------|
| Syllable segmenter | `SyllableSegmenter.swift` | `SyllableSegmenter.kt` |

### Tone

| Function | iOS | Android |
|----------|-----|---------|
| Tone conversion | `ToneConverter.swift` | `ToneConverter.kt` |
| Phonetics engine | `TaigiPhonetics.swift` | `TaigiPhonetics.kt` |
| Tone restoration | `ToneRestoration.swift` | `ToneRestoration.kt` |
| Tone utilities | `ToneUtilities.swift` | `ToneConverterModels.kt` + `ToneCharacterUtils.kt` |

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
| `ContentView.swift` | `SettingsMainActivity.kt` | Tab container |
| `Tab1.swift` | `Tab1Fragment.kt` | Home |
| `Tab2.swift` | `Tab2Fragment.kt` | Layout |
| `Tab3.swift` | `Tab3Fragment.kt` | Dictionary |
| `Tab4.swift` | `Tab4Fragment.kt` | Settings |

### Tab1 Sub-pages

| iOS Page | iOS File | Android File |
|----------|----------|--------------|
| SetupGuide | `SetupGuideView.swift` | `SetupGuideActivity.kt` |
| FeatureDetail | `FeatureDetailView.swift` | `DetailActivity.kt` |
| FAQDetail | `FAQDetailView.swift` | `DetailActivity.kt` |
| FeedbackDetail | `FeedbackDetailView.swift` | `DetailActivity.kt` |
| VersionHistory | `VersionHistoryDetailView.swift` | `DetailActivity.kt` |
| Copyright | `CopyrightView.swift` | `CopyrightActivity.kt` |
| AppearanceSettings | `AppearanceSettingsView.swift` | `AppearanceSettingsActivity.kt` |

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
| `ImageSlideshowView.swift` | `ImageSlideshowView.kt` | Auto image slideshow |
| `LocalizedTextView` (extension) | `LocalizedTextView.kt` | Localized text display |

### Other App Files

| iOS File | Android File | Description |
|----------|--------------|-------------|
| `DebugView.swift` | `DebugActivity.kt` | Debug page |

---

## Directory Structure

### iOS (`ios/Sources/TaigiKeyboard/`)

```
TaigiKeyboard/
├── _Keyboard/       # IME main entry
├── Actions/         # Action handlers
├── App/             # Main App UI
│   ├── Assets/      # Image resources
│   ├── Components/  # Shared components
│   └── Tabs/        # Tab pages
├── Autocomplete/    # Autocomplete
│   ├── Models/
│   ├── Services/
│   └── Views/
├── Callouts/        # Long-press menus
├── Debug/           # Debug tools
├── Diagnostics/     # Diagnostic info service
├── Emojis/          # Emoji related
├── Input/           # Input and composing
│   └── Tone/        # Tone processing
├── Layout/          # Keyboard layout
│   └── Flick/       # Flick input
├── Lexicon/         # Dictionary query
│   ├── Database/
│   ├── Models/
│   ├── Services/
│   ├── Trie/
│   └── Utils/
├── Localization/    # Localization
├── Settings/        # Settings
└── Styling/         # Button styling & theming
    ├── Helpers/
    └── Providers/
```

### Android (`android/app/src/main/java/.../taigikeyboard/`)

```
taigikeyboard/
├── ime/
│   ├── core/        # TaigiKeyboard, PrefHelper, InputView
│   ├── dictionary/  # Trie, Lexicon, Tone
│   ├── keyboard/    # EmojiSkinTone
│   ├── lifecycle/   # LifecycleInputMethodService
│   ├── media/       # Emoji
│   │   └── emoji/   # EmojiKeyboardView, EmojiPaletteView
│   ├── popup/       # Key popups
│   └── text/
│       ├── composing/   # Composing, Autocomplete
│       ├── key/         # KeyView, KeyData
│       ├── keyboard/    # KeyboardView
│       ├── layout/      # LayoutManager
│       └── smartbar/    # Smartbar
├── localization/    # Localization
├── model/           # Data models
├── onboarding/      # Onboarding flow
├── settings/        # Settings pages
├── ui/theme/        # Theme
└── util/            # Utilities
```

---

## Resource Files

### iOS (`ios/Resources/`)

| File | Size | Description |
|------|------|-------------|
| `dictionary.db` | ~40MB | SQLite dictionary |
| `dictionary.trie` | ~4MB | MARISA Trie |
| `Iansui-Regular.ttf` | ~9MB | Iansui font |
| `jf-openhuninn-2.1.ttf` | ~5MB | jf-openhuninn font |

### Android (`android/app/src/main/assets/`)

| File | Description |
|------|-------------|
| `dictionary.db` | SQLite dictionary |
| `dictionary.trie` | MARISA Trie |

---

## Naming Conventions

| Type | iOS | Android |
|------|-----|---------|
| Service | `XxxService.swift` | `XxxService.kt` |
| Manager | `XxxManager.swift` | `XxxManager.kt` |
| View | `XxxView.swift` | `XxxView.kt` |
| Models | `XxxModels.swift` | `XxxModels.kt` |
