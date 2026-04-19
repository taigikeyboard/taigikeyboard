# Data Artifact Portability Audit (G10)

Snapshot of how the five data artifacts that back the IME are produced, stored, and consumed on iOS and Android, plus the open decisions that must be resolved before any shared Rust core owns them. Not a design doc for the Rust side — an inventory of constraints.

- **Date**: 2026-04-19
- **Phase**: I (iOS exemplar), task G10
- **Gating signal**: this file's existence closes Phase I gate #8 (`docs/architecture/ios-exemplar-plan.md`).
- **Scope**: shipped and runtime artifacts whose byte layout or schema crosses platforms. UI assets (keyboard layouts, fonts, images), persisted preferences (`SharedSettings` / DataStore blobs), and logs are out of scope — they are not candidates for shared-core ownership.

## Summary

| Artifact | Status | Risk to Rust extraction |
|---|---|---|
| `dictionary.trie` (MARISA) | **Portable as-is** | Low — identical bytes, identical C++ lib |
| `dictionary.bin` | **Portable as-is** | Low — LE / UTF-8 / fixed layout; bitmask semantics duplicated across platforms |
| `association.bin` | **Portable as-is** | Low — LE / UTF-8 / sorted keys; bitmask is a 9-bit subset of dictionary.bin |
| `user_frequency.db` (SQLite) | **Portable** | Low — schemas aligned; version spaces aligned |
| `user_association.db` (SQLite) | **Portable** | Low — schemas aligned; `PRAGMA user_version` aligned on both platforms |
| `custom_dictionary.db` (SQLite) | **Portable with caveat** | Medium — schemas converge but version numbers + migration mechanisms diverge |

No artifact is a blocker for Phase II. Seven open decisions (D1–D7) are registered below for resolution before the relevant Rust slice lands.

---

## 1. `dictionary.trie` — MARISA trie

Compressed prefix trie holding dictionary keys (`tl:`, `poj:`, `hanzi:` plus toneless / abbrev / numeric variants) mapped to rowids.

### Pipeline

- **Producer**: `dictionary/build/04_create_trie.py` — `marisa_trie.RecordTrie("<I", pairs)`. Key layout: `utf8_key + 0xFF + LE u32 rowid`.
- **Shipped path**: `ios/Resources/Dictionaries/dictionary.trie` · `android/app/src/main/assets/dictionary.trie`.
- **Size**: 5.2 MB, byte-identical both platforms (same build script, same file).

### Readers

| Platform | Entry point | Underlying lib |
|---|---|---|
| iOS | `ios/Sources/TaigiKeyboard/Lexicon/Services/TrieService.swift` → `marisa_bridge.{h,cpp}` | Vendored MARISA-trie C++ |
| Android | `android/app/src/main/java/com/siansiansu/taigikeyboard/ime/dictionary/TrieService.kt` → `trie_jni.cpp` | Vendored MARISA-trie C++ (same source) |

Both sides hard-code `VALUE_SEPARATOR = 0xFF` and unpack the trailing u32 rowid as little-endian.

### Rust-core path

Two viable options, deferred to Phase IV-A:
- Bind the same vendored C++ lib from Rust via `cxx` / build-script.
- Swap to a Rust-native trie (`fst`, `marisa-rs`) and re-validate byte-level reproducibility.

### Open decisions

- **D1**. MARISA lib strategy — C++ bind vs Rust port. _Resolve by Phase IV-A design._

---

## 2. `dictionary.bin` — dictionary entry table

Custom binary format holding per-rowid dictionary entries.

### Format

- **Producer**: `dictionary/build/10_create_dictionary_bin.py`.
- **Header (16 bytes)**: magic `"TKDB"` (4) · `version: u32` (currently `1`) · `count: u32` · `build_ts: u32`.
- **Offset table**: `count × u32` absolute byte offsets (LE).
- **Record**: `bitmask: u16 · frequency: u32 · hanzi_len: u8 · tl_len: u8 · hanzi_bytes · tl_bytes` — UTF-8 strings, fixed-layout prefix.
- **Bitmask (13 bits)**: bits 0–7 = eight text sources (`kautian, taigitv, itaigi, sitbut, taihoa, taijit, kungge, stti`); bits 8–11 = four Hoklo-sourced dicts (`khpoo, khiin, dev, lkk`); bit 12 = `is_variant`.

### Readers

| Platform | Entry point | Mapping |
|---|---|---|
| iOS | `ios/.../Lexicon/Database/DictionaryBinaryReader.swift` | `Data(contentsOf:options:.mappedIfSafe)` + `UnsafeRawPointer.loadUnaligned` |
| Android | `android/.../ime/dictionary/DictionaryBinaryReader.kt` | `RandomAccessFile → MappedByteBuffer` (READ_ONLY) |

### Divergence

- **Endianness**: both hardcoded LE; no byte-swap paths.
- **UTF-8 error policy (convergent)**: both platforms treat `hanzi` as optional and `tl` as required.
  - iOS (`DictionaryBinaryReader.swift:136-152`): `hanzi` decode failure → record returned with `hanzi = nil`; `tl` decode failure → record dropped (`return nil`).
  - Android (`DictionaryBinaryReader.kt:76-88`): same contract — `hanzi` uses `decodeUtf8Strict` returning `null` on invalid bytes (code comment: `// matches iOS`); `tl` uses the same helper and the record is dropped on `null` (`?: return null`, comment: `// matches iOS`).
  - Neither platform logs on failure. The "hanzi-optional, tl-required" contract is shared — this is today's invariant, not a divergence to resolve.
- **Bitmask semantics duplication**: iOS `sourcesFromBitmask()` and Android `BIT_TO_SOURCE` define identical bit-to-source mapping in separate code. Silent drift risk.

### Rust-core path

Straightforward Rust re-implementation — stdlib (`byteorder` + `std::str::from_utf8`) is sufficient. Memory-mapped read via `memmap2`.

### Open decisions

- **D2**. UTF-8 error reporting policy for the Rust reader. Both platforms today share the "hanzi-optional, tl-required" contract but neither logs or surfaces a decode failure. Decide whether the Rust reader stays silent, emits a log/metric, or errors — aligning the platforms to the chosen policy.
- **D3**. `version` field has never been bumped. Codify a bump policy (header-compat matrix, reader fallback) **before** Rust parser lands.
- **D4**. Lift bitmask semantics into a single enum / constants table shipped alongside the Rust parser; platforms consume from one source of truth.

---

## 3. `association.bin` — next-word lookup table

Read-only next-word bigram/phrase table. Sibling to `dictionary.bin` with a distinct header and a narrower bitmask.

### Format

- **Producer**: `dictionary/build/11_create_association_bin.py`.
- **Header (20 bytes)**: magic `"TKWA"` (4) · `version: u32` (currently `1`) · `key_count: u32` · `entry_count: u32` · `build_ts: u32`.
- **Cohesion contract**: `build_ts` **must match** `dictionary.bin`. The build pipeline shares `.build_ts` between the two writers (`dictionary/build/10_create_dictionary_bin.py` and `dictionary/build/11_create_association_bin.py`). Readers on both platforms expose `buildTimestamp`.
- **Key offset table**: `key_count × u32` absolute offsets.
- **Key entry**: `prev_word_len: u8 · prev_word: utf8 · entry_offset: u32 · entry_count: u16`. Keys sorted by UTF-8 byte order for binary search.
- **Entry**: `bitmask: u16 · count: u32 · next_word_len: u8 · next_tl_len: u8 · next_word: utf8 · next_tl: utf8`.
- **Bitmask (9 bits)**: `kautian, taigitv, itaigi, sitbut, taihoa, taijit, kungge, stti, khpoo`. Subset of dictionary.bin bits 0–8. No `is_variant`, no `khiin / dev / lkk`.

### Readers

| Platform | Entry point |
|---|---|
| iOS | `ios/.../Lexicon/Database/AssociationBinaryReader.swift` — mmap + `UnsafeRawPointer.loadUnaligned`, binary search on key offset table |
| Android | `android/.../ime/dictionary/AssociationBinaryReader.kt` — `MappedByteBuffer` absolute-position reads, binary search on key offset table |

Both expose `lookup(prevWord, limit=...)`. Behavior aligned.

### Divergence

None observed. Key sort order, bitmask layout, and entry skip semantics match. Same UTF-8 policy skew as `dictionary.bin` applies (D2 covers both).

### Rust-core path

Re-implement reader in Rust alongside dictionary.bin parser. Share `byteorder` + `memmap2`. Binary-search harness is trivial.

### Open decisions

Covered by D2 (UTF-8 error policy) and D3 (version-bump policy). No additional decisions unique to this artifact.

---

## 4. Runtime SQLite — `user_frequency.db`

Per-install word-usage counter. Drives candidate ranking.

### Schema (identical both platforms)

```sql
CREATE TABLE user_frequency (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    word TEXT NOT NULL UNIQUE,
    count INTEGER DEFAULT 1,
    last_used TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
-- Indexes
CREATE INDEX idx_word ON user_frequency(word);
CREATE INDEX idx_count ON user_frequency(count DESC);
CREATE INDEX idx_last_used ON user_frequency(last_used DESC);
-- Side: metadata(key TEXT PRIMARY KEY, value TEXT)
```

### Pruning (identical constants)

| Constant | iOS | Android | Source |
|---|---|---|---|
| `maxEntries` | 20000 | 20000 | `UserFrequencyPruner.swift:12` · `UserFrequencyService.kt:48` |
| Record-check interval | 100 | 100 | |
| Prune batch size | 2000 | 2000 | |
| Selection order | `count ASC, last_used ASC` | `count ASC, last_used ASC` | iOS code comments "matches Android" |

### Divergence

- **Metadata seed**: iOS `INSERT OR IGNORE` with `schema_version = "1.0"`; Android `INSERT OR REPLACE` with `schema_version = "1"`. Cosmetic, no runtime impact — neither side reads the value back as a gate. Noted for cleanup.
- **Library**: iOS calls SQLite3 C API directly via `SQLiteConnectionManager`; Android uses `SQLiteOpenHelper`.

### Rust-core path

`rusqlite` on desktop / test; platform SQLite via FFI on mobile.

### Open decisions

None blocking — schema + pruning already parity.

---

## 5. Runtime SQLite — `user_association.db`

User-learned next-word associations. Sibling of the shipped `association.bin` for the write path.

### Schema (identical both platforms)

```sql
CREATE TABLE user_association (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    prev_word TEXT NOT NULL,
    prev_tl TEXT DEFAULT '',
    next_word TEXT NOT NULL,
    next_tl TEXT DEFAULT '',
    count INTEGER DEFAULT 1,
    last_used TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(prev_word, next_word, next_tl)
);
CREATE INDEX idx_user_prev_word ON user_association(prev_word);
CREATE INDEX idx_user_prev_word_tl ON user_association(prev_word, prev_tl);
```

### Migration mechanism

- **Both platforms** use `PRAGMA user_version`.
- **Version spaces aligned**: `schemaVersion = 4` on iOS (`ios/.../NextWord/Repository/NextWordSchema.swift:11`) and `DATABASE_VERSION = 4` on Android (`android/.../NextWordService.kt:54`).
- **History**: v3→v4 added `prev_tl` on both sides; Android shipped a v0/v1→v2 drop that pre-dates iOS and aligned Android's shape to iOS's at the time.

### Divergence

None at schema or migration-version level. Scoring constants in `NextWordService.kt:57` carry a `CROSS-PLATFORM INVARIANT` code comment — an existing alignment tag.

**Journal mode (parity confirmed)**: both platforms ship a one-time WAL → DELETE migration that runs on connection open — iOS `SQLiteConnectionManager.swift:42-98` (`sqlite3_wal_checkpoint_v2` + `PRAGMA journal_mode=DELETE`), Android `NextWordService.kt:605-636` (`PRAGMA wal_checkpoint(TRUNCATE)` + `PRAGMA journal_mode=DELETE`). Steady-state journal mode is `DELETE` on both platforms.

### Rust-core path

Same as `user_frequency.db` — `rusqlite` + platform SQLite.

### Open decisions

None blocking.

---

## 6. Runtime SQLite — `custom_dictionary.db`

User-authored custom dictionary entries with derived search keys.

### Schema (identical both platforms)

```sql
CREATE TABLE custom_dictionary (
    id TEXT PRIMARY KEY,              -- UUID
    roman TEXT NOT NULL,
    hanzi TEXT NOT NULL,
    notone TEXT DEFAULT '',           -- derived
    abbrev TEXT DEFAULT '',           -- derived
    roman_num TEXT DEFAULT '',        -- derived
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_custom_roman     ON custom_dictionary(roman);
CREATE INDEX idx_custom_notone    ON custom_dictionary(notone);
CREATE INDEX idx_custom_abbrev    ON custom_dictionary(abbrev);
CREATE INDEX idx_custom_roman_num ON custom_dictionary(roman_num);
```

### Derived-column logic

`notone` / `abbrev` / `roman_num` are pure functions of `roman`. iOS canonicalizes this in `ios/.../Lexicon/Database/CustomDictionaryDerivation.swift` (a shared-core candidate). Android computes the same transforms independently. Both write-time and read-time paths must agree.

### Migration mechanism — **diverges**

| Concern | iOS | Android |
|---|---|---|
| Version tracking | `PRAGMA user_version` | `SQLiteOpenHelper.DATABASE_VERSION` + `onUpgrade` |
| Current version value | `1` (`CustomDictionarySchema.swift:17`) | `5` (`CustomDictionaryService.kt:30`) |
| Migration record | Single `CustomDictionaryMigrator` re-runs ALTER + backfill based on column-presence checks | Four explicit steps: `migrateV1ToV2` adds `notone` + `abbrev` with backfill; `migrateV2ToV3` regenerates `notone` (space-strip fix); `migrateV3ToV4` regenerates `notone` (POJ nasal ⁿ fix); `migrateV4ToV5` adds `roman_num` with backfill |

Terminal schema converges. Version _numbers_ diverge because Android recorded each schema change — including pure backfill regenerations — as a version step, while iOS's migrator bootstraps from current column presence under a single `user_version = 1` stamp. Two of Android's four migrations (v2→v3, v3→v4) are pure derivation-logic changes with no DDL; the iOS side's column-presence approach cannot distinguish those from the current state. Any future shared migration owner (e.g., Rust core managing migrations) needs an explicit version-namespace mapping **and** a derivation-version stamp before it can reason about both sides.

### Rust-core path

- Shared migration code must either (a) pick one version-tracking mechanism and migrate the other platform's existing installs into it, or (b) keep both stamps and maintain a cross-map.

### Open decisions

- **D5**. `custom_dictionary` version-namespace unification. Must land **before** any Rust-owned migration code. Track as a Phase IV-A prerequisite.
- **D6**. Lift `CustomDictionaryDerivation` transforms to the shared core so both platforms stop maintaining parallel implementations. Already a shared-core roster candidate — prioritise in the Phonetics slice (Phase IV-A proof).

---

## Update / delivery

Dictionary updates today: `dictionary.trie` + `dictionary.bin` + `association.bin` are regenerated by the Python build pipeline and shipped in the app bundle (iOS) / assets (Android). No OTA channel exists. `build_ts` in `dictionary.bin` and `association.bin` is the only version signal readers expose.

**Invariants the delivery mechanism must preserve** — regardless of whether a future Rust core changes the distribution channel:

1. The two binary artifacts with a header (`dictionary.bin`, `association.bin`) carry a matching `build_ts` — enforced by the shared `.build_ts` file in the build pipeline.
2. `dictionary.trie` has **no timestamp or version in its bytes** — the format is a raw MARISA RecordTrie. Today the three artifacts' cohesion relies entirely on the build script producing all three in the same run; readers cannot detect a stale trie paired with fresh bins.
3. User-writable SQLite databases (`user_frequency.db`, `user_association.db`, `custom_dictionary.db`) are per-install and must not be shipped as read-only assets.
4. Schema migrations run on first open after an app update; the delivery mechanism does not modify these files directly.

Distribution-channel design (OTA vs app-bundle) is out of scope for this audit — to be revisited during Phase IV-B planning.

---

## Decision register

| # | Item | Resolve by |
|---|---|---|
| D1 | MARISA lib strategy (C++ bind vs Rust port) | Phase IV-A design |
| D2 | `dictionary.bin` + `association.bin` UTF-8 error policy | Before Rust parser lands |
| D3 | `dictionary.bin` + `association.bin` version-bump policy | Before Rust parser lands |
| D4 | Lift bitmask semantics to single shared-core enum | Phase IV-A (Phonetics slice prep) |
| D5 | `custom_dictionary` version-namespace unification — plus derivation-version stamp so pure-logic migrations (v2→v3, v3→v4 on Android) are expressible in both worlds | Before shared migration code |
| D6 | Lift `CustomDictionaryDerivation` to shared core | Phase IV-A (bundled with Phonetics slice) |
| D7 | Cross-artifact cohesion check for the shipped trio — e.g., embed a trie header or compute a manifest hash covering all three files so readers can detect a stale `dictionary.trie` paired with fresh `.bin` files | Before any OTA / incremental delivery mechanism |

None of D1–D7 blocks Phase II. All must be catalogued before Phase IV-A design freezes.

## Out of scope

- Rust FFI surface design — Phase IV-A.
- OTA / incremental-update delivery — Phase IV-B.
- UI assets (keyboard layouts, fonts, icons, tab1 content JSON).
- Persisted settings (`SharedSettings` · Android DataStore blobs · `colorSettings`) — not shared-core candidates.
- Build-time-only SQLite artifacts (`dictionary.db`, `trie.db`) — consumed by the Python pipeline, never shipped.
