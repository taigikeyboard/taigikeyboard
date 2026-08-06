# App UI Design Guide

> **Type**: Feature
> **Keywords**: `App`, `Tab`, `Settings`, `UI`
> **Related**: theme.md

---

## Summary

- iOS/Android main app UI design specifications
- 4-Tab structure: Home (頭頁), Layout (佈局), Dictionary (詞庫), Settings (設定)
- Supports Light/Dark Mode (iOS)

---

## Tab Structure

| Tab | iOS | Android |
|-----|-----|---------|
| Home (頭頁) | `HomeTab.swift` | `HomeScreen.kt` |
| Layout (佈局) | `LayoutTab.swift` | `LayoutScreen.kt` |
| Dictionary (詞庫) | `DictionaryTab.swift` | `DictionarySettingsScreen.kt` |
| Settings (設定) | `SettingsTab.swift` | `InputSettingsScreen.kt` |

Android uses Jetpack Compose screens (not Fragments). Tab container: `MainSettingsScreen.kt`.

---

## iOS Spacing Guidelines (8pt multiples)

| Use | Spacing |
|-----|---------|
| Between sections | 32pt |
| Horizontal padding | 20pt |
| Top padding | 24pt |
| Card interior | 16pt |
| List item vertical | 14pt |

---

## Font Sizes (Apple HIG)

| Use | Size | Method |
|-----|------|--------|
| Hero Title | 34pt | `themeFontHeroTitle()` |
| Title | 24pt | `themeFontTitle()` |
| Headline | 18pt | `themeFontHeadline()` |
| Body | 17pt | `themeFontBody()` |
| Caption | 14pt | `themeFontCaption()` |

---

## Navigation Patterns

| Type | Use | Method |
|------|-----|--------|
| Sub-pages | Detail pages | `NavigationLink` |
| First launch | Onboarding | `fullScreenCover` |
| External links | Web/settings | `openURL` |

---

## Component Guidelines

### List Items

```
[Icon/Number] [Title] [Spacer] [chevron.right]
```

- Icon: 16pt, width 24pt
- Chevron: 12pt
- Horizontal padding: 16pt
- Vertical padding: 14pt

### Card Container

- cornerRadius: 12pt
- elevation: 0
- strokeWidth: 1pt

---

## Home Content Architecture (v3.4.7+)

Home tab content is **JSON-driven** — feature descriptions and FAQ are loaded from shared JSON files instead of hardcoded enums.

| File | Platform | Description |
|------|----------|-------------|
| `i18n/content/features.json` | Shared | Feature descriptions (input modes, autocomplete, etc.) |
| `i18n/content/faq.json` | Shared | FAQ entries |
| `FeatureContent.swift` / `.kt` | Both | Data model |
| `FeatureContentLoader.swift` / `.kt` | Both | JSON loader |

These live under `i18n/` to keep all translatable JSON in one folder, but use their own nested-tree schema (NOT the flat i18n namespace schema). The codegen glob is non-recursive (`i18n/*.json`), so the `i18n/content/` subfolder is skipped by `make i18n` — the platforms decode these files directly.

Content sync: both platform resource files are **symlinks** into `i18n/content/`:
- `ios/Sources/TaigiKeyboard/App/Tabs/Home/{features,faq}.json` → `i18n/content/{features,faq}.json`
- `android/app/src/main/assets/{features,faq}.json` → `i18n/content/{features,faq}.json`

Editing `i18n/content/{features,faq}.json` is immediately visible on both platforms — no sync step required.

---

## Dictionary Data Management (v3.4.5+)

The Dictionary tab expanded from basic dictionary settings to full data management:

| Sub-screen | iOS | Android | Description |
|------------|-----|---------|-------------|
| Custom Dictionary | `CustomDictionaryView.swift` | `CustomDictionaryScreen.kt` | CRUD, import/export |
| Frequency Data | `FrequencyDataView.swift` | `FrequencyDataScreen.kt` | View/clear user frequency |
| Association Data | `AssociationDataView.swift` | `AssociationDataScreen.kt` | View/clear next-word data |
| Data Management | `DataManagementView.swift` | `DataManagementScreen.kt` | Backup/restore |

Previous Debug screens (`DebugView`, `DebugActivity`) were removed and replaced by these production views.

---

## Localization Architecture

**Mid-migration — per-file correspondence is in flux.** App-UI strings are moving to a generated-resource pipeline (`i18n/*.json` → `tools/i18n/generate.py` → `i18n/generated/{L10n,StringKey,GeneratedTaigiStrings}.kt`). Android migrated namespaces use the generated accessors; not-yet-migrated namespaces keep hand-written `localization/*Texts.kt`. iOS still uses `Strings/*Texts.swift`. Authoritative status: [`../architecture/i18n-multilang-plan.md`](../architecture/i18n-multilang-plan.md).

---

## Image Resources

### Naming Convention

- Must include `@3x` suffix
- Example: `layout_standard_preview@3x.png`

### Size Specifications

| Type | Display size | @3x image |
|------|--------------|-----------|
| Layout preview | 180×120pt | 540×360px |
| Screenshots | Adaptive | Varies |

### Layout Preview Screenshots

When layout appearance changes (font size, key labels, etc.), update these screenshots.

**iOS** — `ios/Resources/Assets/LayoutPreviewAssets.xcassets/`

| Layout | Light | Dark |
|--------|-------|------|
| Standard | `layout_standard_preview.imageset/layout_standard_preview_light@3x.png` | `layout_standard_preview.imageset/layout_standard_preview_dark@3x.png` |
| PhahTaigi | `layout_phahtaigi_preview.imageset/layout_phahtaigi_preview_light@3x.png` | `layout_phahtaigi_preview.imageset/layout_phahtaigi_preview_dark@3x.png` |
| MOE1 | `layout_moe1_preview.imageset/layout_moe1_preview_light@3x.png` | `layout_moe1_preview.imageset/layout_moe1_preview_dark@3x.png` |
| MOE2 | `layout_moe2_preview.imageset/layout_moe2_preview_light@3x.png` | `layout_moe2_preview.imageset/layout_moe2_preview_dark@3x.png` |
| TPS | `layout_tps_preview.imageset/layout_tps_preview_light@3x.png` | `layout_tps_preview.imageset/layout_tps_preview_dark@3x.png` |

**Android** — `android/app/src/main/res/`

| Layout | Light | Dark |
|--------|-------|------|
| Standard | `drawable-xxxhdpi/layout_standard_preview.png` | `drawable-night-xxxhdpi/layout_standard_preview.png` |
| PhahTaigi | `drawable-xxxhdpi/layout_phahtaigi_preview.png` | `drawable-night-xxxhdpi/layout_phahtaigi_preview.png` |
| MOE1 | `drawable-xxxhdpi/layout_moe1_preview.png` | `drawable-night-xxxhdpi/layout_moe1_preview.png` |
| MOE2 | `drawable-xxxhdpi/layout_moe2_preview.png` | `drawable-night-xxxhdpi/layout_moe2_preview.png` |
| TPS | `drawable-xxxhdpi/layout_tps_preview.png` | `drawable-night-xxxhdpi/layout_tps_preview.png` |

**Android code references** (where screenshots are displayed):
- `LayoutScreen.kt` — App settings layout selection page (`R.drawable.layout_{name}_preview`)
- `LayoutSelectionOverlayView.kt` — Keyboard overlay layout switcher (`R.drawable.layout_{name}_preview`)

---

## Platform File Correspondence

### iOS

| Component | File |
|-----------|------|
| Tab container | `ContentView.swift` |
| Theme colors | KeyboardKit adaptive colors + `SharedSettings.colorSettings` |
| Localization | `Strings/` i18n resolver — `DisplayLanguageStore` + `StringResolver` + generated `StringKey` / `StringResolverFormats`; strings codegen from `i18n/*.json` → `Localizable.xcstrings`. (All `*Texts.swift` constant files removed in R2b.) |
| Image assets | `Assets.xcassets/` |

### Android

| Component | File |
|-----------|------|
| Tab container | `SettingsMainActivity.kt` + `MainSettingsScreen.kt` |
| Navigation | Compose Navigation (no XML) |
| Theme | `Theme.kt` + `AppStyle.kt` |
| Strings | generated `i18n/generated/L10n.kt` (migrated namespaces) + residual `localization/*Texts.kt` — see Localization Architecture above |
