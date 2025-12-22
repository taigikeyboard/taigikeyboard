# 台語鍵盤未來功能實作計畫

> **整合來源**：khiin-rs + librime 功能借鑑
> **更新日期**：2025-12-19

---

## 優先級定義

| 優先級 | 說明 | 複雜度 | 預估時程 |
|--------|------|--------|----------|
| P0 | 立即可用，改善現有功能 | 低 | 1-2 天/項 |
| P1 | 短期實作，明顯提升體驗 | 中 | 3-5 天/項 |
| P2 | 中期規劃，需要架構調整 | 中高 | 1-2 週/項 |
| P3 | 長期參考，大幅重構 | 高 | 未定 |

---

## Phase 1：NextWord 與排序優化

### 1.1 時間衰減權重 `P0`

**來源**：librime
**現狀**：`count` 計數，舊詞新詞權重相同
**目標**：近期使用的詞彙優先

```kotlin
fun calculateWeight(count: Int, lastUsedMs: Long): Double {
    val ageHours = (System.currentTimeMillis() - lastUsedMs) / 3600000.0
    val decay = exp(-ageHours / 168.0)  // 一週半衰期
    return count * decay
}
```

**影響檔案**：
- `NextWordService.kt` - 修改 `predict()` 分數計算
- `UserFrequencyService.kt` - 修改權重計算

---

### 1.2 使用者詞彙固定加分 `P0`

**來源**：librime
**現狀**：`USER_WEIGHT = 50` 乘法加權
**目標**：改為加法加分，更可預測

```kotlin
const val USER_BONUS = 50  // 固定加分

val finalScore = dictScore + (if (isUserPhrase) USER_BONUS else 0)
```

**效益**：使用者選過一次就明顯優先。

---

### 1.3 N-gram 頻率統計 `P2`

**來源**：khiin-rs
**目標**：根據上下文提供更準確的候選詞

```sql
-- Bigram 表（目前 word_association 類似）
CREATE TABLE bigrams (
    lgram TEXT,      -- 前詞
    rgram TEXT,      -- 後詞
    n INTEGER NOT NULL,
    UNIQUE(lgram, rgram)
);

-- 排序時優先考慮 bigram
ORDER BY bigram.n DESC, unigram.n DESC, weight DESC
```

**與現有整合**：可擴展 `word_association` 表。

---

## Phase 2：聲調與拼寫處理

### 2.1 聲調位置自動判斷 `P0`

**來源**：khiin-rs
**功能**：自動找出聲調符號標記位置

```kotlin
object TonePositionFinder {
    private val VOWEL_PRIORITY = listOf("oa", "o", "a", "e", "u", "i", "ng", "n", "m")

    fun findTonePosition(syllable: String): Int {
        val lower = syllable.lowercase()
        for (pattern in VOWEL_PRIORITY) {
            val index = lower.indexOf(pattern)
            if (index >= 0) return index
        }
        return -1
    }
}
```

**效益**：確保 `goá` 而非 `góa`。

---

### 2.2 合法音節驗證 `P0`

**來源**：khiin-rs
**功能**：即時驗證輸入是否為合法台語音節

```kotlin
object SyllableValidator {
    private val PATTERN = Regex(
        """^·?((chh|[ckpt]h|[bhgjklmnpst])?(iau|io͘|oai|a[iu]|i[aou]|o[ae͘]|ui|[aeiou])?(ng|[mnptkh])?|(chh|[ckpt]h|[hkmnpst])ng?|(ng|m)h?)(n|ⁿ)?$""",
        RegexOption.IGNORE_CASE
    )

    fun isValid(syllable: String): Boolean = PATTERN.matches(syllable)
}
```

**效益**：即時回饋使用者輸入正確性。

---

### 2.3 特殊符號轉換 `P0`

**來源**：khiin-rs
**功能**：處理台語特有符號

| 輸入 | 輸出 | 說明 |
|------|------|------|
| `nn` | `ⁿ` | 鼻音 |
| `ou`/`oo` | `o͘` | 長音 o |
| 數字 | 變音符號 | `a2` → `á` |

```kotlin
object SpecialSymbolConverter {
    fun convert(text: String): String {
        return text
            .replace("nn", "ⁿ")
            .replace(Regex("o[ou]"), "o͘")
            .let { Normalizer.normalize(it, Normalizer.Form.NFC) }
    }
}
```

---

### 2.4 Telex 聲調輸入 `P1`

**來源**：khiin-rs
**功能**：用字母輸入聲調（初學者友善）

| 聲調 | 數字 | Telex |
|------|------|-------|
| 2 | 2 | s |
| 3 | 3 | f |
| 5 | 5 | l |
| 7 | 7 | j |
| 9 | 9 | w |

```kotlin
object TelexConverter {
    private val MAP = mapOf('s' to '2', 'f' to '3', 'l' to '5', 'j' to '7', 'w' to '9')

    fun convert(char: Char): Char? = MAP[char.lowercaseChar()]
}
```

**效益**：不需切換數字鍵盤。

---

### 2.5 POJ/TL 統一輸入 `P2`

**來源**：librime (Spelling Algebra)
**功能**：任一種拼寫都能查到詞彙

```kotlin
object SpellingNormalizer {
    private val POJ_TO_TL = mapOf(
        "ch" to "ts", "chh" to "tsh",
        "oa" to "ua", "oe" to "ue",
        "eng" to "ing", "ek" to "ik"
    )

    fun expandVariants(input: String): List<String> {
        var normalized = input
        POJ_TO_TL.forEach { (poj, tl) -> normalized = normalized.replace(poj, tl) }
        return listOf(input, normalized).distinct()
    }
}
```

**效益**：使用者不需記住用 POJ 還是 TL。

---

### 2.6 模糊音支援 `P2`

**來源**：librime
**功能**：處理常見混淆音

| 混淆 | 範例 |
|------|------|
| n/l | lâng ↔ nâng |
| in/ing | sin ↔ sing |

```kotlin
object FuzzyMatcher {
    private val RULES = listOf("n" to "l", "l" to "n", "in$" to "ing", "ing$" to "in")

    fun expand(input: String): List<Pair<String, Double>> {
        val results = mutableListOf(input to 1.0)
        RULES.forEach { (from, to) ->
            if (input.contains(Regex(from))) {
                results.add(input.replace(Regex(from), to) to 0.5)
            }
        }
        return results
    }
}
```

---

## Phase 3：輸入體驗提升

### 3.1 歷史記錄快捷輸入 `P1`

**來源**：librime
**功能**：快速重複輸入常用詞

```kotlin
object HistoryService {
    private const val MAX_SIZE = 10
    private val history = LinkedList<HistoryEntry>()

    data class HistoryEntry(val hanzi: String, val tl: String, val timestamp: Long)

    fun record(entry: HistoryEntry) {
        history.removeIf { it.hanzi == entry.hanzi }
        history.addFirst(entry)
        if (history.size > MAX_SIZE) history.removeLast()
    }

    fun getHistory(): List<HistoryEntry> = history.toList()
}
```

**觸發**：長按空白鍵。

---

### 3.2 撤銷最近輸入 `P1`

**來源**：librime
**功能**：3 秒內可撤銷錯誤選詞

```kotlin
object UndoService {
    private const val TIMEOUT_MS = 3000L
    private var lastCommit: CommitRecord? = null

    data class CommitRecord(val text: String, val timestamp: Long)

    fun canUndo(): Boolean {
        val record = lastCommit ?: return false
        return System.currentTimeMillis() - record.timestamp < TIMEOUT_MS
    }
}
```

**效益**：避免污染學習資料。

---

### 3.3 反查功能（漢字查羅馬字）`P1`

**來源**：librime
**功能**：輸入漢字查詢發音

```kotlin
object ReverseLookupService {
    fun lookup(hanzi: String, db: SQLiteDatabase): List<ReverseLookupResult> {
        val sql = "SELECT tl, poj FROM words WHERE hanzi = ? ORDER BY frequency DESC LIMIT 10"
        // ...
    }
}
```

**觸發**：特殊前綴（如 `` ` ``）或長按候選詞。

**效益**：台語學習者的好幫手。

---

### 3.4 多層次候選詞查詢 `P1`

**來源**：khiin-rs
**功能**：確保有候選詞

```kotlin
fun query(input: String): List<Candidate> {
    // 1. 精確匹配（帶聲調）
    queryExact(input).takeIf { it.isNotEmpty() }?.let { return it }

    // 2. 模糊匹配（無聲調）
    queryWithoutTone(input).takeIf { it.isNotEmpty() }?.let { return it }

    // 3. 分詞查詢
    return querySegmented(input)
}
```

---

### 3.5 三種輸入模式 `P2`

**來源**：khiin-rs

| 模式 | 說明 | 適用 |
|------|------|------|
| Continuous | 自動連續轉換 | 熟練者 |
| Classic | 音節後顯示候選，數字快選 | 一般 |
| Manual | 手動控制轉換 | 進階 |

```kotlin
enum class InputMode { CONTINUOUS, CLASSIC, MANUAL }

interface InputModeHandler {
    fun onKeyPress(key: KeyEvent): Boolean
    fun shouldShowCandidates(): Boolean
}
```

---

## Phase 4：連續輸入與分詞

### 4.1 動態規劃分詞 `P2`

**來源**：khiin-rs + librime
**功能**：支援連續輸入，減少空白鍵

```kotlin
object DPSegmenter {
    private const val FREQ_BIAS = 1.0
    private const val LEN_BIAS = 0.2

    fun segment(input: String, syllableFreq: Map<String, Int>): List<String> {
        val n = input.length
        val dp = DoubleArray(n + 1) { Double.MAX_VALUE }
        val prev = IntArray(n + 1) { -1 }
        dp[0] = 0.0

        for (i in 0 until n) {
            if (dp[i] == Double.MAX_VALUE) continue
            for (j in (i + 1)..minOf(i + 6, n)) {
                val syl = input.substring(i, j)
                val freq = syllableFreq[syl] ?: continue
                val cost = ln(1.0 / freq.toDouble().pow(FREQ_BIAS)) / j.toDouble().pow(LEN_BIAS)
                if (dp[i] + cost < dp[j]) {
                    dp[j] = dp[i] + cost
                    prev[j] = i
                }
            }
        }

        return backtrack(input, prev)
    }
}
```

---

### 4.2 緩衝區狀態機 `P2`

**來源**：khiin-rs
**功能**：清晰的狀態管理

```kotlin
sealed class EditState {
    object Empty : EditState()
    data class Composing(val buffer: String) : EditState()
    data class Converted(val candidates: List<Candidate>) : EditState()
    data class Selecting(val index: Int) : EditState()
}
```

**狀態轉換**：
```
EMPTY → COMPOSING → CONVERTED → SELECTING → EMPTY
```

---

### 4.3 輕聲處理 `P2`

**來源**：khiin-rs

| 模式 | 範例 |
|------|------|
| Khinless | `ê` |
| Hyphen | `--ê` |
| Dot | `·ê` |

---

## Phase 5：架構升級（長期）

### 5.1 Pipeline 架構 `P3`

**來源**：librime

```
Input → Processor → Segmentor → Translator → Filter → Output
```

**好處**：模組化、易測試、可擴展。

---

### 5.2 配置驅動 `P3`

**來源**：librime

```yaml
schema:
  name: 台語輸入法

speller:
  alphabet: abcdefghijklmnopqrstuvwxyz

translator:
  dictionary: taigi
```

**好處**：不改程式碼即可調整行為。

---

## 實作順序總覽

```
Phase 1：NextWord 優化（1-2 週）
├── 1.1 時間衰減權重 [P0]
├── 1.2 使用者詞彙加分 [P0]
└── 1.3 N-gram 統計 [P2] (可延後)

Phase 2：聲調拼寫（2-3 週）
├── 2.1 聲調位置判斷 [P0]
├── 2.2 合法音節驗證 [P0]
├── 2.3 特殊符號轉換 [P0]
├── 2.4 Telex 聲調 [P1]
├── 2.5 POJ/TL 統一 [P2]
└── 2.6 模糊音 [P2]

Phase 3：輸入體驗（2-3 週）
├── 3.1 歷史記錄 [P1]
├── 3.2 撤銷功能 [P1]
├── 3.3 反查功能 [P1]
├── 3.4 多層次查詢 [P1]
└── 3.5 三種輸入模式 [P2]

Phase 4：連續輸入（3-4 週）
├── 4.1 DP 分詞 [P2]
├── 4.2 緩衝區狀態機 [P2]
└── 4.3 輕聲處理 [P2]

Phase 5：架構升級（未定）
├── 5.1 Pipeline 架構 [P3]
└── 5.2 配置驅動 [P3]
```

---

## 快速參考：功能來源

| 功能 | 來源 | 優先級 |
|------|------|--------|
| 時間衰減權重 | librime | P0 |
| 使用者詞彙加分 | librime | P0 |
| 聲調位置判斷 | khiin-rs | P0 |
| 合法音節驗證 | khiin-rs | P0 |
| 特殊符號轉換 | khiin-rs | P0 |
| Telex 聲調 | khiin-rs | P1 |
| 歷史記錄 | librime | P1 |
| 撤銷功能 | librime | P1 |
| 反查功能 | librime | P1 |
| 多層次查詢 | khiin-rs | P1 |
| POJ/TL 統一 | librime | P2 |
| 模糊音 | librime | P2 |
| 三種輸入模式 | khiin-rs | P2 |
| DP 分詞 | khiin-rs + librime | P2 |
| N-gram 統計 | khiin-rs | P2 |
| 緩衝區狀態機 | khiin-rs | P2 |
| 輕聲處理 | khiin-rs | P2 |
| Pipeline 架構 | librime | P3 |
| 配置驅動 | librime | P3 |

---

## 相關文件

### 參考專案分析
- `spec/khiin.md` - khiin-rs 基礎分析
- `spec/khiin-features.md` - khiin-rs 功能詳解
- `spec/rime.md` - librime 基礎分析
- `spec/rime-future.md` - librime 功能詳解

### 深入專題
- `spec/rime-segmentation.md` - DAG 分詞演算法
- `spec/rime-pipeline.md` - Pipeline 架構
- `spec/rime-config-detail.md` - 配置驅動系統
