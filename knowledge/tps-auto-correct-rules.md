# TPS Auto-Correct Rules (方音符號自動校正規則)

Cross-referenced with `taigi-phonetics-reference.md` and the codebase implementations.

**Scope**: These rules apply ONLY when `inputMode == .tps` (iOS) / `keyboardLayoutType == "tps"` (Android). They do not affect POJ/TL/English layouts.

---

## Overview

TPS auto-correct handles cases where:
1. The keyboard has one physical key for multiple context-dependent symbols
2. The user types a phonetically invalid sequence that has a clear correct form
3. The display form (rawInput) should accurately reflect standard TPS notation

All rules operate on `rawInput` at keystroke time, before `toTL()` conversion.

---

## Rule 1: Palatalization (顎化自動校正)

### Phonetic basis

> When [ts], [tsʰ], [s], [dz] combine with [i] (or [ɪ]), they become palatalized [tɕ], [tɕʰ], [ɕ], [dʑ]. In TPS, the symbols ㄐ (ts+i), ㄑ (tsh+i), ㄒ (s+i), ㆢ (j+i) are used before ㄧ.
> — `taigi-phonetics-reference.md` Section 2, Notes on initials

### Rule

When the incoming character is **ㄧ** or **ㆪ**, and the last character of rawInput is a non-palatalized affricate, replace the last character with its palatalized form:

| rawInput last char | Incoming | Replace last with | Example |
|---|---|---|---|
| ㄗ (ts) | ㄧ or ㆪ | ㄐ | ㄗ + ㄧ → ㄐㄧ (tsi) |
| ㄘ (tsh) | ㄧ or ㆪ | ㄑ | ㄘ + ㆪ → ㄑㆪ (tshinn) |
| ㄙ (s) | ㄧ or ㆪ | ㄒ | ㄙ + ㄧ → ㄒㄧ (si) |
| ㆡ (j) | ㄧ or ㆪ | ㆢ | ㆡ + ㄧ → ㆢㄧ (ji) |

### Why ㆪ triggers palatalization

ㆪ is the nasalized form of ㄧ (inn = ĩ). Phonetically, the [i] component still triggers palatalization. Wikipedia confirms: 錢（ㄐㆪˊ tsînn） — palatalized ts before nasalized i.

### Non-triggers

- ㄙ + ㄚ → no change (ㄚ is not an i-vowel)
- ㄒ + ㄧ → no change (already palatalized)
- ㄍ + ㄧ → no change (ㄍ is not an affricate)

### Implementation

| Platform | Function | Location |
|---|---|---|
| iOS | `TPSConverter.palatalizationReplacement(forIncoming:lastRawChar:)` | `TPSConverter.swift` |
| Android | `TPSConverter.palatalizationReplacement(incoming, lastRawChar)` | `TPSConverter.kt` |

Called after `adjustTPSInitialKey` in the input pipeline. Uses `composingManager.replaceLastCharacter()` to modify rawInput retroactively.

---

## Rule 2: ㄇ/ㄫ Position Auto-Selection (聲母/韻尾自動判斷)

### Phonetic basis

> TPS ㆠ is the voiced counterpart of ㄅ; ㆣ is the voiced counterpart of ㄍ; ㆡ is the voiced counterpart of ㄗ
> — `taigi-phonetics-reference.md` Section 2

m and ng have different TPS symbols depending on syllable position:
- **Initial** (syllable start): ㄇ, ㄫ
- **Final/syllabic** (after vowel): ㆬ, ㆭ
- **Special**: ㄧ + ng → ㄥ (dedicated symbol for -ing rhyme)

### Rule

When the user taps ㄇ or ㄫ, check the last character of rawInput:

**At syllable start** (keep initial form):
- rawInput is empty
- Last char is a tone mark: ˋ ˪ ˊ ˇ ˫ ˙ ˆ
- Last char is a checked tone final: ㆴ ㆵ ㆻ ㆷ
- Last char is a space

**Not at syllable start** (convert to final/syllabic form):

| Key tapped | After ㄧ | Other positions |
|---|---|---|
| ㄇ | → ㆬ | → ㆬ |
| ㄫ | → ㄥ | → ㆭ |

### Examples

| Input sequence | rawInput result | TL |
|---|---|---|
| (empty) + ㄇ | ㄇ | m (initial) |
| ㄚ + ㄇ | ㄚㆬ | am |
| ㄧ + ㄫ | ㄧㄥ | ing |
| ㄚ + ㄫ | ㄚㆭ | ang |
| ˋ + ㄇ | ...ˋㄇ | m (new syllable) |

### Known limitation

Standalone syllabic m/ng at syllable start (e.g. ㆬ˫ = m̄) will display as initial form (ㄇ˫) because at input time we cannot predict whether a vowel will follow. Search is unaffected — both map to the same TL.

### Implementation

| Platform | Function | Location |
|---|---|---|
| iOS | `TPSConverter.adjustTPSInitialKey(_:afterRawInput:)` | `TPSConverter.swift` |
| Android | `TPSConverter.adjustTPSInitialKey(char, afterRawInput)` | `TPSConverter.kt` |

---

## Rule 3: ㆮ/ㆯ Nasalized Vowel Auto-Correct (鼻化韻母自動校正)

### Phonetic basis

> | TL | POJ | Examples |
> | ainn | aiⁿ | 借 |
> | iaunn | iauⁿ | 嘔 |
> — `taigi-phonetics-reference.md` Section 3.2.2

"iainn" is NOT a valid Taiwanese final. After ㄧ, the nasalized diphthong must be "iaunn" (ㆯ), not "iainn" (ㆮ).

### Rule

When the incoming character is **ㆮ** (ainn) and the last character of rawInput is **ㄧ**, replace the incoming character with **ㆯ** (aunn):

| rawInput last char | Incoming | Adjusted to | Reason |
|---|---|---|---|
| ㄧ | ㆮ (ainn) | ㆯ (aunn) | "iainn" invalid; "iaunn" is correct |
| anything else | ㆮ | ㆮ (no change) | "ainn" is valid |

### Implementation

| Platform | Function | Location |
|---|---|---|
| iOS | `TPSConverter.adjustTPSNasalizedVowelKey(_:afterRawInput:)` | `TPSConverter.swift` |
| Android | `TPSConverter.adjustTPSNasalizedVowelKey(char, afterRawInput)` | `TPSConverter.kt` |

---

## Input Pipeline Execution Order

All auto-correct rules execute in sequence in the character input handler, BEFORE the character is appended to rawInput:

```
User taps key
  │
  ├─ 1. CaseTransformer (caps handling)
  │
  ├─ 2. adjustTPSInitialKey()        ← Rule 2: ㄇ/ㄫ position
  │     Modifies: incoming character
  │
  ├─ 3. adjustTPSNasalizedVowelKey() ← Rule 3: ㆮ/ㆯ
  │     Modifies: incoming character
  │
  ├─ 4. palatalizationReplacement()  ← Rule 1: palatalization
  │     Modifies: last character of rawInput (retroactive)
  │
  └─ 5. appendCharacter(finalChar)   ← incoming char added to rawInput
```

**Key difference**: Rules 2 and 3 modify the **incoming** character before it enters rawInput. Rule 1 modifies the **previous** character already in rawInput (look-back correction).

---

## toTL() Fallback Handling

The `toTL()` converter has built-in compensation for invalid TPS sequences that were NOT auto-corrected (e.g., user typed via popup or pasted text):

| Invalid sequence | toTL() behavior | Correct auto-corrected form |
|---|---|---|
| ㄙㄧ (non-palatalized + ㄧ) | Insert space: `"s i"` | ㄒㄧ → `"si"` |
| ㄗㄧ | Insert space: `"ts i"` | ㄐㄧ → `"tsi"` |
| ㄘㄧ | Insert space: `"tsh i"` | ㄑㄧ → `"tshi"` |
| ㆡㄧ | Insert space: `"j i"` | ㆢㄧ → `"ji"` |

These fallbacks produce broken TL (two syllables instead of one), so auto-correction at input time is preferred.

---

## TL → TPS Post-Processing (convertSyllableToTPS)

These are display-only corrections applied when converting TL back to TPS for candidate display. They are NOT input auto-corrections, but they enforce the same phonetic rules:

| Step | Rule | Example |
|---|---|---|
| 5 | Standalone m/ng → syllabic form: ㄇ→ㆬ, ㄫ→ㆭ | `m7` → ㆬ˫ |
| 5b | ㄧ + ㆭ → ㄧㄥ (ing special case) | `ing5` → ㄧㄥˊ |
| 6 | Palatalized + ㄣㄣ → ㆪ (nasalized collapse) | `tsinn5` → ㄐㆪˊ |
| 7 | ㄛ → ㆦ before ㆴ/ㆵ/ㆻ (not ㆷ) | `ok4` → ㆦㆻ |

---

## Long-Press Popup Keys (User-Initiated, Not Auto-Correct)

These are manual alternatives accessible via long-press, providing access to related symbols:

| Main key | Popup | Usage |
|---|---|---|
| ㄅ | ㆴ | -p entering tone coda |
| ㄉ | ㆵ | -t entering tone coda |
| ㄍ | ㆻ | -k entering tone coda |
| ㄏ | ㆷ | -h entering tone coda |
| ㄗ | ㄐ | Palatalized (manual override) |
| ㄘ | ㄑ | Palatalized (manual override) |
| ㄙ | ㄒ | Palatalized (manual override) |
| ㆡ | ㆢ | Palatalized (manual override) |
| ㄇ | ㆬ | Syllabic m (manual override) |
| ㄋ | ㄣ | Final n |
| ㄫ | ㆭ, ㄥ | Syllabic ng / -ing variant |
| ㆰ | ㆱ | om variant |
| ㆤ | ㄝ | ee variant |
| ㆪ | ㆳ | Vertical glyph variant |
| ㆮ | ㆯ | aunn variant |

---

## Cross-Reference Summary

| Phonetic rule (from reference) | Auto-correct implementation | Status |
|---|---|---|
| Palatalization: ts/tsh/s/j + i → tɕ/tɕʰ/ɕ/dʑ | Rule 1: `palatalizationReplacement()` | Implemented |
| m position: initial vs final/syllabic | Rule 2: `adjustTPSInitialKey()` | Implemented |
| ng position: initial vs final (ㆭ) vs -ing (ㄥ) | Rule 2: `adjustTPSInitialKey()` | Implemented |
| iainn invalid → iaunn | Rule 3: `adjustTPSNasalizedVowelKey()` | Implemented |
| o → oo before -p/-t/-k (not -h) | Post-processing Step 7 (display only) | Display only |
| ㄧ + ㆭ → ㄧㄥ (ing rhyme) | Post-processing Step 5b (display only) | Display only |
| Palatalized + nn → nasalized ㆪ | Post-processing Step 6 (display only) | Display only |
