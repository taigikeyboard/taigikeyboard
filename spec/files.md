# 檔案索引與命名規則

> **類型**: 索引
> **關鍵字**: `Files`, `Structure`, `Mapping`, `Naming`
> **相關**: README.md

---

## 重點摘要

- iOS/Android 跨平台檔案對應表
- 統一命名規則與目錄結構

---

## 功能模組對照

| 功能代號 | iOS 目錄 | Android 目錄 |
|----------|----------|--------------|
| `Composing` | `Input/` | `ime/text/composing/` |
| `Autocomplete` | `Autocomplete/` | `ime/text/composing/` |
| `Lexicon` | `Lexicon/` | `ime/dictionary/` |
| `Trie` | `Lexicon/Trie/` | `ime/dictionary/` |
| `Tone` | `Input/Tone/` | `ime/dictionary/` |
| `UserFrequency` | `Lexicon/` | `ime/text/composing/` |
| `NextWord` | `Lexicon/` | `ime/dictionary/` |
| `Layout` | `Layout/` | `ime/text/layout/` |
| `Smartbar` | `Autocomplete/Views/` | `ime/text/smartbar/` |
| `Settings` | `Settings/` | `settings/` |
| `Theme` | `Theme/` | `ui/theme/` |

---

## 核心檔案對照

### IME 入口

| 功能 | iOS | Android |
|------|-----|---------|
| 主入口 | `KeyboardViewController.swift` | `TaigiKeyboard.kt` |
| 鍵盤 View | `TaigiKeyboardView.swift` | `KeyboardView.kt` |
| 偏好設定 | `SharedSettings.swift` | `PrefHelper.kt` |

### Composing

| 功能 | iOS | Android |
|------|-----|---------|
| 組字管理 | `ComposingManager.swift` | `ComposingManager.kt` |
| TPS 轉換 | `TPSConverter.swift` | - |
| 大小寫轉換 | `CaseTransformationService.swift` | `SuggestionCaseTransformer.kt` |

### Autocomplete

| 功能 | iOS | Android |
|------|-----|---------|
| 台語自動完成 | `AutocompleteService.swift` | `TaigiAutocompleteService.kt` |
| 英語自動完成 | - | `EnglishAutocompleteService.kt` |
| 候選詞 View | `CandidateView.swift` | `SmartbarView.kt` |
| 候選詞 Adapter | - | `CandidateAdapter.kt` |
| 展開浮層 | `ExpandedCandidateOverlay.swift` | `CandidateOverlayView.kt` |

### Lexicon

| 功能 | iOS | Android |
|------|-----|---------|
| 字典服務 | `LexiconService.swift` | `LexiconService.kt` |
| Trie 服務 | `TrieService.swift` | `TrieService.kt` |
| 輸入正規化 | `InputNormalizer.swift` | `InputNormalizer.kt` |
| 詞彙模型 | `TaigiWord.swift` | `DictionaryModels.kt` |

### Tone

| 功能 | iOS | Android |
|------|-----|---------|
| 聲調轉換 | `ToneConverter.swift` | `ToneConverter.kt` |
| 調符映射 | `ToneMappings.swift` | `ToneConverterModels.kt` |

### UserFrequency

| 功能 | iOS | Android |
|------|-----|---------|
| 頻率服務 | `UserFrequencyService.swift` | `UserFrequencyService.kt` |
| 頻率 Repository | `UserFrequencyRepository.swift` | (內建) |

---

## 目錄結構

### iOS (`ios/Sources/TaigiKeyboard/`)

```
TaigiKeyboard/
├── _Keyboard/       # IME 主入口
├── Actions/         # 動作處理
├── App/             # 主 App UI
│   ├── Assets/      # 圖片資源
│   ├── Components/  # 共用元件
│   └── Tabs/        # Tab 頁面
├── Autocomplete/    # 自動完成
│   ├── Models/
│   ├── Services/
│   └── Views/
├── Callouts/        # 長按選單
├── Debug/           # Debug 工具
├── Emojis/          # Emoji 相關
├── Input/           # 輸入與組字
│   └── Tone/        # 聲調處理
├── Layout/          # 鍵盤佈局
│   └── Flick/       # Flick 輸入
├── Lexicon/         # 字典查詢
│   ├── Database/
│   ├── Models/
│   ├── Services/
│   ├── Trie/
│   └── Utils/
├── Localization/    # 多語言
├── Settings/        # 設定
├── Styling/         # 按鈕樣式
│   ├── Helpers/
│   └── Providers/
└── Theme/           # 主題
```

### Android (`android/app/src/main/java/.../taigikeyboard/`)

```
taigikeyboard/
├── ime/
│   ├── core/        # TaigiKeyboard, PrefHelper, InputView
│   ├── dictionary/  # Trie, Lexicon, Tone
│   ├── keyboard/    # EmojiSkinTone
│   ├── lifecycle/   # LifecycleInputMethodService
│   ├── media/       # Emoji
│   │   └── emoji/   # EmojiKeyboardView, EmojiPaletteView
│   ├── popup/       # 按鍵彈出
│   └── text/
│       ├── composing/   # Composing, Autocomplete
│       ├── key/         # KeyView, KeyData
│       ├── keyboard/    # KeyboardView
│       ├── layout/      # LayoutManager
│       └── smartbar/    # Smartbar
├── localization/    # 多語言
├── model/           # 資料模型
├── settings/        # 設定頁面
├── ui/theme/        # 主題
└── util/            # 工具函數
```

---

## 資源檔案

### iOS (`ios/Resources/`)

| 檔案 | 大小 | 說明 |
|------|------|------|
| `dictionary.db` | ~40MB | SQLite 詞典 |
| `dictionary.trie` | ~4MB | MARISA Trie |
| `Iansui-Regular.ttf` | ~9MB | 芫荽字體 |
| `jf-openhuninn-2.1.ttf` | ~5MB | 粉圓字體 |

### Android (`android/app/src/main/assets/`)

| 檔案 | 說明 |
|------|------|
| `dictionary.db` | SQLite 詞典 |
| `dictionary.trie` | MARISA Trie |

---

## 命名慣例

| 類型 | iOS | Android |
|------|-----|---------|
| Service | `XxxService.swift` | `XxxService.kt` |
| Manager | `XxxManager.swift` | `XxxManager.kt` |
| View | `XxxView.swift` | `XxxView.kt` |
| Models | `XxxModels.swift` | `XxxModels.kt` |
