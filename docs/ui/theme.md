# Theme & Styling

> **Type**: Feature
> **Keywords**: `Theme`, `Color`, `Style`, `Appearance`, `Font`
> **Related**: app-ui.md, device.md

---

## Summary

- Flat design: no shadows, uses borders
- User-customizable colors, fonts, and key sizing
- iOS: Light/Dark Mode; Android: Light/Dark/Auto Mode
- Platform-native color systems with custom overrides via `KeyboardColorSettings`

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

### Customizable Properties

| Property | Range | Default |
|----------|-------|---------|
| Key height scale | 0.85–1.15 | 1.0 |
| Key font size scale | 0.85–1.15 | 1.0 |
| Candidate text size scale | 0.85–1.15 | 1.0 |
| Key corner radius | 0–15 pt/dp | 6 |
| Key border width | 0–3 pt/dp | 0 |
| Keyboard background color | RGBA | (adaptive) |
| Key text color | RGBA | (adaptive) |
| Normal key fill color | RGBA | (adaptive) |
| Special key fill color | RGBA | (adaptive) |
| Candidate text color | RGBA | (adaptive) |
| Candidate background color | RGBA | (adaptive) |

### Font Options
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
| Font management | `FontManager` (@ObservedObject) | `TypefaceLoader` + `PrefHelper` |
| Appearance settings UI | `AppearanceSettingsView.swift` (SwiftUI) | `AppearanceSettingsScreen.kt` (Compose) |
| Settings sync | `UserDefaults.didChangeNotification` | DataStore Flow observation |

---

## Design Philosophy

### Avoid Modern Style

- No heavy shadows
- No floating card effects
- No overly rounded corners
- No uniform icon circles

### Use Retro Elements

- Flat design + thin borders
- Left color bar (file folder style)
- Dashed separators
- Left-aligned titles
