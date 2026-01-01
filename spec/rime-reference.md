# librime 參考研究

> **類型**: 參考
> **關鍵字**: `librime`, `RIME`, `Pipeline`, `DAG`, `SpellingAlgebra`
> **相關**: khiin-reference.md, azookey-reference.md

---

## 重點摘要

- 模組化 Pipeline 架構：Processor → Segmentor → Translator → Filter
- DAG 分詞演算法處理連續輸入
- Spelling Algebra 規則式拼寫變體
- 動態權重 + 時間衰減的使用者學習
- YAML 配置驅動設計

---

## 架構概覽

### Pipeline 處理流程

```
按鍵輸入
    ↓
Processors   處理按鍵事件（Speller, Selector, Punctuator）
    ↓
Segmentors   分割輸入（AbcSegmentor, PunctSegmentor）
    ↓
Translators  產生候選詞（ScriptTranslator, TableTranslator）
    ↓
Filters      過濾/排序（Uniquifier, Simplifier）
    ↓
候選列表
```

### 核心類別

| 類別 | 職責 |
|------|------|
| Engine | 編排器，管理 Pipeline 組件 |
| Context | 上下文，存放輸入、候選、狀態 |
| Schema | 配置，定義輸入法行為 |

---

## 使用者字典與排序

### 資料結構

```cpp
struct UserDbValue {
    int commits = 0;      // 提交次數
    double dee = 0.0;     // 動態權重因子
    TickCount tick = 0;   // 時間戳記
};
```

### 動態權重演算法

```cpp
// 時間衰減公式
inline double formula_d(double d, double t, double da, double ta) {
    return d + da * exp((ta - t) / 200);
}
```

### 候選排序

| 優先順序 | 說明 |
|---------|------|
| 1 | 精確匹配 > 預測匹配 |
| 2 | 使用者詞彙 +0.5 加分 |
| 3 | 完整輸入 > 自動完成（-1 懲罰）|
| 4 | 按 quality 值排序 |

### 儲存

- LevelDB key-value 資料庫
- Key: `{code}\t{phrase}`
- Value: `c={commits} d={dee} t={tick}`

---

## 配置系統

### Schema 結構

```yaml
schema:
  schema_id: luna_pinyin
  name: 朙月拼音

engine:
  processors: [speller, selector, punctuator]
  segmentors: [abc_segmentor, punct_segmentor]
  translators: [script_translator]
  filters: [uniquifier]

speller:
  alphabet: zyxwvutsrqponmlkjihgfedcba
  algebra:
    - derive/^([nl])ve$/$1ue/
```

### 配置指令

| 指令 | 功能 |
|------|------|
| `__include` | 引入其他配置 |
| `__patch` | 修補配置 |
| `__append` | 追加到列表 |
| `__merge` | 合併樹狀結構 |

### 使用者自訂

`.custom.yaml` 檔案覆蓋原始配置：

```yaml
# luna_pinyin.custom.yaml
patch:
  menu/page_size: 9
  speller/algebra/+:
    - derive/^([jqxy])u/$1v/
```

---

## 拼寫與分詞

### Spelling Algebra

| 類型 | 語法 | 功能 | 懲罰 |
|------|------|------|------|
| `xform` | `xform/pattern/replacement/` | 正則變換 | 無 |
| `derive` | `derive/pattern/replacement/` | 衍生變體 | 無 |
| `fuzz` | `fuzz/pattern/replacement/` | 模糊音 | -0.693 |
| `abbrev` | `abbrev/pattern/replacement/` | 縮寫 | -0.693 |
| `erase` | `erase/pattern/` | 刪除 | - |

### DAG 分詞

輸入 `xian` 的分詞圖：

```
位置:  0 ──── 1 ──── 2 ──── 3 ──── 4
       └──xi──┴──an──┘
       └─────────xian──────────────┘
```

可能分割：`xi + an`（西安）或 `xian`（先）

### 懲罰值

| 類型 | 懲罰值 |
|------|--------|
| 縮寫/模糊音 | -0.693 |
| 自動完成 | -0.693 |
| 糾錯 | -4.605 |
| 歧義 | -23.026 |

---

## 其他功能

| 功能 | 說明 |
|------|------|
| OpenCC | 簡繁轉換整合 |
| 反查 | 用一種輸入法查詢另一種編碼 |
| 標點 | 配置驅動的標點處理 |
| 並擊 | 同時按多鍵產生輸入 |
| 造句 | Poet + Grammar 組合最佳句子 |
| 歷史 | 記錄最近輸入詞彙 |

---

## 台語鍵盤應用建議

### P0 - 立即可用

**1. 時間衰減權重**
```kotlin
fun calculateWeight(count: Int, lastUsedMs: Long): Double {
    val ageHours = (System.currentTimeMillis() - lastUsedMs) / 3600000.0
    val decay = exp(-ageHours / 168.0)  // 一週半衰期
    return count * decay
}
```

**2. 使用者詞彙固定加分**
```kotlin
// 改為加法而非乘法
val finalScore = dictScore + (if (isUserPhrase) USER_BONUS else 0)
```

### P1 - 短期實作

| 功能 | 說明 |
|------|------|
| 歷史記錄 | 記錄最近 N 個詞彙，快捷叫出 |
| 撤銷輸入 | 3 秒內可撤銷錯誤選詞 |
| 反查 | 漢字查羅馬字，輔助學習 |

### P2 - 中期規劃

| 功能 | 說明 |
|------|------|
| POJ/TL 統一 | 任一拼寫都能找到詞彙 |
| 模糊音 | 處理 n/l、in/ing 等混淆 |
| 連續分詞 | 支援不加空白的連續輸入 |

### P3 - 長期參考

| 功能 | 說明 |
|------|------|
| Pipeline | 將處理流程模組化 |
| 配置驅動 | 用 YAML 定義輸入法行為 |

---

## 實作優先順序

```
Phase 1（NextWord 優化）
├── 時間衰減權重
├── 使用者詞彙加分
└── 歷史記錄

Phase 2（輸入體驗）
├── 撤銷功能
├── 反查功能
└── POJ/TL 統一

Phase 3（進階功能）
├── 模糊音
└── 連續輸入分詞

Phase 4（架構升級）
├── Pipeline 架構
└── 配置驅動
```

---

## 與本專案比較

| 項目 | librime | TaigiKeyboard |
|------|---------|---------------|
| 儲存 | LevelDB | SQLite |
| 權重 | 動態權重 + 時間衰減 | count 計數 |
| 分詞 | DAG + Viterbi | 單詞搜尋 |
| 變體 | Spelling Algebra | 手動建多筆資料 |
| 擴展 | 外掛系統 | 直接修改程式碼 |

---

## librime 原始碼參考

| 目錄 | 內容 |
|------|------|
| `src/rime/algo/` | 演算法（dynamics, syllabifier, algebra） |
| `src/rime/dict/` | 字典（user_dictionary, prism, table） |
| `src/rime/gear/` | 組件（processors, translators, filters） |
| `src/rime/config/` | 配置（config_compiler, config_types） |
