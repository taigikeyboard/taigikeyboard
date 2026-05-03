# TPS Taiwanese Phonetic Symbols (方音符號)

> **Type**: Feature
> **Keywords**: `TPS`, `phonetics::tps`, `方音符號`, `Taiwanese Zhuyin`
> **Related**: tone.md, binary-format.md, ../ui/layout.md

---

## Summary

- TPS (Taiwanese Phonetic Symbols) input method
- Input phonetic symbols, automatically convert to TL for Trie search
- QWERTY-style keyboard layout

---

## Ownership

Since v3.5.1 (PR #186) all TPS conversion + key-level auto-adjust lives in Rust `engine/phonetics::tps` / `tps_adjust`. Platform side calls via `RustEngineBridge`.

| Concern | Location |
|---------|----------|
| TPS detection / tone-mark tables | Rust `phonetics::tps::is_zhuyin`, `phonetics::tables` |
| TPS → TL (numeric tone) | Rust `phonetics::tps::from_zhuyin` (re-exported as `phonetics::tps_to_tl`) |
| TL → TPS (display + numeric) | Rust `phonetics::api::to_tone_marks` + TPS path inside same crate |
| Key-level auto-adjust (positional ㄇ/ㆬ + ㄫ/ㆭ/ㄥ, palatalization ㄗ→ㄐ, syllabic nasal, ㆮ/ㆯ) | Rust `phonetics::tps_adjust` |
| Bridge — detection | `RustEngineBridge.containsTPS(_)` / `isTPSToneMark(_)` |
| Bridge — TPS↔TL | `RustEngineBridge.tpsToTL(_)` / `tlNumericToTPS(_)` / `tlDisplayToTPS(_)` |
| Bridge — input adjust | `RustEngineBridge.tpsInputAdjust(incoming:rawInput:)` returning `(adjusted, replaceLast?)` |
| iOS TPS-aware glue | `Layout/TaigiLayouts.swift` (layout def), `Settings/SharedSettings.swift` (`.tps` type), `Autocomplete/Views/CandidateCellHelper.swift` (candidate TPS display), `Input/CharacterInputPipeline.swift` (calls bridge) |
| Android TPS-aware glue | `ime/text/CharacterInputPipeline.kt`, `ime/text/TextInputManager.handleTaigiInput()`, layout JSON under `ime/text/characters/tps*.json` |

---

## Conversion Flow

```
User input → TPS detection → TPSToTL.convert() → InputNormalizer → Trie search
     ↓              ↓                ↓                ↓              ↓
  ㄉㄧㄠˊ    →   Is TPS    →      tiau5       →     tiau5    →  Found word
```

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

```
Row 1 (Voiced/Tones): [ㆠ] [ˋ] [˪] [ㆣ] [ˊ] [ˇ] [˫] [˙] [ㆪ] [ㆩ]
Row 2 (Stops/Vowels): [ㄅ] [ㄉ] [ㄍ] [ㄏ] [ㄧ] [ㄚ] [ㄞ] [ㄤ] [ㆫ] [ㆧ]
Row 3 (Aspirated):    [ㄆ] [ㄊ] [ㄎ] [ㄗ] [ㄨ] [ㄛ] [ㄠ] [ㆲ] [ㆥ] [ㆮ]
Row 4 (Nasals/Other): [ㄇ] [ㄋ] [ㄫ] [ㄘ] [ㄜ] [ㆦ] [ㄢ] [ㆰ] [，] [⌫]
Row 5 (Function):     [123] [ㄌ] [ㆡ] [ㄙ] [ㆨ] [ㆤ] [Space] [⏎]
```

Entering tone codas (ㆴ/ㆵ/ㆻ/ㆷ) are accessed via **long-press popups**:

| Main key | Popup |
|----------|-------|
| ㄅ | ㆴ (-p coda) |
| ㄉ | ㆵ (-t coda) |
| ㄍ | ㆻ (-k coda) |
| ㄏ | ㆷ (-h coda) |

**Important**: ㄏ (U+310F) and ㆷ (U+31B7) are distinct Unicode characters. ㄏ is always an initial consonant; ㆷ is the entering tone -h coda (BOPOMOFO FINAL LETTER H). The `toTL()` converter treats them differently — ㆷ maps to "h4"/"h8" via the tones table, while ㄏ maps to initial "h" via the consonants table.

### Layout Variants

| Suffix | Description |
|--------|-------------|
| `tps_iPhone` | No globe key |
| `tps_withGlobe` | Has globe key (iPhone SE, iPad) |

---

## Bridge API (called from platform)

```swift
// iOS — Engine/RustEngineBridge.swift
RustEngineBridge.containsTPS("ㄉㄧㄠˊ")             // true
RustEngineBridge.isTPSToneMark("ˋ")                // true
RustEngineBridge.tpsToTL("ㄉㄧㄠˊ")                 // "tiau5"
RustEngineBridge.tlNumericToTPS("tiau5", orMapsToER: false)  // "ㄉㄧㄠˊ"
RustEngineBridge.tlDisplayToTPS("guá",  orMapsToER: false)   // "ㄍㄨㄚˋ"

// Key-level adjust (positional ㄇ/ㆬ, palatalization ㄗ→ㄐ, syllabic nasal, ㆮ/ㆯ):
let r = RustEngineBridge.tpsInputAdjust(incoming: "ㄇ", rawInput: "ㄅㄚ")
// r.adjusted = "ㆬ", r.replaceLast = nil

// Lookup-side normalization (any input form → TL numeric)
RustEngineBridge.normalizeInput("ㄉㄧㄠˊ")          // "tiau5"
```

Internally the iOS / Android `CharacterInputPipeline` calls `RustEngineBridge.tpsInputAdjust` once per keystroke from `ActionHandler+KeyActions.handleCharacterInput` (iOS) / `TextInputManager.handleTaigiInput()` (Android) — no platform-side phonetic logic remains.

---

## Candidate Display and Output

In TPS mode, candidates display phonetic symbols instead of romanization, and output phonetic symbols on tap.

### Display Logic

| isTranslateSwapped | Title | Subtitle |
|-------------------|-------|----------|
| false (default) | Phonetic symbols | Hanzi |
| true | Hanzi | Phonetic symbols |

### Output Logic

| isTranslateSwapped | Tap Candidate Output |
|-------------------|---------------------|
| false (default) | Phonetic symbols |
| true | Hanzi |

### Implementation Location

`CandidateCellHelper.swift`:
- `displayTitle()` - Calculate title
- `displaySubtitle()` - Calculate subtitle
- `suggestionToHandle()` - Calculate tap output

### Conversion Examples

| TL (original) | TPS (display/output) |
|---------------|---------------------|
| tiau5 | ㄉㄧㄠˊ |
| su5 | ㄙㄨˊ |
| peh8 | ㄅㆤㆷ̇ |
| kau2 | ㄍㄠˋ |
| gua2-gua2 | ㄍㄨㄚˋ ㄍㄨㄚˋ |

### Hyphen Handling

Romanization hyphen `-` converts to TPS space separator:

```
gua2-gua2  →  ㄍㄨㄚˋ ㄍㄨㄚˋ
tsi̍t-pái  →  ㄐㄧㆵ̇ ㄅㄞˋ
```

---

## TL → TPS Conversion Post-Processing

`convertSyllableToTPS()` applies several post-processing steps after initial table lookup:

### Step 5: Standalone m/ng → Syllabic Form

When consonant is matched but no vowel follows, the consonant is actually a syllabic nasal:

| Consonant | No vowel | Result |
|-----------|----------|--------|
| ㄇ (m) | vowel empty | → ㆬ (syllabic m) |
| ㄫ (ng) | vowel empty | → ㆭ (syllabic ng) |

Examples: `m7` → ㆬ˫, `ng5` → ㆭˊ

### Step 5b: ing Special Case — ㄧㆭ → ㄧㄥ

The `tlToTPSVowels` table maps `"ng"` → `"ㆭ"` (final ng). But when preceded by ㄧ (i), the combination ㄧㆭ should display as ㄧㄥ, because ㄥ is the dedicated symbol for the `-ing` rhyme.

| Vowel result | Correction | Example |
|-------------|------------|---------|
| ㄧㆭ | → ㄧㄥ | `ing5` → ㄧㄥˊ, `king5` → ㄍㄧㄥˊ |
| ㄨㆭ | (no change) | `ung7` → ㄨㆭ˫ |
| ㆭ alone | (no change) | `ng5` → ㆭˊ |

### Step 6: Palatalized + ㄣㄣ → Nasalized ㆪ

When a palatalized initial (ending in ㄧ) is followed by ㄣㄣ (nn), it collapses to the nasalized vowel ㆪ (inn):

`tsinn5` → ㄐㆪˊ, `tshinn5` → ㄑㆪˊ

### Step 7: o → oo Before Stop Tone

Before checked tone finals ㆴ (p), ㆵ (t), ㆻ (k), the vowel ㄛ (o) becomes ㆦ (oo):

`ok4` → ㆦㆻ, `bok4` → ㆠㆦㆻ

This does not apply to ㆷ (h): `oh4` → ㄛㆷ

---

## ㄇ/ㆬ and ㄫ/ㆭ/ㄥ Auto-Selection

In TPS, the same sound has different symbols depending on syllable position:

| Sound | Initial (syllable start) | Final / Syllabic (other) |
|-------|-------------------------|--------------------------|
| m | ㄇ | ㆬ |
| ng | ㄫ | ㆭ (general) / ㄥ (after ㄧ only) |

### Auto-Selection Rule

**Scope**: TPS layout only. Gated by `inputMode == .tps` (iOS) / `keyboardLayoutType == "tps"` (Android).

When the user taps ㄇ or ㄫ, `adjustTPSInitialKey()` checks the last character of `rawInput`:

**At syllable start** (any of the following) → keep initial form (ㄇ / ㄫ):
- Empty buffer (not composing)
- Last char is a tone mark: ˋ ˪ ˊ ˇ ˫ ˙ ˆ
- Last char is a checked tone final: ㆴ ㆵ ㆻ ㆷ
- Last char is a space

**Not at syllable start** → convert to final/syllabic form:

| Key tapped | After ㄧ | Other position |
|------------|---------|----------------|
| ㄇ | → ㆬ | → ㆬ |
| ㄫ | → ㄥ | → ㆭ |

### Implementation

| Platform | Helper | Call site |
|----------|--------|-----------|
| iOS | `RustEngineBridge.tpsInputAdjust` (Rust `phonetics::tps_adjust::adjust_initial_key` under the hood) | `ActionHandler+KeyActions.handleCharacterInput` |
| Android | `RustEngineBridge.tpsInputAdjust` (Kotlin shim with same Rust backend) | `TextInputManager.handleTaigiInput()` |

### Syllabic Nasal Tone-Triggered Correction (v3.4.7)

When a tone mark follows bare ㄇ or ㄫ at syllable start, the consonant is retroactively corrected to its syllabic form:

| Input sequence | Auto-corrected to | TL equivalent |
|----------------|-------------------|---------------|
| ㄇ + ˫ | ㆬ + ˫ | m7 |
| ㄫ + ˊ | ㆭ + ˊ | ng5 |

**Implementation**: Rust `phonetics::tps_adjust::syllabic_nasal_replacement` (called via `RustEngineBridge.tpsInputAdjust`).

---

## Font Size Adjustment

Phonetic symbols are visually larger than romanization, TPS mode reduces font size by 15%.

### Scale Ratio

| Type | Scale Ratio |
|------|-------------|
| Regular candidates | 0.85x |
| Long-word candidates | 0.85x |

### Implementation Location

`CandidateViewModels.swift`:
- `tpsScale = 0.85` - TPS scale ratio
- `tpsPrimaryFontSize` - TPS title font
- `tpsSecondaryFontSize` - TPS subtitle font
- `tpsLongCellPrimaryFontSize` - TPS long-word title font
- `tpsLongCellSecondaryFontSize` - TPS long-word subtitle font

`CandidateCellHelper.swift`:
- `titleFontSize(isTranslateSwapped:)` - Regular candidate title
- `subtitleFontSize(isTranslateSwapped:)` - Regular candidate subtitle
- `longCellTitleFontSize(isTranslateSwapped:)` - Long-word title
- `longCellSubtitleFontSize(isTranslateSwapped:)` - Long-word subtitle

### Scale Logic

| isTranslateSwapped | Title Scale | Subtitle Scale |
|-------------------|-------------|----------------|
| false (default) | Yes (displays phonetic) | No (displays hanzi) |
| true | No (displays hanzi) | Yes (displays phonetic) |

---

## Test Cases

| TPS Input | TL Output | Description |
|-----------|-----------|-------------|
| ㄉㄧㄠˊ | tiau5 | Initial + vowel + tone |
| ㄙㄨˊㄅㆤㆻ̇ | su5pek8 | Multi-syllable + entering tone |
| ㄍㄠˋㄏㄧㆲˊ | kau2hiong5 | Compound vowel |
| ㄅㄚ | pa | No tone |
| ㆴ | p4 | Single entering tone final |

---

## Auto-Correct Features (v3.4.6+)

### Palatalization Auto-Correct

Non-palatalized affricates (ㄗ/ㄘ/ㄙ/ㆡ) followed by ㄧ or ㆪ are auto-corrected to palatalized forms:

| Input sequence | Auto-corrected to | TL equivalent |
|----------------|-------------------|---------------|
| ㄗ + ㄧ | ㄐ + ㄧ | tsi |
| ㄘ + ㄧ | ㄑ + ㄧ | tshi |
| ㄙ + ㄧ | ㄒ + ㄧ | si |
| ㆡ + ㄧ | ㆢ + ㄧ | ji |

**Implementation**: Rust `phonetics::tps_adjust::palatalization_replacement` (via `RustEngineBridge.tpsInputAdjust`).

Called from:
- iOS: `ActionHandler+CharacterInput.swift`
- Android: `TextInputManager.handleTaigiInput()`

### Nasalized Vowel Auto-Correct

ㆮ (ainn) after ㄧ is auto-corrected to ㆯ (aunn), because "iainn" is not a valid Taiwanese final — only "iaunn" exists.

| Input sequence | Auto-corrected to | TL equivalent |
|----------------|-------------------|---------------|
| ㄧ + ㆮ | ㄧ + ㆯ | iaunn |

**Implementation**: Rust `phonetics::tps_adjust::adjust_nasalized_vowel_key`.

### Multi-Syllable Boundary Detection (v3.4.7)

TPS multi-syllable input inserts automatic spaces at syllable boundaries when a tone mark or entering tone coda is followed by a new consonant or vowel.

**Implementation**: space insertion logic inside Rust `phonetics::tps::from_zhuyin` uses tone marks (ˋ ˪ ˊ ˇ ˫ ˙) and entering tone codas (ㆴ ㆵ ㆻ ㆷ) as boundary signals.

---

## Known Limitation: Tone 1/4 Ambiguous Syllable Matching

> **Status**: NOT YET IMPLEMENTED — documented for future improvement.

### Problem

TPS tone 1 and tone 4 are **unmarked** (no symbol). When a user types `ㄗㄨㄚˋ`, the toTL output is `"tsua2"` (single syllable). This matches 紙 (tl_num: `tsua2`) but NOT 珠仔 (tl_num: `tsu1a2`), because the implicit tone 1 on `tsu` is absent from the search key.

| Word | tl_num | TPS input ㄗㄨㄚˋ | Match? |
|------|--------|------------------|--------|
| 紙 tsuá | `tsua2` | `tsua2` | ✓ |
| 珠仔 tsu-á | `tsu1a2` | `tsua2` | ✗ |

In contrast, POJ/TL users can type `tsua` (toneless) which matches the `tl_notone` key `"tsua"` → both words found. But TPS users who type an explicit tone 2 get `"tsua2"`, which doesn't match `"tsu1a2"`.

### Proposed Solution: `tps:` Prefix Family in `dictionary.fst`

Build a separate set of fst keys under a `tps:` prefix where tone digits 1 and 4 are stripped:

```
紙  tl_num: tsua2   → tps key: tsua2   (no 1/4 to strip)
珠仔 tl_num: tsu1a2  → tps key: tsua2   (tone 1 removed)
甘  tl_num: kam1    → tps key: kam     (tone 1 removed)
角  tl_num: kak4    → tps key: kak     (tone 4 removed)
```

Search side: TPS mode uses `tps:` prefix in `lexicon::key_normalizer::build` and the search key strips 1/4 before lookup.

### Implementation Plan

1. **fst builder** (`dictionary/build/create_fst.py` + `engine/build-helpers/fst-builder`): emit a `tps:` key family — `tps_num` = `tl_num` with `'1'` and `'4'` stripped, plus reuse `tl_notone` / `tl_abbrev` under `tps:`.
2. **Rust `lexicon::key_normalizer`**: extend `KeyMode::Tps` so it produces a `tps:` prefix instead of falling through to `tl:`.
3. **Search-key normalization**: extend `phonetics::normalize_input` (or call site) to strip `'1'` / `'4'` for the TPS path so input matches stored keys.
4. **Platform InputMode**: ensure both iOS `InputMode.tps` and Android `InputMode.TPS` flow through to the engine seam unchanged.
5. **Rebuild assets**: run the build pipeline to regenerate `dictionary.fst` with the new `tps:` keys (byte-identical across platforms — see `binary-format.md` §3).

### Why Tone 1/4 Specifically

| Tone | TPS symbol | Explicitly typed? |
|------|-----------|-------------------|
| 1 | (none) | No |
| 2 | ˋ | Yes |
| 3 | ˪ | Yes |
| 4 | (none, implied by checked coda) | No |
| 5 | ˊ | Yes |
| 7 | ˫ | Yes |
| 8 | ˙ | Yes |
| 9 | ˆ | Yes |

Stripping 1/4 = removing tones that TPS users physically cannot type, while preserving tones they explicitly chose.

### Impact Estimate

- fst size: ~50% more keys (~3 per entry added to existing 6).
- The Burntsushi fst is highly compressed — estimated ~1–2 MB increase on top of the current ~9.1 MB asset.
- Zero impact on POJ/TL (they continue using `tl:` / `poj:` prefix).

---

## Pure Core vs Platform Glue (post-Rust)

All TPS phonetic logic now lives in Rust and is shared by both platforms via the FFI seam. Platform code retains only the keystroke dispatcher + UI rendering.

### Rust core (cross-platform)

| Module | Responsibility |
|--------|----------------|
| `engine/phonetics::tables` | Pure data — string → string mappings, no platform deps |
| `engine/phonetics::tps` | TPS detection (`is_zhuyin`) + TPS → TL (`from_zhuyin`) |
| `engine/phonetics::tps_adjust` | Key-level adjustments (palatalization, syllabic nasal, positional ㄇ/ㆬ, ㆮ/ㆯ) |
| `engine/phonetics::api` | TL → TPS display + numeric paths |

### Platform glue (stays per-platform)

| Concern | iOS | Android |
|---------|-----|---------|
| Composing platform wrapper | `Input/Composing/ComposingManager.swift` (KeyboardKit / Combine) | `ime/text/composing/ComposingManager.kt` |
| Effect → text-region binding | `Input/Composing/ComposingDelegate.swift` (`UITextDocumentProxy`) | `ime/text/composing/ComposingDelegate.kt` (`InputConnection`) |
| Keystroke dispatcher | `Input/CharacterInputPipeline.swift` | `ime/text/CharacterInputPipeline.kt` |
| Layout / candidate UI | `Layout/`, `Autocomplete/Views/` | `ime/text/layout/`, `ime/text/smartbar/` |

---

## References

- [Tailo-TPS-Converter](https://github.com/leechunhoe/Tailo-TPS-Converter)
- [MOE Taiwanese Dictionary](https://sutian.moe.edu.tw/zh-hant/siannuntiao/)
