# Android / iOS 實作一致性追蹤

> **目的**：追蹤兩平台實作差異，規劃未來統一方向
> **更新日期**：2025-12-22

---

## 概述

本文件記錄 Android 和 iOS 在核心模組的實作差異，標記未來應該一致的部分。

---

## 1. ToneConverter（聲調轉換）

### 檔案結構

| 平台 | 檔案 | 說明 |
|------|------|------|
| Android | `ToneConverter.kt` | 主轉換邏輯（443 行） |
| Android | `ToneConverterModels.kt` | 資料模型與映射表（214 行） |
| iOS | `ToneConverter.swift` | 統一介面 enum |
| iOS | `POJToneConverter.swift` | POJ 模式實作 |
| iOS | `TLToneConverter.swift` | TL 模式實作 |
| iOS | `ToneRestoration.swift` | 聲調還原（Android 無此功能） |
| iOS | `ToneMappings.swift` | 映射表 |
| iOS | `VowelAnalyzer.swift` | 元音分析工具 |

### 核心邏輯差異

| 面向 | Android | iOS | 應統一？ |
|------|---------|-----|----------|
| 架構模式 | 單一 object 集中 | enum 介面 + 分離實作 | 否（各有優勢） |
| 預處理 | `preprocess()` 處理 oo→o͘, nn→ⁿ | POJ 才預處理，TL 不預處理 | **是** |
| 元音尋找 | 8 個獨立函數 | `VowelAnalyzer` 模組化 | 否（iOS 更佳） |
| 聲調還原 | **無此功能** | `ToneRestoration.swift` | **是**（Android 應補上） |

### API 差異

```kotlin
// Android
fun convertToToneMarks(input: String, mode: InputMode): String
fun extractToneNumber(syllable: String): Pair<String, Int>
```

```swift
// iOS
static func convertToToneMarks(_ input: String, mode: InputMode) -> String
static func restoreTone(_ text: String, mode: InputMode) -> String?  // Android 無
```

### 待統一項目

- [ ] Android 補上 `restoreTone()` 功能（退格時還原聲調）
- [ ] 確認 TL 模式預處理邏輯一致

---

## 2. ComposingManager（組字管理）

### 檔案結構

| 平台 | 檔案 | 說明 |
|------|------|------|
| Android | `ComposingManager.kt` | 337 行，接收 InputConnection |
| iOS | `ComposingManager.swift` | 405 行，使用 @Published + KeyboardViewController |

### 核心邏輯差異

| 面向 | Android | iOS | 應統一？ |
|------|---------|-----|----------|
| 狀態管理 | 四個獨立屬性 | `ComposingState` enum + @Published | 否（各平台慣例） |
| UI 更新 | `InputConnection.setComposingText()` | @Published 響應式 | 否（框架差異） |
| 候選詞選擇 | `selectSuggestion(String)` | `selectSuggestion(Suggestion)` | **是** |
| 刪除邏輯 | 特殊處理 ⁿ（對應 nn） | 同樣邏輯 | 已一致 |

### API 差異

| 方法 | Android | iOS | 差異 |
|------|---------|-----|------|
| selectSuggestion | 接收 `String` | 接收 `Autocomplete.Suggestion` | 型別不同 |
| confirmSelectedCandidate | 接收 `List<String>` | 接收 `[Autocomplete.Suggestion]` | 型別不同 |
| 輸出參數 | 需傳入 `InputConnection` | 無需（內部持有 weak ref） | 設計不同 |

### 待統一項目

- [x] `selectSuggestion` 直接使用候選詞文字（已統一）
- [ ] 考慮統一候選詞資料結構（目前 Android 用 String/TaigiWord，iOS 用 Suggestion）

---

## 3. AutocompleteService（自動完成）

### 檔案結構

| 平台 | 檔案 | 說明 |
|------|------|------|
| Android | `TaigiAutocompleteService.kt` | 185 行，獨立服務 |
| iOS | `AutocompleteService.swift` | 403 行，實作 KeyboardKit 協議 |

### 核心邏輯差異

| 面向 | Android | iOS | 應統一？ |
|------|---------|-----|----------|
| 框架整合 | 獨立服務 | 實作 KeyboardKit 協議 | 否（框架要求） |
| 候選詞位置 0 | 插入當前組字文字 | 同樣 | 已一致 |
| 輸入類型判斷 | `InputType` enum | 同樣 | 已一致 |
| 大小寫模式 | `CasePattern` enum | 同樣 | 已一致 |
| 去重機制 | `deduplicateRomanWords()` | `deduplicateRomanSuggestions()` | 已一致 |
| 未使用方法 | 無 | 協議強制實作（全返回 false） | 否（框架要求） |

### API 差異

```kotlin
// Android
suspend fun getSuggestions(rawInput: String, displayText: String): List<TaigiWord>
```

```swift
// iOS
func autocomplete(_ text: String) async throws -> Autocomplete.ServiceResult
```

### 待統一項目

- [ ] 候選詞資料結構：Android 用 `TaigiWord`，iOS 用 `Autocomplete.Suggestion`
- [ ] 考慮定義共用的候選詞介面規格

---

## 4. NextWordService（下一詞預測）

### 檔案結構

| 平台 | 檔案 | 說明 |
|------|------|------|
| Android | `NextWordService.kt` | 599 行，object 單例 |
| iOS | `NextWordService.swift` | 572 行，class + shared 單例 |

### 核心邏輯差異

| 面向 | Android | iOS | 應統一？ |
|------|---------|-----|----------|
| Bigram 策略 | 字典用最後一字，使用者用完整詞 | 同樣 | 已一致 |
| 權重配置 | USER=50, DICT=1 | 同樣 | 已一致 |
| 時間衰減 | 指數衰減，半衰期 168 小時 | 同樣 | 已一致 |
| 清理策略 | 50,000 上限，5,000 批次刪除 | 同樣 | 已一致 |
| Prediction 結構 | 包含 `delimiter` 欄位 | **不包含** `delimiter` | **是** |
| Context 參數 | 所有方法需要 | 不需要 | 否（平台差異） |

### API 差異

```kotlin
// Android
suspend fun predict(word: String, limit: Int, context: Context): List<Prediction>
suspend fun recordAssociation(prev: String, nextHanzi: String, nextTl: String, nextPoj: String, delimiter: String, context: Context)
```

```swift
// iOS
func predict(word: String, limit: Int) async -> [Prediction]
func recordAssociation(prev: String, nextHanzi: String, nextTl: String, nextPoj: String) async
```

### Prediction 結構差異

```kotlin
// Android
data class Prediction(
    val hanzi: String,
    val tl: String,
    val poj: String,
    val delimiter: String,  // iOS 無此欄位
    val score: Double
)
```

```swift
// iOS
struct Prediction {
    let hanzi: String
    let tl: String
    let poj: String
    // 無 delimiter
    let score: Double
}
```

### 待統一項目

- [ ] 統一 `Prediction` 結構（決定是否保留 `delimiter`）
- [x] 時間衰減公式（已一致）
- [x] 清理策略（已一致）

---

## 5. ActionHandler / InputManager（輸入處理）

### 檔案結構

| 平台 | 檔案 | 說明 |
|------|------|------|
| Android | `TextInputManager.kt` | 主要輸入處理 |
| Android | `SmartbarManager.kt` | 候選詞管理、NextWord 觸發 |
| iOS | `ActionHandler.swift` | 主要動作處理 |
| iOS | `ActionHandler+Suggestions.swift` | 候選詞選擇處理 |
| iOS | `ActionHandler+CharacterInput.swift` | 字元輸入處理 |

### 核心邏輯差異

| 面向 | Android | iOS | 應統一？ |
|------|---------|-----|----------|
| 空白鍵處理 | 不觸發 autocomplete 更新 | 同樣（已修正） | 已一致 |
| 連字符處理 | NextWord 模式下保留候選詞 | 同樣（已修正） | 已一致 |
| NextWord 觸發 | `handleNextWordPrediction()` | `triggerNextWordPrediction()` | 已一致 |
| 上下文重置 | 標點、超時、切換輸入框 | 同樣 | 已一致 |

### 待統一項目

- [x] 空白鍵不清除 NextWord 候選詞（已統一）
- [x] 連字符不清除 NextWord 候選詞（已統一）
- [x] NextWord 候選詞格式（已統一：text=羅馬字, subtitle=漢字）

---

## 優先級分類

### 高優先級（功能性差異）

| 項目 | 說明 | 狀態 |
|------|------|------|
| ToneConverter.restoreTone | Android 無此功能 | 待補 |
| Prediction.delimiter | 結構不一致 | 待決定 |

### 中優先級（API 設計）

| 項目 | 說明 | 狀態 |
|------|------|------|
| 候選詞資料結構 | TaigiWord vs Suggestion | 待統一規格 |
| 方法簽名 | Context 參數差異 | 可接受（平台差異） |

### 低優先級（程式碼風格）

| 項目 | 說明 | 狀態 |
|------|------|------|
| 檔案組織 | Android 集中 vs iOS 模組化 | 可接受 |
| 命名慣例 | 各自遵循平台慣例 | 可接受 |

---

## 變更記錄

### 2025-12-22: 初始盤點

- 完成 ToneConverter、ComposingManager、AutocompleteService、NextWordService 比較
- 標記已一致和待統一項目
