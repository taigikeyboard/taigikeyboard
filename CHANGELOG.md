# Changelog

## v3.4.6 (rel-v3.4.6-bugfix-refactor-phase)

### Cross-Platform (iOS & Android)

#### New Features
- **Settings selection overlay**: In-keyboard settings panel accessible from toolbar for changing behavior settings (括號標註, 自動大本字, 自動空白, 家私櫥自動切換, etc.) without leaving the keyboard
- **Toolbar auto-collapse toggle**: New "家私櫥自動切換" setting (default: on) — when disabled, toolbar stays open during typing and mode changes
- **TPS palatalization auto-correct**: ㄗ/ㄘ/ㄙ/ㆡ + ㄧ/ㆪ auto-corrects to ㄐ/ㄑ/ㄒ/ㆢ (palatalized compound initials)
- **TPS nasalized vowel auto-correct**: ㆮ (ainn) → ㆯ (aunn) when preceded by ㄧ (since "iainn" is not a valid Taiwanese final)
- **TPS number key shortcuts**: Digit keys (0-9) accessible via long-press on TPS row 1 keys
- **TPS syllable boundary detection**: TPSConverter correctly distinguishes initial consonants from codas (e.g., ㄏ vs ㆷ, ㄫ vs ㆭ, ㄇ vs ㆬ) using syllable state tracking

#### Bug Fixes
- **TPS popup hint on symbol mode**: Fixed TPS popup hints showing on non-character keyboard modes
- **Toolbar settings not taking effect (Android)**: Settings toggles in the toolbar overlay required a keyboard restart to take effect; fixed by adding synchronous cache update in `PrefHelper` setters and refreshing consumer-level caches (`cachedOutputBothScripts`, `ComposingManager.enableDoubleTapOO/NN`) on overlay hide
- **NextWord backspace shows romanization (Android)**: After selecting a NextWord candidate (e.g., custom dictionary word), pressing backspace showed stale romanization instead of deleting committed text; fixed by calling `composingManager.reset()` after `commitText` in the NextWord path
- **Custom dictionary NavigationLink blocked (iOS)**: "自訂詞庫" link in Tab3 was unclickable due to `.onTapGesture` intercepting taps; replaced with `.scrollDismissesKeyboard(.interactively)`

#### Changes
- **Dictionary defaults**: 台語工藝詞庫 (kungge) and 學科術語辭典 (stti) now enabled by default

#### Refactoring
- **TPSConverter**: Added syllable state tracking (hasConsonant/hasVowel/lastConsonantTPS) for accurate TPS-to-TL conversion
- **ComposingManager**: Added `replaceLastCharacter()` for palatalization auto-correct; removed continuous segmentation calls, simplified search key building
- **SyllableSegmenter**: Removed continuous input segmentation (`segment()`, `groupIntoWords()`, DAG+DP algorithm); retained syllable trie and prefix validation only
- **InputNormalizer (Android)**: Simplified `buildSearchKey()` — removed segmentation-based key building

### iOS

#### New Features
- **SettingsSelectionOverlay**: New `SettingsSelectionOverlay.swift` for in-keyboard settings panel
- **TPS layout emoji key**: Added emoji key to TPS layout bottom row

#### Bug Fixes
- **Custom dictionary NavigationLink blocked**: "自訂詞庫" in Tab3 was unclickable after v3.4.5 added dictionary search bar; `.onTapGesture` on Form intercepted taps; replaced with `.scrollDismissesKeyboard(.interactively)`
- **Tab2 appearance settings card**: Fixed corner style — `.cornerRadius(10)` → `.clipShape(RoundedRectangle(cornerRadius: 12, style: .continuous))` to match iOS native Form section corners

#### Refactoring
- **ExpandedCandidateOverlay decomposed**: Extracted `ControlButton`, `candidateRow()`, `controlButtonPanel()`, `candidateScrollContent`, `backgroundView` from monolithic body
- **TaigiKeyboardView decomposed**: Extracted `keyboardWithOverlays()` to reduce `body` type-checker complexity
- **CustomLayoutService**: Extracted `resolveLayout()` helper for globe/iPhone layout resolution
- **DictionaryRepository**: Extracted `parseSearchResult()` from duplicated inline row-parsing code
- **NextWordService**: Refactored `buildDictWhereCondition()` — data-driven dictionary filter using array of tuples
- **SQLiteConnectionManager.sqliteTransient**: Shared constant replacing inline `unsafeBitCast(-1, ...)` across all repositories
- **Boolean @State naming**: Renamed to use `is`/`has` prefix per Swift conventions (e.g., `upButtonPressed` → `isUpButtonPressed`)
- **RowItem → CandidateRowItem**: Private nested type renamed with parent context prefix

#### Refactoring (continued)
- **AutocompleteService**: Simplified search key building — removed segmentation-based continuous input handling
- **ComposingManager**: Removed `segmentAndGroup()`, simplified `buildSearchKey()`

#### Unit Tests
- `TPSConverterTests`: Added 22 new test cases for syllable boundary detection, palatalization auto-correct, and non-palatalized affricate handling
- Removed `EngineIntegrationTests` (segmentation-dependent tests no longer applicable)
- Reduced `SyllableSegmenterTests` to prefix validation tests only

### Android

#### New Features
- **SettingsSelectionOverlayView**: New `SettingsSelectionOverlayView.kt` and `settings_selection_overlay.xml` for in-keyboard settings panel
- **TPS layout emoji key**: Added emoji key to TPS layout bottom row
- **Toolbar settings → settings overlay**: Settings button now opens in-keyboard settings overlay instead of launching app

#### Bug Fixes
- **TPS popup hint on symbol mode**: Fixed popup hints showing in SYMBOLS mode by adding `KeyboardMode.CHARACTERS` check
- **TaigiKeyboard null safety**: Fixed force-unwrap crash risks — `taigikeyboardInstance!!` → safe throw, `inputView!!` → safe return
- **Toolbar settings not taking effect**: All 7 toolbar settings toggles required keyboard restart; fixed `PrefHelper` setters to update in-memory cache synchronously + refresh `SmartbarManager.cachedOutputBothScripts` and `ComposingManager.enableDoubleTapOO/NN` on overlay hide
- **NextWord backspace shows romanization**: After selecting a NextWord candidate, pressing backspace restored stale composing text; fixed by resetting `ComposingManager` in both `handleCandidateClick` and `handleOverlaySuggestionSelected` NextWord paths

#### Refactoring
- Variable naming: `n` → `popupCount` (KeyPopupManager), `p` → `prefHelper` (LexiconService, NextWordService), `i` → `intent` (TaigiKeyboard), `f` → `fraction` (ToolbarManager)

#### Unit Tests
- `TPSConverterTest`: New file with 19 test cases covering syllable boundary, palatalization, and non-palatalized affricate handling
- Removed `EngineIntegrationTest` (segmentation-dependent tests no longer applicable)
- Reduced `SyllableSegmenterTest` to prefix validation tests only

### Dictionary

#### Data Updates
- Updated `dictionary.db` on both platforms
- Added dictionary build split packages script (`08_split_packages.py`) and publish release script (`09_publish_release.sh`)

### Shared

#### Documentation
- Added `taigi-converter` git submodule
- Added `knowledge/taigi-phonetics-reference.md` and `knowledge/tps-auto-correct-rules.md`
- Moved phonetics reference files from root to `knowledge/` directory
- Updated `CLAUDE.md` with project structure, phonetic conversion reference, test conventions, Swift naming conventions
- Updated `docs/engine/tps.md` and `docs/ui/app-ui.md`
- Added `docs/engine/dictionary-download.md` and `docs/reports/refactor-backlog.md`

---

## v3.4.5 (rel-v3.4.5)

### Cross-Platform (iOS & Android)

#### New Features
- **Symbol selection overlay**: In-keyboard symbol panel accessible from toolbar for inserting special characters (台語標點、數學符號、箭頭等)
- **Improved digit input**: Enhanced number key input handling on the keyboard
- **Dictionary search bar**: Search bar in dictionary settings page (Tab3) for looking up words across all enabled dictionaries; supports Hanji, romanization, and abbreviation search
- **TPS initial key auto-selection**: ㄇ and ㄫ keys auto-select between initial (ㄇ/ㄫ) and final (ㆬ/ㆭ/ㄥ) forms based on composing context; uses initial form at syllable start, final form after vowels, and ㄧ+ㄫ→ㄥ for `ing`

#### Bug Fixes
- **Trie lookup truncation**: Increased `TrieService.lookup()` maxResults from 100 to 1000; notone keys can map to hundreds of rowids, and the previous buffer silently truncated results causing missing candidates
- **removeDuplicates dropped tone variants**: Hanzi-only dedup collapsed entries sharing the same hanzi but different tones (e.g., 伴手 tone-2 and tone-7) into one candidate; fixed by using `roman|hanzi` as dedup key on both platforms
- **Symbol panel invisible characters (iOS)**: Fixed backtick (`) and macron (¯) not rendering in symbol grid by using system font for glyph coverage
- **Symbol panel tab usability**: Increased tab touch targets (iOS 24→36pt, Android 36→44dp) and top buffer (8→12pt/dp) to reduce accidental toolbar taps
- **TPS `ng` vowel mapping**: Added missing `ng → ㆭ` mapping to TPS vowel conversion table; fixed `ing` special case (ㄧㆭ → ㄧㄥ)

#### Changes
- **TPS layout punctuation**: Fixed TPS layout comma and parentheses to use half-width characters by default with fullWidth variants (iOS `TaigiLayouts`, Android `western_default.json`)
- **Dictionary settings redesign**: Tab3 redesigned with info buttons, section headers (教育部用字, 其他辭典, 補充資料), and dictionary descriptions
- **Dictionary defaults**: STTI (學科術語辭典) and Kungge (工藝中心辭典) dictionaries now disabled by default for new installs; Newword (公視台語台台語新詞) moved to second position in the list
- **Custom dictionary seed**: Removed emoji from default custom dictionary entry ("你好😀" → "你好")
- **Toolbar dismiss keyboard**: Replaced emoji button with dismiss keyboard button in expanded toolbar
- **Dictionary search result lookup order**: MOE dictionary link now appears above ChhoeTaigi link
- **Symbol panel tab animation (iOS)**: Removed tab switching animation for instant response
- **Tab1 "問題回報" renamed to "寄付"**: Feedback page simplified to donation-only; removed feedback description and Google Form link
- **Tab3 section header renamed**: "教育部推薦用字" → "教育部用字"

### iOS

#### New Features
- **SymbolSelectionOverlay**: New `SymbolSelectionOverlay.swift` and `SymbolData.swift` for in-keyboard symbol panel
- **DictionarySearchViewModel**: New `DictionarySearchViewModel.swift` and `DictionarySearchResult.swift` for dictionary search in settings
- **TPS initial key auto-selection**: `adjustTPSInitialKey` in `TPSConverter` and `ActionHandler+CharacterInput` for context-aware ㄇ/ㄫ key selection

#### Changes
- **Navigation bar font**: App navigation bar titles now use jf-openhuninn (粉圓) font via UIKit appearance
- **TPS input mode**: Added `tps` case to `InputMode` enum; TPS now recognized as a Taigi input mode
- **TPS layout improvements**: Updated `CustomLayoutService`, `ToneConverter`, `ButtonTextProvider`, `ConfirmKeyTextHelper`, `TaigiButtonContent` to handle TPS mode
- **SyllableSegmenter**: Switched from `>` to `>=` tie-breaking in DP scoring
- **Toolbar dismiss keyboard**: Replaced emoji button (`face.smiling`) with dismiss keyboard button (`keyboard.chevron.compact.down.fill`)

#### Unit Tests
- `TextProcessorTests`: Added removeDuplicates regression tests (tone variants, exact duplicates, roman-only entries)
- `TPSConverterTests`: Added tests for `ing` special case and `adjustTPSInitialKey` (11 new test cases)
- `DictionaryContentTests`: New SQL-based dictionary content verification for kautian=1 (教育部臺灣台語常用詞辭典) — total count baseline, uniqueness, tone variant golden cases, data integrity checks

### Android

#### New Features
- **TPS layout promoted**: TPS (方音符號) layout moved from "Coming Soon" to production; selectable in layout picker
- **TPSConverter**: New `TPSConverter.kt` for converting TPS input to romanization for dictionary lookup
- **TPS character maps**: Added `tps.json`, `tps_fullwidth.json`, `mod/tps_halfwidth.json`, `mod/tps_fullwidth.json`
- **Reset all settings**: Added `resetToDefaults()` in `PrefHelper` to reset preferences while preserving version info; accessible from settings
- **SymbolSelectionOverlay**: New `SymbolData.kt` and `SymbolSelectionOverlayView.kt` for in-keyboard symbol panel
- **DictionarySearchViewModel**: New `DictionarySearchViewModel.kt` and `DictionarySearchResult.kt` for dictionary search in settings
- **TPS initial key auto-selection**: `adjustTPSInitialKey` in `TPSConverter` and `TextInputManager` for context-aware ㄇ/ㄫ key selection
- **Toolbar dismiss keyboard**: Replaced emoji button with dismiss keyboard button (`ic_keyboard_hide.xml`); layout selection button moved before globe button

#### Bug Fixes
- **Overlay missing composing text**: Expanded candidate overlay now includes position 0 (composing text), matching iOS behavior

#### Refactoring
- **TextInputManager decomposed**: Extracted `CandidateUpdateCoordinator` (candidate update orchestration) and `CapsStateManager` (caps lock/sentence case state management) from `TextInputManager`
- **SmartbarManager decomposed**: Extracted `CandidateClickHandler` (candidate selection logic), `NextWordHandler` (next-word suggestion flow), and `ToolbarManager` (toolbar UI management) from `SmartbarManager`
- **AppearanceSettingsScreen decomposed**: Extracted `ColorPickerDialog`, `FontPickerContent`, `KeyboardPreviewPanel`, and shared components `ColorRow`/`SliderRow`
- **DictEnabledSnapshot**: Atomic snapshot of all dictionary-enabled flags in `PrefHelper` to avoid torn reads
- **ToneConverterModels cleaned up**: Removed unused tone mapping constants (consolidated into `TaigiPhonetics`)
- **ToneUtilities**: Simplified by delegating to `TaigiPhonetics` shared utilities
- **Build fix**: Added missing `import kotlinx.coroutines.cancel` in `TaigiKeyboard`; fixed duplicate `/**` in `PrefHelper` KDoc

### Dictionary

#### Data Updates
- Updated 台語新詞辭庫 raw data and processing pipeline
- Added LKK (李江却台語文教基金會漢羅合用建議用字) dictionary data
- Added developer supplementary dictionary data
- Updated `dictionary.db` and `dictionary.trie` on both platforms
- Updated merge script (`01_merge_csv.py`) and build script (`02_create_app_db.sh`)

### Shared

#### Documentation
- Updated `docs/ui/theme.md` to match actual Styling providers architecture (removed outdated ThemeTokens references)
- Fixed `docs/file-structure.md`: added missing `Diagnostics/` directory, corrected `Theme/` → `Styling/`
- Added `docs/engine/custom-dictionary.md` spec
- Added `docs/engine/diagnostics.md` spec
- Updated `docs/keywords.md` with CustomDictionary, Diagnostics, and corrected Theme keywords
- Updated `docs/README.md` index
- Added `docs/reports/` directory with audit reports (codebase health, docs sync, Khiin/RIME research, segmentation tie bug)
- Added context management guidelines to `CLAUDE.md`
- Added `docs/simplify.md` code review checklist

---

## v3.4.4 (rel-v3.4.2)

### Dictionary

#### Bug Fixes
- **Trie/dictionary ID mismatch**: Fixed `03_create_trie_db.sh` — trie.db was built independently from dictionary.db with no deduplication (`INSERT` vs `INSERT OR IGNORE`), causing 1,847 extra entries and divergent AUTOINCREMENT IDs. MARISA trie stored trie.db rowids, but runtime lookups used dictionary.db, returning wrong entries for any ID past the first duplicate. Fixed by `ATTACH`ing dictionary.db and `JOIN`ing to use correct IDs. Affects both iOS and Android.

### Shared
- Updated `dictionary.trie` on both platforms with corrected row IDs

---

## v3.4.2 (rel-v3.4.2)

### Cross-Platform (iOS & Android)

#### New Features
- **Custom dictionary (自訂詞庫)**: Users can add, edit, and delete custom dictionary entries; supports CSV import/export; custom entries appear with highest priority in candidate suggestions
- **Diagnostic info**: Copy, share, or email device/app diagnostic info for bug reporting (iOS Tab4, Android InputSettingsScreen)

#### Bug Fixes
- **TL-to-POJ display conversion**: Fixed `tlDisplayToPOJDisplay()` to preserve spaces as word boundaries (previously only split by hyphen, breaking word-grouped display like `guá kin-á-ji̍t`)
- **Missing TL finals**: Added `erk`, `iri`, `eeh` to `TaigiPhonetics.tlFinals` on both platforms
- **romanToBase matching**: Fixed to strip spaces in addition to hyphens for word-grouped roman matching (e.g., `m̄ bat` → `mbat`)
- **Custom dictionary search**: Fixed `generateNotone()` to strip spaces so multi-syllable entries (e.g., `lí hó` → notone `liho`) are searchable; normalized search input via `generateNotone()` so toned input (`li2ho2`, `li2-ho2`) also matches
- **Custom dictionary DB initialization (iOS)**: Fixed keyboard extension not initializing `CustomDictionaryRepository`, causing `searchSync` to always return empty; added eager initialization in `LexiconService.init()`
- **Custom dictionary search bypasses segmenter (iOS)**: Pass unsegmented `rawInput` to custom dictionary search via new `rawInput` parameter on `LexiconService.search()`, preventing syllable segmentation from breaking prefix matching
- **Custom dictionary capitalization**: Fixed custom dictionary entries not following keyboard case rules; changed custom dict marker ID from `-1` to `-2` to distinguish from NextWord entries; iOS applies `CandidateProcessor.capitalize()`, Android `SuggestionCaseTransformer` now includes `id == -2`
- **Custom dictionary seed on clear**: Fixed default entries reappearing after user clears all entries; moved `seedDefaultEntryIfEmpty()` from page load to app/IME startup
- **Custom dictionary duplicate column migration**: Fixed `ALTER TABLE ADD COLUMN` error for existing `notone`/`abbrev` columns by checking `pragma_table_info` before adding (iOS)

#### Changes
- Renamed "腔口補充辭典" → "腔口補充資料" across both platforms

#### New Files

| iOS | Android | Description |
|-----|---------|-------------|
| `CustomDictionaryView.swift` | `CustomDictionaryScreen.kt` | Custom dictionary UI |
| `CustomDictionaryEditView.swift` | `CustomDictionaryActivity.kt` | Custom dictionary edit/activity |
| `CSVDocument.swift` | — | CSV file document type for import/export |
| `CustomDictionaryRepository.swift` | `CustomDictionaryService.kt` | Custom dictionary data layer |
| `CustomDictionaryEntry.swift` | — | Custom dictionary model |
| `CustomDictionaryService.swift` | — | Custom dictionary service layer |
| `DiagnosticService.swift` | `DiagnosticService.kt` | Device/app diagnostic info gathering |
| — | `DiagnosticTexts.kt` | Diagnostic localization texts |

### iOS

#### Changes
- Removed "Coming Soon" label from custom dictionary section (now functional)

### Android

#### Changes
- **App font changed to 粉圓 (jf-openhuninn)**: All Compose typography styles now use HuninnFontFamily instead of system default
- **Candidate layout redesigned**: Candidate items now show title and subtitle vertically stacked (matching iOS CandidateButtonView), replacing single-line Button
- **Smartbar height increased**: From 50dp to 56dp to accommodate new candidate layout

### Dictionary

#### Data Updates
- Updated `dictionary.db` and `dictionary.trie` on both platforms
- Updated 齒盤補充辭典 (khpoo) data

### Shared
- Removed unused files: `design.md`, `flick.html`

---

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

---

## v3.3.10 (develop-kk10)

### Android

#### Performance Optimization
- **Candidate display refactor**: Migrated from LinearLayout to RecyclerView + DiffUtil + ListAdapter
- **Debounce + Cancel mechanism**: Added debounce and cancellation to candidate updates to prevent redundant calculations
- **UI update performance**: Reduced from ~145ms to 4-6ms

#### Bug Fixes
- **isTranslateSwapped toggle failure**: After RecyclerView refactor, DiffUtil could not detect state changes; added `notifyDataSetChanged()`
- **Dark Mode icon visibility**: Fixed fillColor for `ic_translate`, `ic_keyboard_arrow_up`, `ic_keyboard_arrow_down`, `ic_backspace`
- **Symbol keyboard duplicate number row**: Removed redundant `number_row` extension in SYMBOLS mode
- **English spellcheck timeout**: Added 2-second timeout to prevent coroutine blocking

#### Feature Changes
- **Search syllable limit**: Increased from 3 syllables to 4
- **Removed Recent Emoji**: Deleted `EmojiHistory.kt`, `EmojiHistoryManager.kt`, and `RECENTLY_USED` category

#### New Files
- `CandidateAdapter.kt`: RecyclerView candidate adapter
- `item_candidate.xml`: Candidate item layout

### Shared

#### Dictionary Scripts
- `02_create_app_db.sh`: Syllable limit changed from `<= 3` to `<= 4`
- `03_create_trie_db.sh`: Syllable limit changed from `<= 3` to `<= 4`

---

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
