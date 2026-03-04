# Composing Manager

> **Type**: Feature
> **Keywords**: `Composing`, `rawInput`, `composingText`, `ComposingManager`
> **Related**: autocomplete.md, tone.md

---

## Summary

- Maintains "dual-state": `rawInput` (for search) + `composingText` (for display)
- Input `gua2` → rawInput=`gua2`, composingText=`guá`

---

## Core Concepts

### Dual-State Model

| State | Purpose | Example |
|-------|---------|---------|
| `rawInput` | Trie search (numeric tone) | `gua2` |
| `composingText` | UI display (diacritics) | `guá` |

### Why Dual-State?

- Trie index format: `tl:gua2`, `poj:goa2`
- Users expect to see diacritics, not numbers

---

## Core Operations

| Operation | Description |
|-----------|-------------|
| `appendCharacter` | Append character, update dual-state |
| `deleteBackward` | Delete character, handle tone restoration |
| `commitComposition` | Confirm composition, output text |
| `selectSuggestion` | Select candidate, replace composition |

### appendCharacter Flow

1. Update rawInput (preserve original ASCII)
2. Check character combinations (POJ: `oo`→`o͘`, `nn`→`ⁿ`)
3. Check tone conversion (number → diacritic)
4. Update composingText
5. Sync to input field

### deleteBackward Flow

1. Attempt tone restoration (`guá`→`gua`)
2. Success: rawInput deletes number, composingText updates
3. Failure: both delete last character
4. Special: `ⁿ` corresponds to `nn` in rawInput (2 characters)

---

## Character Combinations (POJ Only)

| Input | Output | Unicode |
|-------|--------|---------|
| `oo` | `o͘` | U+006F + U+0358 |
| `nn` | `ⁿ` | U+207F |

- TL mode: no conversion, keeps original

---

## Tone Handling

| Tone | Handling | Example |
|------|----------|---------|
| 1, 4 | Keep number | `gua1`→`gua1` |
| 2,3,5,6,7,8,9 | Convert to diacritic | `gua2`→`guá` |

---

## Platform Correspondence

| Item | iOS | Android |
|------|-----|---------|
| Composing manager | `ComposingManager.swift` | `ComposingManager.kt` |
| Tone converter | `ToneConverter.swift` | `ToneConverter.kt` |
| Phonetics engine | `TaigiPhonetics.swift` | `TaigiPhonetics.kt` |

---

## Test Cases

| Input sequence | rawInput | composingText |
|----------------|----------|---------------|
| g u a | `gua` | `gua` |
| g u a 2 | `gua2` | `guá` |
| g u a 1 | `gua1` | `gua1` |
| h o o 2 (POJ) | `hoo2` | `hó͘` |

| Before delete | After rawInput | After composingText |
|---------------|----------------|---------------------|
| `guá` / `gua2` | `gua` | `gua` |
| `Phiaⁿ` / `Phiann` | `Phia` | `Phia` |
