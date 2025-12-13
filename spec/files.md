# 台語鍵盤 - 檔案索引

## Android (`android/app/src/main/java/.../taigikeyboard/`)

### 核心
- `TaigiKeyboard.kt` - IME 主服務
- `TextInputManager.kt` - 文字輸入管理（按鍵處理、候選詞更新）
- `PrefHelper.kt` - 偏好設定 (DataStore)

### 組字與候選詞 (`ime/text/composing/`)
- `ComposingManager.kt` - 組字狀態（rawInput + composingText 雙狀態）
- `TaigiAutocompleteService.kt` - 候選詞搜尋入口
- `UserFrequencyService.kt` - 使用者頻率記錄

### 字典查詢 (`ime/dictionary/`)
- `LexiconService.kt` - Trie + SQLite 混合查詢
- `TrieService.kt` - MARISA-trie JNI 封裝
- `InputNormalizer.kt` - 輸入正規化（調符→數字）
- `ToneConverter.kt` - 聲調轉換（數字→調符）
- `ToneConverterModels.kt` - 聲調映射表

### 智慧列 (`ime/text/smartbar/`)
- `SmartbarManager.kt` - 候選詞顯示與選擇
- `SmartbarView.kt` - UI 容器
- `CandidateOverlayView.kt` - 候選詞浮層

### 按鍵系統
- `KeyCode.kt` - 按鍵代碼
- `KeyView.kt` - 按鍵視圖
- `LayoutManager.kt` - 佈局載入

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

## 關鍵設計

1. **雙狀態組字**：`rawInput`（Trie 搜尋用）、`composingText`（UI 顯示用）
2. **Trie + SQLite**：Trie 前綴匹配取 rowid → SQLite 批次查詢
3. **聲調處理**：輸入數字即時轉調符顯示，搜尋用原始數字
