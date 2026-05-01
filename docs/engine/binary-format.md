# Binary Asset Formats

> **Type**: Reference
> **Keywords**: `dictionary.bin`, `association.bin`, `dictionary.trie`, `MARISA`, `bitmask`, `mmap`, `cross-platform invariant`
> **Related**: `trie.md`, `nextword.md`, `custom-dictionary.md`
> **Audience**: anyone touching `DictionaryBinaryReader`, `AssociationBinaryReader`, `TrieService`, `EnabledDictionaries`, or the build script that generates these assets.

---

## Summary

Three read-only binary assets live in the dictionary bundle and are mmap-loaded by both iOS and Android:

| File | Magic | Purpose | Size |
|---|---|---|---|
| `dictionary.bin` | `TKDB` | rowid → {bitmask, frequency, hanzi, tl} | ~4.4 MB |
| `association.bin` | `TKWA` | prev_word → list of next-word predictions | ~3.1 MB |
| `dictionary.fst` | (fst) | prefix key → rowid (for prefix + exact lookup) | ~9.2 MB |

All formats use **little-endian** integers and **strict UTF-8** strings. Both platforms ship reader code that must agree byte-for-byte; mismatches surface as silent decode failures.

**Source-of-truth** for layout: this document.
**Source-of-truth** for *content*: the Python build pipeline at `dictionary/build/` (`merge_csv.py` → `create_dictionary_bin.py` → `create_fst.py` → `create_association_bin.py` → `audit.py` → `deploy.sh`; the binary writers read `dictionary.csv` directly via `dictionary_records.py` / `associations.py`, while `create_fst.py` shells to the Rust `engine/build-helpers/fst-builder`). When any step changes the binary layout, this document and both readers must be updated **in the same change set**.

---

## 1. `dictionary.bin` — Dictionary Records

### 1.1 File layout

```
+---------------------------------------------------+
| Header (16 bytes)                                 |
|   "TKDB"                4 bytes                   |
|   version (u32 LE)      4 bytes  (currently 1)    |
|   record_count (u32 LE) 4 bytes                   |
|   build_ts (u32 LE)     4 bytes  (unix epoch)     |
+---------------------------------------------------+
| Offset table                                      |
|   record_count × u32 LE                           |
|   absolute byte offset to each record             |
+---------------------------------------------------+
| Record section (variable-length records)          |
|   bitmask        u16 LE  (2 bytes)                |
|   frequency      u32 LE  (4 bytes)                |
|   hanzi_len      u8      (1 byte; may be 0)       |
|   tl_len         u8      (1 byte; must be > 0)    |
|   hanzi          hanzi_len bytes UTF-8            |
|   tl             tl_len   bytes UTF-8             |
+---------------------------------------------------+
```

### 1.2 Record access

`record(rowId)` is **1-based**. `rowId == 0` returns `null`/`nil`.

The end of a record is determined by the *next* record's offset (or `data.count` for the last record). Readers MUST validate `recordEnd > recordOffset` and `recordEnd <= data.count` before parsing.

### 1.3 Constraints

- `tl_len > 0` for every record (TL is required; hanzi may be absent).
- `hanzi == ""` is encoded as `hanzi_len == 0` and yields `null`/`nil` in the reader, **not** an empty string.
- UTF-8 must be valid; readers reject the record (return `null`) on `CharacterCodingException` (Android) or `String(bytes:encoding:)` failure (iOS).
- Record bytes are not aligned; both readers use `loadUnaligned` (Swift) / absolute-position `getInt` (Java NIO) accordingly.

### 1.4 Validation performed by reader

| Check | Reader behaviour on failure |
|---|---|
| File ≥ 16 bytes | `init?` returns `nil` |
| Magic == `TKDB` | `init?` returns `nil` |
| Version == 1 | `init?` returns `nil` |
| File ≥ `header + record_count × 4` | `init?` returns `nil` |
| `record_count ≤ Int.MAX_VALUE` (Android only — u32 → Int) | `open` returns `null` |
| Per-record bounds (`recordEnd ≤ data.count`) | `record()` returns `null` |
| Per-record min size 8 bytes | `record()` returns `null` |
| `pos + hanziLen + tlLen ≤ recordEnd` | `record()` returns `null` |
| TL UTF-8 valid | `record()` returns `null` |

---

## 2. `association.bin` — Bigram Predictions

### 2.1 File layout

```
+---------------------------------------------------+
| Header (20 bytes)                                 |
|   "TKWA"                4 bytes                   |
|   version (u32 LE)      4 bytes  (currently 1)    |
|   key_count (u32 LE)    4 bytes                   |
|   entry_count (u32 LE)  4 bytes  (informational)  |
|   build_ts (u32 LE)     4 bytes                   |
+---------------------------------------------------+
| Key offset table                                  |
|   key_count × u32 LE                              |
|   absolute byte offset to each key entry          |
+---------------------------------------------------+
| Key section (sorted by prev_word UTF-8 ascending) |
|   For each key:                                   |
|     prev_word_len  u8                             |
|     prev_word      prev_word_len bytes UTF-8      |
|     entry_offset   u32 LE   → into entry section  |
|     entry_count    u16 LE   → entries for this    |
|                              key                  |
+---------------------------------------------------+
| Entry section (each key's entries sorted          |
| by `count` DESC)                                  |
|   For each entry:                                 |
|     bitmask        u16 LE  (2 bytes)              |
|     count          u32 LE  (4 bytes)              |
|     next_word_len  u8                             |
|     next_tl_len    u8                             |
|     next_word      next_word_len bytes UTF-8      |
|     next_tl        next_tl_len bytes UTF-8        |
+---------------------------------------------------+
```

### 2.2 Lookup

`lookup(prevWord, limit=60)` does **byte-wise binary search** on the key offset table:

1. Binary search by raw UTF-8 byte comparison (matches the on-disk sort).
2. On hit, follow `entry_offset` → read up to `min(entry_count, limit)` entries.
3. Apply `passesFilter` per entry (caller's responsibility).

The byte-wise comparison is critical: any sort order divergence between build script and reader breaks lookups silently.

### 2.3 Constraints

- Keys MUST be sorted by raw UTF-8 byte order, ascending (NOT by Unicode code point order — same in this case for valid UTF-8, but the invariant is byte-order).
- Within a key group, entries SHOULD be sorted by `count` DESC; readers honour file order and apply `limit` as a count cap.
- `next_tl` UTF-8 invalid → entry skipped (reader continues with next entry).
- `next_word` UTF-8 invalid → entry skipped, but `pos` advances past `next_tl` to stay synchronized with the format.

### 2.4 Validation performed by reader

| Check | Reader behaviour on failure |
|---|---|
| File ≥ 20 bytes | `init?` returns `nil` |
| Magic == `TKWA` | `init?` returns `nil` |
| Version == 1 | `init?` returns `nil` |
| File ≥ `header + key_count × 4` | `init?` returns `nil` |
| `keyOffset < buffer.capacity()` | comparison returns `-1` (treated as key < target) |
| `keyStart + keyLen ≤ buffer.capacity()` | comparison returns `-1` |
| `metaPos + 6 ≤ capacity` | `readEntries` returns `[]` |
| `entryOffset ≤ capacity` | `readEntries` returns `[]` |
| Per-entry `pos + 8 ≤ capacity` | loop breaks |
| Per-entry `pos + nwLen + ntLen ≤ capacity` | loop breaks |

---

## 3. `dictionary.trie` — MARISA RecordTrie

### 3.1 Format

`dictionary.trie` is a **MARISA RecordTrie**: a prefix trie over keys with auxiliary `uint32` payload per key.

```
raw_key_in_trie = utf8_key + 0xFF + uint32_le(rowid)
```

The `0xFF` byte is the [Python `marisa_trie`](https://github.com/pytries/marisa-trie) `RecordTrie` separator. UTF-8 never produces a `0xFF` byte, so it is safe as a separator within keys that may contain arbitrary UTF-8.

### 3.2 Key prefixes

All trie keys carry one of three semantic prefixes (also UTF-8 ASCII, no separator collision):

| Prefix | Indexed against | Example |
|---|---|---|
| `tl:` | TL numeric, TL no-tone, TL abbreviation | `tl:hoo2boo5`, `tl:hooboo`, `tl:hb` |
| `poj:` | POJ numeric, POJ no-tone, POJ abbreviation | `poj:ho2bo5`, `poj:hobo`, `poj:hb` |
| `hanzi:` | hanzi (for reverse lookup, prefix search only) | `hanzi:好` |

**Invariant**: `LexiconService` chooses the prefix from `InputMode`. Callers MUST NOT prepend the prefix in `InputNormalizer.buildSearchKey()`; it is added at the `TrieService` boundary. (See `trie.md` §"Query Flow" for the canonical flow.)

### 3.3 Bridge layer

| Platform | Bridge | API style |
|---|---|---|
| iOS | `marisa_bridge.cpp` / `.h` | C, handle-based (`trie_handle_t`), supports up to 8 simultaneous tries |
| Android | `trie_jni.cpp` | JNI |

`extractRowId()` parses the `0xFF` separator and decodes the trailing `uint32_le`. Returns `-1` if the separator is absent or fewer than 4 trailing bytes — caller treats `-1` as "skip this entry".

### 3.4 Operations

| Op | Underlying MARISA call | Notes |
|---|---|---|
| `prefixSearch(prefix, limit)` | `predictive_search(query=prefix)` | Returns up to `limit` rowids whose key starts with `prefix`. |
| `lookup(key, limit)` | `predictive_search(query=key+0xFF)` | Filters results that match `key` exactly before the separator. |
| `getKeyCount()` | `num_keys()` | Total trie keys (incl. all prefix variants). |

---

## 4. Bitmask Layout (cross-platform invariant)

Bit positions are **shared** by `dictionary.bin`, `association.bin`, `EnabledDictionaries.sourceBitmask()`, and the build script.

```
bit  0  kautian      (教育部臺灣台語常用詞辭典)
bit  1  taigitv      (台語新詞辭庫)
bit  2  itaigi       (iTaigi 華台對照典)
bit  3  sitbut       (台灣植物名彙)
bit  4  taihoa       (台華線頂對照典)
bit  5  taijit       (台日大辭典)
bit  6  kungge       (台語工藝詞庫)
bit  7  stti         (學科術語辭典)
bit  8  khpoo        (腔口補充資料)
bit  9  khiin        (在來字)              ← filtered by exclusion layer
bit 10  dev          (always-included flag) ← never user-toggleable
bit 11  lkk          (LKK漢羅合用建議用字)
bit 12  is_variant   (異用字)              ← filtered by exclusion layer
bits 13–15  reserved
```

### 4.1 `association.bin` uses bits 0–8 only

`associationBitmask()` masks `sourceBitmask() & 0x1FF`. Bits 9–15 are not present in association entries.

### 4.2 Filter layers (`DictionaryBinaryReader.passesFilter`)

```
Layer 1 — Variant exclusion:   if !enabled.variant && record has bit 12 → reject
Layer 2 — Khiin exclusion:     if !enabled.khiin   && record has bit 9  → reject
Layer 3 — Source OR match:
    if enabled.allEnabled                                                → accept
    elif (record & enabledMask) != 0  || (record & devBit) != 0          → accept
    else                                                                 → reject
```

### 4.3 Filter layers (`AssociationBinaryReader.passesFilter`)

```
if enabled.allAssociationSourcesEnabled                                  → accept
elif enabledMask == 0                                                    → reject
elif (entry & enabledMask) != 0                                          → accept
else                                                                     → reject
```

Note: association filter does **not** apply variant/khiin/dev exclusions (those bits do not exist in association entries).

### 4.4 `allEnabled` semantics

| Concept | iOS | Android | Members |
|---|---|---|---|
| `allEnabled` | property | `allEnabled()` | 10 sources: kautian, taigitv, kungge, itaigi, taijit, taihoa, sitbut, stti, khpoo, **lkk** |
| `allAssociationSourcesEnabled` | property | `allAssociationSourcesEnabled()` | 9 sources: same minus **lkk** |

Excludes `variant`, `khiin`, `dev` from "all" — those are exclusion / always-on flags, not user sources.

---

## 5. Cross-Platform Invariants (drift hot list)

When ANY of the following changes, ALL listed files MUST be updated in the same commit:

| Invariant | Files that depend on it |
|---|---|
| `dictionary.bin` byte layout | build script, iOS `DictionaryBinaryReader.swift`, Android `DictionaryBinaryReader.kt`, this doc |
| `association.bin` byte layout | build script, iOS `AssociationBinaryReader.swift`, Android `AssociationBinaryReader.kt`, this doc |
| Bitmask bit positions | build script, iOS `EnabledDictionaries.swift`, Android `EnabledDictionaries.kt`, both `BinaryReader`s (filter constants), this doc |
| Trie key prefix list (`tl:` / `poj:` / `hanzi:`) | build script, `LexiconService` (both platforms), `InputNormalizer` (both), `DictionaryConstants.kt` (Android), `LexiconConstants.swift` (iOS), this doc, `trie.md` |
| RecordTrie separator (`0xFF`) | build script, `marisa_bridge.cpp`, `trie_jni.cpp`, this doc |
| Magic bytes (`TKDB` / `TKWA`) | build script, both `BinaryReader`s, this doc |
| File version (`1`) | build script, both `BinaryReader`s, this doc |
| Endianness (little-endian) | build script, both `BinaryReader`s |

### 5.1 No-checksum acknowledgement

None of the three formats carries a CRC or hash. Corruption surfaces as:

- Magic / version mismatch → reader fails to initialize (loud).
- Truncated file → reader fails to initialize (loud).
- Mid-file corruption inside a record → reader returns `null` for that record or skips that entry (silent — visible only as "missing word"; **no** hard failure).

If silent corruption ever becomes a real-world concern, add a header-level CRC32 over the offset table + record section. Out of scope for now.

### 5.2 Recommended startup assertion (debug only)

To catch build-script ↔ reader drift early, the app SHOULD assert at debug build startup that:

- `DictionaryBinaryReader.open()` returns non-nil.
- `AssociationBinaryReader.open()` returns non-nil.
- `TrieService.lookup("tl:tsit-ma")` (or any well-known sentinel key) returns at least one rowid, and the resulting record's TL begins with `tsit-ma`.

This cheap round-trip catches every drift category above except bit-layout swaps among already-set bits.

---

## 6. Build Pipeline

The Python build pipeline lives at `dictionary/build/`. Steps relevant to the formats described above (post-v3.5.6 part 2 — every binary writer reads `dictionary.csv` directly via `dictionary_records.py` / `associations.py`; SQLite intermediates removed):

| Step | Output | Notes |
|---|---|---|
| `merge_csv.py` | `dictionary.csv` | merged per-source CSVs + khiin/dev/lkk supplements |
| `dictionary_records.py` | (in-memory) | filtered records + rowid 1..N — shared by `create_dictionary_bin` + `create_fst` |
| `associations.py` | (in-memory) | bigram + char-to-phrase generator — shared by `create_association_bin` |
| `create_dictionary_bin.py` | `dictionary.bin` | TKDB format per §1; writes shared `.build_ts` |
| `create_fst.py` | `dictionary.fst` | shells to `engine/build-helpers/fst-builder` (Rust) for fst encoding |
| `create_association_bin.py` | `association.bin` | TKWA format per §2; reads shared `.build_ts` |
| `audit.py` | `audit_report.txt` + `audit/*.csv` | sanity checks against `dictionary.csv` |
| `deploy.sh` | bundles into platform asset directories | iOS bundle + Android assets |

The build pipeline must:

1. Sort `association.bin` keys by raw UTF-8 byte order ascending.
2. Sort each association key's entries by `count` DESC.
3. Emit fst with `0xFF` separator and `uint32_le` rowid payload (`engine/build-helpers/fst-builder`).
4. Use bit positions exactly per §4.
5. Set magic bytes per §1, §2.
6. Use version `1` for both `.bin` files.
7. Include all six trie key forms (TL num/no-tone/abbrev, POJ num/no-tone/abbrev) plus `hanzi:` keys for reverse lookup.

---

## 7. Test Coverage Today

| Format | iOS test | Android test |
|---|---|---|
| `dictionary.bin` (content) | `DictionaryContentTests` (counts) | `DictionaryCoverageTest` (counts) |
| `dictionary.bin` (parser) | — | — |
| `association.bin` | — | — |
| `dictionary.trie` | — (only via integration) | — (only via integration) |
| Bitmask filter | — | — |

§6 of `lexicon-refactor-plan.md` lists these as Stage-0 characterization-test targets.
