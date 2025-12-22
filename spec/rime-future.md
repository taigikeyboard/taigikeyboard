# librime 功能借鑑清單

> **用途**：整理 librime 中對台語鍵盤有價值的功能
> **更新日期**：2025-12-19

---

## 優先級分類

| 優先級 | 說明 | 預估複雜度 |
|--------|------|-----------|
| P0 | 立即可用，改善現有功能 | 低 |
| P1 | 短期實作，明顯提升體驗 | 中 |
| P2 | 中期規劃，需要架構調整 | 中高 |
| P3 | 長期參考，大幅重構 | 高 |

---

## P0 - 立即可用

### 1. 時間衰減權重

**來源**：`spec/rime.md`

**現狀**：使用者學習只用 `count` 計數，舊詞和新詞權重相同。

**改進**：加入時間衰減，近期使用的詞彙優先。

```kotlin
// NextWordService.kt 或 UserFrequencyService.kt
fun calculateWeight(count: Int, lastUsedMs: Long): Double {
    val ageHours = (System.currentTimeMillis() - lastUsedMs) / 3600000.0
    val decay = exp(-ageHours / 168.0)  // 一週半衰期
    return count * decay
}
```

**影響範圍**：
- `NextWordService.predict()` - 修改分數計算
- `UserFrequencyService` - 修改權重計算
- 資料庫已有 `last_used` 欄位，無需修改 schema

**效益**：使用者習慣變化時，新詞彙能更快浮現。

---

### 2. 使用者詞彙固定加分

**來源**：`spec/rime.md`

**現狀**：使用者詞彙用 `USER_WEIGHT = 50` 乘法加權。

**改進**：改為加法加分，更可預測。

```kotlin
// 目前：score = count * USER_WEIGHT (乘法)
// 改進：score = dictScore + USER_BONUS (加法)

const val USER_BONUS = 50  // 固定加分
const val DICT_BASE = 1    // 字典基礎分

// 合併分數時
val finalScore = dictScore + (if (isUserPhrase) USER_BONUS else 0)
```

**效益**：使用者選過一次的詞彙，就會明顯優先於字典詞彙。

---

## P1 - 短期實作

### 3. 歷史記錄快捷輸入

**來源**：`spec/rime-features.md`

**功能**：記錄最近輸入的 N 個詞彙，按特定鍵快速叫出。

**設計**：
```kotlin
object HistoryService {
    private const val MAX_SIZE = 10
    private val history = LinkedList<HistoryEntry>()

    data class HistoryEntry(
        val hanzi: String,
        val tl: String,
        val poj: String,
        val timestamp: Long
    )

    fun record(entry: HistoryEntry) {
        history.removeIf { it.hanzi == entry.hanzi }
        history.addFirst(entry)
        if (history.size > MAX_SIZE) history.removeLast()
    }

    fun getHistory(): List<HistoryEntry> = history.toList()
}
```

**觸發方式**：
- 長按空白鍵顯示歷史
- 或特定手勢觸發

**效益**：重複輸入常用詞時更快速。

---

### 4. 撤銷最近輸入

**來源**：`spec/rime.md`

**功能**：3 秒內可撤銷錯誤的選詞記錄。

**設計**：
```kotlin
object UndoService {
    private const val UNDO_TIMEOUT_MS = 3000L
    private var lastCommit: CommitRecord? = null

    data class CommitRecord(
        val text: String,
        val timestamp: Long,
        val frequencyDelta: Int,
        val associationPrev: String?
    )

    fun recordCommit(record: CommitRecord) {
        lastCommit = record
    }

    fun canUndo(): Boolean {
        val record = lastCommit ?: return false
        return System.currentTimeMillis() - record.timestamp < UNDO_TIMEOUT_MS
    }

    fun undo() {
        // 回滾頻率更新和關聯記錄
    }
}
```

**效益**：誤選時可以撤銷，避免污染學習資料。

---

### 5. 反查功能（漢字查羅馬字）

**來源**：`spec/rime-features.md`

**功能**：輸入漢字查詢對應的羅馬字拼音。

**使用場景**：
- 學習台語時查詢發音
- 忘記拼寫時快速查詢

**設計**：
```kotlin
// 反查服務
object ReverseLookupService {
    fun lookup(hanzi: String, context: Context): List<ReverseLookupResult> {
        // 從 dictionary.db 查詢 hanzi -> tl, poj
        val sql = """
            SELECT tl, poj, frequency
            FROM words
            WHERE hanzi = ?
            ORDER BY frequency DESC
            LIMIT 10
        """
        // ...
    }
}

data class ReverseLookupResult(
    val hanzi: String,
    val tl: String,
    val poj: String
)
```

**觸發方式**：
- 特殊前綴（如 `` ` ``）
- 或長按候選詞顯示發音

**效益**：對台語學習者特別有幫助。

---

## P2 - 中期規劃

### 6. POJ/TL 統一輸入（Spelling Algebra 簡化版）

**來源**：`spec/rime-spelling.md`

**現狀**：dictionary.db 同時存 POJ 和 TL 兩種拼寫。

**改進**：允許使用者用任一種拼寫輸入，自動對應。

**設計**：
```kotlin
object SpellingNormalizer {
    // POJ -> TL 映射規則
    private val POJ_TO_TL = mapOf(
        "ch" to "ts",
        "chh" to "tsh",
        "oa" to "ua",
        "oe" to "ue",
        "eng" to "ing",
        "ek" to "ik"
    )

    fun normalize(input: String): String {
        var result = input
        POJ_TO_TL.forEach { (poj, tl) ->
            result = result.replace(poj, tl)
        }
        return result
    }

    fun expandVariants(input: String): List<String> {
        // 產生所有可能的變體
        return listOf(input, normalize(input)).distinct()
    }
}
```

**查詢時**：
```kotlin
fun search(input: String): List<Candidate> {
    val variants = SpellingNormalizer.expandVariants(input)
    return variants.flatMap { searchSingle(it) }.distinctBy { it.hanzi }
}
```

**效益**：使用者不需記住用 POJ 還是 TL，任一種都能找到詞彙。

---

### 7. 模糊音支援

**來源**：`spec/rime-spelling.md`

**功能**：處理台語常見的混淆音。

**常見混淆**：
| 混淆類型 | 範例 |
|---------|------|
| n/l 不分 | lâng ↔ nâng |
| in/ing 不分 | sin ↔ sing |
| 濁音清音 | g ↔ k |

**設計**：
```kotlin
object FuzzyMatcher {
    private val FUZZY_RULES = listOf(
        "n" to "l",
        "l" to "n",
        "in$" to "ing",
        "ing$" to "in"
    )

    fun expandFuzzy(input: String): List<Pair<String, Double>> {
        val results = mutableListOf(input to 1.0)

        FUZZY_RULES.forEach { (from, to) ->
            if (input.contains(Regex(from))) {
                val fuzzy = input.replace(Regex(from), to)
                results.add(fuzzy to 0.5)  // 模糊音懲罰
            }
        }

        return results
    }
}
```

**效益**：降低拼寫錯誤導致找不到詞彙的情況。

---

### 8. 連續輸入分詞（簡化版）

**來源**：`spec/rime-spelling.md`、`spec/khiin.md`

**現狀**：需要空白鍵分隔每個詞。

**改進**：支援連續輸入，自動分詞。

**簡化設計**（不用完整 DAG）：
```kotlin
object SimpleSegmenter {
    // 使用貪婪匹配 + 回溯
    fun segment(input: String, syllables: Set<String>): List<String>? {
        if (input.isEmpty()) return emptyList()

        // 嘗試最長匹配
        for (len in minOf(input.length, 6) downTo 1) {
            val prefix = input.substring(0, len)
            if (syllables.contains(prefix)) {
                val rest = segment(input.substring(len), syllables)
                if (rest != null) {
                    return listOf(prefix) + rest
                }
            }
        }

        return null  // 無法分詞
    }
}
```

**效益**：輸入更流暢，減少空白鍵次數。

---

## P3 - 長期參考

### 9. Pipeline 架構

**來源**：`spec/rime-config.md`

**概念**：將輸入處理拆分為獨立模組。

```
Input → Processor → Segmentor → Translator → Filter → Output
```

**好處**：
- 每個模組職責單一
- 易於測試和擴展
- 支援多種輸入方案

**考量**：需要較大的架構重構，建議在 v2.0 時考慮。

---

### 10. 配置驅動

**來源**：`spec/rime-config.md`

**概念**：用配置檔定義輸入法行為，減少程式碼修改。

```yaml
# taigi.schema.yaml
schema:
  name: 台語輸入法
  version: "1.0"

speller:
  alphabet: abcdefghijklmnopqrstuvwxyz
  delimiter: " -"

translator:
  dictionary: taigi
  enable_user_dict: true

punctuator:
  half_shape:
    ',': '，'
    '.': '。'
```

**好處**：
- 使用者可自訂行為
- 支援多種輸入方案切換
- 減少程式碼修改

**考量**：需要設計配置解析器，複雜度高。

---

## 實作建議順序

```
Phase 1（NextWord 優化）
├── P0-1. 時間衰減權重
├── P0-2. 使用者詞彙加分
└── P1-3. 歷史記錄

Phase 2（輸入體驗）
├── P1-4. 撤銷功能
├── P1-5. 反查功能
└── P2-6. POJ/TL 統一

Phase 3（進階功能）
├── P2-7. 模糊音
└── P2-8. 連續輸入分詞

Phase 4（架構升級）
├── P3-9. Pipeline 架構
└── P3-10. 配置驅動
```

---

## 相關文件

- `spec/rime.md` - 使用者字典、候選排序
- `spec/rime-config.md` - 配置系統、Pipeline
- `spec/rime-spelling.md` - 拼寫系統、分詞
- `spec/rime-features.md` - 其他功能
- `spec/khiin.md` - khiin-rs 參考（Bigram、DP 分割）
