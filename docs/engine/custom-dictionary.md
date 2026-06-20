# Custom Dictionary

> **Type**: Feature
> **Keywords**: `CustomDictionary`, `Import`, `Export`, `CSV`
> **Related**: autocomplete.md, binary-format.md

---

## Summary

- User-defined dictionary entries (romanization + hanzi pairs)
- CRUD operations with SQLite storage
- CSV import/export
- Integrated into autocomplete with highest priority (before system dictionary)

---

## Data Model

| Field | Type | Description |
|-------|------|-------------|
| `id` | String (UUID) | Unique identifier |
| `roman` | String | Romanization in the user's native form (TL or POJ display, whichever the user typed when saving). The engine treats it raw on the lattice axis; the freq-key commit value is canonicalized to TL at synthesis time (`phonetics::api::canonical_tl_form`, v3.5.9 B-4). |
| `hanzi` | String | Chinese/Taiwanese characters |
| `notone` | String | Derived: toneless form for matching |
| `abbrev` | String | Derived: first letter of each syllable (min 2 syllables) |
| `createdAt` | Timestamp | Creation time (UTC) |
| `updatedAt` | Timestamp | Last update time |

Example: `roman="gâu-tsá"` → `notone="gautsa"` → `abbrev="gt"`

---

## Storage

**Database**: `custom_dictionary.db` (iOS: App Group shared container; Android: app-private storage)

```sql
CREATE TABLE custom_dictionary (
    id              TEXT PRIMARY KEY,
    roman           TEXT NOT NULL,
    hanzi           TEXT NOT NULL,
    notone          TEXT DEFAULT '',           -- derived
    abbrev          TEXT DEFAULT '',           -- derived
    roman_num       TEXT DEFAULT '',           -- derived (v3.6.1-R3)
    created_at      TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at      TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_custom_roman ON custom_dictionary(roman);
CREATE INDEX idx_custom_notone ON custom_dictionary(notone);
CREATE INDEX idx_custom_abbrev ON custom_dictionary(abbrev);
CREATE INDEX idx_custom_roman_num ON custom_dictionary(roman_num);
```

A `custom_search_key` side table backs cross-input-mode (三索引 TL / POJ / TPS) lookup — see `CustomDictionarySchema.swift` (iOS) / `CustomDictionaryService.kt` (Android) for its DDL.

---

## CRUD Operations

| Operation | iOS (`CustomDictionaryRepository`) | Android (`CustomDictionaryService`) |
|-----------|-----|---------|
| Create/Update | `upsert(_ entry)` async | `save(entry)` suspend |
| Read all | `fetchAll()` async | `fetchAll()` suspend |
| Search | `searchSync(family:, form:, key:, limit:)` | `search(family, form, key, limit)` suspend |
| Delete one | `delete(id:)` async | `delete(id)` suspend |
| Delete all | `deleteAll()` async | `deleteAll()` suspend |
| Count | `count()` async | `totalCount()` sync |
| Batch import | `batchImport(_ entries:)` async | (via loop) |

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

---

## Autocomplete Integration

Custom entries have **highest priority** — shown before system dictionary results.

**Flow** (both platforms):
1. `LexiconService.search()` calls custom dictionary search with raw (unsegmented) input
2. Results converted to `TaigiWord` with `id = -2` marker
3. System dictionary queried separately
4. Merge: custom words first, then system words, deduplicated

**Search uses three indexes**: roman prefix, notone prefix, abbrev prefix

---

## Derived Field Generation

### `generateNotone(roman)` → String
1. Lowercase → NFD → remove combining marks + digits + hyphens + spaces → NFC

Example: `"gâu-tsá"` → `"gautsa"`

### `generateAbbrev(roman)` → String
1. Split by `-` or space
2. Return `""` if < 2 syllables
3. Take first char of each syllable, strip diacritics

Example: `"gâu-tsá"` → `"gt"`

---

## Default Entries

Both platforms seed if empty:
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
| Frequency tracking | Not tracked | Via UserFrequencyService |
| Search method | Prefix (roman/notone/abbrev) | Trie-based |
| Segmentation | Raw input (unsegmented) | Segmented input |

---

## UI

### iOS (`CustomDictionaryView` + `CustomDictionaryEditView`)
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
| Service | `CustomDictionaryRepository.swift` | `CustomDictionaryService.kt` |
| Entry model | `CustomDictionaryEntry.swift` | nested `Entry` in `CustomDictionaryService.kt` |
| List view | `CustomDictionaryView.swift` | `CustomDictionaryScreen.kt` |
| Edit view | `CustomDictionaryEditView.swift` | (inline dialog) |
| Localization | `DictionaryTexts.swift` | `DictionaryTexts.kt` |
