# khiin-rs 功能借鑑清單

> **用途**：整理 khiin-rs 中對台語鍵盤有價值的功能
> **更新日期**：2025-12-19

---

## 優先級分類

| 優先級 | 說明 | 複雜度 |
|--------|------|--------|
| P0 | 立即可用，直接參考 | 低 |
| P1 | 短期整合，需小幅修改 | 中 |
| P2 | 中期規劃，需架構調整 | 中高 |
| P3 | 長期參考，大幅重構 | 高 |

---

## P0 - 立即可用

### 1. 聲調位置判斷演算法

**來源**：`ji/src/lomaji.rs` - `get_tone_position()`

**功能**：自動判斷聲調符號應該標記在哪個母音上。

**規則**：
```
優先順序: oa > o > a > e > u > i > ng > n > m
```

**Kotlin 實作建議**：
```kotlin
object TonePositionFinder {
    private val VOWEL_PRIORITY = listOf("oa", "o", "a", "e", "u", "i", "ng", "n", "m")

    fun findTonePosition(syllable: String): Int {
        val lower = syllable.lowercase()

        for (pattern in VOWEL_PRIORITY) {
            val index = lower.indexOf(pattern)
            if (index >= 0) {
                // 返回該模式的第一個字母位置
                return index
            }
        }

        return -1  // 找不到
    }
}
```

**效益**：確保聲調符號正確放置（如 `goá` 而非 `góa`）。

---

### 2. 合法音節驗證

**來源**：`ji/src/lomaji.rs` - `is_legal_lomaji()`

**功能**：用 Regex 驗證輸入是否為合法台語音節。

**Regex 模式**：
```regex
^·?((chh|[ckpt]h|[bhgjklmnpst])?(iau|io͘|oai|a[iu]|i[aou]|o[ae͘]|ui|[aeiou])?(ng|[mnptkh])?|(chh|[ckpt]h|[hkmnpst])ng?|(ng|m)h?)(n|ⁿ|ᴺ)?$
```

**Kotlin 實作建議**：
```kotlin
object SyllableValidator {
    private val LEGAL_SYLLABLE_PATTERN = Regex(
        """^·?((chh|[ckpt]h|[bhgjklmnpst])?(iau|io͘|oai|a[iu]|i[aou]|o[ae͘]|ui|[aeiou])?(ng|[mnptkh])?|(chh|[ckpt]h|[hkmnpst])ng?|(ng|m)h?)(n|ⁿ|ᴺ)?$""",
        RegexOption.IGNORE_CASE
    )

    fun isValid(syllable: String): Boolean {
        return LEGAL_SYLLABLE_PATTERN.matches(syllable)
    }
}
```

**效益**：即時回饋使用者輸入是否正確。

---

### 3. 特殊符號轉換

**來源**：`ji/src/lomaji.rs`

**功能**：處理台語特有的符號轉換。

| 輸入 | 輸出 | 說明 |
|------|------|------|
| `nn` | `ⁿ` | 鼻音 |
| `ou` / `oo` | `o͘` | 長音 o |
| 數字聲調 | 變音符號 | 如 `a2` → `á` |

**Kotlin 實作建議**：
```kotlin
object SpecialSymbolConverter {
    // 鼻音轉換
    fun convertNasalization(text: String): String {
        return text.replace("nn", "ⁿ")
    }

    // 長音 o 轉換
    fun convertLongO(text: String): String {
        return text
            .replace("ou", "o͘")
            .replace("oo", "o͘")
    }

    // Unicode NFC 正規化
    fun normalize(text: String): String {
        return java.text.Normalizer.normalize(text, java.text.Normalizer.Form.NFC)
    }
}
```

---

### 4. Telex 聲調輸入

**來源**：`ji/src/tone.rs`

**功能**：用字母代替數字輸入聲調（初學者友善）。

**對應表**：
| 聲調 | 數字 | Telex |
|------|------|-------|
| 1 | 1 | 1 |
| 2 | 2 | s |
| 3 | 3 | f |
| 4 | 4 | 4 |
| 5 | 5 | l |
| 6 | 6 | 6 |
| 7 | 7 | j |
| 8 | 8 | j |
| 9 | 9 | w |

**Kotlin 實作建議**：
```kotlin
object TelexToneConverter {
    private val TELEX_TO_NUMERIC = mapOf(
        's' to '2',
        'f' to '3',
        'l' to '5',
        'j' to '7',  // 也可能是 8
        'w' to '9'
    )

    fun convert(char: Char): Char? {
        return TELEX_TO_NUMERIC[char.lowercaseChar()]
    }

    fun isTelex(char: Char): Boolean {
        return char in "sflj" || char in "SFLJ"
    }
}
```

**效益**：不需切換到數字鍵盤即可輸入聲調。

---

## P1 - 短期整合

### 5. 三種輸入模式

**來源**：`khiin/src/engine.rs`

**模式說明**：

| 模式 | 說明 | 適用場景 |
|------|------|----------|
| **Continuous** | 自動連續轉換，即時顯示候選 | 熟練使用者 |
| **Classic** | 每個音節後顯示候選，數字快選 | 一般使用者 |
| **Manual** | 完全手動控制轉換時機 | 進階使用者 |

**Kotlin 設計建議**：
```kotlin
enum class InputMode {
    CONTINUOUS,  // 連續模式
    CLASSIC,     // 傳統模式
    MANUAL       // 手動模式
}

interface InputModeHandler {
    fun onKeyPress(key: KeyEvent): Boolean
    fun onSpace(): Boolean
    fun onEnter(): Boolean
    fun shouldShowCandidates(): Boolean
}

class ClassicModeHandler : InputModeHandler {
    override fun onKeyPress(key: KeyEvent): Boolean {
        // 數字鍵直接選擇候選詞
        if (key.isDigit && hasCandidates()) {
            selectCandidate(key.digit - 1)
            return true
        }
        // ...
    }
}
```

**效益**：滿足不同使用者習慣。

---

### 6. 動態規劃分詞

**來源**：`khiin/src/data/segmenter.rs`

**演算法**：使用 DP 找出最佳分詞方式。

**成本計算**：
```rust
const FREQUENCY_BIAS: f64 = 1.0;      // 頻率權重
const LETTER_COUNT_BIAS: f64 = 0.2;   // 字母長度偏好
const SYLLABLE_COUNT_BIAS: f64 = 0.2; // 音節數量偏好

cost = (1.0 / frequency.powf(FREQUENCY_BIAS)).ln();
cost = cost / letter_bias * syllable_bias;
```

**Kotlin 實作建議**：
```kotlin
object DPSegmenter {
    private const val FREQUENCY_BIAS = 1.0
    private const val LETTER_COUNT_BIAS = 0.2
    private const val SYLLABLE_COUNT_BIAS = 0.2

    fun segment(input: String, syllableFreq: Map<String, Int>): List<String> {
        val n = input.length
        val dp = DoubleArray(n + 1) { Double.MAX_VALUE }
        val prev = IntArray(n + 1) { -1 }
        dp[0] = 0.0

        for (i in 0 until n) {
            if (dp[i] == Double.MAX_VALUE) continue

            for (j in (i + 1)..minOf(i + MAX_SYLLABLE_LEN, n)) {
                val syllable = input.substring(i, j)
                val freq = syllableFreq[syllable] ?: continue

                val cost = calculateCost(freq, syllable.length, 1)
                if (dp[i] + cost < dp[j]) {
                    dp[j] = dp[i] + cost
                    prev[j] = i
                }
            }
        }

        return backtrack(input, prev)
    }

    private fun calculateCost(freq: Int, letterCount: Int, syllableCount: Int): Double {
        val baseCost = ln(1.0 / freq.toDouble().pow(FREQUENCY_BIAS))
        val letterBias = letterCount.toDouble().pow(LETTER_COUNT_BIAS)
        val syllableBias = syllableCount.toDouble().pow(SYLLABLE_COUNT_BIAS)
        return baseCost / letterBias * syllableBias
    }
}
```

**效益**：支援連續輸入，減少空白鍵次數。

---

### 7. 多層次候選詞查詢

**來源**：`khiin/src/input/converter.rs`

**查詢策略**：
1. 先查帶聲調的精確匹配
2. 再查無聲調的模糊匹配
3. 最後查可分詞的長句

**SQL 查詢範例**：
```sql
-- 基本查詢
SELECT c.*
FROM conversion_lookups c
LEFT JOIN unigrams u ON c.output = u.gram
WHERE c.key_sequence = :query
ORDER BY
    u.n DESC,        -- Unigram 頻率優先
    c.weight DESC    -- 權重次之
```

**Kotlin 實作建議**：
```kotlin
class CandidateQueryService(private val db: SQLiteDatabase) {

    fun query(input: String, withTone: Boolean): List<Candidate> {
        // 1. 精確匹配（帶聲調）
        if (withTone) {
            val exact = queryExact(input)
            if (exact.isNotEmpty()) return exact
        }

        // 2. 模糊匹配（無聲調）
        val fuzzy = queryWithoutTone(input)
        if (fuzzy.isNotEmpty()) return fuzzy

        // 3. 分詞查詢
        return querySegmented(input)
    }

    private fun queryExact(input: String): List<Candidate> {
        val sql = """
            SELECT output, tl, poj, frequency
            FROM words
            WHERE tl = ? OR poj = ?
            ORDER BY frequency DESC
            LIMIT 30
        """
        return db.rawQuery(sql, arrayOf(input, input)).use { cursor ->
            // ...
        }
    }
}
```

**效益**：確保有候選詞，提升使用體驗。

---

## P2 - 中期規劃

### 8. N-gram 頻率統計

**來源**：`khiin/src/db/migrations/001/up.sql`

**資料結構**：
```sql
-- Unigram：單字頻率
CREATE TABLE unigrams (
    gram TEXT NOT NULL UNIQUE,
    n INTEGER NOT NULL
);

-- Bigram：雙字組合頻率
CREATE TABLE bigrams (
    lgram TEXT,      -- 左詞
    rgram TEXT,      -- 右詞
    n INTEGER NOT NULL,
    UNIQUE(lgram, rgram)
);
```

**排序公式**：
```sql
ORDER BY
    bigram.n DESC,   -- Bigram 優先
    unigram.n DESC,  -- Unigram 次之
    weight DESC      -- 權重最後
```

**效益**：根據上下文提供更準確的候選詞。

---

### 9. 緩衝區狀態機

**來源**：`khiin/src/buffer/buffer_mgr.rs`

**狀態定義**：
```rust
enum EditState {
    ES_EMPTY,      // 空白
    ES_COMPOSING,  // 組字中
    ES_CONVERTED,  // 已轉換
    ES_SELECTING,  // 選擇候選中
    ES_ILLEGAL,    // 非法輸入
}
```

**狀態轉換**：
```
EMPTY → COMPOSING (輸入字母)
COMPOSING → CONVERTED (顯示候選)
CONVERTED → SELECTING (選擇候選)
SELECTING → EMPTY (確認選擇)
COMPOSING → ILLEGAL (非法輸入)
ILLEGAL → COMPOSING (繼續輸入)
```

**Kotlin 設計建議**：
```kotlin
sealed class EditState {
    object Empty : EditState()
    data class Composing(val buffer: String) : EditState()
    data class Converted(val candidates: List<Candidate>) : EditState()
    data class Selecting(val index: Int) : EditState()
    data class Illegal(val buffer: String) : EditState()
}

class BufferManager {
    private var state: EditState = EditState.Empty

    fun onInput(char: Char) {
        state = when (state) {
            is EditState.Empty -> EditState.Composing(char.toString())
            is EditState.Composing -> {
                val newBuffer = (state as EditState.Composing).buffer + char
                if (isValidInput(newBuffer)) {
                    EditState.Composing(newBuffer)
                } else {
                    EditState.Illegal(newBuffer)
                }
            }
            // ...
        }
    }
}
```

**效益**：清晰的狀態管理，減少 bug。

---

### 10. 輕聲處理

**來源**：`khiin/src/config/conf.rs`

**模式**：
| 模式 | 說明 | 範例 |
|------|------|------|
| Khinless | 不顯示輕聲 | `ê` |
| Hyphen | 連字符表示輕聲 | `--ê` |
| Dot | 點號表示輕聲 | `·ê` |

**資料庫支援**：
```sql
-- conversions 表包含輕聲欄位
khin_ok INTEGER,     -- 支援輕聲
khinless_ok INTEGER  -- 支援非輕聲
```

**效益**：支援不同的輕聲標記習慣。

---

## P3 - 長期參考

### 11. Protobuf 通訊協定

**來源**：`protos/src/command.proto`

**優點**：
- 跨平台一致性（Android/iOS/Desktop）
- 高效的二進位序列化
- 強類型定義

**考量**：需要引入 Protobuf 依賴。

---

### 12. Rust 引擎 + JNI

**來源**：`android/app/src/main/kotlin/.../EngineManager.kt`

**架構**：
```
Kotlin UI ←→ JNI ←→ Rust Engine
```

**優點**：
- 高效能核心引擎
- 跨平台共用邏輯

**考量**：增加開發複雜度。

---

## 實作建議順序

```
Phase 1（聲調處理）
├── P0-1. 聲調位置判斷
├── P0-2. 合法音節驗證
├── P0-3. 特殊符號轉換
└── P0-4. Telex 聲調輸入

Phase 2（候選詞優化）
├── P1-5. 三種輸入模式
├── P1-6. 動態規劃分詞
└── P1-7. 多層次查詢

Phase 3（進階功能）
├── P2-8. N-gram 頻率統計
├── P2-9. 緩衝區狀態機
└── P2-10. 輕聲處理

Phase 4（架構升級）
├── P3-11. Protobuf 協定
└── P3-12. Rust 引擎
```

---

## 關鍵檔案清單

### 必讀（直接參考）
| 檔案 | 內容 |
|------|------|
| `ji/src/lomaji.rs` | 聲調轉換、音節驗證 |
| `ji/src/tone.rs` | 聲調處理、Telex |
| `khiin/src/input/syllable.rs` | 音節結構 |

### 參考（需改造）
| 檔案 | 內容 |
|------|------|
| `khiin/src/data/segmenter.rs` | DP 分詞 |
| `khiin/src/input/converter.rs` | 候選詞生成 |
| `khiin/src/buffer/buffer_mgr.rs` | 緩衝區管理 |
| `khiin/src/db/migrations/001/up.sql` | 資料庫結構 |

---

## 相關文件

- `spec/khiin.md` - khiin-rs 基礎分析（N-gram、DP 分割）
- `spec/rime-future.md` - librime 功能借鑑清單
