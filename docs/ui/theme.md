# Theme & Styling

> **Type**: Feature
> **Keywords**: `Theme`, `Color`, `Style`, `Appearance`, `Font`
> **Related**: app-ui.md, device.md

---

## Summary

- Theme picker (v3.6.2, Theme tab): built-in theme families + up to 5 saved custom themes — see § Theme Picker below
- Keys keep the platform's standard subtle key shadow by default; a custom theme can set its own key-shadow intensity (0 = flat … 4)
- Font is a global keyboard setting (Settings tab), not part of a theme
- iOS: Light/Dark Mode; Android: Light/Dark/Auto Mode
- Platform-native color systems with custom overrides via `KeyboardColorSettings`, bundled per theme as `ThemeAppearance`

---

## Theme Picker (v3.6.2, current state)

Shipped in v3.6.2 (`changelog/mobile-v3.6.2.md`) as the Theme tab of the main app (tab order in [app-ui.md](app-ui.md)).

### Built-in catalog

Three key-style **families** over one shared set of 7 colours (`ios/Sources/TaigiKeyboard/Settings/BuiltInThemes.swift`, Android `ime/core/BuiltInThemes.kt`):

| Family | Key rendering |
|---|---|
| Filled | filled keys |
| Outlined | transparent keys + 1pt outline |
| Borderless | transparent keys, no outline |

Colours per family: Default (adaptive, follows light/dark), five light-only soft gradients (Sakura / Gold / Sea Breeze / Jade / Wisteria), and Catppuccin (Catppuccin Mocha, dark-only). The active theme paints every keyboard surface (keyboard, candidate strip, expanded candidate overlay, symbol / layout / settings overlays).

### Custom themes

`Create New…` on the custom shelf opens the editor (`App/Tabs/Theme/ThemeEditorView.swift` + `ThemeEditorViewModel.swift`; Android `ui/tabs/theme/ThemeEditorScreen.kt` hosted by `settings/ThemeEditorActivity.kt`). A custom theme captures one `ThemeAppearance` bundle (`Settings/KeyboardThemeModels.swift`, Android `ime/core/ThemeAppearance.kt`): the background surface + four role colours (`KeyboardColorSettings`), the five size scalars, and `keyShadowIntensity`. Up to `UserThemeStore.maxUserThemes = 5` (`Settings/UserThemeStore.swift`; Android `ime/core/UserThemeStore.kt` `MAX_USER_THEMES`). A live keyboard preview (`KeyboardPreviewPanel.swift` / `ThemePreviewEnvironment.swift`) is pinned in the editor.

**Editor order** (USER 2026-09-19, three sections = three surfaces; both platforms): **Background** — segmented Solid / Gradient / Photo, then the solid colour row, or the Start Color / End Color rows (no Direction row — the direction is the pointer on the pinned preview), or the Choose Photo / Change Photo picker row + a Fade slider · **Keys** — Key Fill · Key Text · Key Corner Radius · Key Border Width · Key Shadow · Keyboard Height · Key Font Size · **Candidate Bar** — Candidate Text · Candidate Text Size · Reset to Defaults · pinned preview. The candidate bar has no colour of its own: it is the same surface as the keyboard.

**Scheme-invariant user themes** (USER 2026-09-19). A new custom theme starts from `UserThemeSeed` (background `0xD4D5DD`, key text `0x000000`, key fill `0xFFFFFF`, candidate text `0x000000`; a CROSS-PLATFORM INVARIANT mirrored in Android `UserThemeSeed`), so every role is concrete and the theme renders identically in light and dark mode. `UserThemeStore.load()` (iOS) / `UserTheme.fromJson` (Android) fills any `nil` role of an older saved theme from the same seed (no migration write). **One key fill** (USER 2026-09-26): a user theme has a single Key Fill row that writes both `normalKeyFillColor` and `specialKeyFillColor` (iOS `keyFillColor`, Android `withKeyFill`); loading folds an older theme's separate special fill into its letter fill. Built-in themes keep distinct fills. Each colour row's reset arrow restores the seed value; Reset to Defaults restores the whole seed. The `default` buffer and built-in themes keep `nil` = adaptive.

### Background surface

`KeyboardColorSettings.background: ThemeBackground?` is the ONE field that paints the keyboard + candidate-bar surface (`Settings/KeyboardColorSettings.swift`; Android `ime/core/KeyboardColorSettings.kt` sealed `ThemeBackground`). `nil` = adaptive (KeyboardKit's dynamic background, Liquid Glass eligible; Android `?keyboard_bgColor` — the Filled Default head only).

| Case | JSON | Render (iOS) | Render (Android) |
|---|---|---|---|
| `.solid(color)` | `{"type":"solid","color":{r,g,b,a}}` (Android: `"color":argb`) | root `.background` = the colour; candidate bar `backgroundColor` = the same colour (`TaigiKeyboardView.candidateStyle`) | `KeyboardThemeSurfaceController` paints a `ColorDrawable` on `text_input_content`; smartbar chrome + Compose body transparent |
| `.gradient(ThemeGradient)` | `{"type":"gradient","stops":[…],"angle":180}` | `ThemeBackgroundSurface` paints a `LinearGradient` from `ThemeGradient.unitPoints`; candidate bar `.clear`; expanded overlay repaints the same gradient; panel backdrops pass a `KeyboardSurfaceSlice` so the gradient's points are remapped into panel space (`ThemeGradient.unitPoints(in:)`) |
| `.image(ThemeImageBackground)` | `{"type":"image","file":"<uuid>.jpg","dim":0.35}` | `ThemeBackgroundSurface` draws the photo in a `Canvas` (aspect-fill over the whole keyboard, `ThemeImageBackground.coverRect`, then the panel's slice), saturation ×0.7, then a tone overlay (white when the key text is dark, black otherwise) at `dim` (0…0.8). Photos live in the App Group `theme_images/<uuid>.jpg` (`ThemeImageStore`: downscaled to a 1280 px long edge, JPEG 0.85, backup-excluded; `SharedSettings.sweepThemeImages` removes files no saved theme references after every theme mutation); the extension decodes through `ThemeImageCache` (NSCache, 3 photos). Picked via `PhotosPicker` (no library permission). | a `PaintDrawable` shader built from the same unit points on `text_input_content`; Compose overlays / preview / card use `Modifier.themeBackground` → `ThemeGradient.brush` (unit points over the full keyboard, shifted by the panel's top inset) |

`ThemeGradient.angle` follows the CSS / Figma convention: `0` = bottom→top, `90` = left→right, `180` = top→bottom (`defaultAngle`, used by every built-in gradient theme), clockwise. `unitPoints` normalises the direction vector by its larger component so the diagonals (the 45° presets) run corner to corner. **Setting the angle** (both platforms; iOS `App/Tabs/Theme/GradientDirectionControl.swift`, Android `ui/tabs/theme/GradientDirectionControl.kt`): while the background kind is Gradient, `GradientDirectionOverlay` covers the editor's live preview: dragging a finger sets the angle to the direction from the preview's centre to the finger (`GradientDirectionDrag.angle`), snapping to the nearest 45° preset (`ThemeGradient.presetStep` / `PRESET_STEP`) within ±6° with a haptic tick, else a whole degree; an 8 pt/dp dead zone at the centre is ignored. The overlay draws the axis + arrowhead (white on a dark halo) and is the whole control — no Direction row (USER 2026-09-19: seeing the pointer is enough); for VoiceOver / TalkBack the overlay is one adjustable element (label Direction, value in degrees) stepping through the presets. The direction math (`direction(degrees:)` / `degrees(of:)`) lives on `ThemeGradient` and is shared with `unitPoints`. Decoding is legacy-compatible: an old `backgroundColor` becomes `.solid`, an old `backgroundGradient` (≥2 stops) becomes `.gradient` at `defaultAngle`, an old `candidateBackgroundColor` is ignored, and an unknown `type` degrades to `nil`. Encoding writes only `background`.

### Key shadow

Slider range 0…4 in 0.5 steps (`App/Tabs/Theme/ThemeControlRows.swift` `ThemeSliderRanges.shadow`). Three-state at render time (`KeyboardExtension/TaigiKeyboardView.swift` § "Per-theme key shadow"): `nil` (default and built-in themes) leaves KeyboardKit's standard button shadow; `0` renders flat (`.noShadow`); `> 0` renders a `Keyboard.ButtonShadowStyle` of that point size. Android mirrors this in `ime/text/keyboard/KeyContent.kt` (`keyShadowSpec(intensity)` → `null` for 0, a Compose drop shadow otherwise).

### Resolution and storage

| Piece | iOS (`Settings/`) | Android (`ime/core/`) |
|---|---|---|
| Theme id + selection | `KeyboardThemeModels.swift` `ThemeId` | `ThemeId.kt` |
| Built-in catalog | `BuiltInThemes.swift` | `BuiltInThemes.kt` |
| Resolver (selected id → `ThemeAppearance`) | `ThemeResolver.swift` | `ThemeResolver.kt` (+ `ThemeAppearanceCache.kt`) |
| User themes persistence | `UserThemeStore.swift` (UserDefaults, App Group) | `UserThemeStore.kt` (DataStore) |
| Picker UI | `App/Tabs/Theme/ThemePickerView.swift` | `ui/tabs/theme/ThemePickerScreen.kt` |

---

## Color System

### Default Colors (no user customization)

| Element | iOS | Android |
|---------|-----|---------|
| Keyboard background | `Color.keyboardBackground` (adaptive) | `keyboard_bgColor` theme attr |
| Normal key fill | `Color.keyboardButtonBackground` (adaptive) | `key_bgColor` theme attr |
| Special key fill | `Color.keyboardDarkButtonBackground` (adaptive) | `key_function_bgColor` theme attr |
| Key text | `Color.keyboardButtonForeground` (adaptive) | `key_fgColor` → `?android:textColor` |
| Candidate text | `Color(.label)` (system) | `smartbar_candidate_fgColor` theme attr |
| Candidate background | (keyboard background) | `smartbar_bgColor` theme attr (= `?keyboard_bgColor`) |

### Light/Dark Mode

- iOS: `@Environment(\.colorScheme)` + KeyboardKit adaptive colors
- Android: XML theme variants — `values/themes.xml` (light) / `values-night/themes.xml` (dark)
- Custom user colors are **single RGBA/ARGB values** — not mode-aware (same color in both modes); since 2026-09-19 a user theme carries no `nil` role at all (seeded), so it never follows the scheme

### Color Fallback Chain

1. User custom color (from `KeyboardColorSettings`) → use it
2. Else: platform adaptive color (respects light/dark automatically)

---

## User Customization

### Customizable Properties (per custom theme; `ThemeAppearance`)

| Property | Range | Default |
|----------|-------|---------|
| Key height scale | 0.85–1.15 | 1.0 |
| Key font size scale | 0.85–1.15 | 1.0 |
| Candidate text size scale | 0.85–1.15 | 1.0 |
| Key corner radius | 0–15 pt/dp | 6 |
| Key border width | 0–3 pt/dp | 0 |
| Key shadow intensity | 0–4 pt/dp | 0 (flat) for a new custom theme; built-in themes keep the platform standard shadow |
| Background (keyboard + candidate bar) | `ThemeBackground`: solid RGBA, or a 2-stop gradient + angle | seed `0xD4D5DD` solid (user theme); adaptive (`default` buffer) |
| Key text color | RGBA | seed `0x000000` (user theme); adaptive (`default` buffer) |
| Normal key fill color | RGBA | seed `0xFFFFFF` (user theme); adaptive (`default` buffer) |
| Special key fill color | RGBA | = normal key fill (user theme); adaptive (`default` buffer) |
| Candidate text color | RGBA | seed `0x000000` (user theme); adaptive (`default` buffer) |

### Font Options (global setting — Settings tab, not per theme)
- System default
- jf-openhuninn-2.1
- Iansui-Regular

### Persistence
- iOS: `SharedSettings` → UserDefaults (App Group), `KeyboardColorSettings` as JSON
- Android: `PrefHelper` → DataStore, `KeyboardColorSettings` as JSON

### Real-time Preview
Both platforms render an actual keyboard view in appearance settings for instant feedback.

---

## Button Styling Providers (iOS)

| Provider | Responsibility |
|----------|---------------|
| `ButtonFontProvider` | Resolves font + scale per action/layout (e.g., MOE2 3-char keys at 75%) |
| `ButtonTextProvider` | Button labels, tone hints, MOE punctuation hints |
| `ButtonImageProvider` | SF Symbols images for special keys |

Applied via KeyboardKit's `keyboardButtonStyle { }` closure (per-action customization).

---

## Liquid Glass (iOS 26+)

When `KeyboardContext.isLiquidGlassEnabled == true` AND `background == nil` (adaptive):
- Keyboard background: `Color.white.opacity(0.001)` (transparent pass-through)
- Candidate items: `.opacity(0.6)` when pressed/selected

When a custom background is set (solid or gradient): paints it, ignores Liquid Glass.

---

## App UI Card Style

| Property | Value | Description |
|----------|-------|-------------|
| cornerRadius | 10dp/pt | Retro square |
| elevation | 0 | No shadow |
| strokeWidth | 1dp/pt | Thin border |
| background | surfaceSecondary | Cream white |

### Left Color Bar Design

- Width: 4dp
- Gray-green: Primary features
- Deep red: Important features
- Pink: Warm accent

---

## Platform Correspondence

| Function | iOS | Android |
|----------|-----|---------|
| Color definitions | SwiftUI `Color` + KeyboardKit adaptive | `colors.xml` + `themes.xml` (light/dark) |
| Custom colors | `SharedSettings.colorSettings` | `PrefHelper` → `KeyboardColorSettings` |
| Button styling | `Styling/Providers/` (`ButtonFont/Text/ImageProvider`) | Theme attributes + runtime overrides |
| Font management | `FontRegistration` (enum — `CTFontManagerRegisterFontsForURL`) | `TypefaceLoader` + `PrefHelper` |
| Appearance settings UI | `App/Tabs/Theme/ThemeTab.swift` (SwiftUI) | `ui/tabs/theme/ThemePickerScreen.kt` (Compose) |
| Settings sync | `UserDefaults.didChangeNotification` | DataStore Flow observation |

---

## Design Philosophy

### Avoid Modern Style

- No heavy shadows (keys keep the platform's subtle default; custom themes cap at 4pt)
- No floating card effects
- No overly rounded corners
- No uniform icon circles

### Use Retro Elements

- Flat design + thin borders
- Left color bar (file folder style)
- Dashed separators
- Left-aligned titles
