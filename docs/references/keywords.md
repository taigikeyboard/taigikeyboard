# Keyword Mapping

Standardized keyword mapping for core input method functionality and UI components.

---

## Code Naming Conventions

How engine and platform identifiers name recurring concepts (maintainability rounds R9 / R10, 2026-10-01). New code follows these; the frozen names below keep their spelling because data on users' devices or in shipped files already uses it.

| Term | Meaning in code | Examples |
|------|-----------------|----------|
| **hanji** | Han characters of a word. Every code identifier spells it `hanji` (Taiwanese romanization of 漢字), never `hanzi` | `DictionaryRecord.hanji`, `is_hanji`, proto `CandidateMessage.hanji`, iOS/Android `isHanjiFirst` |
| **Identity** | A word is the `(hanji, canonical-TL)` pair — neither field alone (AGENTS.md Core Principle #6) | `CandidateMessage.canonical_tl`, user-data `(word, tl)` keys |
| **Record** | One decoded row of a bundled binary file | `lexicon::DictionaryRecord` (`dictionary.bin`) |
| **Row** | One row of a user-data SQLite store, a search result, or a test fixture row | `userdata::{CustomDictionaryRow, LearnedPhraseRow, FrequencyRow, AssociationRow}`, `lexicon::search::SearchRow`, `test_support::TkdbRow` |
| **Entry** | An in-memory value a reader or the ranker consumes | `lexicon::AssociationEntry` (`association.bin`), `lexicon::{CustomEntry, LearnedEntry}` (user rows handed to continuous fetch) |
| **Key** | A lookup string `<family>:<body>` in `dictionary.fst` or a user store; `KeyFamily` names the family | `phonetics::KeyFamily::search_key`, `phonetics::CustomSearchKey`; not to be confused with `ranking::CandidateSortKey` (an ordering key) |
| **notone** | The stored toneless form a pipeline or derivation produces — the name the `dictionary.csv` columns use | `tl_notone`, `tps_notone_from_tl`, `derive_notone` |
| **toneless** | The runtime matching side: keys built from the typed buffer and the guards that compare them | `custom_toneless_key`, `matches_continuous_toneless_key`, `KeyFamily::toneless_face` |
| **shadow** | The canonicalized working copy of the raw composing buffer (mode-aware POJ fold, hyphens stripped) that the lattice segments; offset maps lead back to raw bytes | `composing::shadow::{build_continuous_keys, ShadowLattice}` |
| **requests** | A domain crate's request façade: decodes its proto request, runs it, encodes the response | `composing::requests::handle`, `lexicon::requests::handle`; the `engine/dispatch` crate routes the envelope to them |
| **previous / next** | The two words of an association (bigram) | `nextword::api::Association { previous, previous_tl, next, next_tl }`, `userdata::FollowingRow { next, next_tl }` |
| **predictions visible** | Whether the next-word strip is showing | nextword `SetPredictionsVisible`, `DecideResult.predictions_visible` |

**Frozen persisted names** (keep the old spelling; renaming breaks stored data or shipped files):

- `hanzi` — SQLite columns (`custom_dictionary.db`, `learned_phrases.db`), `.taigi` backup JSON key `customDictionary[].hanzi`, the `hanzi:` FST key prefix (`phonetics::HANJI_KEY_PREFIX`), the `dictionary.csv` column, the `hanzi_len` field of the `dictionary.bin` layout, i18n keys `dictionary.hanziLabel` / `hanziPlaceholder`, iOS `Suggestion.additionalInfo["hanzi"]`.
- `prev_word` / `prev_tl` — SQLite columns, `.taigi` JSON keys `prevWord` / `prevTl`, the `association.bin` layout.
- `isTranslateSwapped` / `keyboard__is_translate_swapped` settings keys, the `toggleTranslateSwapped` shortcut action id, `didMoveTranslateSwappedDefaultToBacktick`.
- Retired proto field tags stay `reserved`.

---

## Core Input Method Keywords

### 1. Composing (`engine/composing.md`)
Engine state machine lives in Rust `engine/composing` (since v3.5.4). Platform side is the effect interpreter.

| Keyword | Definition | Owner |
|---------|-----------|-------|
| **rawInput** | Numeric-tone ASCII preedit (e.g. `gua2`) — drives lexicon search-key | Rust `composing::Phase::Continuous { raw }` (the pending tail) |
| **composingText** | Derived display text (e.g. `guá`) — Rust applies tone marks per `AppConfig.input_mode` | Rust `composing::derived` |
| **ComposingState** | `Phase::Idle` or `Phase::Continuous { raw, caret, nailed, conversion }` | Rust `composing::EngineState` |
| **Intent** | Input intents: text input (Start / Append / AppendHyphen / ReplaceLast / DeleteBackward / CommitRaw / CommitPreeditThenInsertExternal / Reset), continuous input (FetchAtPos / CommitContinuous), desktop keys (TelexKey / MoveCaret / TpsKey / CommitAsShown / CommitAsTyped) — 15 ops in `composing.proto` `oneof method`; `CommitDerived` / `EnterContinuous` / `ResetContinuous` were removed in R12 (tags 15 / 30 / 33 reserved), `SelectCandidate` removed 2026-10-09 (round A1a) (tag 17 reserved) | Rust `composing::Intent` |
| **Effect** | Platform-neutral effect enum (updatePreedit / clearPreeditWithoutCommit / commitTextReplacingPreedit / clearCandidates / refreshCandidates / resetCandidateContext / nextWord*; `deleteBackwardFromDocument` was removed in R12, tag 4 reserved) | Rust `composing::transition` |
| **commitComposition** | Effect interpreter inserts derived text + clears preedit | iOS `ComposingDelegate.execute(_:)` / Android `ComposingDelegate` |
| **markedText** | iOS inline composition display via `setMarkedText` | iOS `HostTextWriter.update(_:)` |

### 2. Autocomplete (`engine/continuous-candidate-display.md`)
| Keyword | Definition | Owner |
|---------|-----------|-------|
| **Suggestion** | A candidate word (text + title + subtitle + metadata) | iOS KeyboardKit `AutocompleteSuggestion` (built in `Candidates/Services/TaigiAutocompleteService.swift`) / Android `ime/text/candidates/CandidateStrip.kt` composable |
| **InputType** | Retired 2026-09-30 with the `Search` op: the proto enum, `SearchRequest` and the in-process `SearchInputType` are gone; romanization queries go through `lexicon::search::search`, hanji queries through `search_by_hanji` | `engine/lexicon/src/search.rs` |
| **literalRomanCandidate** | The typed-text candidate (the romanization as typed), built in the engine; `FetchAtPos.literal_roman_candidate_disabled` (§34 / S22) gates it | `engine/protos/proto/composing.proto` (`literal_roman_candidate_disabled`), `engine/composing/src/requests.rs` |
| **phraseSuggestion** | Learned phrase candidates inserted at position 1 | Rust `lexicon::lookup_associations` |
| **searchKey** | fst lookup key (`tl:` / `poj:` / `hanzi:` prefix + normalized form) | Rust `phonetics::KeyFamily::search_key` (`hanzi:` = `phonetics::HANJI_KEY_PREFIX`) |

### 3. Tone Engine (`engine/tone.md`)
All tone logic lives in Rust `engine/phonetics` (since v3.5.1). Bridge surface: `RustEngineBridge+Phonetics.swift` / `PhoneticsBridge.kt` (`tlToPoj`, `externalLookupDigitForm`, …); preedit tone rendering runs inside composing ops.

| Keyword | Definition | Owner |
|---------|-----------|-------|
| **numericTone** | Tone as digit suffix: 1-9 (1,4 = no diacritic) | Rust `phonetics::tables::COMBINING_TO_TONE_NUM` |
| **toneMarks** | Unicode diacritics: á(2), à(3), â(5), ā(7), a̍(8) | Rust `phonetics::api::to_tone_marks` |
| **tonePosition** | Vowel receiving the diacritic (TL vs POJ rules differ) | Rust `phonetics::poj::to_poj` / `phonetics::tl::to_tl` |

### 4. Dictionary & Lexicon (`engine/binary-format.md`, `engine/sort.md`)
fst prefix index (replaced MARISA in v3.5.6) + dictionary/association mmap readers all live in Rust `engine/lexicon`.

| Keyword | Definition | Owner |
|---------|-----------|-------|
| **fst prefix index** | `dictionary.fst` — Burntsushi `fst` crate, stores `key → rowid` for `tl:` / `poj:` / `hanzi:` keys | Rust `lexicon::prefix_index::PrefixIndex` |
| **prefixSearch** | Iterate keys with a given prefix, returning rowid list | Rust `lexicon::search::search` |
| **DictionaryReader** | Binary mmap reader: rowid → `DictionaryRecord {bitmask, frequency, hanji, tl, syllable_count, kautian_subtag, walker_cost}` | Rust `lexicon::dictionary_reader::DictionaryReader` |
| **AssociationReader** | Binary mmap reader: prev_word → bigram entries | Rust `lexicon::association_reader::AssociationReader` |
| **EnabledDictionaries** | Per-source toggle + 16-bit `source_bitmask` for filter | Rust `lexicon::dictionary_filters` (`DictionarySourceToggles` → `dictionary_filter_bitmask` / `association_bitmask`; replaced the deleted iOS / Android `EnabledDictionaries` files; bitmask layout from `binary-format.md`) |
| **bitmaskFilter** | 16-bit source bitmask replaces SQL WHERE for dictionary filtering | Rust `lexicon::dictionary_reader::Filter` |
| **InputNormalizer** | Converts any input form to TL numeric tone format | Rust `phonetics::normalization::normalize_input` |
| **searchKey** | Normalized key format: prefix + lowercase, no hyphens, numeric tones (e.g. `tl:gua2si7`) | Rust `phonetics::KeyFamily::search_key` |
| **scoringFormula** | Retired additive score (removed 2026-09-25). Ranking is the lexicographic `CandidateSortKey`: `coverage_kind`, `tier`, `-user_weight`, `context_rank`, `walker_cost`, `-score`, `-frequency`, `-coverage`, `source_rank`, `stable_idx` | Rust `ranking::sort_key::CandidateSortKey` (`engine/ranking/src/sort_key.rs`) |
| **userFrequency** | Per-word usage count, dominates ranking. `user_frequency.db`, engine-owned; read inside `FetchAtPos`, written by `RecordUsage` | Rust `userdata::UserFrequencyStore` |
| **timeDecay** | Exponential decay of the user weight, `exp(−age / USER_WEIGHT_DECAY_TAU_MS)` (τ = 30 days); the 1-hour `RECENCY_WINDOW_MS` bonus is retired | Rust `ranking::score::decayed_user_weight_delta` (`USER_WEIGHT_DECAY_TAU_MS`) |

### 5. Segmentation — ARCHIVED (removed in v3.4.6)
| Keyword | Definition | Notes |
|---------|-----------|-------|
| ~~**SyllableSegmenter**~~ | Removed in v3.4.6 | Replaced by Rust `composing::syllabifier` (see `engine/syllabifier.md`) |

### 6. Next-Word Prediction (`engine/nextword.md`)
NextWord state machine lives in Rust `engine/nextword` (since v3.5.5). Platform glue handles timer/threading + UI.

| Keyword | Definition | Owner |
|---------|-----------|-------|
| **bigram** | Word-level prediction from association.bin (lookup by previous word) | Rust `lexicon::lookup_associations` |
| **userAssociation** | User-learned word associations (`user_association.db`, engine-owned; read by `PredictNext`, written from `nextword::Handled.associations`) | Rust `userdata::UserAssociationStore` |
| **lastSelectedWord** | Context trigger for next-word prediction | Rust `nextword::PersistedState.last_selected_word` |
| **decayScoring** | RIME-style decay + dict/user weighting | Rust `nextword::scorer` |
| **currentGeneration** | u64 counter that drops stale async results | Rust `nextword::PersistedState.current_generation` |

### 7. Custom Dictionary (`engine/custom-dictionary.md`)
| Keyword | Definition | Key Class/Method |
|---------|-----------|-----------------|
| **CustomDictionaryEntry** | User-defined word (roman + hanji; search keys derived into `custom_search_key`) | proto `CustomDictionaryEntry` / Rust `userdata::CustomDictionaryRow` |
| **notone** | Toneless romanization for prefix matching (e.g. `"lí hó"` → `"liho"`) | Rust `phonetics` `derive_notone` |
| **abbrev** | Leading-spelling-unit abbreviation (§46) for quick lookup (e.g. `"lí hó"` → `"lh"`) | Rust `phonetics::derive_abbrev` |
| **batchImport** | CSV import with deduplication by `roman\|hanzi` key | engine op `ImportCustomCsv` → `CustomDictionaryStore::batch_import` |
| **customWordMarker** | Custom entries use `id = -2` to distinguish from system dictionary | iOS `DictionarySearchResult.customDictMarkerId`; Android `DictionarySearchResult.CUSTOM_DICT_MARKER_ID` |

### 8. Diagnostics (`engine/diagnostics.md`)
| Keyword | Definition | Key Class/Method |
|---------|-----------|-----------------|
| **DiagnosticInfo** | Minimal device metadata (appVersion, buildNumber, osVersion, deviceModel) | `DiagnosticService.gather()` |
| **diagnosticActions** | Copy / Share / Email — user-initiated only, no automatic transmission | Tab4 UI buttons |

### 9. Keyboard Overlays (v3.4.5+)
| Keyword | Definition | Key Class/Method |
|---------|-----------|-----------------|
| **SymbolOverlay** | Quick symbol insertion overlay on candidate bar | `SymbolSelectionOverlay` / `SymbolSelectionOverlayView` |
| **SettingsOverlay** | Quick settings toggle overlay on candidate bar | `SettingsSelectionOverlay` / `SettingsSelectionOverlayView` |
| **LayoutOverlay** | Layout switcher overlay on candidate bar | `LayoutSelectionOverlay` / `LayoutSelectionOverlayView` |
| **SymbolData** | Symbol definitions (punctuation, math, etc.) | `SymbolData` |
| **ToolbarManager** | Toolbar visibility and overlay mode switching (Android) | `ToolbarManager.kt` |

### 10. Data Management (v3.4.5+)
| Keyword | Definition | Key Class/Method |
|---------|-----------|-----------------|
| **Backup (`.taigi`)** | Export/import user data (custom dict, frequency, associations) | engine ops `ExportBackup` / `ImportBackup` → `userdata::export_backup` / `import_backup` |
| **DataManagement** | Production UI for user data (replaced Debug screens) | `DataManagementView` / `DataManagementScreen` |
| **DictionarySearch** | In-app dictionary search from settings (uses `RustEngineBridge.searchByHanji` for hanzi inputs) | iOS / Android `DictionarySearchService` (policy, over a `LexiconClient`); Android `DictionarySearchViewModel`; Windows + Linux `taigi_desktop_core::engine::dictionary_search` (one lookup for both settings windows, custom rows through `SearchCustomEntries`) |

### 11. Input Flow (`architecture/system-overview.md` §4)
| Keyword | Definition | Key Class/Method |
|---------|-----------|-----------------|
| **ActionHandler** | Central dispatcher for all keyboard actions | `ActionHandler` |
| **characterInput** | Letter/digit keystroke → composing logic | `handleCharacterInput()` |
| **spaceAction** | Commit composing + insert space | `handleSpaceAction()` |
| **backspaceAction** | Delete within composing or text field | `handleBackspaceAction()` |
| **returnAction** | Confirm selected candidate or commit composing | `handleReturnAction()` |
| **refreshCandidates** | Engine effect `RefreshCandidates`: re-query candidates after the composition changed (iOS runs KeyboardKit `performAutocomplete()`) | `KeyboardViewController.performAutocomplete()` |

---

## UI Region Keywords

### Region Map
```
┌─────────────────────────────────────┐
│          CandidateBar               │  ← Smartbar / Candidate row
├─────────────────────────────────────┤
│  Q  W  E  R  T  Y  U  I  O  P      │  ← AlphaRow1
│   A  S  D  F  G  H  J  K  L        │  ← AlphaRow2
│  ⇧  Z  X  C  V  B  N  M  ⌫        │  ← AlphaRow3
│  🌐 123  ,     Space     .  Enter   │  ← SystemRow
└─────────────────────────────────────┘
         ↑ Flick callout overlay
         ↑ ExpandedCandidateOverlay (grid)
```

### UI Components
| Keyword | Region | Description | Key Class |
|---------|--------|-------------|-----------|
| **CandidateBar** | Top strip | Horizontal scrolling candidate row | `CandidateView` |
| **CandidateCell** | Inside CandidateBar | Individual candidate item (roman + hanzi) | `CandidateCell` |
| **ExpandedOverlay** | Full-screen overlay | Grid view of all candidates | `ExpandedCandidateOverlay` |
| **AlphaRow1-3** | Main area | Letter key rows | `TaigiLayouts` |
| **SystemRow** | Bottom row | Globe, 123, comma, space, period, enter | `TaigiLayouts` |
| **LongPressCallout** | Overlay on key | Tone 8 / special character popup | `TaigiCallouts+Builder.swift` |
| **MarkedText** | Inline in text field | Underlined composing text | `setMarkedText()` |

### App Screens (Main App, not keyboard extension)
| Keyword | Tab | Description | Key View |
|---------|-----|-------------|----------|
| **HomeTab** | Tab 1 | Setup guide, feature overview | `App/Tabs/Home/HomeTab.swift` (tabs hosted by `App/ContentView.swift`) |
| **LayoutTab** | Tab 2 | Keyboard layout preview & selection | `App/Tabs/Layout/LayoutTab.swift` |
| **DictionaryTab** | Tab 3 | Dictionary management | `App/Tabs/Dictionary/DictionaryTab.swift` |
| **SettingsTab** | Tab 4 | Input mode, appearance, advanced settings | `App/Tabs/Settings/SettingsTab.swift` |

### Device Adaptation (`ui/device.md`)
| Keyword | Definition |
|---------|-----------|
| **phoneCompact** | iPhone SE / small screens |
| **phoneRegular** | Standard iPhone |
| **phoneLarge** | iPhone Plus / Max |
| **pad** | iPad |

### Theme & Styling (`ui/theme.md`)
| Keyword | Definition |
|---------|-----------|
| **KeyboardColorSettings** | User-customizable color overrides (RGBA, stored as JSON) |
| **ButtonFontProvider** | Resolves font + scale per action/layout |
| **ButtonTextProvider** | Button labels, tone hints, punctuation hints |
| **ButtonImageProvider** | SF Symbols images for special keys |
| **deviceFont** | Font size scaled by device classification |

