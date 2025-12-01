## 詞庫結構與 N-gram 模型分析重點

### 1. 現有詞庫 (`dictionary.db`) 結構分析

*   **DB 類型**: SQLite。
*   **結構**: 單一 `dictionary` 寬表，包含 `tl`, `poj`, `hanzi`, `tl_no_tone`, `poj_no_tone`, `syllable_count` 等欄位。
*   **優點**: 查詢高效 (利用 FTS5 全文搜尋及索引)，結構直觀，實現簡單。
*   **缺點**: 擴充性較低，不易直接整合複雜語言模型。

### 2. `khiin-rs` 詞庫結構與 N-gram 模型分析

*   **DB 類型**: SQLite。
*   **結構**: 高度正規化，多表設計。核心表包括 `inputs` (輸入碼), `conversions` (候選字), `key_sequences` (按鍵序列)。
*   **N-gram 實作**: **有**。明確定義 `unigrams` (單詞頻率) 和 `bigrams` (詞對頻率) 資料表，並有 `ngrams` 視圖整合。
*   **N-gram 數據來源**: `data/data/frequency.csv` (包含台語 Unigram/Bigram 頻率)。
*   **優點**: 支援複雜語言模型 (如 N-gram 預測)，擴充性強，資料分離清晰。
*   **缺點**: 結構較複雜，查詢可能涉及多表 JOIN。

### 3. 遷移到利用 `khiin-rs` N-gram 模型實作的建議 (不使用 Rust)

**目標**: 利用 `khiin-rs` 的台語 N-gram 數據，增強 `taigikeyboard` 的智慧預測能力。

**步驟**:

1.  **獲取 N-gram 數據**:
    *   從 `khiin-rs` 專案複製 `data/data/frequency.csv` 檔案。此檔案包含台語 Unigram 和 Bigram 的頻率數據。

2.  **資料庫整合**:
    *   在您的 `dictionary.db` 中新增兩個資料表：
        *   `unigrams` (gram TEXT PRIMARY KEY, n INTEGER NOT NULL)
        *   `bigrams` (lgram TEXT NOT NULL, rgram TEXT NOT NULL, n INTEGER NOT NULL, PRIMARY KEY (lgram, rgram))

3.  **數據匯入**:
    *   編寫一個 Python 腳本，讀取 `frequency.csv` 檔案，並將其數據匯入到 `dictionary.db` 中的 `unigrams` 和 `bigrams` 表。

4.  **應用程式整合**:
    *   在您的 Swift (iOS) 和 Kotlin (Android) 程式碼中，實作查詢 `unigrams` 和 `bigrams` 表的函式。
    *   利用查詢到的 N-gram 頻率數據，來優化候選詞的排序（例如，根據前一個詞的上下文，將更常出現的詞排在前面），或實現更進階的詞語預測功能。

**原則**: 循序漸進，先從簡單的頻率排序開始，讓功能需求驅動架構的逐步演進。非必要，不進行大規模重構。