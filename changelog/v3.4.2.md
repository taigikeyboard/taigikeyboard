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
