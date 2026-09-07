# Binary Asset Formats

> **Type**: Reference
> **Keywords**: `dictionary.bin`, `association.bin`, `dictionary.fst`, `fst`, `bitmask`, `mmap`, `cross-platform invariant`
> **Related**: `nextword.md`, `custom-dictionary.md`, `engine/lexicon`
> **Audience**: anyone touching the Rust `engine/lexicon` readers (`DictionaryReader`, `AssociationReader`, `PrefixIndex`), the filter logic (`engine/lexicon::dictionary_filters`) + platform `DictionarySource` DTO, or the Python build script (`dictionary/build/`).

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
**Source-of-truth** for *content*: the Python build pipeline at `dictionary/build/` (`merge_csv.py` → `create_dictionary_bin.py` → `create_fst.py` → `create_association_bin.py` → `verify_poj_integrity.py` → `version_snapshot.py` → `deploy.sh`; the binary writers read `dictionary.csv` directly via `dictionary_records.py` / `associations.py`, while `create_fst.py` shells to the Rust `engine/build-helpers/fst-builder`). When any step changes the binary layout, this document and both readers must be updated **in the same change set**.

---

## 1. `dictionary.bin` — Dictionary Records

### 1.1 File layout

```
+---------------------------------------------------+
| Header (16 bytes)                                 |
|   "TKDB"                4 bytes                   |
|   version (u32 LE)      4 bytes  (currently 3)    |
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
|   syllable_count u8      (1 byte; v2; 1..=4)      |
|   kautian_subtag u16 LE  (2 bytes; v3)           |
|   hanzi          hanzi_len bytes UTF-8            |
|   tl             tl_len   bytes UTF-8             |
+---------------------------------------------------+
```

**v1 → v2 (v3.5.8 Phase 1)**: added per-record `syllable_count` u8 between
`tl_len` and the `hanzi` payload. Used by Phase 5 span-local candidate
ranking to disambiguate same-toneless-key entries with different syllable
counts (e.g. `tsua` → `紙` (syll=1) vs `珠仔` (syll=2)). Range is
`1..=4` (capped by `MAX_SYLLABLES` in
`dictionary/build/dictionary_records.py`); `0` is reserved.

**v2 → v3 (kautian subcollections Phase 2)**: added per-record
`kautian_subtag` u16 between `syllable_count` and the `hanzi` payload.
Records which kautian subcollection(s) a row belongs to (main / accent[10] /
name) so the engine filter can independently gate them while the kautian
source bit (bit 0) stays a single badge/ranking signal. `0` for every
non-kautian row. Layout in §4.5. Reserved bits 12-15 are masked off on read.

Each reader supports exactly ONE version; it will NOT parse older binaries
— rebuild + redeploy artifacts in lockstep (see `dictionary/build/deploy.sh`).

### 1.2 Record access

`record(rowId)` is **1-based**. `rowId == 0` returns `null`/`nil`.

The end of a record is determined by the *next* record's offset (or `data.count` for the last record). Readers MUST validate `recordEnd > recordOffset` and `recordEnd <= data.count` before parsing.

### 1.3 Constraints

- `tl_len > 0` for every record (TL is required; hanzi may be absent).
- `hanzi == ""` is encoded as `hanzi_len == 0` and yields `None` in the reader, **not** an empty string.
- UTF-8 must be valid; the Rust reader returns `None` for the whole record on `from_utf8` failure for either `tl` or `hanzi` (when `hanzi_len > 0`). Pre-Phase IV-B platform readers tolerated bad `hanzi` bytes by setting `hanzi = None` and keeping the record; the Rust reader is stricter.
- Record bytes are not aligned; the Rust reader uses unaligned little-endian reads via `byteorder::LE`.

### 1.4 Validation performed by reader (Rust `engine/lexicon::dictionary_reader`)

| Check | Reader behaviour on failure |
|---|---|
| File ≥ 16 bytes | `open` returns `Err(LexiconError::InvalidBinary)` |
| Magic == `TKDB` | `open` returns `Err(LexiconError::InvalidBinary)` |
| Version == 3 (v1/v2 surface explicit `v1/v2→v3` rebuild guidance) | `open` returns `Err(LexiconError::InvalidBinary)` |
| File ≥ `header + record_count × 4` | `open` returns `Err(LexiconError::InvalidBinary)` |
| Per-record bounds (`recordEnd ≤ data.len()`) | `record()` returns `None` |
| Per-record min size 11 bytes (v3 fixed prefix) | `record()` returns `None` |
| `pos + hanzi_len + tl_len ≤ record_end` | `record()` returns `None` |
| TL UTF-8 valid | `record()` returns `None` |

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

### 2.4 Validation performed by reader (Rust `engine/lexicon::association_reader`)

| Check | Reader behaviour on failure |
|---|---|
| File ≥ 20 bytes | `open` returns `Err(LexiconError::InvalidBinary)` |
| Magic == `TKWA` | `open` returns `Err(LexiconError::InvalidBinary)` |
| Version == 1 | `open` returns `Err(LexiconError::InvalidBinary)` |
| File ≥ `header + key_count × 4` | `open` returns `Err(LexiconError::InvalidBinary)` |
| `key_offset < buffer.len()` | binary search treats key as < target |
| `key_start + key_len ≤ buffer.len()` | binary search treats key as < target |
| `meta_pos + 6 ≤ capacity` | `read_entries` returns empty `Vec` |
| `entry_offset ≤ capacity` | `read_entries` returns empty `Vec` |
| Per-entry `pos + 8 ≤ capacity` | loop terminates |
| Per-entry `pos + nw_len + nt_len ≤ capacity` | loop terminates |

---

## 3. `dictionary.fst` — Burntsushi FST Prefix Index

### 3.1 Format

`dictionary.fst` is a Burntsushi [`fst`](https://github.com/BurntSushi/fst) Set. Each entry is a single byte sequence:

```
key_bytes (UTF-8)  ||  0xFF separator  ||  rowid_le_4 (u32 little-endian)
```

The `0xFF` separator is safe inside otherwise-UTF-8 keys (UTF-8 never produces a `0xFF` byte). Multiple rowids per key are encoded as multiple distinct entries sharing the `key + 0xFF` prefix; per-key insertion order is preserved through fst's deterministic byte-sorted iteration plus a stable sort over `(key, rowid)` at build time.

The fst is built offline by the Rust binary `engine/build-helpers/fst-builder` (invoked from Python `create_fst.py`) and consumed by Rust `engine/lexicon::prefix_index::PrefixIndex` via `mmap-host` + `fst::Set`. Replaces the historical MARISA RecordTrie (v3.5.6, PR #199).

### 3.2 Key prefixes

Logical keys (the `key_bytes` part before the separator) carry one of three semantic prefixes (UTF-8 ASCII):

| Prefix | Indexed against | Example logical key |
|---|---|---|
| `tl:` | TL numeric, TL no-tone, TL abbreviation | `tl:hoo2boo5`, `tl:hooboo`, `tl:hb` |
| `poj:` | POJ numeric, POJ no-tone, POJ abbreviation | `poj:ho2bo5`, `poj:hobo`, `poj:hb` |
| `hanzi:` | hanzi (for reverse lookup, prefix search only) | `hanzi:好` |

**Invariant**: the prefix is added by `lexicon::key_normalizer::build` based on `(KeyType, KeyMode)` at the engine seam. Callers (platform classifiers, `lexicon::search`) MUST NOT prepend the prefix themselves.

### 3.3 Reader & host crate

| Concern | Location |
|---|---|
| fst load + range/prefix search | Rust `engine/lexicon::prefix_index::PrefixIndex` |
| mmap unsafe boundary | Rust `engine/mmap-host::MmapHandle` (only crate not `forbid unsafe_code`) |
| Lookup orchestration | Rust `engine/lexicon::search::search` |
| Platform bridge | `RustEngineBridge.search` / `searchByHanzi` / `searchWithSources` (iOS `RustEngineBridge+Lexicon.swift`, Android `LexiconBridge.kt`) |

There is no on-device build pathway — the fst is a read-only asset, one byte-identical copy per platform, written by `dictionary/build/deploy.sh`.

### 3.4 Operations

| Op | Call | Notes |
|---|---|---|
| Prefix range scan | `Set::range().ge(prefix_bytes).lt(next_lex_sibling(prefix_bytes))` | Iterates every wire entry whose logical key starts with the prefix; trailing 4 bytes are decoded as `u32` little-endian rowid. Returns `Vec<u32>` in fst byte-sort order. |
| Exact lookup | range scan over `[key + 0xFF, key + 0x100)` | Returns all rowids stored against `key` — multiple rowids per logical key are supported via repeated entries. |
| Key count | `Set::len()` | Total fst entries (NOT distinct logical keys; each (key, rowid) pair is one entry). |

---

## 4. Bitmask Layout (cross-platform invariant)

Bit positions are **shared** by `dictionary.bin`, `association.bin`, the filter logic (`engine/lexicon::dictionary_filters`), and the build script (`dictionary/common/source_bits.py`).

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
bit 10  dev          (詞庫增補檔案 — user-toggleable, default on)
bit 11  lkk          (LKK漢羅合用建議用字)
bit 12  is_variant   (異用字)              ← filtered by exclusion layer
bits 13–15  reserved
```

### 4.1 `association.bin` uses bits 0–8 only

`associationBitmask()` masks `sourceBitmask() & 0x1FF`. Bits 9–15 are not present in association entries.

### 4.2 Filter layers (`engine/lexicon::dictionary_reader::Filter`)

```
Layer 1 — Variant exclusion:   if !enabled.variant && record has bit 12 → reject
Layer 2 — Khiin exclusion:     if !enabled.khiin   && record has bit 9  → reject
Layer 3 — kautian subcollection gate (v3, see §4.5):
    effective = effective_source_bitmask(record_bitmask, record_subtag, enabled)
      = record_bitmask, EXCEPT a kautian-source row whose subcollections are
        all disabled has its bit 0 cleared (other source bits untouched).
Layer 4 — Source OR match (on the EFFECTIVE bitmask):
    if enabled.all_enabled                                               → accept
    elif (effective & enabled_mask) != 0                                 → accept
    else                                                                 → reject
```
dev (bit 10, 詞庫增補檔案) rides `enabled_mask` like any other source
(default on, user-toggleable). It used to be an unconditional `|| DEV_BIT`
floor in Layer 4; the 詞庫增補檔案 toggle made it a normal source.

The same `effective_source_bitmask` is emitted as the candidate's
`source_bitmask` so a multi-source survivor ranks by its other source's tier,
not kautian's (DD6 ranking-weight drop).

### 4.3 Filter layers (`engine/lexicon::association_reader::AssocFilter`)

```
if enabled.all_association_sources_enabled                               → accept
elif enabled_mask == 0                                                   → reject
elif (entry & enabled_mask) != 0                                         → accept
else                                                                     → reject
```

Note: association filter does **not** apply variant/khiin exclusions (those bits do not exist in association entries); dev is not an association source either.

### 4.4 `allEnabled` semantics

| Concept | iOS | Android | Members |
|---|---|---|---|
| `allEnabled` | property | `allEnabled()` | 10 sources: kautian, taigitv, kungge, itaigi, taijit, taihoa, sitbut, stti, khpoo, **lkk** |
| `allAssociationSourcesEnabled` | property | `allAssociationSourcesEnabled()` | 9 sources: same minus **lkk** |

Excludes `variant`, `khiin` from "all" — those are exclusion flags, not main sources. `dev` (詞庫增補檔案) is a user-toggleable source (default on), carried in the source-OR like the other sources.

### 4.5 `kautian_subtag` (v3) + wire subcollection-enable bits

The per-record `kautian_subtag` u16 (dictionary.bin §1.1) and the user's
subcollection-enable bits in `enabled_sources_bitmask` (the wire field on
`SearchRequest` / Tab3 / continuous) share ONE 12-bit layout so the filter
test is a single AND:

```
subcollection bit layout (12 bits):
  bit  0      main         (主條目 / headword)
  bits 1..=10 accent[0..9] (語音差異 — config.yaml dialect_columns order:
                            0 鹿港 1 三峽 2 臺北 3 宜蘭 4 臺南 5 高雄
                            6 金門 7 馬公 8 新竹 9 臺中)
  bit  11     name         (姓名附錄 — 名 + 姓)
  bits 12-15  reserved (record subtag masks these off on read)
```

**Storage** (`dictionary.bin` record `kautian_subtag`): which subcollections a
row belongs to. `0` for non-kautian rows. A row may carry multiple classes
(e.g. a headword that is also an accent reading = main + accent bits).

**Wire** (`enabled_sources_bitmask` high region): which subcollections the user
has enabled.

```
  bit  13      KAUTIAN_SUBCOLL_ACTIVE — control sentinel. 0 ⇒ engine SKIPS the
               subcollection gate entirely (legacy / pre-UI default = all on,
               zero behaviour change). A platform that has the toggles sets this.
  bits 14..=25 subcollection enable mask, SAME 12-bit layout as the subtag.
  bits 26-31   reserved (unknown high bits ignored for forward-compat).
```

`u32::MAX` (the all-enabled sentinel) carries bit 13 + every enable bit set, so
"all sources/subcollections on" stays consistent. Owner of the bit positions:
`dictionary/common/source_bits.py::encode_kautian_subtag` +
`engine/lexicon/src/dictionary_reader.rs` (`KAUTIAN_SUBTAG_*` / `WIRE_KAUTIAN_SUBCOLL_*`).

The toggle→wire ENCODE landed in Phase 3 (iOS): `compute_filters` sets bit 13 +
the enable mask from `DictionaryToggles.kautian_subcoll` (a nested message —
PRESENCE is the active sentinel). iOS always sends it (it ships the toggles);
a caller that leaves it absent (Android until its UI phase, NextWord) keeps the
gate off = legacy all-on. The subcollection-enable filtering applies to the
Tab3 dictionary-browse path; the keyboard continuous path still pins
`u32::MAX` (no per-source gating there — separate deferred plumbing).

---

## 5. Cross-Platform Invariants (drift hot list)

When ANY of the following changes, ALL listed files MUST be updated in the same commit:

| Invariant | Files that depend on it |
|---|---|
| `dictionary.bin` byte layout | build script, Rust `engine/lexicon::dictionary_reader`, this doc |
| `association.bin` byte layout | build script, Rust `engine/lexicon::association_reader`, this doc |
| Bitmask bit positions | build script (`dictionary/common/source_bits.py`), Rust `engine/lexicon::dictionary_filters`, platform `DictionarySource` DTO, this doc |
| `kautian_subtag` + wire subcollection-enable bit layout (§4.5) | `dictionary/common/source_bits.py` (`encode_kautian_subtag`), `dictionary/build/create_dictionary_bin.py`, Rust `engine/lexicon::dictionary_reader` (`KAUTIAN_SUBTAG_*` / `WIRE_KAUTIAN_SUBCOLL_*`), this doc |
| Key prefix list (`tl:` / `poj:` / `hanzi:`) | build script (`create_fst.py`), Rust `lexicon::key_normalizer`, this doc |
| Magic bytes (`TKDB` / `TKWA`) | build script, Rust readers, this doc |
| File version (`dictionary.bin = 3`, `association.bin = 1`) | build script, Rust readers, this doc |
| Endianness (little-endian) | build script, Rust readers |

### 5.1 No-checksum acknowledgement

None of the three formats carries a CRC or hash. Corruption surfaces as:

- Magic / version mismatch → reader fails to initialize (loud).
- Truncated file → reader fails to initialize (loud).
- Mid-file corruption inside a record → reader returns `null` for that record or skips that entry (silent — visible only as "missing word"; **no** hard failure).

If silent corruption ever becomes a real-world concern, add a header-level CRC32 over the offset table + record section. Out of scope for now.

### 5.2 Recommended startup assertion (debug only)

To catch build-script ↔ reader drift early, the engine `LexiconService.install()` (called once at app start via `RustEngineBridge.install`) returns `dictionary_record_count` + `prefix_index_entry_count`; both platforms should assert these are non-zero and equal across builds. A debug-build round-trip lookup of a well-known sentinel key (e.g. `tl:tsit-ma`) via `RustEngineBridge.search` should also return at least one record whose TL begins with `tsit-ma`.

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
| `verify_poj_integrity.py` | (exit code) | fatal gate: halts build if `poj`/derived ≠ `convert_tl_to_poj(tl)` |
| `version_snapshot.py` | drop/diff summary (stdout + `output/version_diff.txt`) | build-drop + `(hanzi, tl)` diff vs previous release tag's `dictionary.csv` (read via `git show`; no file stored) |
| `deploy.sh` | bundles into platform asset directories | iOS bundle + Android assets |

The build pipeline must:

1. Sort `association.bin` keys by raw UTF-8 byte order ascending.
2. Sort each association key's entries by `count` DESC.
3. Emit fst via `engine/build-helpers/fst-builder` — keys carry the prefix (`tl:` / `poj:` / `hanzi:`) and the value packs rowid in the low 32 bits.
4. Use bit positions exactly per §4.
5. Set magic bytes per §1, §2.
6. Use version `3` for `dictionary.bin` (v2 added per-record `syllable_count`; v3 added per-record `kautian_subtag`) and version `1` for `association.bin`.
7. Include all six trie key forms (TL num/no-tone/abbrev, POJ num/no-tone/abbrev) plus `hanzi:` keys for reverse lookup.

---

## 7. Test Coverage Today

| Format | Test |
|---|---|
| `dictionary.bin` (content count) | iOS `DictionaryContentTests` (no Android counterpart) |
| `dictionary.bin` (parser) | Rust `engine/lexicon/tests/parity.rs` |
| `association.bin` | Rust `engine/lexicon/tests/parity.rs` |
| `dictionary.fst` | Rust `engine/lexicon/tests/parity.rs` (round-trip a sentinel key set) |
| Bitmask filter | Rust `engine/lexicon::dictionary_reader` unit tests + integration via `parity.rs` |

The Rust `parity.rs` test suite is the canonical check; platform tests cover content/coverage at the asset bundle level only.
