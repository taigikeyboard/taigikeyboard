# UserFrequency 詞頻與排序

> **類型**: 功能
> **關鍵字**: `UserFrequency`, `Sort`, `Score`, `FrequencyData`
> **相關**: autocomplete.md, trie.md

---

## 重點摘要

- 使用者頻率主導排序（穩定性優先）
- Recency 和完全匹配只做微調
- SQLite 預排序 + 記憶體精排序

---

## 排序公式（v3）

```
score = userFreqScore + recencyBonus + exactBonus + baseFreqScore
```

| 項目 | 說明 | 分數範圍 |
|------|------|----------|
| `userFreqScore` | 使用者頻率（主導） | count × 100，max 10000 |
| `recencyBonus` | 最近 1 小時用過 | +200 |
| `exactBonus` | roman == input | +100 |
| `baseFreqScore` | 詞庫頻率（fallback） | ~0-100 |

---

## 設計理念

1. **使用者頻率主導**：常用詞穩定排前，不因其他因素變動
2. **微調因素**：Recency/exactBonus 只在相近分數時有影響
3. **移除長度懲罰**：讓使用者行為決定排序

---

## 排序優先順序

1. **使用者頻率**：max +10000
2. **Recency**：同頻率時的 tiebreaker，+200
3. **完全匹配**：+100
4. **詞庫頻率**：新詞的 fallback

---

## 資料結構

### FrequencyData

| 欄位 | 類型 | 說明 |
|------|------|------|
| `count` | Int | 使用次數 |
| `lastUsedMillis` | Int64 | 最後使用時間 (ms) |

### 資料庫 Schema

```sql
CREATE TABLE user_frequency (
    word TEXT NOT NULL UNIQUE,
    count INTEGER DEFAULT 1,
    last_used TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
```

---

## 詞頻來源

| 來源 | 資料庫 | 欄位 | 說明 |
|------|--------|------|------|
| 詞庫頻率 | `dictionary.db` | `frequency` | 靜態 |
| 使用者頻率 | `user_frequency.db` | `count` | 動態，選詞累加 |
| 最後使用 | `user_frequency.db` | `last_used` | 動態，選詞更新 |

---

## 平台對照

| 項目 | iOS | Android |
|------|-----|---------|
| 計分 | `TextProcessor.calculateScore()` | `LexiconService.calculateScore()` |
| 頻率服務 | `UserFrequencyService.swift` | `UserFrequencyService.kt` |
| 資料庫 | `UserFrequencyRepository.swift` | 內建於 Service |

---

## API

| 方法 | 說明 |
|------|------|
| `recordUsage(word)` | 選詞時 count+1, last_used=now |
| `getFrequencyData(word)` | 取得單詞頻率資料 |
| `getFrequencyDataBatch(words)` | 批次取得頻率資料 |

---

## 範例

**輸入 `gua`**：

| 候選詞 | userFreq | recency | exact | base | total |
|--------|----------|---------|-------|------|-------|
| `gua2-ho2` (freq=80, 剛用) | 8000 | 200 | 0 | ~10 | ~8210 |
| `gua2` (freq=50) | 5000 | 0 | 0 | ~8 | ~5008 |
| `gua` (freq=0) | 0 | 0 | 100 | ~5 | ~105 |

結果：`gua2-ho2` > `gua2` > `gua`

---

## 注意事項

### MARISA-trie 排序特性

- `predictive_search` 按深度優先，長詞先返回
- 短詞容易被 limit 截斷

### 解決方案

1. 完全匹配 + 前綴搜尋合併
2. SQLite 依 frequency 排序保留高頻詞
3. notone 索引讓無聲調詞可完全匹配
