# 待辦事項

> **關鍵字**: `todo`, `待辦`, `規劃`

---

## NextWord 下一詞預測

### Stage 0: 資料流程調整 ✅
- [x] 修改 `cleanup` 階段：字數限制放寬到 ≤5 字
- [x] 修改 `02_create_app_db.sh`：加入 ≤3 字篩選
- [x] 修改 `03_create_trie_db.sh`：加入 ≤3 字篩選

### Stage 1: 資料準備 ✅
- [x] 撰寫 `dictionary/build/08_generate_association.py`
- [x] ~~從 cleanup 後的資料（≤5 字）產生「首字→詞尾」關聯~~
- [x] ~~輸出 `dictionary/output/word_association.db`~~
- [x] **改用相鄰字 Bigram 模型**（2025-12-20）
- [x] **輸出到 `dictionary.db` 的 `word_association` 表**

### Stage 2: Android 實作 ✅
- [x] ~~部署 `word_association.db` 到 Android assets~~
- [x] 建立 `ime/dictionary/NextWordService.kt`
- [x] 修改 `SmartbarManager.kt`，選詞後顯示關聯詞
- [x] 實作使用者關聯記錄
- [x] **改為從 `dictionary.db` 讀取**（2025-12-20）
- [x] **查詢邏輯改為「最後一字」（Bigram 模型）**

### Stage 2.5: Android Debug ✅
- [x] 修復 companion object 重複定義錯誤
- [x] 修復 `prefs.inputMode` 類型比較
- [x] 修復 NextWord 候選詞點擊無輸出問題
- [x] 移除錯誤的 fallback 邏輯，改為過濾
- [x] 修復 PRAGMA 語句執行方式 (`rawQuery`)
- [x] 修復 NextWord 候選詞使用組字背景問題（index 0 不套用組字背景）
- [x] 退格鍵觸發 NextWord 重新預測（文字清空時清除候選詞）
- [x] 修復 recordAssociation 未傳入羅馬字問題（user_association.db 缺少 tl/poj）
- [x] 驗證 NextWord 功能正常運作（Bigram 模型 + delimiter）(2025-12-20 Code Review)

### Stage 3: 清理 ✅ (2025-12-20)
- [x] 移除 `android/app/src/main/assets/word_association.db`
- [x] 移除 `ios/Resources/Dictionaries/word_association.db`
- [x] 移除 `dictionary/output/word_association.db`
- [x] 移除 `05_deploy.sh` 中的 `word_association.db` 複製邏輯
- [x] 移除 `NextWordService.kt` 中的版本檔案邏輯

### Stage 3.5: 分隔符 (delimiter) ✅ → 已簡化 (2025-12-20)
- [x] `08_generate_association.py` 加入 `delimiter` 欄位（預設 `-`）
- [x] `NextWordService.kt` 支援 delimiter（Prediction, recordAssociation）
- [x] `DictionaryModels.kt` TaigiWord 加入 delimiter 欄位
- [x] ~~`SmartbarManager.kt` 追蹤 `lastInputDelimiter`~~ (已移除)
- [x] ~~`SmartbarManager.kt` 羅馬字模式輸出時加上 delimiter~~ (已移除)
- [x] ~~`TextInputManager.kt` 輸入 `-` 時呼叫 `setInputDelimiter("-")`~~ (已移除)
- [x] 重新產生並部署 `dictionary.db`
- [x] 更新 `spec/nextword.md` 文件
- [x] **簡化：使用者自行輸入分隔符，移除自動判斷邏輯**

### Stage 4: 優化 ✅ (2025-12-20)
- [x] ~~調整權重參數（user ×50, dict ×1）~~ 加入時間衰減
- [x] 限制使用者關聯數量上限（50,000 筆）
- [x] 清除學習資料（整合到「挕掉捷用詞紀錄」選項）

---

## 架構變更記錄

### 2025-12-20: 分隔符 (delimiter) 簡化

**變更摘要**：
- ~~羅馬字模式輸出 NextWord 時自動加上正確的分隔符~~ (已移除)
- 使用者自行輸入分隔符（`-` 或空格）
- delimiter 欄位保留在資料庫，但不自動使用
**修改的檔案**：
| 檔案 | 變更 |
|------|------|
| `08_generate_association.py` | 加入 `delimiter="-"` 欄位 |
| `NextWordService.kt` | Prediction/recordAssociation 保留 delimiter 參數 |
| `DictionaryModels.kt` | TaigiWord 加入 delimiter 欄位 |
| `SmartbarManager.kt` | ~~追蹤 lastInputDelimiter~~ → 已移除 |
| `TextInputManager.kt` | ~~呼叫 setInputDelimiter~~ → 已移除 |

---

### 2025-12-20: Association 併入 dictionary.db + Bigram 模型

**變更摘要**：
- 將 `word_association` 表併入 `dictionary.db`
- 模型從「首字→詞尾」改為「相鄰字 Bigram」

**舊模型（首字→詞尾）**：
```
「早起床」→ 早 → 起床
```

**新模型（相鄰字 Bigram）**：
```
「早起床」→ 早 → 起、起 → 床
```

**修改的檔案**：
| 檔案 | 變更 |
|------|------|
| `dictionary/build/08_generate_association.py` | Bigram 演算法 + 輸出到 dictionary.db |
| `dictionary/build/05_deploy.sh` | 移除 word_association.db 複製 |
| `NextWordService.kt` | 從 dictionary.db 讀取 + 查詢用最後一字 |

**刪除的檔案**：
- `android/app/src/main/assets/word_association.db`
- `ios/Resources/Dictionaries/word_association.db`

---

## 其他待辦

### NextWord 詞層級優化 ✅ (2025-12-20)

**目標**：NextWord 建議完整詞（2 字以上），而非只有單字

**實作方式**：混合模式
- 字典關聯：維持字元層級（冷啟動）
- 使用者關聯：改為詞層級（學習後）

**已完成**：
- [x] 修改 `SmartbarManager.kt`：記錄完整詞到 user_association
- [x] 修改歷史視窗：記錄完整詞而非單字

**效果**：
- 字典查詢：用最後一字 → 預測單字（冷啟動）
- 使用者查詢：用完整詞 → 預測完整詞（學習後）
