# On-Demand Dictionary Download

> **Type**: Feature Spec (Planned)
> **Keywords**: `Dictionary`, `Download`, `Storage`, `Trie`, `SQLite`
> **Related**: trie.md, autocomplete.md, custom-dictionary.md
> **Status**: Phase 1 complete (split build + GitHub Release, 2026-03-21) — Phase 2-5 not started

---

## Summary

- Split bundled dictionary into core (必裝) + downloadable (按需) packages
- Users can download/delete individual dictionaries from Tab 3
- Reduces app size from ~50MB to ~25MB
- Dictionaries hosted on public GitHub repo (separate from private main repo)

---

## Current Architecture

```
Bundle (read-only):
  dictionary.db   (~40MB) — single SQLite, 12 sources as boolean columns
  dictionary.trie (~4MB)  — single MARISA trie, mmap-loaded

Query flow:
  Input → InputNormalizer → TrieService.lookup() → DictionaryRepository.queryByIds()
```

**Limitations**:
- MARISA trie is read-only, cannot add/remove entries at runtime
- All 12 dictionaries compiled into one file, cannot selectively remove
- Bundle resources cannot be modified after install

---

## Proposed Architecture

### Three-Layer Dictionary System

```
Layer 1: Core (bundled in app, always available)
  core.db + core.trie (~15-20MB)
  Contains: kautian (教育部) + dev supplement

Layer 2: Downloadable (user chooses)
  Per-dictionary packages stored in shared container:
    taigitv.db  + taigitv.trie   — 台語新詞辭庫
    itaigi.db   + itaigi.trie    — iTaigi 華台對照典
    taijit.db   + taijit.trie    — 台日大辭典
    sitbut.db   + sitbut.trie    — 台灣植物名彙
    taihoa.db   + taihoa.trie    — 台華線頂對照典
    kungge.db   + kungge.trie    — 台語工藝詞庫
    stti.db     + stti.trie      — 學科術語辭典
    khpoo.db    + khpoo.trie     — 腔口補充資料
    lkk.db      + lkk.trie       — LKK漢羅合用建議用字

Layer 3: User dictionary (existing, no change)
  custom_dictionary.db
```

### Storage Location

```
iOS:  App Group shared container
      ~/Library/Group Containers/group.com.siansiansu.TaigiKeyboard/
      ├── dictionaries/
      │   ├── core.db + core.trie           ← copied from bundle on first launch
      │   ├── taigitv.db + taigitv.trie     ← downloaded
      │   ├── itaigi.db + itaigi.trie       ← downloaded
      │   └── manifest.json                 ← tracks installed dictionaries + versions
      ├── custom_dictionary.db
      └── user_frequency.db

Android: context.filesDir/dictionaries/ (same structure)
```

Why App Group shared container:
- Keyboard Extension and Main App both need read access
- Downloaded files need writable location (Bundle is read-only)

### Query Flow (Refactored)

```
User input
    ↓
InputNormalizer
    ↓
DictionaryManager.getActiveDictionaries()
    ↓
┌─ core.trie lookup ──────── → core.db batch query
├─ taigitv.trie lookup ──── → taigitv.db batch query   (if installed + enabled)
├─ itaigi.trie lookup ───── → itaigi.db batch query    (if installed + enabled)
├─ ... (each installed dictionary)
└─ custom_dictionary search
    ↓
Merge + Dedup (by tl+hanzi) + Sort by frequency
    ↓
Return candidates
```

---

## Trie Strategy

**Multiple MARISA tries** (recommended over rebuilding single trie):
- Each dictionary has its own `.trie` file
- Query iterates all loaded tries, merges rowid results
- MARISA lookup is μs-level; even 10 tries < 1ms total
- Existing MARISA binding (iOS Swift / Android JNI) can be reused
- TrieService changes: manage array of trie instances instead of single instance

---

## Dictionary Package Format

Each downloadable dictionary is a zip containing:
```
taigitv-v1.0.zip
├── taigitv.db      — SQLite with same schema as current dictionary table
├── taigitv.trie    — MARISA trie (keys → rowids in this db)
└── manifest.json   — version, entry count, size, checksum
```

### manifest.json (per-package)
```json
{
  "id": "taigitv",
  "name": "台語新詞辭庫",
  "version": "1.0",
  "entries": 12345,
  "size_bytes": 3145728,
  "checksum_sha256": "abc123...",
  "min_app_version": "3.5.0"
}
```

### Remote manifest (server-side)
```json
{
  "format_version": 1,
  "dictionaries": [
    {
      "id": "taigitv",
      "name": "台語新詞辭庫",
      "description": "Contemporary Taiwanese vocabulary",
      "version": "1.0",
      "download_url": "https://github.com/siansiansu/taigikeyboard-dictionaries/releases/download/dict-v1.0/taigitv-v1.0.zip",
      "size_bytes": 3145728,
      "checksum_sha256": "abc123..."
    }
  ]
}
```

---

## Hosting

**Recommended: Public GitHub repo** (`taigikeyboard-dictionaries`)
- Main code stays private; dictionary data is from public sources (教育部, iTaigi, etc.)
- GitHub Releases for versioned downloads, no auth needed
- Stable URL format: `https://github.com/{owner}/taigikeyboard-dictionaries/releases/download/{tag}/{filename}`
- Free, sufficient bandwidth for current user scale

**Future migration**: Cloudflare R2 (free egress) when download volume exceeds GitHub limits.

---

## Build Pipeline Changes

Current: `dictionary/build.sh` → single `dictionary.db` + `dictionary.trie`

Proposed: `dictionary/build.sh` → per-source outputs
```
dictionary/output/
├── core.db + core.trie              ← kautian + dev
├── packages/
│   ├── taigitv-v1.0.zip
│   ├── itaigi-v1.0.zip
│   └── ...
└── remote-manifest.json             ← upload to GitHub Release
```

Build script needs new mode: `build.sh --split` to generate per-dictionary packages.

---

## Code Changes Required

### Modify Existing

| Component | Current | Change |
|-----------|---------|--------|
| `TrieService` | Load 1 trie from Bundle | Load N tries from shared container |
| `DictionaryRepository` | Query 1 db with WHERE filter | Query N dbs, merge results |
| `LexiconService` | Single trie + db pipeline | Iterate active dictionaries via DictionaryManager |
| `EnabledDictionaries` | SQL boolean filter | Check if dictionary is installed + enabled |
| `SharedSettings` | Per-source boolean toggles | Add download state tracking |
| Build pipeline | Single output | Per-source output + manifest generation |

### New Components

| Component | Responsibility |
|-----------|---------------|
| `DictionaryManager` | Registry of installed dictionaries, load/unload trie+db pairs |
| `DictionaryDownloader` | Download zip, verify checksum, extract to shared container |
| `DictionaryMigrator` | First-launch migration: copy core from bundle to shared container |
| Tab 3 UI | Dictionary list with download/delete/size info |

---

## Migration Plan

### First launch after update (v3.5 → v4.0)

1. Copy `core.db` + `core.trie` from bundle to shared container
2. Check if user had dictionaries enabled in old version
3. Show prompt: "以下辭典需要下載才能使用：[list]. 要現在下載嗎？"
4. Download enabled dictionaries in background
5. During download: core dictionary still works, partial functionality

### Rollback

- If download fails: user sees "辭典下載失敗，請稍後重試"
- Core dictionary always available as fallback
- No data loss — user frequency and custom dictionary unaffected

---

## Tab 3 UI Design

```
┌─────────────────────────────────┐
│ 辭典管理                         │
├─────────────────────────────────┤
│ ✅ 教育部臺灣台語常用詞辭典      │
│    15.2 MB · 核心辭典 (必裝)     │
├─────────────────────────────────┤
│ ✅ 台語新詞辭庫         [刪除]   │
│    3.1 MB · 已安裝              │
├─────────────────────────────────┤
│ ⬇️ iTaigi 華台對照典    [下載]   │
│    5.8 MB · 未安裝              │
├─────────────────────────────────┤
│ ✅ 台日大辭典           [刪除]   │
│    8.2 MB · 已安裝              │
├─────────────────────────────────┤
│ ...                             │
├─────────────────────────────────┤
│ 已使用空間：26.5 MB / 可釋放：11.3 MB │
└─────────────────────────────────┘
```

---

## Estimated Effort

| Phase | Work | Duration |
|-------|------|----------|
| Build pipeline split | Modify build.sh for per-source output | 2-3 days |
| DictionaryManager + multi-trie | Core architecture change | 3-5 days |
| Download/delete flow | Network + file management | 2-3 days |
| Tab 3 UI | iOS + Android | 2-3 days |
| Migration + testing | First-launch migration, edge cases | 2-3 days |
| **Total** | | **~2-3 weeks** |

---

## Open Questions

1. Should `variant` (異用字) and `khiin` (在來字) be separate downloadable dictionaries or flags within each dictionary?
2. Should NextWord association data be per-dictionary or kept in core only?
3. Version update strategy: full re-download or incremental patches?
4. Offline-first: should all dictionaries be pre-downloaded on Wi-Fi?
