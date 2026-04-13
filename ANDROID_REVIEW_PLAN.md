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

## Completed Summary

### Stage A: Dead code removal, UI consistency fixes
- Removed unused fields, overrides, imports, data entries
- Unified trailing icons (`ArrowForward` → `KeyboardArrowRight`), section spacing, `SectionHeader` usage
- Added error Toast for `resetAllSettings` failure

### Stage B: Localization deduplication
- Extracted 14 shared `LocalizedText` entries into `CommonTexts.kt`
- Localized hard-coded English error strings

### Stage C: Larger refactors — all DROPPED (no over-engineering)

### Stage D: Folder restructure `ui/settings/` → `ui/tabs/`

### Stage E: Theme unification
- Merged `Type.kt` into `Theme.kt` with iOS-aligned Typography (34/18/17/14sp)
- Migrated 65+ call sites to `MaterialTheme.typography.*`
- Deleted `Type.kt`; AppStyle narrowed to spacing + colors only

### Stage F (F1–F4): Style variable centralization
- F1: Rewrote `ui-style-guide.md` to reflect current architecture
- F2: Replaced hardcoded `14.sp` → `MaterialTheme.typography.labelLarge` (3 files)
- F3: Extracted `AppStyle.scrollContentBottomPadding = 40.dp` (7 files)
- F4: Collapsed icon tiers 16/17/20dp → `smallIconSize=16dp` + `selectionIconSize=20dp`; unified trailing chevron `trailingChevronSize=24dp`

---

## Pending: F5 — Shared Row Composables (under evaluation)

**Goal**: Absorb repeated inline `Row(heightIn=48.dp, padding=20/12dp)` patterns into shared composables

### Analysis

10 inline Row patterns using `heightIn(min = 48.dp)` outside of existing components:

| Category | Count | Files |
|----------|-------|-------|
| LoadingRow (centered spinner) | 4 | DataManagement, Frequency, Association, CustomDictionary |
| SelectionRow (label + checkmark) | 2 | InputModeScreen, FontPickerContent |
| ValueNavigationRow (label + value + chevron) | 2 | AppearanceSettings, InputSettings |
| DataEntryRow (label + delete) | 1 | CustomDictionaryScreen |
| ToggleRow (label + info + switch) | 1 | DictionarySettingsScreen |

### Evaluation

- `SelectionRow` and `ValueNavigationRow` each have only **2 usages** — premature abstraction
- `LoadingRow` has **4 identical usages** — worth extracting
- Creating 2–3 new components increases API surface without significant complexity reduction
- **Alternative**: Extract `48.dp` / `20.dp` / `12.dp` as AppStyle constants (simpler, same single-source-of-truth benefit)

### Decision: Low priority, likely skip

`SelectionRow`/`ValueNavigationRow` 各 2 次用量不值得抽元件。`LoadingRow` (4 次) 是唯一可能值得的。Theme review 已完成，F5 屬於 component 重構範疇，非 theme 相關。

---

## Pre-existing issues (noted, not yet addressed)

- `HomeScreen.kt:62` — `externalLink` variable defined but never used (dead code)
- `SettingsMainActivity.kt:187` — `// Handle exception` comment in `openUrl` catch block does nothing

---

## Current Architecture (for next session reference)

```
ui/theme/
├── Theme.kt      ← M3 ColorScheme (Light/Dark) + AppTypography (HuninnFontFamily + iOS-aligned sizes)
└── AppStyle.kt   ← Icon sizes, spacing, dimensions, non-M3 colors (warningOrange, switchColors), SectionHeader
```

**Typography** → `MaterialTheme.typography.*` (headlineLarge=34sp, titleMedium=18sp, bodyLarge=17sp, labelLarge=14sp)
**Colors** → `MaterialTheme.colorScheme.*` + `AppStyle.warningOrange()` / `AppStyle.switchColors()`
**Icon sizes** → `AppStyle.trailingChevronSize(24dp)`, `smallIconSize(16dp)`, `selectionIconSize(20dp)`
**Spacing** → `AppStyle.sectionSpacing(24dp)`, `scrollContentBottomPadding(40dp)`, `sectionHeaderBottomPadding(6dp)`

## Verification (pending user testing)

- [ ] `cd android && ./gradlew assembleDebug` compiles
- [ ] Tab1–Tab4 screens render correctly
- [ ] Body text 18→17sp, section header 19→18sp visual change confirmed (iOS alignment)
- [ ] Trailing chevron icons uniformly 24dp across all tabs
- [ ] Info help icon renders at 16dp (was 17dp)
