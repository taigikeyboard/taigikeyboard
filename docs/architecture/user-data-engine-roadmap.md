# User Data in the Engine — Roadmap

> **Type**: Planning (design record + PR table; becomes Reference once shipped)
> **Keywords**: `user data`, `SQLite`, `rusqlite`, `user_frequency`, `user_association`, `custom_dictionary`, `learned_phrases`, `.taigi`, `migration`, `engine`
> **Status**: P0 #219 and P1 #220 merged (2026-09-26); P2 in progress.
> **Session memory**: project memory `project_user_data_engine.md` (Claude auto-memory)
> **Supersedes**: the "user-data SQLite stays platform-native" rule (`.claude/rules/rust-migration-policy.md` §6, rewritten 2026-09-26) and every `wont_migrate` row for the four stores in `docs/engine/migration-inventory.csv`

---

## Summary

- **Decision** (USER 2026-09-26): "moving the shared implementation into the engine is what makes sense; it keeps the implementation consistent". The engine owns the four user-writable SQLite stores; each platform supplies a directory and the UI, nothing else.
- **Why now**: four hand-written implementations (iOS, Android, macOS, `desktop/` Rust for Windows + Linux) have drifted (§ Drift), and the next suggestion features (brainstorm `docs/reports/2026-09-24-mobile-smart-suggestions-brainstorm.md` batch A/B: shown-counter column, two-word context) are schema changes that would otherwise be written four times. This is also brainstorm R5: once the engine reads user frequency itself, the two-call candidate fetch on every platform disappears.
- **Starting point**: `desktop/crates/taigi-desktop-storage` already implements all four stores in Rust (rusqlite 0.40 `bundled`, ~2.9k lines, Windows + Linux in production, SQL stated byte-identical to macOS). It moves down into a new engine crate; the phones and macOS then switch to it.
- **Shape**: extract (behaviour freeze) → engine lifecycle + migration → engine-side reads/writes → admin codec ops → switch one platform at a time (all four stores per platform at once) → cleanup. Release scope and timing are USER-gated.

---

## Today (grounded in code, 2026-09-26 audit)

### Who reads and writes what

| Path | Today | File:line |
|---|---|---|
| Candidate fetch | Every platform: neutral `FetchAtPos` → SQL frequency lookup on the returned display texts → second `FetchAtPos` with `frequency_entries` + `now_ms`; custom and learned rows pushed as `custom_entries` / `learned_entries` | iOS `ios/Sources/TaigiKeyboard/Input/Composing/ComposingManager.swift:225-310`; Android `android/.../ime/text/composing/ComposingManager.kt:486-560`; macOS `macos/Sources/TaigiInputMethodCore/Composing/ComposingManager.swift:297-321`; desktop `desktop/crates/taigi-desktop-core/src/composing/manager.rs:242-301`; proto `engine/protos/proto/composing.proto:193-233` |
| Next-word prediction | Platform SQL (`CASE prev_tl` tiers, then `count DESC, last_used DESC, id ASC`) → `PredictNext.user_rows`; first row wins a collision (behavioral-invariants §24) | iOS `NextWord/Repository/NextWordRepository.swift:144-187`; Android `ime/dictionary/NextWordService.kt:91-101`; engine `engine/dispatch/src/predict.rs:15-41` |
| Association write | Engine emits `RecordAssociation` / `RecordCompoundAssociations`; platform persists | `engine/nextword/src/decide.rs:76-156`; iOS `NextWord/NextWordController.swift:175-204`; desktop `taigi-desktop-core/src/composing/learner.rs:69-82` |
| Learned-phrase write | Engine emits `Effect.phrase_learned` ("the engine decides, the platform persists") | `engine/composing/src/transition.rs:854-856`; `composing.proto:515-526` |
| Frequency write | No effect; platform records on tap (desktop / macOS gate on a frequency-recording setting) | iOS `Actions/ActionHandler+Suggestions.swift:175,231`; Android `CandidateClickHandler.kt:143`; macOS `ComposingManager.swift:518-519`; desktop `manager.rs:451-453` |
| Custom-dict search keys | Engine derives (`DeriveCustomSearchKeys` / `DeriveCustomQueryKey`); platform runs the SQL | `engine/phonetics/src/dispatch.rs:74,82` |
| Engine file paths | Read-only mmaps via `LexiconRequest.install`; no writable directory concept | `engine/lexicon/src/api.rs:22-35`, `paths.rs:32-55` |

### The four implementations

| | iOS | Android | macOS | Desktop Rust (Win + Linux) |
|---|---|---|---|---|
| SQLite | system (`import SQLite3`) | system, minSdk 28 → 3.22 (no UPSERT / `RETURNING` / `DROP COLUMN`) | system | rusqlite 0.40.2 bundled (`desktop/Cargo.toml:44`) |
| Location | App Group container (`Lexicon/Database/SharedDatabasePath.swift:12-20`) | `databases/`; `user_association.db` in `filesDir` (`NextWordService.kt:193`) | `~/Library/Application Support/<bundle id>/` (`Storage/UserDataDirectory.swift:30-54`) | `%APPDATA%` / XDG (`taigi-desktop-storage/src/directory.rs:27-30`) |
| Processes | main app + keyboard extension share files | one (IME + settings) | one (settings window in the IME process) | IME host + settings app |
| Journal | `DELETE` pinned, `synchronous=NORMAL`, `cache_size=10000`; one-time WAL→DELETE (`SQLiteConnectionManager.swift:62-88,114-121`) | default; association WAL→DELETE on open (`NextWordService.kt:467-493`) | default (`SQLiteConnection.swift:83-104`) | WAL (`database.rs:153-154`) |
| busy timeout | **none** | none | 250 ms | writer 250 ms, reader 0 + `try_lock` |
| Threading | serial queue per store, hot-path `queue.sync` | `Dispatchers.IO` + init mutex | serial queue per file | writer thread + bounded queue (256, drops when full) + read-only reader (`database.rs:47-76,195-230`) |
| Backup exclusion | `isExcludedFromBackup` (§29) | `allowBackup="false"` + rules exclude all (`AndroidManifest.xml:23-25`) | deliberately not excluded (named divergence) | n/a |
| `.taigi` | JSON v2 export/import (`Lexicon/Services/BackupService.swift`) | JSON v2 (`ime/dictionary/BackupService.kt`) | none (pane retired) | none |

Schema versions: `user_frequency` v2 and `user_association` v6 and `learned_phrases` v1 agree everywhere. `custom_dictionary` has **three version namespaces for one logical schema** (portability D5):

| Logical step | Android `DATABASE_VERSION` | iOS `user_version` | macOS / desktop |
|---|---|---|---|
| derived columns (`notone`/`abbrev`/`roman_num`) | 1–5 | 1 | — |
| `custom_search_key` side table | 6 | 2 | — |
| POJ glyph re-derive | 7 | 3 | — |
| §46 abbreviation re-derive | 8 | 4 | 4 |
| unreleased learned-in-custom shape (`origin`, `learn_count`) | 9 | 5 | — |
| learned phrases moved out (§50) | 10 | 6 | — (drops leftovers via `DROP COLUMN`) |

Sources: `CustomDictionaryService.kt:36-42,533-547`; `CustomDictionarySchema.swift:39`, `CustomDictionaryMigrator.swift:6-84`; `macos/.../Storage/CustomDictionaryStore.swift:51-60`; `taigi-desktop-storage/src/custom_dictionary.rs:19,567-603`. iOS and Android still write the legacy derived columns for rollback.

### Drift the audit found

Each is a symptom of four implementations; each is fixed once by the engine store, not per platform.

1. iOS app + extension share files with no `busy_timeout`; `SQLITE_BUSY` fails at once and fire-and-forget writes swallow it.
2. iOS and Android wipe `user_frequency` (and iOS `user_association`) by unlinking the file while another connection may hold it (`UserFrequencyRepository.swift:176-194`, `NextWordService.swift:219-236`, `UserFrequencyService.kt:387-401`); learned phrases deliberately `DELETE` instead (`LearnedPhraseRepository.swift:156-158`).
3. CSV: iOS parser toggles on `"` while export doubles quotes — a quoted field does not round-trip (`CustomDictionaryService.swift:166-192`); Android splits on `\n` before unquoting (`CustomDictionaryService.kt:481-489`).
4. iOS custom-dictionary timestamps use a `DateFormatter` without `en_US_POSIX` (`CustomDictionaryRepository.swift:444-449`).
5. Reset scope differs (iOS wipes three stores, Android differs in scope and error handling — `SettingsResetCoordinator.kt:14-19`).
6. Restore sets `last_used = now` and exports `lastUsed: ""` on both phones.
7. Journal mode, busy timeout and threading all differ (table above); desktop's "byte-identical" claim holds only against macOS.

---

## Design (Codex ANALYSIS-ONLY pre-review 2026-09-26 applied)

**U1 — One engine crate `engine/userdata`.** Moved from `taigi-desktop-storage`: `database`, `capacity`, `frequency`, `association`, `custom_dictionary`, `learned_phrases`, `csv`, `timestamp`. The row types and store traits move down from `taigi-desktop-core` (`composing/stores.rs:8-41`, `engine/composing.rs:30-52`, `engine/nextword.rs:22`, `engine/phonetics.rs:21`); the search-key deriver becomes a direct call into `phonetics`. The injectable `SearchKeyDeriver` seam (an `Option`-returning closure built for the proto round-trip) stays through P1 so the moved store bodies are byte-identical; P4 drops it and the stores call `phonetics` directly. `directory.rs`, `settings_file.rs`, `font_library.rs` stay in `desktop/`. New dependencies: rusqlite (bundled), uuid (custom-dictionary ids). rusqlite 0.40.2 / libsqlite3-sys 0.38.2 declare no `rust-version`; the engine stays on 1.86 unless the moved code needs newer syntax (checked in P1, not assumed). No `unsafe` in the moved code; the workspace `forbid` holds.

**U2 — SQLite: rusqlite `bundled` on every platform.** One SQLite version and dialect everywhere, which also removes Android's 3.22 ceiling. On Apple the process then carries a second SQLite beside the system `libsqlite3`; that is safe only while no file is opened by both copies, so a platform switches all four stores in one PR (U6) and native code never opens these files again. The iOS keyboard extension's cost (binary size, memory — note iOS pins `cache_size=10000`) is measured by an on-device spike before the iOS phase.

**U3 — Journal mode is a platform parameter of `Open`; schema and SQL are not.** iOS keeps `DELETE` (App Group two-process access; §29 backup exclusion covers no `-wal`/`-shm` sidecars; Apple `0xdead10cc` terminates a suspended process that holds a file lock — Apple TN2408, "SIGKILL" doc). Desktop keeps WAL (its reader/writer split relies on it, `database.rs:47-56`). Android and macOS keep today's `DELETE`/default unless the spike shows a reason. Every platform gains a `busy_timeout`.

**U4 — Two write classes.** Learning writes (frequency, association, learned phrase) go through one serial writer, queued and non-blocking on the keystroke path; queue-full drops a learning write (desktop behaviour today, and the phones' fire-and-forget writes lose the same class of data). Admin writes (custom-dictionary CRUD, import, reset, `.taigi` restore) are synchronous and return the transaction result. Keystroke-path reads stay synchronous. The semantics of queue-full, process exit and read-after-write are written into the P2 spec. Reset uses `DELETE` in a transaction, never file unlink.

**U5 — One semantic source, two call styles.** Swift and Kotlin reach the stores only through proto ops (`UserDataRequest` domain: `Open{directory, journal_mode, layout}`, `Reset`, typed CRUD — never paths or raw SQL beyond `Open`). Windows and Linux may call `engine/userdata` in-process through the same Rust API that the ops dispatch to. iOS app and extension are separate processes; each opens its own handle.

**U6 — Switch per platform, all four stores at once.** No store is ever written by native code and the engine on the same platform at the same time. Extraction and deletion PRs exceed the usual 200–500 LOC; that is accepted over a store-by-store dual-owner transition.

**U7 — Migration by schema shape, not version number.** A convergent migrator inspects columns, indexes and unique keys (the way iOS `CustomDictionaryMigrator` and association `rebuildToV6` already do), migrates in one transaction, and keeps a copy of the file taken before the first engine migration. An unrecognised (future) shape fails safe: the store stays closed and ranking falls back to neutral — today's cold-start behaviour. `user_version` is a hint only. Golden fixture files for every known shape: iOS custom v6 with legacy columns, Android v10 with and without v9 `origin`/`learn_count`, macOS/desktop v4, frequency v1 without `tl`, association v3–v5, Android `filesDir` association.

*As built in P2:* the takeover marker is `PRAGMA application_id` = `TAIG` (`engine/userdata/src/database.rs` `TAIGI_APPLICATION_ID`; no platform's native store sets one). Opening a file checks, in order: `user_version` above the store's highest known stamp (frequency 2, association 6, learned 1, custom dictionary 10) → closed, untouched; not yet taken over and holding tables → `VACUUM INTO <file>.pre-engine` once (failure keeps the store closed; the next launch retries); journal mode; the store's shape-detecting `apply`; the marker. Frequency: a table without `tl` is rebuilt (ids / counts / timestamps kept, `tl = ''`). Association: below 6, v2 is dropped (both phones' rule), any other table carrying `id` / `prev_word` / `next_word` / `count` / `last_used` is rebuilt under the v6 key with `COALESCE`d readings, anything else dropped — NAMED DIVERGENCE: iOS used to drop every pre-v3 table, the engine keeps one that has the columns, as Android did for v0/v1. Custom dictionary: taken over only once `rederive_search_keys_if_needed` has re-derived every entry (always on the first engine open — Android v7 is iOS v3, so no stamp says which derivation wrote the keys), stamp raised to 4 but never lowered (Android keeps 10, iOS 6), the phones' legacy `notone` / `abbrev` / `roman_num` columns left in place (their native code still writes them), the unreleased v9 `origin` / `learn_count` pair dropped column by column. Fixtures: `engine/userdata/tests/takeover.rs`, each built from the DDL the platform shipped (commit cited per fixture).

**U8 — Downgrade matrix (decided in P2).** An old app reopening a taken-over file must not silently corrupt it. Traps: Android `SQLiteOpenHelper` throws on a higher version (`onDowngrade` default), and a `currentVersion >= schemaVersion` check (`NextWordService.kt:520-543`, `NextWordSchema.swift:27-42`) would skip migrations. Hence no "one number above every namespace" stamp; each store keeps a number that platform's own older release accepts.

| Older build reopening… | Result |
|---|---|
| Windows / Linux ≤ 3.6.10, a file this engine took over | Same SQL (these stores were the engine's code since P1); `application_id` and `<file>.pre-engine` are ignored. Compatible. |
| macOS native store, a file the engine took over (after P6) | Same SQL and stamps (custom v4). Compatible. |
| iOS / Android native store, an existing file the engine took over | Stamps never lowered: freq 2, assoc 6, learned 1; custom dictionary `max(v, 4)` — iOS released 0 – 3 (last `mobile-3.6.8` = 3) → 4, Android 3 → 4, 5 / 7 / 10 kept (fixtures for every released shape, P8a / P7a). Legacy columns kept, so no migration re-runs and native writes still succeed. Compatible — except a file from before `roman_num` (v3.4.2) taken over and then reopened by an older iOS build: its v3 migrator sees 4 ≥ 3, skips, and its inserts name the missing `roman_num` (TestFlight downgrade after skipping every release since v3.4.2 only). The unreleased v9 `origin` / `learn_count` columns are gone, which no released build reads. |
| iOS / Android native store, a custom dictionary the ENGINE created fresh | **Incompatible**: no legacy columns and stamp 4, so the old app's own migrator and its `notone` inserts fail. Reachable only by a build downgrade that keeps app data: TestFlight's previous builds or `adb install -d` on a debuggable build. The App Store serves only the current version and Google Play refuses a lower `versionCode`, so store users cannot reach it. |

**U9 — Transition compatibility is temporary.** Until P9, the engine honours the old row fields (`frequency_entries`, `custom_entries`, `learned_entries`, `PredictNext.user_rows`) and the persistence effects only when no store is open for that process. It never merges DB rows with proto rows (double weighting / double learning). Temporary, removed in P9. P9a: the engine no longer honours the `FetchAtPos` rows nor emits `Effect.PhraseLearned`. P9b: nor `PredictNext.user_rows` nor the association effects. P9c: the wire fields and effects are `reserved` — U9 closed.

**U10 — Admin surface.** Custom-dictionary CRUD, CSV codec and search-key management are engine ops. `.taigi` is an engine codec (export → bytes, import ← bytes, format v2 compatible both ways, learned phrases still excluded per §50); file pickers, share sheets and UI stay on the platform. The frequency-recording setting stays a platform setting passed with the commit op; learned-phrase recording stays always-on.

**U11 — The stores sit behind a `sqlite` feature; the types and traits do not.** `windows/Makefile` `check-msvc` must keep `cargo check`ing the msvc target from macOS for the crates it lists (`taigi-windows-platform`, `taigi-windows-update`, both over `taigi-desktop-core`) (verified 2026-09-26: bundled SQLite's build script fails there, `stdlib.h` not found). `userdata`'s default `sqlite` feature carries rusqlite + uuid and the stores; `desktop-core` takes the crate with `default-features = false` (row types, store traits, the search-key deriver), `taigi-desktop-storage` turns `sqlite` on. Cargo unifies features per build, so a build that also selects `taigi-desktop-storage` compiles SQLite for `desktop-core`'s copy too — exactly as before P1, when storage carried rusqlite; only builds without storage (the `check-msvc` roster) stay C-free. P3 meets the same constraint: once `dispatch` reads the stores, `desktop-core → dispatch` would pull SQLite in. *Decided in P3a (Codex pre-review 2026-09-26):* `dispatch` gains a `user-data` feature (`src/user_data.rs`, pulling `userdata` with SQLite); it is in `dispatch`'s own defaults so `cargo test -p dispatch` covers it, and every workspace (engine, desktop, windows, linux) declares `dispatch` with `default-features = false`, so `swift-ffi`, `android-jni` and `taigi-desktop-core` link no SQLite until a platform's switch PR turns `dispatch/user-data` on (P5 the desktop shells, P6 `swift-ffi` — which also ships in the iOS xcframework — P8 `android-jni`). Verified: `cargo tree -p swift-ffi` / `-p android-jni` / `-p taigi-desktop-core` carry no `libsqlite3-sys`. Without the feature a `user_data` request answers `FAIL_INVARIANT`.

---

## Phases

| # | PR | Type | Content | Status |
|---|---|---|---|---|
| 0 | admin | docs | this roadmap; `rust-migration-policy.md` §6 rewritten; project memory | Merged #219 `7a1af073` |
| 1 | `refactor(engine): move user-data stores into engine/userdata` | Refactor — Windows/Linux behaviour freeze | U1 extraction; `taigi-desktop-storage` keeps desktop-only modules and re-uses the engine crate; `refactor-reviewer` + `/code-review` | Merged #220 `1a1c0730` |
| 2 | `feat(engine): user-data takeover and migration` | Feature (engine, `userdata` crate only) | U3 journal parameter (`JournalMode`) + reader wait under a rollback journal; per-file `UserDataPaths`; U7 takeover marker, future-version guard, pre-takeover copy, shape-detecting migrations + fixtures; U8 matrix. No platform caller yet: the proto ops and the process-wide handle moved to P3 with the dispatch wiring, where U11's fork is decided | Merged #222 `e392f670` |
| 3a | `feat(engine): user-data open and reset ops` | Feature (engine) | `user_data.proto` (`OpenUserData` / `ResetUserData`, envelope tag 15); process-wide handle in `dispatch` behind the `user-data` feature (U11 decided above); `UserDataStores::open_blocking` opens, re-derives and seeds before answering which stores are ready — the one open path, so no caller has to remember `rederive_search_keys_if_needed` again; platform bindings regenerated | Merged #223 `8972cda8` |
| 3b | `feat(engine): engine reads user data` | Feature (engine) | `dispatch` answers `FetchAtPos` from the open stores: the pending buffer (`composing::EngineHandle::pending_raw`) → query key → custom (unless the new `FetchAtPos.custom_dictionary_disabled`) + learned rows → neutral fetch → frequency rows for its candidates → re-ranked fetch, all in-process (one FFI call — brainstorm R5); `PredictNext` reads `user_association.db` with the new `PredictNext.roman` (§24 tiers, `UserAssociationStore::rows_following`); rows a platform still sends are replaced, never merged (U9); equivalence test: engine reads answer exactly what the platform rows answered | Merged #224 `e479af96` |
| 3c | `feat(engine): engine writes user data` | Feature (engine) | once the stores are open the engine persists what it decides — `Effect.phrase_learned` into `learned_phrases.db`, `NextWordEffect.record_association` / `record_compound_associations` into `user_association.db` — and leaves those effects out of the response; the pick-time writes the platform decides (candidate or prediction tap: frequency count + learned-phrase touch) become `UserDataRequest.record_usage`, sent where each platform called `recordUsage`, with the desktop's recording setting as `frequency_recording_disabled` | Merged #225 `0a126e1b` |
| 4a | `feat(engine): custom dictionary page ops` | Feature (engine) | U10: `ListCustomEntries` (page + `total` + `matching_total`), `SaveCustomEntry` (add / edit), `DeleteCustomEntry`, `ImportCustomCsv` / `ExportCustomCsv` (bytes; the engine's one CSV codec — drift 3), user-facing outcomes as `CustomDictionaryRefusal` (Codex pre-review 2026-09-26); delete-all stays `ResetUserData` | Merged #226 `43dc7e65` |
| 4b | `feat(engine): .taigi backup codec` | Feature (engine) | `.taigi` v2 export / import in the engine: lenient decode (missing fields default, unknown keys ignored, version ≥ 1), iOS-style encode (sorted keys, `lastUsed: ""`), custom rows through `batch_import` skipping empty Hanji and `origin == 1` (Android rule), frequency / association `MAX(count)` merges with `last_used` = now and POJ → TL readings, then capacity enforced (new: the phones never trimmed after an import) | Merged #227 `bced0d84` |
| 5 | `refactor(desktop): Windows + Linux use engine user data` | Platform switch | the IME path: one `FetchAtPos` (the engine reads frequency, custom dictionary and learned phrases), picks through the new `UsageRecorder` seam (`EngineUsage` → `record_usage`; `NoUsage` with no data directory), `PhraseLearned` and associations persisted by the engine; the shells tell the engine to open the user data on the first key (`OpenUserData.in_background`: stores in use before the call returns, the takeover finished on the engine's own thread — nothing reported in the first seconds is lost); `ComposingSessionCoordinator::for_desktop` builds the manager for both shells; `dispatch/user-data` on in `taigi-windows-tsf` and `taigi-linux-core`. Kept: the learner's `AssociationSink` (production `NoStores`) so the desktop's own context rules stay testable — U9, removed in P9; the settings apps keep the in-process `userdata` API (U5) — the dictionary search pages need a search-key lookup no op offers | Merged #228 `cad683d9` |
| 6 | `refactor(macos): use engine user data` | Platform switch | one `FetchAtPos` (`customDictionaryDisabled` carries the setting); picks through `UsageRecorder` (`EngineUsageRecorder` → `record_usage`); `PhraseLearned` a no-op; `NextWordLearner` hands bigrams to an `AssociationSink` (production `EngineOwnedAssociations`, U9 as on the desktop); `OpenUserData` at launch over `~/Library/Application Support/<bundle id>/`, `DELETE` journal (U3), `in_background`; the Custom Dictionary page and the unlisted Dictionary Search pane through `UserDataClient` (engine ops, off the main actor). Engine additions: `SearchCustomEntries` (the dictionary search's key-prefix lookup — Android needs it in P8), page requests wait for a background open to finish (settled once in `handle`; `RecordUsage` never waits), `ResetUserData` attempts every selected store and names each failure (`UserDataReset.failures`), `OpenUserData.directory` (the engine applies the shared file names; the desktop sends it too), `ListCustomEntries` pulls an offset past the end back to the last page and answers `offset`, refusals carry the engine's own `detail` line. swift-ffi `user-data` feature, on only in the macOS xcframework build (iOS stays SQLite-free until P7; CI guard). `Storage/` stores, CSV codecs and 70 store tests deleted (engine-covered) | Merged #229 `c34c35b4` |
| S | iOS spike (no PR) | Spike | bundled rusqlite in the xcframework; open an App Group DB from the extension; binary size, extension memory, app + extension concurrent writes, suspension with no held lock | Passed 2026-09-26 (iPhone 12): Release extension +2.33 MB; app open 7 ms / extension 23 ms, +~1 MB each, extension footprint 23 MB of 64; both processes open the same files; no journal left idle |
| 7a | `feat(engine): released iOS user-data shapes; takeover waits out the other process` | Feature (engine, `userdata` only) | takeover fixtures at iOS numbering — custom dictionary stamps 0 (no `roman_num`), 0 / 1 (no side table), 2, 3 (`mobile-3.6.8`) → 4, shared with Android's in one table; association v4 (with `idx_user_prev_word`) — all pass unchanged. The open and the custom dictionary's takeover wait up to 10 s for another process's lock (the iOS app and its keyboard both open the App Group files on a first launch; a failed open stays closed for the process), then back to 250 ms. U8 corrected (iOS released custom v3, not v6) | In progress |
| 7b | `refactor(ios): use engine user data` | Platform switch | `UserDataOpening.open(in:)` in both processes after `RustEngineBridge.install` — the keyboard only with Full Access (without it the container is read-only), the app never under XCTest (`TEST_HOST` = app); App Group directory, `DELETE`, `in_background`, then a waiting repeat open marks every file and `<file>.pre-engine` copy excluded from backup (§29); one `FetchAtPos` (`nowMs`, `customDictionaryDisabled`); picks → `UsageRecorder` (`EngineUsageRecorder`, skipped when this process has not opened); `PhraseLearned` + association effects no-op (U9); `PredictNext.roman` with the engine call off the main actor; Dictionary pages, search, backup, reset through `UserDataClient`; swift-ffi `default = ["user-data"]` (the macOS flag and the iOS CI guard go); native repositories / schemas / migrators / services / `SQLiteConnectionManager` / derive ops and their tests deleted | Merged #233 `3b213e25` |
| 8a | `feat(engine): released Android user-data shapes` | Feature (engine, `userdata` only) | takeover fixtures from the DDL every Android release shipped — custom dictionary v3 (first release, no `roman_num`, no side table), v5, v7 (`mobile-3.6.8`), frequency v1 with its `metadata` table — all pass unchanged; the engine creates a file's directory on open (Android's `databases/` exists only once something wrote there) | Merged #230 `c046fc6b` |
| 8b | `refactor(android): use engine user data` | Platform switch | `OpenUserData` first in `TaigiKeyboardApplication.onCreate` (`databases/` + `filesDir/user_association.db` override, `DELETE`, `in_background`); one `FetchAtPos` (`nowMs`, `customDictionaryDisabled`); picks → `UsageRecorder` (`EngineUsageRecorder` → `RecordUsage`, Hanji on the continuous path); `PhraseLearned` + association effects persisted by the engine (arms no-op, U9); `PredictNext.roman`, no user rows, lexicon wait moved into `NextWordController`; settings through `UserDataClient` on `Dispatchers.IO` — the list is one `ListCustomEntries` with `limit` 0 (every match — new engine rule; client-side filter kept), CSV import pre-read 5 MB check, backup via `ExportBackup` / `ImportBackup`, reset via `ResetUserData`; android-jni `default = ["user-data"]`, build script checks both ABIs + arm64 16 KB alignment; 10 services / helpers (~2.6k lines) and 6 SQL tests deleted | Merged #231 `004532dc` |
| 9a | `refactor(engine): user rows are engine-internal` | Refactor (engine) | composing takes the user rows as Rust types (`composing::UserRows` on `Intent::FetchAtPos`, `EngineHandle::query`) and hands the phrase a final commit taught back beside the response (`Applied.learned`, `EngineHandle::handle_learning`) — no `Effect.PhraseLearned`; `decode_intent` ignores the wire rows; `ranking::build_frequency_map` (proto) gone, dispatch builds the `FrequencyMap` from the store rows; composing / lexicon / dispatch tests off the wire rows (test-side `Fetch`) | Merged #234 `452e94b5` |
| 9b | `refactor(engine): association rows and records are engine-internal` | Refactor (engine + desktop / macOS learners) | `PredictNext` user rows passed as an argument into the `FilterPredictions` build (wire `user_rows` ignored); nextword hands the bigrams a decision records back beside the response (`nextword::Association`, `Handled`, `EngineHandle::handle_recording`) — no `RecordAssociation` / `RecordCompoundAssociations` effects; dispatch writes one decision's pairs in one transaction, drops them before the open. The desktop and macOS managers report handshakes through a `NextWordPort` (production `EngineNextWord`); `NextWordLearner`, `userdata::AssociationSink` / `NoStores`, macOS `AssociationSink` / `EngineOwnedAssociations` and the desktop / macOS effect decode go; manager tests assert the handshakes, the decisions stay the engine's (Codex F5; `a_comma_between_two_commits_keeps_the_pair`, `a_hanji_only_commit_pairs_with_an_empty_tl` added); iOS bridge record asserts dropped | Merged #235 `0cc15c0d` |
| 9c | `chore: reserve the retired user-data wire fields` | Cleanup | `reserved` FetchAtPos 2/4/7, PredictNext 2, Effect 11, NextWordEffect 3/4; messages `FrequencyEntry` / `CustomDictEntry` / `LearnedEntry` / `PhraseLearned` / `AssociationPair` / `RecordAssociation` / `RecordCompoundAssociations` removed; the proto comments that described platform-side persistence rewritten (`nextword.proto` header / `PredictNext` / `FilterPredictions`, `composing.proto` `FetchAtPos`, `user_data.proto` `RecordUsage`); Swift / Java bindings regenerated (stale Java classes deleted); the dead decode cases and no-op arms go on iOS (`NextWordAssociationPair`, `phraseLearned`), Android, macOS, desktop, Linux, Windows; engine tests drop the retired effect names | Merged #236 `6486b9db` |
| 9d | `docs: user data is the engine's` | Docs | `migration-inventory.csv` (user-data rows `rust_shipped`, `userdata` module rows), `data-artifacts-portability.md` (§4–8, D5 resolved), `system-overview.md`, `behavioral-invariants.md` (§2, §10, §24, §26–29, §40, §46, §50, §54), `nextword-engine-boundary.md`, `custom-dictionary.md`, `nextword.md`, `sort.md`, `keywords.md`, `docs/README.md`, `rust-core-proto.md`, `ffi-safety.md`, `ios-exemplar.md`, `macos-roadmap.md` (superseded notes), `ios-shared-core-candidates.md`; engine comments that said they mirror the retired platform stores now name them as the source they were ported from | In progress |

Each platform switch carries a dogfood item (`docs/architecture/dogfood-checklist.md`) covering: existing data survives the upgrade (frequency ranking, predictions, custom entries, learned phrases), reset, CSV round-trip, `.taigi` round-trip.

Until P9 merges, new suggestion features land in the engine store only (§6); brainstorm batch A/B waits for the platforms that will use it.

---

## Best practices alignment

**Rules per phase**

| Phase | Constraining rule |
|---|---|
| 1, 6, 7, 8 | `~/.claude/rules/round-workflow.md` § Workflow types — Refactor: behaviour-freeze boundary, every caller listed; `refactor-reviewer` |
| 2, 3, 4 | `.claude/rules/rust-migration-policy.md` §6 (engine-owned) + `.claude/rules/rust-best-practices.md` §1a (dependency direction: `userdata` below `dispatch`, beside `lexicon`) |
| 2 | `~/.claude/rules/diagnosis-discipline.md` § Verify hard-prerequisite claims — every migration shape proven by a fixture built from a real file |
| 3 | `~/.claude/rules/planning.md` § No redundant fallback — U9 is temporary and removed in P9 |
| 7 | `.claude/rules/taigi-incidents.md` — spike a platform-capability assumption before planning on it (phase S) |
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

---

## Review record

- 2026-09-26 Codex ANALYSIS-ONLY pre-review (design forks F1–F9): recommendations adopted as U1–U10. Changes from the draft: journal mode made a platform parameter (was "unify"); write classes split (was "reuse desktop queue"); version stamping replaced by shape detection + a downgrade matrix (was "one version above every namespace"); `.taigi` UI kept on the platform; frequency-recording setting preserved; engine MSRV not raised by assumption.
