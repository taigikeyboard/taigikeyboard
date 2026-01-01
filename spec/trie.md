# Trie 索引與 Lexicon 查詢

> **類型**: 功能
> **關鍵字**: `Trie`, `Lexicon`, `MARISA`, `InputNormalizer`, `TrieService`
> **相關**: autocomplete.md, sort.md

---

## 重點摘要

- MARISA-trie 存 key → SQLite rowid
- 前綴區分 TL/POJ：`tl:gua2`、`poj:goa2`
- InputNormalizer 只做調符→數字，不轉換拼法

---

## 架構

| 元件 | 內容 | 用途 |
|------|------|------|
| MARISA-trie | key → rowid | 前綴查詢 |
| SQLite | 完整詞條 | 資料儲存 |

---

## Trie Key 格式

| Key 類型 | 範例 | 說明 |
|----------|------|------|
| `tl:tl_num` | `tl:hoo2boo5` | TL 數字聲調 |
| `tl:tl_notone` | `tl:hooboo` | TL 無聲調 |
| `tl:tl_abbrev` | `tl:hb` | TL 縮寫 |
| `poj:poj_num` | `poj:ho2bo5` | POJ 數字聲調 |
| `poj:poj_notone` | `poj:hobo` | POJ 無聲調 |
| `poj:poj_abbrev` | `poj:hb` | POJ 縮寫 |

---

## 查詢流程

```
輸入 "goa2" (POJ 模式)
  → InputNormalizer.normalize() → "goa2"
  → 加前綴 → "poj:goa2"
  → TrieService.prefixSearch("poj:goa2")
  → rowid 列表
  → SQLite 批次查詢
  → 按 frequency 排序
```

---

## InputNormalizer

將使用者輸入正規化為 Trie key 格式。

### 處理步驟

1. 轉小寫
2. 去除連字符
3. 調符轉數字聲調（`hó` → `ho2`）

### 調符對照

| 標記 | Unicode | 聲調 |
|------|---------|------|
| ́ (acute) | U+0301 | 2 |
| ̀ (grave) | U+0300 | 3 |
| ̂ (circumflex) | U+0302 | 5 |
| ̌ (caron) | U+030C | 6 |
| ̄ (macron) | U+0304 | 7 |
| ̍ (vertical) | U+030D | 8 |
| ̆ (breve) | U+0306 | 9 |

### 注意

- 不做 POJ → TL 轉換（Trie 用前綴區分）
- 聲調數字放音節尾（`gáb` → `gab2`）

---

## 平台對照

| 項目 | iOS | Android |
|------|-----|---------|
| Trie 服務 | `TrieService.swift` | `TrieService.kt` |
| 正規化 | `InputNormalizer.swift` | `InputNormalizer.kt` |
| 字典服務 | `LexiconService.swift` | `LexiconService.kt` |
| JNI | marisa_bridge.cpp | trie_jni.cpp |

---

## Android JNI

### RecordTrie 格式

```
raw_key = utf8_key + \xff + uint32_le(rowid)
```

### 核心函數

| 函數 | 說明 |
|------|------|
| `nativeLoad(path)` | mmap 載入 trie |
| `nativePrefixSearch(prefix, limit)` | 前綴查詢 |
| `nativeLookup(key)` | 完全匹配 |

---

## 設計決策

| 決策 | 理由 |
|------|------|
| rowid 作為 value | 不需另存 global id |
| 數字聲調存 Trie | ASCII 相容 |
| POJ/TL 前綴區分 | 查詢時根據 InputMode 選擇 |
| 聲調/無聲調都存 | 查詢時不需額外過濾 |
| mmap 載入 | 節省記憶體 |

---

## 空間估算

- 30 萬筆 × 6 個 key ≈ 180 萬 key-value
- dictionary.trie ≈ 2.5-4 MB
- dictionary.db ≈ 15-20 MB

---

## Debug

```bash
adb logcat -s TrieService:D LexiconService:D
```

| 問題 | 檢查點 |
|------|--------|
| Trie 未初始化 | `isReady=false` |
| 正規化錯誤 | `[NORMALIZE]` 結果 |
| 查無結果 | lookup + prefixSearch 都為 0 |
