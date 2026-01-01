# 開發待辦事項

> **類型**: 規劃
> **關鍵字**: `TODO`, `Roadmap`, `Task`
> **相關**: nextword.md

---

## 重點摘要

- iOS KeyboardKit 10 升級進度追蹤
- 待處理的已知問題
- 程式碼優化任務

---

## KeyboardKit 10 升級

### Phase 1-3: 完成 ✅

- 升級套件版本
- 修復 Breaking Changes
- Debug 驗證

### Phase 4: 程式碼優化（進行中）

| 任務 | 狀態 |
|------|------|
| Emoji 鍵盤高度問題 | 低優先度 |
| 移除冗餘程式碼 | 待處理 |
| 增加可讀性 | 待處理 |
| 大小寫處理 Review | 進行中 |

---

## 大小寫處理任務

### 待排查

- 「自動大寫」開關 bug
- 當 `isAutoCapitalizationEnabled = false` 時，手動 Shift/Caps Lock 應仍可使用

### 待 Review

| 檔案 | 內容 |
|------|------|
| `ActionHandler+CharacterInput.swift` | 字元輸入大小寫 |
| `ActionHandler+Utilities.swift` | transformCharacterForCase |
| `TextProcessor.swift` | 候選詞大小寫 |
| `ToneUtilities.swift` | 聲調字母轉換 |

### 待撰寫測試

- `transformCharacterForCase` 函數
- `ToneUtilities.uppercaseToneLetter` 函數
- 自動大寫開關行為
- 手動 Shift/Caps Lock 行為

---

## 參考資料

| 文件 | 內容 |
|------|------|
| `kk10.md` | KeyboardKit 10 API 變更 |
| `case.md` | 大小寫處理設計 |
