# Tone Processing

> **Type**: Feature
> **Keywords**: `Tone`, `ToneConverter`, `TaigiPhonetics`, `ToneRestoration`
> **Related**: composing.md

---

## Summary

- Taiwanese has 8 tones; tones 1/4 have no diacritics
- Input numeric tones, display converted to diacritics (except 1/4)
- Supports POJ and TL romanization systems

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

### 1. Input Display (ToneConverter)

| Input | rawInput | composingText |
|-------|----------|---------------|
| `gua1` | `gua1` | `gua1` |
| `gua2` | `gua2` | `guá` |
| `at4` | `at4` | `at4` |

### 2. Dictionary Generation (to_numeric_tone)

```python
# Syllables without tone diacritics get tone based on final
if syllable[-1] in "ptkh":
    syllable += "4"  # Entering tone (入聲)
else:
    syllable += "1"  # Yin-ping
```

### 3. Tone Restoration (ToneRestoration)

Restores diacritics to base characters when deleting:
- `guá` → `gua`
- `hó͘` → `ho͘` (POJ)

---

## POJ vs TL Differences

| Item | POJ | TL |
|------|-----|-----|
| Vowel oo | `o͘` (U+006F+U+0358) | `oo` |
| Nasalization | `ⁿ` (U+207F) | `nn` |
| Tone 9 | `̆` breve (U+0306) | `̋` double acute (U+030B) |
| Initial ts | `ch` | `ts` |
| Initial tsh | `chh` | `tsh` |
| Final ua | `oa` | `ua` |
| Final ue | `oe` | `ue` |
| Final ing | `eng` | `ing` |
| Final ik | `ek` | `ik` |

---

## Platform Correspondence

| Item | iOS | Android |
|------|-----|---------|
| Phonetics engine | `TaigiPhonetics.swift` | `TaigiPhonetics.kt` |
| Converter | `ToneConverter.swift` | `ToneConverter.kt` |
| Restoration | `ToneRestoration.swift` | `ToneRestoration.kt` |
| Utilities | `ToneUtilities.swift` | `ToneConverterModels.kt` + `ToneCharacterUtils.kt` |
| POJ↔TL conversion | `RomanizationConverter.swift` | (in `TaigiPhonetics.kt`) |

---

## Test Cases

| Input | POJ Display | TL Display |
|-------|-------------|------------|
| `gua1` | `gua1` | `gua1` |
| `gua2` | `guá` | `guá` |
| `hoo2` | `hó͘` | `hóo` |
| `phiann` | `phiaⁿ` | `phiann` |
