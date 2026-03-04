# Debug Log Convention

> **Keywords**: `log`, `debug`, `Logcat`, `Console`, `OSLog`

---

## Unified Format

```
[TAG] message key1=value1 key2=value2
```

---

## Platform Implementation

### Android

```kotlin
companion object {
    private const val TAG = "ClassName"
}

// Single line
if (BuildConfig.DEBUG) Log.d(TAG, "[CATEGORY] message key='$value'")

// Multi-line
if (BuildConfig.DEBUG) {
    Log.d(TAG, "[PERF] elapsed=${System.currentTimeMillis() - start}ms")
    Log.d(TAG, "[RESULT] count=${results.size}")
}
```

**Rules:**
1. All logs must be wrapped with `if (BuildConfig.DEBUG)`
2. TAG defined as `companion object { private const val TAG = "ClassName" }`
3. Use standard import: `import android.util.Log`

### iOS

```swift
import OSLog

private let logger = Logger(
    subsystem: LexiconConstants.Logging.subsystem,
    category: "ClassName"
)

// Usage
#if DEBUG
logger.debug("[CATEGORY] message key=\(value)")
#endif
```

**Rules:**
1. Use `#if DEBUG` for debug level logs (optional, Logger auto-filters)
2. Subsystem: use `LexiconConstants.Logging.subsystem`
3. Category: use class name

---

## Common Categories

| Category | Use | Timing |
|----------|-----|--------|
| `INIT` | Initialization | Service startup, database load |
| `SEARCH` | Search | Dictionary query, Trie query |
| `PREDICT` | Prediction | NextWord prediction |
| `INPUT` | Input | Key handling, composing input |
| `NORMALIZE` | Normalization | InputNormalizer processing |
| `TONE` | Tone | ToneConverter processing |
| `LAYOUT` | Layout | Keyboard layout load/switch |
| `PERF` | Performance | Execution time measurement |
| `RESULT` | Results | Query results, candidates |
| `CANDIDATES` | Candidates | Candidate display and selection |
| `SCROLL` | Scroll | UI scroll events |
| `PREF` | Preferences | Settings changes |
| `PARSE` | Parse | Data parsing |
| `SQL` | SQL | Database queries |
| `TRIE` | Trie | Trie operations |
| `RECORD` | Record | User behavior recording |
| `PRUNE` | Prune | Database cleanup |
| `ERROR` | Error | Exception handling |
| `CLOSE` | Close | Resource release |
| `PREVIEW` | Preview | SwiftUI Preview |

---

## Log Levels

| Level | Android | iOS | Use |
|-------|---------|-----|-----|
| Debug | `Log.d()` | `logger.debug()` | Development debug info |
| Info | `Log.i()` | `logger.info()` | Important state changes |
| Warning | `Log.w()` | `logger.warning()` | Recoverable errors |
| Error | `Log.e()` | `logger.error()` | Severe errors |

---

## Filter Commands

### Android (Logcat)

```bash
# Filter specific services
adb logcat -s LexiconService:D ToneConverter:D InputNormalizer:D

# Filter performance logs
adb logcat | grep PERF

# Filter all Taigi keyboard logs
adb logcat | grep -E "(LexiconService|NextWordService|ToneConverter|LayoutManager)"
```

### iOS (Console)

```
subsystem:com.siansiansu.taigikeyboard category:LexiconService
subsystem:com.siansiansu.taigikeyboard category:ToneConverter
```

---

## Service Correspondence Table

| Function | Android | iOS |
|----------|---------|-----|
| Dictionary query | `LexiconService` | `LexiconService` |
| Next-word prediction | `NextWordService` | `NextWordService` |
| Tone conversion | `ToneConverter` | `ToneConverter` |
| Input normalization | `InputNormalizer` | `InputNormalizer` |
| Keyboard layout | `LayoutManager` | `CustomLayoutService` |
| Autocomplete | `TaigiAutocompleteService` | `AutocompleteService` |
| User frequency | `UserFrequencyService` | `UserFrequencyService` |
| Trie service | `TrieService` | `TrieService` |
| Keyboard main entry | `TaigiKeyboard` | `KeyboardViewController` |
| Preferences | `PrefHelper` | `SharedSettings` |
