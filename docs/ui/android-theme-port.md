# Android Theme-Picker Port (v3.6.2)

> **Type**: Planning (forward-looking, multi-PR)
> **Keywords**: `theme`, `android`, `port`, `gradient`, `cross-platform`
> **Status**: Complete — all phases merged (P5 = `0957cbb9`, PR #418, cleanup + `SettingNavigationRow` extraction). ⚠ device dogfood pending.
> **iOS source**: chain #400-411 (main `786c8366`); spec in memory `project_v362_theme_picker.md` + `docs/ui/theme.md` / `theme-presets-brainstorm.md`

---

## Goal

Port the iOS v3.6.2 keyboard-theme-picker feature to Android, mirroring the iOS final state. iOS reorganized appearance editing into a Theme system: a Theme tab gallery (built-in + custom themes), built-in gradient themes, a custom-theme CRUD editor, and a keyboard render seam (gradient background + transparent candidate bar + key shadow).

## Grounding correction (verified in code, not from the roadmap text)

Android is **not** a from-scratch build. It already has, mirroring iOS's *pre-theme* state:

- `ime/core/KeyboardColorSettings.kt` — 6 nullable colors (header: `Matches iOS KeyboardColorSettings structure`)
- All 5 size scalars (key height / font / candidate / corner / border) in DataStore + consumed in render
- Reusable UI: `ColorRow`, `SliderRow`, `ColorPickerDialog`, `FontPickerContent`, `KeyboardPreviewPanel`
- The DataStore live-read seam (`KeyboardAppearanceResolver.snapshot()`) that already propagates color/size edits to the running keyboard

So the port is **additive on solid existing infra**. Genuinely new: the theme concept (`ThemeAppearance` / `ThemeId` / `BuiltInThemes` / `UserTheme` / `ThemeResolver`), `ThemeGradient` + gradient render, `keyShadowIntensity` + shadow render, transparent-candidate-over-gradient, Theme tab + gallery + CRUD editor, built-in preview drawables.

## User decisions (2026-06-08)

1. **Appearance page → full iOS parity (remove).** Remove `AppearanceSettingsScreen` / `AppearanceSettingsActivity` + the Layout-tab entry. Layout tab = layout selection only. Global font picker moves to the Settings tab. Color/size editing happens only via the new theme editor. The `經典` (default) theme = the preserved global `colorSettings` + scalars (adaptive for fresh users, kept for customized users, no dedicated edit page).
2. **Built-in catalog → full iOS mirror (include scaffold).** `經典` (default + 海風/翠青/藤紫 gradients) + Swifty (4 scaffold) + Minimal (4 scaffold).
3. **Custom themes = single appearance, no dark/light split** (built-in themes keep light+dark variants).

## Codex pre-impl decisions (folded into the plan)

- **Gradient continuity** → paint the vertical gradient ONCE on the common parent (`InputView` container); keep both the Smartbar and Keyboard Compose hosts transparent. Avoids per-host coordinate alignment + handles dynamic smartbar height. Risk: confirm the parent span is exactly candidate-top→keyboard-bottom.
- **Key shadow** → Compose `dropShadow()` (NOT `Modifier.shadow`, which is Material elevation). Map intensity `0 = none`, `1..4 = radius intensity.dp, offsetY intensity/2.dp, alpha 0.30`. Fallback `Modifier.shadow(elevation = intensity*1.5.dp, shape, clip=false)` if the Compose BOM lacks `dropShadow()`. Device-screenshot verify (clipping risk).
- **Persistence** → DataStore JSON string keys (matches existing `colorSettings`); write `userThemes` + `selectedThemeId` in one transaction.
- **No `themeRevision` counter** (intentional divergence from iOS). DataStore write does not auto-trigger `publishAppearance()`. The appearance-republish Flow (observing `selectedThemeId + userThemes + colorSettings + scalars` so a *visible* keyboard updates without a keystroke; observing `selectedThemeId` alone is insufficient since editing the active theme keeps the id) was **deferred from P2 to P3/P4** — P2 is dormant (themes unselectable), and `onWindowShown → refreshTheme()` already covers the settings-Activity-return case. Add the Flow when an in-keyboard or live theme-change surface lands.
- **Resolver threading** preserves existing customized users: `selectedThemeId == "default"` → `legacyAppearance` (current `colorSettings` + 5 scalars + shadow 0). `fontType` stays global, not a theme field. Smartbar must read the same resolved appearance, not raw `prefs.colorSettings`.
- **Model shape** → mirror semantics, not Swift types. `data class` + `BuiltInThemes` object; `ThemeId` = string constants; `UserThemeStore` = plain JSON parse/write (no repository abstraction).

## Documented divergences (vs iOS)

| Divergence | Reason |
|---|---|
| No `themeRevision` counter | Android live-reads per `snapshot()`; idle refresh handled by a republish Flow instead. |
| Gradient painted on the parent container (not one `.background`) | Android keyboard body + smartbar are separate Compose hosts. |
| `userThemes` in DataStore JSON (not a standalone file) | Android is already `allowBackup="false"` app-wide; iOS used a file only for per-file backup exclusion. |
| User-created themes preserved across a full settings-reset | Treated as user content (like the SQLite user DBs), not wiped on reset. |

## Phase table (one PR per phase)

| Phase | Scope | Status |
|---|---|---|
| P0 | Roadmap doc + memory (admin) | Merged (folded into `ad49504b`) |
| P1 | Model + persistence + resolver + unit tests (no UI, no render change) | Merged `ad49504b` (PR #414, 40 tests green) |
| P2 | Render seam: gradient bg + transparent candidate + key shadow (republish Flow deferred to P3/P4) | Merged `aa11ebf1` (PR #415) |
| P3 | Theme tab (index 1) + built-in shelves + `ThemeTexts` strings | Merged `ff0c13a9` (PR #416) |
| P4 | Custom-theme shelf (Create New + per-card menu + `CustomThemeButtonPreview` + delete/orphan-guard) + theme editor CRUD (Save-time name dialog, cap 5, auto-apply, bottom preview) | Merged `4325af61` (PR #417) |
| P5 | Cleanup: remove appearance page, font → Settings tab, string migration, `SettingNavigationRow` extraction | Merged `0957cbb9` (PR #418) |

## Per-phase file inventory

### P1 — Model + persistence + resolver

New (`ime/core/`, next to `KeyboardColorSettings.kt`):
- `ThemeGradient` (data class, `stops: List<Int>` ARGB, top→bottom) — co-located with `KeyboardColorSettings`
- `ThemeAppearance.kt` (6 colors + `keyShadowIntensity` + 5 scalars; `DEFAULT`; forward-compatible JSON decode)
- `ThemeId.kt` (`DEFAULT = "default"`, `isUserTheme(id)`)
- `BuiltInThemes.kt` (`BuiltInTheme` + `BuiltInThemeFamily` + `BuiltInThemes` object; 經典 + Swifty + Minimal; exact iOS hex)
- `UserTheme.kt` (id String / name / appearance / createdAt+updatedAt epoch ms; JSON list codec)
- `UserThemeStore.kt` (pure: `read`/`write` lambdas injected, cap 5, add/update/delete)
- `ThemeResolver.kt` (pure mapping)

Modified:
- `KeyboardColorSettings.kt` — `+ backgroundGradient: ThemeGradient?` `+ hasBackgroundGradient` + gradient JSON
- `PreferenceDataStore.kt` — `SELECTED_THEME_ID` + `USER_THEMES` keys
- `PrefHelper.kt` — `selectedThemeId` + `userThemes` vars + `legacyAppearance` + `resolvedAppearance(isDark)` + `loadUserThemes()`; preserve `USER_THEMES` / reset `SELECTED_THEME_ID` in `resetToDefaults`
- `app/build.gradle.kts` — `testImplementation("org.json:json:...")` (unit tests stub `org.json` via `returnDefaultValues`)

Tests (`app/src/test/.../ime/core/`): `ThemeResolverTest`, `BuiltInThemesTest`, `UserThemeStoreTest`, `KeyboardColorSettingsGradientTest`, `ThemeAppearanceJsonTest` — mirror the iOS XCTest suites.

### P2 — Render seam (merged `aa11ebf1`, PR #415)
- `ThemeAppearanceCache` (new, `ime/core`) — string-eq cache over `ThemeResolver` inputs; delegates `PrefHelper.resolvedAppearance`. Held by `KeyboardAppearanceResolver` (keys) + `SmartbarManager` (candidates) so both read the SAME resolved theme. `+isKeyboardNightMode` shared helper (`NavigationBarManager.isDarkMode` delegates).
- `KeyboardAppearanceResolver.snapshot()` resolves `ThemeAppearance`; maps scalars + new `keyShadowIntensity` into `KeyboardAppearance`. `+resolvedColors()` for the View-layer apply.
- `KeyboardLayout` bg transparent when `hasBackgroundGradient`; `KeyContent` `dropShadow(shape)` before background, pure `keyShadowSpec(i)` map (`0=none, 1..4 → radius i / offsetY i/2 / alpha .30`).
- `KeyboardThemeSurfaceController` (new) — paints `GradientDrawable(TOP_BOTTOM)` on `text_input_content`; `SmartbarView.applyThemeSurface` toggles chrome transparency. Candidate-bg ownership consolidated here (removed per-update `applyCustomBackgroundColor`); `currentDisplay()` forces the candidate strip transparent over a gradient explicitly.
- `TextInputManager.refreshTheme()` (= `pushAppearance` + surface apply) called from `onWindowShown`.
- Deferred to P3 device dogfood: gradient seam screenshot (no selection UI in P2) + intensity-4 bottom-edge shadow clip (~1dp, cosmetic). Republish Flow deferred to P3/P4 (see Codex decisions).

### P3 — Theme tab + built-in shelves (PR #416)

**Scope refinement**: the custom-theme shelf (Create New + per-card menu + `CustomThemeButtonPreview` + delete/orphan-guard) moved P3→P4. P3 had no theme-creation path (editor is P4), so the custom shelf would be unexercisable this round; P4 owns all custom-theme UI as one coherent unit. USER-confirmed (2026-06-08, option A).

- insert tab at index 1 (`SettingsMainActivity` / `MainSettingsScreen`; renumber `TAB_*`); `initialTab.coerceIn(...)` guards the exported Activity against a stale/external `EXTRA_START_TAB`.
- `ui/tabs/theme/ThemePickerScreen.kt` — one horizontal shelf per `BuiltInThemes.families` (經典 / Swifty / Minimal); `ThemeCard` reuses the `LayoutCard` selection overlay; cards **200.dp** (matches Android `LayoutCard`, intentional divergence from iOS 240pt); built-in screenshots placeholder until supplied (explicit `R.drawable` map later, never `getIdentifier`).
- apply → `prefs.selectedThemeId` (tap-write guarded); reactive via new `PrefHelper.observeSelectedThemeId()` + `collectAsStateWithLifecycle`.
- tab icon = existing `R.drawable.ic_palette` (no new drawable); `strings.xml` `tab_theme`.
- `localization/ThemeTexts.kt` — `tabTitle` only.
- Built-in gradient selection wakes the dormant P2 render seam → P2 gradient/shadow device dogfood now possible.

### P4 — Custom shelf + editor CRUD (PR #417, in review)
- custom-theme shelf in `ThemePickerScreen`: `CreateNewThemeCard` (hidden at cap 5) + user-theme cards (`CustomThemeButtonPreview` single styled key; nil roles → KeyboardTheme adaptive attrs) + per-card apply/edit/delete overflow menu + delete orphan-guard (`selectionAfterDelete`, unit-tested). Reactive via new `PrefHelper.observeUserThemes()` (distinctUntilChanged on raw JSON before decode). `ThemeCard` generalized: `preview` slot + optional menu; tap-to-apply on preview only.
- `ThemeEditorScreen` + `ThemeEditorActivity` (full-screen Activity, mirrors `AppearanceSettingsActivity` — NOT intra-tab nav-child, so the pinned preview is not squeezed by the bottom tab bar): draft `ThemeAppearance` (6 colors + 5 size sliders + key-shadow slider + reset) in `rememberSaveable` (survives rotation); Save-time name dialog (no inline field); cap pre-check; `onSave` returns Boolean so a cap TOCTOU race re-shows the dialog; edit loads latest by id and a missing target is a no-op finish (never create); save strips `backgroundGradient` (custom themes flat).
- `KeyboardPreviewPanel` +`keyShadowIntensity` param (default flat; Layout-tab caller unaffected). **Gradient deliberately NOT added to the editor preview** — custom themes never carry a gradient (no editor control; new = DEFAULT; user themes are flat), so the draft's `backgroundGradient` is always null and `KeyboardLayout` renders flat correctly. Divergence from the original "extend for gradient/shadow" plan note: shadow only.
- reuse `ColorRow` / `SliderRow` / `ColorPickerDialog` / `KeyboardPreviewPanel`. `ColorSettingRow` + `ColorPickerTarget` temporarily duplicate the `AppearanceSettingsScreen` private copies (cross-package private; consolidated to `ui/components` in P5 when that screen is deleted — tracked with a `// P5:` note).

### P5 — Cleanup (last phase; NEXT round)

Removes the now-superseded appearance editor and consolidates the P4-deferred dups. Pure refactor — behavior-preserving (the theme editor already owns color/size editing; font is the only feature that relocates).

- **Remove the appearance editor**: delete `ui/tabs/layout/AppearanceSettingsScreen.kt` + `settings/AppearanceSettingsActivity.kt` + its manifest `<activity>` entry. In `SettingsMainActivity`, drop the `onAppearanceSettings` lambda passed to `LayoutScreen` (the Intent launch). In `LayoutScreen.kt`, remove the appearance `ActionRow` entry → Layout tab = layout selection only.
- **Font picker → Settings tab**: `FontPickerContent.kt` currently lives in `ui/tabs/layout` and is shown from `AppearanceSettingsScreen`. Move the font-setting entry point into `ui/tabs/settings/InputSettingsScreen.kt` (the Settings tab). Font stays a global setting (`PrefHelper.fontType`); not a theme field.
- **String migration**: appearance labels duplicated across `LayoutTexts` (used only by the deleted `AppearanceSettingsScreen`) and `ThemeTexts` (P4) — delete the now-orphaned `LayoutTexts` copies (keyHeight / keyFontSize / candidateTextSize / keyCornerRadius / keyBorderWidth / 6 color labels / section headers / appearanceResetAll / customFont / appearanceSettings) once their only caller is gone. `ThemeTexts` is the survivor.
- **Consolidate P4-deferred dup**: `ThemeEditorScreen.kt` has `private fun ColorSettingRow` + `private data class ColorPickerTarget` that byte-for-byte duplicate the copies in `AppearanceSettingsScreen.kt` (kept separate in P4 to avoid a cross-package hoist mid-migration; marked with a `// P5:` note). When `AppearanceSettingsScreen` is deleted, hoist the single surviving pair into `ui/components/` (next to `ColorRow`/`SliderRow`) as `internal`, or keep them private in `ThemeEditorScreen` if it becomes the only consumer. Grep `// P5:` for the tracked site.
- **Verify**: `cd android && ./gradlew :app:assembleDebug` + `:app:testDebugUnitTest`. Engine untouched → no `make build`. Grep for any remaining `AppearanceSettings` / `LayoutTexts.appearance*` references before declaring done. Refactor-freeze: behavior-preserving, no new `INVARIANT_*`.

**Realized (PR #418 `0957cbb9`)**: plan items ①–④ as written. Two adjustments from review:
- **`customFont` ownership**: the plan listed `customFont` among the deleted `LayoutTexts` orphans, but `FontPickerContent` still consumes it — so `customFont` was *moved* to `ThemeTexts` (mirrors iOS `ThemeTexts.swift:22`), not deleted. Font *name* strings stay in `LayoutTexts`. Font row hosted as a `showFontPicker` sub-page inside `InputSettingsScreen` (mirrors the existing `showInputModePicker` pattern) with a `BackHandler` (Codex post-impl caught its absence — without it, system back from the picker finishes `SettingsMainActivity`). `Activity.recreate()` dropped (no embedded preview in the Settings tab).
- **+ `SettingNavigationRow` extraction** (beyond plan, Core Principle #6 direction-first / ui-style-guide Rule 7): the input-mode + font "label + value + chevron" rows were hand-rolled twice → extracted a shared `ui/components/SettingNavigationRow`; both call it. `ui-style-guide.md` reusable-components list updated. Net −421 LOC.
- Item ④: `ColorSettingRow` / `ColorPickerTarget` kept private in `ThemeEditorScreen` (sole consumer after the appearance page deletion) — not hoisted (YAGNI).
- ⚠ Device dogfood pending: font row shows/opens picker → keyboard re-renders next open; system back from picker returns to the Settings tab (not app exit); Layout tab = layout-only; theme editor color/size unchanged.

## Built-in gradient hex (from `ios/Sources/TaigiKeyboard/Settings/BuiltInThemes.swift`)

| id | name | light top / bottom | dark top / bottom |
|---|---|---|---|
| `standardBlue` | 海風 | `BFD2EA` / `DCE2EC` | `323E58` / `262E40` |
| `standardGreen` | 翠青 | `C3D8C8` / `DCE5DD` | `324235` / `28342A` |
| `standardPurple` | 藤紫 | `CDC4E4` / `DEDAEA` | `3A3252` / `2C2640` |

Neutral keys (function keys forced same fill as letter keys → white in light, soft dark in dark): light fill `FFFFFF` / text `1C1C1E`; dark fill `3A3A3C` / text `FFFFFF`. `backgroundColor` + `candidateBackgroundColor` stay null (the gradient owns the background, the candidate bar is transparent over it). Hex are visual estimates from KeyboardKit reference shots — fine-tune on device.

## Gates

Each phase PR: `cd android && ./gradlew :app:assembleDebug` + `:app:testDebugUnitTest` (background, post-`gh pr create`). Engine untouched → no `make build` / `make dict`.

## Dependencies on USER

- Built-in preview drawables (`theme_standardBlue_preview` / `theme_standardGreen_preview` / `theme_standardPurple_preview`, 585×369) — placeholder until supplied (P3).
- Device dogfood per phase (Android Studio build; no `make build`).
