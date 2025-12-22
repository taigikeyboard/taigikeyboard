# Trie 索引與 Lexicon 查詢

> **功能代號**: `Trie`, `Lexicon`
> **關鍵字**: `Trie`, `MARISA`, `Lexicon`, `InputNormalizer`, `TrieService`, `LexiconService`

---

## 架構

| 元件 | 內容 | 用途 |
|------|------|------|
| MARISA-trie | key → SQLite rowid | 前綴查詢 |
| SQLite | 完整資料 | 儲存詞條 |

## 建構流程

```
join.py → dictionary.csv
build.sh → dictionary.db (SQLite)
build_trie.py → dictionary.trie (Python marisa_trie.RecordTrie)
```

## Trie Key 格式

使用前綴區分 TL 和 POJ，多個 key 指向同一 rowid：

| Key 類型 | 範例 | 說明 |
|----------|------|------|
| tl:tl_num | `tl:hoo2boo5` | TL 數字聲調 |
| tl:tl_notone | `tl:hooboo` | TL 無聲調 |
| tl:tl_abbrev | `tl:hb` | TL 縮寫（2+ 音節） |
| poj:poj_num | `poj:ho2bo5` | POJ 數字聲調 |
| poj:poj_notone | `poj:hobo` | POJ 無聲調 |
| poj:poj_abbrev | `poj:hb` | POJ 縮寫（2+ 音節） |

**前綴區分策略**：TL 和 POJ 使用不同前綴，查詢時根據 InputMode 決定前綴。

## 查詢流程

```
POJ 模式輸入 "goa2" 或 "goá"
  → InputNormalizer.normalize() → "goa2"
  → 加前綴 → "poj:goa2"
  → TrieService.prefixSearch("poj:goa2")
  → rowid 列表
  → SQLite 批次查詢
  → 按 frequency 排序

TL 模式輸入 "gua2" 或 "guá"
  → InputNormalizer.normalize() → "gua2"
  → 加前綴 → "tl:gua2"
  → TrieService.prefixSearch("tl:gua2")
  → rowid 列表
  → SQLite 批次查詢
  → 按 frequency 排序
```

## InputNormalizer

將使用者輸入正規化為 Trie key 格式（數字聲調，不做 POJ/TL 轉換）。

**支援輸入格式**：
| InputMode | 輸入 | 正規化結果 |
|-----------|------|------------|
| POJ | hó | ho2 |
| POJ | ho2 | ho2 |
| TL | hóo | hoo2 |
| TL | hoo2 | hoo2 |

**處理步驟**：
1. 轉小寫
2. 去除連字符
3. 調符轉數字聲調（`hó` → `ho2`）

注意：不做 POJ → TL 轉換，因為 Trie 使用前綴區分。

**調符轉換**：根據 Unicode 組合標記判斷聲調
| 標記 | 聲調 |
|------|------|
| `́` (U+0301) acute | 2 |
| `̀` (U+0300) grave | 3 |
| `̂` (U+0302) circumflex | 5 |
| `̌` (U+030C) caron | 6 |
| `̄` (U+0304) macron | 7 |
| `̍` (U+030D) vertical line | 8 |
| `̆` (U+0306) breve | 9 |

**聲調數字位置**：放在音節尾端（`gáb` → `gab2`，非 `ga2b`）

**檔案**：`android/.../ime/dictionary/InputNormalizer.kt`

## 空間估算

- 30 萬筆 × 6 個 key（TL 3 + POJ 3）≈ 180 萬 key-value pairs
- dictionary.trie ≈ 2.5-4 MB
- dictionary.db ≈ 15-20 MB

---

## Android 實作

### 檔案結構

```
android/app/src/main/
├── java/.../ime/dictionary/
│   └── TrieService.kt          # Kotlin singleton
└── cpp/
    ├── CMakeLists.txt          # NDK 編譯設定
    ├── trie_jni.cpp            # JNI wrapper
    └── marisa-src/             # MARISA-trie 原始碼
```

### TrieService.kt

```kotlin
object TrieService {
    // Native library: libtaigi_trie.so
    init { System.loadLibrary("taigi_trie") }

    suspend fun init(context: Context): Boolean  // 從 assets 複製並載入
    fun prefixSearch(prefix: String, limit: Int = 1000): IntArray
    fun lookup(key: String): IntArray            // 完全匹配
    fun isReady(): Boolean
    fun close()
}
```

- **Mutex 保護**：`init()` 使用 coroutine Mutex 避免重複初始化
- **版本檢查**：App 版本更新時自動更新 trie 檔案

### JNI 層 (trie_jni.cpp)

**RecordTrie 格式**：Python `marisa_trie.RecordTrie` 編碼
```
raw_key = utf8_key + \xff + uint32_le(rowid)
```

**核心函數**：
| 函數 | 說明 |
|------|------|
| `nativeLoad(path)` | mmap 載入 trie |
| `nativePrefixSearch(prefix, limit)` | `predictive_search` 前綴查詢 |
| `nativeLookup(key)` | 查詢 `key + \xff` 開頭的所有 raw key |

**extractRowId**：解析 raw key 最後 4 bytes (little-endian uint32)

### CMake 設定

- C++17、支援 16KB page size（Google Play 2025/11 要求）
- MARISA 編譯為 static library
- JNI wrapper 編譯為 shared library (libtaigi_trie.so)

---

## 設計決策

1. **rowid 作為 value**：不需另存 global id
2. **數字聲調存 Trie**：ASCII 相容，與 khiin-rs 一致
3. **POJ/TL 前綴區分**：`tl:` 和 `poj:` 前綴，查詢時根據 InputMode 選擇
4. **聲調/無聲調都存**：查詢時不需額外過濾
5. **mmap 載入**：節省記憶體，系統自動管理
6. **不做 POJ→TL 轉換**：InputNormalizer 只處理調符→數字，不轉換拼法

---

## Debug 除錯

### Logcat 指令

```bash
adb logcat -s TaigiAutocompleteService:D LexiconService:D TrieService:D
```

### Debug Log 流程

只在 `BuildConfig.DEBUG` 時輸出：

```
[TaigiAutocompleteService]
  [INPUT] rawInput='goa2', displayText='goá', mode=POJ
  [INPUT] inputType=RomanWithTone
  [RESULT] LexiconService returned 15 words

[LexiconService]
  [SEARCH] input='goa2', mode=POJ, limit=50
  [NORMALIZE] 'goa2' -> 'goa2'
  [TRIE] trieKey='poj:goa2', isReady=true, keyCount=123456
  [TRIE] lookup('poj:goa2') -> 5 exact matches: [123, 456, ...]
  [TRIE] prefixSearch('poj:goa2', 150) -> 20 prefix matches: [...]
  [TRIE] Total 25 unique rowIds
  [SQL] queryByIds: 25 ids, enabledDicts=...
  [SQL] queryByIds returned 15 words
  [SQL]   - goá / 我
  [SQL]   - goá-ê / 我的
```

### 常見問題檢查點

| 問題 | 檢查 Log |
|------|----------|
| Trie 未初始化 | `isReady=false` 或 `keyCount=0` |
| 正規化錯誤 | `[NORMALIZE]` 結果不符預期 |
| Trie 查無結果 | `lookup` 和 `prefixSearch` 都為 0 |
| SQLite 查無結果 | `queryByIds returned 0 words` |
| 辭典全關閉 | `all dicts disabled` |
