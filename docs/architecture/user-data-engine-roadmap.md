# User Data in the Engine — Roadmap

> **Type**: Design record (roadmap done)
> **Keywords**: `user data`, `SQLite`, `rusqlite`, `user_frequency`, `user_association`, `custom_dictionary`, `learned_phrases`, `.taigi`, `migration`, `engine`
> **Status**: Done — P0–P9d merged; user data is engine-owned on every platform (shipped in mobile v3.6.10 and desktop v3.7.0).
> **Supersedes**: the "user-data SQLite stays platform-native" rule (`docs/contributing/rust-migration-policy.md` §6, rewritten 2026-09-26) and every `wont_migrate` row for the four stores in `docs/engine/migration-inventory.csv`

---

## Summary
- **Decision**: the engine owns the four user-writable SQLite stores (custom dictionary, frequency, association, learned phrases); each platform supplies a directory and the UI, nothing else, so the implementation stays consistent.
- **Why**: four hand-written implementations (iOS, Android, macOS, `desktop/` Rust for Windows + Linux) had drifted (busy timeouts, unsafe wipes, CSV round-trip, reset scope, journal mode), and every new suggestion feature would otherwise be a schema change written four times. Once the engine reads user frequency itself, the two-call candidate fetch disappears.
- **Starting point**: `desktop/crates/taigi-desktop-storage` already implemented all four stores in Rust (rusqlite `bundled`); they moved down into `engine/userdata`, and the phones and macOS switched to it.
- **Shape**: extract (behaviour freeze) → engine lifecycle + migration → engine-side reads/writes → admin codec ops → switch one platform at a time (all four stores per platform at once) → cleanup.

---

## Design

**U1 — One engine crate `engine/userdata`.** Moved from `taigi-desktop-storage`: `database`, `capacity`, `frequency`, `association`, `custom_dictionary`, `learned_phrases`, `csv`, `timestamp`. The stores call `phonetics` directly for search keys. The single-impl store traits and their mirror row types are gone (maintainability R4 PR-B, 2026-09-30) — `dispatch` calls the stores' own `rows_matching`, each store owns its row type (`FrequencyRow` in `frequency.rs`, `AssociationPair` in `association.rs`), and the one search-key type is `phonetics::api::CustomSearchKey`. `directory.rs`, `settings_file.rs`, `font_library.rs` stay in `desktop/`. New dependencies: rusqlite (bundled), uuid (custom-dictionary ids). No `unsafe`; the workspace `forbid` holds.

**U2 — SQLite: rusqlite `bundled` on every platform.** One SQLite version and dialect everywhere, which also removes Android's 3.22 ceiling. On Apple the process then carries a second SQLite beside the system `libsqlite3`; that is safe only while no file is opened by both copies, so a platform switches all four stores in one PR (U6) and native code never opens these files again. The iOS keyboard extension's cost (binary size, memory — note iOS pins `cache_size=10000`) is measured by an on-device spike before the iOS phase.

**U3 — Journal mode is a platform parameter of `Open`; schema and SQL are not.** iOS keeps `DELETE` (App Group two-process access; §29 backup exclusion covers no `-wal`/`-shm` sidecars; Apple `0xdead10cc` terminates a suspended process that holds a file lock — Apple TN2408, "SIGKILL" doc). Windows and Linux keep WAL (their reader/writer split relies on it). Android and macOS keep `DELETE`. Every platform gains a `busy_timeout`.

**U4 — Two write classes.** Learning writes (frequency, association, learned phrase) go through one serial writer, queued and non-blocking on the keystroke path; queue-full drops a learning write (desktop behaviour today, and the phones' fire-and-forget writes lose the same class of data). Admin writes (custom-dictionary CRUD, import, reset, `.taigi` restore) are synchronous and return the transaction result. Keystroke-path reads stay synchronous. The semantics of queue-full, process exit and read-after-write are written into the P2 spec. Reset uses `DELETE` in a transaction, never file unlink.

**U5 — One semantic source, one call style.** Every platform reaches the stores only through proto ops (`UserDataRequest` domain: `Open{directory, journal_mode, layout}`, `Reset`, typed CRUD — never paths or raw SQL beyond `Open`). Windows and Linux were allowed to call `engine/userdata` in-process through the same Rust API the ops dispatch to; their settings windows were the last such caller and switched to the ops on 2026-09-30 (`taigi_desktop_core::engine::user_data`, the `taigi-desktop-storage` re-export shim and the `desktop-core → userdata` edge gone with it). iOS app and extension are separate processes; each opens its own handle.

**U6 — Switch per platform, all four stores at once.** No store is ever written by native code and the engine on the same platform at the same time. Extraction and deletion PRs exceed the usual 200–500 LOC; that is accepted over a store-by-store dual-owner transition.

**U7 — Migration by schema shape, not version number.** A convergent migrator inspects columns, indexes and unique keys (the way iOS `CustomDictionaryMigrator` and association `rebuildToV6` already do), migrates in one transaction, and keeps a copy of the file taken before the first engine migration. An unrecognised (future) shape fails safe: the store stays closed and ranking falls back to neutral — today's cold-start behaviour. `user_version` is a hint only. Golden fixture files for every known shape: iOS custom v6 with legacy columns, Android v10 with and without v9 `origin`/`learn_count`, macOS/desktop v4, frequency v1 without `tl`, association v3–v5, Android `filesDir` association.

*As built in P2:* the takeover marker is `PRAGMA application_id` = `TAIG` (`engine/userdata/src/database.rs` `TAIGI_APPLICATION_ID`; no platform's native store sets one). Opening a file checks, in order: `user_version` above the store's highest known stamp (frequency 2, association 6, learned 1, custom dictionary 10) → closed, untouched; not yet taken over and holding tables → `VACUUM INTO <file>.pre-engine` once (failure keeps the store closed; the next launch retries); journal mode; the store's shape-detecting `apply`; the marker. Frequency: a table without `tl` is rebuilt (ids / counts / timestamps kept, `tl = ''`). Association: below 6, v2 is dropped (both phones' rule), any other table carrying `id` / `prev_word` / `next_word` / `count` / `last_used` is rebuilt under the v6 key with `COALESCE`d readings, anything else dropped — NAMED DIVERGENCE: iOS used to drop every pre-v3 table, the engine keeps one that has the columns, as Android did for v0/v1. Custom dictionary: taken over only once `rederive_search_keys_if_needed` has re-derived every entry (always on the first engine open — Android v7 is iOS v3, so no stamp says which derivation wrote the keys), stamp raised to 4 but never lowered (Android keeps 10, iOS 6), the phones' legacy `notone` / `abbrev` / `roman_num` columns left in place (their native code still writes them), the unreleased v9 `origin` / `learn_count` pair dropped column by column. Fixtures: `engine/userdata/tests/takeover.rs`, each built from the DDL the platform shipped (commit cited per fixture).

**U8 — Downgrade matrix (decided in P2).** An old app reopening a taken-over file must not silently corrupt it. Traps: Android `SQLiteOpenHelper` throws on a higher version (`onDowngrade` default), and the phones' native `currentVersion >= schemaVersion` check (Android `NextWordService`, iOS `NextWordSchema`) would skip migrations. Hence no "one number above every namespace" stamp; each store keeps a number that platform's own older release accepts.

| Older build reopening… | Result |
|---|---|
| Windows / Linux ≤ 3.6.10, a file this engine took over | Same SQL (these stores were the engine's code since P1); `application_id` and `<file>.pre-engine` are ignored. Compatible. |
| macOS native store, a file the engine took over (after P6) | Same SQL and stamps (custom v4). Compatible. |
| iOS / Android native store, an existing file the engine took over | Stamps never lowered: freq 2, assoc 6, learned 1; custom dictionary `max(v, 4)` — iOS released 0 – 3 (last `mobile-3.6.8` = 3) → 4, Android 3 → 4, 5 / 7 / 10 kept (fixtures for every released shape, P8a / P7a). Legacy columns kept, so no migration re-runs and native writes still succeed. Compatible — except a file from before `roman_num` (v3.4.2) taken over and then reopened by an older iOS build: its v3 migrator sees 4 ≥ 3, skips, and its inserts name the missing `roman_num` (TestFlight downgrade after skipping every release since v3.4.2 only). The unreleased v9 `origin` / `learn_count` columns are gone, which no released build reads. |
| iOS / Android native store, a custom dictionary the ENGINE created fresh | **Incompatible**: no legacy columns and stamp 4, so the old app's own migrator and its `notone` inserts fail. Reachable only by a build downgrade that keeps app data: TestFlight's previous builds or `adb install -d` on a debuggable build. The App Store serves only the current version and Google Play refuses a lower `versionCode`, so store users cannot reach it. |

**U9 — Transition compatibility is temporary.** Until P9, the engine honours the old row fields (`frequency_entries`, `custom_entries`, `learned_entries`, `PredictNext.user_rows`) and the persistence effects only when no store is open for that process. It never merges DB rows with proto rows (double weighting / double learning). Temporary, removed in P9. P9a: the engine no longer honours the `FetchAtPos` rows nor emits `Effect.PhraseLearned`. P9b: nor `PredictNext.user_rows` nor the association effects. P9c: the wire fields and effects are `reserved` — U9 closed.

**U10 — Admin surface.** Custom-dictionary CRUD, CSV codec and search-key management are engine ops. `.taigi` is an engine codec (export → bytes, import ← bytes, format v2 compatible both ways; v4 adds learned phrases and the learned rows' last-use times, USER 2026-11-20, #1032); file pickers, share sheets and UI stay on the platform. The frequency-recording setting stays a platform setting passed with the commit op; learned-phrase recording stays always-on.

**U11 — SQLite links only through `dispatch/user-data`.** `userdata` has no features — it always builds the stores with bundled SQLite — and its only edge is `dispatch`'s optional `user-data` feature, the one gate on SQLite linkage (`dispatch` always compiles `src/user_data.rs`; only `user_data/with_stores.rs` needs the feature). The feature is in `dispatch`'s own defaults so `cargo test -p dispatch` covers it, and every workspace declares `dispatch` with `default-features = false`; each shell turns it on on its own edge: `swift-ffi`, `taigi-macos-ffi`, `android-jni`, `taigi-linux-core` and `taigi-web` directly, the Windows TIP and the Windows / Linux settings windows through `taigi-desktop-core`'s `user-data` feature (maintainability R8, 2026-11-16). `taigi-desktop-core` itself links no SQLite, so `windows/Makefile` `check-msvc` can keep `cargo check`ing its C-free crates for the msvc target from macOS (bundled SQLite's build script fails there, `stdlib.h` not found). Without the feature a `user_data` request answers `FAIL_INVARIANT`.

---

## Phases

All merged; one line per phase (labels are cited from code).

| Phase | Content |
|---|---|
| P1 | U1 extraction: the four stores move from `taigi-desktop-storage` into `engine/userdata` (Windows / Linux behaviour freeze) |
| P2 | U3 journal parameter, per-file paths, U7 takeover marker, future-version guard, pre-takeover copy, U8 downgrade matrix |
| P3a | `OpenUserData` / `ResetUserData` ops; process-wide handle in `dispatch` behind `user-data` (U11) |
| P3b | The engine reads user data: `FetchAtPos` answered from the open stores |
| P3c | The engine writes user data: learned phrases, associations, usage |
| P4a | Custom-dictionary page ops (U10): list, save, delete, CSV import / export |
| P4b | `.taigi` backup codec in the engine |
| P5 | Windows + Linux switch: one `FetchAtPos`, picks through the `UsageRecorder` seam |
| P6 | macOS switch |
| S | iOS spike: bundled rusqlite in the extension, App Group DB, memory, concurrent writes, suspension |
| P7a / P7b | Released iOS store shapes; iOS switch (both processes) |
| P8a / P8b | Released Android store shapes; Android switch |
| P9a | User rows are engine-internal (composing takes Rust types) |
| P9b | Association rows and records are engine-internal |
| P9c | Retired user-data wire fields `reserved` |
| P9d | Docs: user data is the engine's |

Each platform switch carried a dogfood item covering: existing data survives the upgrade (frequency ranking, predictions, custom entries, learned phrases), reset, CSV round-trip, `.taigi` round-trip. New suggestion features land in the engine store only (`rust-migration-policy.md` §6).

---

## Best practices alignment

**Rules per phase**

| Phase | Constraining rule |
|---|---|
| 1, 6, 7, 8 | `docs/contributing/cross-platform-alignment.md` §1 — refactor: behaviour-freeze boundary, every caller listed; `refactor-reviewer` |
| 2, 3, 4 | `docs/contributing/rust-migration-policy.md` §6 (engine-owned) + `docs/contributing/rust-best-practices.md` §1a (dependency direction: `userdata` below `dispatch`, beside `lexicon`) |
| 2 | Verify hard-prerequisite claims — every migration shape proven by a fixture built from a real file |
| 3 | `AGENTS.md` § Design principles (no redundant fallback) — U9 is temporary and removed in P9 |
| 7 | `docs/contributing/known-pitfalls.md` — spike a platform-capability assumption before planning on it (phase S) |
| all | `docs/architecture/behavioral-invariants.md` §24 (prediction order), §29 (backup exclusion), §46, §50 |

**Mainstream practice**

| Mainstream practice | Source file:line | This plan (phase) |
|---|---|---|
| Taigi IME core owns its user DB through rusqlite `bundled` with versioned migrations, shared by Windows / Android / iOS shells | `references/khiin-rs/khiin/Cargo.toml:25-26` (`rusqlite` bundled + `rusqlite_migration`), `khiin/src/db/migrations` | U1, U2, U7 (P1–P2) |
| One core library owns the user dictionary for every frontend (Squirrel, Weasel, fcitx5-rime, ibus-rime) | `references/librime/src/rime/dict/user_db.cc`, `user_dictionary.cc` | U5, U6 (P5–P8) |
| An iOS keyboard extension runs the core's user DB in-process | `references/Hamster` (iOS keyboard over librime) | spike S, P7 |
| Learning history lives in the converter core, not the client | `references/mozc/src/prediction/user_history_predictor.cc` | P3 |
| Bounded learning store with a stated eviction order | `references/ChiaKey/.../Manjusri/Headers/LanguageModel.h:97-122` | unchanged caps (20k / 50k / 30k / 2k) move as-is (P1) |

**Deliberately not adopted**

- **Out-of-process store server** (mozc converter server, rakukan engine host, hazkey socket server): our engine is in-process on every platform; a server adds IPC and lifecycle for no data-consistency gain.
- **One journal mode everywhere**: rejected for U3 — iOS App Group suspension and desktop reader/writer needs differ; consistency lives in schema and SQL.
- **`rusqlite_migration` crate**: our migrations are shape-detecting (U7), not a linear version list; the three custom-dictionary namespaces do not fit a single sequence.
- **Store-by-store migration across platforms**: rejected for U6 (dual owner, two SQLite copies on one file).
- **Moving `.taigi` file UI into the engine**: file pickers are OS UI (U10).
- **YAGNI**: no encryption layer, no sync, no new backup contents — none is asked for; add when a feature needs it.
