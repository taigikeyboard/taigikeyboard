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

### Decision: TBD

Options:
1. **Extract only `LoadingRow`** (4 usages) + raw constants for 48/20/12dp
2. **Skip entirely** — the remaining duplication is manageable
3. **Full extraction** — all 3 composables (highest churn)

---

## Pre-existing issues (noted, not yet addressed)

- `HomeScreen.kt:62` — `externalLink` variable defined but never used (dead code)
- `SettingsMainActivity.kt:187` — `// Handle exception` comment in `openUrl` catch block does nothing

---

## Verification (pending user testing)

- [ ] `cd android && ./gradlew assembleDebug` compiles
- [ ] Tab1–Tab4 screens render correctly
- [ ] Body text 18→17sp, section header 19→18sp visual change confirmed (iOS alignment)
- [ ] Trailing chevron icons uniformly 24dp across all tabs
- [ ] Info help icon renders at 16dp (was 17dp)
