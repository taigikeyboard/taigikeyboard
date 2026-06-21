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

---

## English-mode Autocomplete (non-Taigi path)

> Scope: this is the **English** input-mode path, separate from the Taigi lexicon path above. It never touches the Rust engine or the Taigi dictionary. It is current — not covered by any SUPERSEDED note on the Taigi path.

Runs when `inputMode == english` (a distinct mode from TL / POJ / TPS). This is an **intentional, documented cross-platform divergence** (`EnglishAutocompleteService.kt` header): iOS keeps Apple's OS spell-checker; Android self-bundles a wordlist because some devices' system `SpellCheckerSession` returns null (candidates would otherwise always be empty — Samsung is the observed instance).

| File | Responsibility |
|------|----------------|
| iOS `Autocomplete/Services/EnglishAutocompleteService.swift` | Wraps Apple `UITextChecker` (completions + correction guesses); conforms to `KeyboardKit.AutocompleteService` |
| Android `ime/text/composing/EnglishAutocompleteService.kt` | Asset I/O + lazy fail-closed loading; delegates matching to `EnglishWordMatcher` |
| Android `ime/text/composing/EnglishWordMatcher.kt` | Pure-Kotlin matcher (Shared-Core Candidate): prefix completion (binary search) + bounded OSA correction |
| Android `assets/english_freq.txt` | Bundled wordlist — 30000 words, `word\tcount` freq-descending (SymSpell / SCOWL, MIT) |

### Cross-platform divergence

| Aspect | iOS | Android |
|---|---|---|
| Engine | Apple `UITextChecker` (OS-provided) | Self-bundled 30k wordlist + `EnglishWordMatcher` |
| Completion | `checker.completions(forPartialWordRange:)` | prefix binary search over sorted index |
| Correction | `checker.guesses(...)` — **fallback** only when completions empty | OSA edit-distance ≤ 2, length-pruned — runs **alongside** prefix hits, gated `len ≥ 3` + all `a-z` |
| Ranking | UITextChecker order | EXACT < PREFIX < CORRECTION, then distance asc, freq desc, alpha |
| Current-word split | last **space** only | last whitespace OR `.,!?;:` (apostrophe/hyphen do not split — `don't` is one token) |
| Casing | UITextChecker native | `applyCasing` mirrors input (lower / Title / ALL-CAPS) |
| Max suggestions | 3 | 3 (`MAX_SUGGESTIONS`) |
| Integration | KeyboardKit framework trigger | `CandidateUpdateCoordinator` + `SmartbarManager`, 50 ms debounce |
| Tests | none | `EnglishWordMatcherTest.kt` (JVM) |

Android candidates are wrapped as `TaigiWord(id = -100 - index, roman = text, hanzi = null)`; the `id <= -100` sentinel marks them English (distinct from NextWord `id < 0`). Commit-on-tap deletes the current word and `commitText`s the suggestion — no frequency learning. Constants: `MAX_EDIT_DISTANCE = 2`, `MIN_CORRECTION_LENGTH = 3`, `MAX_SUGGESTIONS = 3`, wordlist `TOP_N = 30000`.
