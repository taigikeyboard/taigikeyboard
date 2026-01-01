# Layout 鍵盤佈局

> **類型**: 功能
> **關鍵字**: `Layout`, `KeyDef`, `LayoutConverter`, `CustomLayoutService`
> **相關**: files.md

---

## 重點摘要

- 完整佈局定義，一眼看到全貌
- 全形/半形在按鍵定義處直接指定
- 裝置差異：`_iPhone` vs `_withGlobe`

---

## 檔案結構

| 檔案 | 說明 |
|------|------|
| `KeyDef.swift` | 按鍵定義 enum |
| `TaigiLayouts.swift` | 所有鍵盤佈局 |
| `LayoutConverter.swift` | 轉換為 KeyboardLayout |
| `CustomLayoutService.swift` | 選擇佈局邏輯 |

---

## 設計原則

1. **完整佈局** - 每個鍵盤是完整 `[[KeyDef]]`
2. **全形/半形內嵌** - 按鍵處指定 `fullWidth`
3. **裝置差異明確** - iPhone 無 globe / 有 globe
4. **無重複符號** - Numeric、Symbolic 分開
5. **人體工學** - 常用標點放第 4 列

---

## 命名規則

| 後綴 | 說明 | 適用 |
|------|------|------|
| `_iPhone` | 無 globe 鍵 | 一般 iPhone |
| `_withGlobe` | 有 globe 鍵 | iPhone SE、iPad |

---

## 佈局類型

### Alphabetic（字母）

| 佈局 | 特點 |
|------|------|
| PhahTaigi | `!` `?` 取代 `q` `w`，`,` `.` 取代 `z` `x` |
| QWERTY TL | 標準 QWERTY |
| QWERTY POJ | 多一個 `o͘` 鍵 |

### Numeric（常用符號）

- Row 1: 數字
- Row 2: 引號/括號
- Row 3: 標點
- Row 4: 最常用（手指位置）

### Symbolic（進階符號）

- Row 1: 程式括號
- Row 2: 書名號
- Row 3: 貨幣/特殊
- Row 4: 數學符號

---

## 全形/半形

| 設定 | 顯示 | 用途 |
|------|------|------|
| 半形（預設） | `. , ? !` | 全羅文章 |
| 全形 | `。，？！` | 漢羅文章 |

### 永遠半形

- 程式符號：`[ ] { } ( ) < >`
- 技術符號：`_ \ / @ = $ % # &`
- 貨幣：`€ £ ¥ ¢`
- 數學：`± × ÷ ≠ ≈ ∞ √`

---

## 平台對照

| 項目 | iOS | Android |
|------|-----|---------|
| 佈局定義 | `TaigiLayouts.swift` | `LayoutData.kt` |
| 轉換器 | `LayoutConverter.swift` | `LayoutManager.kt` |
| 按鍵定義 | `KeyDef.swift` | `KeyData.kt` |
