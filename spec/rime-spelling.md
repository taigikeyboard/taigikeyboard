# librime 拼寫系統

> **關鍵字**: `librime`, `Spelling Algebra`, `Segmentation`, `模糊音`
> **更新日期**：2025-12-19

---

## Spelling Algebra 拼寫變換

### 概念

Spelling Algebra 是 librime 的核心功能，允許透過規則定義多種拼寫變體，實現：
- 模糊音（如 n/l 不分）
- 縮寫（首字母輸入）
- 拼寫糾錯

### 變換類型

| 類型 | 語法 | 功能 | 懲罰分數 |
|------|------|------|----------|
| `xlit` | `xlit\|abc\|xyz\|` | 字符對應替換 | 無 |
| `xform` | `xform/pattern/replacement/` | 正則變換 | 無 |
| `erase` | `erase/pattern/` | 刪除符合的拼寫 | - |
| `derive` | `derive/pattern/replacement/` | 衍生新拼寫（保留原拼寫） | 無 |
| `fuzz` | `fuzz/pattern/replacement/` | 模糊音 | -0.693 |
| `abbrev` | `abbrev/pattern/replacement/` | 縮寫 | -0.693 |

### 實際範例

```yaml
speller:
  algebra:
    # 刪除特殊標記
    - erase/^xx$/

    # 首字母縮寫：zhongguo -> z
    - abbrev/^([a-z]).+$/$1/

    # 衍生變體：nve -> nue（保留 nve）
    - derive/^([nl])ve$/$1ue/

    # 模糊音：ang -> agn
    - fuzz/([aeiou])ng$/$1gn/

    # 字符替換：v -> u
    - xlit/v/u/
```

### 懲罰機制

```cpp
// 定義在 spelling.h
constexpr double kAbbreviationPenalty = -std::log(0.5);     // -0.693
constexpr double kFuzzySpellingPenalty = -std::log(0.5);    // -0.693
constexpr double kCompletionPenalty = -std::log(0.5);       // -0.693
constexpr double kCorrectionPenalty = -std::log(0.01);      // -4.605
constexpr double kAmbiguousPenalty = -std::log(1e-10);      // -23.026
```

候選詞的最終分數會加上這些懲罰值，使得模糊音/縮寫的候選詞排名較低。

---

## SpellingProperties 結構

```cpp
struct SpellingProperties {
    SpellingType type;      // Normal, Fuzzy, Abbreviation, Completion
    size_t end_pos;         // 結束位置
    double credibility;     // 可信度（log 概率）
    string tips;            // 顯示提示
};

enum SpellingType {
    kNormalSpelling,        // 正常拼寫
    kFuzzySpelling,         // 模糊音
    kAbbreviation,          // 縮寫
    kCompletion,            // 自動完成
    kCorrection,            // 糾錯
    kAmbiguousSpelling      // 歧義音節
};
```

---

## Segmentation 分詞系統

### 音節圖 (SyllableGraph)

librime 使用 DAG（有向無環圖）表示多種分詞可能性：

```cpp
struct SyllableGraph {
    size_t input_length;           // 輸入長度
    size_t interpreted_length;     // 已解析長度
    VertexMap vertices;            // 頂點：位置 → 拼寫類型
    EdgeMap edges;                 // 邊：起點 → 終點 → 音節映射
    SpellingIndices indices;       // 索引
};
```

### 圖形化範例

輸入 `xian`：

```
位置:  0 ──── 1 ──── 2 ──── 3 ──── 4
       │      │      │      │      │
       └──xi──┴──a───┴──n───┘      │
       │             │             │
       └────xian─────┴─────────────┘
       │      │
       └──x───┴──ian──────────────→
```

可能的分割：
- `xi + an`（西安）
- `xian`（先/鮮）
- `x + i + an`（縮寫模式）

### Segmentor 類型

| 類型 | 功能 |
|------|------|
| `abc_segmentor` | 字母分段，基於音節表 |
| `affix_segmentor` | 前綴/後綴分段（如反查觸發符） |
| `punct_segmentor` | 標點分段 |
| `fallback_segmentor` | 兜底分段 |

### AbcSegmentor 配置

```yaml
speller:
  alphabet: zyxwvutsrqponmlkjihgfedcba   # 允許的字母
  initials: zyxwvutsrqponmlkjihgfedcba   # 聲母
  finals: aeiou                          # 韻母
  delimiter: " '"                        # 分隔符
  max_code_length: 6                     # 最大編碼長度
```

### Syllabifier 演算法

```cpp
class Syllabifier {
    // 構建音節圖
    int BuildSyllableGraph(const string& input,
                          Prism& prism,
                          SyllableGraph* graph);

private:
    // 使用優先佇列構建圖
    void PushVertex(priority_queue<Vertex>& queue, size_t pos);

    // 檢查重疊拼寫（處理歧義）
    void CheckOverlappedSpellings(SyllableGraph* graph, size_t start);

    // 建立反向索引
    void Transpose(SyllableGraph* graph);
};
```

---

## Prism 拼寫索引

### 資料結構

Prism 使用 **Double-Array Trie (Darts)** 實現極速前綴匹配：

```cpp
struct Metadata {
    char format[32];
    uint32_t num_syllables;      // 音節數量
    uint32_t num_spellings;      // 拼寫數量
    uint32_t double_array_size;  // Trie 大小
    OffsetPtr<char> double_array;
    OffsetPtr<SpellingMap> spelling_map;
    char alphabet[256];          // 字母表
};
```

### 查詢方法

```cpp
class Prism {
    // 前綴搜索：找出所有匹配的前綴
    void CommonPrefixSearch(const string& key,
                           vector<Match>* result);

    // 擴展搜索：自動補全
    void ExpandSearch(const string& key,
                     vector<Match>* result,
                     size_t limit);

    // 查詢拼寫屬性
    bool QuerySpelling(SpellingId id,
                      SpellingProperties* props);
};
```

---

## 拼寫糾錯

### Corrector 類別

```cpp
class Corrector {
    // 計算編輯距離
    size_t EditDistance(const string& a, const string& b);

    // 容錯搜索
    void ToleranceSearch(Prism& prism,
                        const string& key,
                        vector<Correction>* corrections,
                        size_t tolerance);
};
```

### 糾錯懲罰

每次編輯操作（插入、刪除、替換）會增加 `kCorrectionPenalty` (-4.605) 的懲罰。

---

## 台語應用建議

### POJ/TL 變體處理

可以用 Spelling Algebra 統一 POJ 和 TL 輸入：

```yaml
# 概念範例
speller:
  algebra:
    # POJ -> TL 變體
    - derive/ch/ts/           # ch -> ts
    - derive/chh/tsh/         # chh -> tsh
    - derive/oa/ua/           # oa -> ua
    - derive/oe/ue/           # oe -> ue
    - derive/eng/ing/         # eng -> ing

    # 聲調變體
    - derive/([aeiou])(\d)/$1/  # 去聲調
    - abbrev/^(.+)\d$/$1/       # 省略聲調
```

### 模糊音範例

```yaml
# 台語常見混淆音
speller:
  algebra:
    - fuzz/n$/ng/             # in/ing 不分
    - fuzz/l/n/               # l/n 不分
    - fuzz/g/k/               # 濁音清音
```

---

## 與 TaigiKeyboard 的比較

| 項目 | librime | TaigiKeyboard |
|------|---------|---------------|
| **變體處理** | Spelling Algebra（規則驅動） | 手動建立多筆資料 |
| **分詞** | DAG + 動態規劃 | 無（單詞搜尋） |
| **模糊音** | 內建支援 + 懲罰 | 未實作 |
| **縮寫** | 內建支援 | 未實作 |

### 值得借鑑

1. **規則式變體**：用規則產生變體，而非手動維護多筆資料
2. **DAG 分詞**：支援連續輸入，提升輸入效率
3. **懲罰機制**：模糊音/縮寫候選排名較低，精確匹配優先

---

## 相關檔案

- `src/rime/algo/algebra.{h,cc}` - 拼寫代數
- `src/rime/algo/calculus.{h,cc}` - 變換計算
- `src/rime/algo/spelling.h` - 拼寫屬性定義
- `src/rime/algo/syllabifier.{h,cc}` - 音節圖構建
- `src/rime/dict/prism.{h,cc}` - Prism 索引
- `src/rime/dict/corrector.{h,cc}` - 拼寫糾錯
- `src/rime/gear/abc_segmentor.{h,cc}` - ABC 分段器
