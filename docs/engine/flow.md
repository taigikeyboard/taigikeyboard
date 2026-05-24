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

1. Get `rawInput` (for search) from the composing engine snapshot.
2. `RustEngineBridge.classifyInput(rawInput)` → `(input_type, search_key)` (Rust `lexicon::classify_input`).
3. `RustEngineBridge.search(...)` (Rust `lexicon::search`) — fst prefix scan + DictionaryReader rowid resolution + source-bitmask filter.
4. `RustEngineBridge.processCandidates(raw, ..., frequencyData: ..., nowMs: ...)` — Rust `ranking::process_candidates` runs dedup + score + sort.
5. Platform layer prepends `composingText` at index 0 and applies suggestion case-transform via `RustEngineBridge.transformSuggestion`.

### Engine search ownership

| Step | Rust crate / function |
|------|----------------------|
| Input classification | `engine/lexicon::classify_input` |
| Trie-key normalization | `engine/lexicon::key_normalizer::build` (calls `phonetics::normalize_input`) |
| fst prefix scan | `engine/lexicon::prefix_index::PrefixIndex` |
| Rowid → record resolution | `engine/lexicon::dictionary_reader::DictionaryReader` |
| Source bitmask filter | `engine/lexicon::dictionary_reader::Filter` |
| Candidate dedup / score / sort | `engine/ranking::process_candidates` |

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

| Phase | iOS | Android | Rust crate (if any) |
|-------|-----|---------|----------------------|
| Input dispatch | `ActionHandler.swift` + extensions | `TextInputManager.kt` | — |
| Composing engine | (Rust) | (Rust) | `engine/composing` |
| Composing wrapper | `ComposingManager.swift` + `ComposingDelegate.swift` | `ComposingManager.kt` + `ComposingDelegate.kt` | — |
| Autocomplete / classify + search | `AutocompleteService.swift` (engine-only post v3.5.8 Item 13 retire) | `TaigiAutocompleteService.kt`; `LexiconService.kt` retained for Tab3 `searchWithSources` / `searchByHanzi` only | `engine/lexicon` |
| Ranking | (calls `processCandidates` via bridge) | (calls `processCandidates` via bridge) | `engine/ranking` |
| Phonetics / case | (calls bridge) | (calls bridge) | `engine/phonetics`, `engine/case-transform` |
| Display | `TaigiKeyboardView.swift`, `CandidateView.swift` | `SmartbarView.kt`, `CandidateAdapter.kt` | — |
| Selection | `ActionHandler+Suggestions.swift` | `CandidateClickHandler.kt` | — |
| NextWord engine | (Rust) | (Rust) | `engine/nextword` |
| NextWord platform glue | `NextWord/NextWordController.swift`, `NextWord/Services/NextWordService.swift` | `ime/text/smartbar/NextWordHandler.kt`, `ime/dictionary/NextWordService.kt` | — |
| FFI seam (facade) | `Engine/RustEngineBridge.swift` | `engine/RustEngineBridge.kt` | `engine/dispatch` + `engine/swift-ffi` / `engine/android-jni` |
| FFI seam (per-slice extensions) | `Engine/RustEngineBridge+Phonetics.swift`, `+Composing.swift`, `+Lexicon.swift`, `+CaseTransform.swift`, `+NextWord.swift` (B3, PR #324) | `engine/PhoneticsBridge.kt`, `ComposingBridge.kt`, `LexiconBridge.kt`, `CaseTransformBridge.kt`, `NextWordBridge.kt` sibling impl objects (B4, PR #325) | — |
| FFI seam (infra) | `Engine/SwiftLoggerSink.swift`, `Engine/RustVec+UInt8.swift` | `ime/core/logging/LoggerBackend.kt` + `AndroidLoggerBackend.kt` (B12, PR #317 — all Android log sites route through facade) | — |
| Settings / prefs | `Settings/SharedSettings.swift` + `SettingsKey.swift` (B8a iOS PR-1/PR-2) | `ime/core/PrefHelper.kt` (B6 delegate, B11 companion-hoist, B8a `TpsCascade.kt`) | — |
| Smartbar container state | — | `ime/text/smartbar/SmartbarContainer` enum + `ToolbarManager.kt` (B7, PR #320) | — |

### Keyboard Overlays (v3.4.5+)

In addition to the main candidate bar, the smartbar/candidate area supports overlay modes:

| Overlay | Trigger | iOS | Android |
|---------|---------|-----|---------|
| Symbol | Toolbar button | `SymbolSelectionOverlay.swift` | `SymbolSelectionOverlayView.kt` |
| Settings | Toolbar button | `SettingsSelectionOverlay.swift` | `SettingsSelectionOverlayView.kt` |
| Layout | Toolbar button | `LayoutSelectionOverlay.swift` | `LayoutSelectionOverlayView.kt` |
| Expanded candidates | Expand toggle | `ExpandedCandidateOverlay.swift` | `CandidateOverlayView.kt` |
