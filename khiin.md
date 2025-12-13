# khiin-rs 參考筆記

## 聲調轉換

khiin-rs 有類似 ToneConverter 的功能，在 `syllable.rs` 的 `compose()` 函數：
- 將數字聲調轉換成聲調符號（如 `ho2` → `hó`）
- 組字區永遠顯示帶聲調符號，無論是否有匹配的候選詞

## convert_guess 機制

**不是「猜候選詞」，而是「猜音節分割」**

### 流程

```
輸入: "goalaite2"
  ↓
parse_whole_input() - 解析輸入
  ↓
can_segment_max() - DP 找出可分割的最長前綴
  ↓
segment() - 最小成本演算法分割成音節
  ↓
結果: ["goa", "lai", "te2"] → "goá-lâi-tè"
```

### 核心：動態規劃分割

- 分割依據是「子字串是否為合法台語音節」
- 成本計算：`COST = ln(1 / P)`，P = 詞頻 / 總詞數
- 選擇成本最低的分割方式

### 詞庫量少的影響

| 情境 | 影響 |
|------|------|
| 音節表完整 | 分割正確 |
| 音節表缺少 | 可能分割錯誤或標記為 Plaintext |
| 漢字詞庫少 | 不影響分割，只是沒有漢字候選詞 |

## 與台語鍵盤的差異

| 項目 | khiin-rs | 台語鍵盤 |
|------|----------|----------|
| 分割依據 | 音節表 + DP | 無（單詞搜尋） |
| Fallback | 自動分割 + 帶調羅馬字 | 帶調組字文字 |
| 適用場景 | 連續輸入（無空格） | 單詞輸入（有空格） |

## 分隔符號處理

### autospace 邏輯

在 `buffer.rs` 的 `autospace()` 函數判斷是否加空格：

| 前一個 | 後一個 | 結果 |
|--------|--------|------|
| 漢字 | 漢字 | 不加空格 |
| 漢字 | 羅馬字 | 加空格 |
| 羅馬字 | 漢字 | 加空格 |
| 羅馬字 | 羅馬字 | 加空格 |

### 連字符號處理

連字符號 `-` 是在**資料庫建立時就寫入 output 欄位**，不是動態產生：

| 欄位 | 格式 |
|------|------|
| input | 空格分隔：`"goa beh chiah png"` |
| output（漢字） | 無分隔：`"我欲食飯"` |
| output（羅馬字） | 連字符號：`"góa-beh-chia̍h-pn̄g"` |

### 流程範例

```
使用者輸入: goabehchiahpng
  ↓
DP 分割: ["goa", "beh", "chiah", "png"]
  ↓
查詢 input: "goa beh chiah png"
  ↓
取得 output:
  - 漢字模式: "我欲食飯"
  - 羅馬字模式: "góa-beh-chia̍h-pn̄g"
```

## 相關檔案

- `khiin/src/input/syllable.rs` - 聲調轉換
- `khiin/src/input/converter.rs` - convert_guess
- `khiin/src/input/parser.rs` - 輸入解析
- `khiin/src/data/segmenter.rs` - DP 分割演算法
- `khiin/src/buffer/buffer_mgr.rs` - Preedit 構建
- `khiin/src/buffer/buffer.rs` - autospace 邏輯
- `khiin/src/db/models/key_conversion.rs` - 資料庫模型
