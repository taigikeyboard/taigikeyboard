# Flick 聲調鍵盤

> **類型**: 功能
> **關鍵字**: `Flick`, `FlickKeyDef`, `FlickLayout`, `FlickTab`, `聲調輸入`
> **相關**: layout.md, tone.md, azookey-reference.md, custard.md

---

## 重點摘要

- 滑動輸入聲調，解決台語輸入最大痛點
- 4列×6欄 網格佈局（參考 azooKey），母音 Flick 聲調、子音 Flick 分組
- 內建數字/符號鍵盤與 ABC 鍵盤，Tab 切換不離開 Flick 模式
- 翻譯按鍵支援漢字/羅馬字切換

---

## 檔案結構

| 檔案 | 說明 |
|------|------|
| `FlickModels.swift` | Flick 資料模型（方向、按鍵、佈局） |
| `FlickTaigiLayout.swift` | 台語 Flick 佈局定義（TL/POJ） |
| `FlickKeyView.swift` | 單一 Flick 按鍵視圖 |
| `FlickKeyboardView.swift` | Flick 鍵盤網格 |
| `TaigiFlickKeyboardView.swift` | 整合候選詞的主視圖 |

---

## 聲調對應

| 操作 | 聲調 | 範例 |
|------|------|------|
| 點擊 | 第1聲（陰平） | a |
| ← 左滑 | 第2聲（陰上） | á |
| ↑ 上滑 | 第3聲（陰去） | à |
| → 右滑 | 第5聲（陽平） | â |
| ↓ 下滑 | 第7聲（陽去） | ā |
| 長按 | 第8聲（陽入） | a̍ |

> 第4聲由韻尾 -p/-t/-k/-h 決定，不需調號

---

## 佈局設計

### 座標系統（參考 azooKey CustardKit）

- **4 列 (y=0-3) × 6 欄 (x=0-5)**
- x: 水平欄位（0=左 → 5=右）
- y: 垂直列位（0=上 → 3=下）

### 按鍵分區

| 欄位 | 用途 | 說明 |
|------|------|------|
| x=0 | Tab 切換欄 | ☆123, ABC, 譯/台語, 🌐 |
| x=1-4 | 字符輸入區 | 16 格輸入按鍵 |
| x=5 | 系統功能欄 | ⌫, 空白, 改行(跨2列) |

### 台語主鍵盤（4列×6欄）

```
         x=0       x=1      x=2      x=3      x=4      x=5
y=0      ☆123      a        e        i        o        ⌫
y=1      ABC       oo       u        p        k        空白
y=2      譯        ts       n       nn，。   「」『』   改行
y=3      🌐       ，。     ？！     《》〈〉   —⋯       ┘
```

### 數字符號鍵盤（4列×6欄）

```
         x=0       x=1      x=2      x=3      x=4      x=5
y=0      ☆123      1        2        3        4        ⌫
y=1      ABC       5        6        7        8        空白
y=2      台語      9        0       ()[]     .,-/      改行
y=3      🌐       ¥$€      %°#     +-×÷      <=>       ┘
```

### ABC 鍵盤（4列×6欄）

```
         x=0       x=1      x=2      x=3      x=4      x=5
y=0      ☆123     @#/&_    ABC      DEF       GHI      ⌫
y=1      abc       JKL      MNO     PQRS      TUV      空白
y=2      台語     WXYZ     '\"()    .,?!      a/A      改行
y=3      🌐       0-9      +-=      *&^      _|\\      ┘
```

---

## Tab 切換

### 第一欄 (x=0) 系統按鍵

| 位置 | 台語鍵盤 | 數字鍵盤 | ABC 鍵盤 |
|------|----------|----------|----------|
| y=0 | ☆123 | ☆123(選中) | ☆123 |
| y=1 | ABC | ABC | abc(選中) |
| y=2 | 譯 | 台語 | 台語 |
| y=3 | 🌐 | 🌐 | 🌐 |

### 切換說明

| 按鍵 | 功能 | 位置 |
|------|------|------|
| ☆123 | 切換到數字符號 | (x=0, y=0) |
| ABC/abc | 切換到 ABC | (x=0, y=1) |
| 譯 | 漢字/羅馬字切換 | 台語鍵盤 (x=0, y=2) |
| 台語 | 切換到台語主鍵盤 | 數字/ABC 鍵盤 (x=0, y=2) |
| a/A | 大小寫切換 | ABC 鍵盤 (x=4, y=2) |
| 🌐 | 切換輸入法 | (x=0, y=3) |

> Tab 切換在 Flick 模式內進行，不會跳回 QWERTY

---

## 按鍵類型

### 母音按鍵（聲調 Flick）

| 按鍵 | 中心 | ←左 | ↑上 | →右 | ↓下 | 長按 |
|------|------|------|------|------|------|------|
| a | a | á | à | â | ā | a̍ |
| e | e | é | è | ê | ē | e̍ |
| i | i | í | ì | î | ī | i̍ |
| o | o | ó | ò | ô | ō | o̍ |
| oo | oo | óo | òo | ôo | ōo | o̍o |
| u | u | ú | ù | û | ū | u̍ |

### 子音按鍵（分組 Flick）

| 按鍵 | 中心 | ←左 | ↑上 | →右 | ↓下 |
|------|------|------|------|------|------|
| p | p | ph | b | m | - |
| k | k | kh | g | ng | h |
| ts | ts | tsh | s | j | l |
| n | n | th | t | - | - |

### 特殊按鍵

| 按鍵 | 中心 | ←左 | ↑上 | →右 | ↓下 | 說明 |
|------|------|------|------|------|------|------|
| nn，。 | nn | ， | 。 | ？ | ！ | 鼻化+標點 |
| 「」『』 | 「 | 」 | 『 | 』 | - | 引號括號 |
| ，。 | ， | 。 | 、 | ； | ： | 逗句標點 |
| ？！ | ？ | ！ | ～ | ‥ | … | 問號驚嘆 |
| 《》 | 《 | 》 | 〈 | 〉 | 【 | 書名號 |
| —⋯ | — | ⋯ | － | ＿ | - | 破折省略 |

### 數字按鍵

| 按鍵 | 中心 | ←左 | ↑上 | →右 | ↓下 |
|------|------|------|------|------|------|
| 1 | 1 | ① | ⑴ | ⒈ | - |
| 2 | 2 | ② | ⑵ | ⒉ | - |
| 3 | 3 | ③ | ⑶ | ⒊ | - |
| 4 | 4 | ④ | ⑷ | ⒋ | - |
| 5 | 5 | ⑤ | ⑸ | ⒌ | - |
| 6 | 6 | ⑥ | ⑹ | ⒍ | - |
| 7 | 7 | ⑦ | ⑺ | ⒎ | - |
| 8 | 8 | ⑧ | ⑻ | ⒏ | - |
| 9 | 9 | ⑨ | ⑼ | ⒐ | - |
| 0 | 0 | ⓪ | ⑽ | ⒑ | - |
| ()[] | ( | ) | [ | ] | {} |
| .,-/ | . | , | - | / | - |

### 數字符號區按鍵

| 按鍵 | 中心 | ←左 | ↑上 | →右 | ↓下 |
|------|------|------|------|------|------|
| ¥$€ | ¥ | $ | € | £ | ₩ |
| %°# | % | ° | # | ‰ | ℃ |
| +-×÷ | + | - | × | ÷ | ± |
| <=> | < | = | > | ≤ | ≥ |

### ABC 按鍵

| 按鍵 | 中心 | ←左 | ↑上 | →右 | ↓下 |
|------|------|------|------|------|------|
| @#/&_ | @ | # | / | & | _ |
| ABC | a | b | c | - | - |
| DEF | d | e | f | - | - |
| GHI | g | h | i | - | - |
| JKL | j | k | l | - | - |
| MNO | m | n | o | - | - |
| PQRS | p | q | r | s | - |
| TUV | t | u | v | - | - |
| WXYZ | w | x | y | z | - |
| '\"() | ' | \" | ( | ) | - |
| .,?! | . | , | ? | ! | - |

### ABC 符號區按鍵

| 按鍵 | 中心 | ←左 | ↑上 | →右 | ↓下 |
|------|------|------|------|------|------|
| 0-9 | 0 | 1 | 2 | 3 | 4 |
| +-= | + | - | = | * | - |
| *&^ | * | & | ^ | ~ | - |
| _\|\\ | _ | \| | \\ | \` | - |

---

## 標籤顯示樣式

| 樣式 | 用途 | 顯示方式 |
|------|------|----------|
| `directional` | 母音/子音 | 四向提示在邊緣 |
| `verticalSub` | 數字/符號 | 副字在主字下方 |

### directional 範例（母音）

```
┌─────────┐
│    à    │  ← 上（第3聲）
│ á  a  â │  ← 左/中心/右
│    ā    │  ← 下（第7聲）
└─────────┘
```

### verticalSub 範例（數字）

```
┌─────────┐
│    1    │  ← 主字（大）
│   、⋯—   │  ← 副字（小）
└─────────┘
```

---

## 資料模型

### FlickTab

```swift
enum FlickTab {
    case taigi   // 台語主鍵盤
    case number  // 數字符號
    case abc     // ABC 英文
}
```

### FlickDirection

```swift
enum FlickDirection {
    case left   // ← 第2聲
    case top    // ↑ 第3聲
    case right  // → 第5聲
    case bottom // ↓ 第7聲
}
```

### FlickLabelStyle

```swift
enum FlickLabelStyle {
    case directional  // 四向提示模式（母音/子音）
    case verticalSub  // 垂直副字模式（數字/符號）
}
```

### FlickSystemKey

```swift
enum FlickSystemKey {
    case delete, space, enter, globe
    case switchToQwerty
    case translate           // 漢字/羅馬字切換
    // Flick 內部 Tab 切換
    case flickTabTaigi
    case flickTabNumber
    case flickTabAbc
    case flickTabShift
}
```

### FlickKeyDef

```swift
enum FlickKeyDef {
    case vowel(base: String, tones: [FlickDirection: String], tone8: String)
    case consonant(center: String, label: String, variations: [FlickDirection: String])
    case symbol(center: String, label: String, variations: [FlickDirection: String])
    case system(FlickSystemKey)
}
```

### FlickKeyPosition

```swift
struct FlickKeyPosition: Hashable {
    let row: Int     // 列（y=0-3）
    let column: Int  // 欄（x=0-5）
    var width: Int   // 寬度（預設 1）
    var height: Int  // 高度（預設 1，Enter 為 2）
}
```

---

## 設定整合

### KeyboardLayoutType

```swift
enum KeyboardLayoutType: String {
    case phahTaigi  // PhahTaigi 佈局
    case qwerty     // 標準 QWERTY
    case flick      // Flick 聲調
}
```

### 切換流程

1. 主 App Tab2 選擇佈局
2. `SharedSettings.keyboardLayoutType` 儲存設定
3. `KeyboardViewController` 根據設定載入對應視圖

---

## POJ 模式差異

| TL | POJ |
|-----|-----|
| oo | o͘ |
| ts | ch |
| tsh | chh |

---

## 可用佈局

| 識別碼 | 說明 | 網格 |
|--------|------|------|
| `taigiTone` | 台語聲調（TL 模式） | 4列×6欄 |
| `taigiTonePOJ` | 台語聲調（POJ 模式） | 4列×6欄 |
| `flickNumber` | 數字符號 | 4列×6欄 |
| `flickAbc` | ABC 小寫 | 4列×6欄 |
| `flickAbcUpper` | ABC 大寫 | 4列×6欄 |

---

## 平台對照

| 項目 | iOS | Android |
|------|-----|---------|
| 佈局定義 | `FlickTaigiLayout.swift` | 待實作 |
| 按鍵視圖 | `FlickKeyView.swift` | 待實作 |
| 鍵盤視圖 | `TaigiFlickKeyboardView.swift` | 待實作 |
| 樣式定義 | `FlickKeyStyle.swift` | 待實作 |
