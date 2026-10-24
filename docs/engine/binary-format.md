# Binary Asset Formats

> **Type**: Reference
> **Keywords**: `dictionary.bin`, `association.bin`, `dictionary.fst`, `fst`, `bitmask`, `mmap`, `cross-platform invariant`
> **Related**: `nextword.md`, `custom-dictionary.md`, `engine/lexicon`
> **Audience**: anyone touching the Rust `engine/lexicon` readers (`DictionaryReader`, `AssociationReader`, `PrefixIndex`), the filter logic (`engine/lexicon::dictionary_filters`) + platform `DictionarySource` DTO, or the Python build script (`dictionary/build/`).

---

## Summary

Four read-only binary assets live in the dictionary bundle (`assets/dictionaries/`) and are mmap-loaded by the shared Rust `engine/lexicon` readers on every platform — iOS, Android, and the three desktops (macOS, Windows, Linux) through `desktop/crates/taigi-desktop-core/src/dictionary_artifacts.rs`:

| File | Magic | Purpose | Size |
|---|---|---|---|
| `dictionary.bin` | `TKDB` | rowid → {bitmask, frequency, hanzi, tl, walker_cost} | ~5.8 MB |
| `association.bin` | `TKWA` | prev_word → list of next-word predictions | ~3.1 MB |
| `dictionary.fst` | (fst) | prefix key → rowid (for prefix + exact lookup) | ~9.2 MB |
| `syllables.fst` | (fst) | valid-syllable inventory per romanization family (§3.5) | small |

All formats use **little-endian** integers and **strict UTF-8** strings. The Rust reader is the only reader; the Python writer and the Rust reader must agree byte-for-byte, and mismatches surface as silent decode failures.

**Source-of-truth** for layout: this document.
**Source-of-truth** for *content*: the Python build pipeline at `dictionary/build/` ([`dictionary/README.md` § Pipeline](../../dictionary/README.md#pipeline)). When any step changes the binary layout, this document, the Python writer and the Rust reader must be updated **in the same change set**.

---

## 1. `dictionary.bin` — Dictionary Records

### 1.1 File layout

```
+---------------------------------------------------+
| Header (40 bytes)                                 |
|   "TKDB"                4 bytes                   |
|   version (u32 LE)      4 bytes  (currently 5)    |
|   record_count (u32 LE) 4 bytes                   |
|   build_ts (u32 LE)     4 bytes  (build id)       |
|   tokens_N (u64 LE)     8 bytes                  |
|   vocabulary_V (u64 LE) 8 bytes                  |
|   alpha (f64 LE)        8 bytes                  |
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
|   walker_cost    u16 LE  (2 bytes; v4)           |
|   hanzi          hanzi_len bytes UTF-8            |
|   tl             tl_len   bytes UTF-8             |
+---------------------------------------------------+
```

`syllable_count` is `1..=4` (capped by `MAX_SYLLABLES` in `dictionary/build/dictionary_records.py`); `0` is reserved. It separates same-toneless-key entries (`tsua` → 紙 (1) vs 珠仔 (2)).

`kautian_subtag` records which kautian subcollection(s) a row belongs to; `0` for every non-kautian row. Layout in §4.5.

`walker_cost` is the walker model's `−ln p` of the record's `(hanzi, tl_num)` in milli-nats, rounded half to even (`dictionary/build/walker_lm.py`: Lidstone add-α over the segmented corpus counts in `dictionary/shared/data/word_unigrams.tsv`). Records sharing a `(hanzi, tl_num)` share the value; a word with no corpus count carries the smoothed unseen value. The writer fails the build on a value above `u16::MAX`. Model parameters and the cost distribution of a build: `dictionary/output/walker_lm_stats.txt`. `frequency` still feeds the sort's tie-breaks ([continuous-input-ranking.md §3.2](continuous-input-ranking.md#32-user-weight-and-sort-key)).

Each reader supports exactly ONE version; it will NOT parse older binaries
— rebuild + redeploy artifacts in lockstep (see `dictionary/build/deploy.sh`).

Version 5 adds the model metadata at byte offsets 16 (`N`), 24 (`V`), and
32 (`α`); the v4 record layout is unchanged. The writer takes the metadata
and every record cost from the same `WalkerModel` instance. The Rust reader
validates the metadata on open and keeps it with that reader, so replacing a
dictionary also replaces its model without an engine rebuild.

The engine retains the deliberate effective user count `2_000` as a policy
parameter, and derives its custom/learned-word cost from the loaded model
once per walk: `round_half_even(−ln((2_000 + α)/(N + αV)) × 1000)`.
The same value caps user-row spans and selected multi-syllable words. An
effective count exceeding the model's normalizer is assigned probability 1
(cost 0); no negative probability cost is produced for a small dictionary.
No compiled model price or compatibility default is used.

### 1.2 Record access

`record(rowId)` is **1-based**. `rowId == 0` returns `null`/`nil`.

The end of a record is determined by the *next* record's offset (or `data.count` for the last record). Readers MUST validate `recordEnd > recordOffset` and `recordEnd <= data.count` before parsing.

### 1.3 Constraints

- `tl_len > 0` for every record (TL is required; hanzi may be absent).
- `hanzi == ""` is encoded as `hanzi_len == 0` and yields `None` in the reader, **not** an empty string.
- UTF-8 must be valid; the Rust reader returns `None` for the whole record on `from_utf8` failure for either `tl` or `hanzi` (when `hanzi_len > 0`).
- Record bytes are not aligned; the Rust reader uses unaligned little-endian reads via `byteorder::LE`.

### 1.4 Validation performed by reader (Rust `engine/lexicon::dictionary_reader`)

| Check | Reader behaviour on failure |
|---|---|
| File ≥ 16 bytes for common header; ≥ 40 bytes for v5 metadata | `open` returns `Err(LexiconError::InvalidBinary)` |
| Magic == `TKDB` | `open` returns `Err(LexiconError::InvalidBinary)` |
| Version == 5 (v1–v4 surface explicit `v1/v2/v3/v4→v5` rebuild guidance) | `open` returns `Err(LexiconError::InvalidBinary)` |
| V > 0; α finite and positive; N + αV finite; unseen cost finite and representable as u16 | `open` returns `Err(LexiconError::InvalidBinary)` |
| File ≥ `header + record_count × 4` | `open` returns `Err(LexiconError::InvalidBinary)` |
| Per-record bounds (starts after header + offset table, `recordEnd ≤ data.len()`) | `record()` returns `None` |
| Per-record min size 13 bytes (v4 fixed prefix) | `record()` returns `None` |
| `pos + hanzi_len + tl_len ≤ record_end` | `record()` returns `None` |
| TL UTF-8 valid | `record()` returns `None` |

---

## 2. `association.bin` — Bigram Predictions

### 2.1 File layout

```
+---------------------------------------------------+
| Header (20 bytes)                                 |
|   "TKWA"                4 bytes                   |
|   version (u32 LE)      4 bytes  (currently 2)    |
|   key_count (u32 LE)    4 bytes                   |
|   entry_count (u32 LE)  4 bytes  (informational)  |
|   build_ts (u32 LE)     4 bytes                   |
+---------------------------------------------------+
| Key offset table                                  |
|   key_count × u32 LE                              |
|   absolute byte offset to each key entry          |
+---------------------------------------------------+
| Key section (sorted by raw prev_word UTF-8 bytes) |
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

### 2.1a Key namespaces (v2)

One byte-sorted key section holds two kinds of key; the reader needs no special casing because `\u{1}` never occurs inside a hanji or TL string, so the namespaces cannot collide:

| Key | Built by | Entries | `count` |
|---|---|---|---|
| one hanji (v1 shape) | `build/associations.py::compute_associations` | intra-word pairs from 2–5-character dictionary words (`dictionary.csv`) | dictionary frequency |
| `hanji\u{1}tl` — display-form TL, `\u{1}` separator (`association_reader::word_key`) | `build/associations.py::compute_word_associations` | corpus continuations (`shared/data/word_bigrams.tsv`, P2 of the bigram LM roadmap), variant hanji folded to the 教典 form via `variants.csv`, top 30 per key, weighted count ≥ 2 | weighted corpus pair count (`taigi_bible_nt` × 0.5, others × 1) |

`bitmask` is the source flags of the **next** word's dictionary row in every namespace, so `AssociationFilter` (§4.3) filters both alike. v1 files (character keys only) are rejected by the reader with a `v1→v2` rebuild hint — the two artifacts ship in lockstep (`docs/architecture/bigram-lm-roadmap.md` § D3).

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
| Version == 2 | `open` returns `Err(LexiconError::InvalidBinary)` (v1 → message carries the `v1→v2` rebuild hint) |
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

The fst is built offline by the Rust binary `engine/build-helpers/fst-builder` (invoked from Python `create_fst.py`) and consumed by Rust `engine/lexicon::prefix_index::PrefixIndex` via `mmap-host` + `fst::Set`.

### 3.2 Key prefixes

Logical keys (the `key_bytes` part before the separator) carry one of these semantic prefixes (UTF-8 ASCII):

| Prefix | Indexed against | Example logical key |
|---|---|---|
| `tl:` | TL numeric, TL no-tone | `tl:hoo2boo5`, `tl:hooboo` |
| `poj:` | POJ numeric, POJ no-tone | `poj:ho2bo5`, `poj:hobo` |
| `tps:` | TPS with tone marks, TPS no-tone (+ er↔or variant) | `tps:ㄏㆦ˫ㄅㆦˊ`, `tps:ㄏㆦㄅㆦ` |
| `tl-abbrev:` / `poj-abbrev:` / `tps-abbrev:` | the per-syllable abbreviation face of the row (`tl_abbrev` / `poj_abbrev` / `tps_abbrev` + variant) | `tl-abbrev:hb`, `poj-abbrev:hb`, `tps-abbrev:ㄏㄅ` |
| `hanzi:` | hanzi (for reverse lookup, prefix search only) | `hanzi:好` |

The abbreviation face has its own family per romanization (`behavioral-invariants.md` §46) so a prefix scan over a phonetic family never meets an acronym key: the continuous-input hydration needs no acronym heuristics, `lexicon::fetch_abbrev_candidates` reads the abbreviation family alone, and Tab3 `lexicon::search` unions both families to keep acronym search. `phonetics::abbrev_family_key` derives the twin key.

**Invariant**: the prefix comes from `phonetics::KeyFamily` (`prefix()` / `search_key()`, `HANJI_KEY_PREFIX`) at the engine seam. Callers (platform classifiers, `lexicon::search`, `composing::shadow`) MUST NOT spell the prefix themselves.

### 3.3 Reader & host crate

| Concern | Location |
|---|---|
| fst load + range/prefix search | Rust `engine/lexicon::prefix_index::PrefixIndex` |
| mmap unsafe boundary | Rust `engine/mmap-host::MmapHandle` (only crate not `forbid unsafe_code`) |
| Lookup orchestration | Rust `engine/lexicon::search::search` |
| Platform bridge (dictionary search page) | iOS `RustEngineBridge.lexiconSearchWithSources` / `lexiconSearchByHanji` (`RustEngineBridge+Lexicon.swift`); Android `RustEngineBridge.searchWithSources` / `searchByHanji` (`LexiconBridge.kt`) |

There is no on-device build pathway — the fst is a read-only asset, one byte-identical copy per platform, written by `dictionary/build/deploy.sh`.

### 3.4 Operations

| Op | Call | Notes |
|---|---|---|
| Prefix range scan | `Set::range().ge(prefix_bytes).lt(next_lex_sibling(prefix_bytes))` | Iterates every wire entry whose logical key starts with the prefix; trailing 4 bytes are decoded as `u32` little-endian rowid. Returns `Vec<u32>` in fst byte-sort order. |
| Exact lookup | range scan over `[key + 0xFF, key + 0x100)` | Returns all rowids stored against `key` — multiple rowids per logical key are supported via repeated entries. |
| Key count | `Set::len()` | Total fst entries (NOT distinct logical keys; each (key, rowid) pair is one entry). |

---

## 3.5 `syllables.fst` — Syllable Inventory

A Burntsushi `fst::Set` of every phonotactically valid syllable, one key per entry (no rowid), opened by `engine/lexicon::syllable_inventory::SyllableInventory`. The syllabifier asks it whether a span is a syllable (`SyllableInventory::contains_in(mode, syllable)`); it never carries words.

| Family | Key forms per syllable | Canonicalizer (build and lookup) |
|---|---|---|
| `tl:` | numeric + toneless (`tl:tsua7`, `tl:tsua`) | `phonetics::canonicalize_syllable` (POJ source spellings fold to TL) |
| `poj:` | numeric + toneless (`poj:choa7`, `poj:choa`) | `phonetics::canonicalize_poj_syllable` (POJ ASCII spelling kept) |
| `tps:` | tone-marked + toneless Bopomofo | `phonetics::canonicalize_tps_syllable` |

The prefix comes from `phonetics::KeyFamily::for_input_mode(mode)`; English input uses the `tl:` family. Why TL and POJ are separate families: [`syllabifier.md` § Family-split rationale](syllabifier.md#family-split-rationale).

Built by `dictionary/build/create_syllables_fst.py`, which stages the dictionary's `tl_num` / `poj_num` strings and pre-split TPS syllables and runs `fst-builder build-syllables <output.fst> --tl-input … --poj-input … --tps-input …` (`engine/build-helpers/fst-builder/src/syllables.rs`). TL / POJ lines are split at ASCII tone digits; a token without a closing tone digit aborts the build, while a phonotactically invalid syllable is counted, sampled to stderr and skipped. The reader validates only that the bytes parse as an fst. The desktops treat a missing or empty `syllables.fst` as a corrupt install.

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
bit  8  khpoo        (accent supplement data)
bit  9  khiin        (Conventional Characters) ← filtered by exclusion layer
bit 10  dev          (Supplementary Word List — user-toggleable, default on)
bit 11  lkk          (LKK漢羅合用建議用字)
bit 12  is_variant   (Variant Characters)      ← filtered by exclusion layer
bits 13–15  reserved
```

### 4.1 `association.bin` uses bits 0–8 only

Association entries carry the next word's source bits 0–8 only; the filter masks the wire value with `0x1FF` (§4.3). Bits 9–15 are not present in association entries.

### 4.2 Filter layers (`engine/lexicon::dictionary_reader::Filter`)

```
Layer 1 — Variant exclusion:   if !enabled.variant && record has bit 12 → reject
Layer 2 — Khiin exclusion:     if !enabled.khiin   && record has bit 9  → reject
Layer 3 — kautian subcollection gate (v3, see §4.5):
    effective = effective_source_bitmask(record_bitmask, record_subtag, enabled)
      = record_bitmask, EXCEPT a kautian-source row whose subcollections are
        all disabled has its bit 0 cleared (other source bits untouched).
Layer 4 — Source OR match (on the EFFECTIVE bitmask):
    if wire mask == u32::MAX (all_enabled)                               → accept
    elif (effective & enabled_mask) != 0                                 → accept
    else                                                                 → reject
```
dev (bit 10, Supplementary Word List) rides `enabled_mask` like any other source
(default on, user-toggleable). `Filter::from_enabled_bitmask` decodes the wire
`enabled_sources_bitmask`; variant and khiin ride bits 12 and 9 of it.

The same `effective_source_bitmask` is emitted as the candidate's
`source_bitmask` so a multi-source survivor ranks by its other source's tier,
not kautian's (DD6 ranking-weight drop).

### 4.3 Filter layers (`engine/lexicon::association_reader::AssociationFilter`)

The engine filters associations itself (`AssociationReader::lookup` applies the filter before the `limit` cut, so a key whose top entries come from disabled sources still yields its enabled ones):

```
AssociationFilter::from_sources_bitmask(wire):
    all_enabled  = wire == u32::MAX
    enabled_mask = wire & 0x1FF            (bits 0-8)
passes(entry) = all_enabled || (entry & enabled_mask) != 0
```

The wire mask comes from `dictionary_filters::association_bitmask`, which answers the `u32::MAX` sentinel when all nine association sources are on, else their toggled bits. No variant / khiin exclusion applies (those bits are not in association entries); dev, khiin and lkk are not association sources.

### 4.4 "All enabled" sentinel

There is no platform `allEnabled` flag. `u32::MAX` on the wire means "every source on" for both filters (`Filter::all_enabled`, `AssociationFilter::all_enabled`). `dictionary_filters::dictionary_filter_bitmask` always sends the real per-source bits (12 sources, plus variant bit 12 and the §4.5 subcollection bits); `association_bitmask` sends the sentinel when all nine association sources (bits 0–8) are on.

### 4.5 `kautian_subtag` (v3) + wire subcollection-enable bits

The per-record `kautian_subtag` u16 (dictionary.bin §1.1) and the user's
subcollection-enable bits in `enabled_sources_bitmask` (the wire field on
`SearchWithSourcesRequest` / `SearchByHanjiRequest` / continuous) share ONE 13-bit layout so the filter
test is a single AND:

```
subcollection bit layout (13 bits):
  bit  0      main         (headword)
  bits 1..=10 accent[0..9] (accent variants — config.yaml dialect_columns order:
                            0 鹿港 1 三峽 2 臺北 3 宜蘭 4 臺南 5 高雄
                            6 金門 7 馬公 8 新竹 9 臺中)
  bit  11     name         (name appendix — given + family names)
  bit  12     alt_reading  (alternative readings — the 又唸作 sheet)
  bits 13-15  reserved (record subtag masks these off on read)
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
  bits 14..=26 subcollection enable mask, SAME 13-bit layout as the subtag.
  bits 27-31   reserved (unknown high bits ignored for forward-compat).
```

`u32::MAX` (the all-enabled sentinel) carries bit 13 + every enable bit set, so
"all sources/subcollections on" stays consistent. Owner of the bit positions:
`dictionary/common/source_bits.py::encode_kautian_subtag` +
`engine/lexicon/src/dictionary_reader.rs` (`KAUTIAN_SUBTAG_*` / `WIRE_KAUTIAN_SUBCOLL_*`).

ENCODE: `compute_filters` sets bit 13 + the enable mask from
`DictionarySourceToggles.kautian_subcollections` (a nested message — PRESENCE
is the active sentinel). Every platform always sends it; a caller that leaves
it absent keeps the gate off = legacy all-on. The same mask gates the Tab3
dictionary-browse path and the keyboard fetch (`engine/composing/src/requests.rs`
`source_filter_bitmask` resolves the fetch's `toggles`). NextWord filters with
the source-only association mask, so subcollection toggles do not reach it.

---

## 5. Cross-Platform Invariants (drift hot list)

When ANY of the following changes, ALL listed files MUST be updated in the same commit:

| Invariant | Files that depend on it |
|---|---|
| `dictionary.bin` byte layout | build script, Rust `engine/lexicon::dictionary_reader`, this doc |
| `association.bin` byte layout | build script, Rust `engine/lexicon::association_reader`, this doc |
| Bitmask bit positions | build script (`dictionary/common/source_bits.py`), Rust `engine/lexicon::dictionary_filters`, platform `DictionarySource` DTO, this doc |
| `kautian_subtag` + wire subcollection-enable bit layout (§4.5) | `dictionary/common/source_bits.py` (`encode_kautian_subtag`), `dictionary/build/create_dictionary_bin.py`, Rust `engine/lexicon::dictionary_reader` (`KAUTIAN_SUBTAG_*` / `WIRE_KAUTIAN_SUBCOLL_*`), this doc |
| Key prefix list (`tl:` / `poj:` / `tps:`, `tl-abbrev:` / `poj-abbrev:` / `tps-abbrev:`, `hanzi:`) | build script (`create_fst.py`), Rust `phonetics::KeyFamily`, this doc |
| `syllables.fst` families + canonicalizers (§3.5) | `create_syllables_fst.py`, `fst-builder` `syllables.rs`, Rust `phonetics::canonicalize_*syllable`, `engine/lexicon::syllable_inventory`, this doc |
| Magic bytes (`TKDB` / `TKWA`) | build script, Rust readers, this doc |
| File version (`dictionary.bin = 4`, `association.bin = 2`) | build script, Rust readers, this doc |
| Endianness (little-endian) | build script, Rust readers |

### 5.1 No-checksum acknowledgement

None of the three formats carries a CRC or hash. Corruption surfaces as:

- Magic / version mismatch → reader fails to initialize (loud).
- Truncated file → reader fails to initialize (loud).
- Mid-file corruption inside a record → reader returns `null` for that record or skips that entry (silent — visible only as "missing word"; **no** hard failure).

If silent corruption ever becomes a real-world concern, add a header-level CRC32 over the offset table + record section. Out of scope for now.

### 5.2 Install counts

`LexiconRequest.install` answers `InstallResponse { dictionary_record_count, prefix_index_entry_count }`; the platforms only log them (e.g. desktop `lexicon.installed records=… prefix_entries=…`), nothing asserts on them. Build-script ↔ reader drift is caught by the Rust tests in §7.

---

## 6. Build Pipeline

Steps, inputs and outputs: [`dictionary/README.md` § Pipeline](../../dictionary/README.md#pipeline). Format rules the writers must keep:

1. Sort `association.bin` keys by raw UTF-8 byte order ascending, and each key's entries by `count` DESC.
2. Emit `dictionary.fst` and `syllables.fst` via `engine/build-helpers/fst-builder`, keys carrying the family prefix (`tl:` / `poj:` / `tps:` / `*-abbrev:` / `hanzi:`).
3. Use bit positions exactly per §4, magic bytes per §1 / §2, version `4` for `dictionary.bin` and `2` for `association.bin`.
4. Include every key form (TL / POJ / TPS numeric + toneless under the phonetic family, each abbreviation under its `*-abbrev:` family) plus `hanzi:` keys for reverse lookup.

---

## 7. Test Coverage Today

| Format | Test |
|---|---|
| `dictionary.bin` (record layout, `record_count` header) | Rust `engine/lexicon/tests/dictionary_reader_v4.rs` |
| `dictionary.bin` (parser) | Rust `engine/lexicon/tests/parity.rs` |
| `association.bin` | Rust `engine/lexicon/tests/parity.rs` (filter), `tests/association_v2.rs` (namespaces, v1 rejection) |
| `dictionary.fst` | Rust `engine/lexicon/tests/parity.rs` (round-trip a sentinel key set) |
| Bitmask filter | Rust `engine/lexicon::dictionary_reader` unit tests + integration via `parity.rs` |

The Rust `parity.rs` test suite is the canonical check; platform tests cover content/coverage at the asset bundle level only.
