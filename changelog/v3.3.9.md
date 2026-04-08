## v3.3.9

### iOS

#### New Features
- **Flick keyboard**: Added Japanese-style swipe input layout
- **English mode**: Independent `EnglishAutocompleteService` with pure English input support
- **Case transformation**: Added `CaseTransformationService` and `SuggestionCaseTransformer`

#### Architecture Refactor
- **KeyboardKit 10 upgrade**: Refactored Action Handler, Layout, and Styling
- **Layout system**: Added `TaigiLayouts`, `LayoutConverter`, `DeviceConfiguration`
- **Tab1 split**: Separated FAQ, Feature, Copyright into sub-pages
- **Theme simplification**: Removed custom Theme, adopted Apple standard Form + Section

#### Removed
- `ThemeTokens`, `ThemeComponents`, `ButtonStyles`
- `AlphabeticLayoutBuilder`, `BottomRowBuilder`
- `CopyrightView` (legacy), `SettingsComponents`

### Android

#### New Features
- **English mode**: Added `EnglishAutocompleteService` and Smartbar English candidates
- **InputNormalizer**: Auto-completion for tones 1/4, POJ o͘ (U+0358) conversion

#### Dependency Upgrades
- `compileSdk` 35 → 36
- `core-ktx` 1.15.0 → 1.17.0
- `activity` 1.10.1 → 1.12.2
- `compose-bom` 2024.10.01 → 2025.12.01
- `lifecycle` 2.8.7 → 2.10.0
- `datastore` 1.0.0 → 1.2.0
- `serialization-json` 1.6.0 → 1.8.0

#### Cleanup
- Removed unused: Room, KSP, runtime-livedata
- Removed `ThemeUtils.kt` and custom colors
- Lint fixes: `UseAppTint`, `MissingDefaultResource`, `Locale` deprecation

#### Tooling
- Added `gradle-versions-plugin` for dependency checking

### Shared
- Updated dictionary test scripts
- Added documentation: `case.md`, `device.md`, `flow.md`, `layout.md`, `flick.md`
