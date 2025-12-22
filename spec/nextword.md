# NextWord 下一詞預測

> **功能代號**: `NextWord`
> **關鍵字**: `NextWord`, `WordPrediction`, `聯想詞`, `關聯詞`, `Bigram`, `delimiter`
> **更新日期**: 2025-12-20

---

## 概述

當使用者完成組字並選擇候選詞後，系統根據上下文預測可能的下一個字，顯示於候選詞列供快速選擇。

**範例**：
- 選擇「早安」→ 用「安」查詢 → 預測「安」後面常接的字
- 選擇「我」→ 用「我」查詢 → 預測「欲」「是」等

---

## 核心需求

1. **基礎詞彙關聯**：從現有字典的多字詞產生初始關聯（冷啟動）
2. **使用者學習**：記錄使用者連續選詞的關聯，個人化預測結果
3. **排序機制**：結合字典頻率與使用者習慣排序

---

## 技術設計

### 資料模型：混合式 Bigram

採用 **混合式** 模型，結合字元層級與詞層級：

| 資料來源 | 查詢 key | 預測結果 | 說明 |
|----------|----------|----------|------|
| 字典 | 選中詞的**最後一字** | 單字 | 字元層級（冷啟動）|
| 使用者學習 | 選中詞的**完整詞** | 完整詞 | 詞層級（學習後）|

**範例**：
```
選擇「早安」
  ├─ 字典查詢：prev_word = '安' → 預測「你」「好」（單字）
  └─ 使用者查詢：prev_word = '早安' → 預測「你好」「大家」（完整詞）
```

**效果**：新使用者看到單字預測，隨著使用會逐漸出現完整詞預測。

### 資料庫結構

#### 字典關聯 (dictionary.db 的 word_association 表)

```sql
CREATE TABLE word_association (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    prev_word TEXT NOT NULL,      -- 前一個字（單字）
    next_word TEXT NOT NULL,      -- 下一個字（單字）
    next_tl TEXT,                 -- 下一個字的 TL 羅馬字
    next_poj TEXT,                -- 下一個字的 POJ 羅馬字
    delimiter TEXT DEFAULT '-',   -- 分隔符（羅馬字模式用）
    count INTEGER DEFAULT 1,      -- 頻率（累加）
    UNIQUE(prev_word, next_word)
);

CREATE INDEX idx_prev_word ON word_association(prev_word);
```

**資料範例**：
| prev_word | next_word | next_tl | delimiter | count |
|-----------|-----------|---------|-----------|-------|
| 皮 | 箱 | siunn | - | 242 |
| 起 | 來 | lâi | - | 1271 |
| 查 | 某 | bóo | - | 1049 |

#### 使用者學習 (user_association.db)

```sql
CREATE TABLE user_association (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    prev_word TEXT NOT NULL,      -- 前一個詞（完整詞）
    next_word TEXT NOT NULL,      -- 下一個詞（完整詞）
    next_tl TEXT,
    next_poj TEXT,
    delimiter TEXT DEFAULT ' ',   -- 分隔符（使用者輸入的："-" 或 " "）
    count INTEGER DEFAULT 1,
    last_used TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(prev_word, next_word)
);

CREATE INDEX idx_user_prev_word ON user_association(prev_word);
```

### 分隔符處理

台語羅馬字中，同一詞的音節以 `-` 連接（如 `phuê-siunn` = 皮箱），不同詞則以空格分隔。

#### 設計原則

**套用「自動空白」設定**：NextWord 候選詞與一般候選詞行為一致，依照「齒盤設定」中的「自動空白」設定決定是否自動加空白。

| 自動空白設定 | NextWord 行為 | 說明 |
|-------------|---------------|------|
| 開啟 | 選詞後自動加空白 | 與一般候選詞行為一致 |
| 關閉 | 選詞後不加空白 | 使用者自行輸入分隔符 |

#### 使用流程範例（自動空白開啟）

```
1. 使用者輸入 phuê，選擇候選詞「皮」→ 輸出 "phuê "
2. NextWord 候選詞出現「箱」
3. 使用者選擇 NextWord「箱」→ 輸出 "siunn "
4. 最終結果：phuê siunn ✓
```

#### 優點

- 與一般候選詞行為一致，使用者無需區分
- 尊重使用者設定偏好

---

### 與 UserFrequency 的差異

| | **UserFrequency**（現有） | **NextWord**（新功能） |
|---|---|---|
| **模型** | Unigram（單詞頻率） | Bigram（詞與詞關聯） |
| **資料結構** | `word → count` | `prev_word → next_word → count` |
| **觸發時機** | 組字中，前綴搜尋排序 | 組字結束後，預測下一詞 |
| **解決問題** | 「我常用哪些詞」 | 「這個詞後面常接什麼」 |

---

## 資料來源

### 1. 字典基礎關聯（離線產生）

從 `dictionary.csv` 的多字詞產生「相鄰字 Bigram」關聯：

```python
# dictionary/build/08_generate_association.py

def generate_bigrams(hanzi: str, tl: str, poj: str, frequency: int):
    """從多字詞產生相鄰字 Bigram"""
    chars = list(hanzi)
    tl_parts = tl.split("-") if tl else []

    for i in range(len(chars) - 1):
        yield {
            "prev_word": chars[i],
            "next_word": chars[i + 1],
            "next_tl": tl_parts[i + 1] if i + 1 < len(tl_parts) else "",
            "count": frequency
        }

# 範例：
# 「早餐」→ (早, 餐)
# 「早起床」→ (早, 起), (起, 床)
# 「我欲食飯」→ (我, 欲), (欲, 食), (食, 飯)
```

**字數限制**：2-3 字的多字詞

### 2. 使用者關聯學習（即時記錄，詞層級）

**記錄時機**：使用者連續選擇候選詞（間隔 < 10 秒）

**單層關聯記錄**：只記錄「前一詞 → 當前詞」的關聯：

```
使用者依序選擇 早安, 你好, 再見：

選擇「早安」→ 無記錄（無前一詞）
選擇「你好」→ 記錄：早安 → 你好
選擇「再見」→ 記錄：你好 → 再見
```

**效果**：使用者輸入「早安」後，會建議「你好」（完整詞，非單字）

---

## Android 實作

### 核心流程

```
使用者選擇候選詞 / 按 Enter 確認
    ↓
handleNextWordPrediction()
    ↓
記錄關聯（若有前一個詞，且間隔 < 10 秒）
    ↓
查詢預測
  ├─ 字典：WHERE prev_word = 最後一字
  └─ 使用者學習：WHERE prev_word = 完整詞
    ↓
合併結果，依分數排序
    ↓
更新候選詞列（顯示關聯詞）
```

### 觸發時機

| 操作 | 漢字模式 | 羅馬字模式 |
|------|----------|------------|
| 點擊候選詞 | 觸發 NextWord | 觸發 NextWord |
| 按 Enter 確認 | 不觸發（清除候選詞） | 觸發 NextWord |

### 預測查詢

```kotlin
// NextWordService.kt

suspend fun predict(word: String, limit: Int = 30): List<Prediction> {
    // Bigram 模型：使用最後一字作為查詢 key
    val lastChar = word.last().toString()
    val results = mutableMapOf<String, Prediction>()

    // 1. 字典查詢（用最後一字）
    dictDatabase?.rawQuery(
        "SELECT ... FROM word_association WHERE prev_word = ?",
        arrayOf(lastChar)
    )

    // 2. 使用者學習查詢（用完整詞）
    userDatabase?.rawQuery(
        "SELECT ... FROM user_association WHERE prev_word = ?",
        arrayOf(word)
    )

    // 3. 合併結果，user 權重 ×50，dict 權重 ×1
    return results.values.sortedByDescending { it.score }.take(limit)
}
```

### 記錄關聯

```kotlin
// SmartbarManager.kt -> handleNextWordPrediction()

// 記錄關聯（完整詞 → 完整詞）
if (shouldRecordAssociation && lastSelectedWord != null) {
    NextWordService.recordAssociation(
        prev = lastSelectedWord!!,
        nextHanzi = displayText,
        nextTl = nextTl,
        nextPoj = nextPoj,
        context = context
    )
}
```

### 狀態管理

```kotlin
// SmartbarManager.kt

private var lastSelectedWord: String? = null
private var lastSelectionTime: Long = 0
private var isShowingNextWord: Boolean = false

private val ASSOCIATION_TIMEOUT_MS = 10000L  // 連續選詞間隔（10 秒）
private val CONTEXT_TIMEOUT_MS = 30_000L     // 上下文超時（30 秒）
private val SENTENCE_END_PUNCTUATION = setOf('。', '！', '？', '.', '!', '?')
```

### 上下文重置

| 事件 | 處理 |
|------|------|
| 輸入句末標點（。！？） | 重置 `lastSelectedWord` |
| 長時間無輸入（> 30 秒） | 重置 |
| 切換輸入框 | 重置 |
| 退格清空文字 | 重置 |

---

## 相關檔案

### Android 檔案

| 檔案 | 說明 |
|------|------|
| `ime/dictionary/NextWordService.kt` | NextWord 服務（predict, recordAssociation） |
| `ime/dictionary/DictionaryModels.kt` | TaigiWord 資料模型（含 delimiter） |
| `ime/text/smartbar/SmartbarManager.kt` | 候選詞管理、NextWord 觸發、delimiter 追蹤 |
| `ime/text/TextInputManager.kt` | 輸入處理、Enter 確認、setInputDelimiter |

### 資料檔案

| 檔案 | 說明 |
|------|------|
| `dictionary/build/08_generate_association.py` | 字典關聯產生腳本（Bigram） |
| `assets/dictionary.db` | 字典 + word_association 表（隨 App 發布） |
| `files/user_association.db` | 使用者學習（App 內部建立） |

---

## 實作進度

### Android

- [x] NextWordService.kt（predict, recordAssociation）
- [x] 字典關聯查詢（Bigram：最後一字查詢）
- [x] 使用者學習查詢（完整詞查詢）
- [x] 候選詞點擊觸發 NextWord
- [x] Enter 確認觸發 NextWord（羅馬字模式）
- [x] 上下文重置（標點、超時、切換輸入框）
- [x] NextWord 模式下輸入 "-" 保留候選詞
- [x] 改用 dictionary.db 的 word_association 表
- [x] ~~分隔符 (delimiter) 自動判斷~~ (已移除)
- [x] 套用「自動空白」設定（與一般候選詞行為一致）
- [x] ~~多層級關聯記錄~~ 簡化為單層關聯（前一詞 → 當前詞）
- [x] 時間衰減（指數衰減，半衰期一週）
- [x] 使用者關聯數量上限（50,000 筆）
- [x] 雜訊過濾（標點符號、數字不作為 context）
- [x] 驗證功能正常運作（2025-12-20 Code Review）

### 優化

- [x] ~~調整權重參數（目前 user ×50, dict ×1）~~ 加入時間衰減
- [x] 限制使用者關聯數量上限
- [x] 清除學習資料（整合到「挕掉捷用詞紀錄」選項）

---

## 備註

1. **效能考量**：設定使用者關聯上限（如 50,000 條）
2. **隱私考量**：使用者關聯資料僅存本機，不上傳
3. **冷啟動**：字典基礎關聯確保新使用者也有預測結果
4. **漸進學習**：使用者越常使用，預測越準確

---

## 變更記錄

### 2025-12-20: 修正 word_association 髒資料

**問題**：
- 字典產生腳本對混合內容（漢字/羅馬字）逐字切分，產生無效 bigram
- 例：`"a-khah"` → `(a, -)`, `(-, k)`, `(k, h)` ... ✗

**修正**：使用 TL 欄位的連字符決定切分邊界
- TL 有明確的音節邊界（用 `-` 或 `--` 分隔）
- 漢字逐字切分後，與 TL 音節數量比對
- 數量一致才產生 bigram，否則跳過（資料品質問題）
- 單一字元必須是 CJK 漢字

**新邏輯**：
```python
tl_parts = re.split(r'-+', tl)      # "tsá-an" → ["tsá", "an"]
hanzi_chars = [c for c in hanzi if is_cjk(c)]  # "早安" → ["早", "安"]

if len(hanzi_chars) == len(tl_parts):
    # 數量一致，產生 bigram
```

**效果**：
| hanzi | tl | 結果 |
|-------|-----|------|
| `早安` | `tsá-an` | `(早, 安)` ✓ |
| `早起床` | `tsá-khí-chhn̂g` | `(早, 起)`, `(起, 床)` ✓ |
| `a-khah` | `a-khah` | 無結果（非 CJK）✓ |

**修改的檔案**：
| 檔案 | 變更 |
|------|------|
| `08_generate_association.py` | 新增 `is_cjk()`，改用 TL 切分邏輯 |

---

### 2025-12-20: 詞層級 User Association

**變更**：
- User association 改為記錄完整詞（非單字）
- 歷史視窗改為記錄完整詞
- 查詢時使用完整詞查 user_association

**混合模式**：
| 來源 | prev_word | next_word | 說明 |
|------|-----------|-----------|------|
| 字典 | 最後一字 | 單字 | 字元層級（冷啟動）|
| 使用者 | 完整詞 | 完整詞 | 詞層級（學習後）|

**效果**：
```
輸入「早安」後的候選詞：
1. 你好 (user, 詞層級)
2. 大家 (user, 詞層級)
3. 你 (dict, 字元層級)
4. 好 (dict, 字元層級)
```

**修改的檔案**：
| 檔案 | 變更 |
|------|------|
| `SmartbarManager.kt` | recordAssociation 使用 displayText，歷史記錄完整詞 |

---

### 2025-12-20: 雜訊過濾

**變更**：
- 新增 `NOISE_CHARS` 常數（標點符號、空白、數字）
- 新增 `isNoise()` 函數
- 記錄關聯時跳過雜訊
- 雜訊不加入 selectionHistory

**過濾的字元**：
- 標點符號：`。！？.!?，,、；;：:「」『』""'（）()【】[]{}—–-～~…·`
- 空白：` `（半形）、`　`（全形）
- 數字：`0-9`

**效果**：
- 避免記錄無意義的關聯（如 `。→ 我`）
- 保持 selectionHistory 只包含有意義的詞

**修改的檔案**：
| 檔案 | 變更 |
|------|------|
| `SmartbarManager.kt` | 新增 NOISE_CHARS、isNoise()、過濾邏輯 |

---

### 2025-12-20: 使用者關聯數量上限

**變更**：
- 新增 `MAX_USER_ASSOCIATIONS = 50,000`（上限）
- 新增 `PRUNE_CHECK_INTERVAL = 100`（每 100 次記錄後檢查）
- 新增 `PRUNE_BATCH_SIZE = 5,000`（每次清理筆數）
- 新增 `pruneOldAssociations()` 清理函數
- 新增 `getAssociationCount()` 查詢函數

**清理策略**：
- 每記錄 100 次後檢查是否超過上限
- 超過時刪除 `count` 最低且 `last_used` 最舊的關聯
- 優先保留：常用（count 高）且近期使用（last_used 新）的關聯

**修改的檔案**：
| 檔案 | 變更 |
|------|------|
| `NextWordService.kt` | 新增常數、recordCounter、pruneOldAssociations()、getAssociationCount() |

---

### 2025-12-20: 時間衰減

**變更**：
- 使用者關聯分數加入時間衰減因子（參考 RIME）
- 分數類型從 Int 改為 Double
- 新增 `calculateDecay()` 函數

**公式**：
```kotlin
// 指數衰減，半衰期 168 小時（一週）
decay = exp(-ageHours / 168.0 * 0.693)

// 最終分數
userScore = count × USER_WEIGHT × decay
```

**效果**：
| 時間 | 衰減因子 | 說明 |
|------|----------|------|
| 剛使用 | 1.0 | 完整權重 |
| 1 週後 | 0.5 | 權重減半 |
| 2 週後 | 0.25 | 權重 1/4 |
| 1 月後 | 0.06 | 幾乎無影響 |

**修改的檔案**：
| 檔案 | 變更 |
|------|------|
| `NextWordService.kt` | 新增 DECAY_HALF_LIFE_HOURS、calculateDecay()、score 改 Double |
| `SmartbarManager.kt` | prediction.score.toInt() 轉換 |

---

### 2025-12-21: 簡化為單層關聯記錄

**變更**：
- 移除 `selectionHistory` 滑動視窗
- 移除 `HISTORY_WINDOW_SIZE` 常數
- 改為只記錄「前一詞 → 當前詞」的單層關聯

**原因**：
- 多層級關聯會產生組合爆炸（O(n²) 筆資料）
- 邊際效益低：「我想欲」→「食」的精準度提升有限
- 與字典層級一致：字典也是單層 Bigram

**範例**：
```
選擇「欲」時（lastSelectedWord = 想）：
  → 記錄：想 → 欲（只記錄一筆）
```

**修改的檔案**：
| 檔案 | 變更 |
|------|------|
| `SmartbarManager.kt` | 移除 selectionHistory、HISTORY_WINDOW_SIZE，簡化關聯記錄邏輯 |

---

### 2025-12-20: 多層級關聯記錄（已移除）

~~原設計：維護 selectionHistory 滑動視窗，記錄多層級關聯。~~
已於 2025-12-21 簡化為單層關聯。

---

### 2025-12-20: NextWord 套用「自動空白」設定

**變更**：
- NextWord 候選詞選擇後，依照「自動空白」設定決定是否加空白
- 移除 `!isNextWordPrediction` 條件，讓 NextWord 與一般候選詞行為一致

**原因**：
- 使用者預期一致：同樣是「選擇候選詞」的動作，行為應該一致
- 設定語意明確：「自動空白」應涵蓋所有候選詞選擇場景

**修改的檔案**：
| 檔案 | 變更 |
|------|------|
| `SmartbarManager.kt` | 移除自動空白條件中的 `!isNextWordPrediction` |

---

### 2025-12-20: 移除 delimiter 自動判斷

**變更**：
- 移除 `lastInputDelimiter` 狀態追蹤
- 移除 `setInputDelimiter()` 和 `resetInputDelimiter()` 方法

**原因**：
- 簡化程式邏輯
- 避免雙重分隔符問題

**修改的檔案**：
| 檔案 | 變更 |
|------|------|
| `SmartbarManager.kt` | 移除 lastInputDelimiter、setInputDelimiter、resetInputDelimiter |
| `TextInputManager.kt` | 移除 setInputDelimiter 呼叫 |

---

### 2025-12-20: 分隔符 (delimiter) 欄位（保留但不自動使用）

**變更**：
- `word_association` 表加入 `delimiter` 欄位（預設 `-`）
- `user_association` 表加入 `delimiter` 欄位（預設空格）
- `TaigiWord` 加入 `delimiter` 屬性

**註**：delimiter 欄位保留供未來使用，但目前不會自動加到輸出

---

### 2025-12-20: Bigram 模型重構

**變更**：
- 模型從「首字→詞尾」改為「相鄰字 Bigram」
- `word_association` 表從獨立 db 併入 `dictionary.db`
- 查詢 key 從「首字」改為「最後一字」

**原因**：
- 「首字→詞尾」只能預測同一詞的後半段，不是真正的語言模型
- 相鄰字 Bigram 是標準的 N-gram 模型，可預測下一個字
