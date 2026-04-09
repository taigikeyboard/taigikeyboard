## v3.4.1 (bugfix-v3.4.1)

### Cross-Platform (iOS & Android)

#### New Features
- **Tone diacritic hints on number keys**: Display standalone diacritics (ˊˋˆˇˉˈ˘˝) above number keys as tone hints, following user's current input mode (POJ/TL); excluded from TPS layout
- **Punctuation hints on MOE1/MOE2 keys**: Show `@` above `-`, `:;` above `,/，`, `!?` above `./。` on MOE1 and MOE2 layouts
- **MOE keyboard layouts**: iOS — MOE Layout 1/2 with TL/POJ variants; Android — MOE Layout 1/2 via `qwerty_moe1.json`, `qwerty_moe1_poj.json`, `qwerty_moe2.json`, `qwerty_moe2_poj.json`
- **Appearance settings**: iOS — `AppearanceSettingsView` with full color customization, key height/font/candidate scales, corner radius; Android — `KeyboardColorSettings` + `AppearanceSettingsActivity`
- **Caps Lock vs. Sentence Case**: Distinguishes Caps Lock (full uppercase, e.g., "tsh" → "TSH") from sentence case (first letter only, e.g., "tsh" → "Tsh"); iOS — `CaseTransformationService`; Android — `uppercaseToneLetter` + `fullUppercaseToneLetter`
- **STTI dictionary toggle**: `sttiDictEnabled` setting for 學科術語辭典 with filter conditions in `LexiconService` and `NextWordService`
- **NextWord memory strength scoring**: User entries rank above dictionary entries with learning bonus (300) and count-based decay floors for near-permanent retention of frequently used words
- **Phrase learning**: Consecutive word selections recorded as phrases (e.g., selecting 勇 then 伯公 learns 勇伯公), surfaced in autocomplete when user types the trigger input
- **SyllableSegmenter**: DAG + DP syllable segmentation engine with CVC+V tie-breaking via dictionary trie prefix search
- **Word-grouped composing display**: Greedy longest-match word grouping; hyphens within words, spaces between words (e.g., "guá kin-á-ji̍t")
- **Strict POJ/TL input mode separation**: Mode-aware syllable tries; POJ mode rejects TL-only patterns, TL mode rejects POJ-only patterns

#### Bug Fixes
- **TL tone mark placement**: Fixed consecutive identical vowels (e.g., hee7 → hēe) in `TaigiPhonetics.placePOJToneMark()`
- **POJ partial input matching**: Fixed POJ input not finding candidates when POJ↔TL vowels differ (e.g., `chiannpe` for 正爿); solved by storing mode-prefixed keys (`tl:`/`poj:`) in the MARISA trie so POJ input matches POJ keys directly
- **Shared Hanzi/tone detection**: Replaced duplicated private methods in AutocompleteService with shared `isHanzi()` and `InputNormalizer.hasToneMarks()` utilities
- **Phrase frequency pollution**: Phrase suggestions no longer recorded in user frequency database (individual words already recorded when first selected)
- **MOE2 font sizing**: iOS — `ButtonFontProvider` adjusts for multi-letter keys; Android — `KeyView` uses 0.65× font for MOE2 character keys

#### Refactoring
- **Unified TaigiPhonetics engine**: Single NFD/NFC-based pipeline for all tone conversion; iOS — consolidated 4 files → 1 (`POJToneConverter`, `TLToneConverter`, `ToneMappings`, `VowelAnalyzer` removed); Android — ported from iOS
- **Dual-prefix trie**: Mode-prefixed keys (`tl:`/`poj:`) in MARISA trie for native input matching; removed `normalizeToTL()` from InputNormalizer; iOS — prefix-aware `WordPrefixChecker` in `ComposingManager` and `AutocompleteService`; Android — `InputNormalizer.buildSearchKey()` prepends mode prefix
- **NextWordService simplified**: TL-only storage; POJ derived at display time; iOS — extracted `calculateUserScore()`, unified duplicate methods, added >=2 char input gate; Android — removed `poj`/`delimiter` fields from `Prediction`/`Association`, POJ via `TaigiPhonetics.tlDisplayToPOJDisplay()`
- **InputNormalizer refactor**: Removed `normalizeToTL()` conversion; input stays in native romanization form; trie prefix determines match namespace

#### Changes
- **TaiHua dictionary enabled by default**: iOS `taiHuaDictEnabled`, Android `taihoaDictEnabled` both default to `true`
- **Localization updates**: "自動大寫" → "自動大本字" on both platforms

#### New Files

| iOS | Android | Description |
|-----|---------|-------------|
| `TaigiPhonetics.swift` | `TaigiPhonetics.kt` | Unified tone conversion engine |
| `RomanizationConverter.swift` | (in `TaigiPhonetics.kt`) | POJ↔TL display conversion |
| `SyllableSegmenter.swift` | `SyllableSegmenter.kt` | DAG + DP syllable segmentation |
| `AppearanceSettingsView.swift` | `AppearanceSettingsActivity.kt` + `KeyboardColorSettings.kt` | Appearance settings |
| — | `ToneRestoration.kt` | Standalone tone restoration (extracted from ToneConverter) |

#### Unit Tests

| iOS | Android | Tests |
|-----|---------|------:|
| `TaigiPhoneticsTests.swift` | `TaigiPhoneticsTest.kt` | 44 |
| `ToneConverterTests.swift` | `ToneConverterTest.kt` | 8 |
| `ToneRestorationTests.swift` | `ToneRestorationTest.kt` | 12 |
| `ToneUtilitiesTests.swift` | `ToneConverterModelsTest.kt` | 19 |
| `SyllableSegmenterTests.swift` | `SyllableSegmenterTest.kt` | 15 |
| `EngineIntegrationTests.swift` | `EngineIntegrationTest.kt` | 9 |
| `TPSConverterTests.swift` | — | iOS-only (TPS) |
| — | `InputNormalizerTest.kt` | 2 |
| Total | | 109 Android / 7 iOS files |

### iOS

#### New Features
- **Layout selection overlay**: Layout panel accessible from expanded toolbar via "photo" button; shows layout cards with preview screenshots for immediate layout switching without opening the main app
- **Toggle toolbar**: Input mode shortcuts (POJ/TL/En) in candidate bar via "+" button with smooth rotation animation
- **TPS layout promoted**: Taiwan Phonetic Symbols (方音符號) layout moved from Debug-only to production
- **Per-setting reset buttons**: Individual reset buttons for appearance settings with smooth toggle animation
- **Uppercase nasalization marker**: Support for ⁿ → ᴺ in POJ Caps Lock mode (not yet on Android)

#### Bug Fixes
- **Keyboard background rendering**: Fixed background color consistency between keyboard extension and preview panel
- **Color reactivity**: Fixed live color updates using correct KeyboardKit style API
- **DeviceConfiguration**: Replaced deprecated `context.deviceType` with `UIDevice.current.userInterfaceIdiom`
- **LayoutConverter**: Replaced deprecated `context.interfaceOrientation` with `UIScreen.main.bounds`

#### Refactoring
- **ComposingManager**: Refactored to use `rawInput` as single source of truth for composition state
- **Tab2 redesign**: Layout selection redesigned as horizontal swipe cards with preview images
- **Tab4 restructure**: Input mode picker moved to sub-view; font selector removed from inline picker
- **Toolbar layout**: Removed separator after "+", moved gear icon to right, unified expanded state
- **Shared candidate constants**: `CandidateViewModels` constants shared between real keyboard and preview

#### Removed
- **Flick keyboard**: Entire implementation removed (9 files: `FlickDesign`, `FlickJapaneseLayout`, `FlickKeyView`, `FlickKeyboardView`, `FlickModels`, `FlickSuggestShapes`, `FlickSuggestView`, `FlickTaigiLayout`, `TaigiFlickKeyboardView`)
- **ActionHandler+CharacterInput.swift**: Logic merged into `ActionHandler`
- **Tab1 sub-views**: Removed `IssueDetailView`, `UpcomingDetailView`, `IssueType`, `UpcomingType`
- **DebugCardView.swift**: Removed unused debug card component
- **ThemeTokens.swift**: Removed legacy theme tokens

#### Changes
- **Layout preview assets**: Moved from app-only `Assets.xcassets` to shared `Styling/LayoutPreviewAssets.xcassets` for keyboard extension access

#### New Files
- `LayoutSelectionOverlay.swift` — In-keyboard layout selection overlay
- `Styling/LayoutPreviewAssets.xcassets` — Shared layout preview images

### Android

#### New Features
- **Keyboard layout type preference**: New `keyboardLayoutType` property in `PrefHelper` supporting `phahTaigi`, `qwerty`, `moe1`, `moe2` with backward compatibility for `phahTaigiLayoutEnabled`
- **Layout selection UI**: `Tab2Fragment` updated from binary PhahTaigi/Standard toggle to 4-option layout picker (PhahTaigi, Lohankha, MOE1, MOE2)
- **Space bar input mode label**: Space bar displays current input mode (POJ/TL/EN) in character keyboard mode

#### Performance
- **Guard debug logs**: Guard `Log.d` calls with `BuildConfig.DEBUG` across `KeyView`, `ComposingManager`, `TextInputManager` hot paths
- **Cache typeface and color settings**: Cache `FontUtils.getTypefaceByType()` and `KeyboardColorSettings` at `KeyboardView` level with dirty-check, eliminating per-frame resolution in `onDraw()`
- **Reuse TaigiAutocompleteService**: Reuse instance across keystrokes, only recreated when input mode changes
- **Eliminate hot-path runBlocking**: Pass warmed-up `PrefHelper` through service chain (`LexiconService`, `NextWordService`, `TaigiAutocompleteService`) to avoid ~19 `runBlocking` calls per keystroke
- **Pre-compile regex**: Pre-compile `Regex` patterns in `TaigiPhonetics.placePOJToneMark()` and `TextInputManager` double-space-period
- **Async display derivation**: Move `ComposingManager.deriveDisplay()` off main thread to `Dispatchers.Default`
- **Candidate overlay RecyclerView**: Migrated expanded candidate overlay from `ScrollView` + `LinearLayout` to `RecyclerView` with row-level ViewHolder recycling (`CandidateOverlayAdapter`)

#### Bug Fixes
- **SyllableSegmenter split bug**: Fixed `split("-", limit = -1)` crash (Kotlin rejects negative limits)
- **Privacy policy URL**: Fixed broken privacy policy link (added `.html` extension)
- **Key font size**: Adjusted base key font size ratio from 0.42 to 0.45 for better readability

#### Refactoring
- **ToneConverter delegated**: Replaced redundant lookup-table engine with thin wrapper around `TaigiPhonetics.convertToToneMarks()`
- **ToneRestoration extracted**: New standalone `ToneRestoration` object using NFD decomposition (matching iOS architecture)
- **SyllableSegmenter dedup**: Replaced inline phonetics data with references to `TaigiPhonetics` (single source of truth)
- **Cross-module naming alignment**: `getSuggestions` → `autocomplete`, `Association` → `AssociationEntry`, `getAllAssociations` → `allAssociations`, `TONE_MARK_TO_NUMBER` → derived from `TaigiPhonetics.combiningToToneNum`, `CHECKED_ENDINGS` → `checkedEndings`

#### Removed
- **Fragment-based settings**: Removed `Tab1Fragment`, `Tab2Fragment`, `Tab3Fragment`, `Tab4Fragment`, `ImageSlideshowView`, `LocalizedTextView`
- **Old XML layouts**: Removed `activity_appearance_settings.xml`, `activity_detail.xml`, `activity_main_tabs.xml`, `activity_setup_guide.xml`, `circle_number_background.xml`
- **Old menus**: Removed `bottom_navigation.xml`, `settings_navigation.xml`
- **DebugActivity**: Removed debug activity declaration from `AndroidManifest.xml`

#### Changes
- **Jetpack Compose migration**: Settings screens migrated from Fragment+XML to Jetpack Compose (`MainSettingsScreen`, `AppearanceSettingsScreen`, `DetailScreen`, `InputModeScreen`, `SetupGuideScreen`)
- **Layout selector style alignment**: Aligned layout card styling with iOS (corner radius, border width, overlay opacity, font weight, title color, capsule background on coming soon card, section header weight)
- **Build tooling upgrade**: AGP 8.x → 9.0.0, Kotlin → 2.2.10, Gradle wrapper updated
- **Unit test configuration**: Added `unitTests.isReturnDefaultValues = true` for `android.util.Log` mock support

#### New Files
- `CandidateOverlayAdapter.kt` — RecyclerView adapter for candidate overlay rows
- `candidate_overlay_row.xml` — Row layout for candidate overlay RecyclerView

### Dictionary

#### New Data Sources
- **STTI (學科術語辭典)**: Added full pipeline (`01_extract.py` through `09_add_variants.py`) and ~4,400 entries from 教育部學科術語辭典
- **齒盤補充辭典**: Added full pipeline and ~3,950 entries (keyboard supplementary vocabulary)

#### Database Migration
- **TL-only database**: Removed POJ column from `dictionary.db`; added `poj_num` column to `trie.db` for dual-prefix key generation
- **Dual-prefix trie**: MARISA trie stores `tl:` and `poj:` prefixed keys for mode-native matching; replaces single TL-only namespace
- **Reduced database size**: `dictionary.db` 44MB → 41MB; `dictionary.trie` 4.3MB → 4.4MB (slight increase from dual-prefix keys)

#### Raw Data
- **Ai Ong Taigi**: Added raw dictionary CSV, English words CSV, and Hanji corrections CSV for future integration

#### Data Cleanup Improvements
- **Preserve romanization-only entries**: No longer removes entries with empty `hanzi`; only removes entries with empty `tl`
- **Clear roman letters in hanzi**: New `check_roman_in_hanzi` option clears `hanzi` fields containing Latin letters or tone diacritics instead of discarding the entire entry
- **Merge deduplication fix**: `groupby` now uses `dropna=False` to retain entries where `hanzi` is NaN

#### Dictionary Pipeline
- `01_merge_csv.py`: Added `stti` source column and input file
- `02_create_app_db.sh`: Removed POJ column, added `stti` column to schema
- `03_create_trie_db.sh`: Unified to single TL-only trie table
- `04_create_trie.py`: Generate `tl:` and `poj:` prefixed keys from `poj_num` column
- `05_generate_association.py`: Added `stti` to source columns; added `generate_phrase_associations()` for char→phrase entries on 3+ char words (renamed from `08_generate_association.py`)
- Removed: `06_generate_android_test.py`, `07_generate_ios_test.py`
- Multiple source cleanup scripts updated (`itaigi`, `taihoa`, `taijit`)

### Shared
- Updated dictionary database (`dictionary.db`, `dictionary.trie`) for both iOS and Android
- Updated InputNormalizer test cases (iOS and Android)
- Translated all spec documents and CLAUDE.md from Chinese to English
- Added new spec documents: `moe-taigi-reference.md`, `moe-taigi-asr-reference.md`, `segmentation.md`
- Updated docs to reflect TL-only trie and unified TaigiPhonetics engine
- Updated Khiin reference with detailed architecture comparison
- Removed unused image files (`IMG_0656.png`, `images.png`)
- Reorganized `docs/` directory: engine/ (8 files), ui/ (6 files), references/ (5 files), root (7 files); deleted obsolete files (hamster-case, todo, custard, instruction, kk10, modulize, librime-predict); renamed for clarity; merged page-mapping into file-structure; fixed all cross-references
