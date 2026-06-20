# File Index and Naming Conventions

> **Type**: Index
> **Keywords**: `Files`, `Structure`, `Mapping`, `Naming`
> **Related**: system-overview.md (Mermaid architecture diagrams), README.md, engine/migration-inventory.csv

---

## Summary

- iOS / Android cross-platform file correspondence (post-Rust extraction).
- Engine logic lives in `engine/` Rust crates; platforms hold thin bridges + glue.
- For row-level Rust pub-item inventory see `engine/migration-inventory.csv`.

---

## Feature Module Correspondence (logical owner)

| Feature Code | Rust crate | iOS Directory | Android Directory |
|--------------|------------|---------------|-------------------|
| `Phonetics` | `engine/phonetics` | `Engine/` (bridge) | `engine/` (bridge) |
| `Composing` | `engine/composing` | `Input/Composing/` (wrapper) | `ime/text/composing/` |
| `Autocomplete` | `engine/lexicon` + `engine/ranking` | `Autocomplete/` | `ime/text/composing/` |
| `Lexicon` | `engine/lexicon` (+ `engine/mmap-host`) | `Lexicon/` | `ime/dictionary/` |
| `BinaryReader` | `engine/lexicon` (mmap dictionary.bin / association.bin) | (Rust) | (Rust) |
| `Tone` | `engine/phonetics` | `Engine/RustEngineBridge.swift` | `engine/RustEngineBridge.kt` |
| `CaseTransform` | `engine/phonetics::case_transform` | `Engine/RustEngineBridge+CaseTransform.swift` | `engine/CaseTransformBridge.kt` |
| `NextWord` | `engine/nextword` | `NextWord/` (controller + service glue) | `ime/dictionary/` + `ime/text/smartbar/NextWordHandler.kt` |
| `UserFrequency` | — (platform SQLite, `wont_migrate`) | `Lexicon/Database/` | `ime/text/composing/UserFrequencyService.kt` |
| `CustomDictionary` | — (platform SQLite, `wont_migrate`) | `Lexicon/Database/` + `Lexicon/Services/` | `ime/dictionary/CustomDictionary*.kt` |
| `Layout` | — | `Layout/` | `ime/text/layout/` |
| `Smartbar` | — | `Autocomplete/Views/` | `ime/text/smartbar/` |
| `Settings` | — | `Settings/` | `settings/` + `ime/core/settings/` |
| `Styling` | — | `Styling/` | `ui/theme/` |
| `Diagnostics` | — | `Diagnostics/` | `diagnostics/` |
| `FFI` | `engine/dispatch` + `engine/swift-ffi` + `engine/android-jni` | `Engine/RustEngineBridge.swift` | `engine/RustEngineBridge.kt` |

---

## IME Core File Correspondence

### Entry Points

| Function | iOS | Android |
|----------|-----|---------|
| Main entry | `KeyboardViewController.swift` | `TaigiKeyboard.kt` |
| Keyboard View | `TaigiKeyboardView.swift` | `KeyboardView.kt` |
| Preferences | `SharedSettings.swift` | `PrefHelper.kt` |
| Rust FFI bridge | `Engine/RustEngineBridge.swift` | `engine/RustEngineBridge.kt` |
| Lexicon bridge | `Engine/RustEngineBridge+Lexicon.swift` | `engine/LexiconBridge.kt` |
| Case-transform bridge | `Engine/RustEngineBridge+CaseTransform.swift` | `engine/CaseTransformBridge.kt` |
| NextWord bridge | `Engine/RustEngineBridge+NextWord.swift` | (in `RustEngineBridge.kt`) |

### Composing

Pure state machine lives in `engine/composing` (Rust). Platform side holds the effect interpreter.

| Function | iOS | Android |
|----------|-----|---------|
| Engine state machine | (Rust `engine/composing`) | (Rust `engine/composing`) |
| Platform wrapper | `ComposingManager.swift` | `ComposingManager.kt` |
| Effect interpreter | `ComposingDelegate.swift` | `ComposingDelegate.kt` |
| Caps state | (KeyboardKit managed) | `CapsStateManager.kt` |
| Candidate coordinator | (inline in TaigiAutocompleteService) | `CandidateUpdateCoordinator.kt` |

### Autocomplete

| Function | iOS | Android |
|----------|-----|---------|
| Input classifier | (Rust `engine/lexicon::classify_input`; platform shells deleted v3.5.8 Item 13) | (Rust; shell deleted) |
| Taigi autocomplete | `TaigiAutocompleteService.swift` (engine-only) | `TaigiAutocompleteService.kt` (engine-only) |
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

fst prefix index + dictionary.bin / association.bin readers all live in Rust `engine/lexicon`. Platform side holds asset paths + lifecycle + UI glue.

| Function | iOS | Android |
|----------|-----|---------|
| Engine search / classify / assoc | (Rust `engine/lexicon`) | (Rust `engine/lexicon`) |
| Lifecycle service | (iOS `LexiconService.swift` deleted v3.5.8 Item 13; install via `KeyboardViewController+Setup`) | `ime/dictionary/LexiconService.kt` (Tab3 + asset lifecycle) |
| Search service (Tab3) | `Lexicon/Services/DictionarySearchService.swift` | (Tab3 methods in `LexiconService.kt`) |
| Word model | `Lexicon/Models/TaigiWord.swift` | `ime/dictionary/TaigiWord.kt` |
| Enabled dictionaries | `Lexicon/Models/EnabledDictionaries.swift` | `ime/dictionary/EnabledDictionaries.kt` |
| Custom dictionary service | `Lexicon/Services/CustomDictionaryService.swift` | `ime/dictionary/CustomDictionaryService.kt` |
| Custom dict repo (iOS) | `Lexicon/Database/CustomDictionaryRepository.swift` | (built into Service) |
| Custom dict model | `Lexicon/Models/CustomDictionaryEntry.swift` | (in `TaigiWord.kt` + `CustomDictionaryService.kt`) |
| Custom dict derivation | `Lexicon/Database/CustomDictionaryDerivation.swift` | `ime/dictionary/CustomDictionaryDerivation.kt` |
| Search result UI model | `Lexicon/Models/DictionarySearchResult.swift` | `ime/dictionary/DictionarySearchResult.kt` |
| External lookup URL | `Lexicon/Utils/ExternalLookupURLBuilder.swift` | `ime/dictionary/ExternalLookupURLBuilder.kt` |
| NextWord service | `NextWord/Services/NextWordService.swift` | `ime/dictionary/NextWordService.kt` |
| NextWord controller | `NextWord/NextWordController.swift` | `ime/text/smartbar/NextWordHandler.kt` |
| Backup | `Lexicon/Services/BackupService.swift` | `ime/dictionary/BackupService.kt` |

### UserFrequency (platform SQLite — `wont_migrate`)

| Function | iOS | Android |
|----------|-----|---------|
| Frequency service | `Lexicon/Services/UserFrequencyService.swift` | `ime/text/composing/UserFrequencyService.kt` |
| Frequency repository | `Lexicon/Database/UserFrequencyRepository.swift` | (built-in) |
| Schema / pruner | `Lexicon/Database/UserFrequencySchema.swift`, `UserFrequencyPruner.swift` | (in service) |
| SQLite plumbing | `Lexicon/Database/SQLiteConnectionManager.swift`, `SQLiteBindingHelpers.swift`, `SharedDatabasePath.swift` | (Android Room / SQLiteOpenHelper internal) |

---

## App Page File Correspondence

### Tab Structure

| iOS File | Android File | Description |
|----------|--------------|-------------|
| `ContentView.swift` | `MainSettingsScreen.kt` | Tab container |
| `HomeTab.swift` | `HomeScreen.kt` | Home |
| `LayoutTab.swift` | `LayoutScreen.kt` | Layout |
| `DictionaryTab.swift` | `DictionarySettingsScreen.kt` | Dictionary |
| `SettingsTab.swift` | `InputSettingsScreen.kt` | Settings |

### Home Sub-pages

| iOS Page | iOS File | Android File |
|----------|----------|--------------|
| SetupGuide | `SetupGuideView.swift` | `SetupGuideScreen.kt` |
| FeatureDetail | `FeatureDetailView.swift` | `DetailScreen.kt` |
| FAQDetail | `FAQDetailView.swift` | `DetailScreen.kt` |
| FeedbackDetail | `FeedbackDetailView.swift` | `DetailScreen.kt` |
| VersionHistory | `VersionHistoryDetailView.swift` | `DetailScreen.kt` |
| Copyright | `CopyrightView.swift` | `CopyrightScreen.kt` |
| AppearanceSettings | `AppearanceSettingsView.swift` | `AppearanceSettingsScreen.kt` |

### Home Content Models

| iOS File | Android File | Description |
|----------|--------------|-------------|
| `FeatureContent.swift` | `FeatureContent.kt` | Feature data model |
| `FeatureContentLoader.swift` | `FeatureContentLoader.kt` | JSON loader |

### Dictionary Sub-pages

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

**Mid-migration — per-file correspondence is in flux.** The app-UI strings are moving from hand-written `*Texts` classes to a generated-resource pipeline (`i18n/*.json` → `tools/i18n/generate.py` → committed output). Authoritative plan + phase status: [`i18n-multilang-plan.md`](i18n-multilang-plan.md).

| Side | Current state (main) |
|------|----------------------|
| Android | Migrated namespaces use generated `i18n/generated/{L10n,StringKey,GeneratedPseudoStrings,GeneratedTaigiStrings}.kt`. Namespaces not yet migrated keep hand-written `localization/*Texts.kt` (set shrinks one namespace per phase — current status in the plan). |
| iOS | Not migrated (R2b blocked on `.pbxproj` hand-off) — all strings still in `Strings/*Texts.swift` (`Home`, `Layout`, `Theme`, `Dictionary`, `Common`, `Settings`). |

### Shared Components

| iOS Component | Android Component | Use |
|---------------|-------------------|-----|
| `ImageSlideshowView.swift` | - | Auto image slideshow |
| `SettingInfoButton.swift` | `SettingInfoButton.kt` | Info icon button |

### Diagnostics

| iOS File | Android File | Description |
|----------|--------------|-------------|
| `DiagnosticService.swift` | `DiagnosticService.kt` | Device/app diagnostic info |
| `DiagnosticTexts.swift` | `DiagnosticTexts.kt` | Diagnostic string localization |

---

## Directory Structure

### Rust workspace (`engine/`)

```
engine/
├── phonetics/         # POJ/TL/TPS conversion + normalize + case-transform
├── composing/         # Composing state machine (Phase × Intent → Effect)
├── nextword/          # NextWord prediction (decay/score/booster)
├── ranking/           # Candidate dedup / score / sort
├── lexicon/           # fst prefix index + dictionary/association mmap readers
├── dispatch/          # Top-level FFI dispatch (bytes-in / bytes-out + catch_unwind)
├── swift-ffi/         # iOS swift-bridge entry point
├── android-jni/       # Android JNI entry point
├── mmap-host/         # Centralized mmap unsafe carve-out
├── protos/            # prost-build proto codegen
└── build-helpers/
    └── fst-builder/   # Offline FST builder (dictionary.fst producer)
```

### Engine crate layering

Dependency edges flow **one way, top → bottom** (caller depends on callee). The dependency-direction invariant + enforcement notes live in `.claude/rules/rust-best-practices.md` §1a. Exact per-crate edge graph (Mermaid): [`system-overview.md`](system-overview.md) §2.

```
┌─ adapters ─────────────────────────────────────────────────┐
│  swift-ffi · android-jni   thin: bytes in/out, catch_unwind │
└───────────────────────────┬─────────────────────────────────┘
                            │ depends ↓
┌─ use-case ────────────────┴─────────────────────────────────┐
│  dispatch                  only crate that sees all domains  │
└───────────────────────────┬─────────────────────────────────┘
                            │ depends ↓
┌─ domain ──────────────────┴─────────────────────────────────┐
│  composing → lexicon, ranking, phonetics                     │
│  lexicon   → ranking, phonetics, mmap-host                   │
│  ranking   → phonetics                                       │
│  nextword  → phonetics                                       │
└───────────────────────────┬─────────────────────────────────┘
                            │ depends ↓
┌─ leaf / shared kernel ────┴─────────────────────────────────┐
│  phonetics  pure fns (POJ/TL/TPS, tone, normalize)           │
│  protos     prost-generated message types (shared by all)    │
│  mmap-host  unsafe mmap carve-out (infra)                     │
└──────────────────────────────────────────────────────────────┘
```

### Request data-flow (one keystroke)

```
keystroke (platform UI)
  → RustEngineBridge (Swift / Kotlin)          marshal proto request bytes
  → swift-ffi / android-jni                    extern entry + catch_unwind
  → dispatch                                   decode proto → match request kind
  → composing / lexicon / ranking / nextword   domain work
  → phonetics                                  POJ/TL/TPS, tone (called by the above)
  ← proto response bytes                       back up the same path → candidates rendered
```

### iOS (`ios/Sources/TaigiKeyboard/`)

```
TaigiKeyboard/
├── Actions/         # Action handlers (ActionHandler + extensions)
├── App/             # Main App UI
│   ├── Components/  # Shared components
│   └── Tabs/        # Tab pages
│       ├── Home/        # Home tab (setup guide, features, FAQ)
│       │   ├── Models/      # FeatureContent, FeatureContentLoader
│       │   ├── DetailViews/ # Feature/FAQ/Feedback/Copyright views
│       │   └── SetupGuide/  # Setup guide views
│       ├── Layout/      # Layout tab
│       ├── Dictionary/  # Dictionary tab (data management)
│       └── Settings/    # Settings tab
├── Autocomplete/    # Autocomplete + classifier glue
│   ├── Models/      # CandidateViewModels, SymbolData
│   ├── Services/    # TaigiAutocompleteService, EnglishAutocompleteService, AutocompleteProviders, SuggestionCaseTransformer
│   └── Views/       # CandidateView, overlays (Layout/Symbol/Settings)
├── Callouts/        # Long-press menus
├── Logging/         # LoggerBackend protocol + LoggerFactory
├── Composition/     # Cross-tab composition root (DI)
├── Diagnostics/     # DiagnosticService
├── Emojis/          # Emoji related
├── Engine/          # Rust FFI bridge
│   ├── Generated/   # swift-bridge generated bindings + proto .pb.swift
│   ├── RustEngineBridge.swift
│   ├── RustEngineBridge+Lexicon.swift
│   ├── RustEngineBridge+CaseTransform.swift
│   └── RustEngineBridge+NextWord.swift
├── Input/           # Input pipeline + composing platform wrapper
│   └── Composing/   # ComposingManager, ComposingDelegate
├── KeyboardExtension/ # KeyboardViewController + extensions
├── Layout/          # Keyboard layout
├── Lexicon/         # Dictionary glue (Rust does the actual queries)
│   ├── Database/    # SQLite repos (custom-dict + user-freq) + asset path resolver
│   ├── Models/      # TaigiWord, CustomDictionaryEntry, EnabledDictionaries, FrequencyData, etc.
│   ├── Services/    # LexiconService, DictionarySearchService, UserFrequencyService, CustomDictionaryService, BackupService
│   └── Utils/       # CandidateProcessor (residual), ExternalLookupURLBuilder
├── NextWord/        # NextWord platform glue
│   ├── Repository/  # SQLite repo (legacy) + schema
│   ├── Services/    # NextWordService
│   └── NextWordController.swift
├── Overlays/        # System overlays
├── Settings/        # SharedSettings, EngineSettings/Provider, InputMode, ToneToggles
├── Strings/         # *Texts.swift localization (Home/Layout/Theme/Dictionary/Common/Settings) — pre-i18n-migration
└── Styling/         # Button styling & theming
    ├── Helpers/
    └── Providers/
```

### Android (`android/app/src/main/java/.../taigikeyboard/`)

```
taigikeyboard/
├── engine/          # Rust JNI bridge
│   ├── RustEngineBridge.kt
│   ├── LexiconBridge.kt
│   ├── CaseTransformBridge.kt
│   └── proto/       # prost / generated proto Java
├── ime/
│   ├── core/        # TaigiKeyboard, PrefHelper, InputView, Subtype, AppVersionTracker, SubtypeLocaleAdapter
│   │   ├── settings/    # EngineSettings/Provider, InputMode, ToneToggles
│   │   └── logging/     # AndroidLoggerBackend, LoggerBackend
│   ├── dictionary/  # Lexicon glue, Custom dict, NextWord, Backup, SuggestionCaseTransformer, DictionaryCsvCodec
│   ├── lifecycle/   # LifecycleInputMethodService
│   ├── media/       # MediaInputManager
│   │   └── emoji/   # EmojiKeyboardView, EmojiPaletteView
│   ├── popup/       # Key popups
│   ├── theme/       # ThemeAttributeColors (theme-attr → color resolver for IME rendering)
│   └── text/
│       ├── composing/   # ComposingManager, ComposingDelegate, AutocompleteServices, classifier, UserFrequency
│       ├── key/         # KeyView, KeyData, KeyCode, KeyType, KeyLabelCaseCache
│       ├── keyboard/    # KeyboardView, KeyboardRowView
│       ├── layout/      # LayoutManager, LayoutData
│       └── smartbar/    # SmartbarManager, CandidateAdapter, overlays, NextWordHandler, ToolbarManager
├── content/         # ContentResolver entry point
├── i18n/generated/  # Generated i18n: L10n, StringKey, GeneratedPseudoStrings, GeneratedTaigiStrings
├── localization/    # Residual hand-written *Texts (namespaces not yet migrated) — see i18n-multilang-plan.md
├── settings/        # Activity wrappers (Compose host) + LauncherIconController
├── typeface/        # TypefaceLoader (R.font → android.graphics.Typeface)
├── ui/
│   ├── components/  # Reusable Compose components + DrawableResourceResolver
│   ├── tabs/        # home / layout / dictionary / settings Compose screens
│   ├── theme/       # Compose Material theme (Type)
│   └── EdgeToEdgeActivityExtensions.kt
```

---

## Resource Files

### iOS (`ios/Resources/Dictionaries/`)

| File | Size | Description |
|------|------|-------------|
| `dictionary.fst` | ~9.1 MB | Burntsushi `fst` prefix index (`tl:` / `poj:` / `hanzi:` keys → rowid) — replaced MARISA in v3.5.6 |
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
| `dictionary.fst` | ~9.1 MB | Burntsushi `fst` prefix index (same as iOS) |
| `dictionary.bin` | ~4.4 MB | Binary mmap dictionary (same as iOS) |
| `association.bin` | ~3.1 MB | Binary mmap word associations (same as iOS) |

Binary files are **platform-independent** — identical files on both platforms.
SQLite only for writable user data: `user_frequency.db`, `user_association.db`, `custom_dictionary.db` (`status=wont_migrate`).

---

## Naming Conventions

| Type | iOS | Android |
|------|-----|---------|
| Service | `XxxService.swift` | `XxxService.kt` |
| Manager | `XxxManager.swift` | `XxxManager.kt` |
| View | `XxxView.swift` | `XxxView.kt` |
| Models | `XxxModels.swift` | `XxxModels.kt` |
| Rust crate | `engine/<area>` | `engine/<area>` |
| FFI bridge | `Engine/RustEngineBridge*.swift` | `engine/<X>Bridge.kt` |
