# Codebase Health Check (2026-03-11)

**Codebase size**: iOS 97 Swift files (14,363 LOC) | Android 100+ Kotlin files (20,431+ LOC)

## Clean Areas
- Unused imports: **0** (iOS & Android)
- TODO/FIXME/HACK: **0**
- Dead code: None significant

## Android — Critical File Sizes (>1000 lines)

| File | Lines | Concern |
|------|-------|---------|
| `SmartbarManager.kt` | 1364 | Toolbar appearance + actions + theming mixed |
| `AppearanceSettingsScreen.kt` | 1239 | Single Composable with color picker, preview, gradients |
| `TextInputManager.kt` | 1231 | All input dispatch, composing, tone, suggestions |

Recommendation: Split each into 2-3 single-responsibility components.

## iOS — Notable Files (acceptable, all <700 lines)

| File | Lines | Note |
|------|-------|------|
| `NextWordService.swift` | 669 | Bigram + decay + SQLite |
| `ExpandedCandidateOverlay.swift` | 539 | 12-level SwiftUI nesting |
| `UserFrequencyRepository.swift` | 515 | SQL transaction handling |
| `CustomDictionaryRepository.swift` | 463 | Custom dictionary CRUD |
| `AppearanceSettingsView.swift` | 459 | Settings UI |

## Test Coverage

### Well Tested
SyllableSegmenter, TaigiPhonetics, ToneConverter, ToneRestoration, ToneUtilities, InputNormalizer, CaseTransformationService, TextProcessor, TPSConverter, EngineIntegration

### Untested (could benefit from tests)
ActionHandler dispatch (5 files, 900+ LOC), database operations, CustomDictionary CRUD

### Untested (acceptable)
All UI/SwiftUI code (50+ view files) — industry standard

## Code Smells

### iOS
- `SharedSettings.shared` singleton in 15+ files (limits testability)
- 3 repository files with repeated SQLite boilerplate (could extract shared protocol)
- `ExpandedCandidateOverlay` 12-level nesting

### Android
- 20+ branch when/switch blocks in TextInputManager and SmartbarManager
- Repeated UI theming boilerplate across Composables
