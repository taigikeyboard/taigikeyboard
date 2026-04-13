# Android Main App Tab1–Tab4 Review Plan

Review date: 2026-04-13

## Review Goals

1. **No redundant or duplicated code** — Eliminate identical strings, near-duplicate composables, and copy-paste patterns
2. **Adheres to clean code principles** — Single responsibility, clear intent, no dead code
3. **No over-engineering** — Only implement what's needed; don't merge distinct patterns into one overly-parameterized component
4. **No impact on existing functionality** — All changes are refactor-only; runtime behavior must be preserved
5. **Good readability, maintainability, and proper decoupling** — Centralize shared constants, reduce boilerplate where the benefit is clear
6. **Clear and consistent variable and file naming** — Follow project naming conventions; rename only when the current name is genuinely misleading

---

## Completed

### Zero Risk (pure cleanup)
- [x] Remove unused `subtypeManager` field + import + init — `SettingsMainActivity.kt`
- [x] Remove 5 unused `DictionaryInfoData` entries (moe, stti, newword, kungge, lkk) — `DictionarySettingsScreen.kt`
- [x] Remove unused `isSystemInDarkTheme` imports — `HomeScreen.kt`, `DictionarySettingsScreen.kt`
- [x] Remove redundant `updateLauncherIconStatus()` in `onDestroy` — `SettingsMainActivity.kt`

### Low Risk (visual consistency)
- [x] Unify trailing navigation icon: `ArrowForward` → `KeyboardArrowRight` (4 places) — `DictionarySettingsScreen.kt`
- [x] Replace inline `Text()` with shared `SectionHeader()` (2 places) — `LayoutScreen.kt`
- [x] Unify section spacing: `32.dp` → `AppStyle.sectionSpacing` (4 places) — `HomeScreen.kt`

### Small Changes (low risk)
- [x] Remove dead `onNewIntent` + empty `onDestroy` override — `SettingsMainActivity.kt`
- [x] Add error Toast for `resetAllSettings` failure (new `Tab4Texts.resetFailed`) — `SettingsMainActivity.kt`, `Tab4Texts.kt`
- [x] `DictionarySearchViewModel`: `PrefHelper` from per-search creation → class field — `DictionarySearchViewModel.kt`

---

## Stage B: Text / Localization

**Goal**: Eliminate duplicated localization strings, localize hard-coded text
**Status**: Complete

### B1+B2: Extract shared strings to `CommonTexts` *(MERGED — was B1 + B2)*

**Verdict**: KEEP — 14 exact duplicates confirmed across Tab1Texts/Tab3Texts/Tab4Texts. Direct deduplication, no behavior change.

Duplicated strings found:

| Key | Value | Files |
|-----|-------|-------|
| `moeDict` | 教育部臺灣台語常用詞辭典 | Tab1, Tab3 |
| `iTaigiDict` | iTaigi愛台語 | Tab1, Tab3 |
| `newwordDict` | 公視台語台台語新詞辭庫 | Tab1, Tab3 |
| `taiwanPlantDict` | 台灣植物名彙 | Tab1, Tab3 |
| `taiHuaDict` | 台華線頂對照典 | Tab1, Tab3 |
| `taiwanJapanDict` | 臺日大辭典台語譯本 | Tab1, Tab3 |
| `kunggeDict` | 工藝中心臺灣台語工藝詞庫 | Tab1, Tab3 |
| `sttiDict` | 教育部學科術語臺灣台語對譯 | Tab1, Tab3 |
| `accentDict` / `khpooDict` | 腔口差 | Tab1, Tab3 |
| `viewWebsite` | 官方網站 | Tab1, Tab3 |
| `cancel` | 取消 | Tab3, Tab4 |

**Plan**:
1. Create `localization/CommonTexts.kt` with all shared entries (dictionary names + `cancel` + `viewWebsite`)
2. Update `Tab1Texts`, `Tab3Texts`, `Tab4Texts` → reference `CommonTexts.xxx`
3. Remove duplicate entries from all three files
4. Update call sites in `HomeScreen.kt`, `DictionarySettingsScreen.kt`, `CopyrightScreen.kt`

### B3: Hard-coded string localization *(SIMPLIFIED)*

**Verdict**: KEEP — user-visible English error strings violate readability. Add to existing localization objects, no new files needed.

| File | String | Action |
|------|--------|--------|
| `DataManagementActivity.kt` | `"Export failed: ${e.message}"` | Add `Tab3Texts.exportFailed` |
| `DataManagementActivity.kt` | `"Import failed: ${e.message}"` | Add `Tab3Texts.importFailed` |
| `InputSettingsScreen.kt` | `"No email app found"` | Add `Tab4Texts.noEmailApp` |
| `DataManagementActivity.kt` | `"備份復原_$dateStr.taigi"` | Skip (filename, not user-facing) |

**Success Criteria**:
- No duplicate `LocalizedText` entries across Tab*Texts objects
- No user-visible hard-coded English strings in catch blocks

---

## Stage C: Larger Refactors — Review Verdicts

**Goal**: Improve code structure and consistency
**Status**: Reviewed — all items dropped (see rationale below)

### C1: Unify dictionary toggle components — DROPPED

**Original problem**: `DictRowWithDescription` and `DictionaryInfoSwitch` appear to serve the same purpose.

**Review finding**: These composables serve **distinct visual patterns**:
- `DictRowWithDescription` — multi-line layout, clickable URL label, description text below
- `DictionaryInfoSwitch` — single-line layout, info popup button, `enabled` parameter with alpha

Merging them into one component with optional `url`, `description`, `enabled` would create a conditional-heavy composable that's harder to read than two focused ones. This violates **goal 3 (no over-engineering)** — the current separation is clean single-responsibility design, not duplication.

### C2: Rename `FrequentWordsActivity` — DROPPED

**Original problem**: Name only reflects "frequency" but hosts both frequency and association screens.

**Review finding**: The name is understandable and follows Android convention (Activity named after its primary content). Renaming requires manual `AndroidManifest.xml` change, import updates, and provides minimal readability improvement. The `createIntent(context, type)` factory already documents the dual-purpose nature. Low value / high churn ratio violates **goal 3 (no over-engineering)**.

### C3: `InputSettingsScreen` state → ViewModel — DROPPED

**Original problem**: 15 `remember`/`mutableStateOf` pairs with `resetCounter` cascade.

**Review finding**: The `resetCounter` cascade is a valid Compose pattern:
```
resetCounter changes → remember(resetCounter) re-reads prefs → mutableStateOf updates → UI recomposes
```
15 state variables for 15 settings toggles is a 1:1 mapping — not reducible. Extracting to a ViewModel would:
- Add a new class without reducing state count
- Replace `remember`/`mutableStateOf` with `StateFlow`/`collectAsState` (same verbosity)
- Require migrating the `resetCounter` mechanism to ViewModel-internal logic

Net complexity stays the same or increases. Violates **goal 3 (no over-engineering)** — change the pattern only when it causes a real problem.

---

## Stage D: Folder Restructure — Complete

**Goal**: Rename `ui/settings/` → `ui/tabs/` and organize into tab-based sub-packages
**Status**: Complete

### Changes
- Renamed `ui/settings/` → `ui/tabs/` with sub-packages `tab1/`, `tab2/`, `tab3/`, `tab4/`
- Merged `DiagnosticTexts.kt` into `Tab4Texts.kt` (5 entries, prefixed with `diagnostic*`)
- Moved `diagnostics/DiagnosticService.kt` → `ui/tabs/tab4/`
- Deleted empty `diagnostics/` and `localization/DiagnosticTexts.kt`
- Updated all 8 Activity imports and removed self-import in `InputSettingsScreen`

### Final structure
```
ui/tabs/
├── MainSettingsScreen.kt       ← tab 容器
├── tab1/                       ← 頭頁 (Home)
├── tab2/                       ← 齒盤排列 (Layout)
├── tab3/                       ← 詞庫管理 (Dictionary)
└── tab4/                       ← 齒盤設定 (Settings)
```

---

## Simplify Review (Stage A code)

Code review of completed Stage A changes (6 files) found **no issues to fix**. Details:

- **Reuse**: No duplicated functionality introduced. Toast pattern is consistent with rest of app (~6 call sites, not enough to extract helper)
- **Quality**: All removals (subtypeManager, onNewIntent, onDestroy, unused imports/data) are clean. No dead code introduced
- **Efficiency**: PrefHelper class field promotion is safe (Application context via AndroidViewModel, no leak). `lifecycleScope` auto-cancels on destroy, so Toast race condition is not a concern

**Pre-existing issues noted** (not from this diff, for future cleanup):
- `HomeScreen.kt:62` — `externalLink` variable defined but never used (dead code)
- `SettingsMainActivity.kt:187` — `// Handle exception` comment in `openUrl` catch block does nothing

---

## Notes

- All changes must compile and not alter runtime behavior
- iOS has parallel `Tab1Texts.swift` etc. — dictionary name extraction (B1+B2) should be mirrored on iOS for cross-platform consistency
- All stages complete: A (cleanup), B (localization), D (folder restructure)
- Stage C dropped after review (no over-engineering)
