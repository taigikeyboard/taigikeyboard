# UI Style Guide (iOS + Android app UI)

Shared values for the main-app UI. Android takes them from `ui/theme/Theme.kt` (`AppTypography`) and `ui/theme/AppStyle.kt`; iOS from `App/Components/AppStyle.swift`. Screens use those, never inline `.sp` / point sizes or raw colours (excepted: Canvas drawing code, and the Android-only `ColorPickerDialog.kt` tab labels 13sp / slider labels 15sp — iOS uses the system picker, so no shared tier exists). A value genuinely missing goes into the style file and this table first, then onto both platforms.

## Typography

| Tier | Usage | Android | iOS |
|---|---|---|---|
| Page title, expanded | top-level tabs | `headlineLarge` 34sp in `LargeTopAppBar` | `.large` title, 34pt Open Huninn |
| Page title, collapsed | top-level tab scrolled | `titleLarge` 22sp | inline title 22pt |
| Sub-page title | Setup Guide, Appearance, … | `TopAppBar` (`titleLarge`) | `.inline` title |
| Section header | card group labels | `titleMedium` 18sp, `onSurfaceVariant` (`SectionHeader(text)`) | `AppStyle.sectionHeaderFont` 18pt, `.secondary` |
| Emphasis | row headline | — | `AppStyle.headlineFont` 18pt bold |
| Body / row label | rows, toggles, empty states | `bodyLarge` 17sp, `onSurface` | `AppStyle.bodyFont` 17pt, `.primary` |
| Caption | trailing values, tags, dates, TL annotations | `labelLarge` 17sp, `onSurfaceVariant` | `AppStyle.captionFont` 17pt, `.secondary` |

Font family: JF Open Huninn 2.1 (`HuninnFontFamily`; `KeyboardFonts.openHuninnFontName` via `UINavigationBarAppearance`).

## Colours

| Role | Android (light / dark) | iOS |
|---|---|---|
| Primary / secondary text | `onSurface` / `onSurfaceVariant` | `.primary` / `.secondary` |
| Interactive, links, leading icons, checkmarks | `primary` #007AFF / #0A84FF | `.accentColor` (`AppStyle.accentBlue`) |
| Warning, feature icons | `AppStyle.warningOrange()` #FF9500 / #FF9F0A | `.orange` (`AppStyle.warningOrange`) |
| Error / destructive | `error` #FF3B30 / #FF453A | `.red` / `.destructive` |
| Trailing chevron | `onSurfaceVariant` | `.secondary` |
| Card / page background | `surface` / `surfaceContainer` | system grouped background |
| Dashed outline | `AppStyle.secondaryLabel()` #3C3C43 / #EBEBF5 α 0.6 | `.secondary` |
| Gallery create card | `AppStyle.createCardFill()` #D1D1D6 / #3A3A3C | `systemGray4` |
| Bordered button fill, also disabled | `AppStyle.neutralFill()` #787880 α 0.16 / 0.32 | `.bordered` (secondarySystemFill) |
| Destructive bordered button fill | `AppStyle.destructiveFill()` `error` α 0.18 / 0.20 | `.bordered` + `.tint(.red)` |
| Disabled button content | `AppStyle.disabledContent()` #3C3C43 / #EBEBF5 α 0.3 | `.bordered` disabled (tertiaryLabel) |

## Sizes

| Item | Android | iOS |
|---|---|---|
| Row leading icon | 24dp | 24pt |
| Trailing chevron | `AppStyle.trailingChevronSize` 24dp | system |
| Small / help / external-link icon | `AppStyle.smallIconSize` 16dp | `.body` (help), `.subheadline` (link) |
| Checkmark / reset icon | `AppStyle.selectionIconSize` 20dp | system |
| Section spacing / header bottom pad | `AppStyle.sectionSpacing` 24dp / `sectionHeaderBottomPadding` 6dp | `Form` default |
| Scroll content bottom pad | `AppStyle.scrollContentBottomPadding` 40dp | `Form` default |
| Rows | `ActionRow` / `SwitchRow` / `SettingNavigationRow` / `ColorRow`: 20dp × 12dp padding, min height 56dp (action) / 48dp (switch, setting), 12dp icon gap; `NavigationRow`: 16dp all sides | `Form` + `Toggle` / `NavigationLink` |
| Card | `SettingsCard`, 12dp corners, `surface` | `Section` in `Form` |
| Divider | `SettingsDivider`, 0.5dp `outlineVariant` α 0.4, full width | system |
| Info dialog | `SettingInfoButton`: `bodyLarge` text, line height 26sp, SemiBold button | 17pt, system button |

Android rows reuse the `ui/components/` kit; iOS uses `Form` patterns. No one-off row layouts.

## Switch / Toggle

Intentional divergence — each platform keeps its native toggle: Android Material 3 default `Switch` (checked track = `primary`, 48dp touch target); iOS system green `Toggle`. Do not emulate one on the other.

## External links

Every action that leaves the app (web page, share sheet, email) uses the one link icon: iOS `arrow.up.forward.square`, Android `TaigiIcons` `Icons.AutoMirrored.Outlined.OpenInNew`. A leading glyph may name the destination (envelope for email); the trailing link icon stays. In-app actions (Copy, Export via the file exporter) use their own icons.

Exception: a link inline in a footnote line (attribution, copyright) drops the icon and takes the surrounding text's style and colour. None ships today; a new one follows the project site's footer (`taigi-converter/index.html`).

## Grouping

Group settings by how often they are used, not by which table holds the data: Custom Dictionary (frequent editing) has its own page; Backup and Restore (rare) sits on a separate management page.
