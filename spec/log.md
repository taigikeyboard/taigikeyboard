# Debug Log 規範

> **關鍵字**: `log`, `debug`, `除錯`, `Logcat`, `Console`, `OSLog`

---

## 統一格式

```
[TAG] message key1=value1 key2=value2
```

---

## 平台實作

### Android

```kotlin
companion object {
    private const val TAG = "ClassName"
}

// 單行
if (BuildConfig.DEBUG) Log.d(TAG, "[CATEGORY] message key='$value'")

// 多行
if (BuildConfig.DEBUG) {
    Log.d(TAG, "[PERF] elapsed=${System.currentTimeMillis() - start}ms")
    Log.d(TAG, "[RESULT] count=${results.size}")
}
```

**規則：**
1. 所有 Log 必須用 `if (BuildConfig.DEBUG)` 包裹
2. TAG 定義為 `companion object { private const val TAG = "ClassName" }`
3. 使用標準 import：`import android.util.Log`

### iOS

```swift
import OSLog

private let logger = Logger(
    subsystem: LexiconConstants.Logging.subsystem,
    category: "ClassName"
)

// 使用
#if DEBUG
logger.debug("[CATEGORY] message key=\(value)")
#endif
```

**規則：**
1. 使用 `#if DEBUG` 包裹 debug level log（可選，Logger 會自動過濾）
2. Subsystem 統一使用 `LexiconConstants.Logging.subsystem`
3. Category 使用類別名稱

---

## 常用 CATEGORY

| Category | 用途 | 使用時機 |
|----------|------|----------|
| `INIT` | 初始化 | 服務啟動、資料庫載入 |
| `SEARCH` | 搜尋 | 詞典查詢、Trie 查詢 |
| `PREDICT` | 預測 | NextWord 預測 |
| `INPUT` | 輸入 | 按鍵處理、組字輸入 |
| `NORMALIZE` | 正規化 | InputNormalizer 處理 |
| `TONE` | 聲調 | ToneConverter 處理 |
| `LAYOUT` | 佈局 | 鍵盤佈局載入/切換 |
| `PERF` | 效能 | 執行時間測量 |
| `RESULT` | 結果 | 查詢結果、候選詞 |
| `CANDIDATES` | 候選詞 | 候選詞顯示與選擇 |
| `SCROLL` | 滾動 | UI 滾動事件 |
| `PREF` | 偏好設定 | 設定變更 |
| `PARSE` | 解析 | 資料解析 |
| `SQL` | SQL | 資料庫查詢 |
| `TRIE` | Trie | Trie 操作 |
| `RECORD` | 記錄 | 使用者行為記錄 |
| `PRUNE` | 清理 | 資料庫清理 |
| `ERROR` | 錯誤 | 例外處理 |
| `CLOSE` | 關閉 | 資源釋放 |
| `PREVIEW` | 預覽 | SwiftUI Preview |

---

## Log 等級

| 等級 | Android | iOS | 用途 |
|------|---------|-----|------|
| Debug | `Log.d()` | `logger.debug()` | 開發除錯資訊 |
| Info | `Log.i()` | `logger.info()` | 重要狀態變更 |
| Warning | `Log.w()` | `logger.warning()` | 可恢復錯誤 |
| Error | `Log.e()` | `logger.error()` | 嚴重錯誤 |

---

## 過濾指令

### Android (Logcat)

```bash
# 過濾特定服務
adb logcat -s LexiconService:D ToneConverter:D InputNormalizer:D

# 過濾效能 log
adb logcat | grep PERF

# 過濾所有台語鍵盤 log
adb logcat | grep -E "(LexiconService|NextWordService|ToneConverter|LayoutManager)"
```

### iOS (Console)

```
subsystem:com.siansiansu.taigikeyboard category:LexiconService
subsystem:com.siansiansu.taigikeyboard category:ToneConverter
```

---

## 服務對照表

| 功能 | Android | iOS |
|------|---------|-----|
| 詞典查詢 | `LexiconService` | `LexiconService` |
| 下一詞預測 | `NextWordService` | `NextWordService` |
| 聲調轉換 | `ToneConverter` | `ToneConverter` |
| 輸入正規化 | `InputNormalizer` | `InputNormalizer` |
| 鍵盤佈局 | `LayoutManager` | `CustomLayoutService` |
| 自動完成 | `TaigiAutocompleteService` | `AutocompleteService` |
| 使用者頻率 | `UserFrequencyService` | `UserFrequencyService` |
| Trie 服務 | `TrieService` | `TrieService` |
| 鍵盤主入口 | `TaigiKeyboard` | `KeyboardViewController` |
| 偏好設定 | `PrefHelper` | `SharedSettings` |
