# Trie Index and Lexicon Queries

> **Type**: Feature
> **Keywords**: `Trie`, `Lexicon`, `MARISA`, `InputNormalizer`, `TrieService`
> **Related**: autocomplete.md, sort.md

---

## Summary

- MARISA-trie stores key → SQLite rowid
- TL-only trie: single namespace, no POJ/TL prefix distinction
- InputNormalizer converts diacritics→numbers AND POJ spelling→TL (in POJ mode)

---

## Architecture

| Component | Content | Purpose |
|-----------|---------|---------|
| MARISA-trie | key → rowid | Prefix queries |
| SQLite | Full word entries | Data storage |

---

## Trie Key Format

All keys use TL spelling. POJ input is converted to TL at query time by `InputNormalizer`.

| Key type | Example | Description |
|----------|---------|-------------|
| `tl_num` | `hoo2boo5` | TL numeric tone |
| `tl_notone` | `hooboo` | TL without tone |
| `tl_abbrev` | `hb` | TL abbreviation |

---

## Query Flow

```
Input "goa2" (POJ mode)
  → InputNormalizer.normalize(mode: .poj)
    → diacritics→digits: "goa2"
    → POJ→TL spelling: "gua2"
  → TrieService.prefixSearch("gua2")
  → rowid list
  → SQLite batch query (returns TL diacritics)
  → RomanizationConverter.tlToPOJ() for display
  → Sort by frequency
```

---

## InputNormalizer

Normalizes user input to Trie key format.

### Processing Steps

1. TPS detection → convert to TL if needed
2. Convert to lowercase
3. Split by hyphens, per-syllable:
   - Convert nasal markers (`ⁿ`/`ᴺ` → `nn`)
   - Convert `o͘` (U+0358) → `oo`
   - Strip combining diacritics → numeric tone digit
   - Auto-assign default tone (1 or 4) if other syllables have tones
4. If POJ mode: convert POJ spelling → TL via `RomanizationConverter.pojInputToTL()`

### POJ→TL Spelling Conversion (POJ mode only)

| POJ | TL | Example |
|-----|-----|---------|
| `ch` | `ts` | `chui` → `tsui` |
| `chh` | `tsh` | `chha` → `tsha` |
| `ou` | `oo` | `hou` → `hoo` |
| `oa` | `ua` | `goa` → `gua` |
| `oe` | `ue` | `koe` → `kue` |
| `eng` | `ing` | `seng` → `sing` |
| `ek` | `ik` | `tek` → `tik` |

### Note

- Tone number placed at syllable end (`gáb` → `gab2`)
- POJ→TL conversion is order-dependent (`ch→ts` before vowel rules)

---

## Platform Correspondence

| Item | iOS | Android |
|------|-----|---------|
| Trie service | `TrieService.swift` | `TrieService.kt` |
| Normalizer | `InputNormalizer.swift` | `InputNormalizer.kt` |
| Dictionary service | `LexiconService.swift` | `LexiconService.kt` |
| JNI | marisa_bridge.cpp | trie_jni.cpp |

---

## Android JNI

### RecordTrie Format

```
raw_key = utf8_key + \xff + uint32_le(rowid)
```

### Core Functions

| Function | Description |
|----------|-------------|
| `nativeLoad(path)` | mmap load trie |
| `nativePrefixSearch(prefix, limit)` | Prefix query |
| `nativeLookup(key)` | Exact match |

---

## Design Decisions

| Decision | Rationale |
|----------|-----------|
| rowid as value | No need for separate global id |
| Numeric tones in Trie | ASCII compatible |
| TL-only trie | Runtime POJ→TL conversion; halves trie size |
| Store both toned/toneless | No extra filtering needed |
| mmap loading | Saves memory |

---

## Space Estimates

- 300K entries × 3 keys ≈ 900K key-value pairs
- dictionary.trie ≈ 2.5-4 MB
- dictionary.db ≈ 15-20 MB

---

## Debug

```bash
adb logcat -s TrieService:D LexiconService:D
```

| Issue | Checkpoint |
|-------|------------|
| Trie not initialized | `isReady=false` |
| Normalization error | `[NORMALIZE]` result |
| No results | lookup + prefixSearch both 0 |
