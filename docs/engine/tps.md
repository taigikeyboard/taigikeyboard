# TPS Taiwanese Phonetic Symbols (方音符號)

> **Type**: Feature
> **Keywords**: `TPS`, `phonetics::tps`, `方音符號`, `Taiwanese Zhuyin`
> **Related**: tone.md, binary-format.md, ../ui/layout.md, ../phonetics/tps-auto-correct-rules.md

---

## Summary

- TPS (Taiwanese Phonetic Symbols) input method on every platform (`InputMode::Tps`).
- Input stays Bopomofo end to end; the engine looks it up directly in the `tps:` FST key family of `dictionary.fst` (no per-keystroke TPS→TL conversion — see § Tone 1/4 Ambiguous Syllables).
- QWERTY-style keyboard layout.

---

## Ownership

All TPS conversion and key-level auto-adjust live in Rust `engine/phonetics` (`tps.rs`, `tps_adjust.rs`). Platform code keeps only the keystroke dispatcher and UI.

| Concern | Location |
|---------|----------|
| TPS detection / tone-mark tables | Rust `phonetics::tps::is_zhuyin`, `is_tps_char`, `is_tps_tone_mark`; `phonetics::api::contains_tps` (Rust-internal) |
| TPS → TL (numeric tone) | Rust `phonetics::tps::from_zhuyin` (re-exported as `phonetics::tps_to_tl`) — Rust-internal, used by `tps_adjust` syllable validation, `normalization::normalize_input` and `custom_search` |
| TL → TPS (display) | Rust `phonetics::tps::to_zhuyin`, wrapped by `phonetics::api::tl_display_to_tps` / `tl_numeric_to_tps` |
| Build-time TPS keys | Rust `tps_num_from_tl`, `tps_notone_from_tl`, `tps_abbrev_from_tl`; pipeline `dictionary/common/stages/numtone.py::_derive_tps_num` |
| Key-level auto-adjust | Rust `phonetics::tps_adjust::adjust` (rules: [`tps-auto-correct-rules.md`](../phonetics/tps-auto-correct-rules.md)) |
| Candidate cell — TL → TPS | engine `CandidateMessage.tps_text` (`composing::requests::tps_cell_text`), filled under the TPS layout; iOS `CandidateCellHelper`, Android `CandidateStrip` / `CandidateOverlayContent`, desktop-core `CandidateCellContent::cell` show it |
| Input adjust (every platform) | `Method::TpsKey` → `composing::transition::tps_key` → `phonetics::tps_input_adjust`, inside the engine; mobile `ComposingManager.tpsKey` (iOS / Android) |
| iOS TPS-aware glue | `Layout/TaigiLayouts.swift` (layout def), `Settings/SharedSettings.swift` (`.tps` type), `Candidates/Views/CandidateCellHelper.swift` (candidate TPS display), `Actions/ActionHandler+KeyActions.swift` (sends `TpsKey`) |
| Android TPS-aware glue | `ime/text/keyboard/TextInputKeyHandler.handleTaigiInput()` (sends `TpsKey`), layout JSON `assets/ime/text/characters/tps.json` |

---

## Conversion Flow

```
User input → key-level adjust → shadow/lattice → tps: key → FST lookup
     ↓              ↓                  ↓              ↓            ↓
  ㄉㄧㄠˊ  → phonetics::tps_adjust → composing → tps:ㄉㄧㄠˊ → dictionary hit
```

Under `InputMode::Tps`, `phonetics::KeyFamily::for_input_mode` (`engine/phonetics/src/key_family.rs`) selects the `tps` family, and the lexicon fetch (`engine/lexicon/src/continuous/`) looks the key up in `dictionary.fst`, populated at build time by `dictionary/build/create_fst.py`. TPS is never folded into `tl:` keys at runtime.

---
## Initial Consonant Mapping

| TPS | TL | Description |
|-----|-----|-------------|
| ㄅ | p | Bilabial unaspirated voiceless stop |
| ㄆ | ph | Bilabial aspirated voiceless stop |
| ㄇ | m | Bilabial nasal |
| ㆠ | b | Bilabial voiced stop |
| ㄉ | t | Alveolar unaspirated voiceless stop |
| ㄊ | th | Alveolar aspirated voiceless stop |
| ㄋ | n | Alveolar nasal |
| ㄌ | l | Alveolar lateral |
| ㄍ | k | Velar unaspirated voiceless stop |
| ㄎ | kh | Velar aspirated voiceless stop |
| ㄫ | ng | Velar nasal |
| ㆣ | g | Velar voiced stop |
| ㄏ | h | Glottal fricative |
| ㄗ | ts | Alveolar unaspirated affricate |
| ㄘ | tsh | Alveolar aspirated affricate |
| ㄙ | s | Alveolar fricative |
| ㆡ | j | Alveolar voiced affricate |

### Compound Initial Consonants

| TPS | TL | Description |
|-----|-----|-------------|
| ㄐㄧ | tsi | Alveolar unaspirated (before i) |
| ㄑㄧ | tshi | Alveolar aspirated (before i) |
| ㄒㄧ | si | Alveolar fricative (before i) |
| ㆢㄧ | ji | Alveolar voiced (before i) |

---

## Vowel Mapping

### Simple Vowels

| TPS | TL |
|-----|-----|
| ㄚ | a |
| ㆤ | e |
| ㄧ | i |
| ㄛ | o |
| ㄨ | u |
| ㆦ | oo |

### Compound Vowels

| TPS | TL |
|-----|-----|
| ㄞ | ai |
| ㄠ | au |
| ㄢ | an |
| ㄤ | ang |
| ㆲ | ong |
| ㆰ | am |
| ㆱ | om |

### Nasal Vowels

| TPS | TL |
|-----|-----|
| ㆬ | m |
| ㄥ | ng |
| ㆭ | ng |
| ㄣ | n |

### Nasalized Vowels

| TPS | TL |
|-----|-----|
| ㆩ | ann |
| ㆪ | inn |
| ㆥ | enn |
| ㆧ | onn |
| ㆫ | unn |
| ㆮ | ainn |
| ㆯ | aunn |

---

## Tone Mapping

### Regular Tones

| TPS | TL | Description |
|-----|-----|-------------|
| (none) | 1 | High level (陰平) |
| ˋ | 2 | Rising (陰上) |
| ˪ | 3 | Low falling (陰去) |
| ˊ | 5 | Mid rising (陽平) |
| ˫ | 7 | Mid level (陽去) |

### Entering Tone Finals

| TPS | TL | Description |
|-----|-----|-------------|
| ㆴ | p + 4 | Entering tone -p (陰入) |
| ㆵ | t + 4 | Entering tone -t (陰入) |
| ㆻ | k + 4 | Entering tone -k (陰入) |
| ㆷ | h + 4 | Entering tone -h (陰入) |
| ㆴ̇ | p + 8 | Entering tone -p (陽入) |
| ㆵ̇ | t + 8 | Entering tone -t (陽入) |
| ㆻ̇ | k + 8 | Entering tone -k (陽入) |
| ㆷ̇ | h + 8 | Entering tone -h (陽入) |

---

## Keyboard Layout

iOS `tps_iPhone` (`Layout/TaigiLayouts.swift`):

```
Row 1 (Voiced/Tones): [ㆠ] [ˋ] [˪] [ㆣ] [ˊ] [ˇ] [˫] [˙] [ㆪ] [ㆩ]
Row 2 (Stops/Vowels): [ㄅ] [ㄉ] [ㄍ] [ㄏ] [ㄧ] [ㄚ] [ㄞ] [ㄤ] [ㆫ] [ㆧ]
Row 3 (Aspirated):    [ㄆ] [ㄊ] [ㄎ] [ㄗ] [ㄨ] [ㄛ] [ㄠ] [ㆲ] [ㆥ] [ㆮ]
Row 4 (Nasals/Other): [ㄇ] [ㄋ] [ㄫ] [ㄘ] [ㄜ] [ㆦ] [ㄢ] [ㆰ] [，] [⌫]
Row 5 (Function):     [123] [ㄌ] [ㆡ] [ㄙ] [ㆨ] [ㆤ] [Space] [emoji] [⏎]
```

Entering-tone codas (ㆴ/ㆵ/ㆻ/ㆷ) come from Rule 2 auto-selection or the long-press popups on ㄅ/ㄉ/ㄍ/ㄏ (full popup list: [`tps-auto-correct-rules.md` § Long-Press Popup Keys](../phonetics/tps-auto-correct-rules.md#long-press-popup-keys-user-initiated-not-auto-correct)).

**Important**: ㄏ (U+310F) and ㆷ (U+31B7) are distinct characters. ㄏ is always an initial; ㆷ is the entering-tone -h coda (BOPOMOFO FINAL LETTER H). `from_zhuyin` maps ㆷ to `h4` / `h8` via the tones table and ㄏ to initial `h` via the consonants table.

### Layout Variants

| Name | Description |
|------|-------------|
| `tps_iPhone` | No globe key |
| `tps_withGlobe` | Has globe key (iPhone SE, iPad) |

---

## Bridge API (called from mobile platforms)

```swift
// One TPS key (positional ㄇ/ㆬ, palatalization ㄗ→ㄐ, syllabic nasal, ㆮ/ㆯ
// applied inside the engine) — Input/Composing/ComposingManager.swift:
composingManager.tpsKey("ㄇ")   // rawInput "ㄅㄚ" → "ㄅㄚㆬ"
composingManager.tpsKey(" ")    // false when the separator rule refuses it
```

`ActionHandler+KeyActions.handleCharacterInput` (iOS) / `TextInputKeyHandler.handleTaigiInput()` (Android) send one `TpsKey` per keystroke under the TPS input mode; the Space key sends `" "` and commits when the engine refuses it (no effects). TPS → TL has no FFI surface.

---

## Candidate Display and Output

In TPS mode, candidates display phonetic symbols instead of romanization, and output phonetic symbols on tap.

| isHanjiFirst | Title | Subtitle | Tap output |
|--------------|-------|----------|------------|
| false (default) | Phonetic symbols | Hanzi | Phonetic symbols |
| true | Hanzi | Phonetic symbols | Hanzi |

iOS implementation: `CandidateCellHelper.swift` — `displayTitle()`, `displaySubtitle()`, `suggestionToHandle()`.

TPS candidates use the same font sizes as every other layout (iOS `CandidateTheme.primaryFontSize` / `secondaryFontSize`, Android `computeCandidateFontSizes` in `ime/text/candidates/CandidateStrip.kt`).

### Conversion Examples (`tl_display_to_tps`)

| TL | TPS |
|----|-----|
| tiau5 | ㄉㄧㄠˊ |
| su5 | ㄙㄨˊ |
| peh8 | ㄅㆤㆷ̇ |
| kau2 | ㄍㄠˋ |
| gua2-gua2 | ㆣㄨㄚˋ ㆣㄨㄚˋ |
| tsi̍t-pái | ㄐㄧㆵ̇ ㄅㄞˋ |

A romanization hyphen `-` becomes a TPS space separator. A neutral-tone `--` also becomes a
space, and the rest of its word takes the `˙` U+02D9 prefix (`api::tl_numeric_to_tps`, reference §5.5).
Display only: the fused key columns carry no marker (`dictionary/common/taigi_bridge.py`
`_tps_key_tl`).

---

## TL → TPS Post-Processing (`phonetics::tps::to_zhuyin`)

After the table lookup, `to_zhuyin` applies these display corrections:

| Step | Rule | Examples |
|------|------|----------|
| 5 | Consonant with no vowel is a syllabic nasal: ㄇ→ㆬ, ㄫ→ㆭ | `m7` → ㆬ˫, `ng5` → ㆭˊ |
| 5b | ㄧㆭ → ㄧㄥ (ㄥ is the dedicated `-ing` rhyme); ㄨㆭ and bare ㆭ unchanged | `ing5` → ㄧㄥˊ, `king5` → ㄍㄧㄥˊ, `ung7` → ㄨㆭ˫ |
| 6 | Palatalized initial + ㄣㄣ (nn) → nasalized ㆪ | `tsinn5` → ㄐㆪˊ, `tshinn5` → ㄑㆪˊ |
| 7 | ㄛ → ㆦ before ㆴ/ㆵ/ㆻ (not ㆷ) | `ok4` → ㆦㆻ, `bok4` → ㆠㆦㆻ, `oh4` → ㄛㆷ |

---

## Input Auto-Correct

Key-level auto-correct (`phonetics::tps_adjust::adjust`) is specified in [`../phonetics/tps-auto-correct-rules.md`](../phonetics/tps-auto-correct-rules.md): Rule 1 palatalization, Rule 2 nasal / stop position with its phonotactic gate, Rule 2b syllabic nasal + tone, Rule 3 ㆮ→ㆯ, plus the tone-9 digit.

`from_zhuyin` inserts a syllable separator (`-`) between adjacent syllables, using tone marks and entering-tone codas as boundaries.

---

## Test Cases (`from_zhuyin`)

| TPS Input | TL Output | Description |
|-----------|-----------|-------------|
| ㄉㄧㄠˊ | tiau5 | Initial + vowel + tone |
| ㄙㄨˊㄅㆤㆻ̇ | su5-pek8 | Multi-syllable + entering tone |
| ㄍㄠˋㄏㄧㆲˊ | kau2-hiong5 | Compound vowel |
| ㄅㄚ | pa | No tone |
| ㄍㄚㆴ˙ | kap8 | Entering tone 8 |

---

## Tone 1/4 Ambiguous Syllables

TPS tone 1 and tone 4 are **unmarked**. The build pipeline converts each row's TL form to fused Bopomofo (`dictionary/common/stages/numtone.py::_derive_tps_num`), which emits the same `tps_num` for single-syllable `tsua2` (紙) and multi-syllable `tsu1a2` (珠仔) — both render as `ㄗㄨㄚˋ` because the tone-1 `tsu` is unmarked. The `tps:ㄗㄨㄚˋ` key in `dictionary.fst` therefore points to both rowids, and typing `ㄗㄨㄚˋ` recovers both candidates.

---

## er↔or Dialect Dual-Emit

`taigi-converter/src/tables.js` collapses TL `er` and `or` to the same glyph ㄜ (U+311C). The Rust `phonetics::tps::to_zhuyin` honors the `or_maps_to_er` user setting and renders TL `or` as ㄛ (U+311B) when the toggle is OFF (the default).

So that a user typing ㄛ for an `or`-spelled entry still finds it, the build pipeline emits, for every row whose TPS key contains ㄜ, a parallel `tps:<variant>` key at the same rowid with ㄜ → ㄛ (`tps_num_var` / `tps_notone_var` / `tps_abbrev_var` columns, `dictionary/common/stages/{numtone,notone,abbrev}.py`, keyed by `dictionary/build/create_fst.py`). The substitution is always on; `or_maps_to_er` drives **rendering** only (`to_zhuyin` / `tl_display_to_tps`), not recall. A row with two or more ㄜ syllables emits only the fully-swapped variant, not partial mixes.

---

## Platform Glue

| Concern | iOS | Android |
|---------|-----|---------|
| Composing platform wrapper | `Input/Composing/ComposingManager.swift` | `ime/text/composing/ComposingManager.kt` |
| Effect → text-region binding | `Input/Composing/ComposingDelegate.swift` (`UITextDocumentProxy`) | `ime/text/composing/ComposingDelegate.kt` (`InputConnection`) |
| Keystroke dispatcher | `Actions/ActionHandler+KeyActions.swift` | `ime/text/keyboard/TextInputKeyHandler.kt` |
| Layout / candidate UI | `Layout/`, `Candidates/Views/` | `ime/text/layout/`, `ime/text/candidates/` |

---

## References

- [Tailo-TPS-Converter](https://github.com/leechunhoe/Tailo-TPS-Converter)
- [MOE Taiwanese Dictionary](https://sutian.moe.edu.tw/zh-hant/siannuntiao/)
