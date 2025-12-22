# 模組化分析報告

本文件盤點 iOS 與 Android 專案中適合分離成獨立 repository 的功能模組。

## 設計原則

1. **完全獨立** - 每個模組零依賴其他自製模組，可單獨使用
2. **跨平台共用** - 兩平台邏輯相似，適合抽取共用
3. **可重用性** - 其他台語相關專案可直接導入
4. **最小介面** - 清晰的 API 邊界，易於整合

---

## 獨立模組清單

### 1. taigi-trie (高優先)

**說明**: MARISA-trie C++ 庫的跨平台封裝，提供高效前綴搜尋

| 平台 | 現有位置 |
|------|----------|
| iOS | `Lexicon/Trie/TrieService.swift` + C 橋接層 |
| Android | `ime/dictionary/TrieService.kt` + JNI |
| 共用 | C++ MARISA-trie 原始碼 |

**模組內容**:
- MARISA-trie C++ 原始碼
- iOS: Swift Package (C bridging header)
- Android: JNI 封裝 + AAR
- 前綴搜尋 / 精確匹配 API

**外部依賴**: 無 (MARISA-trie 原始碼內含)

**介面設計**:
```swift
// iOS
public class TrieIndex {
    public init()
    public func load(path: String) -> Bool
    public func prefixSearch(_ prefix: String, limit: Int) -> [Int]
    public func exactMatch(_ key: String) -> Int?
    public func close()
}
```
```kotlin
// Android
class TrieIndex {
    fun load(path: String): Boolean
    fun prefixSearch(prefix: String, limit: Int): IntArray
    fun exactMatch(key: String): Int?
    fun close()
}
```

---

### 2. taigi-tone (高優先)

**說明**: POJ/TL 聲調轉換，純邏輯無任何依賴

| 平台 | 現有位置 |
|------|----------|
| iOS | `Input/Tone/*.swift`, `Lexicon/Trie/InputNormalizer.swift` |
| Android | `ime/dictionary/ToneConverter.kt`, `InputNormalizer.kt`, `ToneCharacterUtils.kt` |

**模組內容**:
- 調符 ↔ 數字互轉 (á ↔ a2)
- POJ ↔ TL 格式互轉 (oe ↔ ue, ch ↔ ts)
- 輸入正規化 (任意格式 → TL 數字)
- 母音分析 (聲調標記位置判斷)
- 聲調映射表

**外部依賴**: 無

**介面設計**:
```swift
// iOS
public struct TaigiTone {
    public static func toNumbered(_ text: String) -> String      // á → a2
    public static func toMarked(_ text: String) -> String        // a2 → á
    public static func normalize(_ input: String) -> String      // 任意 → TL數字
    public static func pojToTL(_ text: String) -> String         // POJ → TL
    public static func tlToPOJ(_ text: String) -> String         // TL → POJ
}
```
```kotlin
// Android
object TaigiTone {
    fun toNumbered(text: String): String
    fun toMarked(text: String): String
    fun normalize(input: String): String
    fun pojToTL(text: String): String
    fun tlToPOJ(text: String): String
}
```

---

### 3. taigi-frequency (中優先)

**說明**: 通用詞頻記錄服務，SQLite 持久化

| 平台 | 現有位置 |
|------|----------|
| iOS | `Lexicon/Services/UserFrequencyService.swift`, `UserFrequencyRepository.swift` |
| Android | `ime/text/composing/UserFrequencyService.kt` |

**模組內容**:
- 詞彙使用頻率記錄
- SQLite 資料庫管理
- 頻率衰減演算法 (可選)
- 批次查詢優化

**外部依賴**:
- iOS: 系統 SQLite
- Android: 系統 SQLite

**介面設計**:
```swift
// iOS
public class FrequencyStore {
    public init(databasePath: String)
    public func record(id: Int)
    public func frequency(for id: Int) -> Int
    public func frequencies(for ids: [Int]) -> [Int: Int]
    public func reset()
}
```
```kotlin
// Android
class FrequencyStore(databasePath: String) {
    fun record(id: Int)
    fun frequency(id: Int): Int
    fun frequencies(ids: List<Int>): Map<Int, Int>
    fun reset()
}
```

**設計說明**: 使用泛用 `id: Int` 而非 `wordId`，讓模組可用於任何需要頻率統計的場景。

---

### 4. taigi-sqlite (中優先)

**說明**: 輕量 SQLite 連線管理與查詢工具

| 平台 | 現有位置 |
|------|----------|
| iOS | `Lexicon/Database/SQLiteConnectionManager.swift`, `DictionaryRepository.swift` |
| Android | (分散在各 Service) |

**模組內容**:
- SQLite 連線池管理
- 預備語句快取
- 批次查詢工具
- 錯誤處理封裝

**外部依賴**: 系統 SQLite

**介面設計**:
```swift
// iOS
public class SQLiteConnection {
    public init(path: String, readonly: Bool = true)
    public func query<T>(_ sql: String, params: [Any], mapper: (Row) -> T) -> [T]
    public func execute(_ sql: String, params: [Any]) -> Bool
    public func close()
}
```

**考量**: 此模組偏通用，可評估是否使用現有開源方案 (如 GRDB, SQLDelight)

---

## 不建議模組化

| 功能 | 原因 |
|------|------|
| **LexiconService** | 整合 Trie + SQLite + Frequency，屬於應用層邏輯 |
| **ComposingManager** | 與 UI 狀態緊密耦合 |
| **AutocompleteService** | 依賴平台 IME 框架 |
| **鍵盤 UI** | 依賴 KeyboardKit (iOS) / 自訂 View (Android) |
| **設定/主題** | 平台 API 差異大 |

---

## 模組獨立性對照

| 模組 | 依賴其他自製模組 | 外部依賴 |
|------|------------------|----------|
| taigi-trie | 無 | 無 |
| taigi-tone | 無 | 無 |
| taigi-frequency | 無 | 系統 SQLite |
| taigi-sqlite | 無 | 系統 SQLite |

---

## 架構圖 (獨立模組)

```
┌─────────────────────────────────────────────────────────┐
│                    iOS / Android App                     │
│                                                          │
│  ┌────────────────────────────────────────────────────┐ │
│  │              LexiconService (應用層)               │ │
│  │    整合各獨立模組，提供統一的詞典查詢介面           │ │
│  └──────┬──────────┬──────────┬──────────┬────────────┘ │
│         │          │          │          │              │
│         ▼          ▼          ▼          ▼              │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐   │
│  │  taigi-  │ │  taigi-  │ │  taigi-  │ │  taigi-  │   │
│  │   trie   │ │   tone   │ │frequency │ │  sqlite  │   │
│  │          │ │          │ │          │ │          │   │
│  │ 零依賴   │ │ 零依賴   │ │ 零依賴   │ │ 零依賴   │   │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘   │
│                                                          │
│  各模組可獨立導入，按需組合                              │
└─────────────────────────────────────────────────────────┘
```

---

## 跨平台 Repo 策略

iOS 與 Android 為獨立實作，模組化時有三種 repo 組織方式：

### 方案比較

| 方案 | 說明 | 優點 | 缺點 |
|------|------|------|------|
| **A. 單一 Repo** | 一個 repo 含 iOS + Android | 版本同步、邏輯一致 | 導入時帶另一平台代碼 |
| **B. 分開 Repo** | `*-ios` / `*-android` | 各平台獨立乾淨 | 重複維護、版本可能不同步 |
| **C. KMM 共用** | Kotlin Multiplatform 寫一份 | 真正共用代碼 | 技術棧複雜度增加 |

### 各模組策略

| 模組 | 採用方案 | 原因 |
|------|----------|------|
| **taigi-tone** | C (KMM) | 純邏輯，零平台依賴，最適合 KMM |
| **taigi-trie** | A (單一 Repo) | C++ 核心共用，binding 分平台 |
| **taigi-frequency** | B (分開 Repo) | SQLite API 差異大，分開較單純 |
| **taigi-sqlite** | 不抽取 | 平台 API 完全不同，使用現成方案 |

---

## Repo 結構設計

### taigi-tone (KMM)

```
taigi-tone/
├── shared/
│   └── src/
│       ├── commonMain/kotlin/       # 共用邏輯 (100%)
│       │   └── com/taigi/tone/
│       │       ├── TaigiTone.kt
│       │       ├── ToneConverter.kt
│       │       ├── InputNormalizer.kt
│       │       └── ToneMappings.kt
│       ├── commonTest/kotlin/       # 共用測試
│       ├── iosMain/kotlin/          # iOS 特定 (通常為空)
│       └── androidMain/kotlin/      # Android 特定 (通常為空)
├── build.gradle.kts                 # KMM 設定
└── README.md
```

**發布產物**:
- iOS: XCFramework (透過 SPM 或 CocoaPods)
- Android: AAR (Maven / JitPack)

### taigi-trie (單一 Repo)

```
taigi-trie/
├── cpp/                             # 共用 C++ 核心
│   ├── marisa/                      # MARISA-trie 原始碼
│   ├── trie_wrapper.h
│   └── trie_wrapper.cpp
├── ios/
│   ├── Package.swift
│   └── Sources/
│       ├── TaigiTrie/
│       │   └── TrieIndex.swift
│       └── CTaigiTrie/              # C bridging
│           ├── include/
│           └── trie_bridge.c
├── android/
│   ├── build.gradle.kts
│   └── src/
│       ├── main/
│       │   ├── cpp/                 # JNI 層
│       │   │   └── trie_jni.cpp
│       │   └── kotlin/
│       │       └── TrieIndex.kt
│       └── test/
└── README.md
```

**發布產物**:
- iOS: Swift Package (含預編譯 .a 或原始碼編譯)
- Android: AAR (含 .so)

### taigi-frequency (分開 Repo)

**taigi-frequency-ios/**
```
taigi-frequency-ios/
├── Package.swift
├── Sources/FrequencyStore/
│   ├── FrequencyStore.swift
│   └── SQLiteHelpers.swift
└── Tests/
```

**taigi-frequency-android/**
```
taigi-frequency-android/
├── build.gradle.kts
└── src/
    ├── main/kotlin/
    │   └── FrequencyStore.kt
    └── test/
```

---

## 發布形式總覽

| 模組 | Repo 數量 | iOS 發布 | Android 發布 |
|------|-----------|----------|--------------|
| taigi-tone | 1 (KMM) | XCFramework via SPM | AAR via Maven |
| taigi-trie | 1 (共用) | Swift Package | AAR via Maven |
| taigi-frequency | 2 (分開) | Swift Package | AAR via Maven |

---

## 實作優先順序

```
1. taigi-tone (KMM)
   ├── 建立 KMM 專案模板
   ├── 遷移聲調轉換邏輯
   ├── 撰寫 commonTest
   └── 設定雙平台發布

2. taigi-trie (單一 Repo)
   ├── 整理 C++ 核心代碼
   ├── 建立 iOS Swift Package
   ├── 建立 Android JNI module
   └── 設定 CI 編譯 native 產物

3. taigi-frequency (分開 Repo)
   ├── 抽取 iOS 版本
   ├── 抽取 Android 版本
   └── 各自維護發布
```

---

## 後續行動

1. 建立 KMM 專案模板 (taigi-tone)
2. 驗證 KMM 在兩平台的整合方式
3. 撰寫各模組的單元測試
4. 設定 GitHub Actions CI/CD
5. 逐步從主專案抽取，確保持續運作
