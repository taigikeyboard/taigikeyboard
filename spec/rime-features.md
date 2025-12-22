# librime 其他功能

> **關鍵字**: `librime`, `OpenCC`, `反查`, `標點`, `並擊`, `造句`
> **更新日期**：2025-12-19

---

## 1. OpenCC 簡繁轉換

### 功能

整合 OpenCC 實現簡繁轉換、地區詞彙轉換。

### 配置

```yaml
simplifier:
  opencc_config: t2s.json       # 繁→簡
  # opencc_config: s2t.json     # 簡→繁
  # opencc_config: t2tw.json    # 繁→台灣正體
  # opencc_config: t2hk.json    # 繁→香港繁體

  option_name: simplification   # 開關名稱
  tips: all                     # 顯示提示：none/char/all
  show_in_comment: true         # 在註釋中顯示轉換結果
  excluded_types:               # 排除特定候選類型
    - reverse_lookup
```

### 實作方式

```cpp
class Simplifier : public Filter {
    // 使用 OpenCC 進行轉換
    bool Convert(const string& text, vector<string>* forms);

    // 產生 ShadowCandidate 包裝轉換後的候選
    an<Candidate> Simplify(const an<Candidate>& candidate);
};
```

---

## 2. Reverse Lookup 反查

### 功能

用一種輸入法查詢另一種輸入法的編碼，例如：
- 用倉頡查拼音
- 用拼音查倉頡
- 用筆畫查拼音

### 配置

```yaml
reverse_lookup:
  dictionary: cangjie5          # 反查字典
  prefix: '`'                   # 觸發前綴
  suffix: "'"                   # 結束符號
  tips: 〔倉頡〕                # 提示文字
  preedit_format:
    - "xlit|abcdefghijklmnopqrstuvwxy|日月金木水火土竹戈十大中一弓人心手口尸廿山女田難卜符|"
  comment_format:
    - "xform/$/〕/"
    - "xform/^/〔/"

recognizer:
  patterns:
    reverse_lookup: "`[a-z]*'?$"  # 識別模式
```

### 資料結構

```cpp
class ReverseDb {
    // 查找字符的編碼
    bool Lookup(const string& text, string* result);

    struct Metadata {
        OffsetPtr<char> key_trie;    // 字符 Trie
        OffsetPtr<char> value_trie;  // 編碼 Trie
    };
};
```

---

## 3. Punctuation 標點處理

### 標點類型

| 類型 | 說明 | 範例 |
|------|------|------|
| Unique | 單一映射 | `,` → `，` |
| Alternating | 循環候選 | `/` → `、/／/÷` |
| Auto Commit | 自動上屏 | `.` → `。` |
| Paired | 成對標點 | `(` → `（）` |

### 配置

```yaml
punctuator:
  import_preset: default

  half_shape:
    ',': { commit: '，' }
    '.': { commit: '。' }
    '/': ['、', '/', '／', '÷']
    '(': ['（', '(']
    ')': ['）', ')']

  full_shape:
    ',': '，'
    '.': '。'

  symbols:
    '/fh': ['〈', '〉', '《', '》']  # 自訂符號
```

### 數字分隔符

```yaml
punctuator:
  digit_separators: ".:"         # 識別 3.14, 12:30
  digit_separator_action: commit # 或 forward
```

智能識別數字格式，如：
- `3.14` → 保留小數點
- `12:30` → 保留冒號
- `1,000` → 保留千分位

---

## 4. Chord Typing 並擊輸入

### 概念

同時按下多個按鍵產生輸入，類似速記鍵盤。

### 配置

```yaml
chord_composer:
  alphabet: "qwertasdfgzxcvbyuiophjkl;nm,./"
  algebra:
    - 'xlit|qwertasdfgzxcvbyuiophjkl;nm,./|QWERTASDFGZXCVBYUIOPHJKL:NM<>?|'
    - 'xform/^(.*)$/[$1]/'
  output_format:
    - 'xform/\[(.+)\]/\1/'
  prompt_format:
    - 'xform/^(.*)$/[$1]/'
  finish_chord_on_first_key_release: true  # 釋放首鍵即完成
```

### 工作流程

```
1. 監聽 KeyDown/KeyUp
2. 累積按鍵 → recognized_chord
3. 首鍵釋放（或全部釋放）→ 完成和弦
4. algebra 變換 → 輸入碼
5. output_format → 按鍵序列
6. 送入引擎處理
```

---

## 5. Composition 編輯區

### Editor 類型

| 類型 | 說明 |
|------|------|
| `express_editor` | 快速模式：空格直接上屏 |
| `fluid_editor` | 流式模式：空格分詞，回車上屏 |

### 編輯操作

```cpp
class Editor : public Processor {
    Handler Confirm;                  // 確認選擇
    Handler CommitComposition;        // 提交組合
    Handler CommitRawInput;           // 提交原始輸入
    Handler RevertLastEdit;           // 撤銷
    Handler BackToPreviousSyllable;   // 返回上一音節
    Handler DeleteCandidate;          // 刪除候選
    Handler DeleteChar;               // 刪除字符
    Handler CancelComposition;        // 取消組合
};
```

### Composition 結構

```cpp
struct Segment {
    size_t start, end;              // 範圍
    set<string> tags;               // 標籤
    an<Menu> menu;                  // 候選菜單
    size_t selected_index;          // 選中索引
    string prompt;                  // 提示
};

class Composition : public vector<Segment> {
    // 支援部分確認、回退
    bool HasFinishedComposition();
    bool GetPreedit(Preedit* preedit);
};
```

---

## 6. Poet 造句系統

### 功能

從多個候選詞組合成句子，選擇最佳組合。

### 演算法

```cpp
class Poet {
    using WordGraph = map<int, map<int, DictEntryList>>;

    // 構建句子
    an<Sentence> MakeSentence(
        const WordGraph& graph,
        size_t total_length,
        const string& preceding_text  // 前文上下文
    );
};
```

### 比較策略

| 策略 | 說明 |
|------|------|
| `CompareWeight` | 按權重排序 |
| `LeftAssociateCompare` | 左結合優先（偏好較長詞） |

### 上下文加權

考慮前文進行候選排序：

```cpp
an<Translation> ContextualWeighted(
    an<Translation> translation,
    const string& input,
    size_t start,
    const string& preceding_text,  // 前一個詞
    an<Grammar> grammar            // 語言模型
);
```

---

## 7. History 歷史記錄

### 配置

```yaml
history:
  input: z
  size: 5                    # 記錄數量
  initial_quality: -1        # 初始品質（排序權重）
```

### 功能

- 記錄最近輸入的詞彙
- 按特定按鍵觸發歷史列表
- 方便重複輸入常用詞

---

## 8. Key Binder 按鍵綁定

### 配置

```yaml
key_binder:
  import_preset: default
  bindings:
    - { when: composing, accept: Control+p, send: Up }
    - { when: composing, accept: Control+n, send: Down }
    - { when: has_menu, accept: Tab, send: Page_Down }
    - { when: paging, accept: minus, send: Page_Up }
```

### 條件

| 條件 | 說明 |
|------|------|
| `always` | 始終生效 |
| `composing` | 組字中 |
| `has_menu` | 有候選列表 |
| `paging` | 翻頁中 |

---

## 9. Recognizer 模式識別

### 配置

```yaml
recognizer:
  import_preset: default
  patterns:
    email: "^[A-Za-z][-_.0-9A-Za-z]*@.*$"
    uppercase: "[A-Z][-_+.'0-9A-Za-z]*$"
    url: "^(www[.]|https?:|ftp[.:]|mailto:|file:).*$"
    reverse_lookup: "`[a-z]*'?$"
    punct: "^/([0-9]0?|[A-Za-z]+)$"
```

### 功能

- 識別特殊輸入模式（Email、URL 等）
- 觸發對應的處理器/翻譯器
- 支援正則表達式

---

## 10. Switcher 方案切換

### 配置

```yaml
switcher:
  caption: 〔方案選單〕
  hotkeys:
    - Control+grave          # Ctrl+`
    - Control+Shift+grave
    - F4
  save_options:
    - full_shape
    - ascii_punct
    - simplification
  fold_options: true
  abbreviate_options: true
```

### 功能

- 切換輸入方案
- 切換選項（全形/半形、簡繁等）
- 保存使用者偏好

---

## 設計模式總結

| 模式 | 應用 |
|------|------|
| Pipeline | Processor → Segmentor → Translator → Filter |
| Strategy | Poet 的比較策略 |
| Factory | Component 創建 |
| Observer | Context 更新通知 |
| Visitor | Filter 遍歷候選 |
| Flyweight | 共享 Dictionary 實例 |

---

## 與 TaigiKeyboard 的比較

| 功能 | librime | TaigiKeyboard |
|------|---------|---------------|
| **簡繁轉換** | OpenCC 整合 | 無 |
| **反查** | 內建支援 | 無 |
| **標點處理** | 配置驅動 | 硬編碼 |
| **並擊輸入** | 內建支援 | 無 |
| **造句** | Poet + Grammar | 無（單詞） |
| **歷史記錄** | 內建支援 | 無 |

### 值得借鑑

1. **反查功能**：漢字查羅馬字對台語學習有幫助
2. **標點配置化**：用配置定義標點行為
3. **歷史記錄**：提升常用詞輸入效率
4. **模式識別**：識別 URL、Email 等特殊輸入

---

## 相關檔案

- `src/rime/gear/simplifier.{h,cc}` - OpenCC 整合
- `src/rime/gear/reverse_lookup_translator.{h,cc}` - 反查
- `src/rime/gear/punctuator.{h,cc}` - 標點處理
- `src/rime/gear/chord_composer.{h,cc}` - 並擊輸入
- `src/rime/gear/editor.{h,cc}` - 編輯區
- `src/rime/gear/poet.{h,cc}` - 造句系統
- `src/rime/gear/key_binder.{h,cc}` - 按鍵綁定
- `src/rime/gear/recognizer.{h,cc}` - 模式識別
- `src/rime/gear/switcher.{h,cc}` - 方案切換
