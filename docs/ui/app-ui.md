# App UI Design Guide

> **Type**: Feature
> **Keywords**: `App`, `Tab`, `Settings`, `UI`
> **Related**: theme.md, ../contributing/ui-style-guide.md

---

## Summary

- iOS / Android main app: 5 tabs — Home, Theme, Layout, Dictionary, Settings
- Light and dark appearance on both platforms
- Typography, sizes, spacing, list rows and cards: [ui-style-guide.md](../contributing/ui-style-guide.md) (canonical)

---

## Tab Structure

Tab order is fixed by iOS `App/Tabs/TabType.swift` (`home = 0 … settings = 4`) and mirrored by Android `SettingsMainActivity.kt` `TAB_*` constants. Tab titles come from `i18n/nav.json` (`nav.tabHome` … `nav.tabSettings`).

| Tab | iOS (`ios/Sources/TaigiKeyboard/App/Tabs/`) | Android (`ui/tabs/`) |
|-----|-----|---------|
| Home | `Home/HomeTab.swift` | `home/HomeScreen.kt` |
| Theme | `Theme/ThemeTab.swift` (+ `ThemePickerView`, `ThemeEditorView`) | `theme/ThemePickerScreen.kt` + `ThemeEditorScreen.kt` |
| Layout | `Layout/LayoutTab.swift` (+ `LayoutEditorView`) | `layout/LayoutScreen.kt` |
| Dictionary | `Dictionary/DictionaryTab.swift`, `Dictionary/Views/DictionarySourcesView.swift` | `dictionary/DictionarySettingsScreen.kt`, `dictionary/DictionarySourcesScreen.kt` |
| Settings | `Settings/SettingsTab.swift` | `settings/InputSettingsScreen.kt` |

iOS tab container: `App/ContentView.swift` (`TabView`). Android uses Jetpack Compose screens (not Fragments); tab container: `ui/tabs/MainSettingsScreen.kt` (Material3 `NavigationBar`) mounted by `settings/SettingsMainActivity.kt`. The Theme tab is documented in [theme.md](theme.md). Concept → file map for both platforms: [cross-platform-alignment.md §2](../contributing/cross-platform-alignment.md#2-where-each-concept-lives).

---

## Home Content

Home tab content is **JSON-driven** — feature descriptions and FAQ are loaded from shared JSON files.

| File | Platform | Description |
|------|----------|-------------|
| `i18n/content/features.json` | Shared | Feature descriptions (input modes, autocomplete, etc.) |
| `i18n/content/faq.json` | Shared | FAQ entries |
| `FeatureContent.swift` / `.kt` | Both | Data model |
| `FeatureContentLoader.swift` / `.kt` | Both | JSON loader |

These live under `i18n/` to keep all translatable JSON in one folder, but use their own nested-tree schema (NOT the flat i18n namespace schema). The codegen glob is non-recursive (`i18n/*.json`), so `make i18n` skips `i18n/content/` — the platforms decode these files directly.

Both platform resource files are **symlinks** into `i18n/content/`, so an edit there is visible on both platforms with no sync step:
- `ios/Sources/TaigiKeyboard/App/Tabs/Home/{features,faq}.json`
- `android/app/src/main/assets/{features,faq}.json`

---

## Dictionary Data Management

| Sub-screen | iOS | Android | Description |
|------------|-----|---------|-------------|
| Custom Dictionary | `CustomDictionaryView.swift` | `CustomDictionaryScreen.kt` | CRUD, import/export |
| Data Management | `DataManagementView.swift` | `DataManagementScreen.kt` | Backup/restore |

Word-frequency and word-association records have no viewer page: both are always on and travel only through the `.taigi` backup.

---

## Strings and Preview Images

- App-UI strings are generated from `i18n/*.json` by `make i18n`: [tools/i18n/README.md](../../tools/i18n/README.md); key-management rules: [i18n.md](../contributing/i18n.md).
- Theme / Layout card screenshots are rendered from the real keyboard view by the `previews` skill: [.claude/skills/previews/SKILL.md](../../.claude/skills/previews/SKILL.md) (asset paths and sizes).
