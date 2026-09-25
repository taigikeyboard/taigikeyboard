# Tone Processing

> **Type**: Feature
> **Keywords**: `Tone`, `phonetics`, `numericTone`, `toneMarks`, `normalize_tone`
> **Related**: composing.md, binary-format.md, migration-inventory.csv

---

## Summary

- Taiwanese has 8+1 tones; tones 1/4 carry no diacritic.
- Input is numeric tone (e.g. `gua2`); display converts to diacritics.
- POJ and TL romanizations supported with separate tone-mark tables.
- All tone math lives in Rust crate `engine/phonetics`. Platform side calls it via `RustEngineBridge`.

---

## Tone Reference Table

| Tone | Diacritic | Example | Description |
|------|-----------|---------|-------------|
| 1 | (none) | `gua1` | Yin-ping (陰平), shows number |
| 2 | ́ (acute) | `guá` | Yin-shang (陰上) |
| 3 | ̀ (grave) | `guà` | Yin-qu (陰去) |
| 4 | (none) | `at4` | Yin-ru (陰入), shows number |
| 5 | ̂ (circumflex) | `guâ` | Yang-ping (陽平) |
| 6 | ̌ (caron) | `guǎ` | Yang-shang (陽上, rare) |
| 7 | ̄ (macron) | `guā` | Yang-qu (陽去) |
| 8 | ̍ (vertical) | `gua̍t` | Yang-ru (陽入) |
| 9 | ̆ / ̋ | `guă` | Neutral tone (輕聲) |

---

## Core Logic

### 1. Input Display (`normalize_tone`)

| Input | rawInput | Display (TL) | Display (POJ) |
|-------|----------|--------------|----------------|
| `gua1` | `gua1` | `gua1` | `gua1` |
| `gua2` | `gua2` | `guá` | `guá` |
| `at4` | `at4` | `at4` | `at4` |
| `hoo2` | `hoo2` | `hóo` | `hó͘` |

Rust entry: `phonetics::api::normalize_tone(input, &AppConfig)`, called in-process by `composing::derived` for the preedit. No platform bridge — the `NormalizeTone` op (no production caller) was removed 2026-09-25.

### 2. Default-Tone Heuristic (`normalize_input`)

Syllables without an explicit tone digit get one inferred from the final consonant:

```
last char ∈ {p, t, k, h} → tone "4" (entering tone, 入聲)
otherwise                → tone "1" (yin-ping)
```

This runs inside `phonetics::normalization::normalize_input`, which builds the lookup-side TL numeric form.

### 3. Tone Restoration (retired)

Retired 2026-09-25: `phonetics::normalization::restore_tone`, the `RestoreTone` op and the `RustEngineBridge.restoreTone` bridges had no production caller and were removed. Backspace edits the raw composing buffer in `engine/composing`.

---

## POJ vs TL Differences

| Item | POJ | TL |
|------|-----|-----|
| Vowel oo | `o͘` (U+006F + U+0358 combining dot above right) | `oo` |
| Nasalization | `ⁿ` (U+207F) / `ᴺ` (U+1D3A) | `nn` |
| Tone 9 | `̆` breve (U+0306) | `̋` double acute (U+030B) |
| Initial ts | `ch` | `ts` |
| Initial tsh | `chh` | `tsh` |
| Final ua | `oa` | `ua` |
| Final ue | `oe` | `ue` |
| Final ing | `eng` | `ing` |
| Final ik | `ek` | `ik` |

POJ ↔ TL display conversion: `phonetics::api::poj_display_to_tl_display` / `tl_display_to_poj_display` (rebuilds via `strip_tone_mark` + `to_poj` / `to_tl`).

---

## Ownership

| Concern | Location |
|---------|----------|
| Tone mark tables (POJ + TL combining-mark lookup) | Rust `phonetics::tables`, `phonetics::case_tables` |
| Numeric-tone → tone-mark | Rust `phonetics::api::to_tone_marks` |
| Tone-mark → numeric | Rust `phonetics::api::to_tone_number` |
| POJ doubletap preprocess (`oo`→`o͘`, `nn`→`ⁿ`) | Rust `phonetics::api::normalize_tone` (internally calls crate-private `preprocess_for_normalize_tone`) |
| Nasal-marker case adjust (`ⁿ` ↔ `ᴺ`) | Rust `phonetics::case_transform::adjust_nasal_marker_case` |
| Lookup-side Unicode prep (NFD + nasal → `nn`, dot → `o`) | Rust `phonetics::normalization::taigi_unicode_base_form` |
| Tone-letter case mapping (POJ/TL aware upper/lower) | Rust `phonetics::case_transform` |
| Bridge surface (iOS) | `Engine/RustEngineBridge+Phonetics.swift` + `RustEngineBridge+CaseTransform.swift` |
| Bridge surface (Android) | `engine/RustEngineBridge.kt` + `CaseTransformBridge.kt` |
| Flick tone UI mapping (left=2, top=3, right=5, bottom=7, long-press=8) | iOS `FlickDirection` (UI only; tone math via Rust) |

For per-pub-item descriptions in Taiwanese Mandarin, see `migration-inventory.csv` (filter `area=phonetics`).

---

## Test Cases

| Input | POJ Display | TL Display |
|-------|-------------|------------|
| `gua1` | `gua1` | `gua1` |
| `gua2` | `guá` | `guá` |
| `hoo2` | `hó͘` | `hóo` |
| `phiann` | `phiaⁿ` | `phiann` |

Rust crate-side tests: `engine/phonetics/tests/normalize_tone_nasal_case.rs`, `case_transform_golden.rs`, `roundtrip.rs`, `op_coverage.rs`.
