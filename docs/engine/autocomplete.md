# Autocomplete

> **Type**: Feature
> **Keywords**: `Autocomplete`, `Suggestion`, `Candidate`, `classifyInput`
> **Related**: composing.md, sort.md, binary-format.md, [continuous-candidate-display.md](continuous-candidate-display.md), [continuous-input-ranking.md](continuous-input-ranking.md)

> **⚠️ SUPERSEDED (v3.5.8 Item 13)**: this document describes the legacy
> platform lexicon-backed autocomplete (`classifyInput → lexicon::search →
> processCandidates`, slot-0 composing cell). That platform path was
> **retired** in v3.5.8 Item 13 — the Continuous-input engine is now the
> single candidate source (no platform lexicon fallback, no slot-0 cell).
> The authoritative live flow is [`continuous-candidate-display.md`](continuous-candidate-display.md)
> §15 + [`continuous-input-ranking.md`](continuous-input-ranking.md) §10.
> The engine RPCs below (`classify_input`, `lexicon::search`) still exist
> as engine API surface; they are no longer on the keyboard candidate
> path. Retained for historical/Tab3 context.

---

## Summary

- Searches candidates based on `rawInput`
- Position 0 is always reserved for `composingText` (current composition)
- Determines input type to decide search strategy

---

## Core Flow

```
rawInput → RustEngineBridge.classifyInput → RustEngineBridge.search → RustEngineBridge.processCandidates → candidate list (with composingText at index 0)
```

1. Get `rawInput` from `ComposingManager`.
2. Classify via `lexicon::classify_input` → `(input_type, search_key)`.
3. Search via `lexicon::search` (fst prefix lookup → DictionaryReader rows).
4. Rank via `ranking::process_candidates` (dedup → score → sort).
5. Platform layer prepends `composingText` at index 0 and applies suggestion case-transform (`engine/phonetics::transform_suggestion`).

---

## Input Type Detection

| Input | Type | Description |
|-------|------|-------------|
| `我` | `Hanzi` | Contains Hanzi (漢字) |
| `guá` | `RomanWithTone` | Contains tone marks |
| `gua2` | `RomanWithTone` | Contains numeric tone (2,3,5,6,7,8,9) |
| `gua` | `RomanNoTone` | No tone |
| `gua1` | `RomanNoTone` | Tone 1 not considered as "with tone" |
| `gua4` | `RomanNoTone` | Tone 4 not considered as "with tone" |

### containsNumericTone Rules

- Excludes 0, 1, 4
- Reason: Tones 1/4 have no diacritics in Taiwanese, treated as toneless search

---

## Candidate Structure

```
Index 0: composingText (e.g., guá) ← fixed position
Index 1: candidate 1 (e.g., 我)
Index 2: candidate 2 (e.g., 瓜)
...
```

### iOS additionalInfo Common Keys

| Key | Description |
|-----|-------------|
| `isComposingText` | Marks as composing candidate |
| `isNextWord` | Marks as NextWord prediction |
| `displayText` | Used for frequency recording |

---

## Ownership

| Item | Location |
|------|----------|
| Input classification (`Hanzi` / `RomanWithTone` / `RomanNoTone` + search key) | Rust `engine/lexicon::classification::classify_input` |
| Lexicon search | Rust `engine/lexicon::search` (fst lookup + DictionaryReader) |
| Candidate dedup / score / sort | Rust `engine/ranking::process_candidates` |
| Per-suggestion case transform | Rust `engine/phonetics::case_transform::transform_suggestion` |
| Platform service (iOS) | `Autocomplete/Services/TaigiAutocompleteService.swift` (engine-only after Item 13; `AutocompleteInputClassifier.swift` deleted) |
| Platform service (Android) | `ime/text/composing/TaigiAutocompleteService.kt` (engine-only after Item 13; `AutocompleteInputClassifier.kt` deleted) |
| Trigger (iOS) | `autocompleteText` KeyboardKit property |
| Trigger (Android) | `SmartbarManager.getSuggestions()` direct call |

---

## Notes

### iOS autocompleteText Must Use rawInput

- KeyboardKit triggers autocomplete based on `autocompleteText` changes
- Problem: When inputting `soo1`, composingText doesn't change (tone 1 has no diacritic)
- Solution: Return `rawInput` instead of `composingText`

---

## Test Cases

| rawInput | composingText | Candidates |
|----------|---------------|------------|
| `gua2` | `guá` | [guá, 我, 瓜, ...] |
| `soo1` | `soo1` | [soo1, 所, 鎖, ...] |
| `soo` | `soo` | [soo, 所, 鎖, 蘇, ...] |
