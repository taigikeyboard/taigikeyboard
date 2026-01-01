# Autocomplete 自動完成

> **類型**: 功能
> **關鍵字**: `Autocomplete`, `Suggestion`, `Candidate`, `AutocompleteService`
> **相關**: composing.md, sort.md, trie.md

---

## 重點摘要

- 根據 `rawInput` 搜尋候選詞
- 第 0 個位置固定放 `composingText`（當前組字）
- 判斷輸入類型決定搜尋策略

---

## 核心流程

```
rawInput → determineInputType → LexiconService.search → 排序 → 候選詞列表
```

1. 從 ComposingManager 取得 `rawInput`
2. 判斷輸入類型（漢字/帶聲調/無聲調）
3. 搜尋詞典
4. 插入 composingText 到第 0 位
5. 回傳候選詞列表

---

## 輸入類型判斷

| 輸入 | 類型 | 說明 |
|------|------|------|
| `我` | `Hanzi` | 包含漢字 |
| `guá` | `RomanWithTone` | 包含調符 |
| `gua2` | `RomanWithTone` | 包含數字聲調 (2,3,5,6,7,8,9) |
| `gua` | `RomanWithoutTone` | 無聲調 |
| `gua1` | `RomanWithoutTone` | 聲調 1 不視為有聲調 |
| `gua4` | `RomanWithoutTone` | 聲調 4 不視為有聲調 |

### containsNumericTone 規則

- 排除 0, 1, 4
- 原因：聲調 1/4 在台語不加調號，視為無聲調搜尋

---

## 候選詞結構

```
索引 0: composingText（如 guá）← 固定位置
索引 1: 候選詞 1（如 我）
索引 2: 候選詞 2（如 瓜）
...
```

### iOS additionalInfo 常用鍵值

| Key | 說明 |
|-----|------|
| `isComposingText` | 標記為組字候選詞 |
| `isNextWord` | 標記為 NextWord 預測 |
| `displayText` | 用於記錄使用頻率 |

---

## 平台對照

| 項目 | iOS | Android |
|------|-----|---------|
| 服務 | `AutocompleteService.swift` | `TaigiAutocompleteService.kt` |
| 搜尋 | `LexiconService.swift` | `LexiconService.kt` |
| 觸發 | `autocompleteText` 屬性 | `getSuggestions()` 直接呼叫 |

---

## 注意事項

### iOS autocompleteText 必須用 rawInput

- KeyboardKit 根據 `autocompleteText` 變化決定是否觸發
- 問題：輸入 `soo1` 時 composingText 無變化（聲調 1 不加調號）
- 解法：回傳 `rawInput` 而非 `composingText`

---

## 測試案例

| rawInput | composingText | 候選詞 |
|----------|---------------|--------|
| `gua2` | `guá` | [guá, 我, 瓜, ...] |
| `soo1` | `soo1` | [soo1, 所, 鎖, ...] |
| `soo` | `soo` | [soo, 所, 鎖, 蘇, ...] |
