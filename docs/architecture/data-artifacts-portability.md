# Data Artifact Portability Audit (G10)

Snapshot of how the five data artifacts that back the IME are produced, stored, and consumed on iOS and Android, plus the open decisions that must be resolved before any shared Rust core owns them. Not a design doc for the Rust side — an inventory of constraints.

- **Originally authored**: 2026-04-19 as Phase I G10 deliverable.
- **Current state**: dictionary read path is now in Rust `engine/lexicon` (since v3.5.6); SQLite user-data stays platform-side permanently (`status=wont_migrate` per `migration-inventory.csv`).
- **Scope**: shipped and runtime artifacts whose byte layout or schema crosses platforms. UI assets (keyboard layouts, fonts, images), persisted preferences (`SharedSettings` / DataStore blobs), and logs are out of scope.

## Summary

| Artifact | Status | Owner |
|---|---|---|
| `dictionary.fst` (Burntsushi fst) | **Shipped in Rust** | Rust `engine/lexicon::prefix_index::PrefixIndex` (replaced MARISA in v3.5.6) |
| `dictionary.bin` | **Shipped in Rust** | Rust `engine/lexicon::dictionary_reader` |
| `association.bin` | **Shipped in Rust** | Rust `engine/lexicon::association_reader` |
| `user_frequency.db` (SQLite) | **Native — `wont_migrate`** | iOS `UserFrequencyService.swift` / Android `UserFrequencyService.kt` |
| `user_association.db` (SQLite) | **Native — `wont_migrate`** | iOS `Lexicon/Database/` / Android `ime/text/composing/UserFrequencyService.kt` |
| `custom_dictionary.db` (SQLite) | **Native — `wont_migrate`** | iOS `Lexicon/Database/CustomDictionaryRepository.swift` / Android `ime/dictionary/CustomDictionaryService.kt` |

Per `feedback_user_data_sqlite_stays_native`, all writable user-data DBs stay native (better integration with platform backup / file-provider / encryption). The three read-only assets are byte-identical across iOS and Android and are now consumed by the Rust crate via `mmap-host`.

---

## 1. `dictionary.fst` — Burntsushi fst prefix index

Burntsushi `fst` finite-state transducer holding dictionary keys (`tl:`, `poj:`, `hanzi:` plus toneless / abbrev / numeric variants) mapped to rowids.

### Pipeline

- **Producer**: `dictionary/build/create_fst.py` shells to the Rust binary `engine/build-helpers/fst-builder`. Value layout: rowid packed in the low 32 bits of the `u64` value; high bits reserved.
- **Shipped path**: the repo-root `dictionaries/`, committed once. iOS reads it through an Xcode synchronized folder and Android through an assets source dir, both without a copy; macOS (`macos/scripts/bundle-app.sh`) and Windows (`windows/scripts/release-app.sh`) copy from it at package time.
- **Size**: ~9.1 MB, byte-identical on both platforms (same build, same file).

### Reader

| Platform | Entry point |
|---|---|
| iOS | `RustEngineBridge.search` / `searchByHanzi` / `searchWithSources` → Rust `engine/lexicon` |
| Android | `LexiconBridge.search` / `searchByHanzi` / `searchWithSources` → same Rust crate |

Rust `engine/lexicon::prefix_index::PrefixIndex` opens the file via `mmap-host` (the only crate not `forbid(unsafe_code)`) and exposes `Map::range` / `Map::get` for prefix scans + exact lookup.

### History

D1 (MARISA C++ bind vs Rust port) was **resolved** in v3.5.6 by switching to a Rust-native `fst` index. The C++ bridges (`marisa_bridge.cpp`, `trie_jni.cpp`) and the platform `TrieService.swift` / `TrieService.kt` files were deleted under Path G.

---

## 2. `dictionary.bin` — dictionary entry table

Custom binary format holding per-rowid dictionary entries.

### Format

Authoritative spec lives in [`docs/engine/binary-format.md`](../engine/binary-format.md). Summary:

- **Producer**: `dictionary/build/create_dictionary_bin.py`.
- **Header (16 bytes)**: magic `"TKDB"` (4) · `version: u32` (currently `2`) · `count: u32` · `build_ts: u32`.
- **Offset table**: `count × u32` absolute byte offsets (LE).
- **Record**: `bitmask: u16 · frequency: u32 · hanzi_len: u8 · tl_len: u8 · syllable_count: u8 · hanzi_bytes · tl_bytes` — UTF-8 strings, fixed-layout prefix. v2 (v3.5.8 Phase 1) added the `syllable_count` byte for span-local candidate ranking.
- **Bitmask (13 bits)**: bits 0–7 = eight text sources (`kautian, taigitv, itaigi, sitbut, taihoa, taijit, kungge, stti`); bits 8–11 = four Hoklo-sourced dicts (`khpoo, khiin, dev, lkk`); bit 12 = `is_variant`.

### Readers (Rust-only post-Phase IV-B)

| Consumer | Entry point | Mapping |
|---|---|---|
| Rust core | `engine/lexicon/src/dictionary_reader.rs` | `mmap_host::MmapHandle::open_readonly` + manual LE field reads |

iOS (`DictionaryBinaryReader.swift`) and Android (`DictionaryBinaryReader.kt`) byte-level readers were deleted in v3.5.6 / Path G (PR #199); platforms now copy the bundled `.bin` to a writable location and hand the path to `engine/lexicon::EngineHandle::install`.

### UTF-8 error policy

The Rust reader returns `None` for the whole record when `from_utf8` fails for either `hanzi` (when `hanzi_len > 0`) or `tl`. This is stricter than the pre-Phase-IV-B platform contract, which preserved records on bad `hanzi` bytes by setting `hanzi = nil/null`. No callers depend on the looser behavior.

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

- **Producer**: `dictionary/build/create_association_bin.py`.
- **Header (20 bytes)**: magic `"TKWA"` (4) · `version: u32` (currently `1`) · `key_count: u32` · `entry_count: u32` · `build_ts: u32`.
- **Cohesion contract**: `build_ts` **must match** `dictionary.bin`. The build pipeline shares `.build_ts` between the two writers (`dictionary/build/create_dictionary_bin.py` and `dictionary/build/create_association_bin.py`). Readers on both platforms expose `buildTimestamp`.
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
    UNIQUE(prev_word, prev_tl, next_word, next_tl)
);
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

Dictionary updates today: `dictionary.fst` + `dictionary.bin` + `association.bin` are regenerated by the Python build pipeline (the fst step shells to Rust `engine/build-helpers/fst-builder`) and shipped in the app bundle (iOS) / assets (Android). No OTA channel exists. `build_ts` in `dictionary.bin` and `association.bin` is the only version signal readers expose.

**Invariants the delivery mechanism must preserve**:

1. The two binary artifacts with a header (`dictionary.bin`, `association.bin`) carry a matching `build_ts` — enforced by the shared `.build_ts` file in the build pipeline.
2. `dictionary.fst` has **no timestamp or version in its bytes** — the format is a raw Burntsushi fst. Today the three artifacts' cohesion relies entirely on the build script producing all three in the same run; readers cannot detect a stale fst paired with fresh bins (see `binary-format.md` §5.1 no-checksum acknowledgement).
3. User-writable SQLite databases (`user_frequency.db`, `user_association.db`, `custom_dictionary.db`) are per-install and must not be shipped as read-only assets. They stay native (`status=wont_migrate`) per `feedback_user_data_sqlite_stays_native`.
4. Schema migrations run on first open after an app update; the delivery mechanism does not modify these files directly.

Distribution-channel design (OTA vs app-bundle) is out of scope for this audit.

---

## Decision register

| # | Item | Status |
|---|---|---|
| D1 | MARISA lib strategy (C++ bind vs Rust port) | **Resolved 2026-05-02** — chose Rust-native `fst` (v3.5.6 / PR #199); MARISA C++ bridges deleted under Path G. |
| D2 | `dictionary.bin` + `association.bin` UTF-8 error policy | **Resolved** — Rust readers in `engine/lexicon` follow the platform "hanzi-optional, tl-required" contract; invalid records return `null`/`None`. |
| D3 | `dictionary.bin` + `association.bin` version-bump policy | **Resolved** — `dictionary.bin` is at `version: u32 = 2` (v3.5.8 Phase 1, added `syllable_count`); `association.bin` remains at `version: u32 = 1`. Rust readers reject mismatch at open time, with `dictionary.bin` v1 surfacing an explicit `v1→v2` rebuild message. |
| D4 | Lift bitmask semantics to single shared-core enum | **Resolved** — bitmask constants now live in Rust `engine/lexicon` (`KHIIN_BIT`, `VARIANT_BIT`; the unread `DEV_BIT` was dropped 2026-09-05). Platform `EnabledDictionaries` DTOs mirror the layout for UI toggles only. |
| D5 | `custom_dictionary` version-namespace unification | **Open** — both platforms keep native SQLite (`wont_migrate`); unification only matters if a future Rust slice ever owns custom-dict writes (no plan to do so). |
| D6 | Lift `CustomDictionaryDerivation` to shared core | **Open** — currently `native_pending` in `migration-inventory.csv`; could be folded into `engine/lexicon::key_normalizer` if user-data write path ever moves. |
| D7 | Cross-artifact cohesion check for the shipped trio | **Open** — see `binary-format.md` §5.1 (no checksum acknowledgement); revisit only if OTA delivery ships. |

---

## 7. Android addendum — storage paths, asset copy, update-in-place

Originally authored as Phase II A10 deliverable (2026-04-21). Updated for v3.5.6 fst migration. Pure append — iOS §§1–6, §Update / delivery, and §Decision register are untouched. Cross-references back into those sections by anchor; does not restate their content.

### 7.1 On-disk paths

| Artifact | Android on-disk location | Access mechanism |
|---|---|---|
| `dictionary.fst` | `{filesDir}/dictionary.fst` (copied from `assets/dictionary.fst`) | Rust `engine/lexicon` via `mmap-host::MmapHandle` |
| `dictionary.bin` | `{filesDir}/dictionary.bin` (copied from `assets/dictionary.bin`) | Rust `engine/lexicon::dictionary_reader` via `mmap-host` |
| `association.bin` | `{filesDir}/association.bin` (copied from `assets/association.bin`) | Rust `engine/lexicon::association_reader` via `mmap-host` |
| `user_frequency.db` | `{databases}/user_frequency.db` (`SQLiteOpenHelper`-managed) | `SQLiteOpenHelper.readableDatabase` / `writableDatabase` |
| `user_association.db` | `{filesDir}/user_association.db` (**not** under `databases/`) | `SQLiteDatabase.openOrCreateDatabase(File, null)` |
| `custom_dictionary.db` | `{databases}/custom_dictionary.db` (`SQLiteOpenHelper`-managed) | `SQLiteOpenHelper.readableDatabase` / `writableDatabase` |

`{filesDir}` = `Context.getFilesDir()` (`/data/user/0/<pkg>/files/`). `{databases}` = `Context.getDatabasePath(name).parentFile` (`/data/user/0/<pkg>/databases/`).

Divergence — `user_association.db` lives in `filesDir/`, not `databases/`. It was introduced with a direct `openOrCreateDatabase(File, null)` call rather than `SQLiteOpenHelper`, and the path stuck. Any future tool that enumerates DBs through `Context.getDatabasePath(...)` will miss it. Kept as-is to avoid migrating existing installs; flagged here as platform-only trivia (no cross-platform mapping implication since iOS has no database directory convention).

### 7.2 Asset copy semantics — shipped trio

All three read-only artifacts ship inside the APK at the assets root, from the repo-root `dictionaries/` that `build.gradle.kts` adds as an assets source dir. They are copied to `{filesDir}` on boot so the Rust mmap layer can map a real filesystem path. Rust `engine/lexicon` (via `mmap-host`) opens them through `memmap2`; AAPT2-compressed zip entries cannot be mapped directly, hence the copy.

| Artifact | Stamp file | Copier call-site |
|---|---|---|
| `dictionary.fst` | `{filesDir}/dictionary_app_version.txt` *(shared stamp for the trio)* | `LexiconService.copyAssetsIfNeeded` (Android Kotlin) |
| `dictionary.bin` | `{filesDir}/dictionary_app_version.txt` *(shared stamp for the trio)* | `LexiconService.copyAssetsIfNeeded` |
| `association.bin` | `{filesDir}/dictionary_app_version.txt` *(shared stamp for the trio)* | `LexiconService.copyAssetsIfNeeded` |

The copier reads the stamp, compares against `BuildConfig.VERSION_CODE`, and re-copies all three assets if the stamp is behind — OR if any destination file is missing regardless of stamp. Post-v3.5.6, the historical separate `trie_app_version.txt` stamp is gone (TrieService deleted under Path G); a single stamp now covers all three artifacts.

**Cohesion risk — D7 sub-case.** The three artifacts move together because they share one stamp file written after the copy loop succeeds. An interruption between artifact writes can still leave a partial-copy state on disk; recovery requires `BuildConfig.VERSION_CODE` to bump again or a manual app-data clear. No manifest hash or combined check exists today.

### 7.3 SQLite open mechanism — three patterns

Android runs three distinct SQLite-init patterns across the three runtime DBs. Terminal schemas still match iOS §§4–6; the mechanism divergence is platform-internal.

| DB | Mechanism | Version stamp | Migration shape |
|---|---|---|---|
| `user_frequency.db` | `SQLiteOpenHelper` | `DATABASE_VERSION = 1` (Helper-managed) plus `metadata.schema_version = "1"` text row | `onUpgrade` is a no-op (no migrations have ever shipped on this DB) |
| `user_association.db` | `SQLiteDatabase.openOrCreateDatabase` — no Helper | `PRAGMA user_version` (hand-rolled) — currently `6` | `ensureUserAssocSchema` runs ONE convergent `rebuildToV6` (create-new / copy preserving `id` / drop / rename) for every pre-v6 DB that has a table, then the terminal DDL and the version stamp — all in one transaction. A v2 stamp is dropped rather than rebuilt (its key shape is ambiguous). The old v0→v2→v3→v4→v5 ladder is deleted. |
| `custom_dictionary.db` | `SQLiteOpenHelper` | `DATABASE_VERSION = 5` (Helper-managed) | `onUpgrade` (`CustomDictionaryService.kt:421`) chains `migrateV1ToV2` → `migrateV2ToV3` → `migrateV3ToV4` → `migrateV4ToV5` |

Two of the four `custom_dictionary.db` steps (`v2→v3`, `v3→v4`) are pure derivation-logic regenerations with no DDL; see iOS §6 Migration-mechanism table for the divergence on version-namespace semantics (D5).

**One-shot journal-mode migration (parity with iOS)**. `NextWordService.migrateFromWAL` (`NextWordService.kt:523`) runs on every `user_association.db` open, detects `PRAGMA journal_mode = wal`, executes `PRAGMA wal_checkpoint(TRUNCATE)`, then switches to `PRAGMA journal_mode = DELETE`. Mirrors iOS `SQLiteConnectionManager.swift:42-98`. `user_association.db` is the only Android DB with an explicit WAL→DELETE migration; `user_frequency.db` and `custom_dictionary.db` go through `SQLiteOpenHelper` and take the platform default for journal mode without a corresponding switch. The asymmetry is accepted for Phase II (no cross-process writer on those two DBs); §9 gating signal applies only to the user-association path.

### 7.4 Update-in-place — APK upgrade semantics

1. **Read-only trio** — installing a new APK bumps `BuildConfig.VERSION_CODE`. On next boot `LexiconService` detects a stale stamp and overwrites `{filesDir}/dictionary.fst`, `{filesDir}/dictionary.bin`, `{filesDir}/association.bin` with the new asset bytes. **Partial-copy semantics** — the copier writes through `FileOutputStream(destFile)` (no append, no tmp-and-rename), which truncates the destination to 0 bytes on open. An interruption mid-stream (process kill, I/O error, power loss) leaves the target truncated. On the next boot Rust readers fail their magic / version / fst-header checks; `RustEngineBridge.install` returns a `FailIo` error and the platform surfaces `DictionaryError.DatabaseNotAvailable`. Recovery paths: (a) a version-bump boot always re-runs the copy because `currentAppVersion > lastCopiedVersion`; (b) a same-`VERSION_CODE` boot with a truncated-but-present destination is **not** auto-repaired today — leaving the keyboard degraded until app-data clear or reinstall. No OTA channel.
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
- Phase II gate #8: closed (Phase II audit doc retired post-completion).
- Android exemplar roster and marker convention: `docs/architecture/android-exemplar.md` §§3, 5 (I/O wrappers are explicitly excluded from the candidate roster there).
- Candidate-class markers on the Android readers: `DictionaryBinaryReader.kt` / `AssociationBinaryReader.kt` are platform-only I/O wrappers by construction (mmap over `MappedByteBuffer`) and are not carried on the Shared-Core candidate roster.

---

## Out of scope

- Rust FFI surface design — Phase IV-A.
- OTA / incremental-update delivery — Phase IV-B.
- UI assets (keyboard layouts, fonts, icons, tab1 content JSON).
- Persisted settings (`SharedSettings` · Android DataStore blobs · `colorSettings`) — not shared-core candidates.
- Build-time-only artifacts — none. Post-v3.5.6 part 2 the build pipeline reads `dictionary.csv` directly into the binary writers; the previous SQLite intermediates (`dictionary.db`, `trie.db`) are gone.
