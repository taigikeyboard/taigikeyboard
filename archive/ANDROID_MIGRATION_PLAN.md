# Android Migration Plan: Eliminate dictionary.db — Binary mmap Migration

## Context

iOS binary migration is complete (Stages 1–7 + post-review). Bundle size: 48 MB → 12.7 MB (−73%).
Build pipeline (`dictionary/`) already produces all binary files.
Android currently still uses `dictionary.db` (44 MB SQLite) for read-only lookups.

**Goal**: Port the binary mmap format to Android. Same binary files, same data flow, Kotlin equivalents.

**Scope**: Android only. iOS is done and validated.

---

## Architecture Comparison

```
Before (Android, ~48 MB):                After (~12.7 MB):
  dictionary.db     44 MB                 dictionary.trie   ~4.5 MB (already includes hanzi: keys)
  dictionary.trie    4.1 MB               dictionary.bin    4.4 MB (NEW)
                                           association.bin   3.1 MB (NEW)
```

SQLite remains only for user-writable data: `user_association.db`, `custom_dictionary.db`, `user_frequency.db`.

---

## Key Differences from iOS

| Aspect | iOS | Android |
|--------|-----|---------|
| mmap API | `Data(contentsOf:options:.mappedIfSafe)` + `UnsafeRawPointer` | `FileChannel.map()` → `MappedByteBuffer` |
| Integer decoding | `loadUnaligned(as:)` | `ByteBuffer.order(LITTLE_ENDIAN).getShort()/getInt()` |
| Asset delivery | Bundle resource | APK assets → copy to `filesDir` (existing pattern) |
| Trie bridge | C bridge (`marisa_bridge.cpp`) | JNI (`trie_jni.cpp`) — no changes needed |
| Multi-trie | Handle-based API (Stage 2, but only 1 trie used after merge) | Global trie API (sufficient — only 1 trie) |
| `searchByHanzi` | `hanzi:` trie prefix search | Currently SQL `LIKE '%x%'` → **change to `hanzi:` trie prefix** |
| EnabledDictionaries | Extracted to `Models/EnabledDictionaries.swift` | Currently private in `LexiconService` → **extract + add bitmask** |

---

## Design Decisions

| # | Decision | Rationale |
|---|----------|-----------|
| 1 | **No JNI bridge changes** | Global trie is sufficient — only 1 trie (hanzi.trie merged into dictionary.trie via `hanzi:` prefix). No handle-based API needed. |
| 2 | **Keep filesDir copy pattern** | Android assets are compressed in APK. Current pattern copies to `filesDir` for mmap. Works reliably. noCompress is a future optimization. |
| 3 | **MappedByteBuffer with LITTLE_ENDIAN, absolute-offset reads only** | Standard Kotlin/Java API for mmap. `ByteBuffer.order(ByteOrder.LITTLE_ENDIAN)` matches binary format. **Must use absolute-position methods** (`getShort(offset)`, `getInt(offset)`, `get(offset)`) — not relative reads (`getShort()`) which mutate `position` and are not thread-safe across coroutines. |
| 4 | **Direct replacement** in LexiconService/NextWordService | Same approach as iOS — replace SQLite code directly. Git revert if needed. |
| 5 | **Extract EnabledDictionaries** to shared file | Currently private in `LexiconService.kt`. Needs bitmask methods. Used by `LexiconService`, `NextWordService`, and both binary readers after migration. |
| 6 | **`searchByHanzi()` → trie prefix search** | Matches iOS behavior change. SQL `LIKE '%x%'` (substring) → `hanzi:` trie prefix (prefix match). Intentional UX alignment. |
| 7 | **Preserve `limit * 2` over-fetch** in NextWordService dict path | Matches iOS behavior — over-fetch for dedup merging with user associations. |
| 8 | **Stage D keeps dictionary.db copy for NextWordService** | During transition, LexiconService stops using SQLite but still copies dictionary.db so NextWordService can open it. Removed in Stage F after both services are migrated. |
| 9 | **Shared asset copy logic** | Binary files (dictionary.bin, association.bin) are copied alongside dictionary.trie in `LexiconService.ensureInitialized()`. NextWordService depends on LexiconService having already copied files (existing pattern for dictionary.db). |

---

## Binary Format Specifications

Same as iOS — see `IOS_MIGRATION_PLAN.md` for full specs. Key points:

### dictionary.bin
- Header (16B): magic "TKDB" + version u32 + count u32 + build_ts u32
- Offset table: count × u32 (absolute byte offset per record)
- Records: bitmask u16 + frequency u32 + hanzi_len u8 + tl_len u8 + hanzi + tl
- **Same file as iOS** — binary format is platform-independent

### association.bin
- Header (20B): magic "TKWA" + version u32 + key_count u32 + entry_count u32 + build_ts u32
- Key offset table: key_count × u32
- Key section (sorted by prev_word UTF-8 for binary search)
- Entry section (sorted by count DESC per group)
- **Same file as iOS** — binary format is platform-independent

### Bitmask bit layout (u16, shared with iOS)
```
0=kautian  1=taigitv  2=itaigi  3=sitbut  4=taihoa  5=taijit
6=kungge   7=stti     8=khpoo   9=khiin   10=dev    11=lkk
12=is_variant  13-15=reserved
```

---

## Stages

### Stage A: Deploy Binary Files to Android
**Goal**: Build pipeline deploys .bin files to Android assets alongside .db
**Status**: Complete
**Depends on**: iOS Stage 1 (build pipeline, already complete)

1. Update `dictionary/build/06_deploy.sh`:
   - Add `cp dictionary.bin` and `cp association.bin` to Android assets
   - Keep `dictionary.db` for now (removed in Stage F)
   - Log file sizes for both platforms
2. Run full build pipeline
3. Verify files in `android/app/src/main/assets/`:
   - `dictionary.bin` (4.4 MB)
   - `association.bin` (3.1 MB)
   - `dictionary.trie` (already deployed, now includes `hanzi:` keys)
   - `dictionary.db` (still present, used until Stage F)

**Test**: Run build. All 4 files present in Android assets. `dictionary.trie` has `hanzi:` keys.

**Files**:
- `dictionary/build/06_deploy.sh`

---

### Stage B: EnabledDictionaries Extraction + Bitmask
**Goal**: Shared EnabledDictionaries with bitmask support for binary filter
**Status**: Complete

1. Extract `EnabledDictionaries` from `LexiconService.kt` to new `EnabledDictionaries.kt`
2. Add bitmask methods (matching iOS `EnabledDictionaries.swift`):
   - `sourceBitmask() → UInt16` (bits 0-11, matching dictionary.bin layout)
   - `associationBitmask() → UInt16` (bits 0-8, matching association.bin layout)
   - `allAssociationSourcesEnabled: Boolean` (9 main sources)
3. Update `LexiconService.kt` to import from new file (remove private class)
4. Update `NextWordService.kt` to use `EnabledDictionaries` for bitmask conversion (currently builds SQL from `PrefHelper.DictEnabledSnapshot` directly at line 822)
5. `DictionaryConstants.kt` — add `TRIE_PREFIX_HANZI = "hanzi:"` (matching iOS)
6. Add binary-specific error types to `DictionaryModels.kt`:
   ```kotlin
   object BinaryFileNotFound : DictionaryError() { ... }
   object BinaryFileMagicMismatch : DictionaryError() { ... }
   object BinaryFileVersionMismatch : DictionaryError() { ... }
   object BinaryFileTruncated : DictionaryError() { ... }
   ```

**Bitmask bit layout** (must match iOS `EnabledDictionaries.sourceBitmask()`):
```kotlin
fun sourceBitmask(): Int {
    var mask = 0
    if (kautian) mask = mask or (1 shl 0)
    if (taigitv) mask = mask or (1 shl 1)
    if (itaigi)  mask = mask or (1 shl 2)
    if (sitbut)  mask = mask or (1 shl 3)
    if (taihoa)  mask = mask or (1 shl 4)
    if (taijit)  mask = mask or (1 shl 5)
    if (kungge)  mask = mask or (1 shl 6)
    if (stti)    mask = mask or (1 shl 7)
    if (khpoo)   mask = mask or (1 shl 8)
    // khiin = bit 9 (handled separately in filter)
    // dev = bit 10 (always included)
    if (lkk)     mask = mask or (1 shl 11)
    return mask
}
```

**Test**: Bitmask values match iOS for same enabled combination.

**Files**:
- `android/.../dictionary/EnabledDictionaries.kt` (NEW — extracted from LexiconService)
- `android/.../dictionary/LexiconService.kt` (remove private class, import shared)
- `android/.../dictionary/NextWordService.kt` (import shared EnabledDictionaries)
- `android/.../dictionary/DictionaryConstants.kt` (add HANZI prefix)
- `android/.../dictionary/DictionaryModels.kt` (add binary error types)

---

### Stage C: DictionaryBinaryReader.kt
**Goal**: Kotlin equivalent of iOS DictionaryBinaryReader.swift
**Status**: Complete
**Depends on**: Stage A (binary files in assets)

1. New `DictionaryBinaryReader.kt`:
   - Load via `FileChannel.map(MapMode.READ_ONLY)` → `MappedByteBuffer`
   - `ByteBuffer.order(ByteOrder.LITTLE_ENDIAN)` for all integer decoding
   - Validate: magic "TKDB", version, count, offset monotonicity
   - `record(rowId: Int) → DictionaryRecord?` (1-based rowid)
   - `sourcesFromBitmask(bitmask: Int) → List<DictionarySource>`
   - `passesFilter(recordBitmask: Int, enabledDicts: EnabledDictionaries) → Boolean`
     - Layer 1: variant exclusion
     - Layer 2: khiin exclusion
     - Layer 3: source OR match (dev always included)

2. Data class:
   ```kotlin
   data class DictionaryRecord(
       val bitmask: Int,       // UInt16 stored as Int (Kotlin has no unsigned short)
       val frequency: Int,     // UInt32 stored as Int (max ~185K, fits signed Int)
       val hanzi: String?,
       val tl: String
   )
   ```

**Implementation notes**:
- **CRITICAL: Use absolute-position methods only** — `buffer.getShort(offset)`, `buffer.getInt(offset)`, `buffer.get(offset)` — never position-relative methods (`buffer.getShort()`, `buffer.get()`). Position-relative methods mutate internal `position` state and are NOT thread-safe across coroutines dispatched on `Dispatchers.IO`.
- `MappedByteBuffer.getShort(offset)` returns signed Short → mask with `toInt() and 0xFFFF` for unsigned u16
- `MappedByteBuffer.getInt(offset)` returns signed Int → for UInt32 values that may exceed Int.MAX_VALUE, use `toLong() and 0xFFFFFFFFL`. Frequency max ~185K is safe as signed Int.
- `MappedByteBuffer.get(offset)` returns signed Byte → mask with `toInt() and 0xFF` for unsigned u8
- For UTF-8 string extraction: use absolute-position bulk read: `ByteArray(len).also { for (i in 0 until len) it[i] = buffer.get(offset + i) }` then `String(bytes, Charsets.UTF_8)`. (No position-relative `buffer.get(byteArray)` — that's position-based.)
- Reader holds strong reference to `MappedByteBuffer` (prevents GC and unmap)
- `FileChannel.map()` only works on files in filesystem, NOT on compressed APK assets. Must copy from assets to `filesDir` first (existing pattern in `TrieService.kt:145` and `LexiconService.kt:499`).

**Test**: Load dictionary.bin. Read known rowid. Verify hanzi/tl/frequency match SQLite baseline.

**Files**:
- `android/.../dictionary/DictionaryBinaryReader.kt` (NEW)

---

### Stage D: LexiconService Migration
**Goal**: Replace all SQLite queries with binary reader
**Status**: Complete
**Depends on**: Stages B, C

1. Add `DictionaryBinaryReader` instance (initialized alongside trie)
2. Asset copy logic: add `dictionary.bin` and `association.bin` to the copy-from-assets flow (alongside existing dictionary.db and dictionary.trie copy)
3. Rewrite `queryByIds()`:
   - Iterate rowids → `binaryReader.record(id)` → bitmask filter → collect
   - Sort by frequency DESC → take(limit)
   - POJ display: `TaigiPhonetics.tlDisplayToPOJDisplay(tl)` (unchanged)
4. Rewrite `queryByIdsWithSources()`:
   - Same as above + `DictionaryBinaryReader.sourcesFromBitmask(bitmask)`
   - Map to `DictionarySearchResult`
   - **Must preserve both raw `tl` and display `roman`** — `DictionarySearchResult` expects both fields (`.tl` for URL generation, `.roman` for display). Binary reader returns raw `tl`; convert to POJ for display `roman` when `inputMode == POJ`.
5. Rewrite `searchByHanzi()`:
   - **Before**: SQL `WHERE hanzi LIKE ?` (substring match)
   - **After**: `TrieService.prefixSearch("hanzi:" + input)` → rowids → binary read → bitmask filter
   - This aligns with iOS behavior (prefix search, not substring)
6. Remove from LexiconService:
   - `database: SQLiteDatabase?` field
   - `connect()`, `configure()` methods (SQLite open + PRAGMA)
   - `Column` object (column names no longer needed for LexiconService queries)
   - **Keep `getDatabasePath()` and dictionary.db copy logic** — NextWordService still depends on `filesDir/dictionary.db` being present until Stage E migrates it. Removing it here would break bigram predictions.

**Note**: `ensureInitialized()` still needs to:
- Copy dictionary.db from assets to filesDir (for NextWordService — removed in Stage F)
- Copy dictionary.bin and association.bin from assets to filesDir (NEW)
- Copy dictionary.trie from assets to filesDir (existing, via TrieService)
- Initialize TrieService
- Initialize DictionaryBinaryReader
- (CustomDictionaryService — unchanged)

**Test**: Search known words. Compare results against SQLite baseline. Hanzi prefix search returns correct results. Bitmask filter works for all dictionary combinations.

**Files**:
- `android/.../dictionary/LexiconService.kt` (major rewrite)

---

### Stage E: AssociationBinaryReader.kt + NextWordService Migration
**Goal**: Replace dict SQLite in NextWordService with binary reader
**Status**: Complete
**Depends on**: Stages A, B

1. New `AssociationBinaryReader.kt`:
   - mmap `association.bin` via `MappedByteBuffer`
   - Validate: magic "TKWA", version, key_count, entry_count, build_ts
   - `lookup(prevWord: String, limit: Int = 60) → List<AssociationEntry>`
   - Binary search on sorted key table (UTF-8 byte comparison)
   - `passesFilter(entryBitmask: Int, enabledDicts: EnabledDictionaries) → Boolean`

2. Data class:
   ```kotlin
   data class AssociationEntry(
       val nextWord: String,
       val nextTl: String,
       val count: Int,
       val bitmask: Int
   )
   ```

3. Rewrite `NextWordService.predict()` dict query path:
   - **Before**: SQL `SELECT ... FROM word_association WHERE prev_word = ?` with `buildDictWhereCondition()` (line 822)
   - **After**: `associationReader.lookup(lastChar, limit * 2)` → bitmask filter via `EnabledDictionaries` → Prediction
   - **Preserve `limit * 2` over-fetch** (matches iOS behavior for dedup merging)
   - Convert `AssociationEntry` → `Prediction` with `DICT_WEIGHT` scoring
   - Build `EnabledDictionaries` from `PrefHelper.DictEnabledSnapshot` (same pattern as LexiconService)
4. Remove from NextWordService:
   - `dictDatabase: SQLiteDatabase?` field
   - `connectDictDb()` method
   - `buildDictWhereCondition()` method
   - `COL_*` constants (kautian, taigitv, etc.)
   - dictionary.db opening in `ensureInitialized()`
5. **Initialization dependency**: NextWordService must ensure `association.bin` is already copied to `filesDir` before creating `AssociationBinaryReader`. Existing pattern: NextWordService depends on LexiconService having already copied files (see `NextWordService.kt:381`). Stage D adds binary file copying to LexiconService init, so this dependency is satisfied. NextWordService just opens the already-copied `association.bin`.
6. User association path **unchanged** (still SQLite — writable)

**Test**: `predict()` with known words. Verify predictions match SQLite baseline. Dict filter works.

**Files**:
- `android/.../dictionary/AssociationBinaryReader.kt` (NEW)
- `android/.../dictionary/NextWordService.kt` (rewrite dict query path)

---

### Stage F: Cleanup + Validation
**Goal**: Remove dictionary.db from Android, finalize
**Status**: Complete
**Depends on**: All above

1. Remove `dictionary.db` from `android/app/src/main/assets/`
2. Remove dictionary.db asset copy logic from LexiconService (should already be done in Stage D)
3. Remove dictionary.db version file (`dictionary_app_version.txt`) logic — replace with binary version file
4. Update `dictionary/build/06_deploy.sh`:
   - Remove `dictionary.db` copy to Android
   - Update comments
5. Optional: Add `noCompress` to `build.gradle.kts` for `.bin` and `.trie` files:
   ```kotlin
   androidResources {
       noCompress += listOf("bin", "trie")
   }
   ```
   This allows direct APK mmap without extraction (future optimization — requires changing asset loading pattern from copy-to-filesDir to `AssetFileDescriptor`).

**Test**: Full app build. All search paths work. Bundle size reduction verified.

**Files**:
- `android/app/src/main/assets/dictionary.db` (DELETED)
- `dictionary/build/06_deploy.sh`
- `android/app/build.gradle.kts` (optional noCompress)

---

## Dependency Graph

```
Stage A (Deploy Files)       Stage B (EnabledDictionaries)
         \                      /
          \                    /
           Stage C (DictBinaryReader)
                   |
           Stage D (LexiconService — copies binary files, keeps dictionary.db for NWS)
                   |
           Stage E (AssocBinaryReader + NextWordService — uses files copied by Stage D)
                   |
           Stage F (Cleanup — remove dictionary.db)
```

- Stages A and B are independent (can develop in parallel).
- Stage C depends on A (binary files in assets) and B (EnabledDictionaries for bitmask filter).
- Stage D depends on C. **Also copies binary files to filesDir**, making them available for Stage E.
- Stage E depends on D (binary files in filesDir) and B (EnabledDictionaries for bitmask filter).
- Stage F depends on all above.
- **Stages D and E are independently shippable**: Stage D keeps dictionary.db copy logic so NextWordService still works via SQLite. Stage E then migrates NextWordService and removes the SQLite dependency.

---

## Files Summary

### New files
| File | Stage |
|------|-------|
| `android/.../dictionary/EnabledDictionaries.kt` | B |
| `android/.../dictionary/DictionaryBinaryReader.kt` | C |
| `android/.../dictionary/AssociationBinaryReader.kt` | E |

### Modified files
| File | Stage |
|------|-------|
| `dictionary/build/06_deploy.sh` | A, F |
| `android/.../dictionary/DictionaryConstants.kt` | B |
| `android/.../dictionary/DictionaryModels.kt` | B |
| `android/.../dictionary/LexiconService.kt` | B, D |
| `android/.../dictionary/NextWordService.kt` | B, E |

### Deleted files
| File | Stage |
|------|-------|
| `android/app/src/main/assets/dictionary.db` | F |

---

## Cross-Platform Alignment Verification

| Feature | iOS | Android (after migration) | Aligned? |
|---------|-----|---------------------------|----------|
| Romanization autocomplete | trie → binary → bitmask → sort | trie → binary → bitmask → sort | ✓ |
| Hanzi search (Tab3) | `hanzi:` trie prefix → binary | `hanzi:` trie prefix → binary | ✓ |
| NextWord prediction | association.bin binary search → bitmask → merge with user | association.bin binary search → bitmask → merge with user | ✓ |
| Bitmask bit layout | bits 0-12 (see spec) | bits 0-12 (same spec) | ✓ |
| Filter 3 layers | variant → khiin → source OR | variant → khiin → source OR | ✓ |
| `limit * 2` over-fetch | Yes | Yes (preserved) | ✓ |
| Binary format | platform-independent | same files | ✓ |
| Build pipeline | dictionary/ | dictionary/ (same) | ✓ |

---

## Risks & Mitigations

| Risk | Mitigation |
|------|-----------|
| MappedByteBuffer GC/unmap | Reader holds strong reference. Never leak ByteBuffer. |
| Signed/unsigned mismatch (Kotlin) | Mask: `Short.toInt() and 0xFFFF`, `Byte.toInt() and 0xFF`. Document clearly. |
| **MappedByteBuffer thread safety** | **Use absolute-position methods ONLY** (`getShort(offset)`, `getInt(offset)`, `get(offset)`). Position-relative methods mutate internal state — not safe across `Dispatchers.IO` coroutines. |
| Asset copy timing | Copy binary files in same `ensureInitialized()` flow, with version check. |
| **NextWordService init before LexiconService** | NextWordService depends on LexiconService having already copied dictionary.db (existing pattern, `NextWordService.kt:381`). Binary files follow same dependency. If init order changes, add binary file copy to NextWordService as fallback. |
| `searchByHanzi` behavior change | Intentional: substring → prefix. Matches iOS. Document in changelog. |
| Android 32-bit (armeabi-v7a) | Max file 4.4 MB, well within 32-bit mmap limits. |
| dictionary.db still needed during transition | Keep in assets until Stage F. Stage D explicitly retains copy logic for NextWordService. |
| Build timestamp mismatch | Validate `build_ts` matches between dictionary.bin and association.bin at init. |
| **Cold-start latency** | Binary files are smaller than SQLite (12 MB vs 48 MB), so copy-from-assets is actually faster. mmap init is near-instant. Net improvement over current SQLite open + PRAGMA. |
| **No runtime rollback after Stage F** | Git revert for code. For shipped APKs: dictionary.db removal is only in Stage F, shipped last after full validation. Consider keeping dictionary.db in one release cycle as safety net. |
| **FileChannel.map() on APK assets** | Not possible — APK assets are compressed. Must copy to filesDir first. Existing pattern. noCompress is future optimization only. |

---

## Expected Results

| Metric | Before | After | Change |
|--------|--------|-------|--------|
| Android APK dictionary assets | ~48 MB | ~12 MB | −75% |
| Read-only SQLite databases | 1 (dictionary.db) | 0 | Eliminated |
| Writable SQLite databases | 3 (user_assoc, custom_dict, user_freq) | 3 (unchanged) | — |
| Search data flow | trie → SQLite batch query | trie → binary mmap → bitmask | Faster |
| Hanzi search | SQL LIKE (full scan) | Trie prefix (O(key_length)) | Much faster |

---

## Codex Review (2026-04-12)

Plan reviewed by Codex rescue agent. 6 issues found and addressed:

| # | Issue | Fix |
|---|-------|-----|
| 1 | MappedByteBuffer thread safety: position-relative reads mutate state, not safe across coroutines | Added Design Decision #3: absolute-position reads only. Updated implementation notes. |
| 2 | Stage D missing raw `tl` preservation in `DictionarySearchResult` | Added explicit note in Stage D step 4: must preserve both raw `tl` and display `roman`. |
| 3 | Dependency graph: Stage E has undeclared dependency on D (binary file copying) | Fixed dependency graph. E depends on D (not just A+B). |
| 4 | Stage D not independently shippable: removing dictionary.db copy logic breaks NextWordService | Stage D now explicitly keeps `getDatabasePath()` and dictionary.db copy. Removed in Stage F. |
| 5 | NextWordService needs EnabledDictionaries adoption for bitmask | Added to Stage B step 4. NextWordService currently builds SQL from PrefHelper directly. |
| 6 | Missing binary-specific error types | Added to Stage B step 6. DictionaryError expanded with BinaryFileNotFound, MagicMismatch, etc. |

Additional risks flagged and added to risk table: cold-start latency, runtime rollback, FileChannel on APK assets.
