# Data Artifact Portability Audit

How the data artifacts that back the IME are produced, stored and consumed on iOS and Android (§1–§7) and on macOS / Windows / Linux (§8), plus the decision register. An inventory of constraints, not a design doc.

- **Current state**: the dictionary read path is Rust `engine/lexicon`; the four SQLite user-data stores are Rust `engine/userdata` on every platform. §4–§8 describe the engine stores; the native shapes they took over are named only where they still matter (migration, downgrade).
- **Scope**: shipped and runtime artifacts whose byte layout or schema crosses platforms. UI assets (keyboard layouts, fonts, images), persisted preferences (`SharedSettings` / DataStore blobs), and logs are out of scope.

## Summary

| Artifact | Status | Owner |
|---|---|---|
| `dictionary.fst` (Burntsushi fst) | **Shipped in Rust** | Rust `engine/lexicon::prefix_index::PrefixIndex` |
| `dictionary.bin` | **Shipped in Rust** | Rust `engine/lexicon::dictionary_reader` |
| `association.bin` | **Shipped in Rust** | Rust `engine/lexicon::association_reader` |
| `user_frequency.db` (SQLite) | **Shipped in Rust** | Rust `engine/userdata::UserFrequencyStore` (`frequency.rs`) |
| `user_association.db` (SQLite) | **Shipped in Rust** | Rust `engine/userdata::UserAssociationStore` (`association.rs`) |
| `custom_dictionary.db` (SQLite) | **Shipped in Rust** | Rust `engine/userdata::CustomDictionaryStore` (`custom_dictionary.rs`) |
| `learned_phrases.db` (SQLite) | **Shipped in Rust** | Rust `engine/userdata::LearnedPhraseStore` (`learned_phrases.rs`; shape in `behavioral-invariants.md` §50) |

Per `docs/contributing/rust-migration-policy.md` §6, the writable user-data DBs are engine-owned: the platform supplies the directory (`OpenUserData`), the journal mode and the UI, and reaches the stores only through `UserDataRequest` ops (Windows / Linux settings apps also in-process). The engine takes over the files each platform's native store wrote (shape-detecting migration, `<file>.pre-engine` copy, `PRAGMA application_id`; `user-data-engine-roadmap.md` U7 / U8). The three read-only assets are byte-identical across iOS and Android and are consumed by the Rust crate via `mmap-host`.

---

## 1. `dictionary.fst` — Burntsushi fst prefix index

Burntsushi `fst` finite-state transducer holding dictionary keys (`tl:`, `poj:`, `hanzi:` plus toneless / abbrev / numeric variants) mapped to rowids.

### Pipeline

- **Producer**: `dictionary/build/create_fst.py` shells to the Rust binary `engine/build-helpers/fst-builder`. Value layout: rowid packed in the low 32 bits of the `u64` value; high bits reserved.
- **Shipped path**: the repo-root `assets/dictionaries/`, committed once. iOS reads it through an Xcode synchronized folder and Android through an assets source dir, both without a copy; macOS (`macos/scripts/bundle-app.sh`) and Windows (`windows/scripts/release-app.sh`) copy from it at package time.
- **Size**: ~9.1 MB, byte-identical on both platforms (same build, same file).

### Reader

| Platform | Entry point |
|---|---|
| iOS | `RustEngineBridge.search` / `searchByHanji` / `searchWithSources` → Rust `engine/lexicon` |
| Android | `LexiconBridge.search` / `searchByHanji` / `searchWithSources` → same Rust crate |

Rust `engine/lexicon::prefix_index::PrefixIndex` opens the file via `mmap-host` (the only crate not `forbid(unsafe_code)`) and exposes `Map::range` / `Map::get` for prefix scans + exact lookup.

---

## 2. `dictionary.bin` — dictionary entry table

Custom binary format holding per-rowid dictionary entries.

### Format

Authoritative spec lives in [`docs/engine/binary-format.md`](../engine/binary-format.md). Summary:

- **Producer**: `dictionary/build/create_dictionary_bin.py`.
- **Header (40 bytes)**: magic `"TKDB"` (4) · `version: u32` (currently `5`) · `count: u32` · `build_ts: u32` · `tokens_N: u64` · `vocabulary_V: u64` · `alpha: f64`. The model metadata supplies the scale for custom and learned-word costs.
- **Offset table**: `count × u32` absolute byte offsets (LE).
- **Record**: `bitmask: u16 · frequency: u32 · hanzi_len: u8 · tl_len: u8 · syllable_count: u8 · kautian_subtag: u16 · walker_cost: u16 · hanzi_bytes · tl_bytes` — UTF-8 strings, fixed-layout prefix. `syllable_count` feeds span-local candidate ranking, `kautian_subtag` is the kautian subcollection, `walker_cost` the walker model cost.
- **Bitmask (13 bits)**: bits 0–7 = eight text sources (`kautian, taigitv, itaigi, sitbut, taihoa, taijit, kungge, stti`); bits 8–11 = four Hoklo-sourced dicts (`khpoo, khiin, dev, lkk`); bit 12 = `is_variant`.

### Reader

| Consumer | Entry point | Mapping |
|---|---|---|
| Rust core | `engine/lexicon/src/dictionary_reader.rs` | `mmap_host::MmapHandle::open_readonly` + manual LE field reads |

No platform has a byte-level reader of its own; each hands the bundled path to `engine/lexicon::EngineHandle::install`.

### UTF-8 error policy

The Rust reader returns `None` for the whole record when `from_utf8` fails for either `hanzi` (when `hanzi_len > 0`) or `tl`. The decode failure is not logged.

---

## 3. `association.bin` — next-word lookup table

Read-only next-word bigram/phrase table. Sibling to `dictionary.bin` with a distinct header and a narrower bitmask.

### Format

- **Producer**: `dictionary/build/create_association_bin.py`.
- **Header (20 bytes)**: magic `"TKWA"` (4) · `version: u32` (currently `2`) · `key_count: u32` · `entry_count: u32` · `build_ts: u32`.
- **Cohesion contract**: `build_ts` **must match** `dictionary.bin`. Both writers (`dictionary/build/create_dictionary_bin.py` and `dictionary/build/create_association_bin.py`) fill it with `build.common.build_id()` — the CRC-32 of the `output/dictionary.csv` they are both built from, chained over `word_unigrams.tsv` and the walker α — so one build always matches and unchanged inputs (`dictionary.csv`, the corpus count TSVs, α) rebuild byte-identical. The engine readers (`lexicon::DictionaryReader` / `AssociationReader`) expose it as `build_timestamp()`.
- **Key offset table**: `key_count × u32` absolute offsets.
- **Key entry**: `prev_word_len: u8 · prev_word: utf8 · entry_offset: u32 · entry_count: u16`. Keys sorted by UTF-8 byte order for binary search.
- **Key namespaces (v2)**: one hanji (intra-word pairs from `dictionary.csv`), `hanji\u{1}tl` word keys (corpus pairs from `dictionary/shared/data/word_bigrams.tsv`) — `docs/engine/binary-format.md` §2.1a.
- **Entry**: `bitmask: u16 · count: u32 · next_word_len: u8 · next_tl_len: u8 · next_word: utf8 · next_tl: utf8`.
- **Bitmask (9 bits)**: `kautian, taigitv, itaigi, sitbut, taihoa, taijit, kungge, stti, khpoo`. Subset of dictionary.bin bits 0–8. No `is_variant`, no `khiin / dev / lkk`.

### Readers

| Platform | Entry point |
|---|---|
| Rust core | `engine/lexicon/src/association_reader.rs` — `mmap_host` read-only map, binary search on the key offset table; `lookup(prev_word, limit)` + `build_timestamp()` |

Same UTF-8 policy as `dictionary.bin`.

---

## 4. Runtime SQLite — `user_frequency.db`

Per-install word-usage counter. Drives candidate ranking (`behavioral-invariants.md` §28). Engine-owned on every platform: `engine/userdata/src/frequency.rs`.

### Schema (engine store, every platform)

```sql
CREATE TABLE user_frequency (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    word TEXT NOT NULL,
    tl TEXT NOT NULL DEFAULT '',
    count INTEGER DEFAULT 1,
    last_used TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(word, tl)
);
-- Indexes (the UNIQUE autoindex serves `WHERE word IN`)
CREATE INDEX idx_count ON user_frequency(count DESC);
CREATE INDEX idx_last_used ON user_frequency(last_used DESC);
-- PRAGMA user_version = 2
```

Takeover: a pre-pair-key table (inline `word UNIQUE`, no `tl` — both phones before R5) is rebuilt with `tl = ''`, gated on the shape, not the stamp (`migrate_to_pair_key_if_needed`); Android's side `metadata(key, value)` table is left in place.

### Pruning (one implementation)

| Constant | Value | Source |
|---|---|---|
| `maxEntries` | 20000 | `UserFrequencyStore::shipped_capacity` |
| Record-check interval | 100 | `LearningCapacity::DEFAULT_RECORDS_BETWEEN_CHECKS` |
| Prune batch size | 2000 | `UserFrequencyStore::shipped_capacity` |
| Selection order | `count ASC, last_used ASC` | `LearningCapacity::enforce` (the order both phones pruned in) |

`rusqlite` with `bundled` SQLite (one SQLite version on every platform) in `engine/userdata`, linked only through `dispatch/user-data`.

---

## 5. Runtime SQLite — `user_association.db`

User-learned next-word associations. Sibling of the shipped `association.bin` for the write path. Engine-owned on every platform: `engine/userdata/src/association.rs`.

### Schema (engine store, every platform)

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

Capacity: 50000 rows, pruned 5000 at a time in the same `count ASC, last_used ASC` order (`UserAssociationStore::shipped_capacity`). Reads: `UserAssociationStore::rows_following`, whose tier order is `behavioral-invariants.md` §24. Writes: the bigrams a nextword decision records, one decision in one transaction (`UserAssociationStore::record`).

### Migration mechanism

- **One engine migrator** (`engine/userdata/src/association.rs::apply_schema`), `PRAGMA user_version` = 6 — the number iOS (`NextWordSchema.schemaVersion`) and Android (`NextWordService.DATABASE_VERSION`) had already reached, so an older phone build reopening the file skips its own migrations (roadmap U8).
- **Takeover**: any pre-v6 table is converged in one `BEGIN IMMEDIATE` transaction — v2 dropped (its key shape is ambiguous, as both phones treated it); any other table carrying `id` / `prev_word` / `next_word` / `count` / `last_used` rebuilt under the v6 key with `id` copied and `COALESCE`d readings (`rebuild_to_v6`; subsumes iOS v3–v5 and Android v0/v1 + v3–v5); anything else dropped. NAMED DIVERGENCE: iOS used to drop every pre-v3 table, the engine keeps one that has the columns (roadmap U7).

**Journal mode**: the platform picks it on `OpenUserData.journal` and the engine sets it on every open (roadmap U3) — `DELETE` on iOS, Android and macOS (a phone file still in WAL is switched to `DELETE` on open, as both native stores' one-time WAL → DELETE migration did), WAL on Windows and Linux.

---

## 6. Runtime SQLite — `custom_dictionary.db`

User-authored custom dictionary entries with derived search keys. Engine-owned on every platform: `engine/userdata/src/custom_dictionary.rs`.

### Schema (engine store, every platform)

```sql
CREATE TABLE custom_dictionary (
    id TEXT PRIMARY KEY,              -- UUID
    roman TEXT NOT NULL,
    hanzi TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_custom_roman ON custom_dictionary(roman);
CREATE TABLE custom_search_key (
    entry_id TEXT NOT NULL,
    family   TEXT NOT NULL,           -- tl / poj / tps
    form     TEXT NOT NULL,           -- num / notone / abbrev
    key      TEXT NOT NULL
);
CREATE INDEX idx_csk_lookup ON custom_search_key(family, form, key);
CREATE INDEX idx_csk_entry  ON custom_search_key(entry_id);
```

A file the phones wrote keeps its legacy derived columns `notone` / `abbrev` / `roman_num` (and their indexes): the takeover leaves them so an older phone build's inserts still succeed (roadmap U8), but the engine neither writes nor reads them (`behavioral-invariants.md` §10). Capacity: 30000 rows, never evicted (`behavioral-invariants.md` §27).

### Derived-key logic

The search keys are pure functions of `roman`, derived in-process by the engine (`userdata::derive_custom_search_keys` → `phonetics::custom_search`, `behavioral-invariants.md` §10 / §26) on every write; the query key the same way (`phonetics::api::derive_custom_query_key`). Write-time and read-time keys come from one module, so they agree by construction.

### Migration mechanism — one engine takeover over three native namespaces

| Concern | Native (before the engine) | Engine |
|---|---|---|
| Version tracking | iOS `PRAGMA user_version`; Android `SQLiteOpenHelper.DATABASE_VERSION`; macOS / desktop `user_version` 4 — one logical schema in three namespaces (D5) | `PRAGMA user_version` read as a hint only; shape detected from columns / tables; taken-over marker `PRAGMA application_id` (roadmap U7) |
| Derivation version | iOS column-presence migrator could not tell a derivation change from the current state; Android recorded pure re-derivations (v2→v3, v3→v4) as version steps | every entry's keys re-derived once on the first engine open (`rederive_search_keys_if_needed`), whatever the stamp |
| Stamp written | — | raised to 4, never lowered (a higher stamp — Android 5 / 7 / 10, iOS 6 — is kept); above 10 → the store stays closed, untouched |

The unreleased v9 `origin` / `learn_count` columns (learned phrases parked in this table, `behavioral-invariants.md` §50) are dropped column by column on takeover (`drop_learned_rows_if_present`). Fixtures for every released shape: `engine/userdata/tests/takeover.rs`.

---

## Update / delivery

Dictionary updates today: `dictionary.fst` + `dictionary.bin` + `association.bin` are regenerated by the Python build pipeline (the fst step shells to Rust `engine/build-helpers/fst-builder`) and shipped in the app bundle (iOS) / assets (Android). No OTA channel exists. `build_ts` in `dictionary.bin` and `association.bin` is the only version signal readers expose.

**Invariants the delivery mechanism must preserve**:

1. The two binary artifacts with a header (`dictionary.bin`, `association.bin`) carry a matching `build_ts` — both are the CRC-32 of the same `output/dictionary.csv`, chained over `word_unigrams.tsv` and the walker α (`dictionary/build/common.py::build_id`).
2. `dictionary.fst` has **no timestamp or version in its bytes** — the format is a raw Burntsushi fst. Today the three artifacts' cohesion relies entirely on the build script producing all three in the same run; readers cannot detect a stale fst paired with fresh bins (see `binary-format.md` §5.1 no-checksum acknowledgement).
3. User-writable SQLite databases (`user_frequency.db`, `user_association.db`, `custom_dictionary.db`, `learned_phrases.db`) are per-install and must not be shipped as read-only assets. They are engine-owned per `docs/contributing/rust-migration-policy.md` §6.
4. Schema migrations run in the engine on the first open after an app update (`OpenUserData`; roadmap U7); the delivery mechanism does not modify these files directly.

Distribution-channel design (OTA vs app-bundle) is out of scope for this audit.

---

## Decision register

| # | Item | Outcome |
|---|---|---|
| D1 | MARISA lib strategy | Rust-native `fst` (§1). |
| D2 | `dictionary.bin` + `association.bin` UTF-8 error policy | "hanzi-optional, tl-required"; an invalid record reads as `None` (§2). |
| D3 | `dictionary.bin` + `association.bin` version policy | `dictionary.bin` `version` = 4, `association.bin` = 2; the Rust readers reject a mismatch at open, `dictionary.bin` v1–v3 with an explicit rebuild message. |
| D4 | One bitmask definition | Constants in Rust `engine/lexicon` (`KHIIN_BIT`, `VARIANT_BIT`); platform `EnabledDictionaries` DTOs mirror the layout for UI toggles only. |
| D5 | `custom_dictionary` version namespaces | No unified number: the engine migrates by shape across the Android, iOS and desktop namespaces, re-derives every entry once, and raises the stamp to 4 without ever lowering it (§6). |
| D6 | Custom-dictionary key derivation | `engine/phonetics/src/derivation.rs`, called in-process by `engine/userdata` (§6). |
| D7 | Cross-artifact cohesion check for the shipped trio | **Open** — `binary-format.md` §5.1; revisit only if OTA delivery ships. |

---

## 7. Android addendum — storage paths, asset copy, update-in-place

### 7.1 On-disk paths

| Artifact | Android on-disk location | Access mechanism |
|---|---|---|
| `dictionary.fst` | `{filesDir}/dictionary.fst` (copied from `assets/dictionary.fst`) | Rust `engine/lexicon` via `mmap-host::MmapHandle` |
| `dictionary.bin` | `{filesDir}/dictionary.bin` (copied from `assets/dictionary.bin`) | Rust `engine/lexicon::dictionary_reader` via `mmap-host` |
| `association.bin` | `{filesDir}/association.bin` (copied from `assets/association.bin`) | Rust `engine/lexicon::association_reader` via `mmap-host` |
| `user_frequency.db` | `{databases}/user_frequency.db` | Rust `engine/userdata` (`OpenUserData.directory`) |
| `user_association.db` | `{filesDir}/user_association.db` (**not** under `databases/`) | Rust `engine/userdata` (`OpenUserData.association_path`) |
| `custom_dictionary.db` | `{databases}/custom_dictionary.db` | Rust `engine/userdata` (`OpenUserData.directory`) |
| `learned_phrases.db` | `{databases}/learned_phrases.db` | Rust `engine/userdata` (`OpenUserData.directory`) |

`{filesDir}` = `Context.getFilesDir()` (`/data/user/0/<pkg>/files/`). `{databases}` = `Context.getDatabasePath(name).parentFile` (`/data/user/0/<pkg>/databases/`).

Divergence — `user_association.db` lives in `filesDir/`, not `databases/`, so a tool that enumerates DBs through `Context.getDatabasePath(...)` misses it. Kept to avoid migrating existing installs: `RustEngineBridge.userDataOpen` (`engine/UserDataBridge.kt`, called first in `TaigiKeyboardApplication.onCreate`) passes it to the engine as a per-file override (`OpenUserData.association_path`); the engine creates `databases/` itself on a fresh install.

### 7.2 Asset copy semantics — shipped trio

All four read-only artifacts (`dictionary.fst`, `dictionary.bin`, `association.bin`, `syllables.fst`) ship inside the APK at the assets root, from the repo-root `assets/dictionaries/` that `build.gradle.kts` adds as an assets source dir. They are copied to `{filesDir}` on boot so the Rust mmap layer can map a real filesystem path. Rust `engine/lexicon` (via `mmap-host`) opens them through `memmap2`; AAPT2-compressed zip entries cannot be mapped directly, hence the copy.

The copier, `TaigiKeyboardApplication.installLexiconEngine`, keeps one stamp for all four, `{filesDir}/dictionary_app_version.txt`: it compares the stamp against `BuildConfig.VERSION_CODE` and re-copies all four when the stamp is behind or any destination file is missing.

**Cohesion risk — D7 sub-case.** The four artifacts move together because they share one stamp file written after the copy loop succeeds. An interruption between artifact writes can still leave a partial-copy state on disk; recovery requires `BuildConfig.VERSION_CODE` to bump again or a manual app-data clear. No manifest hash or combined check exists today.

### 7.3 SQLite open mechanism — one engine open

`OpenUserData` (journal `DELETE`, `in_background`) opens all four stores in the engine, which takes over each file by shape (§4–§6, roadmap U7) with the bundled SQLite — so Android's system SQLite 3.22 ceiling (no UPSERT / `RETURNING` / `DROP COLUMN`) does not apply. The engine sets the journal on every open: every Android store is `DELETE`.

### 7.4 Update-in-place — APK upgrade semantics

1. **Read-only artifacts** — installing a new APK bumps `BuildConfig.VERSION_CODE`. On next boot `TaigiKeyboardApplication.installLexiconEngine` detects a stale stamp and overwrites `{filesDir}/dictionary.fst`, `{filesDir}/dictionary.bin`, `{filesDir}/association.bin`, `{filesDir}/syllables.fst` with the new asset bytes. **Partial-copy semantics** — the copier writes through `FileOutputStream(destFile)` (no append, no tmp-and-rename), which truncates the destination to 0 bytes on open. An interruption mid-stream (process kill, I/O error, power loss) leaves the target truncated. On the next boot Rust readers fail their magic / version / fst-header checks; `RustEngineBridge.lexiconInstall` returns `null` and `CompositionRoot.lexiconReady` completes exceptionally (`DictionaryError` has only `QueryExecutionFailed`, the search-time failure). Recovery paths: (a) a version-bump boot always re-runs the copy because `currentAppVersion > lastCopiedVersion`; (b) a same-`VERSION_CODE` boot with a truncated-but-present destination is **not** auto-repaired today — leaving the keyboard degraded until app-data clear or reinstall. No OTA channel.
2. **User SQLite DBs** — `user_frequency.db`, `user_association.db`, `custom_dictionary.db`, `learned_phrases.db` (and the `<file>.pre-engine` copies the first engine takeover leaves beside them) all persist across APK replacement (both `filesDir` and `databases/` survive app update). Uninstalling the app is the only way to lose them. Schema migrations run in the engine on the first open after update per §7.3.
3. **DataStore** — the main `Preferences<Preferences>` file is `{filesDir}/datastore/taigi_keyboard_prefs.preferences_pb` (declared at `PreferenceKeys.kt:16`). A second DataStore `{filesDir}/datastore/emoji_preferences.preferences_pb` holds the emoji skin-tone selection (`EmojiPreferences.kt:20`). Both persist across APK replacement. `PrefHelper.migrateFromSharedPreferences` (`PrefHelper.kt:625`) pulls legacy Android `PreferenceManager` SharedPreferences values forward exactly once, gated on a DataStore-empty check.

DataStore blobs are listed for completeness only; they are not candidates for engine ownership (see *Out of scope*).

---

## 8. macOS / Windows / Linux stores

The desktops run the same engine stores as the phones (`engine/userdata`, the same schemas and stamps as §4–§6: `user_frequency` **v2**, `user_association` **v6**, `custom_dictionary` **v4**, `learned_phrases` **v1**, and the same `LearningCapacity` caps of 20 000 / 50 000 rows): macOS under `~/Library/Application Support/<bundle id>/`, opened with the `DELETE` journal at launch (`RustEngineBridge+UserData.swift`); Windows under `%APPDATA%\TaigiKeyboard\` and Linux under `$XDG_DATA_HOME/taigikeyboard`, opened with WAL (`taigi-desktop-core` `engine/user_data.rs`). The Windows / Linux settings apps read the custom dictionary in-process through the same `userdata` API. Neither desktop reads or writes `.taigi` backups (`macos-roadmap.md` § Settings pane roster); the custom dictionary has CSV import/export only. Desktop databases stay inside Time Machine / roaming-profile scope — the iOS OS-backup exclusion (`behavioral-invariants.md` §29) is not mirrored.

---

## Out of scope

- Rust FFI surface design.
- OTA / incremental-update delivery.
- UI assets (keyboard layouts, fonts, icons, tab1 content JSON).
- Persisted settings (`SharedSettings` · Android DataStore blobs · user themes) — not shared-core candidates.
- Build-time-only artifacts — none: the build pipeline reads `dictionary.csv` directly into the binary writers.
