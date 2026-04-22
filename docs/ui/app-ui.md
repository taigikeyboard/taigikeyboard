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
| Tab1 Home (頭頁) | `Tab1.swift` | `HomeScreen.kt` |
| Tab2 Layout (佈局) | `Tab2.swift` | `LayoutScreen.kt` |
| Tab3 Dictionary (詞庫) | `Tab3.swift` | `DictionarySettingsScreen.kt` |
| Tab4 Settings (設定) | `Tab4.swift` | `InputSettingsScreen.kt` |

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

## Tab1 Content Architecture (v3.4.7+)

Tab1 (Home) content is **JSON-driven** — feature descriptions and FAQ are loaded from shared JSON files instead of hardcoded enums.

| File | Platform | Description |
|------|----------|-------------|
| `content/tab1-features.json` | Shared | Feature descriptions (input modes, autocomplete, etc.) |
| `content/tab1-faq.json` | Shared | FAQ entries |
| `FeatureContent.swift` / `.kt` | Both | Data model |
| `FeatureContentLoader.swift` / `.kt` | Both | JSON loader |

Content sync: both platform resource files are **symlinks** into `content/`:
- `ios/Sources/TaigiKeyboard/App/Tabs/Home/tab1-{features,faq}.json` → `content/tab1-{features,faq}.json`
- `android/app/src/main/assets/tab1-{features,faq}.json` → `content/tab1-{features,faq}.json`

Editing `content/tab1-*.json` is immediately visible on both platforms — no sync step required.

---

## Tab3 Data Management (v3.4.5+)

Tab3 expanded from basic dictionary settings to full data management:

| Sub-screen | iOS | Android | Description |
|------------|-----|---------|-------------|
| Custom Dictionary | `CustomDictionaryView.swift` | `CustomDictionaryScreen.kt` | CRUD, import/export |
| Frequency Data | `FrequencyDataView.swift` | `FrequencyDataScreen.kt` | View/clear user frequency |
| Association Data | `AssociationDataView.swift` | `AssociationDataScreen.kt` | View/clear next-word data |
| Data Management | `DataManagementView.swift` | `DataManagementScreen.kt` | Backup/restore |

Previous Debug screens (`DebugView`, `DebugActivity`) were removed and replaced by these production views.

---

## Localization Architecture

| File | Purpose |
|------|---------|
| `LocalizedText.kt` | Core structure (Android only) |
| `Tab1Texts.swift` / `.kt` | Tab1 text |
| `Tab2Texts.swift` / `.kt` | Tab2 text |
| `Tab3Texts.swift` / `.kt` | Tab3 text |
| `Tab4Texts.swift` / `.kt` | Tab4 text |
| `LanguageManager.kt` | Android language selection (StateFlow) |
| `DisplayLanguage.kt` | Android display language enum |

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

### Tab2 Layout Preview Screenshots

When layout appearance changes (font size, key labels, etc.), update these screenshots.

**iOS** — `ios/Sources/TaigiKeyboard/Styling/LayoutPreviewAssets.xcassets/`

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
| Localization | `CommonTexts.swift`, `Tab*Texts.swift` |
| Image assets | `Assets.xcassets/` |

### Android

| Component | File |
|-----------|------|
| Tab container | `SettingsMainActivity.kt` + `MainSettingsScreen.kt` |
| Navigation | Compose Navigation (no XML) |
| Theme | `Theme.kt` + `Type.kt` |
| Strings | `LocalizedText.kt` + `Tab1-4Texts.kt` |
