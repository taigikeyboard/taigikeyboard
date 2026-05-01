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

---

## 7. Android addendum — storage paths, asset copy, update-in-place

Phase II A10 deliverable (2026-04-21). Fills `docs/architecture/android-state-audit.md` §9 gate #8. Pure append — iOS §§1–6, §Update / delivery, and §Decision register are untouched. Cross-references back into those sections by anchor; does not restate their content.

### 7.1 On-disk paths

| Artifact | Android on-disk location | Access mechanism |
|---|---|---|
| `dictionary.trie` | `{filesDir}/dictionary.trie` (copied from `assets/dictionary.trie`) | JNI MARISA (`trie_jni.cpp`) via `nativeLoad(path)` |
| `dictionary.bin` | `{filesDir}/dictionary.bin` (copied from `assets/dictionary.bin`) | `RandomAccessFile` → `MappedByteBuffer` (READ_ONLY) |
| `association.bin` | `{filesDir}/association.bin` (copied from `assets/association.bin`) | `MappedByteBuffer` (READ_ONLY) |
| `user_frequency.db` | `{databases}/user_frequency.db` (`SQLiteOpenHelper`-managed) | `SQLiteOpenHelper.readableDatabase` / `writableDatabase` |
| `user_association.db` | `{filesDir}/user_association.db` (**not** under `databases/`) | `SQLiteDatabase.openOrCreateDatabase(File, null)` |
| `custom_dictionary.db` | `{databases}/custom_dictionary.db` (`SQLiteOpenHelper`-managed) | `SQLiteOpenHelper.readableDatabase` / `writableDatabase` |

`{filesDir}` = `Context.getFilesDir()` (`/data/user/0/<pkg>/files/`). `{databases}` = `Context.getDatabasePath(name).parentFile` (`/data/user/0/<pkg>/databases/`).

Divergence — `user_association.db` lives in `filesDir/`, not `databases/`. It was introduced with a direct `openOrCreateDatabase(File, null)` call rather than `SQLiteOpenHelper`, and the path stuck. Any future tool that enumerates DBs through `Context.getDatabasePath(...)` will miss it. Kept as-is to avoid migrating existing installs; flagged here as platform-only trivia (no cross-platform mapping implication since iOS has no database directory convention).

### 7.2 Asset copy semantics — shipped trio

All three read-only artifacts ship inside the APK at `android/app/src/main/assets/`. None are read directly from the APK: `android/app/build.gradle.kts` does not declare `noCompress("bin", "trie")`, so AAPT2 compresses them by default. Compressed zip entries cannot be memory-mapped, and the MARISA JNI loader calls `marisa::Trie::mmap(path)` (`android/app/src/main/cpp/trie_jni.cpp:87`) — it needs a real uncompressed filesystem path. Every boot therefore resolves to the uncompressed copy under `{filesDir}`.

| Artifact | Stamp file | Copier call-site |
|---|---|---|
| `dictionary.trie` | `{filesDir}/trie_app_version.txt` | `TrieService.getTriePath` (`TrieService.kt:124`) |
| `dictionary.bin` | `{filesDir}/dictionary_app_version.txt` *(shared stamp for the .bin pair)* | `LexiconService.copyAssetsIfNeeded` (`LexiconService.kt:430`) |
| `association.bin` | `{filesDir}/dictionary_app_version.txt` *(shared stamp for the .bin pair)* | `LexiconService.copyAssetsIfNeeded` (`LexiconService.kt:430`) |

Each copier on boot reads its stamp file, compares against `BuildConfig.VERSION_CODE`, and re-copies the asset(s) if the stamp is behind — OR if the destination file is missing regardless of stamp. Stamp-rewrite timing is **not** symmetric across the two copiers: `TrieService` rewrites `trie_app_version.txt` unconditionally after every successful trie copy; `LexiconService.copyAssetsIfNeeded` only rewrites `dictionary_app_version.txt` when `currentAppVersion > lastCopiedVersion`, so a same-version boot that has to repair a missing `.bin` file (e.g. user cleared app data partially, or a prior copy failed) runs the copy but leaves the stamp untouched.

**Cohesion risk — Android-specific sub-case of D7.** The trie stamp and the `.bin`-pair stamp are two files written independently, each after its own copy loop succeeds, with no cross-file transaction. The two stamps therefore do not move atomically even on the happy path, and any interruption (process kill, power loss, uncaught copy failure) between the two writers can leave the two stamps transiently out of sync. Readers trust whichever file is on disk; no manifest hash or combined check exists today. This is a narrower Android restatement of the iOS-authored D7 item ("embed a trie header or compute a manifest hash covering all three files") — tracked there, not as a new decision.

### 7.3 SQLite open mechanism — three patterns

Android runs three distinct SQLite-init patterns across the three runtime DBs. Terminal schemas still match iOS §§4–6; the mechanism divergence is platform-internal.

| DB | Mechanism | Version stamp | Migration shape |
|---|---|---|---|
| `user_frequency.db` | `SQLiteOpenHelper` | `DATABASE_VERSION = 1` (Helper-managed) plus `metadata.schema_version = "1"` text row | `onUpgrade` is a no-op (no migrations have ever shipped on this DB) |
| `user_association.db` | `SQLiteDatabase.openOrCreateDatabase` — no Helper | `PRAGMA user_version` (hand-rolled) — currently `4` | `migrateUserDb` (`NextWordService.kt:552`) dispatches `migrateV0ToV2` / `migrateV2ToV3` / `migrateV3ToV4`, then stamps `PRAGMA user_version = DATABASE_VERSION` |
| `custom_dictionary.db` | `SQLiteOpenHelper` | `DATABASE_VERSION = 5` (Helper-managed) | `onUpgrade` (`CustomDictionaryService.kt:421`) chains `migrateV1ToV2` → `migrateV2ToV3` → `migrateV3ToV4` → `migrateV4ToV5` |

Two of the four `custom_dictionary.db` steps (`v2→v3`, `v3→v4`) are pure derivation-logic regenerations with no DDL; see iOS §6 Migration-mechanism table for the divergence on version-namespace semantics (D5).

**One-shot journal-mode migration (parity with iOS)**. `NextWordService.migrateFromWAL` (`NextWordService.kt:523`) runs on every `user_association.db` open, detects `PRAGMA journal_mode = wal`, executes `PRAGMA wal_checkpoint(TRUNCATE)`, then switches to `PRAGMA journal_mode = DELETE`. Mirrors iOS `SQLiteConnectionManager.swift:42-98`. `user_association.db` is the only Android DB with an explicit WAL→DELETE migration; `user_frequency.db` and `custom_dictionary.db` go through `SQLiteOpenHelper` and take the platform default for journal mode without a corresponding switch. The asymmetry is accepted for Phase II (no cross-process writer on those two DBs); §9 gating signal applies only to the user-association path.

### 7.4 Update-in-place — APK upgrade semantics

1. **Read-only trio** — installing a new APK bumps `BuildConfig.VERSION_CODE`. On next boot both copiers (`TrieService`, `LexiconService`) detect stale stamps and overwrite `{filesDir}/dictionary.trie`, `{filesDir}/dictionary.bin`, `{filesDir}/association.bin` with the new asset bytes. **Partial-copy semantics** — both copiers write through `FileOutputStream(destFile)` (no append, no tmp-and-rename), which truncates the destination to 0 bytes on open. An interruption mid-stream (process kill, I/O error, power loss) therefore leaves the target truncated or partially written — the prior bytes are **not** preserved. On the next boot the reader's length / magic / version checks fail (`DictionaryBinaryReader.open` returns null; JNI `marisa::Trie::mmap` returns false), `binaryReader` stays null, and subsequent queries surface `DictionaryError.DatabaseNotAvailable`. Recovery paths: (a) a version-bump boot always re-runs the copy because `currentAppVersion > lastCopiedVersion` (stamp is only rewritten after the copy loop completes — an interrupted run leaves the stamp behind); (b) a same-`VERSION_CODE` boot with a truncated-but-present destination is **not** auto-repaired (`needsCopy=false` AND `destFile.exists()=true` → copier skips), leaving the keyboard degraded until app-data clear or reinstall. No OTA channel — every artifact refresh ships as an app update (parity with iOS §Update / delivery).
2. **User SQLite DBs** — `user_frequency.db`, `user_association.db`, `custom_dictionary.db` all persist across APK replacement (both `filesDir` and `databases/` survive app update). Uninstalling the app is the only way to lose them. Schema migrations run on first open after update per §7.3.
3. **DataStore** — the main `Preferences<Preferences>` file is `{filesDir}/datastore/taigi_keyboard_prefs.preferences_pb` (declared at `PreferenceDataStore.kt:16`). A second DataStore `{filesDir}/datastore/emoji_preferences.preferences_pb` holds the emoji skin-tone selection (`EmojiPreferences.kt:20`). Both persist across APK replacement. `PrefHelper.migrateFromSharedPreferences` (`PrefHelper.kt:625`) pulls legacy Android `PreferenceManager` SharedPreferences values forward exactly once, gated on a DataStore-empty check.

DataStore blobs are **out of shared-core scope** per §Summary (iOS `SharedSettings` / Android DataStore blobs). They are listed here only for Android-side audit completeness so a future cross-platform reset / backup tool has a single inventory — they are not candidates for Rust-core ownership.

### 7.5 Read contract — matches iOS

This section is pointer-only — the substantive contract lives in iOS §§1–3.

- **UTF-8 decode** — Android `DictionaryBinaryReader.decodeUtf8Strict` (`DictionaryBinaryReader.kt:106`) uses `CodingErrorAction.REPORT` and returns `null` on invalid bytes with no log. `AssociationBinaryReader` follows the same contract. Matches iOS "hanzi-optional, tl-required" (§2). Policy alignment covered by D2.
- **Bitmask semantics duplication** — `DictionaryBinaryReader.BIT_TO_SOURCE` (`DictionaryBinaryReader.kt:124`) hardcodes the 12 dictionary source bits. `AssociationBinaryReader` holds no source-mapping table of its own; `passesFilter` routes through `EnabledDictionaries.associationBitmask()` (`EnabledDictionaries.kt:52`), which masks `sourceBitmask()` to bits 0–8 (the 9-bit subset iOS §3 documents). Silent-drift risk between `EnabledDictionaries.sourceBitmask` and `DictionaryBinaryReader.BIT_TO_SOURCE` is flagged in §2; covered by D4.
- **`build_ts` cohesion** — both binary readers expose `buildTimestamp` from the mmap header. The build pipeline enforces `build_ts` equality across `dictionary.bin` and `association.bin` (iOS §3 Format). Android does nothing extra here; readers simply surface whatever the shipped bytes carry.

### 7.6 Cross-references

- iOS readers / writers: §§1–6 above.
- Cross-platform invariants the delivery mechanism must preserve: §Update / delivery items 1–4. Android additions in §7.2 (stamp cohesion) and §7.4 (DataStore out-of-scope reminder).
- Phase II gate #8: `docs/architecture/android-state-audit.md` §9 #8.
- Android exemplar roster and marker convention: `docs/architecture/android-exemplar.md` §§3, 5 (I/O wrappers are explicitly excluded from the candidate roster there).
- Candidate-class markers on the Android readers: `DictionaryBinaryReader.kt` / `AssociationBinaryReader.kt` are platform-only I/O wrappers by construction (mmap over `MappedByteBuffer`) and are not carried on the Shared-Core candidate roster.

---

## Out of scope

- Rust FFI surface design — Phase IV-A.
- OTA / incremental-update delivery — Phase IV-B.
- UI assets (keyboard layouts, fonts, icons, tab1 content JSON).
- Persisted settings (`SharedSettings` · Android DataStore blobs · `colorSettings`) — not shared-core candidates.
- Build-time-only artifacts — none. Post-v3.5.6 part 2 the build pipeline reads `dictionary.csv` directly into the binary writers; the previous SQLite intermediates (`dictionary.db`, `trie.db`) are gone.
