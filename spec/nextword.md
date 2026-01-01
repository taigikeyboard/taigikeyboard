# NextWord 下一詞預測

> **類型**: 規劃
> **關鍵字**: `NextWord`, `Bigram`, `WordAssociation`, `UserLearning`
> **相關**: autocomplete.md, sort.md

---

## 重點摘要

- 組字結束後預測下一個可能的詞
- 混合式 Bigram：字典（字元層級）+ 使用者學習（詞層級）
- 時間衰減權重（半衰期一週）
- 使用者關聯上限 50,000 筆

---

## 資料模型

### 混合式 Bigram

| 來源 | 查詢 key | 預測結果 | 說明 |
|------|----------|----------|------|
| 字典 | 選中詞的最後一字 | 單字 | 冷啟動 |
| 使用者 | 選中詞的完整詞 | 完整詞 | 學習後 |

### 資料庫結構

```sql
-- 字典關聯 (dictionary.db)
CREATE TABLE word_association (
    prev_word TEXT NOT NULL,   -- 前一字（單字）
    next_word TEXT NOT NULL,   -- 下一字（單字）
    next_tl TEXT,
    count INTEGER DEFAULT 1,
    UNIQUE(prev_word, next_word)
);

-- 使用者學習 (user_association.db)
CREATE TABLE user_association (
    prev_word TEXT NOT NULL,   -- 前一詞（完整詞）
    next_word TEXT NOT NULL,   -- 下一詞（完整詞）
    next_tl TEXT,
    count INTEGER DEFAULT 1,
    last_used TIMESTAMP,
    UNIQUE(prev_word, next_word)
);
```

---

## 核心流程

```
使用者選擇候選詞
    ↓
記錄關聯（若有前一詞，且間隔 < 10 秒）
    ↓
查詢預測
  ├─ 字典：WHERE prev_word = 最後一字
  └─ 使用者：WHERE prev_word = 完整詞
    ↓
時間衰減計算
    ↓
合併排序，顯示候選詞
```

---

## 權重計算

### 分數公式

```kotlin
// 使用者分數 = count × 50 × decay
// 字典分數 = count × 1

decay = exp(-ageHours / 168.0 * 0.693)  // 半衰期一週
```

### 衰減效果

| 時間 | 衰減因子 |
|------|----------|
| 剛使用 | 1.0 |
| 1 週後 | 0.5 |
| 2 週後 | 0.25 |
| 1 月後 | 0.06 |

---

## 觸發與重置

### 觸發時機

| 操作 | 漢字模式 | 羅馬字模式 |
|------|----------|------------|
| 點擊候選詞 | 觸發 | 觸發 |
| Enter 確認 | 不觸發 | 觸發 |

### 上下文重置

- 輸入句末標點（。！？）
- 長時間無輸入（> 30 秒）
- 切換輸入框
- 退格清空文字

---

## 雜訊過濾

不記錄為關聯的字元：
- 標點符號
- 空白（半形/全形）
- 數字 0-9

---

## 實作狀態

### Android ✅

- NextWordService.kt（predict, recordAssociation）
- 時間衰減計算
- 使用者關聯上限（50,000 筆）
- 雜訊過濾

### iOS ⏳

- 待實作

---

## 平台對照

| 元件 | Android | iOS |
|------|---------|-----|
| 服務 | `NextWordService.kt` | 待實作 |
| 管理 | `SmartbarManager.kt` | 待實作 |
| 資料庫 | `dictionary.db` + `user_association.db` | 同 |
