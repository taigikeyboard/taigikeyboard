# iOS Data Flow

> **Type**: Feature
> **Keywords**: `DataFlow`, `Pipeline`, `ActionHandler`
> **Related**: composing.md, autocomplete.md

---

## Summary

- Input → Composing → Search → Display → Selection → Output
- ActionHandler dispatches events
- ComposingManager maintains dual-state

---

## Overall Flow

```
User input
    ↓
ActionHandler (event dispatch)
    ↓
ComposingManager (rawInput / composingText)
    ↓
AutocompleteService (candidate search)
    ↓
CandidateView (candidate display)
    ↓
User selection
    ↓
Text output
```

---

## Phase 1: Input Handling

### ActionHandler

| Method | Description |
|--------|-------------|
| `handle(gesture:on:)` | Main entry, dispatches key events |
| `handleCharacterInput(_:)` | Character input handling |
| `handleTaigiSpecificAction(_:)` | Taiwanese-specific actions |

### Character Input Flow

1. Case conversion (based on keyboardCase)
2. Punctuation: confirm composition first, then output directly
3. Letters/numbers: enter composing flow

---

## Phase 2: Composing Management

### ComposingManager States

| State | rawInput | composingText |
|-------|----------|---------------|
| idle | "" | "" |
| composing | "gua2" | "guá" |

### State Change Triggers

- `composing` → triggers `performAutocomplete()`
- `idle` → clears markedText

---

## Phase 3: Candidate Search

### AutocompleteService

1. Get `rawInput` (for search)
2. Determine input type
3. Call `LexiconService.search()`
4. Insert composingText at position 0

### LexiconService.search()

1. InputNormalizer normalizes
2. TrieService prefix search
3. SQLite batch query
4. UserFrequency sort

---

## Phase 4: Candidate Display

### TaigiKeyboardView

- Converts candidate case based on keyboardCase
- Passes to CandidateView

### CandidateView

- Horizontal scroll display of candidates
- Tap triggers `onSuggestionTap`

---

## Phase 5: Candidate Selection

### ActionHandler+Suggestions

1. Parse romanization and Hanzi
2. Determine output text (based on settings)
3. Call `composingManager.selectSuggestion()`
4. Record usage frequency
5. Trigger NextWord prediction

### ComposingManager.selectSuggestion()

1. Clear markedText
2. Insert selected text
3. Reset state to idle

---

## State Transition Diagram

```
[Idle] ─── input character ───→ [Composing]
   ↑                                │
   │                                ├── select candidate
   │                                ├── space confirm
   │                                ├── Enter confirm
   └────────────────────────────────┴── delete to empty → [Idle]
```

---

## Key Files

| Phase | iOS | Android |
|-------|-----|---------|
| Input | `ActionHandler.swift` + extensions | `TextInputManager.kt` |
| Composing | `ComposingManager.swift` | `ComposingManager.kt` |
| Search | `AutocompleteService.swift`, `LexiconService.swift` | `TaigiAutocompleteService.kt`, `LexiconService.kt` |
| Display | `TaigiKeyboardView.swift`, `CandidateView.swift` | `SmartbarView.kt`, `CandidateAdapter.kt` |
| Selection | `ActionHandler+Suggestions.swift` | `CandidateClickHandler.kt` |
| NextWord | `NextWordService.swift` | `NextWordHandler.kt`, `NextWordService.kt` |

### Keyboard Overlays (v3.4.5+)

In addition to the main candidate bar, the smartbar/candidate area supports overlay modes:

| Overlay | Trigger | iOS | Android |
|---------|---------|-----|---------|
| Symbol | Toolbar button | `SymbolSelectionOverlay.swift` | `SymbolSelectionOverlayView.kt` |
| Settings | Toolbar button | `SettingsSelectionOverlay.swift` | `SettingsSelectionOverlayView.kt` |
| Layout | Toolbar button | `LayoutSelectionOverlay.swift` | `LayoutSelectionOverlayView.kt` |
| Expanded candidates | Expand toggle | `ExpandedCandidateOverlay.swift` | `CandidateOverlayView.kt` |
