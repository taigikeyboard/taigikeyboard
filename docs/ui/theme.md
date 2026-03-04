# Theme Design

> **Type**: Feature
> **Keywords**: `Theme`, `Color`, `Style`
> **Related**: app-ui.md

---

## Summary

- Flat design: no shadows, uses borders
- iOS App supports Light/Dark Mode
- Android App supports Light/Dark/Auto Mode

---

## Color System

Both platforms use platform-native color systems. The SPY×FAMILY retro palette has been deprecated.

- iOS: System colors via SwiftUI `Color` + custom overrides in `AppearanceSettingsView`
- Android: Material 3 colors via `colors.xml` + `values-night/colors.xml`

---

## Card Style

| Property | Value | Description |
|----------|-------|-------------|
| cornerRadius | 10dp/pt | Retro square |
| elevation | 0 | No shadow |
| strokeWidth | 1dp/pt | Thin border |
| background | surfaceSecondary | Cream white |

---

## Left Color Bar Design

- Width: 4dp
- Color logic:
  - Gray-green: Primary features
  - Deep red: Important features
  - Pink: Warm accent

---

## Platform Correspondence

| Item | iOS | Android |
|------|-----|---------|
| Color definitions | `ThemeTokens.swift` | `colors.xml` |
| Theme styles | `ThemeTokens.swift` | `themes.xml` |
| Dark Mode | Supported | Supported (Light/Dark/Auto) |

---

## Design Philosophy

### Avoid Modern Style

- ❌ Heavy shadows
- ❌ Floating card effects
- ❌ Overly rounded corners
- ❌ Uniform icon circles

### Use Retro Elements

- ✅ Flat design + thin borders
- ✅ Left color bar (file folder style)
- ✅ Dashed separators
- ✅ Left-aligned titles
