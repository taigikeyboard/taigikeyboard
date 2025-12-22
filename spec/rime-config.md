# librime 配置系統

> **關鍵字**: `librime`, `YAML`, `Schema`, `配置編譯`
> **更新日期**：2025-12-19

---

## Schema 配置系統

### 核心概念

librime 使用 **YAML DSL** 定義輸入法方案，每個 schema 是一個獨立的輸入法配置。

### 配置指令

```yaml
# 引入其他配置檔案
__include: another_config.yaml

# 覆蓋現有配置
__patch:
  key: new_value

# 附加到列表
__append:
  - new_item

# 合併配置
__merge:
  key1: value1
```

### Schema 結構

```yaml
# luna_pinyin.schema.yaml 範例
schema:
  schema_id: luna_pinyin
  name: 朙月拼音
  version: "0.9"
  author:
    - 佛振 <chen.sst@gmail.com>

engine:
  processors:           # 按鍵處理器
    - ascii_composer
    - recognizer
    - key_binder
    - speller
    - punctuator
    - selector
    - navigator
    - express_editor
  segmentors:           # 分段器
    - ascii_segmentor
    - matcher
    - abc_segmentor
    - punct_segmentor
    - fallback_segmentor
  translators:          # 翻譯器
    - punct_translator
    - table_translator@custom_phrase
    - reverse_lookup_translator
    - script_translator
  filters:              # 過濾器
    - simplifier
    - uniquifier

speller:
  alphabet: zyxwvutsrqponmlkjihgfedcba
  delimiter: " '"
  algebra:              # 拼寫變換規則
    - erase/^xx$/
    - abbrev/^([a-z]).+$/$1/
    - derive/^([nl])ve$/$1ue/
```

---

## Engine Pipeline

### 處理流程

```
按鍵輸入
   ↓
┌─────────────────┐
│   Processors    │  處理特殊按鍵（Ctrl、方向鍵等）
└────────┬────────┘
         ↓
┌─────────────────┐
│   Segmentors    │  將輸入分割成段落
└────────┬────────┘
         ↓
┌─────────────────┐
│  Translators    │  產生候選詞
└────────┬────────┘
         ↓
┌─────────────────┐
│    Filters      │  過濾/排序候選詞
└────────┬────────┘
         ↓
      候選列表
```

### Processor 類型

| 名稱 | 功能 |
|------|------|
| `ascii_composer` | ASCII 模式切換 |
| `recognizer` | 模式識別（反查等） |
| `key_binder` | 按鍵綁定 |
| `speller` | 拼寫處理 |
| `punctuator` | 標點處理 |
| `selector` | 候選選擇 |
| `navigator` | 游標導航 |
| `express_editor` | 快速編輯 |
| `fluid_editor` | 流式編輯 |
| `chord_composer` | 並擊輸入 |

### Translator 類型

| 名稱 | 功能 |
|------|------|
| `script_translator` | 主翻譯器（拼音→漢字） |
| `table_translator` | 碼表翻譯器 |
| `punct_translator` | 標點翻譯 |
| `reverse_lookup_translator` | 反查翻譯 |
| `echo_translator` | 回顯輸入 |
| `history_translator` | 歷史記錄 |

### Filter 類型

| 名稱 | 功能 |
|------|------|
| `simplifier` | 簡繁轉換（OpenCC） |
| `uniquifier` | 去重 |
| `cjk_minifier` | 限制 CJK 候選數量 |
| `reverse_lookup_filter` | 反查提示 |
| `single_char_filter` | 單字過濾 |

---

## 部署系統

### 編譯流程

```
schema.yaml + dict.txt
        ↓
   DictCompiler
        ↓
┌───────────────────────────────────┐
│  .prism.bin   拼寫索引 (Trie)      │
│  .table.bin   詞條表               │
│  .reverse.bin 反查索引             │
└───────────────────────────────────┘
```

### 部署任務

| 任務 | 功能 |
|------|------|
| `DetectModifications` | 檢測配置變更 |
| `SchemaUpdate` | 更新 schema 並編譯字典 |
| `PrebuildAllSchemas` | 預編譯所有 schema |
| `UserDictUpgrade` | 升級使用者字典 |
| `UserDictSync` | 同步使用者字典 |
| `BackupConfigFiles` | 備份配置 |

### 編譯產物

| 檔案 | 格式 | 用途 |
|------|------|------|
| `.prism.bin` | Darts (Double-Array Trie) | 拼寫→音節映射 |
| `.table.bin` | 自訂二進位 | 詞條及權重 |
| `.reverse.bin` | Memory-mapped | 反查索引 |

---

## 設計模式

### Component Pattern

每個功能都是可插拔組件：

```cpp
// 組件基類
class Component {
public:
    virtual ~Component() = default;
};

// 處理器接口
class Processor : public Component {
public:
    virtual ProcessResult ProcessKeyEvent(const KeyEvent& key) = 0;
};

// 翻譯器接口
class Translator : public Component {
public:
    virtual an<Translation> Query(const string& input, const Segment& segment) = 0;
};
```

### Plugin 模式

配置編譯器支援插件擴展：

```cpp
class ConfigCompilerPlugin {
public:
    virtual bool ReviewCompileOutput(ConfigCompiler* compiler,
                                    an<ConfigResource> resource) = 0;
    virtual bool ReviewLinkOutput(ConfigCompiler* compiler,
                                 an<ConfigResource> resource) = 0;
};
```

---

## 與 TaigiKeyboard 的比較

| 項目 | librime | TaigiKeyboard |
|------|---------|---------------|
| **配置格式** | YAML DSL | Kotlin 硬編碼 |
| **Pipeline** | Processor→Segmentor→Translator→Filter | 單一流程 |
| **擴展性** | 插件系統 | 直接修改程式碼 |
| **字典編譯** | 離線預編譯 | 直接使用 SQLite |

### 值得借鑑

1. **Pipeline 架構**：將處理流程模組化
2. **配置驅動**：用配置檔定義行為，減少程式碼修改
3. **預編譯字典**：Trie 結構加速查詢

---

## 相關檔案

- `src/rime/config/config_compiler.{h,cc}` - 配置編譯器
- `src/rime/lever/deployment_tasks.{h,cc}` - 部署任務
- `src/rime/dict/dict_compiler.{h,cc}` - 字典編譯器
- `src/rime/gear/*.{h,cc}` - 各種處理器/翻譯器/過濾器
