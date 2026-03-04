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

| File | Description |
|------|-------------|
| `Input/TPSConverter.swift` | TPS ↔ TL bidirectional converter |
| `Layout/TaigiLayouts.swift` | TPS layout definition |
| `Settings/SharedSettings.swift` | `.tps` layout type |
| `Layout/CustomLayoutService.swift` | TPS layout selection |
| `Lexicon/Trie/InputNormalizer.swift` | TPS input normalization |
| `Autocomplete/Views/CandidateCellHelper.swift` | Candidate TPS display |

---

## Conversion Flow

```
User input → TPS detection → TPSConverter.toTL() → InputNormalizer → Trie search
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
| ㆶ | k + 4 | Entering tone -k (陰入) |
| ㆷ | h + 4 | Entering tone -h (陰入) |
| ㆴ̇ | p + 8 | Entering tone -p (陽入) |
| ㆵ̇ | t + 8 | Entering tone -t (陽入) |
| ㆶ̇ | k + 8 | Entering tone -k (陽入) |
| ㆷ̇ | h + 8 | Entering tone -h (陽入) |

---

## Keyboard Layout

```
Row 1 (Tones):  [ˊ] [ˋ] [˪] [˫] [ㆴ] [ㆵ] [ㆶ] [ㆷ] [ㆦ] [ㆤ]
Row 2 (Initials): [ㄅ] [ㄉ] [ㆣ] [ㄍ] [ㄎ] [ㄆ] [ㄊ] [ㄗ] [ㆡ] [ㄫ]
Row 3 (Mixed):  [ㄇ] [ㄚ] [ㄨ] [ㄏ] [ㄌ] [ㄘ] [ㄧ] [ㆠ] [ㄢ]
Row 4 (Vowels): [⬆]  [ㄛ] [ㄙ] [ㆬ] [ㄋ] [ㄥ] [ㄤ] [ㄞ] [⌫]
Row 5 (Function): [123] [😀] [     Space     ] [Translate] [⏎]
```

### Layout Variants

| Suffix | Description |
|--------|-------------|
| `tps_iPhone` | No globe key |
| `tps_withGlobe` | Has globe key (iPhone SE, iPad) |

---

## Core API

### TPSConverter

```swift
// Check if contains TPS characters
TPSConverter.containsTPS("ㄉㄧㄠˊ") // true

// TPS → TL conversion (for Trie search)
TPSConverter.toTL("ㄉㄧㄠˊ") // "tiau5"

// TL → TPS conversion (for candidate display)
TPSConverter.toTPS("tiau5") // "ㄉㄧㄠˊ"

// Multi-syllable conversion
TPSConverter.toTLMultiSyllable("ㄉㄧㄠ ㄙㄨˊ") // "tiau su5"
```

### InputNormalizer Integration

```swift
// Auto-detect and convert TPS
InputNormalizer.normalize("ㄉㄧㄠˊ", mode: .tl) // "tiau5"

// Check if input contains TPS symbols
TPSConverter.containsTPS("ㄅㄚ") // true
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
| ㄙㄨˊㄅㆤㆶ̇ | su5pek8 | Multi-syllable + entering tone |
| ㄍㄠˋㄏㄧㆲˊ | kau2hiong5 | Compound vowel |
| ㄅㄚ | pa | No tone |
| ㆴ | p4 | Single entering tone final |

---

## References

- [Tailo-TPS-Converter](https://github.com/leechunhoe/Tailo-TPS-Converter)
- [MOE Taiwanese Dictionary](https://sutian.moe.edu.tw/zh-hant/siannuntiau/)
