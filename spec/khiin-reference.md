# khiin-rs 參考研究

> **類型**: 參考
> **關鍵字**: `khiin`, `起引`, `DPSegment`, `Bigram`, `Trie`
> **相關**: rime-reference.md, azookey-reference.md

---

## 重點摘要

- 雙 Trie 架構：WordTrie（前綴搜尋）+ SyllableTrie（音節驗證）
- N-gram 模型：Unigram + Bigram 頻率排序
- DP 動態規劃分詞：支援連續輸入（無空白）
- 三種輸入模式：Continuous / Classic / Manual

---

## 架構概覽

### 雙 Trie 系統

| Trie | 用途 | 方法 |
|------|------|------|
| WordTrie | 羅馬字 → 詞條 ID | `find_words_by_prefix`, `find_words_from_start` |
| SyllableTrie | 音節驗證 | `is_valid_prefix`, `is_valid_syllable` |

### 資料庫結構

```sql
-- Unigram：單詞頻率
CREATE TABLE unigrams (
    gram TEXT NOT NULL UNIQUE,
    n INTEGER NOT NULL
);

-- Bigram：詞彙關聯
CREATE TABLE bigrams (
    lgram TEXT,      -- 前一個詞
    rgram TEXT,      -- 下一個詞
    n INTEGER NOT NULL
);
```

---

## DP 分詞演算法

### 概念

```
輸入: "goabehchiahpng"
  ↓
DP 分割: ["goa", "beh", "chiah", "png"]
  ↓
查詢 output: "我欲食飯"
```

### 成本計算

```rust
const FREQUENCY_BIAS: f64 = 1.0;
const LETTER_COUNT_BIAS: f64 = 0.2;
const SYLLABLE_COUNT_BIAS: f64 = 0.2;

cost = ln(1.0 / frequency^FREQUENCY_BIAS);
cost = cost / letter_bias * syllable_bias;
```

### Kotlin 實作範例

```kotlin
fun segment(input: String, syllableFreq: Map<String, Int>): List<String> {
    val n = input.length
    val dp = DoubleArray(n + 1) { Double.MAX_VALUE }
    dp[0] = 0.0

    for (i in 0 until n) {
        for (j in (i + 1)..minOf(i + 6, n)) {
            val syllable = input.substring(i, j)
            val freq = syllableFreq[syllable] ?: continue
            val cost = ln(1.0 / freq)
            if (dp[i] + cost < dp[j]) {
                dp[j] = dp[i] + cost
            }
        }
    }
    // 回溯取得結果
}
```

---

## 台語鍵盤應用建議

### P0 - 立即可用

**1. 聲調位置判斷**
```
優先順序: oa > o > a > e > u > i > ng > n > m
```

**2. 合法音節驗證（Regex）**
```regex
^·?((chh|[ckpt]h|[bhgjklmnpst])?(iau|io͘|oai|a[iu]|i[aou]|o[ae͘]|ui|[aeiou])?(ng|[mnptkh])?|(chh|[ckpt]h|[hkmnpst])ng?|(ng|m)h?)(n|ⁿ|ᴺ)?$
```

**3. 特殊符號轉換**

| 輸入 | 輸出 | 說明 |
|------|------|------|
| `nn` | `ⁿ` | 鼻音 |
| `oo` | `o͘` | 長音 o |

**4. Telex 聲調輸入**

| 聲調 | 數字 | Telex |
|------|------|-------|
| 2 | 2 | s |
| 3 | 3 | f |
| 5 | 5 | l |
| 7 | 7 | j |

### P1 - 短期整合

| 功能 | 說明 |
|------|------|
| 三種輸入模式 | Continuous / Classic / Manual |
| 多層次查詢 | 精確 → 模糊 → 分詞 |
| 緩衝區狀態機 | Empty → Composing → Converted → Selecting |

### P2 - 中期規劃

| 功能 | 說明 |
|------|------|
| Bigram 排序 | 考慮前一詞的上下文 |
| 輕聲處理 | Khinless / Hyphen / Dot 三種模式 |
| DP 分詞 | 支援連續輸入 |

---

## 實作優先順序

```
Phase 1（聲調處理）
├── 聲調位置判斷
├── 合法音節驗證
├── 特殊符號轉換
└── Telex 聲調輸入

Phase 2（候選詞優化）
├── 三種輸入模式
├── 多層次查詢
└── 緩衝區狀態機

Phase 3（進階功能）
├── Bigram 排序
├── 輕聲處理
└── DP 分詞
```

---

## 與本專案比較

| 項目 | khiin-rs | TaigiKeyboard |
|------|----------|---------------|
| 資料庫 | 單一 khiin.db | 分開 dictionary.db + word_association.db |
| 關聯模型 | Bigram (lgram → rgram) | 首字 → 詞尾 |
| 分詞 | DP 連續輸入 | 空白分隔單詞 |
| 引擎 | Rust + JNI | Native Kotlin/Swift |

---

## khiin-rs 原始碼參考

| 檔案 | 內容 |
|------|------|
| `ji/src/lomaji.rs` | 聲調轉換、音節驗證 |
| `ji/src/tone.rs` | 聲調處理、Telex |
| `khiin/src/data/segmenter.rs` | DP 分詞 |
| `khiin/src/input/converter.rs` | 候選詞生成 |
| `khiin/src/buffer/buffer_mgr.rs` | 緩衝區管理 |
