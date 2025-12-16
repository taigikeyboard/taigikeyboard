# 台語鍵盤 - 檔案索引

---

## iOS (`ios/Sources/TaigiKeyboard/`)

### 核心
- `_Keyboard/KeyboardViewController.swift` - Keyboard Extension 主入口
- `_Keyboard/TaigiKeyboardView.swift` - 鍵盤 SwiftUI View

### 輸入與組字 (`Input/`)
- `ComposingManager.swift` - 組字狀態管理（rawInput + composingText）
- `KeyboardContext+Composing.swift` - KeyboardKit Context 擴充
- `Tone/*.swift` - 聲調轉換（POJ/TL 調符 ↔ 數字）

### 字典查詢 (`Lexicon/`)
- `Services/LexiconService.swift` - 候選詞搜尋入口
- `Services/UserFrequencyService.swift` - 使用者頻率記錄
- `Database/DictionaryRepository.swift` - Trie + SQLite 查詢
- `Database/SQLiteConnectionManager.swift` - SQLite 連線管理
- `Trie/TrieService.swift` - MARISA-trie Swift 封裝
- `Trie/InputNormalizer.swift` - 輸入正規化（調符→數字）
- `Models/TaigiWord.swift` - 詞彙資料模型

### 自動完成 (`Autocomplete/`)
- `Services/AutocompleteService.swift` - KeyboardKit 自動完成整合
- `Views/CandidateView.swift` - 候選詞視圖

### 設定 (`Settings/`)
- `SharedSettings.swift` - App Group 設定（POJ/TL 模式、詞庫開關）
- `DictionarySettingsView.swift` - 詞庫管理 UI（七大詞庫開關、清除資料）
- `SettingsView.swift` - 齒盤設定 UI

### 資源 (`Resources/`)
- `dictionary.db` - SQLite 字典
- `dictionary.trie` - MARISA-trie 索引

---

## Android (`android/app/src/main/java/.../taigikeyboard/`)

### 核心 (`ime/core/`)
- `TaigiKeyboard.kt` - IME 主服務
- `PrefHelper.kt` - 偏好設定 (DataStore)
- `PreferenceDataStore.kt` - DataStore 封裝

### 文字輸入 (`ime/text/`)
- `TextInputManager.kt` - 文字輸入管理（按鍵處理、候選詞更新）
- `composing/ComposingManager.kt` - 組字狀態（rawInput + composingText 雙狀態）
- `composing/TaigiAutocompleteService.kt` - 候選詞搜尋入口
- `composing/UserFrequencyService.kt` - 使用者頻率記錄

### 字典查詢 (`ime/dictionary/`)
- `LexiconService.kt` - Trie + SQLite 混合查詢
- `TrieService.kt` - MARISA-trie JNI 封裝
- `InputNormalizer.kt` - 輸入正規化（調符→數字）
- `ToneConverter.kt` - 聲調轉換（數字→調符）
- `ToneConverterModels.kt` - 聲調映射表
- `DictionaryModels.kt` - 詞彙資料模型

### 智慧列 (`ime/text/smartbar/`)
- `SmartbarManager.kt` - 候選詞顯示與選擇
- `SmartbarView.kt` - UI 容器
- `CandidateOverlayView.kt` - 候選詞浮層

### 按鍵系統 (`ime/text/key/`, `ime/text/keyboard/`, `ime/text/layout/`)
- `key/KeyCode.kt` - 按鍵代碼
- `key/KeyView.kt` - 按鍵視圖
- `layout/LayoutManager.kt` - 佈局載入

### 設定 (`settings/`)
- `DictionarySettingsActivity.kt` - 詞庫管理（七大詞庫開關、清除資料）
- `KeyboardSettingsActivity.kt` - 齒盤設定
- `SettingsMainActivity.kt` - 設定主頁
- `LanguageManager.kt` - 多語言管理

### 資源 (`assets/`)
- `dictionary.db` - SQLite 字典
- `dictionary.trie` - MARISA-trie 索引
- `ime/text/characters/qwerty_{poj,tl}.json` - 鍵盤佈局

---

## 資料流

```
輸入 → ComposingManager (rawInput/composingText)
         ↓
     TaigiAutocompleteService.getSuggestions(rawInput, displayText)
         ↓
     LexiconService.search() → InputNormalizer → TrieService → SQLite
         ↓
     SmartbarManager.updateCandidates()
```

---

## 跨平台檔案對應

| 功能 | iOS | Android |
|------|-----|---------|
| **IME 主入口** | `KeyboardViewController.swift` | `TaigiKeyboard.kt` |
| **組字管理** | `ComposingManager.swift` | `ComposingManager.kt` |
| **自動完成** | `AutocompleteService.swift` | `TaigiAutocompleteService.kt` |
| **詞典查詢** | `LexiconService.swift` | `LexiconService.kt` |
| **Trie 服務** | `TrieService.swift` | `TrieService.kt` |
| **輸入正規化** | `InputNormalizer.swift` | `InputNormalizer.kt` |
| **聲調轉換** | `ToneConverter.swift` | `ToneConverter.kt` |
| **使用者頻率** | `UserFrequencyService.swift` | `UserFrequencyService.kt` |
| **SQLite 查詢** | `DictionaryRepository.swift` | `LexiconService.kt` |
| **設定儲存** | `SharedSettings.swift` | `PrefHelper.kt` |
| **詞庫管理 UI** | `DictionarySettingsView.swift` | `DictionarySettingsActivity.kt` |
| **齒盤設定 UI** | `SettingsView.swift` | `KeyboardSettingsActivity.kt` |
| **候選詞視圖** | `CandidateView.swift` | `SmartbarView.kt` |

---

## 關鍵設計

1. **雙狀態組字**：`rawInput`（Trie 搜尋用）、`composingText`（UI 顯示用）
2. **Trie + SQLite**：Trie 前綴匹配取 rowid → SQLite 批次查詢
3. **聲調處理**：輸入數字即時轉調符顯示，搜尋用原始數字
4. **詞庫開關**：七大詞庫（教育部、新詞、工藝、iTaigi、台日、台華、植物）可獨立啟用/停用
