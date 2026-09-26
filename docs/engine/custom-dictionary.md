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
| `roman` | String | Romanization in the user's native form (TL or POJ display, whichever the user typed when saving). The engine treats it raw on the lattice axis; the freq-key commit value is canonicalized to TL at synthesis time (`phonetics::api::canonical_tl_form`, v3.5.9 B-4). |
| `hanzi` | String | Chinese/Taiwanese characters |
| search keys | `custom_search_key` rows | Derived per entry by the engine: TL / POJ / TPS families × tone-number / toneless / abbreviation forms (`engine/phonetics/src/custom_search.rs`) |
| `createdAt` | Timestamp | Creation time (UTC) |
| `updatedAt` | Timestamp | Last update time |

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

The `custom_search_key` side table backs cross-input-mode (three-index TL / POJ / TPS) lookup. A file taken over from an iOS / Android release keeps its legacy `notone` / `abbrev` / `roman_num` columns (roadmap U7 / U8); the engine neither reads nor writes them.

---

## CRUD Operations

Engine ops (`UserDataRequest`, `engine/protos/proto/user_data.proto`), reached through each platform's `UserDataClient` (iOS `Lexicon/Services/UserDataClient.swift`, Android `ime/dictionary/UserDataClient.kt`, macOS `Settings/UserDataClient.swift`):

| Operation | Engine op | iOS / Android `UserDataClient` |
|-----------|-----------|-----|
| Create/Update | `SaveCustomEntry` | `save` |
| Read all / page | `ListCustomEntries` (page + `total` + `matching_total`) | `listAll` |
| Search | `SearchCustomEntries` | `search` |
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
2. Parse CSV (data-only, 2 columns: roman, hanzi)
3. Validate format
4. Deduplicate by `roman|hanzi` key
5. Return `ImportResult { imported: Int, skipped: Int }`

CSV is **custom-dictionary-only**. For whole-user-data backup see `.taigi` below.

---

## `.taigi` Backup (whole user data)

A `.taigi` file is a single plain-text **JSON** document carrying **three** user-writable SQLite DBs in one container — `custom_dictionary.db`, `user_frequency.db`, `user_association.db` (`learned_phrases.db` is excluded, §50). It is the **only** cross-device user-data path on the phones (their DBs are excluded from OS auto-backup — behavioral-invariants.md §29).

| File | Responsibility |
|------|----------------|
| engine `engine/userdata/src/backup.rs` (ops `ExportBackup` / `ImportBackup`) | the one encoder / decoder |
| iOS `App/Tabs/Dictionary/Utilities/BackupDocument.swift` + `UserDataClient.exportBackup` / `importBackup` | file document; `UTType.taigiBackup` = `tw.taigikeyboard.backup` |
| Android `ui/tabs/dictionary/DataManagementViewModel.kt` + `UserDataClient.exportBackup` / `importBackup` | file picker / share, bytes to the engine |

### Format

- Top-level: `version`, `exportedAt` (ISO-8601 UTC), `platform` (`ios`/`android`), `appVersion`, plus `customDictionary[]` (`roman`, `hanzi`), `userFrequency[]` (`word`, `tl`, `count`, `lastUsed`), `userAssociation[]` (`prevWord`, `prevTl`, `nextWord`, `nextTl`, `count`, `lastUsed`).
- **Version**: export writes `2`; import accepts `version >= 1`.
- **v1 → v2**: v2 adds the per-row `tl` (canonical-TL) field to `userFrequency`, supporting the `(Hanji, canonical-TL)` pair identity (Core Principle #7 / R5). `tl` is optional on decode — a pre-R5 v1 backup with no `tl` imports into the legacy `tl=""` bucket.
- Custom-dict rows carry only `roman`+`hanzi`; internal id/timestamps/derived columns are regenerated on import.

### Import semantics — **merge, never replace**

- Custom dictionary: adds non-duplicates only (dedup by `roman\thanzi`).
- Frequency + association: **higher-count-wins** merge. Readings are normalized POJ→canonical-TL in the engine so cross-platform backups round-trip; capacity is enforced after the import.

The format is **round-trip compatible across platforms** (same schema, same v2, both tolerate missing `tl`).

> **Row cap (30000)**: a `.taigi` restore fills the space left and stops (`CustomDictionaryStore::import_until_full`, Android's former rule, now every platform); a CSV whose own row count exceeds the cap is refused before anything is written (`batch_import`). The earlier iOS backup-path abort (behavioral-invariants.md §27) went with the native store.

---

## Autocomplete Integration

Custom entries have **highest priority** — shown before system dictionary results.

**Flow** (every platform, inside one `FetchAtPos`):
1. `engine/dispatch/src/user_data.rs` (`buffer_rows`) derives the query key from the raw (unsegmented) pending buffer (`derive_custom_query_key`) and reads the matching custom rows — skipped when `FetchAtPos.custom_dictionary_disabled` — plus the learned phrases
2. The rows reach composing as `composing::UserRows`
3. System dictionary queried in the same fetch
4. Merge: custom words first, then system words, deduplicated

**Search**: key prefix over `custom_search_key` in the query's family (tone-number or toneless form, plus the abbreviation form)

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

The engine seeds an empty dictionary on open (`CustomDictionaryStore::seed_if_empty`, every platform):
```
id: "default-gau-tsa",       roman: "gâu-tsá",        hanzi: "𠢕早"
id: "default-tsiah-pa-bue",  roman: "tsia̍h-pá--buē",  hanzi: "食飽未"
```

---

## Differences from System Dictionary

| Aspect | Custom | System |
|--------|--------|--------|
| Priority | Highest (shown first) | Normal |
| ID marker | `-2` | `≥ 0` (row ID) |
| Frequency tracking | `user_frequency.db`, like any pick — keyed by the `(display_text, canonical_tl)` pair, ranks custom rows among themselves (`lexicon::custom_entry_to_candidate` `user_weight`) | `user_frequency.db` (engine `UserFrequencyStore`) |
| Search method | Prefix (roman/notone/abbrev) | Trie-based |
| Segmentation | Raw input (unsegmented) | Segmented input |

---

## UI

### iOS (`CustomDictionaryView`)
- List with swipe-to-delete, add/edit modal sheet
- Import/Export buttons with help icon
- Delete All with confirmation

### Android (`CustomDictionaryScreen` + `CustomDictionaryActivity`)
- LazyColumn with inline delete, add/edit AlertDialog
- Import/Export section with help dialog
- Delete All with confirmation

---

## Platform Correspondence

| Function | iOS | Android |
|----------|-----|---------|
| Service | `UserDataClient.swift` (engine ops) | `UserDataClient.kt` (engine ops) |
| Entry model | `CustomDictionaryEntry.swift` | `CustomDictionaryWord` in `UserDataClient.kt` |
| List view | `CustomDictionaryView.swift` | `CustomDictionaryScreen.kt` |
| Edit view | (inline alert in `CustomDictionaryView.swift`) | (inline dialog) |
| Localization | `i18n/dictionary.json` → `StringKey.dictionary*` (resolver) | `i18n/dictionary.json` → `L10n` / `StringKey` |
