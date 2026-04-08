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
