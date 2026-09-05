# Keyboard Layout

> **Type**: Feature
> **Keywords**: `Layout`, `KeyDef`, `LayoutConverter`, `CustomLayoutService`
> **Related**: ../architecture/system-overview.md

---

## Summary

- Complete layout definitions with full visibility
- Full-width/half-width specified directly in key definitions
- Device variants: `_iPhone` vs `_withGlobe`

---

## File Structure

| File | Description |
|------|-------------|
| `KeyDef.swift` | Key definition enum |
| `TaigiLayouts.swift` | All keyboard layouts |
| `LayoutConverter.swift` | Convert to KeyboardLayout |
| `CustomLayoutService.swift` | Layout selection logic |

---

## Design Principles

1. **Complete layouts** - Each keyboard is a full `[[KeyDef]]`
2. **Embedded full-width/half-width** - Specify `fullWidth` at key level
3. **Explicit device variants** - iPhone without globe / with globe
4. **No duplicate symbols** - Numeric and Symbolic separated
5. **Ergonomic** - Common punctuation on row 4

---

## Naming Convention

| Suffix | Description | Applies to |
|--------|-------------|------------|
| `_iPhone` | No globe key | Regular iPhone |
| `_withGlobe` | Has globe key | iPhone SE, iPad |

---

## Layout Types

### Alphabetic

| Layout | Features |
|--------|----------|
| PhahTaigi | `!` `?` replace `q` `w`, `,` `.` replace `z` `x` |
| QWERTY TL | Standard QWERTY |
| QWERTY POJ | Extra `o͘` key |
| TPS | Taiwanese Phonetic Symbols (方音符號) |
| MOE1 | MOE Layout 1 (教育部輸入法佈局1) |
| MOE2 | MOE Layout 2 (教育部輸入法佈局2), TL/POJ variants |

### Numeric (Common Symbols)

- Row 1: Numbers
- Row 2: Quotes/brackets
- Row 3: Punctuation
- Row 4: Most common (finger position)

### Symbolic (Advanced Symbols)

- Row 1: Programming brackets
- Row 2: Book title marks (書名號)
- Row 3: Currency/special
- Row 4: Math symbols

---

## Full-Width / Half-Width

| Setting | Display | Use case |
|---------|---------|----------|
| Half-width (default) | `. , ? !` | Full romanization text |
| Full-width | `。，？！` | Mixed Hanzi-romanization |

### Always Half-Width

- Programming: `[ ] { } ( ) < >`
- Technical: `_ \ / @ = $ % # &`
- Currency: `€ £ ¥ ¢`
- Math: `± × ÷ ≠ ≈ ∞ √`

---

## Platform Correspondence

| Item | iOS | Android |
|------|-----|---------|
| Layout definitions | `TaigiLayouts.swift` | `LayoutData.kt` |
| Converter | `LayoutConverter.swift` | `LayoutManager.kt` |
| Key definitions | `KeyDef.swift` | `KeyData.kt` |
