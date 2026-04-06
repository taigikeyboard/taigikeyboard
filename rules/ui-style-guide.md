# UI Style Guide (Cross-Platform)

Mandatory rules for main app UI styling. Both platforms MUST align on these values.
When adding or modifying UI, check this guide first. Do not introduce new size/color values without updating this file.

## Typography Tiers

| Tier | Usage | Android | iOS |
|------|-------|---------|-----|
| Page Title (expanded) | Tab1-4 main titles | LargeTopAppBar, 34sp, expandedHeight=112dp | .navigationBarTitleDisplayMode(.large), 34pt Open Huninn |
| Page Title (collapsed) | Tab1-4 scrolled | titleLarge (22sp) | inline title (22pt Open Huninn) |
| Sub-page Title | SetupGuide, Appearance, etc. | TopAppBar (default titleLarge) | .navigationBarTitleDisplayMode(.inline) |
| Section Header | Card group labels | 19sp, onSurfaceVariant | appFont(size: 18), .secondary (Form inherits system gray) |
| Body / Row Label | List items, toggle labels, descriptions, search empty state | 18sp, onSurface | 17pt (.body), .primary |
| Caption | Trailing values, metadata, tags, badges, TL annotations, dates | 14sp, onSurfaceVariant | 12–16pt (.callout/.caption), .secondary |

## Font Family

- Primary: **JF Open Huninn** (jf-openhuninn-2.1)
- Android: `HuninnFontFamily` via Material 3 Typography (`ui/theme/Type.kt`)
- iOS: `KeyboardModels.Fonts.openHuninnFontName` via UINavigationBarAppearance + `.environment`

## Colors (Semantic)

| Role | Android (Light / Dark) | iOS |
|------|------------------------|-----|
| Primary text | onSurface | .primary |
| Secondary text | onSurfaceVariant | .secondary |
| Interactive blue | #007AFF / #0A84FF | .accentColor / .blue |
| Warning | #FF9500 | .orange |
| Error / Destructive | error (#FF3B30 / #FF453A) | .red / .destructive |
| Link text | #007AFF / #0A84FF | .blue |
| Section header text | onSurfaceVariant | .secondary (or system Section header gray) |
| Card background | surface | system grouped background |
| Page background | surfaceContainer | systemGroupedBackground |

## Icon Sizes

| Usage | Size (dp / pt) |
|-------|----------------|
| Row leading icon | 24 |
| Row trailing chevron | 18 |
| Info / help button | 15 |
| Checkmark (selected) | 20 |
| Small detail icon | 16 |
| External link icon | 12 |

## Icon Colors

| Usage | Android (Light / Dark) | iOS |
|-------|------------------------|-----|
| Interactive icon (leading) | #007AFF / #0A84FF | .accentColor |
| Trailing chevron | onSurfaceVariant | .secondary |
| Checkmark (selected) | #007AFF / #0A84FF | .accentColor |
| Warning / feature icon | #FF9500 | .orange |

## Component Specs

### Row Components

| Property | Android | iOS |
|----------|---------|-----|
| Horizontal padding | 20dp | Form default |
| Vertical padding | 12dp | Form default |
| Min height (action) | 56dp | system |
| Min height (switch) | 48dp | system |
| Icon-to-label spacing | 12dp | system |
| Reusable components | ActionRow, SwitchRow, NavigationRow, ColorRow, SliderRow | Form + Toggle / NavigationLink |

### Cards

| Property | Android | iOS |
|----------|---------|-----|
| Corner radius | 12dp | system grouped |
| Background | surface | system grouped background |
| Wrapper | SettingsCard | Section in Form |

### Dividers

| Property | Android | iOS |
|----------|---------|-----|
| Within card | SettingsDivider, padding horizontal 16dp | system default |

### Info Dialog (SettingInfoButton)

| Property | Android | iOS |
|----------|---------|-----|
| Icon size | bodyFontSize (18dp) | .body (17pt) |
| Icon color | #007AFF / #0A84FF | .blue |
| Dialog text | 17sp, onSurface, lineHeight 26sp | 17pt, .primary |
| Dialog button | 17sp SemiBold | system default |

## Centralized Style Files

All styling values are centralized — change one file, applies everywhere:

| Platform | File | Contents |
|----------|------|----------|
| Android | `ui/theme/AppStyle.kt` | Font sizes, colors, dimensions as `AppStyle.xxx` constants |
| iOS | `App/Components/AppStyle.swift` | Section header font/color as `AppStyle.xxx` properties |

### Android: `AppStyle.kt` constants
- `AppStyle.pageTitleFontSize` (34sp), `AppStyle.sectionHeaderFontSize` (19sp)
- `AppStyle.bodyFontSize` (18sp), `AppStyle.captionFontSize` (14sp)
- `AppStyle.largeTopAppBarExpandedHeight` (112dp)
- `AppStyle.interactiveBlue()`, `AppStyle.warningOrange()`
- `SectionHeader(text)` shared composable

### iOS: `AppStyle.swift` constants
- `AppStyle.sectionHeaderFont` (18pt Open Huninn), `AppStyle.bodyFont` (17pt), `AppStyle.captionFont` (14pt)
- `AppStyle.sectionHeaderColor` (.secondary), `AppStyle.primaryColor`, `AppStyle.secondaryColor`
- `AppStyle.accentBlue` (.accentColor), `AppStyle.warningOrange` (.orange)
- `SectionHeader(text:)` shared view

## Rules

1. **New row items**: Use `AppStyle.bodyFontSize` (Android) / `appFont(.body)` (iOS). Never use inline 16 or 18 for row labels.
2. **New section headers**: Use `SectionHeader(text:)` composable/view, or `AppStyle.sectionHeaderFontSize` / `AppStyle.sectionHeaderFont` with muted color.
3. **New icons**: Leading 24dp/pt, trailing 18dp/pt. Use `AppStyle.interactiveBlue()` (Android) / `.accentColor` (iOS) for leading icons.
4. **New sub-pages**: Use TopAppBar (Android) / .inline title mode (iOS).
5. **Never hardcode font sizes or colors**: Use `AppStyle.xxx` constants (Android) or `appFont(.style)` + semantic colors (iOS). No inline `17.sp`, `18.sp`, or `Color(0xFF...)` in screen files. **Exception:** functional drawing colors inside Canvas-based components (color picker spectrum, slider thumbs) may use `Color.White`/`Black`/`Gray` directly.
6. **New settings rows**: Reuse existing row components (Android) or Form patterns (iOS). Don't create one-off row layouts.
7. **Cross-platform alignment**: When adding a feature to one platform, check this guide to ensure the other platform's equivalent uses matching values.
8. **Updating this guide**: If a new tier or pattern is genuinely needed, add it to `AppStyle` first, update this guide, then implement on both platforms.
