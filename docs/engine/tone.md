# Tone Processing

> **Type**: Feature
> **Keywords**: `Tone`, `phonetics`, `numericTone`, `toneMarks`, `normalize_tone`
> **Related**: composing.md, binary-format.md, migration-inventory.csv

---

## Summary

- Taiwanese has 8+1 tones; tones 1/4 carry no diacritic.
- Input is numeric tone (e.g. `gua2`); display converts to diacritics.
- POJ and TL romanizations supported with separate tone-mark tables.
- All tone math lives in Rust crate `engine/phonetics`; platforms call it through their bridges (see Ownership).

Tone categories and marks: [`../phonetics/taigi-phonetics-reference.md` §5.1](../phonetics/taigi-phonetics-reference.md#51-tone-categories-and-marks).

---

## Core Logic

### 1. Input Display (`normalize_tone`)

| Input | rawInput | Display (TL) | Display (POJ) |
|-------|----------|--------------|----------------|
| `gua1` | `gua1` | `gua1` | `gua1` |
| `gua2` | `gua2` | `guá` | `guá` |
| `at4` | `at4` | `at4` | `at4` |
| `hoo2` | `hoo2` | `hóo` | `hó͘` |
| `phiann` | `phiann` | `phiann` | `phiaⁿ` |

Rust entry: `phonetics::api::normalize_tone(input, &AppConfig)`, called in-process by `composing::derived` for the preedit. There is no platform bridge for it.

### 2. Default-Tone Heuristic (`normalize_input`)

`phonetics::normalization::normalize_input` builds the lookup-side TL numeric form. Only when the input already carries a tone mark (`has_tone_marks`, `normalization.rs`), syllables without a tone get one inferred from the final consonant:

```
last char is a stop coda (p, t, k, h) → tone "4" (entering tone, 入聲)
otherwise                             → tone "1" (yin-ping)
```

Unmarked input (e.g. `gua`) stays toneless so lookup matches every tone. A syllable ending in a digit is passed through unchanged.

---

## POJ vs TL Differences

Spelling differences (ts/ch, oa/ua, ing/eng, ik/ek …): [`../phonetics/taigi-phonetics-reference.md` §10](../phonetics/taigi-phonetics-reference.md#10-key-differences-summary-tl-vs-poj). The engine-specific code points:

| Item | POJ | TL |
|------|-----|-----|
| Vowel oo | `o͘` (U+006F + U+0358 combining dot above right) | `oo` |
| Nasalization | `ⁿ` (U+207F) / `ᴺ` (U+1D3A) | `nn` |
| Tone 9 | `̆` breve (U+0306) | `̋` double acute (U+030B) |

POJ ↔ TL display conversion: `phonetics::api::poj_display_to_tl_display` / `tl_display_to_poj_display` (rebuilds via `strip_tone_mark` + `to_poj` / `to_tl`).

---

## Ownership

| Concern | Location |
|---------|----------|
| Tone mark tables (POJ + TL combining-mark lookup) | Rust `phonetics::tables` |
| Numeric-tone → tone-mark | Rust `phonetics::api::to_tone_marks` |
| Tone-mark → numeric | Rust `phonetics::api::to_tone_number` |
| POJ doubletap preprocess (`oo`→`o͘`, `nn`→`ⁿ`) | Rust `phonetics::api::normalize_tone` (internally calls crate-private `preprocess_for_normalize_tone`) |
| Nasal-marker case adjust (`ⁿ` ↔ `ᴺ`) | Rust `phonetics::case_transform::adjust_nasal_marker_case` |
| Lookup-side Unicode prep (NFD + nasal → `nn`, dot → `o`) | Rust `phonetics::normalization::taigi_unicode_base_form` |
| Tone-letter case mapping (stdlib upper/lower, mode-independent) | Rust `phonetics::case_transform` |
| Bridge surface (iOS) | `Engine/RustEngineBridge+Phonetics.swift` + `RustEngineBridge+CaseTransform.swift` |
| Bridge surface (Android) | `engine/PhoneticsBridge.kt` + `CaseTransformBridge.kt` |
| Tone-variant long-press callouts (POJ / TL tone maps) | iOS `TaigiToneMaps` in `Callouts/TaigiCallouts+Maps.swift`, used by `TaigiCallouts+Builder.swift` (UI only; tone math via Rust) |

For per-pub-item descriptions, see `migration-inventory.csv` (filter `area=phonetics`).

Rust crate-side tests: `engine/phonetics/tests/normalize_tone_nasal_case.rs`, `case_transform_golden.rs`, `roundtrip.rs`, `op_coverage.rs`.
