# 檔案索引與命名規則

> **關鍵字**: `files`, `naming`, `structure`, `對應表`

---

## 命名規則

### 功能模組命名

| 功能代號 | 說明 | iOS 目錄 | Android 目錄 |
|----------|------|----------|--------------|
| **Composing** | 組字管理 | `Input/` | `ime/text/composing/` |
| **Autocomplete** | 自動完成 | `Autocomplete/` | `ime/text/composing/` |
| **Lexicon** | 字典查詢 | `Lexicon/` | `ime/dictionary/` |
| **Trie** | Trie 索引 | `Lexicon/Trie/` | `ime/dictionary/` |
| **Tone** | 聲調處理 | `Input/Tone/` | `ime/dictionary/` |
| **UserFrequency** | 使用者頻率 | `Lexicon/` | `ime/text/composing/` |
| **NextWord** | 下一詞預測 | `Lexicon/` | `ime/dictionary/` |
| **Smartbar** | 候選詞列 | `Autocomplete/Views/` | `ime/text/smartbar/` |
| **Settings** | 設定 | `Settings/` | `settings/` |
| **Theme** | 主題 | `Theme/` | `ui/theme/` |

### 檔案命名慣例

| 類型 | iOS 慣例 | Android 慣例 | 範例 |
|------|----------|--------------|------|
| Service | `XxxService.swift` | `XxxService.kt` | `LexiconService` |
| Repository | `XxxRepository.swift` | （內建於 Service） | `UserFrequencyRepository` |
| Manager | `XxxManager.swift` | `XxxManager.kt` | `ComposingManager` |
| Models | `XxxModels.swift` | `XxxModels.kt` | `DictionaryModels` |
| View | `XxxView.swift` | `XxxView.kt` | `CandidateView` |
| Constants | `XxxConstants.swift` | （內建於相關檔案） | `LexiconConstants` |

---

## 跨平台檔案對應表

### 核心模組

| 功能 | iOS 檔案 | Android 檔案 |
|------|----------|--------------|
| **IME 主入口** | `_Keyboard/KeyboardViewController.swift` | `ime/core/TaigiKeyboard.kt` |
| **鍵盤 View** | `_Keyboard/TaigiKeyboardView.swift` | `ime/text/keyboard/KeyboardView.kt` |
| **偏好設定** | `Settings/SharedSettings.swift` | `ime/core/PrefHelper.kt` |

### Composing 組字

| 功能 | iOS 檔案 | Android 檔案 |
|------|----------|--------------|
| **組字管理** | `Input/ComposingManager.swift` | `ime/text/composing/ComposingManager.kt` |
| **Context 擴充** | `Input/KeyboardContext+Composing.swift` | — |

### Autocomplete 自動完成

| 功能 | iOS 檔案 | Android 檔案 |
|------|----------|--------------|
| **自動完成服務** | `Autocomplete/Services/AutocompleteService.swift` | `ime/text/composing/TaigiAutocompleteService.kt` |
| **候選詞 View** | `Autocomplete/Views/CandidateView.swift` | `ime/text/smartbar/SmartbarManager.kt` |
| **展開浮層** | `Autocomplete/Views/ExpandedCandidateOverlay.swift` | `ime/text/smartbar/CandidateOverlayView.kt` |

### Lexicon 字典查詢

| 功能 | iOS 檔案 | Android 檔案 |
|------|----------|--------------|
| **字典服務** | `Lexicon/Services/LexiconService.swift` | `ime/dictionary/LexiconService.kt` |
| **SQLite 連線** | `Lexicon/Database/SQLiteConnectionManager.swift` | （內建於 LexiconService） |
| **字典 Repository** | `Lexicon/Database/DictionaryRepository.swift` | （內建於 LexiconService） |
| **詞彙模型** | `Lexicon/Models/TaigiWord.swift` | `ime/dictionary/DictionaryModels.kt` |
| **輸入類型** | `Lexicon/Models/InputType.swift` | （內建於 TaigiAutocompleteService） |
| **錯誤定義** | `Lexicon/Models/DictionaryError.swift` | — |
| **常數定義** | `Lexicon/Models/LexiconConstants.swift` | — |

### Trie 索引

| 功能 | iOS 檔案 | Android 檔案 |
|------|----------|--------------|
| **Trie 服務** | `Lexicon/Trie/TrieService.swift` | `ime/dictionary/TrieService.kt` |
| **輸入正規化** | `Lexicon/Trie/InputNormalizer.swift` | `ime/dictionary/InputNormalizer.kt` |
| **JNI 封裝** | — | `cpp/trie_jni.cpp` |

### Tone 聲調

| 功能 | iOS 檔案 | Android 檔案 |
|------|----------|--------------|
| **聲調轉換** | `Input/Tone/ToneConverter.swift` | `ime/dictionary/ToneConverter.kt` |
| **POJ 轉換** | `Input/Tone/POJToneConverter.swift` | （內建於 ToneConverter） |
| **TL 轉換** | `Input/Tone/TLToneConverter.swift` | （內建於 ToneConverter） |
| **調符映射** | `Input/Tone/ToneMappings.swift` | `ime/dictionary/ToneConverterModels.kt` |
| **聲調還原** | `Input/Tone/ToneRestoration.swift` | （內建於 ToneConverter） |
| **母音分析** | `Input/Tone/VowelAnalyzer.swift` | — |
| **工具函數** | `Input/Tone/ToneUtilities.swift` | `ime/dictionary/ToneCharacterUtils.kt` |

### UserFrequency 使用者頻率

| 功能 | iOS 檔案 | Android 檔案 |
|------|----------|--------------|
| **頻率服務** | `Lexicon/Services/UserFrequencyService.swift` | `ime/text/composing/UserFrequencyService.kt` |
| **頻率 Repository** | `Lexicon/Database/UserFrequencyRepository.swift` | （內建於 UserFrequencyService） |

### NextWord 下一詞預測（規劃中）

| 功能 | iOS 檔案 | Android 檔案 |
|------|----------|--------------|
| **預測服務** | `Lexicon/Services/NextWordService.swift` | `ime/dictionary/NextWordService.kt` |
| **關聯 Repository** | `Lexicon/Database/WordAssociationRepository.swift` | （內建於 NextWordService） |

### Settings 設定

| 功能 | iOS 檔案 | Android 檔案 |
|------|----------|--------------|
| **設定主頁** | `App/Tabs/SettingsTab.swift` | `settings/SettingsMainActivity.kt` |
| **詞庫設定** | `App/Tabs/DictionaryTab.swift` | `settings/DictionarySettingsActivity.kt` |
| **鍵盤設定** | — | `settings/KeyboardSettingsActivity.kt` |
| **共用設定** | `Settings/SharedSettings.swift` | `ime/core/PrefHelper.kt` |

### Theme 主題

| 功能 | iOS 檔案 | Android 檔案 |
|------|----------|--------------|
| **主題定義** | `Theme/ThemeTokens.swift` | `ui/theme/Theme.kt` |
| **按鍵樣式** | `Theme/ButtonStyles.swift` | — |

### App 主 APP UI（iOS）

| 功能 | iOS 檔案 |
|------|----------|
| **App 入口** | `App/TaigiKeyboardApp.swift` |
| **Tab 容器** | `App/ContentView.swift` |
| **Tab 類型定義** | `App/Tabs/TabType.swift` |
| **Tab1 頭頁** | `App/Tabs/HomeTab.swift` |
| **Tab2 佈局** | `App/Tabs/LayoutTab.swift` |
| **Tab3 詞庫** | `App/Tabs/DictionaryTab.swift` |
| **Tab4 設定** | `App/Tabs/SettingsTab.swift` |
| **版權聲明** | `Copyright/Views/CopyrightView.swift` |
| **Onboarding** | `Onboarding/OnboardingView.swift` |

### Localization 本地化（iOS）

| 功能 | iOS 檔案 |
|------|----------|
| **核心結構** | `Localization/LocalizedText.swift` |
| **Tab 標題** | `Localization/TabTexts.swift` |
| **App 資訊** | `Localization/AppInfoTexts.swift` |
| **操作文字** | `Localization/ActionTexts.swift` |
| **鍵盤文字** | `Localization/KeyboardTexts.swift` |
| **引導文字** | `Localization/OnboardingTexts.swift` |
| **版權文字** | `Localization/CopyrightTexts.swift` |
| **頁面專用** | 各 Tab 檔案內的 `XxxTexts` enum |

---

## 目錄結構

### iOS (`ios/Sources/TaigiKeyboard/`)

```
TaigiKeyboard/
├── _Keyboard/           # IME 主入口
├── Actions/             # 動作處理（ActionHandler）
├── App/                 # 主 App UI
│   ├── Assets/          # 圖片資源（Assets.xcassets）
│   ├── Components/      # 共用元件（ImageSlideshowView 等）
│   ├── Tabs/            # Tab 頁面
│   │   ├── TabType.swift
│   │   ├── HomeTab.swift      # Tab1 頭頁
│   │   ├── LayoutTab.swift    # Tab2 佈局
│   │   ├── DictionaryTab.swift # Tab3 詞庫
│   │   └── SettingsTab.swift  # Tab4 設定
│   ├── ContentView.swift      # Tab 容器
│   └── TaigiKeyboardApp.swift # App 入口
├── Autocomplete/        # 自動完成
│   ├── Models/
│   ├── Services/
│   └── Views/
├── Callouts/            # 長按選單
├── Copyright/           # 版權資訊
│   └── Views/           # CopyrightView
├── Debug/               # Debug 模式（#if DEBUG）
├── Emojis/              # Emoji 服務
├── Input/               # 輸入與組字
│   └── Tone/            # 聲調處理
├── Layout/              # 鍵盤佈局
├── Lexicon/             # 字典查詢
│   ├── Database/        # SQLite 存取
│   ├── Models/          # 資料模型
│   ├── Services/        # 服務層
│   ├── Trie/            # Trie 相關
│   └── Utils/           # 工具函數
├── Localization/        # 多語言（LocalizedText, XxxTexts）
├── Onboarding/          # 引導流程（OnboardingView）
├── Settings/            # 共用設定（SharedSettings）
├── Styling/             # 樣式
└── Theme/               # 主題（ThemeTokens）
```

### Android (`android/app/src/main/java/.../taigikeyboard/`)

```
taigikeyboard/
├── ime/                 # IME 相關
│   ├── core/            # 核心（TaigiKeyboard, PrefHelper）
│   ├── dictionary/      # 字典（Lexicon, Trie, Tone）
│   ├── text/            # 文字輸入
│   │   ├── composing/   # 組字（Composing, Autocomplete, UserFrequency）
│   │   ├── smartbar/    # 候選詞列
│   │   ├── key/         # 按鍵
│   │   ├── keyboard/    # 鍵盤
│   │   └── layout/      # 佈局
│   ├── clipboard/       # 剪貼簿
│   ├── media/           # 媒體（Emoji）
│   ├── popup/           # 彈出視窗
│   └── lifecycle/       # 生命週期
├── settings/            # 設定
├── onboarding/          # 引導流程
├── ui/                  # UI（Theme）
├── util/                # 工具函數
└── model/               # 共用模型
```

---

### iOS Assets (`App/Assets/Assets.xcassets/`)

```
Assets.xcassets/
├── AccentColor.colorset/
├── AppIcon.appiconset/
├── layout_standard_preview.imageset/    # Tab2 佈局預覽
├── layout_phahtaigi_preview.imageset/   # Tab2 佈局預覽
├── setup_step1.imageset/                # Onboarding 截圖
├── setup_step2.imageset/                # Onboarding 截圖
├── nextword_1.imageset/                 # 新功能截圖
├── nextword_2.imageset/
├── nextword_3.imageset/
└── faq_tone_handling.imageset/          # FAQ 截圖
```

**命名規範**：所有圖片檔案必須加上 `@3x` 後綴（如 `setup_step1@3x.png`）

---

## 資料流

```
使用者輸入
    ↓
ComposingManager (rawInput / composingText)
    ↓
AutocompleteService.getSuggestions(rawInput, displayText)
    ↓
LexiconService.search()
    ↓
InputNormalizer.normalize() → TrieService.prefixSearch() → SQLite
    ↓
UserFrequencyService.getFrequencyDataBatch() → 排序
    ↓
Smartbar / CandidateView 更新
    ↓
使用者選擇候選詞
    ↓
UserFrequencyService.recordUsage()
    ↓
（NextWord）NextWordService.predict() → 顯示關聯詞
```

---

## 相關 Spec

| Spec | 對應功能模組 |
|------|--------------|
| `composing.md` | Composing |
| `autocomplete.md` | Autocomplete |
| `sort.md` | UserFrequency, Lexicon |
| `trie.md` | Trie, Lexicon |
| `tone.md` | Tone |
| `nextword.md` | NextWord |
| `app-ui.md` | 主 APP UI 設計規範 |
| `todo.md` | 待辦事項與變更記錄 |
| `azookey-ui.md` | azooKey UI 參考 |
| `theme.md` | 主題設計 |
