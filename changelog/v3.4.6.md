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
