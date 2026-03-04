# Autocomplete

> **Type**: Feature
> **Keywords**: `Autocomplete`, `Suggestion`, `Candidate`, `AutocompleteService`
> **Related**: composing.md, sort.md, trie.md

---

## Summary

- Searches candidates based on `rawInput`
- Position 0 is always reserved for `composingText` (current composition)
- Determines input type to decide search strategy

---

## Core Flow

```
rawInput → determineInputType → LexiconService.search → sort → candidate list
```

1. Get `rawInput` from ComposingManager
2. Determine input type (Hanzi/with tone/without tone)
3. Search dictionary
4. Insert composingText at position 0
5. Return candidate list

---

## Input Type Detection

| Input | Type | Description |
|-------|------|-------------|
| `我` | `Hanzi` | Contains Hanzi (漢字) |
| `guá` | `RomanWithTone` | Contains tone marks |
| `gua2` | `RomanWithTone` | Contains numeric tone (2,3,5,6,7,8,9) |
| `gua` | `RomanWithoutTone` | No tone |
| `gua1` | `RomanWithoutTone` | Tone 1 not considered as "with tone" |
| `gua4` | `RomanWithoutTone` | Tone 4 not considered as "with tone" |

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

## Platform Correspondence

| Item | iOS | Android |
|------|-----|---------|
| Service | `AutocompleteService.swift` | `TaigiAutocompleteService.kt` |
| Search | `LexiconService.swift` | `LexiconService.kt` |
| Trigger | `autocompleteText` property | `getSuggestions()` direct call |

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
