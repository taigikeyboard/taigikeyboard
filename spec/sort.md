# 詞頻與排序

## 排序公式

```kotlin
// LexiconService.calculateScore()
score = exactBonus + (10000 - candidateLength * 100) + min(userFreq, 50) * 10 + baseFreq / 10
```

| 項目 | 說明 | 分數 |
|------|------|------|
| exactBonus | 完全匹配加分（roman == input） | +500 |
| lengthScore | 長度懲罰（短詞優先） | 10000 - len*100 |
| userFreq | 使用者頻率（上限 50） | 0 ~ 500 |
| baseFreq | 詞庫基礎頻率 | 0 ~ 數百 |

### 排序優先順序

1. **完全匹配**：roman 完全等於輸入（如輸入 `soo`，`soo` 排在 `soo2` 前面）
2. **長度**：短詞排前面（每增加 1 字元扣 100 分）
3. **使用者頻率**：常用詞排前面
4. **詞庫頻率**：作為同長度詞的排序依據

## 詞頻來源

| 來源 | 資料庫 | 欄位 |
|------|--------|------|
| 詞庫頻率 | `dictionary.db` | `frequency` |
| 使用者頻率 | `user_frequency.db` | `count` |

## 使用者詞頻 API

```kotlin
// UserFrequencyService.kt
recordUsage(word)     // 使用者選詞時 count + 1
getFrequency(word)    // 查詢使用次數
```

## 流程

```
Trie 查詢 → SQLite 批次查詢 → 去重 → 分數排序（applyScoredSort）
```

## MARISA-trie 排序特性

`predictive_search` 按**深度優先**遍歷，長詞先返回：

```
輸入 "gua" 前綴搜尋結果順序：
  1. guan5tsi2tsing（最長）
  2. guan5tsi2tuann7
  ...
  717. gua2（短詞，容易被 limit 截斷）
```

**影響**：短詞可能被 limit 截斷，找不到高頻單音節詞。

**解決**：所有詞都產生 `notone`（包含單音節），讓 `gua` 可完全匹配。
