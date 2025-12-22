# khiin-rs 參考筆記

> **關鍵字**: `khiin`, `起引`, `參考`, `DP分割`, `N-gram`, `Bigram`, `Trie`
> **更新日期**：2025-12-20

---

## Trie 架構

khiin-rs 使用**兩種 Trie**，各有不同用途：

### 1. WordTrie（詞彙 Trie）

**檔案**：`khiin/src/data/trie.rs`
**用途**：羅馬字 → 詞條 ID（前綴搜尋）

```rust
// 使用 qp_trie 庫（QP Trie，壓縮 Trie）
type WordTrie = QpTrie<BString, Vec<i64>>;

// key: 羅馬字 (e.g., "ho2")
// value: Vec<input_id> (詞條 ID 列表)
```

**功能**：
| 方法 | 用途 |
|------|------|
| `find_words_by_prefix(query)` | 前綴搜尋，返回所有匹配的詞條 ID |
| `find_words_from_start(query)` | 找出 query 中所有有效的 key（用於分詞） |
| `contains(query)` | 檢查 key 是否存在 |

### 2. SyllableTrie（音節 Trie）

**檔案**：`khiin/src/data/syllable_trie.rs`
**用途**：驗證輸入是否為合法台語音節

```rust
// 自己實作的 HashMap Trie
// 從 SYLLABLES_TXT（預定義音節表）建立
```

**功能**：
| 方法 | 用途 |
|------|------|
| `is_valid_prefix(prefix)` | 檢查是否為合法音節的**前綴**（打字中即時驗證） |
| `is_valid_syllable(word)` | 檢查是否為**完整**的合法音節 |

### 與台語鍵盤的對比

| 功能 | khiin-rs | 台語鍵盤 |
|------|----------|----------|
| 羅馬字 → 詞條 ID | WordTrie (qp_trie) | MARISA-trie |
| 音節驗證 | SyllableTrie | ❌ 無（未來可用 Regex） |
| Association | SQLite (bigrams) | SQLite (word_association) |

### 關鍵發現

**Trie 不是用於 Association**。Association 用 SQLite 儲存是正確的設計。

Trie 的用途是：
1. **前綴搜尋**（輸入候選）
2. **音節驗證**（即時回饋）
3. **分詞輔助**（`find_words_from_start` 用於 DP 分割）

### 為什麼需要音節驗證和分詞？

因為 khiin-rs 支援**連續輸入**（不按空白鍵）：

```
khiin-rs: goabehchiahpng → 自動分割 → ["goa", "beh", "chiah", "png"]
台語鍵盤: goa ␣ beh ␣ chiah ␣ png → 使用者手動分隔
```

台語鍵盤目前是「空白分隔」模式，不需要這些功能。
若未來要支援連續輸入，則需要加入 SyllableTrie 和 DP 分割。

---

## N-gram 資料模型

### 資料庫結構

khiin 使用**單一資料庫** `khiin.db`，包含 N-gram 表格：

```sql
-- Unigram：單詞頻率
CREATE TABLE unigrams (
    gram TEXT NOT NULL UNIQUE,       -- 詞彙
    n INTEGER NOT NULL               -- 出現次數
);

-- Bigram：詞彙關聯（前詞 → 後詞）
CREATE TABLE bigrams (
    lgram TEXT,                      -- 左側詞（前一個詞）
    rgram TEXT,                      -- 右側詞（下一個詞）
    n INTEGER NOT NULL,              -- 共現次數
    UNIQUE(lgram, rgram)
);

-- View：合併查詢
CREATE VIEW ngrams AS
SELECT
    b.lgram,
    u.gram AS rgram,
    u.n AS unigram_count,
    b.n AS bigram_count
FROM unigrams u
LEFT JOIN bigrams b ON u.gram = b.rgram;
```

### 候選詞排序 SQL

```sql
SELECT c.*
FROM conversion_lookups c
    LEFT JOIN unigrams u ON c.output = u.gram
WHERE c.key_sequence = :query
ORDER BY
    u.n DESC,        -- Unigram 頻率（高優先）
    c.weight DESC    -- 權重
```

**備註**：Bigram 排序邏輯被註解掉，目前只使用 Unigram。

### 使用者學習機制

根據 README：
> The database is continually updated with user data during use, to improve
> candidate prediction based on a simple N-gram model that currently uses
> 1-gram and 2-gram frequencies.

- 使用者選詞時更新 unigram 和 bigram
- 資料僅存本機，不上傳

---

## 與 TaigiKeyboard 的比較

| 項目 | khiin-rs | TaigiKeyboard（目前） |
|------|----------|----------------------|
| **資料庫** | 單一 `khiin.db` | 分開 `dictionary.db` + `word_association.db` |
| **關聯模型** | Bigram (lgram → rgram) | 首字 → 詞尾 |
| **使用者學習** | 更新 unigram + bigram | 獨立 `user_association.db` |
| **排序** | unigram.n + weight | frequency + user_weight |

### 建議改進方向

1. **Bigram 模型**：採用 `lgram → rgram`（相鄰字關聯）比「首字→詞尾」更通用
2. **單一資料庫**：考慮將 association 併入 dictionary.db，簡化詞庫管理
3. **N-gram View**：合併 unigram 和 bigram 查詢

---

## 聲調轉換

khiin-rs 有類似 ToneConverter 的功能，在 `syllable.rs` 的 `compose()` 函數：
- 將數字聲調轉換成聲調符號（如 `ho2` → `hó`）
- 組字區永遠顯示帶聲調符號，無論是否有匹配的候選詞

## convert_guess 機制

**不是「猜候選詞」，而是「猜音節分割」**

### 流程

```
輸入: "goalaite2"
  ↓
parse_whole_input() - 解析輸入
  ↓
can_segment_max() - DP 找出可分割的最長前綴
  ↓
segment() - 最小成本演算法分割成音節
  ↓
結果: ["goa", "lai", "te2"] → "goá-lâi-tè"
```

### 核心：動態規劃分割

- 分割依據是「子字串是否為合法台語音節」
- 成本計算：`COST = ln(1 / P)`，P = 詞頻 / 總詞數
- 選擇成本最低的分割方式

### 詞庫量少的影響

| 情境 | 影響 |
|------|------|
| 音節表完整 | 分割正確 |
| 音節表缺少 | 可能分割錯誤或標記為 Plaintext |
| 漢字詞庫少 | 不影響分割，只是沒有漢字候選詞 |

## 與台語鍵盤的差異

| 項目 | khiin-rs | 台語鍵盤 |
|------|----------|----------|
| 分割依據 | 音節表 + DP | 無（單詞搜尋） |
| Fallback | 自動分割 + 帶調羅馬字 | 帶調組字文字 |
| 適用場景 | 連續輸入（無空格） | 單詞輸入（有空格） |

## 分隔符號處理

### autospace 邏輯

在 `buffer.rs` 的 `autospace()` 函數判斷是否加空格：

| 前一個 | 後一個 | 結果 |
|--------|--------|------|
| 漢字 | 漢字 | 不加空格 |
| 漢字 | 羅馬字 | 加空格 |
| 羅馬字 | 漢字 | 加空格 |
| 羅馬字 | 羅馬字 | 加空格 |

### 連字符號處理

連字符號 `-` 是在**資料庫建立時就寫入 output 欄位**，不是動態產生：

| 欄位 | 格式 |
|------|------|
| input | 空格分隔：`"goa beh chiah png"` |
| output（漢字） | 無分隔：`"我欲食飯"` |
| output（羅馬字） | 連字符號：`"góa-beh-chia̍h-pn̄g"` |

### 流程範例

```
使用者輸入: goabehchiahpng
  ↓
DP 分割: ["goa", "beh", "chiah", "png"]
  ↓
查詢 input: "goa beh chiah png"
  ↓
取得 output:
  - 漢字模式: "我欲食飯"
  - 羅馬字模式: "góa-beh-chia̍h-pn̄g"
```

## 相關檔案

- `khiin/src/input/syllable.rs` - 聲調轉換
- `khiin/src/input/converter.rs` - convert_guess
- `khiin/src/input/parser.rs` - 輸入解析
- `khiin/src/data/segmenter.rs` - DP 分割演算法
- `khiin/src/buffer/buffer_mgr.rs` - Preedit 構建
- `khiin/src/buffer/buffer.rs` - autospace 邏輯
- `khiin/src/db/models/key_conversion.rs` - 資料庫模型
