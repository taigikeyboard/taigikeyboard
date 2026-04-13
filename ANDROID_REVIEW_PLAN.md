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

## Stage E: Theme Unification — Complete

**Goal**: Merge two parallel font-size systems into one, align Android/iOS font sizes, reduce file count
**Status**: Complete

### Background

The `ui/theme/` directory has 3 files with two parallel font-size systems:

| File | Role |
|------|------|
| `Theme.kt` | Color scheme + `TaigiKeyboardTheme` wrapper |
| `Type.kt` | Material3 Typography — only provides `HuninnFontFamily`, size definitions never referenced |
| `AppStyle.kt` | Font sizes (raw `.sp`), spacing, colors, `SectionHeader` composable |

**Problem**: Screens use `AppStyle.bodyFontSize` (raw `.sp`) instead of `MaterialTheme.typography.*`. This creates two parallel systems — AppStyle for sizes, Typography for font family. Type.kt's 15 slot size definitions are dead code.

**Cross-platform misalignment** (Android body=18sp, iOS body=17pt; Android sectionHeader=19sp, iOS=18pt).

### M3 Component Side-Effect Analysis

Only one M3 slot change has a side effect:

| M3 Component | Slot | Default | New | Impact |
|--------------|------|---------|-----|--------|
| OutlinedTextField input | `bodyLarge` | 16sp | **17sp** | +1sp, negligible |
| AlertDialog text | `bodyMedium` | 14sp | unchanged | none |
| TopAppBar title | `headlineSmall` | 24sp | unchanged | none |
| Button/SegmentedButton | `labelLarge` | 14sp | unchanged (=captionFontSize) | none |
| NavigationBar label | `labelMedium` | 12sp | unchanged | none |

### Codex Review Findings

1. **`labelLarge` for caption — semantic mismatch but safe**: M3 defines `labelLarge` for buttons/tabs. App uses `captionFontSize` for dates, trailing values, badges. However, `labelLarge` default is already 14sp = `captionFontSize`, so **no size change occurs** — M3 buttons/TextButtons/SegmentedButtons are unaffected. The mismatch is naming only. Alternatives (`bodySmall` 12sp, `labelMedium` 12sp) would require size changes that affect M3 components — worse tradeoff.

2. **`AppStyle.bodyFontSize` used for icon sizing**: `SettingInfoButton.kt:48` uses `.size(AppStyle.bodyFontSize.value.dp)` to size an info icon. This is a non-text usage — replace with direct `.size(17.dp)` during migration.

3. **`sectionHeaderFontSize` direct usages (7 places)**: Used in `Text()` outside `SectionHeader` composable at `DataManagementScreen.kt:97`, `FrequencyDataScreen.kt:230,311`, `AssociationDataScreen.kt:242,323`, `CustomDictionaryScreen.kt:243,339`. Must be included in E2 migration.

4. **Hardcoded `.sp` values outside AppStyle** (pre-existing, out of scope): `SetupGuideScreen.kt`, `ColorPickerDialog.kt`, `CopyrightScreen.kt` have raw sp values. Not part of this refactor.

5. **Scattered `fontFamily` overrides** (pre-existing, out of scope): Some files explicitly set `fontFamily`. Not part of this refactor.

### Plan

#### E1: Merge Type.kt into Theme.kt

1. Move `HuninnFontFamily` and `Typography` definition from `Type.kt` into `Theme.kt`
2. Customize Typography with app's actual sizes (aligned with iOS):

| App semantic | M3 slot | Size (aligned with iOS) | Notes |
|-------------|---------|------------------------|-------|
| Page title | `headlineLarge` | 34sp (iOS 34pt) | M3 default 32sp → 34sp. No M3 component in app uses this slot internally |
| Section header | `titleMedium` | 18sp (iOS 18pt) | M3 default 16sp → 18sp. No M3 component in app uses this slot internally |
| Body text | `bodyLarge` | 17sp (iOS 17pt) | M3 default 16sp → 17sp. OutlinedTextField input +1sp (negligible) |
| Caption | `labelLarge` | 14sp (iOS 14pt) | **Already 14sp — no change**. Buttons/SegmentedButtons unaffected |

3. All other slots: M3 defaults + HuninnFontFamily (no size change)
4. Delete `Type.kt`

#### E2: Migrate call sites to MaterialTheme.typography

Replace all `AppStyle.*FontSize` usages with `MaterialTheme.typography.*` across all screens:

| Before | After |
|--------|-------|
| `fontSize = AppStyle.pageTitleFontSize` | `style = MaterialTheme.typography.headlineLarge` |
| `fontSize = AppStyle.sectionHeaderFontSize` | `style = MaterialTheme.typography.titleMedium` |
| `fontSize = AppStyle.bodyFontSize` | `style = MaterialTheme.typography.bodyLarge` |
| `fontSize = AppStyle.captionFontSize` | `style = MaterialTheme.typography.labelLarge` |

**Special cases**:
- `SettingInfoButton.kt:48` — `.size(AppStyle.bodyFontSize.value.dp)` → `.size(17.dp)` (icon sizing, not text)
- Call sites with explicit `fontWeight`, `color`, `lineHeight` — these override `style` per Compose precedence, no conflict

Migrate screen by screen. Each screen must compile after migration.

#### E3: Clean up AppStyle.kt

1. Remove the 4 font size constants (`pageTitleFontSize`, `sectionHeaderFontSize`, `bodyFontSize`, `captionFontSize`)
2. AppStyle retains: spacing (`sectionSpacing`, `sectionHeaderBottomPadding`), height (`largeTopAppBarExpandedHeight`), colors (`warningOrange`, `switchColors`)
3. Update `SectionHeader` composable to use `style = MaterialTheme.typography.titleMedium` instead of `fontSize = AppStyle.sectionHeaderFontSize`

### Final architecture

```
ui/theme/
├── Theme.kt      ← color scheme + Typography (HuninnFontFamily + app sizes) + TaigiKeyboardTheme
└── AppStyle.kt   ← spacing + colors + SectionHeader composable
```

- **2 files** (was 3)
- **1 font system** (was 2)
- **0 dead code** in Typography (all customized slots are used, rest are M3 defaults)
- **iOS-aligned sizes** (34 / 18 / 17 / 14)

### Success Criteria

- [x] Type.kt deleted, Typography defined in Theme.kt
- [x] All `AppStyle.*FontSize` references replaced with `MaterialTheme.typography.*`
- [x] `SettingInfoButton.kt:48` icon size replaced with direct `17.dp`
- [x] 4 font size constants removed from AppStyle
- [x] SectionHeader uses `style = MaterialTheme.typography.titleMedium`
- [ ] No compilation errors
- [ ] UI appearance unchanged (except body 18→17sp, sectionHeader 19→18sp to align with iOS)

### Changes Made

**E1**: Moved `HuninnFontFamily` + `Typography` into Theme.kt as private `AppTypography`. Customized 3 slots (`headlineLarge`=34sp, `titleMedium`=18sp, `bodyLarge`=17sp). Deleted `Type.kt`.

**E2**: Migrated 65+ call sites across 20 files from `fontSize = AppStyle.*FontSize` to `style = MaterialTheme.typography.*`. Replaced `SettingInfoButton` icon size with direct `17.dp`. Removed unused `AppStyle` imports from 9 files.

**E3**: Removed 4 font size constants from `AppStyle`. Updated `SectionHeader` composable. AppStyle now holds only spacing, dimensions, and colors.

**What simplified**: Eliminated the parallel "AppStyle for sizes, Typography for font family" split. One system, one entry point (`MaterialTheme.typography.*`), standard M3 pattern. AppStyle's responsibility narrowed from mixed concerns to spacing + colors only.

---

## Stage F: Style Variable Centralization — Pending Review

**Goal**: Centralize scattered hardcoded UI constants into AppStyle so that changing a value only requires editing one place
**Status**: Audit complete, pending Codex review

### Completed (trailing chevron)

- [x] Added `AppStyle.trailingChevronSize = 24.dp`
- [x] Updated `ActionRow`, `NavigationRow`, `DetailScreen`, `AppearanceSettingsScreen`, `InputSettingsScreen`
- [x] Updated `ui-style-guide.md` trailing chevron 18 → 24

### Already centralized via reusable components (no action needed)

These values are hardcoded but live inside a single reusable component — changing one file updates all call sites:

| Value | Semantic | Component |
|-------|----------|-----------|
| 24dp | Leading icon size | `ActionRow`, `SwitchRow`, `NavigationRow` |
| 56dp | Action row min height | `ActionRow` |
| 12dp | Icon-text gap | `ActionRow`, `SwitchRow`, `NavigationRow` |
| 12dp | Card corner radius | `SettingsCard` |

### Scattered values (candidates for AppStyle extraction)

Values repeated across multiple screen files with the same semantic meaning. Changing requires editing many files today.

#### Row height

| Value | Semantic | Files | Occurrences |
|-------|----------|-------|-------------|
| `heightIn(min = 48.dp)` | Setting row min height (touch target) | 11 | 12 |

#### Padding

| Value | Semantic | Files | Occurrences |
|-------|----------|-------|-------------|
| `padding(horizontal = 20.dp, vertical = 12.dp)` | Row content padding (in custom rows outside ActionRow/SwitchRow) | 12 | 14 |
| `padding(bottom = 40.dp)` | Screen bottom safe area | 4 | 4 |

#### Icon sizes

| Value | Semantic | Files | Occurrences |
|-------|----------|-------|-------------|
| `Modifier.size(20.dp)` | Checkmark / reset icon | 4 | 4 |
| `.size(17.dp)` | Help info icon (`SettingInfoButton`) | 1 | 1 |
| `.size(16.dp)` | Small detail icon (external link, warning) | 4 | 7 |

#### Redundant hardcoded font sizes

| Value | Semantic | Files | Occurrences | Should be |
|-------|----------|-------|-------------|-----------|
| `fontSize = 14.sp` | Caption text (CopyrightScreen, SetupGuideScreen, ColorPickerDialog) | 3 | 4 | `MaterialTheme.typography.labelLarge` |

### Not centralizing (layout-specific one-offs)

- Micro spacers (`4.dp`, `6.dp`, `8.dp`) — fine-tuning, not reusable semantics
- `200.dp` max height — search dropdown only
- `ColorPickerDialog` internal sizes — self-contained component
- `SetupGuideScreen` step number circle `22.dp` — unique visual element

### Codex Review Verdicts

#### Q1: 48.dp row height (11 files, 12 occurrences)

**Verdict**: Don't extract a raw constant — create shared composables instead.

The repeated `48.dp` rows are not one thing: some are value+chevron rows (`AppearanceSettingsScreen`, `InputSettingsScreen`), some are selectable checkmark rows (`InputModeScreen`, `FontPickerContent`), some are loading rows. Extracting only `AppStyle.rowMinHeight` centralizes the number but leaves the code duplication intact.

**Action**: Introduce focused shared composables (`SelectionRow`, `ValueNavigationRow`) that own both `heightIn(min = 48.dp)` and `padding(horizontal = 20.dp, vertical = 12.dp)` internally. Only add a raw AppStyle height token if a leftover one-off cannot fit any shared component.

#### Q2: 20dp + 12dp padding (12 files, 14 occurrences)

**Verdict**: Same as Q1 — migrate to shared components, not raw constants.

`ActionRow` and `SwitchRow` already encode `20/12`. The remaining inline rows duplicate the full Row+clickable+alignment+content pattern, not just the padding. `AppStyle.rowHorizontalPadding` would still leave scattered boilerplate.

**Action**: Bundle with Q1 — the new shared composables own both the min height and the padding together.

#### Q3: Small icon sizes (16dp / 17dp / 20dp)

**Verdict**: Collapse to two tiers.

- **16dp** → small/detail/help icons (external link, warning, info button)
- **20dp** → selected/reset affordances (checkmark, refresh)

The `17dp` in `SettingInfoButton` is a singleton that conflicts with the style guide. M3 doesn't define a formal mini-scale beyond the standard 24dp.

**Action**: Change `SettingInfoButton` from `17.dp` to `16.dp`. Add two tokens: `AppStyle.smallIconSize = 16.dp`, `AppStyle.selectionIconSize = 20.dp`. Update style guide to match.

#### Q4: 40.dp screen bottom padding (4 files)

**Verdict**: Extract it.

This is not a system-bar substitute — these screens already apply `innerPadding` from Scaffold before the extra `40.dp`. It's a real shared spacing token for scroll breathing room.

**Action**: Extract as `AppStyle.scrollContentBottomPadding = 40.dp`. Normalize lazy-list cases to use content padding instead of a terminal Spacer where practical.

#### Q5: 14.sp hardcoded (3 files)

**Verdict**: Replace with `MaterialTheme.typography.labelLarge`.

The `14.sp` in `CopyrightScreen` and `ColorPickerDialog` is straightforward typography cleanup. Exception: the step-number badge in `SetupGuideScreen` (`14.sp` inside a `22.dp` circle) is unique — use `MaterialTheme.typography.labelLarge.copy(fontWeight = FontWeight.Bold)` or keep it local.

#### Q6: Anything missed?

**Style guide is outdated** — `ui-style-guide.md` still references `Type.kt`, says `AppStyle.kt` contains font sizes, tells people to use `AppStyle.bodyFontSize`, and has wrong icon sizes (guide: trailing `18`, info `15`/`18dp`; actual: trailing `24dp`, info `17dp`). Fix this first — zero code churn, removes inaccuracies that will mislead future work.

**ColorPickerDialog exception** — "not centralizing" is too broad. Its `14.sp` text should follow typography tokens; only canvas drawing values (spectrum colors, thumb geometry) should stay local.

### Execution Plan (priority order, lowest churn first)

| Step | Task | Files | Risk |
|------|------|-------|------|
| F1 | Update `ui-style-guide.md` to reflect Stage E reality (Typography in Theme.kt, correct icon sizes) | 1 | Zero |
| F2 | Replace `14.sp` → `MaterialTheme.typography.labelLarge` | 3 | Low |
| F3 | Extract `AppStyle.scrollContentBottomPadding = 40.dp` | 4 | Low |
| F4 | Collapse icon tiers: add `AppStyle.smallIconSize = 16.dp` + `AppStyle.selectionIconSize = 20.dp`, change `SettingInfoButton` 17→16 | ~6 | Low |
| F5 | New shared row composables (`SelectionRow`, `ValueNavigationRow`) to absorb repeated 48dp + 20/12 padding patterns | ~12 | Medium |

### Success Criteria

- [ ] `ui-style-guide.md` accurately reflects current codebase (no stale references)
- [ ] No hardcoded `14.sp` in screen files
- [ ] `40.dp` bottom padding comes from `AppStyle.scrollContentBottomPadding`
- [ ] Small icon sizes use `AppStyle.smallIconSize` (16dp) or `AppStyle.selectionIconSize` (20dp)
- [ ] Repeated custom Row patterns absorbed into shared composables
- [ ] No compilation errors

---

## Notes

- All changes must compile and not alter runtime behavior (except intentional 1sp iOS alignment)
- iOS has parallel `Tab1Texts.swift` etc. — dictionary name extraction (B1+B2) should be mirrored on iOS for cross-platform consistency
- Stages A, B, D, E complete; Stage C dropped (no over-engineering); Stage F plan reviewed
