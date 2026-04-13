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
| 2 | **Separate build folder during migration** | Started as `dictionaryv2/` (copy of `dictionary/`). Now merged back to `dictionary/`. |
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

### Stage 1: Build Pipeline (binary format)
**Goal**: Produce .bin + hanzi.trie alongside existing .db  
**Status**: Complete

1. Created as `dictionaryv2/` (copy of `dictionary/`), now merged back to `dictionary/`
2. New scripts in `dictionary/build/`:
   - `10_create_dictionary_bin.py` → `output/dictionary.bin`
   - `11_create_association_bin.py` → `output/association.bin`
   - `12_create_hanzi_trie.py` → `output/hanzi.trie`
3. Update `dictionary/build.sh` — add 3 new build steps
4. Update `dictionary/build/06_deploy.sh` — deploy new files to iOS, keep .db for Android
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
| `dictionary/build/10_create_dictionary_bin.py` | 1 |
| `dictionary/build/11_create_association_bin.py` | 1 |
| `dictionary/build/12_create_hanzi_trie.py` | 1 |
| `ios/Resources/Dictionaries/dictionary.bin` | 1 |
| `ios/Resources/Dictionaries/association.bin` | 1 |
| `ios/Resources/Dictionaries/hanzi.trie` | 1 |
| `ios/Sources/TaigiKeyboard/Lexicon/Database/DictionaryBinaryReader.swift` | 4 |
| `ios/Sources/TaigiKeyboard/Lexicon/Database/AssociationBinaryReader.swift` | 5 |

### Modified files
| File | Stage |
|------|-------|
| `dictionary/build.sh` | 1 |
| `dictionary/build/06_deploy.sh` | 1, 6 |
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
- **Build pipeline** (`dictionary/`) already produces files for both platforms
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

## Code Quality Notes (2026-04-12)

Post-cleanup 整體 review，記錄既有結構問題供後續改善參考。與 binary migration 無直接關係。

| # | 問題 | 位置 | 建議 |
|---|------|------|------|
| Q1 | `EnabledDictionaries` 放在 `DictionaryRepository.swift` 但被 3 個檔案使用 | `DictionaryRepository.swift:4-65` | 抽成獨立 `EnabledDictionaries.swift` |
| Q2 | `TrieService` 同時被當 singleton (`.shared`) 和普通 instance (`hanziTrieService`) 使用，生命週期不一致 | `TrieService.swift:19`, `LexiconService.swift:26` | 考慮去掉 `.shared`，由 `LexiconService` 統一管理 |
| Q3 | `NextWordService` 標 `@unchecked Sendable` 但有 mutable 狀態 (`recordCounter`, `isUserTablesCreated`) 無同步保護 | `NextWordService.swift:57,461` | 目前 async 環境不會出事，但不嚴謹 |
| Q4 | `DictionaryBinaryReader`/`AssociationBinaryReader` failable init 失敗時無 logging | `DictionaryBinaryReader.swift:45-84` | 加 `os_log` 或 `DebugLogger`，否則線上查不到失敗原因 |
| Q5 | `query()` 和 `buildSearchResults()` 的 record 遍歷+過濾仍有重複 | `DictionaryRepository.swift` | 可接受的重複——兩個產出型別不同，強行統一反而降低可讀性 |

---

## Stage 7: Merge hanzi.trie into dictionary.trie
**Goal**: Eliminate separate hanzi.trie — merge into dictionary.trie with `hanzi:` prefix  
**Status**: Complete  
**Depends on**: Stage 6 (Cleanup)

### Motivation

hanzi.trie（1.1MB）是獨立的漢字前綴搜尋 trie，只有設定頁 Tab3 使用。它和 dictionary.trie 用相同的 RecordTrie 格式，但需要獨立的 build script、deploy 流程、TrieService instance。合併後統一 prefix pattern（`tl:` / `poj:` / `hanzi:`），消除 ~180 行程式碼，解決 Q2（TrieService singleton 混用）。

### Design Decision

`04_create_trie.py` 從 `dictionary.db` 讀取 hanzi 資料（第二個 DB connection），不修改 `trie.db` schema。理由：trie.db 是羅馬字專用的 staging table，加 hanzi 欄位會連帶影響 `03_create_trie_db.sh`。

### Steps

**Build pipeline（Commit 1）：**

1. **`04_create_trie.py`** — 加 `HANZI_PREFIX = "hanzi:"`，從 `dictionary.db` 讀 hanzi+rowid，用 `add_pair()` 加入 pairs。加測試查詢和 key count log。
2. **`build.sh`** — 移除 `do_create_hanzi_trie()` 和呼叫，step 編號 11→10。
3. **`06_deploy.sh`** — 移除 `HANZI_TRIE` 變數和 iOS deploy。
4. **`12_create_hanzi_trie.py`** — 整個刪除。

**iOS Swift（Commit 2）：**

5. **`LexiconConstants.swift`** — `TriePrefix` 加 `static let hanzi = "hanzi:"`
6. **`DictionaryRepository.swift`** — 刪除 `hanziTrieService` 屬性/參數，`searchByHanzi()` 改用 `trieService` + `hanzi:` prefix。
7. **`LexiconService.swift`** — 刪除 `hanziTrieService` 相關程式碼。
8. **`hanzi.trie`** — `git rm`。提醒使用者手動從 Xcode 移除 reference。

### Files

| File | Action |
|------|--------|
| `dictionary/build/04_create_trie.py` | Modified — add hanzi key generation |
| `dictionary/build.sh` | Modified — remove step 8 |
| `dictionary/build/06_deploy.sh` | Modified — remove hanzi.trie deploy |
| `dictionary/build/12_create_hanzi_trie.py` | Deleted |
| `ios/.../Models/LexiconConstants.swift` | Modified — add hanzi prefix |
| `ios/.../Database/DictionaryRepository.swift` | Modified — remove hanziTrieService |
| `ios/.../Services/LexiconService.swift` | Modified — remove hanziTrieService |
| `ios/Resources/Dictionaries/hanzi.trie` | Deleted |

### Verification

- `python3 04_create_trie.py` → log 顯示 hanzi key count
- `query_trie.py "hanzi:好"` → 回傳結果
- `query_trie.py "tl:hoo2"` → 結果不變（romanization 不受影響）
- iOS Tab3 漢字搜尋正常、打字候選詞正常

### Risks

| Risk | Mitigation |
|------|-----------|
| 合併後 dictionary.trie 變大影響打字效能 | MARISA prefix search 是 O(key_length)，不會掃描 `hanzi:` subtree |
| Android 使用更大的 dictionary.trie | `hanzi:` keys 是惰性的，Android 只查 `tl:`/`poj:`，不受影響 |
| Xcode 需手動移除 hanzi.trie reference | 明確提醒使用者，即使不移除也只是多 1.1MB |

---

## Final Code Review (2026-04-12)

### Methodology

4 independent reviewers analyzed all 9 core files:

| Reviewer | Focus |
|----------|-------|
| Claude (manual) | Data flow correctness, search integrity, SRP, naming |
| Codex | Correctness, unsafe patterns, search regressions |
| Code-simplifier | Redundancy, idioms, clean code, over-design |
| Refactor-reviewer | Regressions, cross-platform alignment, style |

### Search Functionality Verdict

**No regressions found.** All search paths verified:
- Romanization autocomplete: trie → binary reader → bitmask filter → sort → limit ✓
- Hanzi reverse lookup (Tab3): `hanzi:` prefix → trie → binary reader ✓
- NextWord prediction: binary search → association.bin → bitmask filter → merge with user ✓
- Cross-platform: scoring constants identical, source filtering equivalent, no contract broken ✓

### Confirmed Issues

| # | Severity | Issue | Files | Flagged By |
|---|----------|-------|-------|------------|
| R1 | Medium | **TrieService: `@unchecked Sendable` with incomplete synchronization** — `isInitialized`/`handle` protected by `initQueue` in `initialize()`/`close()` but read without queue in `prefixSearch()`/`lookup()`/`isReady`/`deinit`. Technically a data race. | `TrieService.swift:31-32,87,109,44-46` | Codex, Simplifier, Refactor, Manual |
| R2 | Medium | **NextWordService: `@unchecked Sendable` with unsynchronized mutable state** — `recordCounter` (line 57) and `isUserTablesCreated` (line 462) mutated from async contexts without synchronization. | `NextWordService.swift:57,162-164,462,476` | Codex, Simplifier, Manual |
| R3 | Medium | **EnabledDictionaries misplaced** — Defined in `DictionaryRepository.swift` but used by 3 files across 2 directories. Should be in `Lexicon/Models/EnabledDictionaries.swift`. | `DictionaryRepository.swift:4-70` | Simplifier, Manual, Plan Q1 |
| R4 | Medium | **LexiconConstants.Database dead code** — `fileName`/`fileExtension` for `dictionary.db` unused after SQLite removal. | `LexiconConstants.swift:3-6` | Codex, Simplifier, Refactor, Manual |
| R5 | Medium | **`.map(\.self)` anti-pattern** — `.prefix(limit).map(\.self)` followed by `Array(sortedResults)` creates double copy. Use `Array(...)` directly. | `NextWordService.swift:123` | Simplifier, Manual |
| R6 | Medium | **`sqliteTransient` duplicated** — Same constant defined in both `NextWordService.Constants` and `SQLiteConnectionManager`. All other files use the latter. | `NextWordService.swift:37`, `SQLiteConnectionManager.swift:224` | Simplifier |
| R7 | Medium | **Inconsistent `passesFilter` API** — `DictionaryBinaryReader` takes `EnabledDictionaries`, `AssociationBinaryReader` takes raw primitives `(UInt16, UInt16, Bool)`. Caller must pre-compute mask. | `DictionaryBinaryReader.swift:170`, `AssociationBinaryReader.swift:199` | Simplifier |
| R8 | Low | **LexiconService should be `final class`** — All other Lexicon services are `final class`. | `LexiconService.swift:5` | Simplifier |
| R9 | Low | **Magic number 1000 in `lookup()`** — Duplicates `Constants.defaultSearchLimit`. | `TrieService.swift:119` | Simplifier |
| R10 | Low | **Force unwrap `word.last!`** — Safe due to prior `guard !word.isEmpty`, but `guard let` more idiomatic. | `NextWordService.swift:103` | Simplifier |
| R11 | Low | **Stale comment references `dictionary.db`** — Should say `dictionary.bin, association.bin, dictionary.trie`. | `ResourceBundleResolver.swift:5` | Refactor |
| R12 | Low | **Invalid UTF-8 silently → `""`** — `nextWord`/`nextTl` default to `""` on decode failure. `DictionaryBinaryReader` returns `nil` instead. Inconsistent. | `AssociationBinaryReader.swift:172-173` | Codex |
| R13 | Low | **Tautological `>= 0` checks** — `UInt32` cast to `Int` is always >= 0. | `AssociationBinaryReader.swift:112,153` | Simplifier |

### Disputed / Not Actionable

| Finding | Source | Verdict |
|---------|--------|---------|
| Hanzi autocomplete returns `[]` | Codex | **Pre-existing design** — autocomplete is for romanization input. Hanzi lookup goes through `searchByHanzi()` (Tab3 settings only). Not a regression. |
| `prev_tl` excluded from UNIQUE constraint | Codex | **Intentional** — associations are by hanzi context, not romanization. Same character typed via TL/POJ/TPS should share predictions. |
| `g_tries[]` thread safety | Simplifier | **Already accepted** (M5 in post-review cleanup) — trie creation happens on single DispatchQueue during init. |
| Duplicated filter loop `query()` vs `buildSearchResults()` | Codex, Simplifier | **Already accepted** (Q5 in code quality notes) — different output types (`TaigiWord` vs `DictionarySearchResult`); forced unification hurts readability. |
| `Set` dedup loses exact-match priority | Codex | **Not impactful** — results are re-sorted by frequency + user score downstream. Final ordering is deterministic via sort. |
| Duplicated binary reader init (~30 lines) | Simplifier | **Acceptable** — 2 readers with different headers/magic. Shared base class would be over-engineering for this scale. |

### Fixes Applied

All 13 issues fixed in one pass.

**Phase A** (quick cleanup):
- ✅ R4 — Removed `LexiconConstants.Database` dead code
- ✅ R5 — `.map(\.self)` → `Array(...)` in `NextWordService.predict()`
- ✅ R6 — Removed duplicate `sqliteTransient`, now uses `SQLiteConnectionManager.sqliteTransient`
- ✅ R8 — `LexiconService`: `class` → `final class`
- ✅ R9 — `TrieService.lookup()`: magic 1000 → `Constants.defaultSearchLimit`
- ✅ R10 — `word.last!` → `guard let last = word.last else { return [] }`
- ✅ R11 — Updated `ResourceBundleResolver.swift` comment to reference `dictionary.bin, association.bin`
- ✅ R13 — Removed tautological `>= 0` checks in `AssociationBinaryReader`

**Phase B** (structural):
- ✅ R3 — Extracted `EnabledDictionaries` to `Lexicon/Models/EnabledDictionaries.swift` (**needs Xcode target addition**)
- ✅ R7 — Aligned `passesFilter` API: `AssociationBinaryReader` now takes `EnabledDictionaries` (matches `DictionaryBinaryReader`)
- ✅ R12 — `AssociationBinaryReader`: invalid UTF-8 now skips record (`continue`) instead of defaulting to `""`

**Phase C** (thread safety):
- ✅ R1 — `TrieService`: renamed queue to `stateQueue`, all reads of `_isInitialized`/`_handle` now go through queue via `currentHandle` computed property. `deinit` also uses queue.
- ✅ R2 — `NextWordService`: added `NSLock` (`stateLock`) protecting `_recordCounter` and `_isUserTablesCreated`. All access uses `stateLock.withLock { ... }`.

---

## Future

### Android Migration
**Goal**: Port binary format to Android  
**Status**: Complete — See `ANDROID_MIGRATION_PLAN.md`  
**Results**: Android assets 48 MB → 12.7 MB (−75%), dictionary.db eliminated
