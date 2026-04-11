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

## Migration Plan: Eliminate dictionary.db Copy

### Problem

Both `dictionary.db` and `dictionary.trie` are copied from assets to `files/` at runtime because Android's SQLite and mmap both require a filesystem path. This creates duplicate data on disk.

| File | APK (assets) | files/ (copy) | Why copied |
|------|-------------|---------------|------------|
| `dictionary.db` | ~15-20 MB (compressed) | ~15-20 MB | SQLite needs file path |
| `dictionary.trie` | ~3-4 MB (compressed) | ~3-4 MB | mmap needs file path |

Since both files are **read-only**, neither needs to be in `files/`.

### Current Architecture

```
User input → Trie(key → rowid[]) → SQLite(rowid → hanzi, tl, frequency, dict flags)
               ↑ copied to files/     ↑ copied to files/
```

SQLite serves three purposes:

| # | Purpose | Query | Call site |
|---|---------|-------|-----------|
| 1 | **Main lookup**: rowid → word data + dict filtering | `WHERE id IN (?) AND kautian=1 OR ...` | `LexiconService.search()` |
| 2 | **Hanzi reverse lookup** (settings UI only) | `WHERE hanzi LIKE '%input%'` | `LexiconService.searchByHanzi()` → `DictionarySearchViewModel` |
| 3 | **Word association** | `SELECT FROM word_association` | `NextWordService` |

### Proposed Architecture

Two changes: (A) encode dictionary data into trie value to eliminate SQLite, (B) mmap trie directly from APK to eliminate trie copy.

#### A. Encode Data into Trie Value (eliminate dictionary.db)

Instead of `key → rowid → SQLite lookup`, encode all data directly into the trie value:

```
Current:   trie key → rowid    → SQLite query → {hanzi, tl, frequency, flags}
Proposed:  trie key → encoded value            → parse in memory → {hanzi, tl, frequency, bitmask}
```

Example trie entry:
```
key   = "hoo2boo5"
value = "好無\t42\t0x005"    ← hanzi + frequency + bitmask
```

This is what mainstream IMEs do (AOSP LatinIME, mozc) — index and data in one file. No separate data store for read-only dictionary.

**Dictionary bool flags → bitmask:**

12 boolean columns + is_variant encoded as 16-bit int:

```
bit 0  = kautian    bit 4  = taihoa    bit 8  = khpoo
bit 1  = taigitv    bit 5  = taijit    bit 9  = khiin
bit 2  = itaigi     bit 6  = kungge    bit 10 = lkk
bit 3  = sitbut     bit 7  = stti      bit 11 = dev
bit 12 = is_variant
```

Filtering: `if (entry.bitmask and enabledMask != 0) → include`

**Hanzi reverse lookup:** build a second hanzi-keyed trie for prefix matching (replaces `LIKE '%input%'`). Settings UI only — prefix search is sufficient.

**Word association:** needs separate handling (binary format or small standalone file).

#### B. noCompress + Direct mmap from APK (eliminate trie copy)

Mainstream approach (used by AOSP LatinIME / Gboard):

**Build time** — mark assets as uncompressed:
```groovy
// build.gradle
androidResources {
    noCompress ".trie"
}
```

**Runtime** — get file descriptor + offset directly into APK:
```kotlin
val afd = context.assets.openFd("dictionary.trie")
// afd.fileDescriptor → APK fd
// afd.startOffset    → data offset within APK
// afd.length         → data length
```

**Native** — mmap directly from APK, zero copy:
```cpp
void* ptr = mmap(nullptr, afd.length, PROT_READ, MAP_PRIVATE, fd, offset);
```

Trade-off: APK slightly larger (uncompressed asset) but no `files/` copy at all.

Requires modifying MARISA-trie loading to accept fd + offset instead of file path.

### Target Architecture

```
dictionary.trie (noCompress) → direct mmap from APK → key → {hanzi, freq, bitmask}
hanzi.trie     (noCompress) → direct mmap from APK → hanzi prefix search (settings UI)
user_frequency.db           → files/ (SQLite, user data, needs write)
user_association.db         → files/ (SQLite, user data, needs write)
custom_dictionary.db        → files/ (SQLite, user data, needs write)
word_association            → TBD (binary format or standalone file)
```

**Principle:** read-only data shipped with app → binary format, direct mmap from APK. User-generated data needing writes → SQLite in `files/`.

### Feasibility

| Aspect | Assessment |
|--------|------------|
| Encode data in trie value | MARISA-trie supports string values; build script change |
| Bitmask filtering | In-memory AND, nanosecond-level for ~100K entries |
| Hanzi reverse lookup | Second trie with hanzi keys, prefix matching sufficient |
| Word association | Needs separate solution |
| noCompress + direct mmap | Requires JNI change to accept fd+offset; AOSP LatinIME proves this works |
| Build tooling | Modify CSV → trie script to include data in value |
| Disk savings | Eliminate ~15-20 MB (db) + ~3-4 MB (trie) from `files/` |

### Status

**Not started** — documenting for future implementation.

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
