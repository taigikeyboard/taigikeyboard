# Shipped roadmap items, 2026-09-19 .. 09-26 — design notes (frozen snapshot)

> **Type**: Historical
> **Keywords**: `update check`, `desktop menus`, `learned phrases`, `custom theme`, `photo background`, `telex`, `input-source menu`
> **Related**: ../roadmap.md, desktop-3.6.x-design-notes.md, ../architecture/dogfood-checklist.md

---

## Summary

- Verbatim bodies of the roadmap items that were still listed as active after they merged, collapsed out of `docs/roadmap.md` on 2026-09-26. Frozen: do not edit; the roadmap's Closed phases keep one line each, pending dogfood lives in `docs/architecture/dogfood-checklist.md`.
- Superseded since: learned-phrase and every other user-data store moved from the platforms into `engine/userdata` (user-data P0–P9d, #219–#237, `docs/architecture/user-data-engine-roadmap.md`), so the platform stores named below no longer exist; the Linux update check was removed 2026-09-25 (#193).

---

### Linux update check + identical desktop menus (USER-scoped 2026-09-24)

**Status**: Phase 0 `bfc79a53`, phase 1 #175, phase 2 site #20 MERGED; phase 3 #176, phase 4 #177, phase 5 #178 MERGED 2026-09-25 — round complete; dogfood S77 pending. Project memory `project_linux_update_check.md`.

**Linux half reversed 2026-09-25** (before any release shipped it): USER: "for desktop, only macos and windows need the update check; linux doesn't, it can be removed entirely", after a Linux packager's review (an input method is a system package; an update nag works against the distribution). Phases 3 and 5 are removed whole (`refactor/linux-drop-update-check`); phase 1's shared crate and phase 4's shared menu stay — Linux skips the Check for Updates row. Site `appcast/linux.json` (phase 2) is removed in the site repository. `linux-roadmap.md` L10 records the current design; the section below is the historical record.

USER 2026-09-24: "schedule the linux update check for the next round"; "I want the macos, windows and linux menus to have identical contents, including i18n". Round-start decisions (USER 2026-09-24): manual check **and** automatic notification; check logic lifted into a shared `desktop/` crate; one `appcast/linux.json` naming only the download page; widen TaigiKeyboard Settings to macOS + Windows this round. Reverses `docs/architecture/linux-roadmap.md` L10, whose premise ("packages are updated by the package manager") is false: `.deb` / `.rpm` / Arch packages ship only as GitHub release assets (no apt repository, COPR or AUR).

#### Today (grounded in code)

- Windows `windows/crates/taigi-windows-update`: `manifest.rs` (wire format, `DottedVersion`, `PUBLISHED_URL` :12), `checker.rs` (`Outcome`, 24 h `CHECK_INTERVAL_MS`, `is_due`, `stamp_next_check`, `record`, `claim_announcement`), `transport.rs` (ureq 3 native-tls, reads `PUBLISHED_URL` :81) have no Windows API; `installation.rs` / `verify.rs` (Authenticode, `unsafe`) / `toast.rs` (WinRT) are Windows-only. Callers: `taigi-windows-settings` `updates.rs:17`, `main.rs:101-150` (headless `--check-updates`: `is_due` on a plain load, then a separate stamp — two racing launches both fetch), `winui/window.rs:34`, `winui/pages/general.rs:13`, `winui/pane_planning.rs:23`.
- macOS `UpdateChecker.swift` (`appcast/macos.json`; 30 s after launch + 24 h timer, `AppDelegate.swift:121-128`; `NSAlert` manual, `UNUserNotification` automatic, notified-version recorded only after the notification is posted, :350-359).
- Linux: no check. `taigikeyboard-settings/src/pages/general.rs:98-121` = Current Version row + Download; `cli.rs:44` refuses `--check-now` / `--check-updates`; `lib.rs:26-86` is a single-instance `adw::Application` (`HANDLES_COMMAND_LINE`, id `tw.taigikeyboard.Settings`); `jobs.rs:8` = `gio::spawn_blocking` helper; `chrome.rs::menu_items` = 2 switch rows | TaigiKeyboard Settings | About the Keyboard; `taigi-linux-platform/src/launcher.rs:39` `spawn_detached` drops the `Child` unreaped.
- Shared: `taigi-desktop-core::settings::keys` `UPDATE_NEXT_CHECK_MS` / `UPDATE_LAST_NOTIFIED_VERSION` / `UPDATE_PENDING_MANIFEST` (:140-151, unwritten on Linux); `settings::launch` `CHECK_NOW_FLAG` / `CHECK_UPDATES_FLAG`. Every `desktop.update*` string is already scoped `linux`.
- Site: `scripts/announce-release.sh:171-182` writes `_data/linux_release.json` (+ rpm, arch) but no appcast; `wait_for_manifest` polls macOS + Windows only (:194-197; URLs `scripts/lib/release-site.sh:22-23`).
- Menus: Windows `lang_bar.rs:104-136` and Linux `chrome.rs:49-81` build the same row list twice; macOS `TaigiInputController.swift:440-479` a third time. macOS + Windows label the settings row `CommonSettings` (Settings), Linux `desktop.menuSettings` (TaigiKeyboard Settings, linux-only scope, #172).

#### Design (Codex ANALYSIS-ONLY pre-review + `/simplify` 2026-09-24 applied)

- **Schedule in core, network in its own crate.** `taigi-desktop-core::settings::update_schedule` = the pure schedule over the update keys: `CHECK_INTERVAL_MS`, `is_due`, `stamp_next_check` (phase 1), plus `claim_due_check` in phase 5 (due test + stamp in one `store.update` closure; the loser exits — closes the two-launch race on both desktops, and IBus + Fcitx5 activating together). New `desktop/crates/taigi-desktop-update` = `manifest` + `checker` + `transport` (phase 1); `ManualOutcome::alert_text` joins it in phase 3 and the headless sequence (claim → check → record → claim announcement → `Option<UpdateManifest>` to announce) in phase 5 — each lifted when its second consumer lands. Only the settings binaries link it, so ureq never reaches the IBus engine or the Fcitx5 addon `.so`. The manifest URL is a field of `HttpTransport`, one const per caller crate. No re-export shim: the ~6 Windows imports are rewritten.
- **Linux manual check**: `taigikeyboard-settings` `updates.rs` = check → deliver → `ManualOutcome` only (download-page offer; no install stage), fetched through `jobs::spawn`, answered in an `adw::AlertDialog` (pattern `pages/custom_dictionary.rs:738-771`); the version row gains Check for Updates and the pending state (Update Available {version} · Download). `--check-now` accepted; menu row Check for Updates → `launcher::check_for_updates()` (`--pane general --check-now`, Windows `settings_launcher.rs:54-59`).
- **Linux automatic check**: `chrome` asks on activation (IBus `Enable` / `FocusIn`, Fcitx5 `activate` already rebuild the menu through `chrome`, so no new FFI) against an `AtomicI64` next-check on `Runtime` (loaded once; bumped +24 h before the spawn, so a binary that fails to start is not respawned per focus), spawning `taigikeyboard-settings --check-updates` through `taigi-linux-platform` — the precedent `chrome.rs:21` `open_settings`. Spawned children are reaped on a thread. The headless path branches inside `connect_command_line`, never builds a window, posts a `gio::Notification` (default action opens General), holds the app until the notification is handed off. `tw.taigikeyboard.Settings.desktop` gains `DBusActivatable=true` plus a D-Bus service file so a click after the sender exited still reaches the app.
- **TLS**: ureq native-tls on Linux too (OS trust store, distro security updates; same provider as Windows). Cost stated: OpenSSL headers in the Linux build containers, `.rpm` / PKGBUILD build deps; runtime `libssl` resolved by `dpkg-shlibdeps` / rpm auto-requires.
- **Menus identical**: one ordered row list in `taigi-desktop-core` (`MenuCommand` = shortcut action | settings | check updates | about, with its `StringKey`); Windows and Linux map it to their ids, macOS keeps its Swift twin guarded by a unit test listing the same keys. `desktop.menuSettings` scope widened to macOS + Windows; `common.json` `CommonSettings` comment updated.

#### Phases

| # | PR | Type | Content | Status |
|---|---|---|---|---|
| 0 | admin | docs | this section + project memory | Merged `bfc79a53` |
| 1 | `refactor(desktop): lift update check out of the Windows crate` | Refactor — Windows behavior freeze (wire parsing, Failed → keep pending, stamp-before-fetch, pending cleared on UpToDate, once-per-version toast) | core `settings::update_schedule` + `taigi-desktop-update` crate; `taigi-windows-update` keeps install / verify / toast; imports rewritten | Merged #175 `53e06406` |
| 2 | site `taigikeyboard.github.io` | Feature (site) | `appcast/linux.json` rendered from `_data/linux_release.json` (the `.deb`'s data file): `version` + `downloadPageURL`, no package. Must be live before phase 3 merges | Merged site #20 `eba0930`, live 2026-09-25 |
| 3 | `feat(linux): check for updates from the menu and General pane` | Feature | Linux manual check (above); `announce-release.sh` `LINUX_MANIFEST_URL` + `wait_for_manifest`; docs: `linux-roadmap.md` L10, `linux-release.md` § Update check, `desktop-release.md`; OpenSSL build deps (CI, e2e, PKGBUILD) | Merged #176 `63667771` |
| 4 | `feat(desktop): one menu row list on three desktops` | Feature (i18n + 3 platforms) | `taigi_desktop_core::keys::MENU` (Windows `lang_bar::menu_rows` + Linux `chrome::menu_items` map it); macOS settings row → `.desktopMenuSettings`, menu tests assert the same literal; `desktop.menuSettings` scoped to all 3 desktops, `common.settings` back to mobile only; Windows popup ids = row position, both shells dispatch on `MenuCommand` | Merged #177 `5d24be87` |
| 5 | `feat(linux): automatic update check with a notification` | Feature | headless `--check-updates` + `claim_due_check` + shared `run_scheduled_check` (Windows headless adopts it), engine trigger (`update_trigger`, IBus `FocusIn`/`Enable`, Fcitx5 `taigi_runtime_activated`), reaping, `gio::Notification` + `app.show-updates`, D-Bus service file (no `DBusActivatable` — the app-grid launch stays `Exec`) | Merged #178 `9920894a` |

#### Best practices alignment

| Mainstream practice | Source | This plan (phase) |
|---|---|---|
| Static appcast polled on a schedule, notify once per version, manual check from the app menu | macOS `UpdateChecker.swift:192-395` (Sparkle model); Windows W9 `taigi-windows-update/src/checker.rs` | 1, 3, 5 |
| Pure logic in shared crates, OS handles in the shell | `.claude/rules/linux-guidelines.md`, `windows-guidelines.md`; `desktop/Cargo.toml` header (L2 / W1 split by dependency class) | 1 |
| Clickable notification from a process that exits: `GApplication` + `DBusActivatable` | Gio `Notification` / `Application.send_notification` docs | 5 |

**Deliberately not adopted**: in-app download + install on Linux (`.deb` / `.rpm` need root; three formats); per-format manifests (no in-app install to use a package URL — YAGNI); a systemd user timer (packaging change for what the always-running engine already sees); rustls (would diverge from the Windows provider for one binary); the menu list generated into Swift (five rows — a test is cheaper). **Departure from mainstream**: distro IBus engines (`ibus-rime`, `ibus-mozc`) do not check for updates because a repository updates them; this project ships no repository, so L10's precedent does not apply.

#### Dogfood

**S77** (`docs/architecture/dogfood-checklist.md`) on both Linux VMs (KDE + Fcitx5, GNOME + IBus) plus the macOS and Windows menus: menu Check for Updates → dialog; General pane pending row; an older installed build gets one notification after activation and none on the next focus; menus identical on three desktops.

---

### Learned phrases — a phrase composed segment by segment becomes a whole-buffer candidate (USER-scoped 2026-09-20)

**Status**: PR1 engine #109 MERGED `42d4dc5a`, PR2 iOS #110 MERGED `b446d852`, PR3 Android #111 MERGED `3b936e81`, PR4 macOS + Windows #112 MERGED `9b6d6afe` (2026-09-20); #113 removed the toggle. **Follow-up round 2026-09-21 — own store** (§ below): PR-A iOS #125 `bf786a5a`, PR-B Android #126 `9eddf5f3`, PR-C macOS + Windows #127 `96eb2c79`, PR-D i18n #128 — ALL MERGED 2026-09-21. Dogfood S62 pending on all four (rewritten for the own store). Project memory `project_learned_phrases.md`.

#### Own store, not the custom dictionary (USER-scoped 2026-09-21)

USER 2026-09-21: "are the auto-learned records stored in the custom dictionary? … I don't recommend it, because it would muddle the user's custom dictionary" → separate the two; "1. no UI needed 2. not carried" = no list UI for learned phrases, the `.taigi` backup does not carry them. Reverses design 3 above and the "deliberately not adopted: a separate `learned_phrases` table" line (its two costs — a second backup array and a second list UI — no longer exist). Nothing in #109–#113 has shipped (mobile 3.6.8 predates it, desktop 3.6.9 is an unpublished draft), so the schema v5 / DB v9 shape is a dev-only migration step. Codex ANALYSIS-ONLY pre-review 2026-09-21: D1–D7 CONFIRM with revisions applied below.

1. **Own DB file, own store, every platform**: `learned_phrases.db` — iOS `LearnedPhraseRepository` + `LearnedPhraseSchema` (`Lexicon/Database`), Android `LearnedPhraseService` (`SQLiteOpenHelper` v1), macOS `LearnedPhraseStore` on `UserDataDatabase` (added to `UserDataStores`), Windows `taigi-desktop-storage/src/learned_phrases.rs`. Precedent: `macos/…/Storage/UserDataStores.swift:8` — separate files so one kind of data can be deleted without the others. Tables: `learned_phrases(id INTEGER PRIMARY KEY, roman TEXT NOT NULL, hanzi TEXT NOT NULL, learn_count INTEGER NOT NULL DEFAULT 1, updated_at TEXT NOT NULL, UNIQUE(hanzi, roman))` + `learned_search_key(phrase_id INTEGER NOT NULL, family TEXT NOT NULL, form TEXT NOT NULL, key TEXT NOT NULL)` with `(family, form, key)` and `(phrase_id)` indexes. Keys come from the same engine derivation the custom dictionary uses (`deriveCustomSearchKeys`), written once on first insert (a row's roman never changes); the exact whole-buffer query, the 2 000 cap, fewest-then-oldest eviction (never the row just written), touch-on-use and the `learn_count` clamp keep their #110–#112 semantics against the new table. Android keeps the 3.22-safe UPDATE-then-INSERT pair.
2. **No cross-store rule.** No manual-row check at learn time, no takeover on a manual write: the engine already ranks a custom row above a learned one (custom takes the walker edge unconditionally, learned only competes) and the `(roman, hanji, span)` pair-key dedupe keeps the custom duplicate. Consequences accepted: a learned duplicate of a manual pair holds one of the 2 000 slots and one of the 5 per-fetch rows; deleting the manual row leaves the learned one offered.
3. **Custom dictionary back to the pre-§50 contract**: no `Origin` / `learn_count` / `isLearned` / badge / `origin =` filters / takeover / learned eviction; count, search, list and CSV cover every row again. Fresh `CREATE TABLE` has no provenance columns. Mobile schema bump — iOS 6, Android 10 — with one checked transaction: `DROP INDEX IF EXISTS idx_custom_learned_pair`, delete the side keys of `origin = 1` rows, delete the rows, stamp the version. Desktop stores carry no version ladder for this: the same cleanup runs inside `applySchema`, gated on the `origin` column being present (a read-only pragma on every open; the branch is one immediate transaction — the IME and the settings app share the file) and finishes with `DROP COLUMN` on both (macOS 14 / bundled SQLite ≥ 3.35), so it retires itself. On the phones the inert columns stay (Android's SQLite 3.22 has no `DROP COLUMN`; no released build ever wrote them).
4. **Backup `.taigi` back to v2** (custom rows = roman + hanzi). Reader keeps `version >= 1`; a dev v3 file's `origin = 1` rows are skipped in the parser (never turned into visible manual rows), `learnCount` ignored. Learned phrases are never exported or imported.
5. **Lifecycle = learning data.** No list UI, no badge (`i18n/dictionary.json` `learnedBadge` deleted in a follow-up PR-D once PR-A/B/C are all on `main`, per `i18n.md` — each platform PR branches from `main`, where the other platforms' consumers still exist). The wipe joins the existing learning-data clears — mobile `SettingsResetCoordinator.resetAllUserData`, desktop Clear Learning Records (`CustomDictionaryPage.deleteLearningRecords`, Windows `clear_learning_records`) — three stores, each attempted independently. The new store clears by `DELETE` inside the open connection (never by unlinking a file another process may hold — `sqlite.org/howtocorrupt.html`).
6. **Engine unchanged**; platform call sites keep their shape (`ComposingManager` builds `learned_entries`, the effect handler routes `PhraseLearned`, touch-on-use) against the new store. Windows: the learning methods leave `CustomDictionarySource` for a `LearnedPhraseSource` trait (manager injection, `Arc` forwarding, `NoStores`, mocks, runtime). iOS: the new repository warms up beside `custom_dictionary.db` in `KeyboardViewController+Setup` so sync reads answer after a relaunch; one more `SQLiteConnectionManager` (`cache_size` is an on-demand ceiling — a 2 000-row file stays small).

| Phase | Scope | Status |
|---|---|---|
| 0 | roadmap + §50 + S62 + project memory | this commit |
| PR-A | iOS: `LearnedPhraseRepository` / `LearnedPhraseSchema` / service wiring, custom dictionary v6 + model / badge / backup revert, reset wiring, tests | MERGED #125 `bf786a5a` |
| PR-B | Android: `LearnedPhraseService`, custom dictionary DB v10 revert, `SourceBadge` gone, backup revert, reset wiring, JVM SQL fixture | MERGED #126 `9eddf5f3` |
| PR-C | macOS + Windows: `LearnedPhraseStore` / `learned_phrases.rs`, `LearnedPhraseSource` trait, custom stores' column-gated cleanup, badge gone, Clear Learning Records covers three stores, tests | MERGED #127 `96eb2c79` |
| PR-D | delete `dictionary.learnedBadge` + `make i18n` on all four, after A/B/C merge | MERGED #128 |


USER report (2026-09-20): type `kikhilai`, pick 記 → 起 → 來 one segment at a time; however often this is repeated, the next `kikhilai` never offers 記起來 as one candidate. USER decision 2026-09-20 「ok, plan it」 after the survey below. Scope: all four platforms, engine-led.

#### Today (grounded in code)

| Piece | Where | What it does |
|---|---|---|
| Learning that exists | `ios/…/Actions/ActionHandler+Suggestions.swift:217`, `android/…/smartbar/CandidateClickHandler.kt:264` → `user_frequency.db`; `engine/nextword/src/decide.rs:303` bigrams | counts each picked (Hanji, canonical-TL) word; records prev→next pairs. Nothing joins consecutive picks into a new word. |
| Why single-char picks cannot fix it | `engine/composing/src/lattice/cost.rs:229` `WALKER_SINGLE_SYLLABLE_USER_DELTA_SCALE = 0.0` | single-syllable user weight is kept out of segmentation on purpose (Codex BLOCK 2026-05-16: 「的」 must not sweep a sentence). |
| What the engine already knows at final commit | `engine/composing/src/api.rs:36-40` `Phase::Continuous { nailed }`, `api.rs:115-135` `NailedSegment { display_text, canonical_text, raw_text, association_tl, raw_span, syllable_count }`; `transition.rs` `commit_continuous` final branch (`new_pending.is_empty()`) | the whole picked sequence with each segment's canonical TL — the input a phrase learner needs, in one place for all four platforms. |
| Effect channel | `engine/protos/proto/composing.proto:448-460` (`Effect` oneof, kinds 1–10); consumers iOS `KeyboardViewController+TextInput.swift:30-60`, Android `ComposingDelegate.kt:62`, macOS `ComposingManager.swift:554`, Windows `taigi-desktop-core/src/composing/manager.rs:459-474` (exhaustive match) | same shape the nextword `RecordCompoundAssociations` effect uses (`decide.rs:124-129`, iOS `NextWordController.swift:206`). |
| Custom-dictionary path (the closest existing sibling) | `FetchAtPos.custom_entries` (`composing.proto:210`); walker edge override `engine/composing/src/continuous.rs:629-682` (unconditional, `CUSTOM_EFFECTIVE_FREQ = 2000`); whole-buffer row `engine/lexicon/src/continuous/` `custom_entry_to_candidate` (`is_custom = true`, source rank 0); platform search `search(family, form, key, limit 20)` (iOS `ComposingManager.swift` `buildCustomEntries`, Android `ComposingManager.kt`, macOS `ComposingManager.swift:286`, Windows `manager.rs:244`) | verified with production artifacts: a custom row `kì-khí-lâi/記起來` puts 記起來 at the first hanji slot for `kikhilai` with 台日 off (the dictionary row is 台日-only, `SharedSettings.swift:61` default off). |
| Custom-dictionary storage | iOS `CustomDictionarySchema.swift:58` v4 (`id, roman, hanzi, notone, abbrev, roman_num, created_at, updated_at` + `custom_search_key`), Android `CustomDictionaryService.kt` `DATABASE_VERSION = 8`, macOS `CustomDictionaryStore.swift:54` v4, Windows `custom_dictionary.rs:19` v4; backup `.taigi` v2 (`BackupService.swift:72-90`: custom / frequency / association arrays, custom rows = roman + hanzi only) | one table per platform, same derived-key search on all four. |

#### Design (Codex ANALYSIS-ONLY pre-review 2026-09-20 applied: F1/F6/F7/F8 AGREE, F2 + F5 BLOCK resolved as below, F3/F4 PREFER adopted)

1. **Engine emits, platforms persist.** In `commit_continuous`'s final-commit branch, when the composition has ≥ 2 nailed segments, every segment is a hanji-bearing candidate pick, and the summed `syllable_count` ≤ 6 (ChiaKey's 6-character cap), the engine emits `Effect.PhraseLearned { hanji, canonical_tl }` (oneof kind 11) next to `NextWordWordSelected`. `hanji` = the segments' hanji concatenated; `canonical_tl` = the segments' canonical TL joined with `-` (a segment that already starts with the khinsiann `--` keeps it — no `---`). An Enter / raw commit of an unpicked tail does not learn (the tail was never a pick). Nothing is learned from abort / backspace-unnailed segments because the effect fires only on the final commit.
2. **Hanji-bearing is explicit, not inferred (Codex F2 BLOCK).** `CommitContinuous` gains `optional string hanji = 6` (the chosen `CandidateMessage.hanji`, wire-absent for hanji-less / literal / OOV picks), stored on `NailedSegment.hanji`. A composition with any hanji-less segment does not learn. This is the same sidechannel discipline as `canonical_text` / `association_tl` (Bug 1 Option A / R2) — the engine never parses the display string to guess.
3. **Storage = the custom-dictionary table with provenance (Codex F3).** Two columns on every platform: `origin INTEGER NOT NULL DEFAULT 0` (0 manual, 1 learned) and `learn_count INTEGER NOT NULL DEFAULT 0`; a partial unique index on `(hanzi, roman) WHERE origin = 1` so learning is one atomic upsert (`ON CONFLICT … DO UPDATE learn_count = learn_count + 1, updated_at = now`; Android uses the shared 3.22-safe upsert helper from #96). A manual row is never downgraded to learned; learning a pair that exists as a manual row is a no-op. Schema bumps: iOS 5, Android 9, macOS 5, Windows 5. Backup `.taigi` v3 writes `origin` + `learnCount` per custom row; a v2 file imports as manual (missing field → 0). Learned rows list in the existing custom-dictionary UI with a auto-learned badge and the existing delete; export/import go through the existing paths.
4. **One pick learns; ranking is bounded, never an override (Codex F4 + F5 BLOCK).** Learned rows ride a NEW `FetchAtPos.learned_entries = 7` (`LearnedEntry { hanji, canonical_tl }` — `learn_count` stays platform-side until an engine reader exists), not `custom_entries`. Engine treatment mirrors #69's decoupling: (a) at the walker edge whose toneless key equals the learned key, the learned row joins the dict rows in the same `SortKey` pick — `score = calculate_continuous_score(0, 1)` (dict rows out-score it unless user weight says otherwise), source rank **below** dict and custom, `is_custom = false`; (b) `EdgeBest::span_frequency` takes `max(dict, LEARNED_EFFECTIVE_FREQ)` so the whole-buffer edge still wins the segmentation when the dictionary has no word under that key (the 台日-off case); (c) a span-local whole-buffer row like `custom_entry_to_candidate` with the same rank rule. Consequence: a learned 記起來 with 台日 off appears at the first hanji slot after ONE composition; with 台日 on the dictionary row wins the in-edge choice and the pair-key dedupe collapses the duplicate; a mistaken learned phrase never displaces a dictionary word unless the user keeps picking it (user_frequency), and the manual custom dictionary's precedence is untouched.
5. **Platform query for learned rows is exact, not prefix.** `learnedEntries(for rawInput)` = `origin = 1 AND family/form/key = whole-buffer key` (limit 5), separate from the prefix `search(limit 20)` so a learned whole-buffer match can never be truncated out (Codex risk: search truncation).
6. **Touch on use (Codex F6).** When a committed candidate's (hanji, canonical-TL) matches a learned row, the platform bumps `learn_count` / `updated_at` in the same place it records `user_frequency` (one indexed UPDATE, no-op otherwise). Learned rows are capped at 2 000 per platform; past the cap, evict lowest `learn_count`, then oldest `updated_at`, inside the same transaction as the insert. Manual rows are never evicted and keep their own capacity accounting.
7. **No setting — always on** (USER 2026-09-20: "this feature needs no toggle; it is always on by default"; the Auto-Learn New Words toggle shipped in PR2–PR4 was removed the same day). Learning and injection do not depend on Enable Custom Dictionary (`customDictEnabled`), which keeps gating manual rows only. Learned rows are deletable from the custom-dictionary list.
8. **Security / privacy**: learned rows are user text — app-private store, `.taigi` documented sensitive (already), never logged (`security-rules.md`).

Known parity limit carried over from the custom path, not new: under TPS input `custom_toneless_key` accepts only Bopomofo bodies (`shadow.rs` S6 note), so a TL-keyed learned row is a span-local row but not a TPS walker edge — same as today's custom dictionary. POJ input already folds a TL canonical key (verified: custom `kì-khí-lâi` matched `kikhilai` in POJ mode).

#### Phases

| Phase | Scope | Status |
|---|---|---|
| 0 | roadmap section + project memory | `2e1a5e75` 2026-09-20 |
| PR1 | engine: `CommitContinuous.hanji` + `NailedSegment.hanji`; `Effect.PhraseLearned`; `FetchAtPos.learned_entries` + `LearnedEntry`; walker/span-local treatment (design 4); `candidate_dump` `DUMP_CUSTOM` / `DUMP_LEARNED`; `continuous_learned_phrase.rs` (18); §50; four bridge arms | MERGED #109 `42d4dc5a` |
| PR2 | iOS: schema v5 (transactional, checked), learn upsert `RETURNING id` + cap/evict, exact learned query, manual write takes over learned, setting + i18n, auto-learned badge (`TagBadge`), backup v3, 13 tests; S62 | MERGED #110 `b446d852` |
| PR3 | Android: DB v9, UPDATE-then-INSERT learn pair (3.22), same rules, `SourceBadge`, backup v3, JVM SQL fixture + 4 tests | MERGED #111 `3b936e81` |
| PR4 | macOS + Windows: idempotent schema columns + partial index, store `learnPhrase` / `learnedRows(matching:)` / touch, manual takeover, quota accounting, pane toggle + badge, `learned_entries` + `hanji` on both bridges, shared badge (`TagBadge` / `cards::badge`), schema ALTER in one immediate transaction + macOS busy timeout, +6 macOS / +6 Windows tests | MERGED #112 `9b6d6afe` |

#### Best practices alignment

Per-phase rules: PR1 `round-workflow.md` § Codex sandwich + `code-review-rules.md` §5 (every effect consumer + proto decoder is a caller); PR2–PR4 `cross-platform-alignment.md` §1 (behaviour stated first, above), `i18n.md` (new key `dictionary.learnedBadge` — hanji first, tailo/poj copy hanji until USER romanizes, per #107 precedent), `security-rules.md` § Data Storage, `doc-lookup.md` for the Android DataStore key.

| Mainstream pattern | Source | This plan |
|---|---|---|
| Learn the whole committed composition as one user-dict entry, fired when the last segment is confirmed | librime `references/librime/src/rime/gear/script_translator.cc:257-282` `ConcatenatePhrases` + `memory.cc:111-124`; `kConfirmed` only at buffer end `engine.cc:263` | design 1 — the engine's final-commit branch is the same trigger; one composition learns |
| Concatenated conversion inserted as a learned entry only when > 1 segment | mozc `references/mozc/src/prediction/user_history_predictor.cc:2231-2238` | ≥ 2 nailed segments |
| Manual vocabulary and auto-learned vocabulary kept distinguishable; learned tier below manual | MOE Taigi `docs/references/moe-taigi-reference.md:175-199` (`UserVoc` / `LearnedVoc`, `MAX_LEARNED_*`) | design 3 `origin` column + design 4 source rank below custom |
| Bounded learning store: evict fewest selections, then least recent; cap phrase length at 6; reject a pair already in the system/user table | ChiaKey `docs/references/chiakey-reference.md` §3 + table row 2 (`addUserUnigram`) | design 6 cap/evict; 6-syllable cap; manual-row no-op |
| Segmentation priced on the span, word choice priced on the word | this repo #69 `engine/composing/tests/continuous_slot0_user_selection.rs` (`EdgeBest::span_frequency`) | design 4(b) |
| Engine emits a learning effect, platform owns the store | this repo `engine/nextword/src/decide.rs:124-129` → iOS `NextWordController.swift:206` | design 1 |

**Deliberately not adopted**: rime's `core_word_length` prefix/suffix combinations (learns every sub-phrase; noise); a maturity threshold before a learned row surfaces (MOE `RIPE_*_APPROVALS`) — rejected by Codex F4 because bounded ranking already contains a mistaken phrase and the USER expectation is "picked once, offered next time"; reusing `custom_entries` (unconditional walker override — Codex F5 BLOCK); a separate `learned_phrases` table (second search-key derivation, second backup array, second list UI); ChiaKey's desktop mark-mode add-word gesture (no mobile counterpart; manual add already exists in settings); ChiaKey's "pick equals lexicon default → clear override" rule (F8: unnecessary once learned rows compete instead of override; revisit only on a report). YAGNI: a decay column on learned rows (`user_frequency` already decays the ranking once the phrase is picked), context-keyed overrides, a lexicon lookup before learning (F7: the pair-key dedupe already collapses a duplicate of a dictionary word).

#### Dogfood

S62 (rewritten 2026-09-21 for the own store): 台日 off, type `kikhilai`, pick 記 / 起 / 來 (or 記 then 起來), commit; retype `kikhilai` → 記起來 is the first hanji candidate; Custom Dictionary does NOT list it (no row, no badge, count unchanged, CSV / `.taigi` export unchanged); Clear Learning Records (desktop) / Reset to Defaults user-data reset (mobile) → the next `kikhilai` is back to 機起來; a manual custom row with the same pair is untouched. Both mobile platforms, then desktop.

---

### Mobile custom theme — one background surface, gradient direction, photo background (USER-scoped 2026-09-19)

**Status**: all phases MERGED 2026-09-20 — A iOS #90 `ee5e41ab`, B Android #91 `31447804`, C iOS photo #92 `4bfddae5`, D Android photo #93. Dogfood pending: S54 (background surface / gradient / seed / editor order) + S55 (photo background), both platforms. Project memory: `project_mobile_custom_theme_background.md`.

USER request (2026-09-19, five points, verbatim intent): (1) the candidate-bar background and the keyboard background merge into ONE colour; (2) background gradient with a Figma-like selectable direction and selectable colours; (3) upload a photo as the keyboard background — resize / aspect ratio handled, saturation must not be distracting; (4) a custom theme has no light / dark split — same colours in both modes; (5) re-order the editor controls, clean, simple, elegant. Editor sections = 3 (Background / Keys / Candidates), photo tone-down = fixed saturation cap + one Fade slider, gradient direction = 8 arrow presets (USER picks 2026-09-19).

#### Today (grounded in code)

| Piece | iOS | Android |
|---|---|---|
| Colour model | `Settings/KeyboardColorSettings.swift:67` — six optional roles (`nil` = inherit adaptive) + `backgroundGradient: ThemeGradient?` (`:60`, vertical stops, built-in themes only) | `ime/core/KeyboardColorSettings.kt:37` — same shape, ARGB `Int?`, `ThemeGradient` `:14` |
| Candidate bar background | separate `candidateBackgroundColor`; built-in themes never set it (`BuiltInThemes.swift:155`); render `TaigiKeyboardView.swift:445-459` | `SmartbarView.kt:141-156`, `SmartbarManager.kt:709-730`, `KeyboardPreviewPanel.kt:251` |
| Root background | `TaigiKeyboardView.swift:159-172` — liquid glass → vertical `LinearGradient` → solid; overlays repaint the gradient (`Overlays/KeyboardOverlayBackdrop.swift`, `ExpandedCandidateOverlay.swift:212`) | `KeyboardLayout.kt:79` + `KeyboardThemeSurfaceController.kt:24` (`GradientDrawable(TOP_BOTTOM)` on the shared parent) + `KeyboardOverlayAppearance.kt` |
| Light / dark | user themes store one static RGBA per role, but a `nil` role flips with the scheme | same |
| Editor order | Keyboard (background colour · height) → Keys (3 colours · font size · corner radius · border · shadow) → Candidate Bar (2 colours · font size) → Reset to Defaults (`App/Tabs/Theme/ThemeEditorView.swift:28-75`) | `ui/tabs/theme/ThemeEditorScreen.kt:165-310`; `ThemeEditorActivity.kt:79` strips any gradient on save |
| Photo picker | none | none |

#### Design (USER-approved 2026-09-19)

1. **One background surface.** `candidateBackgroundColor` is deleted everywhere; the candidate bar is the same surface as the keyboard: solid → same colour, gradient / photo → transparent over the root paint (the path built-in gradient themes already use).
2. **`background: ThemeBackground?`** replaces `backgroundColor` + `backgroundGradient` (one field, mutually exclusive cases): `solid(color)` · `gradient(stops, angle)` · `image(file, dim)` (image case lands in PR C/D). `nil` = adaptive (the Classic Default head only). JSON `{"type":"solid"|"gradient"|"image", …}`, identical on both platforms. Decoders map the old keys: `backgroundColor` → solid, `backgroundGradient` → gradient angle 180, `candidateBackgroundColor` ignored. Built-in gradient themes keep angle 180.
3. **Gradient direction** = CSS/Figma angle in degrees (180 = top→bottom). Editor: 8 arrow presets (45° steps), two colour rows (Start / End). Unit points derive from the angle with Chebyshev normalisation so diagonals hit the corners; overlay panels transform the same points into panel coordinates (generalises the vertical slice shift in `KeyboardOverlayBackdrop`).
4. **Photo background.** Host app `PhotosPicker` (iOS) / `PickVisualMedia` (Android) — neither needs a permission. On pick: decode, downscale to long edge ≤ 1280 px, JPEG q0.85, write `theme_images/<uuid>.jpg` under the App Group container (iOS, backup-excluded) / `filesDir` (Android). Render: scaled-to-fill, centre-cropped, fixed saturation 0.7, then a Fade overlay (white when key text is dark, black otherwise) at the theme's `dim` (slider 0–0.8, default 0.35). Candidate bar transparent. Extension caches the decoded image by file name; delete / replace removes the file; the store sweeps orphans. Decoded size ≈ 5 MB, far under the 64 MB extension cap. Liquid Glass stays off for any custom background (today's rule).
5. **Scheme-invariant user themes.** A new custom theme seeds every role with a concrete light hex (CROSS-PLATFORM INVARIANT: background `0xD4D5DD`, key text `0x000000`, key fill `0xFFFFFF` (letter + special keys share it, USER 2026-09-26), candidate text `0x000000`), so a user theme never holds `nil`; existing saved themes with `nil` roles resolve through the same seed at load (no migration write). Per-row reset returns the seed value; Reset to Defaults returns the whole seed. Built-in themes untouched.
6. **Editor order** (both platforms): **Background** [Type Solid | Gradient | Photo → Color / Start Color · End Color · Direction / Thumbnail · Change · Remove · Fade] · **Keys** [Key Fill · Text · Corner Radius · Border · Shadow · Height · Font Size] · **Candidates** [Text · Font Size] · Reset to Defaults · pinned live preview.

#### Phases

| Phase | Scope | Status |
|---|---|---|
| 0 | roadmap section + project memory | `f68a360e` |
| A | iOS: `ThemeBackground` model + decode compat, candidate-bg removal, gradient angle + 8-direction row, seeded user themes, editor re-order, i18n keys, tests, `docs/ui/theme.md` | MERGED #90 `ee5e41ab` |
| B | Android port of A (model, render, editor, tests) | MERGED #91 `31447804` |
| C | iOS photo background (`PhotosPicker`, image store, render, editor rows) | MERGED #92 `4bfddae5` |
| D | Android photo background (`PickVisualMedia`, image store, render, editor rows) | MERGED #93 |

#### Best practices alignment

| Mainstream pattern | Source | This plan |
|---|---|---|
| Background = colour ∘ gradient ∘ image ∘ overlay layers, image `contentMode` | KeyboardKit 10 `Keyboard.Background` (`references/KeyboardKit-Documentation/data/documentation/keyboardkit/keyboard/background.json`: `backgroundColor`, `backgroundGradient`, `imageData`, `imageContentMode`, `overlayColor`); Hamster `KeyboardBackgroundStyle.swift:31-52` | same layer order in `ThemeBackground`; KK's gradient is vertical-only, so the angle gradient is drawn with SwiftUI `LinearGradient` on the root as today |
| Image background drawn with `matchParentSize` + `contentScale` inside the themed box; `allowHardware(false)` to avoid decode crashes | florisboard `lib/snygg/src/main/kotlin/org/florisboard/lib/snygg/ui/SnyggBox.kt:94-106` | Android render = Compose `Image` crop over the keyboard content box |
| Bitmap cache keyed by path, evicted on theme change | trime `data/theme/ColorManager.kt:96-167` | per-process decoded-image cache keyed by file name, dropped on theme revision |
| 8-orientation gradient drawable on the View seam | Android `GradientDrawable.Orientation` (already used in `KeyboardThemeSurfaceController.kt`) | the 8 presets map 1:1 to `Orientation` on the View seam; Compose uses unit points |

**Deliberately not adopted**: a free 0–360° angle *dial* (heavier UI + a11y for a keyboard-sized surface); a saturation slider (fixed cap + one Fade knob, USER 2026-09-19); per-scheme light / dark colour pairs for user themes (USER point 4); keeping `candidateBackgroundColor` as a hidden field (USER point 1 — one surface). YAGNI: multi-stop gradients (two stops), radial gradients, image position / zoom controls.

#### Dogfood

S54 (background surface / gradient direction / scheme-invariant colours / editor order) and S55 (photo background) in `docs/architecture/dogfood-checklist.md`, both platforms.

#### Follow-up E — drag the preview to set the gradient direction (USER 2026-09-19: "drag with a finger to choose the gradient centre instead of using buttons"; option 1 of 3 picked, iOS first)

Direct manipulation replaces the 8 arrow buttons: while the background kind is Gradient, dragging on the pinned live preview sets `ThemeGradient.angle` to the direction centre → finger (any whole degree; ±6° snap onto the 45° presets with a selection click). Model / JSON / rendering unchanged (`angle` was already a free `Double`; `unitPoints` handles any angle). No Direction row (USER 2026-09-19: "the 'Direction' row isn't needed, because once users tap and see the pointer below, they know how to adjust it"); VoiceOver / TalkBack keep the presets via an adjustable action on the pointer itself. iOS #94 MERGED `4d064698` 2026-09-19; Android port (same `ThemeGradient.angle: Float`, `PaintDrawable` shader already takes any angle; `pointerInput` + `drawBehind`, `GradientDirectionControl.kt`) = the next PR. Dogfood: S54 gains the drag steps on both platforms.

---

### Desktop Telex keys for tone 1 and tone 4 (USER-decided 2026-09-19)

**Status**: MERGED 2026-09-19 — #98 `17850f17` (engine + both guide tables + docs). Dogfood
pending: S57 (plus S32 / S34, whose examples were updated).

A user reported that Telex has no key for tone 1 or tone 4 (2026-09-11). A second user proposed
pairing the tones by coda: "tone 1 and tone 4 can share one key. Tone 8 can share a key with tone 2"
(2026-09-12). USER 2026-09-19 evaluated user-customizable Telex keys, rejected them as complexity
(the free pool is eight letters, so a custom table can only permute them), and adopted the fixed
table below: "1. per your suggestion, 2. no conflicts with other shortcuts, ok, go".

**Why a key at all.** Tones 1 / 4 are unmarked, so the raw buffer `tai` already reads as tone 1.
The key is an explicit pin: it narrows candidates (`tai` matches every tone, `tai1` only tone 1)
and ends a syllable in continuous input, exactly what the digits `1` / `4` do in the Standard
scheme. The buffer stays numeric-tone (`tai1`, `sit4`), so nothing downstream changes.

**Letter arithmetic.** The free letters are `d f q v w x y z` (`c` is POJ `ch`, `r` the `ir` /
`er` finals). Seven open tones (1 2 3 5 6 7 9) each need their own key — two open tones on one key
cannot be told apart — plus `z` and `f` makes nine, one more than the pool. The coda pairing is
what frees the two: a syllable ending in a stop `p t k h` can only carry tone 4 or 8, any other
only an open tone, so a key may carry one open + one checked tone. Tone 1 (with 4, ~30 % of the
dictionary's syllables) takes the freed slot; tone 6 (2 060 syllables, Lukang / Quanzhou
dialects) stays unoffered as decided 2026-09-08 and is typed under Standard (`tai6`).

| Key | Open tail (any other coda) | Checked tail (`p t k h`) |
|---|---|---|
| `x` | tone 1 (`tai` → `tai1`) | tone 4 (`sit` → `sit4`) |
| `v` | tone 2 (`te` → `te2`) | tone 8 (`tit` → `tit8`) |
| `y` `d` `w` `q` | tone 3 5 7 9, no coda test | same |
| `z` / `f` | unchanged | unchanged |

Engine gate (`engine/composing/src/telex.rs`): strip one trailing tone digit, read the last letter
case-insensitively, `p t k h` → checked digit, else open; then the shipped rules (append / replace
a different digit / same digit no-op / empty or `-`-ended tail no-op). `sit8` + `x` → `sit4`;
`sit4` + `v` → `sit8`. An initial-only tail (`kh`, `tsh`) reads as checked — no tone makes it a
syllable, so the digit does not matter. `TELEX_KEYS` is unchanged, so neither desktop classifier,
the shortcut recorder's bare-letter refusal nor the slot keys move (USER: no shortcut conflicts).

**Behaviour change**: `x` on a checked syllable wrote tone 8 in desktop 3.6.8 (`sitx` → `si̍t`),
now tone 4; tone 8 moves to `v`. Chosen over the zero-regression pairing (`x` = 1/8, `v` = 2/4)
because "one key = the two unmarked tones" is what the guide can teach in one row; Telex is opt-in
and eight days old. No alias period.

**Rejected**: `c` = 1/4 (2026-09-12 draft) — under POJ, `kong` + `chhia` would pin `kong1` and
break `hhia` on every untoned syllable before a `ch` word; `;` / `'` (not letters); dropping the
`ir` / `er` finals (Core Principle #3); a user-customizable key table (2026-09-19).

---

### Desktop input-source menu — global shortcut rows (USER-scoped 2026-09-19)

**Status**: MERGED 2026-09-20 — #100 `f763d8bf` (macOS + Windows in one PR). Dogfood pending: S59 (both platforms).
The menu-bar / tray menu lists the two global shortcuts a click can stand in for (Switch Tâi-lô or Pe̍h-ōe-jī · Switch Candidate Display), each printing the chord the Shortcuts pane holds for it, then Settings (TaigiKeyboard Settings since 2026-09-25, one row list on three desktops — § Linux update check phase 4), then Check for Updates. Labels reuse the pane's i18n keys (USER 2026-09-19: no new strings). Excluded on purpose: Switch Hanji/Romanization Mode (bare-backtick default — a macOS menu key equivalent is dispatched by the text-input menu agent system-wide, so a bare key there is a key the user can no longer type; the rule generalises: a row re-recorded onto a bare key prints no chord) Open Symbol Menu (needs the caret a click has no hold of) and Open Telex Guide (USER 2026-09-20: "very few people use it"). Windows resolves the focused context after the popup closes (`ITfThreadMgr::GetFocus` → `GetTop`) so the click re-presents that context's open list.
