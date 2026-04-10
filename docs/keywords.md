# Keyword Mapping

Standardized keyword mapping for core input method functionality and UI components.

---

## Core Input Method Keywords

### 1. Composing (`engine/composing.md`)
| Keyword | Definition | Key Class/Method |
|---------|-----------|-----------------|
| **rawInput** | Original keystrokes (e.g. `gua2`) — used for Trie search | `ComposingManager.rawInput` |
| **composingText** | Derived display text (e.g. `guá`) — computed via ToneConverter | `ComposingManager.composingText` |
| **ComposingState** | Dual-state model: `.idle` / `.composing(raw:)` | `ComposingManager.state` |
| **commitComposition** | Finalize composing text and insert into text field | `ComposingManager.commitComposition()` |
| **selectSuggestion** | Pick a candidate, clear composing state, insert text | `ComposingManager.selectSuggestion()` |
| **markedText** | iOS inline composition display via `setMarkedText` | `KeyboardViewController.setMarkedText()` |

### 2. Autocomplete (`engine/autocomplete.md`)
| Keyword | Definition | Key Class/Method |
|---------|-----------|-----------------|
| **Suggestion** | A candidate word (text + title + subtitle + metadata) | `Autocomplete.Suggestion` |
| **InputType** | Classification: `.hanzi` / `.romanWithTone` / `.romanWithoutTone` | `AutocompleteService.determineInputType()` |
| **composingTextSuggestion** | Position 0 candidate — always the current composing text | `createComposingTextSuggestion()` |
| **contextBoost** | Promote candidates matching bigram predictions from last selected word | `applyContextBoost()` |
| **phraseSuggestion** | Learned phrase candidates inserted at position 1 | `queryPhraseSuggestions()` |
| **searchKey** | Segmented + tone-filled key for Trie lookup | `buildSearchKey()` |

### 3. Tone Engine (`engine/tone.md`)
| Keyword | Definition | Key Class/Method |
|---------|-----------|-----------------|
| **numericTone** | Tone as digit suffix: 1-8 (1,4 = no diacritic) | `TaigiPhonetics.combiningToToneNum` |
| **toneMarks** | Unicode diacritics: á(2), à(3), â(5), ā(7), a̍(8) | `ToneConverter.convertToToneMarks()` |
| **tonePosition** | Vowel receiving the diacritic (TL vs POJ rules differ) | `TaigiPhonetics.placeTLToneMark / placePOJToneMark` |
| **toneRestoration** | Re-apply tone after backspace deletes a diacritic | `ToneConverter.restoreTone()` |
| **flickTone** | Swipe direction maps to tone: left(2), top(3), right(5), bottom(7), long-press(8) | `FlickDirection` |

### 4. Dictionary & Trie (`engine/trie.md`, `engine/sort.md`)
| Keyword | Definition | Key Class/Method |
|---------|-----------|-----------------|
| **MARISA Trie** | Compact prefix trie storing `key→rowid` mappings | `TrieService` |
| **prefixSearch** | Find all entries matching a key prefix | `TrieService.prefixSearch()` |
| **InputNormalizer** | Converts any input form to TL numeric tone format | `InputNormalizer.normalize()` |
| **trieKey** | Normalized key format: lowercase, no hyphens, numeric tones (e.g. `gua2si7`) | `InputNormalizer` |
| **scoringFormula** | `userFreqScore(×100) + recencyBonus(+200) + exactBonus(+100) + baseFreqScore` | `calculateScore()` |
| **userFrequency** | Per-word usage count, dominates ranking | `recordUsage()` |
| **timeDecay** | Exponential decay with 1-week half-life for recency | `calculateWeight()` |

### 5. Segmentation — ARCHIVED (removed in v3.4.6)
| Keyword | Definition | Key Class/Method |
|---------|-----------|-----------------|
| ~~**SyllableSegmenter**~~ | Removed in v3.4.6. See `engine/segmentation.md` for historical reference | (deleted) |

### 6. Next-Word Prediction (`engine/nextword.md`)
| Keyword | Definition | Key Class/Method |
|---------|-----------|-----------------|
| **bigram** | Character-level prediction from dictionary data | `NextWordService.predict()` |
| **userAssociation** | Word-level associations learned from user selections | `user_association` table |
| **lastSelectedWord** | Context trigger for next-word prediction | `ActionHandler.lastSelectedWord` |
| **phraseLearning** | Multi-word sequences learned from user input patterns | `NextWordService.queryPhrases()` |
| **sentenceStart** | Special token `$` for beginning-of-sentence predictions | bigram table |

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
| **DictionarySearch** | In-app dictionary search from settings | `DictionarySearchViewModel` |

### 11. Input Flow (`engine/flow.md`)
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
| **FlickCallout** | Overlay on key | 4-direction tone swipe indicator | `FlickKeyDef` |
| **LongPressCallout** | Overlay on key | Tone 8 / special character popup | `Callouts+TaigiCalloutBuilder` |
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

---

## Cross-Reference: Spec File → Keywords

| Spec File | Primary Keywords |
|-----------|-----------------|
| `engine/composing.md` | rawInput, composingText, ComposingState, markedText |
| `engine/autocomplete.md` | Suggestion, InputType, composingTextSuggestion, contextBoost |
| `engine/tone.md` | numericTone, toneMarks, tonePosition, toneRestoration |
| `engine/trie.md` | MARISA Trie, prefixSearch, trieKey, InputNormalizer |
| `engine/sort.md` | scoringFormula, userFrequency, timeDecay |
| `engine/nextword.md` | bigram, userAssociation, phraseLearning, lastSelectedWord |
| `engine/flow.md` | ActionHandler, characterInput, performAutocomplete |
| `engine/segmentation.md` | ~~SyllableSegmenter~~ (archived, removed v3.4.6) |
| `engine/tps.md` | TPSConverter, containsTPS, toTL, palatalization, nasalizedVowelAutoCorrect |
| `engine/custom-dictionary.md` | CustomDictionaryEntry, notone, abbrev, batchImport, customWordMarker |
| `engine/diagnostics.md` | DiagnosticInfo, diagnosticActions |
| `ui/layout.md` | AlphaRow, SystemRow, TaigiLayouts, MOE layouts |
| `ui/flick.md` | FlickCallout, FlickDirection, flickTone |
| `ui/case.md` | KeyboardCase, CaseTransformer, autoCapitalization |
| `ui/theme.md` | KeyboardColorSettings, ButtonFontProvider, deviceFont |
| `ui/device.md` | phoneCompact/Regular/Large, pad |
| `ui/app-ui.md` | HomeTab, LayoutTab, DictionaryTab, SettingsTab |
| `file-structure.md` | Directory structure, file-to-service mapping |
