# librime 參考筆記

> **關鍵字**: `librime`, `RIME`, `參考`, `LevelDB`, `動態權重`, `時間衰減`, `Grammar`, `Context`
> **更新日期**：2025-12-20

---

## 文件索引

本文件為 librime 分析的主文件，聚焦於使用者字典與候選排序。其他功能分散於以下文件：

### 基礎分析
| 文件 | 內容 |
|------|------|
| [rime.md](rime.md)（本文件） | 使用者字典、頻率更新、候選排序、LevelDB 儲存 |
| [rime-config.md](rime-config.md) | Schema 配置系統、YAML DSL、Engine Pipeline、部署系統 |
| [rime-spelling.md](rime-spelling.md) | Spelling Algebra、分詞系統、Prism 索引、拼寫糾錯 |
| [rime-features.md](rime-features.md) | OpenCC、反查、標點、並擊、造句、歷史記錄 |
| [rime-future.md](rime-future.md) | 功能借鑑清單、實作建議 |

### 深入專題
| 文件 | 內容 |
|------|------|
| [rime-segmentation.md](rime-segmentation.md) | DAG 分詞演算法、Viterbi 最佳路徑、台語應用 |
| [rime-pipeline.md](rime-pipeline.md) | Pipeline 架構、組件註冊、Signal/Slot、台語應用 |
| [rime-config-detail.md](rime-config-detail.md) | 配置指令詳解、使用者自訂、Schema 切換 |

---

## 使用者字典與頻率更新

### 資料結構

librime 使用 `UserDbValue` 結構儲存使用者詞彙資料：

```cpp
struct UserDbValue {
    int commits = 0;      // 提交次數
    double dee = 0.0;     // 動態權重因子
    TickCount tick = 0;   // 時間戳記
};
```

儲存格式：`c={commits} d={dee} t={tick}`

### 動態權重演算法

**核心公式**（位於 `algo/dynamics.h`）：

```cpp
// 動態權重衰減公式
inline double formula_d(double d, double t, double da, double ta) {
    return d + da * exp((ta - t) / 200);
}

// 最終權重計算公式
inline double formula_p(double s, double u, double t, double d) {
    const double kM = 1 / (1 - exp(-0.005));
    double m = s - (s - u) * pow((1 - exp(-t / 10000)), 10);
    return (d < 20) ? m + (0.5 - m) * (d / kM)
                    : m + (1 - m) * (pow(4, (d / kM)) - 1) / 3;
}
```

**特點**：
- 指數時間衰減：舊詞彙權重隨時間降低
- TickCount 機制：全域時間戳，每次提交遞增
- 結合提交次數、時間戳、可信度計算最終權重

---

## 候選詞排序

### 排序邏輯

```cpp
// DictEntry 比較運算子
bool DictEntry::operator<(const DictEntry& other) const {
    if (weight != other.weight)
        return weight > other.weight;  // 權重大的排前面
    return 0;
}
```

### 品質計算

```cpp
phrase->set_quality(
    std::exp(e->weight) +              // 基礎權重
    options_->initial_quality() +       // 初始品質
    (incomplete ? -1 : 0) +            // 未完成輸入懲罰
    (is_user_phrase ? 0.5 : 0)         // 使用者詞彙加分
);
```

**排序優先順序**：
1. 精確匹配 > 預測匹配
2. 使用者詞彙 > 系統詞彙（+0.5 加分）
3. 完整輸入 > 自動完成（-1 懲罰）
4. 按 `quality` 值排序

---

## 資料庫儲存

### LevelDB

librime 使用 **Google LevelDB** 作為儲存引擎：
- Key-value 資料庫
- 支援範圍查詢（forward scan）
- 支援交易（transactions）

### 儲存格式

| 類型 | 格式 | 範例 |
|------|------|------|
| Key | `{code}\t{phrase}` | `a b c \t你好` |
| Value | `c={commits} d={dee} t={tick}` | `c=10 d=5.234 t=12345` |

### Metadata

- `/tick`: 全域時間戳
- `/user_id`: 使用者 ID
- `/db_type`: "userdb"

### 交易管理

```cpp
bool Memory::StartSession();           // 開始交易
bool Memory::FinishSession();          // 提交交易
bool Memory::DiscardSession();         // 撤銷交易（3 秒內）
```

---

## 詞彙關聯與上下文機制

> **更新日期**：2025-12-20

### 架構設計

librime 的詞彙關聯透過**抽象介面 + 外掛**實現：

```
                    ┌─────────────────┐
                    │   Grammar       │ ← 抽象介面
                    │   Query(ctx,w)  │
                    └────────┬────────┘
                             │
        ┌────────────────────┼────────────────────┐
        ▼                    ▼                    ▼
┌───────────────┐   ┌───────────────┐   ┌───────────────┐
│ librime-      │   │ librime-      │   │ 其他外掛      │
│ octagram      │   │ predict       │   │              │
│ (八元模型)    │   │ (下一詞預測)   │   │              │
└───────────────┘   └───────────────┘   └───────────────┘
```

### Grammar 介面

**檔案**：`src/rime/gear/grammar.h`

```cpp
class Grammar {
  // 查詢「上下文 + 當前詞」的關聯分數
  virtual double Query(const string& context,
                       const string& word,
                       bool is_rear) = 0;  // is_rear = 是否句尾
};

// 權重計算（加法模型）
static double Evaluate(...) {
    return entry_weight + grammar->Query(context, entry_text, is_rear);
}
```

### 上下文定義

**檔案**：`src/rime/gear/poet.cc`

```cpp
string context() const {
  // 回看 2 個詞作為上下文（類似 Trigram）
  return predecessor->last_word() + last_word();
}
```

### ContextualTranslation

**檔案**：`src/rime/gear/contextual_translation.cc`

根據 `preceding_text`（前文）重新計算候選詞權重：

```cpp
an<Phrase> ContextualTranslation::Evaluate(an<Phrase> phrase) {
  bool is_rear = phrase->end() == input_.length();
  double weight = Grammar::Evaluate(
      preceding_text_, phrase->text(),
      phrase->weight(), is_rear, grammar_
  );
  phrase->set_weight(weight);
  return phrase;
}
```

### 外掛支援

librime 透過外掛系統支援進階語言模型：

| 外掛 | 功能 |
|------|------|
| `librime-octagram` | 八元語言模型 |
| `librime-predict` | 下一詞預測 |
| `librime-lua` | Lua 腳本擴充 |

### 與 khiin-rs / 台語鍵盤的比較

| | rime | khiin-rs | 台語鍵盤 |
|--|------|----------|----------|
| **模型** | 外掛式（N-gram） | Bigram（SQL） | 首字→詞尾（SQL） |
| **上下文長度** | 2 詞（Trigram） | 1 詞（Bigram） | 1 字 |
| **權重計算** | 加法 | 乘法/排序 | 乘法 |
| **句尾處理** | `is_rear` 標記 | 無 | 無 |
| **語言模型** | 外部載入 | 內建 | 內建 |

### 值得參考的設計

**1. 上下文長度可調**
```kotlin
// 目前：只看 1 字（首字）
val firstChar = word.first().toString()

// 改進：可看前 N 個詞
fun getContext(words: List<String>, n: Int = 2): String {
    return words.takeLast(n).joinToString("")
}
```

**2. 加法權重（更可預測）**
```kotlin
// 目前：乘法（USER_WEIGHT = 50）
score = count * USER_WEIGHT

// rime 風格：加法
const val USER_BONUS = 0.5
score = dictScore + (if (isUserPhrase) USER_BONUS else 0)
```

**3. 句尾處理**
```kotlin
// 句尾候選詞可能需要不同的排序邏輯
fun predict(word: String, isEndOfSentence: Boolean): List<Prediction>
```

### 關鍵發現

**rime 核心不包含具體的 N-gram 實作**，而是：
- 定義 Grammar 抽象介面
- 透過外掛提供具體實作

對台語鍵盤來說：
- **SQLite 存 association 是正確的**（khiin-rs 和 rime 的使用者學習都是類似方式）
- 可參考的是上下文長度、加法權重、抽象介面設計

---

## 架構設計

### 模組化架構

```
Engine
├── Processors (處理按鍵事件)
├── Segmentors (分詞)
├── Translators (產生候選詞)
│   ├── TableTranslator
│   ├── ScriptTranslator
│   └── ...
└── Filters (過濾候選詞)
```

### Memory 機制

`Memory` 類別整合：
- Dictionary（系統字典）
- UserDictionary（使用者字典）
- Language（語言模型）

---

## 與 TaigiKeyboard 的比較

| 項目 | librime | TaigiKeyboard（目前） |
|------|---------|----------------------|
| **儲存引擎** | LevelDB (key-value) | SQLite |
| **權重模型** | 動態權重 + 時間衰減 | 固定 count 計數 |
| **使用者學習** | UserDbValue (commits, dee, tick) | count + last_used |
| **排序** | quality = exp(weight) + bonus | score = count × weight |
| **交易支援** | 有（可撤銷） | 無 |
| **架構** | 外掛式模組化 | 單一服務 |

### 值得借鑑

1. **時間衰減**：舊詞彙權重降低，讓近期使用的詞彙優先
2. **動態權重公式**：更精準的頻率計算
3. **交易支援**：使用者可撤銷錯誤輸入
4. **使用者加分機制**：使用者詞彙固定加 0.5

### 實作建議

```kotlin
// 簡化版時間衰減公式
fun calculateWeight(count: Int, lastUsedMs: Long): Double {
    val ageHours = (System.currentTimeMillis() - lastUsedMs) / 3600000.0
    val decay = exp(-ageHours / 168)  // 一週半衰期
    return count * decay
}
```

---

## 相關檔案

- `src/rime/dict/user_dictionary.{h,cc}` - 使用者字典
- `src/rime/algo/dynamics.h` - 權重演算法
- `src/rime/dict/vocabulary.{h,cc}` - 排序邏輯
- `src/rime/dict/level_db.{h,cc}` - LevelDB 實作
- `src/rime/gear/memory.{h,cc}` - 記憶體管理
