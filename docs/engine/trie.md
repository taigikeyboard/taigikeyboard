# Trie Index and Lexicon Queries

> **Type**: Feature
> **Keywords**: `Trie`, `Lexicon`, `MARISA`, `InputNormalizer`, `TrieService`, `BinaryReader`, `Bitmask`
> **Related**: autocomplete.md, sort.md

---

## Summary

- MARISA-trie stores key → rowid; binary mmap readers decode records by rowid
- Key prefixes: `tl:`, `poj:`, `hanzi:` — single trie file for all lookup types
- InputNormalizer converts diacritics→numbers AND POJ spelling→TL (in POJ mode)
- SQLite eliminated for read-only dictionary data; binary mmap on both platforms

---

## Architecture

| Component | File | Purpose |
|-----------|------|---------|
| MARISA-trie | `dictionary.trie` | Key → rowid prefix search (~4.5 MB, includes `hanzi:` keys) |
| Dictionary binary | `dictionary.bin` | Rowid → {hanzi, tl, frequency, bitmask} via mmap (~4.4 MB) |
| Association binary | `association.bin` | Prev_word → next_word predictions via binary search (~3.1 MB) |
| User SQLite | `user_frequency.db`, `user_association.db`, `custom_dictionary.db` | Writable user data |

**Total read-only assets**: ~12 MB (was ~48 MB with dictionary.db)

---

## Trie Key Format

| Prefix | Key type | Example | Description |
|--------|----------|---------|-------------|
| `tl:` | tl_num | `tl:hoo2boo5` | TL numeric tone |
| `tl:` | tl_notone | `tl:hooboo` | TL without tone |
| `tl:` | tl_abbrev | `tl:hb` | TL abbreviation |
| `poj:` | poj_num | `poj:ho2bo5` | POJ numeric tone |
| `poj:` | poj_notone | `poj:hobo` | POJ without tone |
| `poj:` | poj_abbrev | `poj:hb` | POJ abbreviation |
| `hanzi:` | hanzi | `hanzi:好無` | Hanzi reverse lookup (prefix search) |

Input mode determines prefix: TL/TPS → `tl:`, POJ → `poj:`, Hanzi → `hanzi:`.

---

## Query Flow

### Romanization Autocomplete

```
Input "goa2" (POJ mode)
  → InputNormalizer.normalize(mode: .poj)
    → TPS preprocessing (if needed)
    → lowercase + hyphen split
    → per-syllable: diacritics → numeric tone digits
    → result: "goa2" (POJ-form preserved, NOT converted to TL here)
  → Search key construction (caller adds trie prefix):
    - iOS: AutocompleteService.buildSearchKey() prepends "poj:" + may also build "tl:gua2" via RomanizationConverter.pojInputToTL()
    - Android: LexiconService picks prefix from InputMode; prepends "poj:" or "tl:"
  → TrieService.prefixSearch("poj:goa2") and/or ("tl:gua2")
  → rowid list
  → DictionaryBinaryReader.record(rowId) for each
  → Bitmask filter (bitwise AND, replaces SQL WHERE)
  → Sort by frequency
  → Display conversion (TL→POJ if POJ mode)
```

**Important**: `InputNormalizer.normalize()` does NOT perform POJ→TL spelling conversion on either platform. POJ→TL is done at the search-key construction layer, by `RomanizationConverter.pojInputToTL()` (iOS) or equivalent caller-side logic.

### Hanzi Reverse Lookup (Settings Tab3)

```
Input "好"
  → TrieService.prefixSearch("hanzi:好")
  → rowid list
  → DictionaryBinaryReader.record(rowId) → bitmask filter → results
```

---

## Bitmask Filter

16-bit bitmask per record replaces SQL boolean column filtering.

```
bit 0=kautian  1=taigitv  2=itaigi  3=sitbut  4=taihoa  5=taijit
6=kungge   7=stti     8=khpoo   9=khiin   10=dev    11=lkk
12=is_variant  13-15=reserved
```

3-layer filter:
1. **Variant exclusion**: Skip if `is_variant` bit set and variants disabled
2. **Khiin exclusion**: Skip if khiin-only and khiin disabled
3. **Source OR match**: Pass if any enabled source bit matches (dev always included)

---

## InputNormalizer

### Processing Steps

1. TPS detection → convert to TL if needed
2. Convert to lowercase
3. Split by hyphens / spaces, per-syllable:
   - Convert nasal markers (`ⁿ`/`ᴺ` → `nn`)
   - Convert `o͘` (U+0358) → `oo`
   - Strip combining diacritics → numeric tone digit
   - Auto-assign default tone (1 or 4) if other syllables have tones

**POJ→TL spelling conversion is NOT done here.** It is done at the search-key construction site:
- iOS: `RomanizationConverter.pojInputToTL()` invoked by `AutocompleteService.buildSearchKey()` when building the `tl:` variant
- Android: handled in `LexiconService` / search-key building when needed

The iOS POJ→TL substitutions (only applied when building a TL-prefixed query):

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

- Tone number placed at syllable end (`gáb` → `gab2`)
- POJ→TL conversion is order-dependent (`ch→ts` before vowel rules)

---

## Platform Correspondence

| Item | iOS | Android |
|------|-----|---------|
| Trie service | `TrieService.swift` | `TrieService.kt` |
| Dictionary reader | `DictionaryBinaryReader.swift` | `DictionaryBinaryReader.kt` |
| Association reader | `AssociationBinaryReader.swift` | `AssociationBinaryReader.kt` |
| Enabled dictionaries | `EnabledDictionaries.swift` | `EnabledDictionaries.kt` |
| Normalizer | `InputNormalizer.swift` | `InputNormalizer.kt` |
| Dictionary service | `LexiconService.swift` | `LexiconService.kt` |
| Native bridge | `marisa_bridge.cpp` (handle-based) | `trie_jni.cpp` |

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
| Rowid as trie value | O(1) binary record lookup via offset table |
| Numeric tones in trie | ASCII compatible |
| `tl:` / `poj:` / `hanzi:` prefixes | Scoped prefix search; single trie file for all types |
| Binary mmap over SQLite | 75% size reduction; faster reads; no SQL overhead |
| Platform-independent binary format | Same .bin/.trie files on iOS and Android |
| mmap loading | Memory-efficient; OS manages page cache |

---

## Space Estimates

| File | Size | Content |
|------|------|---------|
| `dictionary.trie` | ~4.5 MB | ~300K entries × 6 key types + hanzi: keys |
| `dictionary.bin` | ~4.4 MB | 159K records (bitmask + frequency + hanzi + tl) |
| `association.bin` | ~3.1 MB | 5,658 keys × ~31 entries avg |
| **Total** | **~12 MB** | Was ~48 MB with dictionary.db |

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
