# Custom Dictionary

> **Type**: Feature
> **Keywords**: `CustomDictionary`, `Import`, `Export`, `CSV`
> **Related**: continuous-candidate-display.md, binary-format.md

---

## Summary

- User-defined dictionary entries (romanization + hanzi pairs)
- CRUD operations with SQLite storage, owned by the engine (`engine/userdata`) on every platform
- CSV import/export
- Integrated into autocomplete with highest priority (before system dictionary)

---

## Data Model

| Field | Type | Description |
|-------|------|-------------|
| `id` | String (UUID) | Unique identifier |
| `roman` | String | Romanization in the user's native form (TL or POJ display, whichever the user typed when saving). The engine treats it raw on the lattice axis; the freq-key commit value is canonicalized to TL (`phonetics::api::canonical_tl_form`). |
| `hanzi` | String | The Hanji (Han-Lo spellings such as `白phau-phau` included); empty for a word written only in romanization. Every write — add, edit, CSV import, `.taigi` restore — stores a Hanji column that only repeats the roman in romanization — a Latin letter, no ideograph or kana, equal to the stored roman (`lán,lán`, `lan2,lán`) — empty, the rule the dictionary pipeline applies to the shipped rows (`phonetics::api::is_roman_only_spelling`, the port of `dictionary/common/cleanup.py` `is_roman_only_hanzi`; shared cases in `dictionary/tests/fixtures/roman_only_spelling_cases.tsv`), so an imported roman-only word and the shipped one are one candidate. A Latin spelling that differs from the roman — an abbreviation (`khe-thi-bui,KTV`), capitals (`tâi-lâm,Tâi-lâm`), a POJ display for a TL roman — is the form the user chose and is kept. Rows an older build stored are repaired at launch (`normalize_stored_rows_if_needed`). |
| search keys | `custom_search_key` rows | Derived per entry by the engine: TL / POJ / TPS families × tone-number / toneless / abbreviation forms (`engine/phonetics/src/custom_search.rs`) |
| `created_at` | Timestamp | Creation time (UTC) — DB column only, not on the wire (`CustomDictionaryEntry` tag 4 reserved) |
| `updated_at` | Timestamp | Last update time — DB column only, not on the wire (tag 5 reserved) |

Example: `roman="gâu-tsá"` → toneless key `gautsa`, abbreviation key `gt`

---

## Storage

**Database**: `custom_dictionary.db` on every platform (iOS: App Group shared container; Android: app-private `databases/`; macOS: `~/Library/Application Support/<bundle id>/`; Windows + Linux: `%APPDATA%\TaigiKeyboard` / XDG), opened and owned by the engine — `engine/userdata/src/custom_dictionary.rs`, per `docs/architecture/user-data-engine-roadmap.md`. Platforms pass only the directory (`OpenUserData`).

```sql
CREATE TABLE custom_dictionary (
    id              TEXT PRIMARY KEY,
    roman           TEXT NOT NULL,
    hanzi           TEXT NOT NULL,
    created_at      TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at      TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_custom_roman ON custom_dictionary(roman);

CREATE TABLE custom_search_key (
    entry_id TEXT NOT NULL,
    family   TEXT NOT NULL,
    form     TEXT NOT NULL,
    key      TEXT NOT NULL
);
CREATE INDEX idx_csk_lookup ON custom_search_key(family, form, key);
CREATE INDEX idx_csk_entry ON custom_search_key(entry_id);
```

The `custom_search_key` side table backs cross-input-mode (three-index TL / POJ / TPS) lookup. A file from an older iOS / Android release may still carry `notone` / `abbrev` / `roman_num` columns; the engine neither reads nor writes them.

---

## CRUD Operations

Engine ops (`UserDataRequest`, `engine/protos/proto/user_data.proto`), reached through each platform's `UserDataClient` (iOS `Lexicon/Services/UserDataClient.swift`, Android `ime/dictionary/UserDataClient.kt`, macOS `Lexicon/UserDataClient.swift`):

| Operation | Engine op | iOS / Android `UserDataClient` |
|-----------|-----------|-----|
| Create/Update | `SaveCustomEntry` | `save` |
| Read all / page | `ListCustomEntries` (page + `total` + `matching_total`) | `listAll` |
| Search | — (no user-data op: the lexicon `DictionarySearch` reads it inside the engine, `UserDataHandle::custom_entries_matching`) | — (`LexiconClient.dictionarySearch`) |
| Delete one | `DeleteCustomEntry` | `delete` |
| Delete all | `ResetUserData.custom_dictionary` | `deleteAll` |
| CSV import / export | `ImportCustomCsv` / `ExportCustomCsv` | `importCSV` / `exportCSV` (Android `importCsv` / `exportCsv`) |

Refusals (full, empty roman, unsearchable, file too large, not UTF-8, no usable rows) come back as `CustomDictionaryRefusal`.

---

## Import/Export

### Export (CSV)
```
gâu-tsá,𠢕早
tsia̍h-pá--buē,食飽未
```
- Data-only format (no header row)
- Fields containing commas/quotes/newlines are quoted
- Internal quotes escaped as `""`

### Import (CSV File)
1. Read UTF-8
2. Parse CSV (data-only, 2 columns: roman, hanzi; the hanzi column may be empty — `lán,` is a romanization-only word)
3. Validate format
4. Store form: roman in NFC with tone digits as marks; a hanzi column that only repeats the roman in romanization (`lán,lán`) emptied; another Latin spelling (`KTV`) kept
5. Deduplicate by `roman|hanzi` key
6. Return `ImportResult { imported: Int, skipped: Int }`

CSV is **custom-dictionary-only**. For whole-user-data backup see `.taigi` below.

---

## `.taigi` Backup (whole user data)

A `.taigi` file is a single plain-text **JSON** document carrying all **four** user-data SQLite DBs in one container — `custom_dictionary.db`, `user_frequency.db`, `user_association.db`, `learned_phrases.db` (§50, since v4) — with the times ranking reads, so a restore on any platform types as the writer did (#1032). It is the **only** cross-device user-data path on the phones (their DBs are excluded from OS auto-backup — behavioral-invariants.md §29).

| File | Responsibility |
|------|----------------|
| engine `engine/userdata/src/backup.rs` (ops `ExportBackup` / `ImportBackup`) | the one encoder / decoder |
| iOS `App/Tabs/Dictionary/Utilities/BackupDocument.swift` + `UserDataClient.exportBackup` / `importBackup` | file document; `UTType.taigiBackup` = `tw.taigikeyboard.backup` |
| Android `ui/tabs/dictionary/DataManagementViewModel.kt` + `UserDataClient.exportBackup` / `importBackup` | file picker / share, bytes to the engine |
| Windows / Linux `taigi-desktop-core` `settings/learning_records.rs` (`export_backup_job` / `import_backup_job`) + each settings window's Learning Records pane | save / open dialog (open accepts `.taigi` and `.json`), bytes to the engine; a restore reloads Custom Dictionary too |
| macOS `Lexicon/LearningRecordsPage.swift` (`LearningRecordsPageModel.exportBackup` / `importBackup`, the Swift twin of the row above) + `EngineUserDataClient.exportBackup` / `importBackup` | save / open panel on the exported `tw.taigikeyboard.backup` type (open accepts `.taigi` and `.json`), bytes to the engine; Custom Dictionary loads afresh when next shown |

### Format

- Top-level: `version`, `exportedAt` (ISO-8601 UTC), `platform` (`ios`/`android`/`macos`/`windows`/`linux`), `appVersion`, plus `customDictionary[]` (`roman`, `hanzi`, `createdAt`, `updatedAt`), `userFrequency[]` (`word`, `tl`, `count`, `lastUsed`, plus `abbrevCount` / `abbrevLastUsed` — the picks of `count` made through a whole-buffer abbreviation and the last of them — written only for a row that has any), `userAssociation[]` (`prevWord`, `prevTl`, `nextWord`, `nextTl`, `count`, `lastUsed`), `learnedPhrases[]` (`hanzi`, `roman`, `learnCount`, `updatedAt`). Times are the stores' `yyyy-MM-dd HH:mm:ss` UTC text (written since v4; empty before).
- **Version**: export writes `4` (`BACKUP_VERSION`, `engine/userdata/src/backup.rs`); import accepts `version >= 1`. All versions share one shape; reading is lenient (missing fields default, unknown fields ignored).
- `userFrequency[].tl` (canonical TL, the `(Hanji, canonical-TL)` pair identity) is optional on decode — a backup without it imports into the legacy `tl=""` bucket.
- Custom-dict rows carry `roman`+`hanzi` and their times; the id and the search keys are regenerated on import. Same-key custom rows reach the keystroke path by `roman, created_at, hanzi`, so homophones keep their order across a restore.

### Import semantics — **merge, never replace**

- Custom dictionary: adds non-duplicates only (dedup by `roman\thanzi`).
- Frequency + association + learned phrases: **higher-count-wins, later-use-wins** merge; capacity is enforced after the import. Frequency rows keep reading and abbreviation picks apart (`UserFrequencyStore::import_merge`). A file time is stored through SQLite `datetime`, clamped to now; empty (v1–v3) or unreadable counts as used now. Restored learned phrases get their search keys.
- Bigram readings of a version 1 / 2 file (the phones could write POJ there) are folded POJ → TL; version 3 and later readings restore unchanged, since a fold would misread a TL `eng` / `ek` final as POJ.

> **Row cap (30000, `CustomDictionaryStore::MAX_ENTRIES`)**: a `.taigi` restore fills the space left and stops (`CustomDictionaryStore::import_until_full`); a CSV whose own row count exceeds the cap is refused before anything is written (`batch_import`).

---

## Autocomplete Integration

A custom entry is offered by the same rules as a system dictionary word (behavioral-invariants.md §43): only once the typed input reaches into its final syllable (陳建中 `Tân Kiàn-tiong` from `tankiant`, not `t`), or on its whole abbreviation (`tkt`). Once offered, a never-selected entry follows the words the user has selected and leads every other candidate covering the whole input (`preference_class`, `continuous-input-ranking.md`), except that an entry reached through its abbreviation follows every reading of that input (`thng` → 湯 / 糖, then a custom 退伍 `thui-ńg`; `abbreviation` field). Once selected it ranks by its selection weight like any word (USER 2026-10-16: a bulk import filing 屢 `lí` put it ahead of 你, which the user picks, on `li2`). It wins a collision with the same system word (source rank 0). The walker's edge pick weighs the entries under an edge's key together with the dictionary's homophones by the same order.

**Flow** (every platform, inside one `FetchAtPos`):
1. `engine/dispatch/src/user_data/with_stores.rs` (`StoredRows::buffer_rows`) derives the query key from the raw (unsegmented) pending buffer (`phonetics::api::derive_custom_query_key`) and reads the matching custom rows (`CustomDictionaryStore::keystroke_rows`: the query's form by prefix, the abbreviation whole, shortest key first, 20 rows) — skipped when `AppConfig.custom_dictionary_disabled` — plus the learned phrases
2. The rows reach composing as `composing::UserRows`
3. System dictionary queried in the same fetch
4. Merge (`lexicon::continuous`): each custom row is sorted by `custom_reach` into the full block (typed in full), the partial-prefix block (final syllable typed into) or dropped; the abbreviation block takes the whole-abbreviation rows; deduplicated against the system words

**TPS Hanji conversion walk** (desktop, each walk of the closed readings): the walk hands the toneless body of every lattice edge (`ㄉㄢㄙㆩ`) to `StoredRows::rows_for_edge_keys` (`with_stores.rs`), which reads the custom rows (`CustomDictionaryStore::rows_with_keys`, unless `AppConfig.custom_dictionary_disabled`) and learned phrases filed under that `tps:notone` key whole — so a custom word anywhere in the walked text meets its edge. A TL / POJ stored roman keys the edge by its Bopomofo face (`composing::shadow::custom_toneless_key`), and the word commits under its TL-mode `(hanji, canonical TL)` pair. The same Bopomofo-face key lets the `FetchAtPos` walker (slot 0, every platform) take the custom and learned rows `StoredRows::buffer_rows` returns for a TPS buffer.

**Search page**: key prefix over `custom_search_key` in the query's family (tone-number or toneless form, plus the abbreviation form by prefix — `rows_matching`, the system dictionary search's own union)

---

## Derived Field Generation

Search keys are derived in the engine (`engine/phonetics/src/custom_search.rs` `derive_custom_search_keys`, over `derivation.rs`) whenever an entry is saved or imported, and re-derived once when a file is taken over (`rederive_search_keys_if_needed`).

### Toneless key (`derive_notone`)
1. Lowercase → base form (ⁿ / ᴺ → `nn`, `o͘` → `o`) → NFD → remove combining marks + digits + hyphens + spaces → NFC

Example: `"gâu-tsá"` → `"gautsa"`

### Abbreviation key (`derive_abbrev`)
1. Split by `-` or whitespace
2. Return `""` if < 2 syllables
3. Take the leading spelling unit of each syllable (§46), strip diacritics

Example: `"gâu-tsá"` → `"gt"`

---

## Default Entries

None. A fresh install starts with an empty custom dictionary on every platform.

---

## Differences from System Dictionary

| Aspect | Custom | System |
|--------|--------|--------|
| Priority | Never selected: after the selected words, ahead of the rest of the candidates that cover the whole input (`preference_class`), an abbreviation hit included; selected: by selection weight; wins a collision with the same word (source rank 0); the walker's edge pick weighs it with the dictionary's homophones and caps the span at the loaded dictionary model's custom-word cost | Corpus cost |
| Frequency tracking | `user_frequency.db`, like any pick — keyed by the `(display_text, canonical_tl)` pair, ranks custom rows among themselves (`lexicon::custom_entry_to_candidate` `user_weight`) | `user_frequency.db` (engine `UserFrequencyStore`) |
| Search method | Keystroke: roman / notone prefix, abbreviation whole (`keystroke_rows`), then the §43 reach rule; search page: prefix on all three (`rows_matching`) | FST, same rules |
| Segmentation | Raw input (unsegmented) | Segmented input |

---

## UI

### The list's romanization

Every platform's list shows each entry's romanization in the input mode in force, whichever spelling was saved: the platform sends `ListCustomEntries.input_mode` (and `tps_or_maps_to_er`), and the engine fills `CustomDictionaryEntry.display_roman` (`phonetics::api::custom_display_roman`). TL and TPS read a Latin spelling as TL, so a TL special final stays (`téng`); POJ reads it as POJ. English shows the saved spelling, and so does TPS for a row Bopomofo cannot spell (`iPhone`, `páh`). The edit form still starts from the saved `roman`, the filter still matches the saved spelling and the Hanji, and the Manage Dictionaries search keeps its own rendering.

A mode switch respells the rows on screen on macOS (`@AppStorage` change), Linux (the window's refresh) and Windows (the one-second settings beat); iOS and Android respell them at the page's next load.

### iOS (`CustomDictionaryView`)
- List with swipe-to-delete, add/edit modal sheet
- Import/Export buttons with help icon
- Delete All, no confirmation (maintainer 2026-10-16)

### Android (`CustomDictionaryScreen` + `CustomDictionaryActivity`)
- LazyColumn with inline delete, add/edit AlertDialog
- Import/Export section with help dialog
- Delete All, no confirmation (maintainer 2026-10-16)

### Desktop
- Windows / Linux: Custom Dictionary settings pane, model shared in `desktop/crates/taigi-desktop-core/src/settings/custom_dictionary.rs`
- macOS: `CustomDictionaryPage.swift` (`TaigiInputMethodCore/Lexicon/`)

---

## Platform Correspondence

| Function | iOS | Android |
|----------|-----|---------|
| Service | `UserDataClient.swift` (engine ops) | `UserDataClient.kt` (engine ops) |
| Entry model | `CustomDictionaryEntry.swift` | `CustomDictionaryWord` in `UserDataClient.kt` |
| List view | `CustomDictionaryView.swift` | `CustomDictionaryScreen.kt` |
| Edit view | (inline alert in `CustomDictionaryView.swift`) | (inline dialog) |
| Localization | `i18n/dictionary.json` → `StringKey.dictionary*` (resolver) | `i18n/dictionary.json` → `L10n` / `StringKey` |
