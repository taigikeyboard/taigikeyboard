# Implementation Plan: Eliminate dictionary.db — Binary mmap Migration

## Context

`dictionary.db` (44 MB SQLite) is the largest file in the iOS bundle. The MARISA trie already handles search (key → rowid). SQLite serves only as a read-only rowid → {hanzi, tl, frequency, source flags} lookup — overkill for this use case. Mainstream IMEs (AOSP LatinIME, mozc) use compact binary formats for read-only dictionaries.

**Goal**: Replace dictionary.db with binary mmap files. Reduce ~48 MB → ~14-15 MB. Eliminate SQLite for all read-only dictionary data.

**Scope**: iOS first. Android keeps dictionary.db until iOS is validated.

---

## Architecture Comparison

```
Before (48 MB):                        After (~14-15 MB):
  dictionary.db     44 MB               dictionary.trie   4.1 MB (unchanged)
  dictionary.trie    4.1 MB             hanzi.trie        ~1-2 MB (NEW)
                                         dictionary.bin    ~4-5 MB (NEW)
                                         association.bin   ~4-5 MB (NEW)
```

SQLite remains only for user-writable data: `user_frequency.db`, `user_association.db`, `custom_dictionary.db`.

---

## Design Decisions

| # | Decision | Rationale |
|---|----------|-----------|
| 1 | **Full SQLite elimination** for read-only data | dictionary + word_association both move to binary |
| 2 | **dictionaryv2 folder** for new build pipeline | Full copy of `dictionary/`. Replace original when done. Zero risk to existing builds. |
| 3 | **Flat binary + mmap** (not encode in trie values) | Each row has ~6 trie keys. Encoding data in values would duplicate 6×. |
| 4 | **Offset table + variable-length records** | O(1) rowid access. ~4.4 MB vs ~9.4 MB if fixed-size. |
| 5 | **16-bit bitmask** for source flags | 13 flags + is_variant packed into u16. Bitwise AND replaces SQL WHERE. |
| 6 | **Hanzi MARISA trie** for reverse lookup | Prefix search replaces SQL `LIKE '%x%'`. Settings UI only. |
| 7 | **Sorted key table + binary search** for association | 5,658 keys, ~13 binary search steps. No need for a third trie. |
| 8 | **Handle-based multi-trie C bridge** | Current marisa_bridge has single global trie. Needs 2 tries (dictionary + hanzi). |
| 9 | **Direct replacement** in DictionaryRepository | No protocol/adapter. Replace SQLite code directly. git revert if needed. |
| 10 | **iOS first** | Validate binary format on one platform, then Android follows. |

---

## Binary Format Specifications

### dictionary.bin (little-endian)

```
Header (16 bytes):
  magic:       4 bytes  "TKDB"
  version:     u32      1
  count:       u32      159073
  build_ts:    u32      Unix timestamp (shared across all binary files)

Offset table (count × 4 bytes = ~620 KB):
  offsets[0..N-1]: u32  absolute byte offset from file start to record
  Mapping: rowId (1-based) → offsets[rowId - 1]
  Record size: offsets[i+1] - offsets[i] (last record: EOF - offsets[N-1])

Records (variable-length, one per rowid):
  bitmask:   u16   source flags (see below)
  frequency: u32   frequency value (max ~185K, fits u32)
  hanzi_len: u8    UTF-8 byte count (0 = NULL, max 19)
  tl_len:    u8    UTF-8 byte count (max 35)
  hanzi:     [u8]  UTF-8 bytes
  tl:        [u8]  UTF-8 bytes

  NOTE: Records are NOT aligned. Use loadUnaligned(as:) or bytewise
  decoding for u16/u32 fields — UnsafeRawPointer.load(as:) on
  unaligned addresses is undefined behavior in Swift.

Bitmask bit layout (u16):
  0=kautian  1=taigitv  2=itaigi   3=sitbut  4=taihoa   5=taijit
  6=kungge   7=stti     8=khpoo    9=khiin   10=dev     11=lkk
  12=is_variant  13-15=reserved
```

**Filter logic** (3 layers, equivalent to current SQL):
```swift
// Layer 1: variant exclusion
if !variantEnabled && (bitmask & (1 << 12)) != 0 → skip
// Layer 2: khiin exclusion
if !khiinEnabled && (bitmask & (1 << 9)) != 0 → skip
// Layer 3: source OR match (dev always included)
if !allEnabled && (bitmask & sourceMask) == 0 && (bitmask & devBit) == 0 → skip
```

### hanzi.trie

Same RecordTrie format as dictionary.trie:
```
Key:   hanzi UTF-8 string (e.g., "好無")
Value: utf8_key + \xff + uint32_le(rowid)
```
- One entry per (hanzi, rowid) pair. Rows with NULL/empty hanzi are skipped.
- Prefix search: query "早" returns rowids for "早安", "早起", "早起床", etc.
- No key prefix needed (unlike dictionary.trie's "tl:"/"poj:" prefixes).
- Build script must use dictionary.db rowids, NOT generate its own.

### association.bin (little-endian)

```
Header (20 bytes):
  magic:       4 bytes  "TKWA"
  version:     u32      1
  key_count:   u32      5658
  entry_count: u32      177421
  build_ts:    u32      Unix timestamp (must match dictionary.bin)

Key offset table (key_count × u32):
  Byte offset from file start to each key entry

Key section (sorted by prev_word UTF-8 for binary search):
  Each key:
    prev_word_len: u8    (max 4 bytes, mostly single CJK char)
    prev_word:     [u8]  UTF-8 bytes
    entry_offset:  u32   byte offset into entry section
    entry_count:   u16   number of entries

Entry section (sorted by count DESC within each key group):
  Each entry:
    bitmask:       u16   9-bit source flags (kautian..khpoo)
    count:         u32   association count
    next_word_len: u8    (max 12)
    next_tl_len:   u8    (max 27)
    next_word:     [u8]  UTF-8 bytes
    next_tl:       [u8]  UTF-8 bytes
```

---

## Stages

### Stage 1: dictionaryv2 Build Pipeline
**Goal**: Produce .bin + hanzi.trie alongside existing .db  
**Status**: Complete

1. `cp -r dictionary dictionaryv2`
2. New scripts in `dictionaryv2/build/`:
   - `10_create_dictionary_bin.py` → `output/dictionary.bin`
   - `11_create_association_bin.py` → `output/association.bin`
   - `12_create_hanzi_trie.py` → `output/hanzi.trie`
3. Update `dictionaryv2/build.sh` — add 3 new build steps
4. Update `dictionaryv2/build/06_deploy.sh` — deploy new files to iOS, keep .db for Android
5. Each script includes `--verify` mode (round-trip check against SQLite)

**Critical constraints** (from Codex review):
- Binary scripts must **read rowids from dictionary.db**, not generate independently. Rowids must match dictionary.trie exactly.
- Assert `dictionary.bin` record count == `dictionary.trie` entry count == dictionary.db row count.
- All binary files share a `build_ts` timestamp for version consistency.
- Verify offset table is monotonically increasing and within file bounds.

**Test**: Run full build. Verify all files generated. `--verify` passes.

---

### Stage 2: Multi-Trie C Bridge
**Goal**: marisa_bridge supports multiple concurrent trie instances  
**Status**: Complete

1. `marisa_bridge.h` — add handle-based API:
   - `trie_create(path) → trie_handle_t`
   - `trie_h_lookup(handle, key, results, max) → count`
   - `trie_h_prefix_search(handle, prefix, results, max) → count`
   - `trie_h_close(handle)`
2. `marisa_bridge.cpp` — fixed array of 8 `marisa::Trie*` slots. Handle = index.
3. Keep old global API as wrappers (backward compat during transition)

**Test**: Load 2 trie files via new API, query both, verify correct results.

**Files**:
- `ios/Sources/TaigiKeyboard/Lexicon/Trie/marisa_bridge.h`
- `ios/Sources/TaigiKeyboard/Lexicon/Trie/marisa_bridge.cpp`

---

### Stage 3: Swift TrieService Refactoring
**Goal**: TrieService supports multiple instances  
**Status**: Complete  
**Depends on**: Stage 2

1. `TrieService.swift` — allow public init with fileName/fileExtension
   - Each instance holds its own `trie_handle_t`
   - `.shared` continues to manage `dictionary.trie`
2. `LexiconService.swift` — create second TrieService for `hanzi.trie`, pass to DictionaryRepository

**Test**: Both tries load. dictionary.trie returns rowids for romanization. hanzi.trie returns rowids for hanzi prefix.

**Files**:
- `ios/Sources/TaigiKeyboard/Lexicon/Trie/TrieService.swift`
- `ios/Sources/TaigiKeyboard/Lexicon/Services/LexiconService.swift`

---

### Stage 4: DictionaryBinaryReader + DictionaryRepository Migration
**Goal**: Replace all SQLite queries in DictionaryRepository  
**Status**: Complete  
**Depends on**: Stages 1, 3

1. New `DictionaryBinaryReader.swift`:
   - mmap via `Data(contentsOf:options:.mappedIfSafe)`
   - Reader must **strongly retain** the mmap'd `Data` for its full lifetime
   - Validate: magic "TKDB", version, count, offset monotonicity, section bounds
   - Validate `build_ts` matches other binary files at startup
   - `record(at rowId:) → DictionaryRecord?`
   - Use `loadUnaligned(as:)` for u16/u32 fields (records are NOT aligned)
2. `EnabledDictionaries` — add `toBitmask() → UInt16`
3. Rewrite `DictionaryRepository.swift`:
   - `query()` → trie rowids → binary read + bitmask filter
   - `searchWithSources()` → same + bitmask → [DictionarySource]
   - `searchByHanzi()` → hanziTrie.prefixSearch → binary read
   - Remove: `import SQLite3`, `connectionManager`, all `queryBy*` methods

**Test**: Search known words. Compare results against SQLite baseline. Hanzi prefix search. Bitmask filter edge cases.

**Files**:
- `ios/Sources/TaigiKeyboard/Lexicon/Database/DictionaryBinaryReader.swift` (NEW)
- `ios/Sources/TaigiKeyboard/Lexicon/Database/DictionaryRepository.swift`

---

### Stage 5: AssociationBinaryReader + NextWordService Migration
**Goal**: Replace dict-side SQLite in NextWordService  
**Status**: Complete  
**Depends on**: Stages 1, 4

1. New `AssociationBinaryReader.swift`:
   - mmap `association.bin`, strongly retain `Data`
   - Validate: magic "TKWA", version, key_count, entry_count, build_ts
   - `lookup(prevWord:, limit:) → [AssociationEntry]`
   - Binary search on sorted key table
   - Use `loadUnaligned(as:)` for u16/u32 fields
2. Rewrite `NextWordService.swift` dict query path:
   - Remove `dictConnectionManager`
   - `queryDictAssociations()` → binary read + bitmask filter
   - **Preserve `limit * 2` over-fetch** (current behavior for dedup merging)
   - `buildDictWhereCondition()` → `buildDictBitmask() → UInt16`
   - User association path unchanged (still SQLite)

**Test**: `predict()` with known words. Verify predictions match SQLite baseline.

**Files**:
- `ios/Sources/TaigiKeyboard/Lexicon/Database/AssociationBinaryReader.swift` (NEW)
- `ios/Sources/TaigiKeyboard/Lexicon/Services/NextWordService.swift`

---

### Stage 6: Cleanup + Validation
**Goal**: Remove dictionary.db from iOS, finalize  
**Status**: Complete  
**Depends on**: All above

1. ~~Delete `ios/Resources/Dictionaries/dictionary.db`~~ ✅
2. Deploy script already updated (Stage 1)
3. Xcode build passed ✅
4. Device testing — pending user validation

**Results**:
- iOS bundle: 48.1 MB → **12.7 MB** (−73%)
- Xcode build: **passed**

---

## Dependency Graph

```
Stage 1 (Build Pipeline)     Stage 2 (C Bridge)
         \                      /
          \                    /
           Stage 3 (TrieService)
                   |
           Stage 4 (DictionaryRepo)
                   |
           Stage 5 (NextWordService)
                   |
           Stage 6 (Cleanup)
```

Stages 1 and 2 are independent (can develop in parallel).

---

## Files Summary

### New files
| File | Stage |
|------|-------|
| `dictionaryv2/build/10_create_dictionary_bin.py` | 1 |
| `dictionaryv2/build/11_create_association_bin.py` | 1 |
| `dictionaryv2/build/12_create_hanzi_trie.py` | 1 |
| `ios/Resources/Dictionaries/dictionary.bin` | 1 |
| `ios/Resources/Dictionaries/association.bin` | 1 |
| `ios/Resources/Dictionaries/hanzi.trie` | 1 |
| `ios/Sources/TaigiKeyboard/Lexicon/Database/DictionaryBinaryReader.swift` | 4 |
| `ios/Sources/TaigiKeyboard/Lexicon/Database/AssociationBinaryReader.swift` | 5 |

### Modified files
| File | Stage |
|------|-------|
| `dictionaryv2/build.sh` | 1 |
| `dictionaryv2/build/06_deploy.sh` | 1, 6 |
| `ios/.../Trie/marisa_bridge.h` | 2 |
| `ios/.../Trie/marisa_bridge.cpp` | 2 |
| `ios/.../Trie/TrieService.swift` | 3 |
| `ios/.../Services/LexiconService.swift` | 3, 4 |
| `ios/.../Database/DictionaryRepository.swift` | 4 |
| `ios/.../Services/NextWordService.swift` | 5 |

### Deleted files
| File | Stage |
|------|-------|
| `ios/Resources/Dictionaries/dictionary.db` | 6 |

---

## Risks & Mitigations

| Risk | Mitigation |
|------|-----------|
| Rowid gaps in dictionary.db | Verified: 1..159073, no gaps. Build script asserts. |
| Endianness | All iOS = little-endian ARM. Explicit LE in format spec. |
| mmap memory pressure (48-70 MB ext limit) | ~14 MB total < current 44 MB. Net improvement. |
| Binary search correctness (association) | Sort + compare by raw UTF-8 bytes. Round-trip verify in build. |
| Hanzi prefix vs LIKE substring | Prefix is intended UX — discussed and agreed. |
| Unaligned memory access | Use `loadUnaligned(as:)` for all u16/u32 fields in variable-length records. |
| Data lifetime / dangling pointer | Reader strongly retains mmap'd `Data` for its full lifetime. |
| Cross-file version mismatch | Shared `build_ts` in all binary headers. Validate at startup. |
| Binary writers rowid divergence | Scripts read from dictionary.db, assert count matches trie. |
| NextWord result count regression | Preserve `limit * 2` over-fetch in association binary path. |

---

## iOS Completion Summary

### Actual file sizes
| File | Size | Purpose |
|------|------|---------|
| `dictionary.trie` | 4.1 MB | 羅馬字搜尋索引 (MARISA, unchanged) |
| `dictionary.bin` | 4.4 MB | 主字典 binary mmap (rowid → record) |
| `association.bin` | 3.1 MB | 詞彙關聯 binary mmap (prev_word → entries) |
| `hanzi.trie` | 1.1 MB | 漢字前綴搜尋 (MARISA, new) |
| **Total** | **12.7 MB** | was 48.1 MB (−73%) |

### Architecture after migration
```
User input
  → TrieService (dictionary.trie, mmap)  → rowids
  → DictionaryBinaryReader (dictionary.bin, mmap) → {hanzi, tl, freq, bitmask}
  → Bitmask filter (bitwise AND, replaces SQL WHERE)
  → Sort + limit (Swift in-memory)
  → Autocomplete candidates

Hanzi reverse lookup (Tab3 settings UI)
  → TrieService (hanzi.trie, mmap) → rowids
  → DictionaryBinaryReader → same pipeline

NextWord prediction
  → AssociationBinaryReader (association.bin, mmap)
  → Binary search on sorted key table → entries
  → Bitmask filter → predictions
  → Merge with user_association.db (SQLite, writable)
```

### Key implementation details for Android port
- **Binary formats are platform-independent** — same .bin/.trie files work on both platforms
- **Build pipeline** (`dictionaryv2/`) already produces files for both platforms
- **C bridge** handle-based API already exists — Android JNI can use same pattern
- **Android needs**: Kotlin equivalents of `DictionaryBinaryReader` and `AssociationBinaryReader`
  - Use `FileChannel.map()` for mmap (or `AssetFileDescriptor` for direct APK mmap)
  - `ByteBuffer.order(ByteOrder.LITTLE_ENDIAN)` for integer decoding
  - Binary search logic is identical
- **Android trie_jni.cpp** already handles MARISA — add hanzi trie loading
- **Android files to modify**:
  - `LexiconService.kt` — load hanzi trie, create binary readers
  - `DictionaryRepository.kt` (or equivalent) — replace SQLite with binary reader
  - `NextWordService.kt` — replace dict SQLite with association binary reader
  - `trie_jni.cpp` — add handle-based API (same as iOS marisa_bridge.cpp)
- **noCompress optimization** (optional): mark `.bin` and `.trie` as `noCompress` in `build.gradle` for direct APK mmap without extraction

---

## Post-Review Cleanup (2026-04-12)

Code review by Claude + Codex identified 10 issues across 3 severity levels. All fixed.

### Fixes Applied

| # | Severity | Fix | Commit |
|---|----------|-----|--------|
| H1 | High | AssociationBinaryReader: Add bounds validation for `keyOffset`, `metaPos`, `entryOffset` — prevents out-of-bounds on corrupt files | `7b8cc40` |
| M1 | Medium | marisa_bridge: Remove dead global trie API (`g_trie`, `trie_load`, etc.) — 160 lines deleted, only handle-based API remains | `7b8cc40` |
| M2+M3 | Medium | DictionaryRepository: Extract `lookupRowIds()` — eliminates duplicated trie lookup pattern between `query()` and `searchWithSources()` | `0fcaef5` |
| M4 | Medium | Centralize bitmask: Add `EnabledDictionaries.associationBitmask()`, remove `NextWordService.buildDictBitmask()` duplicate | `0c93e93` |
| M6 | Medium | Python build scripts: `struct.pack('b')` → `'B'` for length fields (signed → unsigned byte, matches u8 spec) | `a4f7780` |
| L1 | Low | DictionaryBinaryReader: Name magic bitmask constants (`khiinBit`, `devBit`, `variantBit`) | `66b889f` |
| L2 | Low | DictionaryRepository: `.map(\.self)` → `Array(...)` for ArraySlice conversion | `0fcaef5` |
| L3 | Low | TrieService: Add `deinit` to release trie handle, prevents C bridge handle leak | `66b889f` |
| L5 | Low | Remove unused `EnabledDictionaries.allDisabled` | `0c93e93` |

### Not Fixed (acceptable as-is)

| # | Issue | Reason |
|---|-------|--------|
| M5 | C bridge `g_tries[]` thread safety | Trie creation only happens on a single `DispatchQueue` during init — no real race condition |
| L4 | Python `setup_logging()` / `get_build_timestamp()` duplication | Low impact, would require restructuring `common/` module for 3 scripts |

---

## Future

### Android Migration
**Goal**: Port binary format to Android  
**Status**: Not Started — iOS validated, build pipeline ready  
**Priority**: Next
