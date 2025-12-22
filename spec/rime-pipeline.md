# librime Pipeline 架構

> **關鍵字**: `Pipeline`, `Engine`, `Component`, `Signal/Slot`
> **更新日期**：2025-12-19

---

## 架構概覽

librime 使用 **Pipeline 架構** 處理輸入，將處理流程分為四個階段：

```
按鍵輸入
    ↓
┌─────────────────┐
│   Processors    │  處理按鍵事件
└────────┬────────┘
         ↓
┌─────────────────┐
│   Segmentors    │  分割輸入
└────────┬────────┘
         ↓
┌─────────────────┐
│  Translators    │  產生候選詞
└────────┬────────┘
         ↓
┌─────────────────┐
│    Filters      │  過濾/排序
└────────┬────────┘
         ↓
      候選列表
```

---

## 核心類別

### Engine（引擎編排器）

```cpp
class ConcreteEngine : public Engine {
    the<Schema> schema_;                    // 配置
    the<Context> context_;                  // 上下文
    CommitSink sink_;                       // 輸出信號

    vector<of<Processor>> processors_;      // 處理器鏈
    vector<of<Segmentor>> segmentors_;      // 分段器鏈
    vector<of<Translator>> translators_;    // 翻譯器鏈
    vector<of<Filter>> filters_;            // 過濾器鏈
};
```

### Context（上下文管理）

```cpp
class Context {
    string input_;                    // 原始輸入
    size_t caret_pos_;               // 光標位置
    Composition composition_;         // 分段與候選詞
    CommitHistory commit_history_;   // 提交歷史

    // Signal 通知系統
    Notifier commit_notifier_;
    Notifier select_notifier_;
    Notifier update_notifier_;
};
```

---

## Pipeline 組件介面

### 1. Processor（按鍵處理器）

```cpp
enum ProcessResult {
    kRejected,  // 拒絕，由系統處理
    kAccepted,  // 已處理，停止鏈
    kNoop,      // 未處理，繼續下一個
};

class Processor {
    virtual ProcessResult ProcessKeyEvent(const KeyEvent& key_event) = 0;
};
```

**常見 Processor**：

| 名稱 | 功能 |
|------|------|
| `speller` | 處理字母輸入 |
| `selector` | 候選選擇（數字鍵） |
| `navigator` | 游標移動 |
| `punctuator` | 標點處理 |
| `key_binder` | 快捷鍵綁定 |

### 2. Segmentor（分段器）

```cpp
class Segmentor {
    virtual bool Proceed(Segmentation* segmentation) = 0;
    // 返回 true 繼續，false 終止鏈
};
```

**範例實作**：

```cpp
bool AbcSegmentor::Proceed(Segmentation* segmentation) {
    const string& input = segmentation->input();
    size_t start = segmentation->GetCurrentStartPosition();
    size_t end = start;

    // 識別字母序列
    for (; end < input.length(); ++end) {
        if (alphabet_.find(input[end]) == string::npos) break;
    }

    if (start < end) {
        Segment segment(start, end);
        segment.tags.insert("abc");
        segmentation->AddSegment(segment);
    }

    return true;  // 繼續處理
}
```

### 3. Translator（翻譯器）

```cpp
class Translator {
    virtual an<Translation> Query(const string& input,
                                  const Segment& segment) = 0;
};

class Translation {
    virtual bool Next() = 0;           // 下一個候選
    virtual an<Candidate> Peek() = 0;  // 當前候選
};
```

**特點**：
- 返回 `Translation` 迭代器
- **延遲計算**：按需生成候選詞
- 多個 Translator 結果會合併

### 4. Filter（過濾器）

```cpp
class Filter {
    virtual an<Translation> Apply(an<Translation> translation,
                                  CandidateList* candidates) = 0;
};
```

**常見 Filter**：

| 名稱 | 功能 |
|------|------|
| `uniquifier` | 去除重複候選 |
| `simplifier` | 簡繁轉換（OpenCC） |
| `reverse_lookup_filter` | 反查提示 |

---

## 資料流

### 完整處理流程

```
用戶按 'n'
    │
    ↓
ProcessKey (Processor Chain)
    │ Speller: kAccepted
    │   └─ Context::PushInput('n')
    │      └─ update_notifier_()
    ↓
Engine::Compose
    │
    ├─ CalculateSegmentation
    │   └─ AbcSegmentor::Proceed
    │      └─ AddSegment([0,5], tags=["abc"])
    │
    └─ TranslateSegments
        ├─ ScriptTranslator::Query("nihao")
        │   └─ Translation{候選詞...}
        │
        └─ Menu
            ├─ AddTranslation(translation)
            ├─ AddFilter(uniquifier)
            └─ Prepare(page_size)
                └─ 候選列表
```

### Signal 觸發鏈

```cpp
Context::PushInput('n')
    → input_ = "n"
    → update_notifier_(this)
    → Engine::OnContextUpdate
    → Engine::Compose
```

---

## 組件註冊系統

### Registry（註冊表）

```cpp
class Registry {
    ComponentMap map_;  // map<string, ComponentBase*>

    ComponentBase* Find(const string& name);
    void Register(const string& name, ComponentBase* component);
    static Registry& instance();  // 單例
};
```

### 模組註冊

```cpp
// gears_module.cc
static void rime_gears_initialize() {
    Registry& r = Registry::instance();

    // Processors
    r.Register("speller", new Component<Speller>);
    r.Register("selector", new Component<Selector>);

    // Segmentors
    r.Register("abc_segmentor", new Component<AbcSegmentor>);

    // Translators
    r.Register("script_translator", new Component<ScriptTranslator>);

    // Filters
    r.Register("uniquifier", new Component<Uniquifier>);
}

RIME_REGISTER_MODULE(gears)
```

### Ticket（組件初始化參數）

```cpp
struct Ticket {
    Engine* engine;
    Schema* schema;
    string name_space;  // 命名空間（讀取配置用）
    string klass;       // 類別名稱
};

// 範例：prescription = "table_translator@cangjie"
// → klass = "table_translator"
// → name_space = "cangjie"
```

### 從 Schema 實例化

```cpp
void ConcreteEngine::InitializeComponents() {
    Config* config = schema_->config();

    // 讀取配置
    auto processor_list = config->GetList("engine/processors");

    for (size_t i = 0; i < processor_list->size(); ++i) {
        string prescription = processor_list->GetAt(i)->str();

        // 建立 Ticket
        Ticket ticket{this, "processor", prescription};

        // 查找工廠
        auto factory = Processor::Require(ticket.klass);

        // 建立實例
        processors_.push_back(factory->Create(ticket));
    }
}
```

---

## 架構圖

### 組件層級

```
┌─────────────────────────────────────────────────────────┐
│                    Application Layer                     │
└─────────────────────────────────────────────────────────┘
                            ↕
┌─────────────────────────────────────────────────────────┐
│                      Engine Layer                        │
│  ┌────────────┐  ┌────────────┐  ┌────────────┐        │
│  │  Engine    │←→│  Context   │←→│  Schema    │        │
│  └────────────┘  └────────────┘  └────────────┘        │
└─────────────────────────────────────────────────────────┘
                            ↕
┌─────────────────────────────────────────────────────────┐
│                   Pipeline Components                    │
│  Processors    Segmentors    Translators    Filters     │
└─────────────────────────────────────────────────────────┘
                            ↕
┌─────────────────────────────────────────────────────────┐
│                   Infrastructure Layer                   │
│  Registry      Config       Dictionary      Prism       │
└─────────────────────────────────────────────────────────┘
```

### 資料流轉

```
用戶按鍵
    │
    ↓
┌───────────────────────────────────────────────┐
│  Processor Chain                              │
│  ┌─────────┐   ┌─────────┐   ┌─────────┐    │
│  │ Speller │→→→│Selector │→→→│Punctuator│   │
│  └─────────┘   └─────────┘   └─────────┘    │
└───────────────────────────────────────────────┘
    │
    ↓
┌───────────────────────────────────────────────┐
│  Segmentor Chain                              │
│  ┌─────────┐   ┌─────────┐   ┌─────────┐    │
│  │   Abc   │→→→│  Punct  │→→→│Fallback │    │
│  └─────────┘   └─────────┘   └─────────┘    │
└───────────────────────────────────────────────┘
    │
    ↓
┌───────────────────────────────────────────────┐
│  Translator + Filter                          │
│  ┌────────┐  ┌────────┐                      │
│  │Script  │  │ Table  │                      │
│  │Trans   │  │ Trans  │                      │
│  └────┬───┘  └───┬────┘                      │
│       └─────┬────┘                           │
│             ↓                                 │
│       MergedTranslation                      │
│             ↓                                 │
│       Filter Chain                           │
│             ↓                                 │
│         Candidates                           │
└───────────────────────────────────────────────┘
```

---

## 設計模式

| 模式 | 應用 |
|------|------|
| **Chain of Responsibility** | Processor/Segmentor 鏈式處理 |
| **Strategy** | Translator/Filter 可替換 |
| **Factory** | Component 模板工廠 |
| **Observer** | Signal/Slot 事件通知 |
| **Iterator** | Translation 延遲生成 |
| **Decorator** | Filter 包裝 Translation |

---

## 台語鍵盤應用

### 簡化版 Pipeline

```kotlin
// 定義組件介面
interface KeyProcessor {
    fun process(event: KeyEvent, context: InputContext): ProcessResult
}

interface Segmentor {
    fun segment(input: String): List<Segment>
}

interface Translator {
    fun translate(segment: Segment): List<Candidate>
}

interface CandidateFilter {
    fun filter(candidates: List<Candidate>): List<Candidate>
}

enum class ProcessResult { ACCEPTED, REJECTED, NOOP }
```

### InputEngine 實作

```kotlin
class TaigiInputEngine(
    private val processors: List<KeyProcessor>,
    private val segmentors: List<Segmentor>,
    private val translators: List<Translator>,
    private val filters: List<CandidateFilter>
) {
    private val context = InputContext()

    fun processKey(event: KeyEvent): Boolean {
        // 1. Processor Chain
        for (processor in processors) {
            when (processor.process(event, context)) {
                ProcessResult.ACCEPTED -> {
                    compose()
                    return true
                }
                ProcessResult.REJECTED -> return false
                ProcessResult.NOOP -> continue
            }
        }
        return false
    }

    private fun compose() {
        // 2. Segmentation
        val segments = mutableListOf<Segment>()
        for (segmentor in segmentors) {
            segments.addAll(segmentor.segment(context.input))
        }

        // 3. Translation
        val allCandidates = mutableListOf<Candidate>()
        for (segment in segments) {
            for (translator in translators) {
                allCandidates.addAll(translator.translate(segment))
            }
        }

        // 4. Filtering
        var candidates = allCandidates
        for (filter in filters) {
            candidates = filter.filter(candidates)
        }

        context.candidates = candidates
    }
}
```

### 組件實作範例

```kotlin
// Processor: 處理字母輸入
class LetterProcessor : KeyProcessor {
    override fun process(event: KeyEvent, context: InputContext): ProcessResult {
        if (event.isLetter()) {
            context.appendInput(event.char)
            return ProcessResult.ACCEPTED
        }
        return ProcessResult.NOOP
    }
}

// Segmentor: 台語音節分段
class TaigiSegmentor(
    private val syllableSet: Set<String>
) : Segmentor {
    override fun segment(input: String): List<Segment> {
        // 使用 DAG 分詞（見 rime-segmentation.md）
        return TaigiDAGSegmenter.segment(input, syllableSet)
    }
}

// Translator: 字典查詢
class DictionaryTranslator(
    private val dictionary: TaigiDictionary
) : Translator {
    override fun translate(segment: Segment): List<Candidate> {
        return dictionary.lookup(segment.text)
    }
}

// Filter: 使用者詞彙優先
class UserPhraseFilter(
    private val userDict: UserDictionary
) : CandidateFilter {
    override fun filter(candidates: List<Candidate>): List<Candidate> {
        return candidates.sortedByDescending { candidate ->
            if (userDict.contains(candidate.text)) 1.0 else 0.0
        }
    }
}
```

### 建立 Engine

```kotlin
val engine = TaigiInputEngine(
    processors = listOf(
        LetterProcessor(),
        SelectorProcessor(),
        PunctuationProcessor()
    ),
    segmentors = listOf(
        TaigiSegmentor(syllableSet),
        PunctuationSegmentor()
    ),
    translators = listOf(
        DictionaryTranslator(dictionary),
        UserDictTranslator(userDict)
    ),
    filters = listOf(
        UserPhraseFilter(userDict),
        UniquifyFilter(),
        FrequencyFilter()
    )
)
```

---

## 優點與考量

### 優點

1. **模組化**：每個組件職責單一，易於測試
2. **可擴展**：新增功能只需新增組件
3. **可配置**：組件組合可由配置控制
4. **解耦**：組件間透過介面溝通

### 考量

1. **複雜度**：比單一服務複雜
2. **效能**：多層抽象可能有額外開銷
3. **學習曲線**：需要理解整體架構

### 建議

對於台語鍵盤，可以：
- **Phase 1**：保持現有架構，但重構為更清晰的層次
- **Phase 2**：引入 Translator/Filter 概念
- **Phase 3**：完整 Pipeline（如果需要支援多種輸入方案）

---

## 相關檔案

librime 原始碼：
- `src/rime/engine.{h,cc}` - Engine 編排
- `src/rime/processor.h` - Processor 介面
- `src/rime/segmentor.h` - Segmentor 介面
- `src/rime/translator.h` - Translator 介面
- `src/rime/filter.h` - Filter 介面
- `src/rime/context.{h,cc}` - Context 管理
- `src/rime/registry.{h,cc}` - 組件註冊
