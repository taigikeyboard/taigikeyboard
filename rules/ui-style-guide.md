# UI Style Guide (Cross-Platform)

Mandatory rules for main app UI styling. Both platforms MUST align on these values.
When adding or modifying UI, check this guide first. Do not introduce new size/color values without updating this file.

## Typography Tiers

| Tier | Usage | Android | iOS |
|------|-------|---------|-----|
| Page Title (expanded) | Top-level tab main titles (Home / Layout / Dictionary / Settings) | `MaterialTheme.typography.headlineLarge` (34sp), LargeTopAppBar | .navigationBarTitleDisplayMode(.large), 34pt Open Huninn |
| Page Title (collapsed) | Top-level tab scrolled | titleLarge (22sp) | inline title (22pt Open Huninn) |
| Sub-page Title | SetupGuide, Appearance, etc. | TopAppBar (default titleLarge) | .navigationBarTitleDisplayMode(.inline) |
| Section Header | Card group labels | `MaterialTheme.typography.titleMedium` (18sp), onSurfaceVariant | appFont(size: 18), .secondary |
| Body / Row Label | List items, toggle labels, descriptions, search empty state | `MaterialTheme.typography.bodyLarge` (17sp), onSurface | 17pt (.body), .primary |
| Caption | Trailing values, metadata, tags, badges, TL annotations, dates | `MaterialTheme.typography.labelLarge` (14sp), onSurfaceVariant | 12–16pt (.callout/.caption), .secondary |

**Android font system**: All font sizes and HuninnFontFamily are defined in `ui/theme/Theme.kt` as `AppTypography`. Screens access them via `MaterialTheme.typography.*`. Do NOT use inline `.sp` values for text.

## Font Family

- Primary: **JF Open Huninn** (jf-openhuninn-2.1)
- Android: `HuninnFontFamily` via Material 3 Typography (`ui/theme/Theme.kt`)
- iOS: `KeyboardModels.Fonts.openHuninnFontName` via UINavigationBarAppearance + `.environment`

## Colors (Semantic)

| Role | Android (Light / Dark) | iOS |
|------|------------------------|-----|
| Primary text | onSurface | .primary |
| Secondary text | onSurfaceVariant | .secondary |
| Interactive blue | primary (#007AFF / #0A84FF) | .accentColor / .blue |
| Warning | `AppStyle.warningOrange()` (#FF9500 / #FF9F0A) | .orange |
| Error / Destructive | error (#FF3B30 / #FF453A) | .red / .destructive |
| Link text | primary (#007AFF / #0A84FF) | .blue |
| Section header text | onSurfaceVariant | .secondary (or system Section header gray) |
| Card background | surface | system grouped background |
| Page background | surfaceContainer | systemGroupedBackground |

## Icon Sizes

| Usage | Size (dp / pt) | Android constant |
|-------|----------------|------------------|
| Row leading icon | 24 | (inside row components) |
| Row trailing chevron | 24 | `AppStyle.trailingChevronSize` |
| Small / detail / help icon | 16 | `AppStyle.smallIconSize` |
| Checkmark / reset icon | 20 | `AppStyle.selectionIconSize` |
| External link icon | 12 | — |

## Icon Colors

| Usage | Android (Light / Dark) | iOS |
|-------|------------------------|-----|
| Interactive icon (leading) | `MaterialTheme.colorScheme.primary` | .accentColor |
| Trailing chevron | onSurfaceVariant | .secondary |
| Checkmark (selected) | `MaterialTheme.colorScheme.primary` | .accentColor |
| Warning / feature icon | `AppStyle.warningOrange()` | .orange |

## Component Specs

### Row Components

| Property | Android | iOS |
|----------|---------|-----|
| Horizontal padding | 20dp | Form default |
| Vertical padding | 12dp | Form default |
| Min height (action) | 56dp | system |
| Min height (switch/setting) | 48dp | system |
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
| Icon size | `AppStyle.smallIconSize` (16dp) | .body (17pt) |
| Icon color | `MaterialTheme.colorScheme.primary` | .blue |
| Dialog text | `bodyLarge` (17sp), onSurface, lineHeight 26sp | 17pt, .primary |
| Dialog button | `bodyLarge` (17sp) SemiBold | system default |

## Centralized Style Files

### Android

**Typography** → `ui/theme/Theme.kt` (`AppTypography`)
- Defines HuninnFontFamily + iOS-aligned sizes for headlineLarge (34sp), titleMedium (18sp), bodyLarge (17sp), labelLarge (14sp)
- Access via `MaterialTheme.typography.*` — never use raw `.sp` in screen files

**Dimensions & Colors** → `ui/theme/AppStyle.kt`
- `AppStyle.trailingChevronSize` (24dp)
- `AppStyle.sectionSpacing` (24dp), `AppStyle.sectionHeaderBottomPadding` (6dp)
- `AppStyle.largeTopAppBarExpandedHeight` (112dp)
- `AppStyle.warningOrange()`, `AppStyle.switchColors()`
- `SectionHeader(text)` shared composable

### iOS

**Typography & Colors** → `App/Components/AppStyle.swift`
- `AppStyle.sectionHeaderFont` (18pt Open Huninn), `AppStyle.bodyFont` (17pt), `AppStyle.captionFont` (14pt)
- `AppStyle.sectionHeaderColor` (.secondary), `AppStyle.primaryColor`, `AppStyle.secondaryColor`
- `AppStyle.accentBlue` (.accentColor), `AppStyle.warningOrange` (.orange)
- `SectionHeader(text:)` shared view

## Rules

1. **New row items**: Use `style = MaterialTheme.typography.bodyLarge` (Android) / `appFont(.body)` (iOS). Never use inline `.sp` for row labels.
2. **New section headers**: Use `SectionHeader(text:)` composable/view, or `style = MaterialTheme.typography.titleMedium` with onSurfaceVariant color.
3. **New icons**: Leading 24dp/pt, trailing `AppStyle.trailingChevronSize` (Android). Use `MaterialTheme.colorScheme.primary` (Android) / `.accentColor` (iOS) for leading icons.
4. **New sub-pages**: Use TopAppBar (Android) / .inline title mode (iOS).
5. **Never hardcode font sizes**: Use `MaterialTheme.typography.*` (Android) or `appFont(.style)` (iOS). No inline `.sp` in screen files. **Exception:** functional drawing values inside Canvas-based components (color picker spectrum, slider thumbs) may use raw values.
6. **Never hardcode colors**: Use `MaterialTheme.colorScheme.*` or `AppStyle.*` (Android) / semantic colors (iOS). **Exception:** Canvas drawing colors (`Color.White`/`Black`/`Gray`).
7. **New settings rows**: Reuse existing row components (Android) or Form patterns (iOS). Don't create one-off row layouts.
8. **Cross-platform alignment**: When adding a feature to one platform, check this guide to ensure the other platform's equivalent uses matching values.
9. **Updating this guide**: If a new tier or pattern is genuinely needed, add it to the centralized style file first, update this guide, then implement on both platforms.

## External Link Icons

`arrow.up.forward.square` (iOS) and `Icons.Outlined.OpenInNew` (Android) are the project's standard "external link" icons. Use them for any action that leaves the app to an external resource:

- Web URLs (User Guide, Privacy Policy, dictionary links)
- Share sheet (share diagnostic info)
- Email client (email bug report)

Never replace these icons with action-specific alternatives (don't use a "Share" icon for share, don't use an "Email" icon for email). All external-resource actions use the same link icon for consistency. Only non-link actions (e.g. Copy, which stays in-app) get distinct icons.

## Feature Grouping by Usage Frequency

Separate features by **usage frequency**, not by data relationship. Frequently-edited content goes on its own page; less-frequently-accessed management features group into a separate management page.

- 自訂詞庫 (custom dictionary editing) = frequent edit → own page.
- 詞頻 / 詞關聯 / 備份 (frequency / association / backup) = rare access → grouped management page.

Mixing them creates confusion about a page's purpose. When reorganizing settings or management UIs, group by how often the user interacts with each feature, not by which data table they live in.
