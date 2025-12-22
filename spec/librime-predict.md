# librime-predict 參考筆記

> **關鍵字**: `librime`, `predict`, `NextWord`, `DoubleArray`, `Trie`
> **更新日期**：2025-12-20
> **來源**：https://github.com/rime/librime-predict

---

## 概述

librime-predict 是 RIME 的下一詞預測外掛，功能類似台語鍵盤的 NextWord。

**核心特點**：
- 使用 **Darts DoubleArray Trie** 儲存預測資料（非 SQLite）
- 精確匹配查詢（非前綴搜尋）
- 支援連續預測（選詞後繼續預測下一詞）
- 可配置最大候選詞數和連續預測次數

---

## 架構

```
┌─────────────────────────────────────────────────────────┐
│                      RIME Engine                        │
├─────────────────────────────────────────────────────────┤
│  Predictor (Processor)                                  │
│  ├── 監聽 select_notifier（選詞事件）                    │
│  ├── 監聽 update_notifier（上下文更新）                  │
│  └── 處理 BackSpace/Escape（清除預測）                   │
├─────────────────────────────────────────────────────────┤
│  PredictTranslator (Translator)                         │
│  └── 產生預測候選詞                                      │
├─────────────────────────────────────────────────────────┤
│  PredictEngine                                          │
│  ├── Predict(context_query) - 查詢預測                   │
│  ├── Clear() - 清除快取                                  │
│  └── CreatePredictSegment() - 建立預測段落               │
├─────────────────────────────────────────────────────────┤
│  PredictDb (MappedFile)                                 │
│  ├── key_trie (DoubleArray) - 查詢索引                   │
│  ├── value_trie (StringTable) - 字串儲存                 │
│  └── Lookup(query) - 精確匹配查詢                        │
└─────────────────────────────────────────────────────────┘
```

---

## 資料庫結構

### Metadata

```cpp
struct Metadata {
  char format[32];              // "Rime::Predict/1.0"
  uint32_t db_checksum;
  OffsetPtr<char> key_trie;     // DoubleArray (query -> offset)
  uint32_t key_trie_size;
  OffsetPtr<char> value_trie;   // StringTable
  uint32_t value_trie_size;
};
```

### 查詢流程

```cpp
predict::Candidates* PredictDb::Lookup(const string& query) {
  // 精確匹配（不是前綴搜尋）
  int result = key_trie_->exactMatchSearch<int>(query.c_str());
  if (result == -1)
    return nullptr;
  return Find<predict::Candidates>(result);
}
```

### 與台語鍵盤的比較

| | librime-predict | 台語鍵盤 |
|--|-----------------|----------|
| **儲存** | DoubleArray Trie (mmap) | SQLite |
| **查詢** | 精確匹配 | SQL WHERE |
| **索引** | Trie | B-tree index |
| **更新** | 唯讀（預編譯） | 可讀寫 |

---

## 資料格式

### 輸入格式（sample.txt）

```
$       我      7       ← 句首預測「我」，權重 7
$       他      6
其      一      7       ← 「其」後面預測「一」
其      二      6
本      地      7
```

- `$` 表示**句首**（句子開始時的預測）
- 格式：`key\tvalue\tweight`
- 按權重降序排列

### 資料產生邏輯（make_predict_data）

```rust
// 輸入格式：「詞組\t權重」或「詞1 詞2\t權重」
if key.contains(' ') {
    // Bigram：「詞1 詞2」→ key=詞1, value=詞2
    let ngram = key.split(' ');
    add_record(&mut data, ngram[0], ngram[1], weight);
} else {
    // 單詞：拆成所有前綴
    // 「早安」→ (早, 安)
    for i in 1..chars.len() {
        let key = chars[0..i];    // 前綴
        let value = chars[i..];   // 剩餘
        add_record(&mut data, &key, &value, weight);
    }
}
```

**範例**：
- `早安 100` → `(早, 安, 100)`
- `早餐 80` → `(早, 餐, 80)`
- `早起 60` → `(早, 起, 60)`

---

## 觸發流程

```
使用者選詞
    ↓
OnSelect(ctx)
    ↓
last_action_ = kSelect
    ↓
OnContextUpdate(ctx)
    ↓
檢查條件：
├── composition 為空？
├── prediction 開關開啟？
├── 非 BackSpace/Escape？
├── commit 類型非 punct/raw/thru？
    ↓
PredictAndUpdate(ctx, last_commit.text)
    ↓
PredictEngine::Predict(context_query)
    ↓
PredictEngine::CreatePredictSegment(ctx)
    ↓
顯示預測候選詞
```

### 句首預測

```cpp
if (ctx->commit_history().empty()) {
  PredictAndUpdate(ctx, "$");  // 使用 "$" 查詢句首預測
  return;
}
```

### 連續預測限制

```cpp
if (last_commit.type == "prediction") {
  iteration_counter_++;
  if (max_iterations > 0 && iteration_counter_ >= max_iterations) {
    predict_engine_->Clear();
    iteration_counter_ = 0;
    return;
  }
}
```

---

## 配置項目

```yaml
predictor:
  db: predict.db           # 資料庫檔案
  max_candidates: 5        # 最大候選詞數（0 = 無限）
  max_iterations: 1        # 最大連續預測次數（0 = 無限）
```

### Schema 配置

```yaml
patch:
  'engine/processors/@before 0': predictor
  'engine/translators/@before 0': predict_translator

switches:
  - name: prediction
    states: [ 關閉預測, 開啓預測 ]
    reset: 1
```

---

## 值得參考的設計

### 1. 句首預測（`$` 符號）

```kotlin
// 台語鍵盤可加入句首預測
fun predict(word: String?): List<Prediction> {
    val query = word ?: "$"  // null 表示句首
    return db.lookup(query)
}
```

### 2. 連續預測限制

```kotlin
// 避免無限預測
private var iterationCounter = 0
private val maxIterations = 3

fun onWordSelected(word: String) {
    if (isShowingPrediction) {
        iterationCounter++
        if (iterationCounter >= maxIterations) {
            clearPrediction()
            return
        }
    }
    predict(word)
}
```

### 3. 預測開關

```kotlin
// 讓使用者可以關閉預測功能
if (!prefs.predictionEnabled) return
```

### 4. 清除時機

```kotlin
// BackSpace/Escape 清除預測
fun onKeyEvent(keyCode: Int): Boolean {
    if (keyCode == KeyEvent.KEYCODE_DEL ||
        keyCode == KeyEvent.KEYCODE_ESCAPE) {
        if (isShowingPrediction) {
            clearPrediction()
            return true
        }
    }
    return false
}
```

---

## 與台語鍵盤的差異

| 項目 | librime-predict | 台語鍵盤 NextWord |
|------|-----------------|-------------------|
| **資料來源** | 預編譯 predict.db | word_association.db + user_association.db |
| **使用者學習** | ❌ 無 | ✅ 有 |
| **句首預測** | ✅ `$` 符號 | ❌ 無 |
| **連續預測限制** | ✅ max_iterations | ❌ 無（用超時控制） |
| **儲存格式** | DoubleArray Trie | SQLite |
| **查詢方式** | 精確匹配 | SQL 查詢 |

### 台語鍵盤的優勢

1. **使用者學習**：librime-predict 是唯讀的，無法學習使用者習慣
2. **即時更新**：SQLite 可即時更新，不需重新編譯
3. **靈活查詢**：SQL 可做更複雜的查詢（如模糊匹配）

### librime-predict 的優勢

1. **查詢速度**：Trie 精確匹配比 SQL 快
2. **記憶體效率**：mmap 載入，系統自動管理
3. **句首預測**：`$` 符號支援句首預測

---

## 實作建議

### 短期改進

1. **加入句首預測**
   - 在 `word_association` 加入 `$` 作為特殊 key
   - 選詞後若無 `lastSelectedWord`，查詢 `$`

2. **加入連續預測限制**
   - 避免使用者一直點預測詞造成無限迴圈
   - 建議 `maxIterations = 3`

### 長期考量

若要提升查詢效能，可考慮：
1. 將 `word_association.db` 改為 DoubleArray Trie 格式
2. 但保留 `user_association.db` 用 SQLite（需要讀寫）

---

## 相關檔案

### librime-predict 原始碼

| 檔案 | 說明 |
|------|------|
| `src/predictor.cc` | Processor，監聽選詞和上下文更新 |
| `src/predict_translator.cc` | Translator，產生預測候選詞 |
| `src/predict_engine.cc` | 預測引擎核心 |
| `src/predict_db.cc` | 預測資料庫（DoubleArray Trie） |
| `tools/make_predict_data/src/main.rs` | 資料產生工具（Rust） |
| `tools/build_predict.cc` | 資料庫建構工具（C++） |

### 台語鍵盤對應

| librime-predict | 台語鍵盤 |
|-----------------|----------|
| Predictor | SmartbarManager |
| PredictEngine | NextWordService |
| PredictDb | word_association.db + user_association.db |
| predict_translator | — |
