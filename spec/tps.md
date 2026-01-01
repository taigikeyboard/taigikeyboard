# TPS 台灣方音符號

> **類型**: 功能
> **關鍵字**: `TPS`, `TPSConverter`, `方音符號`, `台灣注音`
> **相關**: layout.md, tone.md, trie.md

---

## 重點摘要

- TPS（Taiwanese Phonetic Symbols）台灣方音符號輸入
- 輸入方音符號，自動轉換為 TL 進行 Trie 搜尋
- 基於 QWERTY 風格的鍵盤佈局

---

## 檔案結構

| 檔案 | 說明 |
|------|------|
| `Input/TPSConverter.swift` | TPS ↔ TL 雙向轉換器 |
| `Layout/TaigiLayouts.swift` | TPS 佈局定義 |
| `Settings/SharedSettings.swift` | `.tps` 佈局類型 |
| `Layout/CustomLayoutService.swift` | TPS 佈局選擇 |
| `Lexicon/Trie/InputNormalizer.swift` | TPS 輸入正規化 |
| `Autocomplete/Views/CandidateCellHelper.swift` | 候選詞 TPS 顯示 |

---

## 轉換流程

```
使用者輸入 → TPS 檢測 → TPSConverter.toTL() → InputNormalizer → Trie 搜尋
     ↓              ↓                ↓                ↓              ↓
  ㄉㄧㄠˊ    →   是 TPS    →      tiau5       →     tiau5    →  找到詞彙
```

---

## 聲母對照表

| TPS | TL | 說明 |
|-----|-----|------|
| ㄅ | p | 雙唇不送氣清塞音 |
| ㄆ | ph | 雙唇送氣清塞音 |
| ㄇ | m | 雙唇鼻音 |
| ㆠ | b | 雙唇濁塞音 |
| ㄉ | t | 舌尖不送氣清塞音 |
| ㄊ | th | 舌尖送氣清塞音 |
| ㄋ | n | 舌尖鼻音 |
| ㄌ | l | 舌尖邊音 |
| ㄍ | k | 舌根不送氣清塞音 |
| ㄎ | kh | 舌根送氣清塞音 |
| ㄫ | ng | 舌根鼻音 |
| ㆣ | g | 舌根濁塞音 |
| ㄏ | h | 喉擦音 |
| ㄗ | ts | 舌尖不送氣清塞擦音 |
| ㄘ | tsh | 舌尖送氣清塞擦音 |
| ㄙ | s | 舌尖擦音 |
| ㆡ | j | 舌尖濁塞擦音 |

### 複合聲母

| TPS | TL | 說明 |
|-----|-----|------|
| ㄐㄧ | tsi | 齒齦不送氣（接 i） |
| ㄑㄧ | tshi | 齒齦送氣（接 i） |
| ㄒㄧ | si | 齒齦擦音（接 i） |
| ㆢㄧ | ji | 齒齦濁（接 i） |

---

## 韻母對照表

### 單韻母

| TPS | TL |
|-----|-----|
| ㄚ | a |
| ㆤ | e |
| ㄧ | i |
| ㄛ | o |
| ㄨ | u |
| ㆦ | oo |

### 複合韻母

| TPS | TL |
|-----|-----|
| ㄞ | ai |
| ㄠ | au |
| ㄢ | an |
| ㄤ | ang |
| ㆲ | ong |
| ㆰ | am |
| ㆱ | om |

### 鼻韻母

| TPS | TL |
|-----|-----|
| ㆬ | m |
| ㄥ | ng |
| ㆭ | ng |
| ㄣ | n |

### 鼻化韻母

| TPS | TL |
|-----|-----|
| ㆩ | ann |
| ㆪ | inn |
| ㆥ | enn |
| ㆧ | onn |
| ㆫ | unn |
| ㆮ | ainn |
| ㆯ | aunn |

---

## 聲調對照表

### 一般聲調

| TPS | TL | 說明 |
|-----|-----|------|
| (無) | 1 | 陰平 |
| ˋ | 2 | 陰上 |
| ˪ | 3 | 陰去 |
| ˊ | 5 | 陽平 |
| ˫ | 7 | 陽去 |

### 入聲韻尾

| TPS | TL | 說明 |
|-----|-----|------|
| ㆴ | p + 4 | 陰入 -p |
| ㆵ | t + 4 | 陰入 -t |
| ㆶ | k + 4 | 陰入 -k |
| ㆷ | h + 4 | 陰入 -h |
| ㆴ̇ | p + 8 | 陽入 -p |
| ㆵ̇ | t + 8 | 陽入 -t |
| ㆶ̇ | k + 8 | 陽入 -k |
| ㆷ̇ | h + 8 | 陽入 -h |

---

## 鍵盤佈局

```
Row 1 (聲調):  [ˊ] [ˋ] [˪] [˫] [ㆴ] [ㆵ] [ㆶ] [ㆷ] [ㆦ] [ㆤ]
Row 2 (聲母):  [ㄅ] [ㄉ] [ㆣ] [ㄍ] [ㄎ] [ㄆ] [ㄊ] [ㄗ] [ㆡ] [ㄫ]
Row 3 (混合):  [ㄇ] [ㄚ] [ㄨ] [ㄏ] [ㄌ] [ㄘ] [ㄧ] [ㆠ] [ㄢ]
Row 4 (韻母):  [⬆]  [ㄛ] [ㄙ] [ㆬ] [ㄋ] [ㄥ] [ㄤ] [ㄞ] [⌫]
Row 5 (功能):  [123] [😀] [     空白     ] [翻譯] [⏎]
```

### 佈局變體

| 後綴 | 說明 |
|------|------|
| `tps_iPhone` | 無 globe 鍵 |
| `tps_withGlobe` | 有 globe 鍵（iPhone SE、iPad） |

---

## 核心 API

### TPSConverter

```swift
// 檢測是否包含 TPS 字符
TPSConverter.containsTPS("ㄉㄧㄠˊ") // true

// TPS → TL 轉換（用於 Trie 搜尋）
TPSConverter.toTL("ㄉㄧㄠˊ") // "tiau5"

// TL → TPS 轉換（用於候選詞顯示）
TPSConverter.toTPS("tiau5") // "ㄉㄧㄠˊ"

// 多音節轉換
TPSConverter.toTLMultiSyllable("ㄉㄧㄠ ㄙㄨˊ") // "tiau su5"
```

### InputNormalizer 整合

```swift
// 自動檢測並轉換 TPS
InputNormalizer.normalize("ㄉㄧㄠˊ", mode: .tl) // "tiau5"

// 混合輸入也支援
InputNormalizer.needsNormalization("ㄅㄚ") // true
```

---

## 候選詞顯示與輸出

TPS 模式下，候選詞顯示方音符號而非羅馬字，點擊後也輸出方音符號。

### 顯示邏輯

| isTranslateSwapped | 主標題 | 副標題 |
|-------------------|--------|--------|
| false（預設） | 方音符號 | 漢字 |
| true | 漢字 | 方音符號 |

### 輸出邏輯

| isTranslateSwapped | 點擊候選詞輸出 |
|-------------------|---------------|
| false（預設） | 方音符號 |
| true | 漢字 |

### 實作位置

`CandidateCellHelper.swift`：
- `displayTitle()` - 計算主標題
- `displaySubtitle()` - 計算副標題
- `suggestionToHandle()` - 計算點擊輸出

### 轉換範例

| TL（原始） | TPS（顯示/輸出） |
|-----------|-----------------|
| tiau5 | ㄉㄧㄠˊ |
| su5 | ㄙㄨˊ |
| peh8 | ㄅㆤㆷ̇ |
| kau2 | ㄍㄠˋ |
| gua2-gua2 | ㄍㄨㄚˋ ㄍㄨㄚˋ |

### 連字符處理

羅馬字連字符 `-` 轉換為 TPS 空白分隔：

```
gua2-gua2  →  ㄍㄨㄚˋ ㄍㄨㄚˋ
tsi̍t-pái  →  ㄐㄧㆵ̇ ㄅㄞˋ
```

---

## 字體大小調整

方音符號視覺上比羅馬字大，TPS 模式下字體縮小 15%。

### 縮放比例

| 類型 | 縮放比例 |
|------|----------|
| 一般候選詞 | 0.85x |
| 長詞候選詞 | 0.85x |

### 實作位置

`CandidateViewModels.swift`：
- `tpsScale = 0.85` - TPS 縮放比例
- `tpsPrimaryFontSize` - TPS 主標題字體
- `tpsSecondaryFontSize` - TPS 副標題字體
- `tpsLongCellPrimaryFontSize` - TPS 長詞主標題字體
- `tpsLongCellSecondaryFontSize` - TPS 長詞副標題字體

`CandidateCellHelper.swift`：
- `titleFontSize(isTranslateSwapped:)` - 一般候選詞主標題
- `subtitleFontSize(isTranslateSwapped:)` - 一般候選詞副標題
- `longCellTitleFontSize(isTranslateSwapped:)` - 長詞主標題
- `longCellSubtitleFontSize(isTranslateSwapped:)` - 長詞副標題

### 縮放邏輯

| isTranslateSwapped | 主標題縮放 | 副標題縮放 |
|-------------------|-----------|-----------|
| false（預設） | 是（顯示方音符號） | 否（顯示漢字） |
| true | 否（顯示漢字） | 是（顯示方音符號） |

---

## 測試案例

| TPS 輸入 | TL 輸出 | 說明 |
|----------|---------|------|
| ㄉㄧㄠˊ | tiau5 | 聲母+韻母+聲調 |
| ㄙㄨˊㄅㆤㆶ̇ | su5pek8 | 多音節+入聲 |
| ㄍㄠˋㄏㄧㆲˊ | kau2hiong5 | 複合韻母 |
| ㄅㄚ | pa | 無聲調 |
| ㆴ | p4 | 單獨入聲韻尾 |

---

## 參考資料

- [Tailo-TPS-Converter](https://github.com/leechunhoe/Tailo-TPS-Converter)
- [教育部臺灣台語常用詞辭典](https://sutian.moe.edu.tw/zh-hant/siannuntiau/)
