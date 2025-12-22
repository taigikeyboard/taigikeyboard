# Debug Log 規範

> **關鍵字**: `log`, `debug`, `除錯`, `Logcat`

---

## 格式

```kotlin
if (BuildConfig.DEBUG) {
    Log.d(TAG, "[CATEGORY] message")
}
```

## 規則

1. **僅限 Debug**：使用 `BuildConfig.DEBUG` 包裹，Release 版本不會輸出
2. **TAG**：類別名稱，定義為 `private const val TAG = "ClassName"`
3. **CATEGORY**：大寫，描述操作類型（如 `INIT`, `SEARCH`, `DELETE`）
4. **message**：小寫，包含關鍵變數值

## 常用 CATEGORY

| Category | 用途 |
|----------|------|
| `INIT` | 初始化 |
| `SEARCH` | 搜尋 |
| `DELETE` | 刪除 |
| `INPUT` | 輸入處理 |
| `RESULT` | 結果 |
| `ERROR` | 錯誤 |
| `CANDIDATES` | 候選詞 |

## 範例

```kotlin
companion object {
    private const val TAG = "TextInputManager"
}

if (BuildConfig.DEBUG) {
    Log.d(TAG, "[DELETE] rawInput='$rawInput', composingText='$composingText'")
    Log.d(TAG, "[CANDIDATES] found ${suggestions.size} suggestions")
}
```

## 過濾指令

```bash
adb logcat -s TextInputManager:D LexiconService:D TrieService:D
```
