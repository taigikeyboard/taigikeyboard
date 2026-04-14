# Android Review Plan

## Phase 1: Tab1–Tab4 Review (2026-04-13) — Complete (A–F5)

## Phase 2: `ui/components/` Review (2026-04-14)

**Scope**: 14 files in `ui/components/` — shared composables, dialogs, icons

### Review Summary

| # | Finding | Severity | Stage |
|---|---------|----------|-------|
| 1 | `SegmentedButtonRow.kt` is dead code (0 usages) | Medium | G1 |
| 2 | Hardcoded "OK" button text in SettingInfoButton | Medium | G2 |
| 3 | `ColorRow` and `SliderRow` missing `modifier` parameter | Low | G3 |
| 4 | Row height inconsistency: ActionRow 56dp vs others 48dp | Note | — |
| 5 | NavigationRow padding 16dp vs others 20dp | Note | — |

### Finding Details

**1. Dead code: `SegmentedButtonRow.kt`**

`SegmentedButtonRow` and `SegmentedOption` have zero usages. The only segmented button in the app (`ColorPickerDialog.kt:135`) uses `SingleChoiceSegmentedButtonRow` directly with custom content (`icon = {}`, `fontSize = 13.sp`) that doesn't match SegmentedButtonRow's API. Adapting the component to fit would add complexity for a single call site.

**Action**: Delete `SegmentedButtonRow.kt`.

**2. Hardcoded "OK" button text in SettingInfoButton**

`SettingInfoButton.kt:88` — `"OK"` is a user-visible button label baked into the component. ContentDescription strings ("Reset"×2, "Info") are accessibility-only and follow the project-wide pattern of English contentDescription (also seen in "Back", "Close", "Clear", "Open" across all screens) — not a component-specific issue.

**Action**: Add `dismissLabel: String = "OK"` parameter to `SettingInfoButton` (same pattern as `ConfirmationDialog`/`ResultDialog`). Add `CommonTexts.ok` so callers with `languageManager` can localize.

**3. Missing `modifier` parameter**

`ColorRow` and `SliderRow` do not accept a `modifier: Modifier = Modifier` parameter, unlike every other row component (ActionRow, SwitchRow, NavigationRow, LoadingRow). This breaks composability — callers cannot apply padding, test tags, or accessibility modifiers.

**Action**: Add `modifier: Modifier = Modifier` parameter to both, thread it to the root composable.

**4–5. Row dimension inconsistency (noted, no action)**

- **ActionRow** uses `heightIn(min = 56.dp)` while ColorRow/LoadingRow/SwitchRow use `48.dp`. This appears intentional — ActionRow is used for prominent action buttons that benefit from a larger touch target.
- **NavigationRow** uses `padding(16.dp)` while others use `padding(horizontal = 20.dp, vertical = 12.dp)`. NavigationRow is only used in HomeScreen (tab-style navigation links) where tighter padding matches the visual density. Different from settings rows.

Both patterns are intentional design choices, not bugs. No change needed.

### Items Reviewed — No Issues Found

- **TaigiIcons.kt** (1173 lines, 21 icons): All icons used. 9 Filled emoji icons in `EmojiCategory.kt`, 5 Outlined icons used directly, 7 Outlined icons used via `SettingsIcons` facade. File is large but intentional (avoids `material-icons-extended` dependency).
- **SettingsIcons.kt**: Clean facade, used in 2 files (InputSettingsScreen, SettingsOverlayContent) to sync icons between app and keyboard overlay.
- **ConfirmationDialog / ResultDialog**: Clean, focused, properly used (4 / 24 usages respectively).
- **SettingsCard / SettingsDivider**: Ubiquitous (58 / 49 usages), clean implementations.
- **ActionRow vs NavigationRow**: Different purposes — ActionRow (optional ImageVector icon, haptic feedback, 56dp) for settings actions, NavigationRow (required Painter icon, no haptic, 16dp padding) for HomeScreen navigation. Keeping separate is correct.
- **SwitchRow**: Clean composition with `SettingInfoButton`, `AppStyle.switchColors()`, `LocalMinimumInteractiveComponentSize`. 24 usages.
- **LoadingRow**: Clean extraction from F5 refactor. 4 usages.

---

### Stages

#### Stage G1: Dead code removal — Complete
**Changes**: Deleted `SegmentedButtonRow.kt` (0 usages)

#### Stage G2: Parameterize hardcoded "OK" button — Complete
**Changes**:
- Added `CommonTexts.ok = LocalizedText(hanji = "好")`
- Added `dismissLabel: String = "OK"` parameter to `SettingInfoButton`
- Internal usage now references `dismissLabel` instead of hardcoded `"OK"`

#### Stage G3: Add missing `modifier` parameters — Complete
**Changes**:
- `ColorRow.kt`: Added `modifier: Modifier = Modifier`, threaded to root `Row`
- `SliderRow.kt`: Added `modifier: Modifier = Modifier`, threaded to root `Column`

---

### Tests

**Current state**: Zero component tests exist. All 10 existing Android tests are in `ime/dictionary/` (engine logic). No `androidTest/` tests at all.

**Assessment**: Compose UI testing (snapshot/interaction) requires `compose-ui-test` dependency and `androidTest` source set — a setup change beyond this review's scope. The G1–G3 changes are purely mechanical (delete dead code, string replacement, add defaulted parameter) with no behavioral risk, so new tests are not required for this phase.

**Existing tests**: Run `cd android && ./gradlew test` to verify no regressions from G1–G3 changes.

---

## Pre-existing issues (from Phase 1, not yet addressed)

- `HomeScreen.kt:62` — `externalLink` variable defined but never used (dead code)
- `SettingsMainActivity.kt:187` — `// Handle exception` comment in `openUrl` catch block does nothing
