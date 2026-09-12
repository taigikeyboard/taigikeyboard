# Keyword Mapping

Standardized keyword mapping for core input method functionality and UI components.

---

## Core Input Method Keywords

### 1. Composing (`engine/composing.md`)
Engine state machine lives in Rust `engine/composing` (since v3.5.4). Platform side is the effect interpreter.

| Keyword | Definition | Owner |
|---------|-----------|-------|
| **rawInput** | Numeric-tone ASCII preedit (e.g. `gua2`) — drives lexicon search-key | Rust `composing::Phase::Composing { raw }` |
| **composingText** | Derived display text (e.g. `guá`) — Rust applies tone marks per `AppConfig.input_mode` | Rust `composing::derived` |
| **ComposingState** | `Phase::Idle` or `Phase::Composing { raw }` + `selected_candidate_index` | Rust `composing::EngineState` |
| **Intent** | 12 input intents (Start / Append / AppendHyphen / ReplaceLast / DeleteBackward / CommitDerived / CommitRaw / SelectSuggestion / CommitPreeditThenInsertExternal / Reset / SetSelectedCandidateIndex / QueryState) | Rust `composing::Intent` |
| **Effect** | Platform-neutral effect enum (updatePreedit / clearPreeditWithoutCommit / commitTextReplacingPreedit / deleteBackwardFromDocument / resetAutocomplete / performAutocomplete / resetAutocompleteContext) | Rust `composing::transition` |
| **commitComposition** | Effect interpreter inserts derived text + clears preedit | iOS `ComposingDelegate.execute(_:)` / Android `ComposingDelegate` |
| **markedText** | iOS inline composition display via `setMarkedText` | iOS `KeyboardViewController.setMarkedText()` |

### 2. Autocomplete (`engine/continuous-candidate-display.md`)
| Keyword | Definition | Owner |
|---------|-----------|-------|
| **Suggestion** | A candidate word (text + title + subtitle + metadata) | iOS `Autocomplete.Suggestion` / Android `CandidateAdapter` |
| **InputType** | Classification: `Hanzi` / `RomanWithTone` / `RomanNoTone`. Engine returns `(input_type, search_key)` | Rust `lexicon::classification::classify_input` |
| **composingTextSuggestion** | Position 0 candidate — always the current composing text | iOS `createComposingTextSuggestion()` |
| **contextBoost** | Promote candidates matching bigram predictions from last selected word | Rust `nextword::booster` |
| **phraseSuggestion** | Learned phrase candidates inserted at position 1 | Rust `lexicon::assoc_lookup` |
| **searchKey** | fst lookup key (`tl:` / `poj:` / `hanzi:` prefix + normalized form) | Rust `lexicon::key_normalizer::build` |

### 3. Tone Engine (`engine/tone.md`)
All tone logic lives in Rust `engine/phonetics` (since v3.5.1). Bridge surface: `RustEngineBridge.normalizeTone` / `restoreTone` / etc.

| Keyword | Definition | Owner |
|---------|-----------|-------|
| **numericTone** | Tone as digit suffix: 1-9 (1,4 = no diacritic) | Rust `phonetics::tables::COMBINING_TO_TONE_NUM` |
| **toneMarks** | Unicode diacritics: á(2), à(3), â(5), ā(7), a̍(8) | Rust `phonetics::api::to_tone_marks` |
| **tonePosition** | Vowel receiving the diacritic (TL vs POJ rules differ) | Rust `phonetics::poj::to_poj` / `phonetics::tl::to_tl` |
| **toneRestoration** | Re-apply tone after backspace deletes a diacritic | Rust `phonetics::normalization::restore_tone` |

### 4. Dictionary & Lexicon (`engine/binary-format.md`, `engine/sort.md`)
fst prefix index (replaced MARISA in v3.5.6) + dictionary/association mmap readers all live in Rust `engine/lexicon`.

| Keyword | Definition | Owner |
|---------|-----------|-------|
| **fst prefix index** | `dictionary.fst` — Burntsushi `fst` crate, stores `key → rowid` for `tl:` / `poj:` / `hanzi:` keys | Rust `lexicon::prefix_index::PrefixIndex` |
| **prefixSearch** | Iterate keys with a given prefix, returning rowid list | Rust `lexicon::search::search` |
| **DictionaryReader** | Binary mmap reader: rowid → `{hanzi, tl, length_score, source_bitmask}` | Rust `lexicon::dictionary_reader::DictionaryReader` |
| **AssociationReader** | Binary mmap reader: prev_word → bigram entries | Rust `lexicon::association_reader::AssociationReader` |
| **EnabledDictionaries** | Per-source toggle + 16-bit `source_bitmask` for filter | iOS `EnabledDictionaries.swift` / Android `.kt` (DTO; bitmask layout from `binary-format.md`) |
| **bitmaskFilter** | 16-bit source bitmask replaces SQL WHERE for dictionary filtering | Rust `lexicon::dictionary_reader::Filter` |
| **InputNormalizer** | Converts any input form to TL numeric tone format | Rust `phonetics::normalization::normalize_input` |
| **searchKey** | Normalized key format: prefix + lowercase, no hyphens, numeric tones (e.g. `tl:gua2si7`) | Rust `lexicon::key_normalizer::build` |
| **scoringFormula** | `userFreqScore(×100) + completionPenalty(-1000) + closenessBonus(+500) + recencyBonus(+200) + exactBonus(+100) + baseFreqScore` | Rust `ranking::score` |
| **userFrequency** | Per-word usage count, dominates ranking. Storage stays platform SQLite (`wont_migrate`) | iOS `UserFrequencyService.swift` / Android `.kt` |
| **timeDecay** | Exponential decay with 1-hour recency window for ranking-side bonus | Rust `ranking::score` constants (`RECENCY_WINDOW_MS=3_600_000`) |

### 5. Segmentation — ARCHIVED (removed in v3.4.6)
| Keyword | Definition | Notes |
|---------|-----------|-------|
| ~~**SyllableSegmenter**~~ | Removed in v3.4.6 | Replaced by Rust `composing::syllabifier` (see `engine/syllabifier.md`) |

### 6. Next-Word Prediction (`engine/nextword.md`)
NextWord state machine lives in Rust `engine/nextword` (since v3.5.5). Platform glue handles timer/threading + UI.

| Keyword | Definition | Owner |
|---------|-----------|-------|
| **bigram** | Word-level prediction from association.bin (lookup by previous word) | Rust `lexicon::assoc_lookup` |
| **userAssociation** | User-learned word associations (SQLite, `wont_migrate`) | iOS `Lexicon/Database/` / Android `ime/text/composing/UserFrequencyService.kt` |
| **lastSelectedWord** | Context trigger for next-word prediction | Rust `nextword::PersistedState.last_selected_word` |
| **decayScoring** | RIME-style decay + dict/user weighting | Rust `nextword::scorer` |
| **currentGeneration** | u64 counter that drops stale async results | Rust `nextword::PersistedState.current_generation` |

### 7. Custom Dictionary (`engine/custom-dictionary.md`)
| Keyword | Definition | Key Class/Method |
|---------|-----------|-----------------|
| **CustomDictionaryEntry** | User-defined word (roman + hanzi + derived notone/abbrev) | `CustomDictionaryEntry` |
| **notone** | Toneless romanization for prefix matching (e.g. `"lí hó"` → `"liho"`) | `generateNotone()` |
| **abbrev** | First-letter abbreviation for quick lookup (e.g. `"lí hó"` → `"lh"`) | `generateAbbrev()` |
| **batchImport** | CSV import with deduplication by `roman\|hanzi` key | `batchImport()` / `importFromFile()` |
| **customWordMarker** | Custom entries use `id = -2` to distinguish from system dictionary | `LexiconService.search()` |

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
| **BackupService** | Export/import user data (custom dict, frequency, associations) | `BackupService` |
| **DataManagement** | Production UI for user data (replaced Debug screens) | `DataManagementView` / `DataManagementScreen` |
| **DictionarySearch** | In-app dictionary search from settings (uses `RustEngineBridge.searchByHanzi` for hanzi inputs) | `DictionarySearchViewModel` |

### 11. Input Flow (`architecture/system-overview.md` §4)
| Keyword | Definition | Key Class/Method |
|---------|-----------|-----------------|
| **ActionHandler** | Central dispatcher for all keyboard actions | `ActionHandler` |
| **characterInput** | Letter/digit keystroke → composing logic | `handleCharacterInput()` |
| **spaceAction** | Commit composing + insert space | `handleSpaceAction()` |
| **backspaceAction** | Delete within composing or text field | `handleBackspaceAction()` |
| **returnAction** | Confirm selected candidate or commit composing | `handleReturnAction()` |
| **performAutocomplete** | Trigger candidate search after composing state changes | `KeyboardViewController.performAutocomplete()` |

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
| **HomeTab** | Tab 1 | Setup guide, feature overview | `ContentView` |
| **LayoutTab** | Tab 2 | Keyboard layout preview & selection | `LayoutSettingsView` |
| **DictionaryTab** | Tab 3 | Dictionary management | `DictionarySettingsView` |
| **SettingsTab** | Tab 4 | Input mode, appearance, advanced settings | `SettingsView` |

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

