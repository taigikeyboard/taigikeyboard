# 台語鍵盤 - 技術規格

> **類型**: 索引
> **關鍵字**: `spec`, `index`, `規格`

---

## 重點摘要

- 確保 iOS / Android 雙平台實作一致
- Claude Code 快速查閱用，條列式、精簡

---

## 文件索引

### 核心功能

| 檔案 | 關鍵字 | 說明 |
|------|--------|------|
| `composing.md` | `Composing`, `rawInput`, `composingText` | 組字狀態管理 |
| `autocomplete.md` | `Autocomplete`, `Suggestion`, `Candidate` | 候選詞搜尋 |
| `tone.md` | `Tone`, `ToneConverter`, `ToneMappings` | 聲調轉換 |
| `sort.md` | `UserFrequency`, `Sort`, `Score` | 候選詞排序 |
| `trie.md` | `Trie`, `Lexicon`, `InputNormalizer` | 字典索引 |
| `flow.md` | `DataFlow`, `Pipeline` | iOS 資料流 |
| `layout.md` | `Layout`, `KeyDef`, `LayoutConverter` | 鍵盤佈局 |

### 結構索引

| 檔案 | 關鍵字 | 說明 |
|------|--------|------|
| `files.md` | `Files`, `Structure`, `Mapping` | 檔案對應表 |

### 框架指南

| 檔案 | 關鍵字 | 說明 |
|------|--------|------|
| `kk10.md` | `KeyboardKit`, `KK10`, `Migration` | KeyboardKit 10 升級 |

### UI 相關

| 檔案 | 關鍵字 | 說明 |
|------|--------|------|
| `app-ui.md` | `App`, `Tab`, `Settings` | 主 App UI 結構 |
| `theme.md` | `Theme`, `Color`, `Style` | 主題設計 |
| `case.md` | `Case`, `Shift`, `CapsLock` | 大小寫處理 |
| `device.md` | `Device`, `iPhone`, `iPad` | 裝置適配 |

### 參考研究

| 檔案 | 關鍵字 | 說明 |
|------|--------|------|
| `rime-reference.md` | `RIME`, `librime`, `Grammar` | RIME 輸入法參考 |
| `khiin-reference.md` | `Khiin`, `起引`, `Trie` | 起引輸入法參考 |
| `azookey-reference.md` | `azooKey`, `Flickr` | azooKey 參考 |

### 規劃文件

| 檔案 | 關鍵字 | 說明 |
|------|--------|------|
| `nextword.md` | `NextWord`, `Prediction`, `Bigram` | 下一詞預測 |
| `todo.md` | `Todo`, `Task` | 待辦事項 |

---

## 格式規範

### 文件結構

```markdown
# [標題]

> **類型**: [功能|參考|規劃|索引]
> **關鍵字**: `keyword1`, `keyword2`
> **相關**: file1.md, file2.md

---

## 重點摘要
- 1-3 句核心用途

## 核心概念
- 條列式重點

## 平台對照
| 項目 | iOS | Android |

## 相關檔案
| 檔案 | 說明 |

## 注意事項
- 關鍵決策或陷阱
```

### 撰寫原則

- 條列式為主，避免大段文字
- 關鍵字統一用英文
- 程式碼只放關鍵片段（< 10 行）
- 保持精簡，減少 token 消耗

---

## 功能模組代號

| 代號 | 說明 | iOS 入口 | Android 入口 |
|------|------|----------|--------------|
| `Composing` | 組字管理 | `ComposingManager.swift` | `ComposingManager.kt` |
| `Autocomplete` | 自動完成 | `AutocompleteService.swift` | `TaigiAutocompleteService.kt` |
| `Lexicon` | 字典查詢 | `LexiconService.swift` | `LexiconService.kt` |
| `Trie` | Trie 索引 | `TrieService.swift` | `TrieService.kt` |
| `Tone` | 聲調處理 | `ToneConverter.swift` | `ToneConverter.kt` |
| `UserFrequency` | 使用者頻率 | `UserFrequencyService.swift` | `UserFrequencyService.kt` |
| `NextWord` | 下一詞預測 | `NextWordService.swift` | `NextWordService.kt` |
| `Layout` | 鍵盤佈局 | `CustomLayoutService.swift` | `LayoutManager.kt` |
| `Theme` | 主題 | `ThemeTokens.swift` | `Theme.kt` |
