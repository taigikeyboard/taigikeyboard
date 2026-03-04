# Flick Tone Keyboard

> **Type**: Feature
> **Keywords**: `Flick`, `FlickKeyDef`, `FlickLayout`, `FlickTab`, `Tone Input`
> **Related**: layout.md, ../engine/tone.md, ../references/azookey-reference.md

---

## Summary

> **Status: Not yet implemented** — This doc is a design spec for a planned feature.

- Swipe input for tones, solving the biggest pain point of Taigi input
- 4 rows × 6 columns grid layout (reference: azooKey), vowels flick for tones, consonants flick for groups
- Built-in number/symbol keyboard and ABC keyboard, tab switching stays in Flick mode
- Translate button supports hanzi/romanization toggle

---

## File Structure

| File | Description |
|------|-------------|
| `FlickModels.swift` | Flick data models (direction, key, layout) |
| `FlickTaigiLayout.swift` | Taigi Flick layout definition (TL/POJ) |
| `FlickKeyView.swift` | Single Flick key view |
| `FlickKeyboardView.swift` | Flick keyboard grid |
| `TaigiFlickKeyboardView.swift` | Main view integrating candidates |

---

## Tone Mapping

| Operation | Tone | Example |
|-----------|------|---------|
| Tap | Tone 1 (陰平) | a |
| ← Left swipe | Tone 2 (陰上) | á |
| ↑ Up swipe | Tone 3 (陰去) | à |
| → Right swipe | Tone 5 (陽平) | â |
| ↓ Down swipe | Tone 7 (陽去) | ā |
| Long press | Tone 8 (陽入) | a̍ |

> Tone 4 is determined by final consonants -p/-t/-k/-h, no tone mark needed

---

## Layout Design

### Coordinate System (reference: azooKey CustardKit)

- **4 rows (y=0-3) × 6 columns (x=0-5)**
- x: horizontal column (0=left → 5=right)
- y: vertical row (0=top → 3=bottom)

### Key Zones

| Column | Use | Description |
|--------|-----|-------------|
| x=0 | Tab switching column | ☆123, ABC, 譯/Taigi, 🌐 |
| x=1-4 | Character input area | 16 input keys |
| x=5 | System function column | ⌫, space, enter (spans 2 rows) |

### Taigi Main Keyboard (4 rows × 6 columns)

```
         x=0       x=1      x=2      x=3      x=4      x=5
y=0      ☆123      a        e        i        o        ⌫
y=1      ABC       oo       u        p        k        space
y=2      譯        ts       n       nn，。   「」『』   enter
y=3      🌐       ，。     ？！     《》〈〉   —⋯       ┘
```

### Number Symbol Keyboard (4 rows × 6 columns)

```
         x=0       x=1      x=2      x=3      x=4      x=5
y=0      ☆123      1        2        3        4        ⌫
y=1      ABC       5        6        7        8        space
y=2      Taigi     9        0       ()[]     .,-/      enter
y=3      🌐       ¥$€      %°#     +-×÷      <=>       ┘
```

### ABC Keyboard (4 rows × 6 columns)

```
         x=0       x=1      x=2      x=3      x=4      x=5
y=0      ☆123     @#/&_    ABC      DEF       GHI      ⌫
y=1      abc       JKL      MNO     PQRS      TUV      space
y=2      Taigi    WXYZ     '\"()    .,?!      a/A      enter
y=3      🌐       0-9      +-=      *&^      _|\\      ┘
```

---

## Tab Switching

### First Column (x=0) System Keys

| Position | Taigi Keyboard | Number Keyboard | ABC Keyboard |
|----------|----------------|-----------------|--------------|
| y=0 | ☆123 | ☆123(selected) | ☆123 |
| y=1 | ABC | ABC | abc(selected) |
| y=2 | 譯 | Taigi | Taigi |
| y=3 | 🌐 | 🌐 | 🌐 |

### Switching Description

| Key | Function | Position |
|-----|----------|----------|
| ☆123 | Switch to number symbols | (x=0, y=0) |
| ABC/abc | Switch to ABC | (x=0, y=1) |
| 譯 | Toggle hanzi/romanization | Taigi keyboard (x=0, y=2) |
| Taigi | Switch to Taigi main keyboard | Number/ABC keyboard (x=0, y=2) |
| a/A | Toggle case | ABC keyboard (x=4, y=2) |
| 🌐 | Switch input method | (x=0, y=3) |

> Tab switching happens within Flick mode, won't jump back to QWERTY

---

## Key Types

### Vowel Keys (Tone Flick)

| Key | Center | ←Left | ↑Up | →Right | ↓Down | Long press |
|-----|--------|-------|-----|--------|-------|------------|
| a | a | á | à | â | ā | a̍ |
| e | e | é | è | ê | ē | e̍ |
| i | i | í | ì | î | ī | i̍ |
| o | o | ó | ò | ô | ō | o̍ |
| oo | oo | óo | òo | ôo | ōo | o̍o |
| u | u | ú | ù | û | ū | u̍ |

### Consonant Keys (Group Flick)

| Key | Center | ←Left | ↑Up | →Right | ↓Down |
|-----|--------|-------|-----|--------|-------|
| p | p | ph | b | m | - |
| k | k | kh | g | ng | h |
| ts | ts | tsh | s | j | l |
| n | n | th | t | - | - |

### Special Keys

| Key | Center | ←Left | ↑Up | →Right | ↓Down | Description |
|-----|--------|-------|-----|--------|-------|-------------|
| nn，。 | nn | ， | 。 | ？ | ！ | Nasalization + punctuation |
| 「」『』 | 「 | 」 | 『 | 』 | - | Quotation brackets |
| ，。 | ， | 。 | 、 | ； | ： | Comma period punctuation |
| ？！ | ？ | ！ | ～ | ‥ | … | Question exclamation |
| 《》 | 《 | 》 | 〈 | 〉 | 【 | Book title marks |
| —⋯ | — | ⋯ | － | ＿ | - | Dash ellipsis |

### Number Keys

| Key | Center | ←Left | ↑Up | →Right | ↓Down |
|-----|--------|-------|-----|--------|-------|
| 1 | 1 | ① | ⑴ | ⒈ | - |
| 2 | 2 | ② | ⑵ | ⒉ | - |
| 3 | 3 | ③ | ⑶ | ⒊ | - |
| 4 | 4 | ④ | ⑷ | ⒋ | - |
| 5 | 5 | ⑤ | ⑸ | ⒌ | - |
| 6 | 6 | ⑥ | ⑹ | ⒍ | - |
| 7 | 7 | ⑦ | ⑺ | ⒎ | - |
| 8 | 8 | ⑧ | ⑻ | ⒏ | - |
| 9 | 9 | ⑨ | ⑼ | ⒐ | - |
| 0 | 0 | ⓪ | ⑽ | ⒑ | - |
| ()[] | ( | ) | [ | ] | {} |
| .,-/ | . | , | - | / | - |

### Number Symbol Zone Keys

| Key | Center | ←Left | ↑Up | →Right | ↓Down |
|-----|--------|-------|-----|--------|-------|
| ¥$€ | ¥ | $ | € | £ | ₩ |
| %°# | % | ° | # | ‰ | ℃ |
| +-×÷ | + | - | × | ÷ | ± |
| <=> | < | = | > | ≤ | ≥ |

### ABC Keys

| Key | Center | ←Left | ↑Up | →Right | ↓Down |
|-----|--------|-------|-----|--------|-------|
| @#/&_ | @ | # | / | & | _ |
| ABC | a | b | c | - | - |
| DEF | d | e | f | - | - |
| GHI | g | h | i | - | - |
| JKL | j | k | l | - | - |
| MNO | m | n | o | - | - |
| PQRS | p | q | r | s | - |
| TUV | t | u | v | - | - |
| WXYZ | w | x | y | z | - |
| '\"() | ' | \" | ( | ) | - |
| .,?! | . | , | ? | ! | - |

### ABC Symbol Zone Keys

| Key | Center | ←Left | ↑Up | →Right | ↓Down |
|-----|--------|-------|-----|--------|-------|
| 0-9 | 0 | 1 | 2 | 3 | 4 |
| +-= | + | - | = | * | - |
| *&^ | * | & | ^ | ~ | - |
| _\|\\ | _ | \| | \\ | \` | - |

---

## Label Display Styles

| Style | Use | Display Method |
|-------|-----|----------------|
| `directional` | Vowels/consonants | Four-direction hints at edges |
| `verticalSub` | Numbers/symbols | Sub-text below main text |

### directional Example (Vowel)

```
┌─────────┐
│    à    │  ← up (tone 3)
│ á  a  â │  ← left/center/right
│    ā    │  ← down (tone 7)
└─────────┘
```

### verticalSub Example (Number)

```
┌─────────┐
│    1    │  ← main text (large)
│   、⋯—   │  ← sub-text (small)
└─────────┘
```

---

## Data Models

### FlickTab

```swift
enum FlickTab {
    case taigi   // Taigi main keyboard
    case number  // Number symbols
    case abc     // ABC English
}
```

### FlickDirection

```swift
enum FlickDirection {
    case left   // ← tone 2
    case top    // ↑ tone 3
    case right  // → tone 5
    case bottom // ↓ tone 7
}
```

### FlickLabelStyle

```swift
enum FlickLabelStyle {
    case directional  // Four-direction hint mode (vowels/consonants)
    case verticalSub  // Vertical sub-text mode (numbers/symbols)
}
```

### FlickSystemKey

```swift
enum FlickSystemKey {
    case delete, space, enter, globe
    case switchToQwerty
    case translate           // Toggle hanzi/romanization
    // Flick internal tab switching
    case flickTabTaigi
    case flickTabNumber
    case flickTabAbc
    case flickTabShift
}
```

### FlickKeyDef

```swift
enum FlickKeyDef {
    case vowel(base: String, tones: [FlickDirection: String], tone8: String)
    case consonant(center: String, label: String, variations: [FlickDirection: String])
    case symbol(center: String, label: String, variations: [FlickDirection: String])
    case system(FlickSystemKey)
}
```

### FlickKeyPosition

```swift
struct FlickKeyPosition: Hashable {
    let row: Int     // Row (y=0-3)
    let column: Int  // Column (x=0-5)
    var width: Int   // Width (default 1)
    var height: Int  // Height (default 1, Enter is 2)
}
```

---

## Settings Integration

### KeyboardLayoutType

```swift
enum KeyboardLayoutType: String {
    case phahTaigi  // PhahTaigi layout
    case qwerty     // Standard QWERTY
    case flick      // Flick tone
}
```

### Switching Flow

1. Main App Tab2 selects layout
2. `SharedSettings.keyboardLayoutType` stores setting
3. `KeyboardViewController` loads corresponding view based on setting

---

## POJ Mode Differences

| TL | POJ |
|----|-----|
| oo | o͘ |
| ts | ch |
| tsh | chh |

---

## Available Layouts

| Identifier | Description | Grid |
|------------|-------------|------|
| `taigiTone` | Taigi tone (TL mode) | 4 rows × 6 columns |
| `taigiTonePOJ` | Taigi tone (POJ mode) | 4 rows × 6 columns |
| `flickNumber` | Number symbols | 4 rows × 6 columns |
| `flickAbc` | ABC lowercase | 4 rows × 6 columns |
| `flickAbcUpper` | ABC uppercase | 4 rows × 6 columns |

---

## Platform Comparison

| Item | iOS | Android |
|------|-----|---------|
| Layout definition | `FlickTaigiLayout.swift` | Pending implementation |
| Key view | `FlickKeyView.swift` | Pending implementation |
| Keyboard view | `TaigiFlickKeyboardView.swift` | Pending implementation |
| Style definition | `FlickKeyStyle.swift` | Pending implementation |
