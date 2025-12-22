# NextWord 下一詞預測 - iOS Debug 中

> 參考文件：`spec/nextword.md`

## 當前狀態

🔧 **iOS Debug 中** - 編譯錯誤已修正，功能測試進行中

### 已修復問題
- [x] **空白鍵導致 NextWord 候選詞消失** - 非組字模式按空白鍵時不觸發 `performAutocomplete()`，對齊 Android 行為
- [x] **連字符導致 NextWord 候選詞消失** - 非組字模式輸入 "-" 且正在顯示 NextWord 時不觸發 `performAutocomplete()`
- [x] **NextWord 候選詞 title/subtitle 交換錯誤** - 統一使用 `text=羅馬字, subtitle=漢字` 格式，讓 CandidateView 統一處理交換
- [x] **選擇組字文字候選詞時連字符重複** - 移除「保留前綴邏輯」，直接使用候選詞文字，對齊 Android

## Android 實作盤點（已完成 ✓）

### 核心檔案
| 檔案 | 職責 |
|------|------|
| `NextWordService.kt` | 預測查詢、關聯記錄、時間衰減、資料清理 |
| `SmartbarManager.kt` | 候選詞管理、NextWord 觸發、上下文追蹤 |
| `TextInputManager.kt` | Enter 確認觸發 |

### 已實作功能
- [x] 混合式 Bigram 模型
- [x] 字典關聯查詢
- [x] 使用者關聯
- [x] 候選詞點擊觸發 NextWord
- [x] Enter 確認觸發（羅馬字模式）
- [x] 上下文重置（標點、超時、切換輸入框）
- [x] 時間衰減
- [x] 使用者關聯上限
- [x] 雜訊過濾
- [x] 複合詞內部關聯記錄
- [x] 退格後重新預測
- [x] 空白確認記錄 lastSelectedWord
- [x] NextWord 模式下 "-" 保留候選詞
- [x] 羅馬字模式過濾無羅馬字候選詞

---

## iOS 實作進度（Debug 中 🔧）

### 核心檔案
| iOS 檔案 | 狀態 | 說明 |
|----------|------|------|
| `NextWordService.swift` | ✓ | 預測查詢、關聯記錄（~450 行）|
| `ActionHandler.swift` | ✓ | 狀態追蹤、Timer、輔助方法 |
| `ActionHandler+Suggestions.swift` | ✓ | 候選詞選擇後觸發、複合詞關聯 |
| `ActionHandler+CharacterInput.swift` | ✓ | 輸入處理、退格重新預測 |
| `KeyboardViewController.swift` | ✓ | 切換輸入框重置 |
| `SettingsView.swift` | ✓ | 清除學習資料 |

### 已修正的編譯錯誤
- [x] 字串中智慧引號轉義（ActionHandler.swift:192）
- [x] `private(set)` 改為 `internal`（extension 存取問題）
- [x] `suggestions` → `suggestionsFromService`（KeyboardKit API）

### 功能對照表

| 功能 | Android | iOS |
|------|---------|-----|
| 混合式 Bigram 模型 | ✓ | ✓ |
| 字典關聯查詢（最後一字） | ✓ | ✓ |
| 使用者關聯查詢（完整詞） | ✓ | ✓ |
| 候選詞點擊觸發 NextWord | ✓ | ✓ |
| 時間衰減（半衰期一週） | ✓ | ✓ |
| 使用者關聯上限（50,000 筆） | ✓ | ✓ |
| 雜訊過濾 | ✓ | ✓ |
| 句末標點重置 | ✓ | ✓ |
| 超時重置（30 秒） | ✓ | ✓ |
| 切換輸入框重置 | ✓ | ✓ |
| 清除學習資料 | ✓ | ✓ |
| 詞庫過濾 | ✓ | ✓ |
| 複合詞內部關聯記錄 | ✓ | ✓ |
| NextWord 模式下 "-" 保留候選詞 | ✓ | ✓ |
| 退格後重新預測 | ✓ | ✓ |
| 空白確認記錄 lastSelectedWord | ✓ | ✓ |
| Enter 觸發 NextWord（羅馬字模式） | ✓ | ✓ |
| 羅馬字模式過濾無羅馬字候選詞 | ✓ | ✓ |
| 空白鍵保留 NextWord 候選詞 | ✓ | ✓ |

---

## 備註

- iOS 使用 KeyboardKit，候選詞選擇流程與 Android 不同
- iOS 的 `dictionary.db` 位於 `ios/Resources/Dictionaries/dictionary.db`（40MB，含 106,387 筆 word_association）
- 使用者關聯資料存放於 Application Support 目錄
