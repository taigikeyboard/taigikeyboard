# Theme & Styling

> **Type**: Feature
> **Keywords**: `Theme`, `Color`, `Style`, `Appearance`, `Font`
> **Related**: app-ui.md, device.md

---

## Summary

- Theme picker (v3.6.2, 主題 tab): built-in theme families + up to 5 saved custom themes — see § Theme Picker below
- Keys keep the platform's standard subtle key shadow by default; a custom theme can set its own key-shadow intensity (0 = flat … 4)
- Font is a global keyboard setting (Settings tab), not part of a theme
- iOS: Light/Dark Mode; Android: Light/Dark/Auto Mode
- Platform-native color systems with custom overrides via `KeyboardColorSettings`, bundled per theme as `ThemeAppearance`

---

## Theme Picker (v3.6.2, current state)

Shipped in v3.6.2 (`changelog/v3.6.2.md`) as the 主題 tab of the main app (tab order in [app-ui.md](app-ui.md)).

### Built-in catalog

Three key-style **families** over one shared set of 7 colours (`ios/Sources/TaigiKeyboard/Settings/BuiltInThemes.swift`, Android `ime/core/BuiltInThemes.kt`):

| Family | Key rendering |
|---|---|
| 經典 (classic) | filled keys |
| 框線 (framed) | transparent keys + 1pt outline |
| 簡潔 (clean) | transparent keys, no outline |

Colours per family: 預設 (adaptive, follows light/dark), five light-only soft gradients (櫻花 / 金煌 / 海風 / 翠青 / 藤紫), and 暗眠山貓 (Catppuccin Mocha, dark-only). The active theme paints every keyboard surface (keyboard, candidate strip, expanded candidate overlay, symbol / layout / settings overlays).

### Custom themes

`Create New…` on the custom shelf opens the editor (`App/Tabs/Theme/ThemeEditorView.swift` + `ThemeEditorViewModel.swift`; Android `ui/tabs/theme/ThemeEditorScreen.kt` hosted by `settings/ThemeEditorActivity.kt`). A custom theme captures one `ThemeAppearance` bundle (`Settings/KeyboardThemeModels.swift`, Android `ime/core/ThemeAppearance.kt`): the six role colours (`KeyboardColorSettings`, `nil` = inherit adaptive), the five size scalars, and `keyShadowIntensity`. Up to `UserThemeStore.maxUserThemes = 5` (`Settings/UserThemeStore.swift`; Android `ime/core/UserThemeStore.kt` `MAX_USER_THEMES`). A live keyboard preview (`KeyboardPreviewPanel.swift` / `ThemePreviewEnvironment.swift`) is pinned in the editor.

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
| Candidate background | (keyboard background) | `smartbar_bgColor` theme attr |

### Light/Dark Mode

- iOS: `@Environment(\.colorScheme)` + KeyboardKit adaptive colors
- Android: XML theme variants — `values/themes.xml` (light) / `values-night/themes.xml` (dark)
- Custom user colors are **single RGBA/ARGB values** — not mode-aware (same color in both modes)

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
| Keyboard background color | RGBA | (adaptive) |
| Key text color | RGBA | (adaptive) |
| Normal key fill color | RGBA | (adaptive) |
| Special key fill color | RGBA | (adaptive) |
| Candidate text color | RGBA | (adaptive) |
| Candidate background color | RGBA | (adaptive) |

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

When `KeyboardContext.isLiquidGlassEnabled == true` AND no custom background:
- Keyboard background: `Color.white.opacity(0.001)` (transparent pass-through)
- Candidate items: `.opacity(0.6)` when pressed/selected

When custom background is set: uses that color, ignores Liquid Glass.

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
