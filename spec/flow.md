# iOS 資料流

> **類型**: 功能
> **關鍵字**: `DataFlow`, `Pipeline`, `ActionHandler`
> **相關**: composing.md, autocomplete.md

---

## 重點摘要

- 輸入 → 組字 → 搜尋 → 顯示 → 選擇 → 輸出
- ActionHandler 負責事件分發
- ComposingManager 維護雙狀態

---

## 整體流程

```
使用者輸入
    ↓
ActionHandler（事件分發）
    ↓
ComposingManager（rawInput / composingText）
    ↓
AutocompleteService（候選詞搜尋）
    ↓
CandidateView（候選詞顯示）
    ↓
使用者選擇
    ↓
文字輸出
```

---

## 階段 1：輸入處理

### ActionHandler

| 方法 | 說明 |
|------|------|
| `handle(gesture:on:)` | 主入口，分發按鍵事件 |
| `handleCharacterInput(_:)` | 字元輸入處理 |
| `handleTaigiSpecificAction(_:)` | 台語專用動作 |

### 字元輸入流程

1. 大小寫轉換（根據 keyboardCase）
2. 標點符號：先確認組字，再直接輸出
3. 字母/數字：進入組字流程

---

## 階段 2：組字管理

### ComposingManager 狀態

| 狀態 | rawInput | composingText |
|------|----------|---------------|
| idle | "" | "" |
| composing | "gua2" | "guá" |

### 狀態變更觸發

- `composing` → 觸發 `performAutocomplete()`
- `idle` → 清除 markedText

---

## 階段 3：候選詞搜尋

### AutocompleteService

1. 取得 `rawInput`（搜尋用）
2. 判斷輸入類型
3. 呼叫 `LexiconService.search()`
4. 插入 composingText 到第 0 位

### LexiconService.search()

1. InputNormalizer 正規化
2. TrieService 前綴搜尋
3. SQLite 批次查詢
4. UserFrequency 排序

---

## 階段 4：候選詞顯示

### TaigiKeyboardView

- 根據 keyboardCase 轉換候選詞大小寫
- 傳遞給 CandidateView

### CandidateView

- 水平捲動顯示候選詞
- 點擊觸發 `onSuggestionTap`

---

## 階段 5：候選詞選擇

### ActionHandler+Suggestions

1. 解析羅馬字與漢字
2. 決定輸出文字（根據設定）
3. 呼叫 `composingManager.selectSuggestion()`
4. 記錄使用頻率
5. 觸發 NextWord 預測

### ComposingManager.selectSuggestion()

1. 清除 markedText
2. 插入選中文字
3. 重置狀態為 idle

---

## 狀態流轉圖

```
[Idle] ─── 輸入字元 ───→ [Composing]
   ↑                          │
   │                          ├── 選擇候選詞
   │                          ├── 空白鍵確認
   │                          ├── Enter 確認
   └──────────────────────────┴── 刪除至空 → [Idle]
```

---

## 關鍵檔案

| 階段 | 檔案 |
|------|------|
| 輸入 | `ActionHandler.swift`, `ActionHandler+CharacterInput.swift` |
| 組字 | `ComposingManager.swift` |
| 搜尋 | `AutocompleteService.swift`, `LexiconService.swift` |
| 顯示 | `TaigiKeyboardView.swift`, `CandidateView.swift` |
| 選擇 | `ActionHandler+Suggestions.swift` |
