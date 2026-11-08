# TPS Auto-Correct Rules (方音符號自動校正規則)

Cross-referenced with `taigi-phonetics-reference.md` and the codebase implementations.

**Scope**: TPS input only, on every platform. iOS / Android call `RustEngineBridge.tpsInputAdjust` only when the TPS layout is active (`inputMode == .tps` / `keyboardLayoutType == "tps"`); the desktops reach the same code through `Method::TpsKey` (`engine/composing/src/transition.rs` `tps_key` → `phonetics::tps_input_adjust`). POJ / TL / English input is unaffected.

---

## Overview

TPS auto-correct handles cases where:
1. The keyboard has one physical key for multiple context-dependent symbols
2. The user types a phonetically invalid sequence that has a clear correct form
3. The display form (rawInput) should accurately reflect standard TPS notation

All rules operate on `rawInput` at keystroke time, before the character joins the raw input.

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

`engine/phonetics/src/tps_adjust.rs::palatalization_replacement`, called after `adjust_initial_key` inside `tps_adjust::adjust`; the result comes back as `replace_last`, which the platform applies to rawInput retroactively.

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

Examples: [§ Manual test checklist](#manual-test-checklist), Rule 2.

### Why auto-correction is necessary

Initial forms (ㄅ/ㄉ/ㄍ/ㄏ/ㄋ) are in the consonant table; final forms (ㆴ/ㆵ/ㆻ/ㆷ/ㄣ) are in the tone/vowel tables. Without auto-correction, `phonetics::tps::from_zhuyin` cannot read an initial form in coda position and leaves it as residue:
- ㄍㄚㄅ → `"kaㄅ"` (broken) instead of ㄍㄚㆴ → `"kap4"` (correct)
- ㄒㄧㄋ → `"siㄋ"` (broken) instead of ㄒㄧㄣ → `"sin"` (correct)

### Implementation

`engine/phonetics/src/tps_adjust.rs::adjust_initial_key` (coda glyph from `dual_final_form`).

---

## Rule 2b: Syllabic Nasal Tone-Triggered Correction (獨立鼻音聲調校正)

### Phonetic basis

Standalone syllabic m and ng are valid Taiwanese syllables (e.g. m̄ = 姆, n̂g = 黃). In TPS, these use the syllabic forms ㆬ and ㆭ (from the vowel table), not the initial forms ㄇ and ㄫ (from the consonant table).

When the user types ㄇ at syllable start, `adjust_initial_key` keeps it as ㄇ because a vowel might follow. But when a tone mark arrives next, we know no vowel followed — the user wants syllabic m/ng.

### Rule

When the incoming character is a **tone mark** (ˋ ˪ ˊ ˇ ˫ ˙ ˆ), and the last character of rawInput is an unconverted initial nasal, retroactively replace it with the syllabic form:

| rawInput last char | Incoming | Replace last with | Example |
|---|---|---|---|
| ㄇ (initial m) | any tone mark | ㆬ (syllabic m) | ㄇ + ˫ → ㆬ˫ (m7) |
| ㄫ (initial ng) | any tone mark | ㆭ (syllabic ng) | ㄫ + ˊ → ㆭˊ (ng5) |

### Why auto-correction is necessary

Lookup would work either way (`from_zhuyin` reads both ㄇ˫ and ㆬ˫ as `m7`), but standard TPS writes the syllabic form, so the raw input and preedit show ㆬ˫ rather than ㄇ˫.

### Why this is safe

If ㄇ were after a vowel, `adjust_initial_key` (Rule 2) would have already converted it to ㆬ. So if rawInput still ends with ㄇ, it must be at syllable start. A tone mark after syllable-start ㄇ unambiguously means syllabic m. Same logic for ㄫ.

### Implementation

`engine/phonetics/src/tps_adjust.rs::syllabic_nasal_replacement`, called after `adjust_nasalized_vowel_key` and before `palatalization_replacement` inside `tps_adjust::adjust`; the result comes back as `replace_last` (same pattern as Rule 1).

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

`engine/phonetics/src/tps_adjust.rs::adjust_nasalized_vowel_key`.

---

## Rule 4: Tone-8 Dot Over a Stop Coda (第八調點)

### Basis

The canonical TPS tone-8 form puts the combining dot above (U+0307) on the stop coda: `p8` ㆴ̇, `t8` ㆵ̇, `k8` ㆻ̇, `h8` ㆷ̇ (`engine/phonetics/src/tps.rs` `ZHUYIN_TONES`; `taigi-converter/src/tables.js` the same). The keyboards type the spacing dot ˙ (U+02D9), which renders beside the coda.

### Rule

| rawInput last char | Incoming | Adjusted to |
|---|---|---|
| ㆴ ㆵ ㆻ ㆷ | ˙ (U+02D9) | ◌̇ (U+0307) |
| anything else | ˙ | ˙ (no change — no coda for a combining mark to sit on) |

Lookups fold both dots to U+0307 (`tps::normalize_tps_lookup_scalar`), so the stored form changes no key — only what the preedit and a glyph commit show (v3.9.0 row 10b, reporter request 2026-10-26).

### Implementation

`engine/phonetics/src/tps_adjust.rs::adjust_tone_eight_dot`.

---

## Input Pipeline Execution Order

All auto-correct rules run in one engine call, `engine/phonetics/src/tps_adjust.rs::adjust` (mobile: `Method::TpsInputAdjust`; desktop: inside `Method::TpsKey`), BEFORE the character is appended to the raw input. The caller gates the call by TPS input and applies both results:

```
User taps key
  │
  ├─ 1. CaseTransformer (caps handling, platform)
  │
  └─ 2. tps_adjust::adjust(incoming, raw_input) → (adjusted, replace_last?)
        ├─ adjust_tone_nine_digit()      ← tone 9 digit
        ├─ adjust_tone_eight_dot()       ← Rule 4: tone-8 dot over a stop coda
        ├─ adjust_initial_key()          ← Rule 2: initial vs coda position
        ├─ adjust_nasalized_vowel_key()  ← Rule 3: ㆮ/ㆯ
        │     (these modify the incoming character)
        ├─ syllabic_nasal_replacement()  ← Rule 2b: ㄇ/ㄫ + tone
        └─ palatalization_replacement()  ← Rule 1: palatalization (only when 2b returns nothing)
              (these return a replacement for the LAST raw character)
```

**Key difference**: Rules 2, 3 and 4 modify the **incoming** character before it enters the raw input. Rules 1 and 2b replace the **previous** character already in the raw input (look-back correction, `replace_last`).

---

## Unadjusted Input

Text that skipped auto-correct (popup keys, paste) still converts: `from_zhuyin` reads non-palatalized ㄙㄧ / ㄗㄧ / ㄘㄧ / ㆡㄧ as `si` / `tsi` / `tshi` / `ji`. The display-side TL → TPS corrections (syllabic m/ng, ㄧㄥ, ㆪ collapse, ㄛ→ㆦ before stops) live in `phonetics::tps::to_zhuyin` — see [`../engine/tps.md` § TL → TPS Post-Processing](../engine/tps.md#tl--tps-post-processing-phoneticstpsto_zhuyin).

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
| ㆪ | ㆳ | Same symbol encoded twice (Unicode L2/18-052); every lookup folds ㆳ onto ㆪ |
| ㆮ | ㆯ | aunn variant |

---

## Manual test checklist

Run on every TPS-capable platform before a release that touches TPS input. Each case: keys typed → expected display (expected search key).

### Rule 1: palatalization

- [ ] ㄗ + ㄧ → ㄐㄧ (tsi)
- [ ] ㄘ + ㄧ → ㄑㄧ (tshi)
- [ ] ㄙ + ㄧ → ㄒㄧ (si)
- [ ] ㆡ + ㄧ → ㆢㄧ (ji)
- [ ] ㄗ + ㆪ → ㄐㆪ (tsinn)
- [ ] ㄘ + ㆪ → ㄑㆪ (tshinn)
- [ ] ㄙ + ㆪ → ㄒㆪ (sinn)
- [ ] ㆡ + ㆪ → ㆢㆪ (jinn)
- Must not trigger: ㄙ + ㄚ → ㄙㄚ (ㄚ is not an i vowel) · ㄒ + ㄧ → ㄒㄧ (already palatal) · ㄍ + ㄧ → ㄍㄧ (ㄍ is not a sibilant)

### Rule 2: initial vs coda position

The same key is an initial at a syllable start and a coda after a vowel.

- Nasal codas: (empty) + ㄇ → ㄇ (initial m) · ㄚ + ㄇ → ㄚㆬ (am) · (empty) + ㄋ → ㄋ · ㄚ + ㄋ → ㄚㄣ (an) · ㄒㄧ + ㄋ → ㄒㄧㄣ (sin) · (empty) + ㄫ → ㄫ · ㄚ + ㄫ → ㄚㆭ (ang) · ㄧ + ㄫ → ㄧㄥ (ing — ㄥ after ㄧ)
- Stop codas: (empty) + ㄅ → ㄅ · ㄍㄚ + ㄅ → ㄍㄚㆴ (kap) · (empty) + ㄉ → ㄉ · ㄍㄚ + ㄉ → ㄍㄚㆵ (kat) · (empty) + ㄍ → ㄍ · ㄍㄚ + ㄍ → ㄍㄚㆻ (kak) · (empty) + ㄏ → ㄏ · ㄍㄚ + ㄏ → ㄍㄚㆷ (kah)
- Syllable boundary keeps the initial: ˋ + ㄇ → …ˋㄇ · ˋ + ㄅ → …ˋㄅ · ㆴ + ㄅ → …ㆴㄅ · space + ㄋ → …ㄋ
- After a nasal coda a new syllable starts: ㄒㄧ + ㄇ(→ㆬ) + ㄇ → ㄒㄧㆬㄇ · ㄚ + ㄋ(→ㄣ) + ㄋ → ㄚㄣㄋ · ㄚ + ㄫ(→ㆭ) + ㄍ → ㄚㆭㄍ · ㄧ + ㄫ(→ㄥ) + ㄍ → ㄧㄥㄍ · ㄒㄧ + ㄇ(→ㆬ) + ㄅ → ㄒㄧㆬㄅ
- Toneless multi-syllable search: ㄒㄧ + ㄇ(→ㆬ) + ㄇ + ㄨ + ㄚ → "sim mua" has candidates · ㄍㄚ + ㄫ(→ㆭ) + ㄍ + ㄚ + ㄨ → "kang kau" has candidates

### Rule 2b: syllabic nasal + tone

- [ ] (empty) + ㄇ + ˫ → ㆬ˫ (m7)
- [ ] (empty) + ㄇ + ˋ → ㆬˋ (m2)
- [ ] (empty) + ㄫ + ˊ → ㆭˊ (ng5)
- [ ] (empty) + ㄫ + ˋ → ㆭˋ (ng2)
- Must not trigger: ㄚ + ㄇ(→ㆬ) + ˫ → ㄚㆬ˫ (already a coda, Rule 2) · (empty) + ㄇ + ㄚ → ㄇㄚ (ㄚ is not a tone mark)

### Rule 3: ㆮ → ㆯ

- [ ] ㄧ + ㆮ → ㄧㆯ (iaunn — Taigi has no iainn)
- [ ] ㄚ + ㆮ → ㄚㆮ (ainn is valid, unchanged)

### Real words

- [ ] 錢 tsînn: ㄗ + ㆪ + ˊ → ㄐㆪˊ → 錢 offered
- [ ] 心 sim: ㄙ + ㄧ + ㄇ → ㄒㄧㆬ → 心 offered
- [ ] 因 in: ㄧ + ㄋ → ㄧㄣ → 因 offered
- [ ] 港 káng: ㄍ + ㄚ + ㄫ + ˊ → ㄍㄚㆭˊ → 港 offered
- [ ] 永 íng: ㄧ + ㄫ + ˊ → ㄧㄥˊ → 永 offered
- [ ] 角 kak: ㄍ + ㄚ + ㄍ → ㄍㄚㆻ → 角 offered
- [ ] 答 tap: ㄉ + ㄚ + ㄅ → ㄉㄚㆴ → 答 offered
- [ ] 食 tsia̍h: ㄗ + ㄧ + ㄚ + ㄏ + ˙ → ㄐㄧㄚㆷ˙ → 食 offered
- [ ] 嘔 iáunn: ㄧ + ㆮ + ˊ → ㄧㆯˊ → 嘔 offered
