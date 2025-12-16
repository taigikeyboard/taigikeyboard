# 詞頻與排序

## 概述

候選詞排序流程：
1. **SQLite 查詢**：依 `frequency`（詞庫頻率）降序排列
2. **記憶體排序**：根據使用者頻率為主，搭配 Recency 和完全匹配做微調

---

## 排序公式（v3 - 簡化版）

```
score = userFreqScore + recencyBonus + exactBonus + baseFreqScore
```

| 項目 | 說明 | 分數 |
|------|------|------|
| userFreqScore | 使用者頻率（主導，上限 100） | count * 100 (max 10000) |
| recencyBonus | 最近 1 小時內用過（微調） | +200 |
| exactBonus | 完全匹配（roman == input，微調） | +100 |
| baseFreqScore | 詞庫基礎頻率（新詞 fallback） | baseFreq / 10 (~0-100) |

### 設計理念

1. **使用者頻率主導排序**（穩定性優先）
   - 常用詞排在前面，不會因其他因素大幅變動
   - 上限 100 次，避免極端值

2. **Recency 和 exactBonus 只做微調**
   - 不會讓低頻詞超過高頻詞
   - 只在相近分數時有影響

3. **移除長度懲罰**
   - 讓使用者行為決定排序
   - 詞庫頻率已隱含長度資訊

### 排序優先順序

1. **使用者頻率**：常用詞排前面 (max +10000)
2. **Recency**：最近 1 小時內用過 (+200，同頻率時的 tiebreaker)
3. **完全匹配**：roman 完全等於輸入 (+100)
4. **詞庫頻率**：新詞的 fallback 排序

---

## 平台實作

### Android

```kotlin
// LexiconService.calculateScore()
private fun calculateScore(
    word: TaigiWord,
    normalizedInput: String,
    frequencyData: UserFrequencyService.FrequencyData
): Int {
    val candidateRoman = word.roman.replace("-", "").lowercase()

    // 使用者頻率（主導因素，上限 100，max 10000）
    val cappedUserFreq = minOf(frequencyData.count, 100)
    val userFreqScore = cappedUserFreq * 100

    // Recency 加分（微調，最近 1 小時內用過 +200）
    val currentTime = System.currentTimeMillis()
    val oneHourMillis = 60 * 60 * 1000L
    val recencyBonus = if (frequencyData.lastUsedMillis > 0 &&
        (currentTime - frequencyData.lastUsedMillis) < oneHourMillis) {
        200
    } else {
        0
    }

    // 完全匹配加分（微調，+100）
    val exactBonus = if (candidateRoman == normalizedInput) 100 else 0

    // 詞庫頻率（新詞 fallback，約 0-100）
    val baseFreqScore = (word.lengthScore ?: 0) / 10

    return userFreqScore + recencyBonus + exactBonus + baseFreqScore
}
```

### iOS

```swift
// TextProcessor.calculateScore()
static func calculateScore(
    word: TaigiWord,
    normalizedInput: String,
    frequencyData: UserFrequencyService.FrequencyData
) -> Int {
    let candidateRoman = word.roman
        .replacingOccurrences(of: "-", with: "")
        .lowercased()

    // 使用者頻率（主導因素，上限 100，max 10000）
    let cappedUserFreq = min(frequencyData.count, 100)
    let userFreqScore = cappedUserFreq * 100

    // Recency 加分（微調，最近 1 小時內用過 +200）
    let currentTime = Int64(Date().timeIntervalSince1970 * 1000)
    let oneHourMillis: Int64 = 60 * 60 * 1000
    let recencyBonus: Int
    if frequencyData.lastUsedMillis > 0 &&
        (currentTime - frequencyData.lastUsedMillis) < oneHourMillis {
        recencyBonus = 200
    } else {
        recencyBonus = 0
    }

    // 完全匹配加分（微調，+100）
    let exactBonus = (candidateRoman == normalizedInput) ? 100 : 0

    // 詞庫頻率（新詞 fallback，約 0-100）
    let baseFreqScore = (word.lengthScore ?? 0) / 10

    return userFreqScore + recencyBonus + exactBonus + baseFreqScore
}
```

---

## 平台一致性

| 項目 | Android | iOS | 一致 |
|------|---------|-----|------|
| 使用者頻率上限 | 100 | 100 | ✓ |
| 使用者頻率權重 | * 100 | * 100 | ✓ |
| Recency 加分 | 最近 1 小時 +200 | 最近 1 小時 +200 | ✓ |
| 完全匹配 | +100 | +100 | ✓ |
| 詞庫頻率權重 | / 10 | / 10 | ✓ |

---

## 排序範例

**輸入 `gua`**：

| 候選詞 | userFreq | recency | exact | base | total |
|--------|----------|---------|-------|------|-------|
| `gua2-ho2` (freq=80, 剛用過) | 8000 | 200 | 0 | ~10 | ~8210 |
| `gua2` (freq=50) | 5000 | 0 | 0 | ~8 | ~5008 |
| `gua` (freq=0) | 0 | 0 | 100 | ~5 | ~105 |

**結果**：`gua2-ho2` > `gua2` > `gua`

**說明**：
- 使用者頻率主導排序
- `gua2-ho2` 因高頻率穩定排第一
- 即使 `gua` 完全匹配（+100），仍無法超過有頻率的詞

---

## 資料結構

### FrequencyData

```kotlin
// Android
data class FrequencyData(
    val count: Int,
    val lastUsedMillis: Long  // Unix timestamp in milliseconds
)
```

```swift
// iOS
struct FrequencyData {
    let count: Int
    let lastUsedMillis: Int64  // Unix timestamp in milliseconds
}
```

### 資料庫 Schema

```sql
CREATE TABLE user_frequency (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    word TEXT NOT NULL UNIQUE,
    count INTEGER DEFAULT 1,
    last_used TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_word ON user_frequency(word);
CREATE INDEX idx_count ON user_frequency(count DESC);
CREATE INDEX idx_last_used ON user_frequency(last_used DESC);
```

---

## 詞頻來源

| 來源 | 資料庫 | 欄位 | 說明 |
|------|--------|------|------|
| 詞庫頻率 | `dictionary.db` | `frequency` | 靜態，詞庫建置時決定 |
| 使用者頻率 | `user_frequency.db` | `count` | 動態，選詞時累加 |
| 最後使用時間 | `user_frequency.db` | `last_used` | 動態，選詞時更新 |

---

## 使用者詞頻 API

### Android

```kotlin
// UserFrequencyService.kt
object UserFrequencyService {
    suspend fun recordUsage(word: String)  // 選詞時 count + 1, last_used = now
    suspend fun getFrequency(word: String): Int
    suspend fun getFrequencyData(word: String): FrequencyData
    suspend fun getFrequencyDataBatch(words: List<String>): Map<String, FrequencyData>
}
```

### iOS

```swift
// UserFrequencyService.swift
final class UserFrequencyService {
    func recordUsage(for word: String)  // 選詞時 count + 1, last_used = now
    func getFrequency(for word: String) -> Int
    func getFrequencyData(for word: String) -> FrequencyData
    func getFrequencyDataBatch(for words: [String]) -> [String: FrequencyData]
}
```

---

## SQLite 預排序

兩平台都在 SQLite 查詢時依 `frequency` 排序：

```sql
SELECT id, roman, hanzi, frequency
FROM dictionary
WHERE id IN (...)
ORDER BY frequency DESC
LIMIT ?
```

這確保高頻詞不會被 limit 截斷。

---

## MARISA-trie 排序特性

`predictive_search` 按**深度優先**遍歷，長詞先返回：

```
輸入 "gua" 前綴搜尋結果順序：
  1. guan5tsi2tsing（最長）
  2. guan5tsi2tuann7
  ...
  717. gua2（短詞，容易被 limit 截斷）
```

**解決方案**：
1. 完全匹配（`lookup`）+ 前綴搜尋（`prefixSearch`）合併
2. SQLite 依 `frequency` 排序保留高頻詞
3. 產生 `notone` 索引讓無聲調詞可完全匹配

---

## 版本歷史

### v3（當前）
- 簡化公式，移除長度懲罰
- 使用者頻率主導（* 100，max 10000）
- Recency 和 exactBonus 降為微調（+200 / +100）
- 設計理念：穩定性優先，讓使用者行為決定排序

### v2
- 新增 Recency 加分（最近 1 小時 +2000）
- 長度懲罰改用對數（降低權重）
- 使用者頻率上限提高至 100，權重 * 30
- 問題：長度懲罰仍會影響排序，recency 權重過高

### v1
- 原始公式：`exactBonus + (10000 - len*100) + min(freq, 50)*10 + base/10`
- 問題：長度權重過高，常用長詞無法排到前面

---

## 相關檔案

| 平台 | 檔案 | 說明 |
|------|------|------|
| Android | `LexiconService.kt` | `calculateScore()`, `applyScoredSort()` |
| Android | `UserFrequencyService.kt` | 使用者頻率服務，`FrequencyData` |
| iOS | `TextProcessor.swift` | `calculateScore()`, `sortByScore()` |
| iOS | `LexiconService.swift` | 呼叫 TextProcessor 排序 |
| iOS | `UserFrequencyService.swift` | 使用者頻率服務 |
| iOS | `UserFrequencyRepository.swift` | 使用者頻率資料庫，`FrequencyData` |
