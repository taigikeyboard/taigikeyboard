# Android Tab1 + MainSettingsScreen Refactoring Plan

**Branch**: `refactor-android-tabs`
**Scope**: `ui/tabs/tab1/` (HomeScreen, DetailScreen, CopyrightScreen, SetupGuideScreen) + `ui/tabs/MainSettingsScreen.kt`
**Goal**: Remove dead code, fix inconsistencies, reduce duplication — no behavior changes.

**Reviewed by**: Initial review + Simplify skill (3 parallel agents: reuse, quality, efficiency) + Codex second opinion

---

## Stage 1: Dead Code Cleanup
**Goal**: Remove unused variables, imports
**Status**: Not Started

### Changes

| File | Line | Issue | Action |
|------|------|-------|--------|
| `HomeScreen.kt` | 62 | Unused variable `externalLink` | Remove declaration |
| `HomeScreen.kt` | 29 | Unused import `Color` | Remove |
| `HomeScreen.kt` | 34 | Unused import `sp` | Remove |
| `HomeScreen.kt` | 40 | Unused import `OpenInNew` (only needed for removed `externalLink`) | Remove |
| `CopyrightScreen.kt` | 34 | Unused import `sp` | Remove |
| `SetupGuideScreen.kt` | 5 | Unused import `isSystemInDarkTheme` | Remove |

### Success Criteria
- [ ] No unused imports or variables in tab1 files
- [ ] `./gradlew assembleDebug` compiles (user verifies)

---

## Stage 2: Minor Inconsistencies
**Goal**: Fix comment numbering, icon size, and lastIndex usage
**Status**: Not Started

### Changes

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `HomeScreen.kt` | 174 | Comment says "Section 3" (duplicate) | Change to `// Section 4: Resources & links` |
| `HomeScreen.kt` | 251 | Comment says "Section 4" (should be 5) | Change to `// Section 5: FAQ` |
| `DetailScreen.kt` | 381 | Hardcoded `Modifier.size(12.dp)` for OpenInNew | Use `Modifier.size(AppStyle.smallIconSize)` to match CopyrightScreen:193 (**visual change**: 12dp → 16dp, verify with screenshot) |
| `CopyrightScreen.kt` | 157 | `page.buttons.size - 1` inconsistent with rest of codebase | Change to `page.buttons.lastIndex` |

### Success Criteria
- [ ] Section comments numbered 1-5 sequentially
- [ ] All OpenInNew trailing icons use `AppStyle.smallIconSize`
- [ ] All last-index checks use `.lastIndex` consistently

---

## Stage 3: Extract `resolveDrawableResId` Utility
**Goal**: Eliminate repeated `getIdentifier` + fallback pattern (6 call sites)
**Status**: Not Started

### Problem
`context.resources.getIdentifier(name, "drawable", context.packageName)` with fallback is repeated 6 times:
- `HomeScreen.kt` lines 123-128, 152-157, 259-264
- `DetailScreen.kt` lines 485, 494, 516

### Changes
Add a private utility (in a shared location or locally):

```kotlin
@DrawableRes
fun resolveDrawableResId(
    context: Context,
    androidIconName: String,
    @DrawableRes fallback: Int,
): Int {
    val resId = context.resources.getIdentifier(androidIconName, "drawable", context.packageName)
    return if (resId != 0) resId else fallback
}
```

Replace all 6 call sites.

### Success Criteria
- [ ] Single function for drawable resolution with fallback
- [ ] No raw `getIdentifier` calls in tab1 UI files

---

## Stage 4: Extract Duplicated Feature List Rendering
**Goal**: Eliminate near-identical feature list code in HomeScreen
**Status**: Not Started

### Problem
Lines 121-142 (typingGuideFeatures) and 150-170 (settingsFeatures) are ~95% identical. The only difference: `iconTint = featureIconTint` on the first block.

### Changes
Extract a private composable in `HomeScreen.kt`:

```kotlin
@Composable
private fun FeatureList(
    features: List<FeatureContent>,
    context: android.content.Context,
    languageManager: LanguageManager,
    chevronRight: ImageVector,
    onFeatureClick: (String, String, Array<String>) -> Unit,
    iconTint: Color = MaterialTheme.colorScheme.primary,
) {
    features.forEachIndexed { index, feature ->
        val iconResId = resolveDrawableResId(context, feature.icon.android, R.drawable.lightbulb_24)
        NavigationRow(
            icon = painterResource(iconResId),
            label = languageManager.text(feature.title),
            trailingIcon = chevronRight,
            iconTint = iconTint,
            onClick = { onFeatureClick(feature.id, "feature", arrayOf(feature.id)) },
        )
        if (index < features.lastIndex) {
            SettingsDivider(Modifier.padding(horizontal = 16.dp))
        }
    }
}
```

Replace both sections with calls to `FeatureList(...)`.

### Success Criteria
- [ ] No duplicated feature rendering code
- [ ] Both sections render identically to before

---

## Stage 5: Merge Near-Duplicate Link Cards in DetailScreen
**Goal**: Merge `NavigationLinkCard` and `ExternalLinkCard` into one composable
**Status**: Not Started

### Problem
Both composables (lines 307-386) share ~90% identical code, differing only in trailing icon type and size.

### Changes
Replace both with a single private composable:

```kotlin
@Composable
private fun LinkCard(
    text: String,
    @DrawableRes iconResId: Int,
    trailingIcon: ImageVector,
    trailingIconSize: Dp = AppStyle.trailingChevronSize,
    fontFamily: FontFamily,
    onClick: () -> Unit,
) { ... }
```

Update call sites in `DetailItemContent`:
- `NavigationLink` → `LinkCard(trailingIcon = KeyboardArrowRight)`
- `ExternalLink` → `LinkCard(trailingIcon = OpenInNew, trailingIconSize = AppStyle.smallIconSize)`

### Success Criteria
- [ ] Single composable replaces both NavigationLinkCard and ExternalLinkCard
- [ ] Detail screen renders identically to before

---

## Stage 6: Stringly-Typed Content Types → Constants
**Goal**: Replace raw string content types with constants
**Status**: Not Started

### Problem
Content type strings `"feature"`, `"faq"`, `"feedback"`, `"version"` are used as raw strings in:
- `HomeScreen.kt` lines 135, 163, 270
- `DetailScreen.kt` lines 114-115, 449-453
- `SettingsMainActivity.kt` lines 104, 113, 119, 122

### Changes
Define constants:

```kotlin
object ContentType {
    const val FEATURE = "feature"
    const val FAQ = "faq"
    const val FEEDBACK = "feedback"
    const val VERSION = "version"
}
```

Replace all raw strings with `ContentType.*`.

### Success Criteria
- [ ] No raw content type strings in UI files
- [ ] Same navigation behavior as before

---

## Stage 7: SlideshowCard Index Reset Fix
**Goal**: Fix latent bug where slideshow index can be out of bounds
**Status**: Not Started

### Problem
`DetailScreen.kt` `SlideshowCard` (line 285): `currentIndex` is not reset when `imageResIds` changes. If the new list is shorter than the old index, it causes an IndexOutOfBoundsException before the `LaunchedEffect` re-fires.

### Changes

**Fix 1**: Reset index when images change:
```kotlin
// Before:
var currentIndex by remember { mutableIntStateOf(0) }

// After:
var currentIndex by remember(imageResIds) { mutableIntStateOf(0) }
```

**Fix 2** (from Codex): Add `intervalMs` to `LaunchedEffect` key so timing updates if interval changes:
```kotlin
// Before:
LaunchedEffect(imageResIds) { ... }

// After:
LaunchedEffect(imageResIds, intervalMs) { ... }
```

### Success Criteria
- [ ] Index resets to 0 when slideshow images change
- [ ] No risk of out-of-bounds access
- [ ] Interval changes take effect immediately

---

## Stage 8: FeatureContentLoader Thread Safety
**Goal**: Make singleton cache thread-safe
**Status**: Not Started

### Problem
`FeatureContentLoader.kt` lines 13-14: `cachedFeatures` and `cachedFAQs` are plain `var` fields with no synchronization. Concurrent calls can parse the JSON twice (race condition on cache check).

### Changes
Add `@Volatile` to both fields:

```kotlin
@Volatile private var cachedFeatures: List<FeatureContent>? = null
@Volatile private var cachedFAQs: List<FeatureContent>? = null
```

This is sufficient because: (1) List assignment is atomic on JVM, (2) worst case is a harmless double-parse, and (3) `@Volatile` ensures visibility across threads.

### Note (from Codex)
The first load is triggered from composition (`HomeScreen`, `DetailScreen`), meaning asset I/O + JSON parsing runs on the main thread. After warm cache the cost is negligible, but the initial render can jank. A future improvement would be to move the initial load to a ViewModel with `Dispatchers.IO`. Out of scope for this refactor.

### Success Criteria
- [ ] No race condition on cache reads
- [ ] Same behavior — just thread-safe

---

## Out of Scope (noted for future)

| Issue | Why deferred |
|-------|-------------|
| Magic number `features.take(6)` in HomeScreen | Requires JSON schema change (add `category` field) |
| `fontFamily` prop drilling → CompositionLocal | Large cross-cutting change, affects many screens |
| Tab state preservation (NavHost/rememberSaveable) | Architectural change beyond tab1 scope |
| Hardcoded `contentDescription = "Back"` | Cross-cutting concern for all screens, not tab1-specific |
| No UI tests for tab1 | Test coverage improvement is a separate effort |
| `buildGenericItems` imperative → `mapNotNull` | Minor style preference, low impact |
| `initialTab` not resynced in MainSettingsScreen | Only set from intent extra, won't change during lifecycle |
| FeatureContentLoader first load on main thread | Needs ViewModel + `Dispatchers.IO`, architectural change |

---

## Execution Order

```
Stage 1 (dead code)  ──┐
Stage 2 (minor fixes) ─┤── independent, safe to commit individually
Stage 3 (utility)     ─┘
         │
Stage 4 (feature list) ── depends on Stage 3 (uses resolveDrawableResId)
Stage 5 (link cards)   ── depends on Stage 2 (uses AppStyle.smallIconSize)
Stage 6 (constants)    ── independent
Stage 7 (slideshow)    ── independent
Stage 8 (thread safety)── independent
```

Stages 1-3, 6-8 are leaf changes (no risk). Stages 4-5 are structural but low-risk (private composable extraction, no public API change). All stages are safe to commit individually.
