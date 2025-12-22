# 台語鍵盤 - 技術規格

> **關鍵字**: `spec`, `規格`, `文件索引`

確保 iOS / Android 雙平台實作一致。

---

## 文件列表

| 檔案 | 功能代號 | 關鍵字 |
|------|----------|--------|
| `files.md` | — | `files`, `naming`, `structure`, `對應表` |
| `composing.md` | Composing | `Composing`, `rawInput`, `composingText`, `組字` |
| `tone.md` | Tone | `Tone`, `聲調`, `調符`, `ToneConverter` |
| `autocomplete.md` | Autocomplete | `Autocomplete`, `候選詞`, `getSuggestions` |
| `sort.md` | UserFrequency | `sort`, `排序`, `UserFrequency`, `詞頻` |
| `trie.md` | Trie, Lexicon | `Trie`, `MARISA`, `Lexicon`, `InputNormalizer` |
| `prediction.md` | NextWord | `NextWord`, `WordPrediction`, `聯想詞`, `Bigram` |
| `theme.md` | Theme | `Theme`, `主題`, `樣式` |
| `log.md` | — | `log`, `debug`, `除錯` |
| `khiin.md` | — | `khiin`, `起引`, `參考`, `Trie`, `Bigram` |
| `rime.md` | — | `rime`, `librime`, `參考`, `Grammar`, `Context` |
| `librime-predict.md` | — | `librime`, `predict`, `NextWord`, `DoubleArray` |
| `todo.md` | — | `todo`, `待辦`, `規劃` |

---

## 使用方式

1. **新增功能前** - 先查閱相關 spec，確認雙平台邏輯
2. **修改一側後** - 更新 spec，確保另一側同步
3. **發現差異時** - 在 spec 記錄，討論是否對齊

---

## 維護原則

- 繁體中文撰寫
- 每份文件頂部標註**功能代號**和**關鍵字**
- 程式碼範例標註來源檔案（如 `// iOS: XxxService.swift`）
- 檔案路徑使用相對路徑（如 `Lexicon/Services/`）
- 保持文件與程式碼同步
- 記錄「為什麼」而非「是什麼」

---

## 功能模組對應

詳見 `files.md` 的命名規則與跨平台對應表。

| 功能代號 | 說明 |
|----------|------|
| **Composing** | 組字管理（rawInput / composingText） |
| **Autocomplete** | 自動完成服務 |
| **Lexicon** | 字典查詢 |
| **Trie** | MARISA-trie 索引 |
| **Tone** | 聲調處理 |
| **UserFrequency** | 使用者頻率與排序 |
| **NextWord** | 下一詞預測（規劃中） |
| **Smartbar** | 候選詞列 UI |
| **Settings** | 設定 |
| **Theme** | 主題 |
