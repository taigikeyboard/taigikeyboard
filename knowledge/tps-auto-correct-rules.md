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

## Rule 2: Nasal/Stop Position Auto-Selection (聲母/韻尾自動判斷)

### Phonetic basis

Nasals (m, n, ng) and stops (p, t, k, h) have different TPS symbols depending on syllable position:

**Nasals:**
- **Initial** (syllable start): ㄇ, ㄋ, ㄫ
- **Final/syllabic** (after vowel): ㆬ, ㄣ, ㆭ
- **Special**: ㄧ + ng → ㄥ (dedicated symbol for -ing rhyme)

**Stops (entering tone codas):**
- **Initial** (syllable start): ㄅ, ㄉ, ㄍ, ㄏ
- **Coda** (after vowel): ㆴ (-p, tone 4), ㆵ (-t, tone 4), ㆻ (-k, tone 4), ㆷ (-h, tone 4)
- Tone 8: append ˙ after coda (e.g., ㆴ˙ = -p tone 8)

### Rule

When the user taps ㄇ, ㄋ, ㄫ, ㄅ, ㄉ, ㄍ, or ㄏ, check the last character of rawInput:

**At syllable start** (keep initial form):
- rawInput is empty
- Last char is a tone mark: ˋ ˪ ˊ ˇ ˫ ˙ ˆ
- Last char is a checked tone final: ㆴ ㆵ ㆻ ㆷ
- Last char is a space
- Last char is a syllabic nasal or precomposed nasal-coda compound final: ㆬ ㄣ ㆭ ㄥ ㆰ ㄢ ㄤ ㆱ ㆲ
- Last char is a nasalized vowel (ainn/aunn/ann/enn/inn/onn/unn → ㆮ ㆯ ㆩ ㆥ ㆪ ㆧ ㆫ)

Pure-vowel finals (ㄚ ㄧ ㄨ ㄛ ㄜ ㄞ ㄠ etc.) are INTENTIONALLY excluded from the boundary set so single-syllable entering-tone input still works (ㄍㄚ + ㄉ → ㄍㄚㆵ for `kat`).

Nasalized vowel + ㄏ also forms the legal nasalized checked final `-nnh` (annh, ennh, innh, iannh per `taigi-phonetics-reference.md` §3.2.5; ainnh, aunnh per §3.2.6 dialectal), but local look-back cannot distinguish that from cross-syllable `<nasalized-vowel> + ㄏ-initial` patterns like `ㄏㄨㆩㄏㄧ` (歡喜 = huann-hi). Continuous-input correctness wins: dictionary/canonical keys still store `-nnh` with final `ㆷ` and surface via dictionary lookup, but TPS auto-correct **deliberately does not synthesize** `ㆷ` from `ㄏ` after a nasalized vowel — `ㄏ` always stays as initial in that position.

**Not at syllable start** (convert to final form):

| Key tapped | After ㄧ | Other positions |
|---|---|---|
| ㄇ | → ㆬ | → ㆬ |
| ㄋ | → ㄣ | → ㄣ |
| ㄫ | → ㄥ | → ㆭ |
| ㄅ | → ㆴ | → ㆴ |
| ㄉ | → ㆵ | → ㆵ |
| ㄍ | → ㆻ | → ㆻ |
| ㄏ | → ㆷ | → ㆷ |

**Phonotactic gate (no-space continuous input).** The "convert to final form" above
applies only when the resulting syllable is phonotactically valid. A dual-form key
whose final/coda would build a NON-syllable (e.g. `ㄍㄠ`+`ㄉ`=`kaut`, `ㄍㄨ`+`ㄇ`=`kum`
— `aut`/`um` are not finals) stays an INITIAL so it begins the next syllable, letting a
first-tone (no tone mark) word segment without a space (交代, 龜毛). Validity is decided
by `is_valid_syllable(tps_to_tl(pending_open_syllable + final))` against `TL_FINALS`.
Covers stops AND nasals. See `docs/architecture/behavioral-invariants.md` §32
(`INVARIANT_TPS_STOPCODA_PHONOTACTIC_GATE`) + §33 (`INVARIANT_TPS_NASALCODA_PHONOTACTIC_GATE`).

### Examples

| Input sequence | rawInput result | TL |
|---|---|---|
| (empty) + ㄇ | ㄇ | m (initial) |
| (empty) + ㄋ | ㄋ | n (initial) |
| (empty) + ㄅ | ㄅ | p (initial) |
| ㄚ + ㄇ | ㄚㆬ | am |
| ㄚ + ㄋ | ㄚㄣ | an |
| ㄒㄧ + ㄋ | ㄒㄧㄣ | sin |
| ㄧ + ㄫ | ㄧㄥ | ing |
| ㄚ + ㄫ | ㄚㆭ | ang |
| ㄍㄚ + ㄅ | ㄍㄚㆴ | kap4 |
| ㄍㄚ + ㄅ + ˙ | ㄍㄚㆴ˙ | kap8 |
| ㄍㄚ + ㄉ | ㄍㄚㆵ | kat4 |
| ㄍㄚ + ㄍ | ㄍㄚㆻ | kak4 |
| ㄍㄚ + ㄏ | ㄍㄚㆷ | kah4 |
| ˋ + ㄇ | ...ˋㄇ | m (new syllable) |
| ˋ + ㄅ | ...ˋㄅ | p (new syllable) |
| ㆴ + ㄅ | ...ㆴㄅ | p (new syllable — ㆴ is in syllableBoundaryChars) |

### Why auto-correction is necessary

Initial forms (ㄅ/ㄉ/ㄍ/ㄏ/ㄋ) are in the consonant table; final forms (ㆴ/ㆵ/ㆻ/ㆷ/ㄣ) are in the tone/vowel tables. Without auto-correction, `toTL()` treats the initial form as a new syllable and inserts a space:
- ㄍㄚㄅ → `"ka p"` (broken) instead of ㄍㄚㆴ → `"kap4"` (correct)
- ㄒㄧㄋ → `"si n"` (broken) instead of ㄒㄧㄣ → `"sin"` (correct)

### Implementation

Rust shared-core single source: `engine/phonetics/src/tps_adjust.rs::adjust_initial_key`. Platform-side mirrors (iOS `TPSConverter.swift`, Android `TPSConverter.kt`) were deleted in the Path G migration; both iOS and Android call into the Rust engine via `Method::TpsInputAdjust`.

---

## Rule 2b: Syllabic Nasal Tone-Triggered Correction (獨立鼻音聲調校正)

### Phonetic basis

Standalone syllabic m and ng are valid Taiwanese syllables (e.g. m̄ = 姆, n̂g = 黃). In TPS, these use the syllabic forms ㆬ and ㆭ (from the vowel table), not the initial forms ㄇ and ㄫ (from the consonant table).

When the user types ㄇ at syllable start, `adjustTPSInitialKey` keeps it as ㄇ because a vowel might follow. But when a tone mark arrives next, we know no vowel followed — the user wants syllabic m/ng.

### Rule

When the incoming character is a **tone mark** (ˋ ˪ ˊ ˇ ˫ ˙ ˆ), and the last character of rawInput is an unconverted initial nasal, retroactively replace it with the syllabic form:

| rawInput last char | Incoming | Replace last with | Example |
|---|---|---|---|
| ㄇ (initial m) | any tone mark | ㆬ (syllabic m) | ㄇ + ˫ → ㆬ˫ (m7) |
| ㄫ (initial ng) | any tone mark | ㆭ (syllabic ng) | ㄫ + ˊ → ㆭˊ (ng5) |

### Why auto-correction is necessary

Without correction, `toTL("ㄇ˫")` produces `"m 7"` (with space — consonant-only + tone is invalid), which does not match any dictionary entry. With correction, `toTL("ㆬ˫")` produces `"m7"` (valid syllabic m).

### Why this is safe

If ㄇ were after a vowel, `adjustTPSInitialKey` (Rule 2) would have already converted it to ㆬ. So if rawInput still ends with ㄇ, it must be at syllable start. A tone mark after syllable-start ㄇ unambiguously means syllabic m. Same logic for ㄫ.

### Implementation

| Platform | Function | Location |
|---|---|---|
| iOS | `TPSConverter.syllabicNasalReplacement(forIncoming:lastRawChar:)` | `TPSConverter.swift` |
| Android | `TPSConverter.syllabicNasalReplacement(incoming, lastRawChar)` | `TPSConverter.kt` |

Called after `adjustTPSNasalizedVowelKey` and before `palatalizationReplacement` in the input pipeline. Uses `composingManager.replaceLastCharacter()` to modify rawInput retroactively (same pattern as Rule 1).

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
  ├─ 3b. syllabicNasalReplacement() ← Rule 2b: ㄇ/ㄫ + tone
  │     Modifies: last character of rawInput (retroactive)
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
| n position: initial vs final (ㄣ) | Rule 2: `adjustTPSInitialKey()` | Implemented |
| ng position: initial vs final (ㆭ) vs -ing (ㄥ) | Rule 2: `adjustTPSInitialKey()` | Implemented |
| p position: initial (ㄅ) vs coda (ㆴ) | Rule 2: `adjustTPSInitialKey()` | Implemented |
| t position: initial (ㄉ) vs coda (ㆵ) | Rule 2: `adjustTPSInitialKey()` | Implemented |
| k position: initial (ㄍ) vs coda (ㆻ) | Rule 2: `adjustTPSInitialKey()` | Implemented |
| h position: initial (ㄏ) vs coda (ㆷ) | Rule 2: `adjustTPSInitialKey()` | Implemented |
| Standalone syllabic m + tone → ㆬ | Rule 2b: `syllabicNasalReplacement()` | Implemented |
| Standalone syllabic ng + tone → ㆭ | Rule 2b: `syllabicNasalReplacement()` | Implemented |
| iainn invalid → iaunn | Rule 3: `adjustTPSNasalizedVowelKey()` | Implemented |
| o → oo before -p/-t/-k (not -h) | Post-processing Step 7 (display only) | Display only |
| ㄧ + ㆭ → ㄧㄥ (ing rhyme) | Post-processing Step 5b (display only) | Display only |
| Palatalized + nn → nasalized ㆪ | Post-processing Step 6 (display only) | Display only |
