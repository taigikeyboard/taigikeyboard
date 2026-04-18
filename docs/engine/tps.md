# TPS Taiwanese Phonetic Symbols (方音符號)

> **Type**: Feature
> **Keywords**: `TPS`, `TPSConverter`, `方音符號`, `Taiwanese Zhuyin`
> **Related**: tone.md, trie.md, ../ui/layout.md

---

## Summary

- TPS (Taiwanese Phonetic Symbols) input method
- Input phonetic symbols, automatically convert to TL for Trie search
- QWERTY-style keyboard layout

---

## File Structure

### iOS — TPS modules (split by responsibility)

| File | Responsibility |
|------|----------------|
| `Input/TPS/TPSTables.swift` | Mapping tables (consonants/vowels/tones) + membership queries (`containsTPS`, `isTPSToneMark`) |
| `Input/TPS/TPSToTL.swift` | TPS → TL parser (`convert`, `convertMultiSyllable`) |
| `Input/TPS/TLToTPS.swift` | TL → TPS parser (`convert`, `convertFromDisplay`) |
| `Input/TPS/TPSInputAdjuster.swift` | Key-level auto-adjust (positional, palatalization, syllabic nasal, ㆮ/ㆯ) |
| `Input/CharacterInputPipeline.swift` | Pure-function pipeline gluing adjusters into a single `adjust(_:inputMode:rawInput:)` call |
| `Input/TPS/TPSConverter.swift` | Thin facade re-exporting the above for test-only callers (deprecated) |

### iOS — Other TPS-aware files

| File | Description |
|------|-------------|
| `Layout/TaigiLayouts.swift` | TPS layout definition |
| `Settings/SharedSettings.swift` | `.tps` layout type |
| `Layout/CustomLayoutService.swift` | TPS layout selection |
| `Lexicon/Trie/InputNormalizer.swift` | TPS input normalization |
| `Autocomplete/Views/CandidateCellHelper.swift` | Candidate TPS display |

### Android

| File | Responsibility |
|------|----------------|
| `ime/dictionary/TPSConverter.kt` | Still monolithic — planned to mirror the iOS split |
| `ime/text/composing/ComposingManager.kt` | Composing state |

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

## Core API

### Detection — `TPSTables`

```swift
TPSTables.containsTPS("ㄉㄧㄠˊ")       // true
TPSTables.isTPSToneMark("ˋ")          // true
```

### TPS → TL — `TPSToTL`

```swift
TPSToTL.convert("ㄉㄧㄠˊ")              // "tiau5"
TPSToTL.convertMultiSyllable("ㄉㄧㄠ ㄙㄨˊ") // "tiau su5"
```

### TL → TPS — `TLToTPS`

```swift
TLToTPS.convert("tiau5")               // "ㄉㄧㄠˊ"
TLToTPS.convertFromDisplay("guá")      // "ㄍㄨㄚˋ"
```

### Key-level adjustments — `TPSInputAdjuster`

```swift
TPSInputAdjuster.adjustInitialKey("ㄇ", afterRawInput: "ㄅㄚ")           // "ㆬ"
TPSInputAdjuster.adjustNasalizedVowelKey("ㆮ", afterRawInput: "ㄧ")     // "ㆯ"
TPSInputAdjuster.palatalizationReplacement(forIncoming: "ㄧ", lastRawChar: "ㄗ") // "ㄐ"
TPSInputAdjuster.syllabicNasalReplacement(forIncoming: "ˊ", lastRawChar: "ㄫ")  // "ㆭ"
```

### Pipeline — `CharacterInputPipeline`

Used by `ActionHandler+KeyActions.handleCharacterInput` to apply all key-level
adjustments in one call. Pure function — no hidden side effects.

```swift
let result = CharacterInputPipeline.adjust(
    "ㄇ", inputMode: .tps, rawInput: "ㄅㄚ",
)
// result.char = "ㆬ", result.replaceLast = nil
```

### InputNormalizer Integration

```swift
InputNormalizer.normalize("ㄉㄧㄠˊ", mode: .tl)  // "tiau5"
TPSTables.containsTPS("ㄅㄚ")                    // true
```

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
| iOS | `TPSInputAdjuster.adjustInitialKey(_:afterRawInput:)` (via `CharacterInputPipeline.adjust`) | `ActionHandler+KeyActions.handleCharacterInput` |
| Android | `TPSConverter.adjustTPSInitialKey(char, afterRawInput)` | `TextInputManager.handleTaigiInput()` |

### Syllabic Nasal Tone-Triggered Correction (v3.4.7)

When a tone mark follows bare ㄇ or ㄫ at syllable start, the consonant is retroactively corrected to its syllabic form:

| Input sequence | Auto-corrected to | TL equivalent |
|----------------|-------------------|---------------|
| ㄇ + ˫ | ㆬ + ˫ | m7 |
| ㄫ + ˊ | ㆭ + ˊ | ng5 |

**Implementation**: `TPSInputAdjuster.syllabicNasalReplacement(forIncoming:lastRawChar:)`

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

**Implementation**: `TPSInputAdjuster.palatalizationReplacement(forIncoming:lastRawChar:)`

Called from:
- iOS: `ActionHandler+CharacterInput.swift`
- Android: `TextInputManager.handleTaigiInput()`

### Nasalized Vowel Auto-Correct

ㆮ (ainn) after ㄧ is auto-corrected to ㆯ (aunn), because "iainn" is not a valid Taiwanese final — only "iaunn" exists.

| Input sequence | Auto-corrected to | TL equivalent |
|----------------|-------------------|---------------|
| ㄧ + ㆮ | ㄧ + ㆯ | iaunn |

**Implementation**: `TPSInputAdjuster.adjustNasalizedVowelKey(_:afterRawInput:)`

### Multi-Syllable Boundary Detection (v3.4.7)

TPS multi-syllable input inserts automatic spaces at syllable boundaries when a tone mark or entering tone coda is followed by a new consonant or vowel.

**Implementation**: Space insertion logic in `TPSToTL.convert()` / `convertMultiSyllable()` uses tone marks (ˋ ˪ ˊ ˇ ˫ ˙) and entering tone codas (ㆴ ㆵ ㆻ ㆷ) as boundary signals.

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

### Proposed Solution: `tps:` Prefix Trie Key

Build a separate set of trie keys with `tps:` prefix where tone digits 1 and 4 are stripped:

```
紙  tl_num: tsua2   → tps key: tsua2   (no 1/4 to strip)
珠仔 tl_num: tsu1a2  → tps key: tsua2   (tone 1 removed)
甘  tl_num: kam1    → tps key: kam     (tone 1 removed)
角  tl_num: kak4    → tps key: kak     (tone 4 removed)
```

Search side: TPS mode uses `tps:` prefix and also strips 1/4 from the search key.

### Implementation Plan

1. **Trie builder** (`dictionary/build/04_create_trie.py`): Add `tps:` prefix keys — `tps_num` = `tl_num` with '1' and '4' removed, plus reuse `tl_notone` and `tl_abbrev` under `tps:` prefix.
2. **Prefix constants** (both platforms): Add `tps:` prefix, map `.tps` → `"tps:"` in `triePrefix()`.
3. **Search key** (both platforms): Strip '1' and '4' from `normalizedInput` when `inputMode == .tps`.
4. **Android InputMode**: Add `InputMode.TPS` enum value (only for trie search; tone/case handling still uses TL).
5. **Rebuild trie**: Run build script to regenerate `dictionary.trie` with `tps:` keys.

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

- Trie size: ~50% more keys (~3 per entry added to existing 6).
- MARISA-trie is highly compressed — estimated ~1-2 MB increase.
- Zero impact on POJ/TL (they continue using `tl:`/`poj:` prefix).

---

## Pure Core vs Platform Glue

The split modules are organized so that the "pure logic" layer can later be
extracted into a cross-platform shared core (SwiftPM module), leaving only the
platform-specific glue in the keyboard extension.

### Pure logic (candidates for a shared core)

| File | Why it's portable |
|------|-------------------|
| `Input/TPS/TPSTables.swift` | Pure data — string → string mappings, no framework imports |
| `Input/TPS/TPSToTL.swift` | Pure string transforms, no platform APIs |
| `Input/TPS/TLToTPS.swift` | Same |
| `Input/TPS/TPSInputAdjuster.swift` | Pure queries over a `String` buffer |
| `Input/CharacterInputPipeline.swift` | Composes adjusters; returns a plain struct, no side effects |

### Platform glue (stays in keyboard extension)

| File | Why it's platform-bound |
|------|--------------------------|
| `Input/Composing/ComposingManager.swift` | Uses `@Published` / `ObservableObject` / `KeyboardContext` (KeyboardKit) |
| `Input/Composing/ComposingDelegate.swift` | Protocol bridging to `UITextDocumentProxy` through `KeyboardViewController` |
| `Input/KeyboardContext+Composing.swift` | Obj-C associated object on `KeyboardContext` |
| `Input/CaseTransformer.swift` | Mixed — transform logic is pure, but takes `Keyboard.KeyboardCase` (KeyboardKit) |

Android has the same pure/platform split conceptually; its `ComposingManager.kt`
talks directly to `InputConnection` instead of going through a delegate.
Extracting the pure core would let both platforms share one implementation of
the tables + parsers + adjusters, keeping only thin platform adapters.

---

## References

- [Tailo-TPS-Converter](https://github.com/leechunhoe/Tailo-TPS-Converter)
- [MOE Taiwanese Dictionary](https://sutian.moe.edu.tw/zh-hant/siannuntiao/)
