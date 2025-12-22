# librime 連續輸入分詞系統

> **關鍵字**: `DAG`, `Syllabifier`, `分詞`, `動態規劃`
> **更新日期**：2025-12-19

---

## 概述

librime 使用 **DAG（有向無環圖）** 來處理連續輸入的分詞問題。核心思想是：
- 不立即決定分詞方式
- 儲存所有可能的分割
- 延遲到句子層才選擇最佳路徑

---

## 核心資料結構

### SyllableGraph（音節圖）

```cpp
struct SyllableGraph {
    size_t input_length;           // 輸入長度
    size_t interpreted_length;     // 已解析長度
    VertexMap vertices;            // 頂點：位置 → 拼寫類型
    EdgeMap edges;                 // 邊：起點 → 終點 → 音節映射
    SpellingIndices indices;       // 反向索引（加速查詢）
};

// edges 結構: edges[起點][終點][音節ID] = EdgeProperties
using EdgeMap = map<size_t, EndVertexMap>;
using EndVertexMap = map<size_t, SpellingMap>;
using SpellingMap = map<SyllableId, EdgeProperties>;
```

### 圖形化範例

輸入 `xian`（西安 or 先）：

```
位置:  0 ──── 1 ──── 2 ──── 3 ──── 4
       │      │      │      │      │
       └──xi──┴──an──┘             │
       │                           │
       └─────────xian──────────────┘
```

可能的分割：
- `xi + an`（西安）
- `xian`（先/鮮）

---

## DAG 構建演算法

### Phase 1: 正向構建（優先佇列）

```cpp
// 使用優先佇列進行圖構建
VertexQueue queue;  // priority_queue with SpellingType as priority
queue.push(Vertex{0, kNormalSpelling});  // 起點

while (!queue.empty()) {
    Vertex vertex = queue.top();
    size_t current_pos = vertex.first;
    SpellingType spelling_type = vertex.second;

    // 去重：只保留每個位置最好的拼寫類型
    if (vertices.find(current_pos) != vertices.end()) {
        continue;
    }
    vertices[current_pos] = spelling_type;

    // 從 Prism (trie) 搜尋所有可能的拼寫
    vector<Prism::Match> matches;
    prism.CommonPrefixSearch(input.substr(current_pos), &matches);

    for (const auto& m : matches) {
        size_t end_pos = current_pos + m.length;

        // 查詢拼寫對應的音節列表
        SpellingAccessor accessor(prism.QuerySpelling(m.value));
        while (!accessor.exhausted()) {
            SyllableId syllable_id = accessor.syllable_id();
            EdgeProperties props = accessor.properties();

            // 添加邊
            spellings[syllable_id] = props;

            accessor.Next();
        }

        // 將新頂點加入佇列
        queue.push(Vertex{end_pos, end_vertex_type});
    }
}
```

### Phase 2: 反向修剪

```cpp
// 從最遠點反向移除不可達的頂點和邊
set<int> good;
good.insert(farthest);

for (int i = farthest - 1; i >= 0; --i) {
    if (vertices.find(i) == vertices.end()) continue;

    // 移除不連接到 good 集合的邊
    for (auto j = edges[i].begin(); j != edges[i].end();) {
        if (good.find(j->first) == good.end()) {
            edges[i].erase(j++);  // 不可達
            continue;
        }
        // ...
    }

    if (edges[i].empty()) {
        vertices.erase(i);
    } else {
        good.insert(i);
    }
}
```

### Phase 3: 建立反向索引

```cpp
// 建立反向索引: indices[起點][音節ID] = [所有邊屬性]
void Syllabifier::Transpose(SyllableGraph* graph) {
    for (const auto& start : graph->edges) {
        auto& index(graph->indices[start.first]);
        for (const auto& end : start.second) {
            for (const auto& spelling : end.second) {
                SyllableId syll_id = spelling.first;
                index[syll_id].push_back(&spelling.second);
            }
        }
    }
}
```

---

## 歧義處理機制

### 問題場景

拼音 `niju'ede` 可能分割為：
- `ni + juede`（你覺得）
- `niju + ede`（？）

### 解決方案：懲罰機制

```cpp
void CheckOverlappedSpellings(SyllableGraph* graph, size_t start, size_t end) {
    // 如果存在 Z = YX，標記 Y 和 X 之間的頂點為歧義連接點
    auto& y_end_vertices = graph->edges[start];

    for (const auto& y : y_end_vertices) {
        size_t joint = y.first;  // Y 的終點 = X 的起點
        if (joint >= end) break;

        auto& x_end_vertices = graph->edges[joint];
        for (auto& x : x_end_vertices) {
            if (x.first == end) {
                // 發現歧義：start->joint->end 與 start->end 重疊

                // 懲罰歧義連接點的所有音節
                const double kPenalty = log(1e-10);  // ≈ -23
                for (auto& spelling : x.second) {
                    spelling.second.credibility += kPenalty;
                }
            }
        }
    }
}
```

---

## 權重計算系統

### 多層次權重

| 層級 | 來源 | 說明 |
|------|------|------|
| 拼寫層 | `EdgeProperties.credibility` | 模糊音、縮寫懲罰 |
| 詞典層 | `DictEntry.weight` | 詞頻 |
| 語法層 | `Grammar.Evaluate` | N-gram 上下文 |

### 懲罰值

```cpp
const double kAbbreviationPenalty = -log(0.5);     // -0.693（縮寫）
const double kFuzzySpellingPenalty = -log(0.5);    // -0.693（模糊音）
const double kCompletionPenalty = -log(0.5);       // -0.693（自動完成）
const double kCorrectionPenalty = -log(0.01);      // -4.605（糾錯）
const double kAmbiguousPenalty = -log(1e-10);      // -23.026（歧義）
```

---

## 與翻譯階段整合

### 整體流程

```
Input → Syllabifier → SyllableGraph → Dictionary Lookup →
    → Phrase Candidates → Poet (Sentence Making) → Final
```

### Dictionary 查詢（BFS 遍歷）

```cpp
bool Table::Query(const SyllableGraph& syll_graph,
                  size_t start_pos,
                  TableQueryResult* result) {
    queue<pair<size_t, TableQuery>> q;
    q.push({start_pos, initial_state});

    while (!q.empty()) {
        size_t current_pos = q.front().first;
        TableQuery query = q.front().second;
        q.pop();

        // 從 transposed index 獲取當前位置的所有音節
        auto index = syll_graph.indices.find(current_pos);

        for (const auto& [syll_id, props_list] : index->second) {
            TableAccessor accessor = query.Access(syll_id);

            for (auto props : props_list) {
                size_t end_pos = props->end_pos;

                if (!accessor.exhausted()) {
                    (*result)[end_pos].push_back(accessor);
                }

                // 繼續探索
                if (query.Advance(syll_id, props->credibility)) {
                    q.push({end_pos, query});
                }
            }
        }
    }
}
```

### Sentence Making（動態規劃）

```cpp
// WordGraph: map<起點, map<終點, DictEntryList>>
for (const auto& x : syllable_graph.edges) {
    auto& same_start_pos = graph[x.first];

    EnrollEntries(same_start_pos,
                  user_dict->Lookup(syllable_graph, x.first));
    EnrollEntries(same_start_pos,
                  dict->Lookup(syllable_graph, x.first));
}

// 使用 DP/Beam Search 找最佳句子
auto sentence = poet_->MakeSentence(graph,
                                    syllable_graph.interpreted_length,
                                    preceding_text);
```

---

## 台語鍵盤應用

### 簡化版實作建議

對於台語鍵盤，可以實作簡化版的分詞系統：

```kotlin
object TaigiSegmenter {
    // 台語音節表（從 dictionary 提取）
    private lateinit var syllables: Set<String>

    /**
     * 貪婪分詞 + 回溯
     */
    fun segment(input: String): List<String>? {
        return segmentRecursive(input, mutableListOf())
    }

    private fun segmentRecursive(
        remaining: String,
        result: MutableList<String>
    ): List<String>? {
        if (remaining.isEmpty()) {
            return result.toList()
        }

        // 嘗試最長匹配（貪婪）
        for (len in minOf(remaining.length, MAX_SYLLABLE_LEN) downTo 1) {
            val prefix = remaining.substring(0, len)

            if (syllables.contains(prefix)) {
                result.add(prefix)
                val rest = segmentRecursive(remaining.substring(len), result)
                if (rest != null) {
                    return rest
                }
                result.removeAt(result.lastIndex)  // 回溯
            }
        }

        return null  // 無法分詞
    }

    companion object {
        private const val MAX_SYLLABLE_LEN = 6
    }
}
```

### 進階版：DAG + Viterbi

```kotlin
data class SegmentNode(
    val start: Int,
    val end: Int,
    val syllable: String,
    val cost: Double
)

object TaigiDAGSegmenter {
    /**
     * 建立 DAG
     */
    fun buildDAG(input: String): Map<Int, List<SegmentNode>> {
        val dag = mutableMapOf<Int, MutableList<SegmentNode>>()

        for (i in input.indices) {
            dag[i] = mutableListOf()

            for (j in (i + 1)..minOf(i + MAX_SYLLABLE_LEN, input.length)) {
                val substr = input.substring(i, j)

                if (isValidSyllable(substr)) {
                    val cost = getSyllableCost(substr)
                    dag[i]!!.add(SegmentNode(i, j, substr, cost))
                }
            }
        }

        return dag
    }

    /**
     * Viterbi 找最佳路徑
     */
    fun findBestPath(dag: Map<Int, List<SegmentNode>>, length: Int): List<String> {
        // dp[i] = (最小成本, 前一個節點)
        val dp = Array(length + 1) { Double.MAX_VALUE to -1 }
        val path = Array(length + 1) { "" }
        dp[0] = 0.0 to -1

        for (i in 0 until length) {
            if (dp[i].first == Double.MAX_VALUE) continue

            for (node in dag[i] ?: emptyList()) {
                val newCost = dp[i].first + node.cost
                if (newCost < dp[node.end].first) {
                    dp[node.end] = newCost to i
                    path[node.end] = node.syllable
                }
            }
        }

        // 回溯
        val result = mutableListOf<String>()
        var pos = length
        while (pos > 0) {
            result.add(0, path[pos])
            pos = dp[pos].second
        }

        return result
    }

    private fun getSyllableCost(syllable: String): Double {
        // cost = -log(P) = -log(freq / total)
        val freq = getSyllableFrequency(syllable)
        return -ln(freq.toDouble() / totalFrequency)
    }
}
```

### 使用範例

```kotlin
// 輸入: "goabehchiahpng"
// 期望輸出: ["goa", "beh", "chiah", "png"]

val input = "goabehchiahpng"
val dag = TaigiDAGSegmenter.buildDAG(input)
val segments = TaigiDAGSegmenter.findBestPath(dag, input.length)

// 查詢字典
val query = segments.joinToString(" ")  // "goa beh chiah png"
val candidates = dictionary.search(query)
```

---

## 效能最佳化

### librime 的最佳化策略

| 策略 | 說明 |
|------|------|
| Transpose index | O(1) 查詢特定音節的所有邊 |
| Priority queue | 優先處理高品質路徑 |
| Partial sort | 只排序需要的候選詞 |
| Darts DoubleArray | 快速 trie 前綴搜尋 |

### 台語鍵盤建議

1. **預建音節表**：從 dictionary.db 提取所有合法音節
2. **Trie 索引**：用 Trie 加速前綴匹配
3. **快取常用分詞**：記憶常見輸入的分詞結果
4. **限制搜尋深度**：設定最大音節長度（如 6）

---

## 相關檔案

librime 原始碼：
- `src/rime/algo/syllabifier.{h,cc}` - 音節圖構建
- `src/rime/gear/abc_segmentor.{h,cc}` - ABC 分段器
- `src/rime/dict/table.{h,cc}` - 字典查詢
- `src/rime/gear/poet.{h,cc}` - 句子組合
- `src/rime/gear/script_translator.{h,cc}` - 整合翻譯
