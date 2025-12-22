# librime 配置驅動系統

> **關鍵字**: `YAML`, `Schema`, `配置編譯`, `動態載入`
> **更新日期**：2025-12-19

---

## 概述

librime 使用 **配置驅動** 設計，所有輸入法行為都由 YAML 配置檔定義：
- 不需修改程式碼即可調整行為
- 支援使用者自訂
- 支援多方案切換

---

## Schema 檔案結構

### 基礎結構

```yaml
# luna_pinyin.schema.yaml
schema:
  schema_id: luna_pinyin      # 唯一識別碼
  name: 朙月拼音               # 顯示名稱
  version: "0.15"
  author:
    - 佛振 <chen.sst@gmail.com>
  description: |
    多行描述文字

# 引擎組件配置
engine:
  processors:
    - ascii_composer
    - recognizer
    - speller
    - punctuator
    - selector
  segmentors:
    - ascii_segmentor
    - abc_segmentor
    - punct_segmentor
  translators:
    - punct_translator
    - script_translator
  filters:
    - simplifier
    - uniquifier

# 組件專屬配置
speller:
  alphabet: zyxwvutsrqponmlkjihgfedcba
  delimiter: " '"
  algebra:
    - derive/^([nl])ve$/$1ue/

translator:
  dictionary: luna_pinyin
  preedit_format:
    - xform/([nljqxy])v/$1ü/
```

### 組件命名空間

使用 `@alias` 語法支援多實例：

```yaml
engine:
  translators:
    - script_translator           # 使用預設命名空間
    - table_translator@cangjie    # 使用 cangjie 命名空間
  filters:
    - simplifier@zh_simp          # 使用 zh_simp 命名空間

# 各命名空間的配置
cangjie:
  dictionary: cangjie5
  prefix: 'C:'
  suffix: ';'

zh_simp:
  option_name: zh_simp
  opencc_config: s2t.json
```

---

## 配置指令

### 1. `__include` - 引入配置

```yaml
# 引入整個檔案
punctuator:
  __include: default:/punctuator

# 引入特定節點
my_config:
  __include: other_file:/path/to/node

# 可選引入（不存在時不報錯）
optional:
  __include: maybe_exists:/node?
```

### 2. `__patch` - 修補配置

```yaml
# 字面修補
base_config:
  __patch:
    key1: new_value
    nested/key: value

# 引用外部修補
__patch: /local/patch

# 修補列表
__patch:
  - patch1
  - patch2
```

### 3. `__append` - 追加內容

```yaml
# 追加到列表
list:
  __include: base_list
  __append:
    - new_item1
    - new_item2

# 使用 + 後綴
some_list/+:
  - appended_item
```

### 4. `__merge` - 合併樹狀結構

```yaml
# 深度合併 Map
merged_config:
  __include: base_config
  override_key: new_value    # 覆蓋
  nested:                    # 深度合併
    sub_key: value
```

---

## 路徑語法

### 基本語法

```yaml
# 路徑使用 / 分隔
menu/page_size              # menu.page_size
engine/processors/@0        # processors 列表第 0 項
```

### 列表索引

| 語法 | 說明 |
|------|------|
| `@0`, `@1` | 絕對索引 |
| `@next` | 列表末尾（追加） |
| `@last` | 最後一個元素 |
| `@before 0` | 在索引 0 之前 |
| `@after 2` | 在索引 2 之後 |

### 操作後綴

| 後綴 | 說明 |
|------|------|
| `/+` | 追加 |
| `/=` | 強制覆蓋（不合併） |

---

## 使用者自訂機制

### .custom.yaml 檔案

使用者修改存放在獨立檔案，不影響原始配置：

```yaml
# luna_pinyin.custom.yaml
patch:
  # 修改頁面大小
  menu/page_size: 9

  # 新增拼寫規則
  speller/algebra/+:
    - derive/^([jqxy])u/$1v/

  # 新增翻譯器
  engine/translators/@next: table_translator
```

### 自動載入機制

```
luna_pinyin.schema.yaml
        ↓
    ConfigCompiler
        ↓
    自動檢查 luna_pinyin.custom.yaml
        ↓
    應用 patch 節點
        ↓
    最終配置
```

---

## 配置編譯流程

### 兩階段處理

```
Phase 1: Compile
├── LoadFromFile() - 載入 YAML
├── ConvertFromYaml() - 建立配置樹
└── 建立依賴關係圖

Phase 2: Link
├── ResolveDependencies() - 解析依賴
├── IncludeReference::Resolve() - 處理 __include
├── PatchReference::Resolve() - 處理 __patch
└── 檢查循環依賴
```

### 依賴優先級

```cpp
enum DependencyPriority {
    kPendingChild = 0,  // 子節點
    kInclude = 1,       // __include
    kPatch = 2,         // __patch（最高）
};
```

執行順序：子節點 → `__include` → `__patch`

---

## Schema 切換

### 配置

```yaml
# default.yaml
schema_list:
  - schema: luna_pinyin
  - schema: cangjie5

switcher:
  caption: 〔方案選單〕
  hotkeys:
    - Control+grave
    - F4
  save_options:
    - full_shape
    - ascii_punct
```

### 條件切換

```yaml
schema_list:
  - case: [mode/expert]
    schema: advanced_pinyin
  - schema: simple_pinyin     # 預設
```

### 選項持久化

選項自動保存到 `user.yaml`：

```yaml
# user.yaml
var:
  option:
    full_shape: false
    ascii_punct: true
  previously_selected_schema: luna_pinyin
```

---

## 組件配置讀取

### 讀取方式

```cpp
// 組件構造時讀取配置
AbcSegmentor::AbcSegmentor(const Ticket& ticket) {
    Config* config = ticket.schema->config();

    // 讀取通用配置
    config->GetString("speller/alphabet", &alphabet_);

    // 讀取組件專屬配置（使用 name_space）
    string key = name_space_ + "/extra_tags";
    config->GetList(key, &extra_tags_);
}
```

### 支援的類型

```cpp
config->GetString("path", &string_value);
config->GetInt("path", &int_value);
config->GetDouble("path", &double_value);
config->GetBool("path", &bool_value);
config->GetList("path");   // 返回 ConfigList
config->GetMap("path");    // 返回 ConfigMap
```

---

## 台語鍵盤應用

### 方案一：簡化版 YAML 配置

```yaml
# taigi.schema.yaml
schema:
  id: taigi
  name: 台語輸入法
  version: "1.0"

# 基本設定
settings:
  page_size: 9
  default_input_mode: hanzi    # hanzi / lomaji
  default_romanization: tl     # tl / poj

# 字典設定
dictionary:
  path: dictionary.db
  user_path: user_dictionary.db

# 候選詞排序
ranking:
  user_weight: 50
  dict_weight: 1
  time_decay_hours: 168        # 一週半衰期

# 拼寫變體（POJ/TL 統一）
spelling:
  variants:
    - from: ch
      to: ts
    - from: chh
      to: tsh
    - from: oa
      to: ua
```

### 方案二：Kotlin 配置類別

```kotlin
// 配置資料類別
data class TaigiSchema(
    val id: String,
    val name: String,
    val settings: Settings,
    val dictionary: DictionaryConfig,
    val ranking: RankingConfig,
    val spelling: SpellingConfig
)

data class Settings(
    val pageSize: Int = 9,
    val defaultInputMode: InputMode = InputMode.HANZI,
    val defaultRomanization: Romanization = Romanization.TL
)

data class RankingConfig(
    val userWeight: Int = 50,
    val dictWeight: Int = 1,
    val timeDecayHours: Int = 168
)

data class SpellingConfig(
    val variants: List<SpellingVariant> = emptyList()
)

data class SpellingVariant(
    val from: String,
    val to: String
)
```

### 方案三：配置載入器

```kotlin
object SchemaLoader {
    private val yaml = Yaml()

    fun load(context: Context, schemaId: String): TaigiSchema {
        // 1. 載入基礎配置
        val baseConfig = loadFromAssets(context, "$schemaId.schema.yaml")

        // 2. 載入使用者自訂（如果存在）
        val customConfig = loadFromFiles(context, "$schemaId.custom.yaml")

        // 3. 合併配置
        return merge(baseConfig, customConfig)
    }

    private fun merge(base: Map<String, Any>, custom: Map<String, Any>?): TaigiSchema {
        val merged = base.toMutableMap()

        custom?.get("patch")?.let { patch ->
            applyPatch(merged, patch as Map<String, Any>)
        }

        return parseSchema(merged)
    }

    private fun applyPatch(target: MutableMap<String, Any>, patch: Map<String, Any>) {
        for ((path, value) in patch) {
            setByPath(target, path, value)
        }
    }

    private fun setByPath(target: MutableMap<String, Any>, path: String, value: Any) {
        val parts = path.split("/")
        var current: MutableMap<String, Any> = target

        for (i in 0 until parts.size - 1) {
            current = current.getOrPut(parts[i]) { mutableMapOf<String, Any>() }
                as MutableMap<String, Any>
        }

        val lastKey = parts.last()
        if (lastKey.endsWith("+")) {
            // 追加模式
            val key = lastKey.dropLast(1)
            val existing = current[key] as? MutableList<Any> ?: mutableListOf()
            existing.addAll(value as List<Any>)
            current[key] = existing
        } else {
            current[lastKey] = value
        }
    }
}
```

### 使用範例

```kotlin
// 載入配置
val schema = SchemaLoader.load(context, "taigi")

// 使用配置
val engine = TaigiInputEngine(
    pageSize = schema.settings.pageSize,
    userWeight = schema.ranking.userWeight,
    spellingVariants = schema.spelling.variants
)

// 使用者自訂
// taigi.custom.yaml:
// patch:
//   settings/page_size: 5
//   ranking/user_weight: 100
```

---

## 優點與考量

### 優點

1. **不改程式碼**：調整行為只需修改配置
2. **使用者自訂**：`.custom.yaml` 不影響原始配置
3. **多方案支援**：可切換不同輸入方案
4. **可追蹤**：配置檔案可版本控制

### 考量

1. **解析成本**：YAML 解析需要時間
2. **驗證複雜**：需要驗證配置正確性
3. **除錯困難**：配置錯誤可能難以追蹤

### 建議

對於台語鍵盤：
- **Phase 1**：使用 Kotlin data class 定義配置結構
- **Phase 2**：支援從 YAML 載入配置
- **Phase 3**：支援 `.custom.yaml` 機制

---

## 相關檔案

librime 原始碼：
- `src/rime/config/config_types.{h,cc}` - 配置類型
- `src/rime/config/config_data.{h,cc}` - YAML 載入
- `src/rime/config/config_compiler.{h,cc}` - 配置編譯
- `src/rime/schema.{h,cc}` - Schema 類別
- `src/rime/switcher.{h,cc}` - Schema 切換

範例配置：
- `data/minimal/default.yaml` - 預設配置
- `data/minimal/luna_pinyin.schema.yaml` - Schema 範例
