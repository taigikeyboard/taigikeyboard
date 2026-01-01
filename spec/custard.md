# CustardKit 參考研究

> **類型**: 參考
> **關鍵字**: `CustardKit`, `Flick`, `azooKey`, `自訂鍵盤`
> **相關**: azookey-reference.md, flick.md

---

## 重點摘要

- azooKey 的自訂鍵盤資料格式定義庫
- 支援 Swift 程式碼生成與 JSON 格式匯出
- 提供 Flick 與 QWERTY 兩種鍵盤樣式
- 豐富的動作類型（輸入、刪除、移動、切換等）

---

## 專案結構

```
CustardKit/
├── swift/
│   ├── sources/           # Swift 核心實作
│   │   ├── CustardKit.swift           # 主要資料結構
│   │   ├── CustardActions.swift       # 動作類型定義
│   │   ├── CustardKeyLabelStyle.swift # 標籤樣式
│   │   ├── CustardInterfaceVariation.swift # 變化定義
│   │   └── FlickDirection.swift       # 滑動方向
│   ├── examples/          # 範例鍵盤
│   └── tests/             # 單元測試
├── json/                  # JSON 格式文件
└── resource/              # 截圖資源
```

---

## 核心資料結構

### Custard（頂層結構）

```swift
struct Custard {
    var identifier: String           // 唯一識別碼
    var language: CustardLanguage    // 轉換語言
    var input_style: CustardInputStyle // 輸入方式
    var metadata: CustardMetadata    // 元資料
    var interface: CustardInterface  // 介面定義
}
```

### CustardInterface（介面定義）

```swift
struct CustardInterface {
    var keyStyle: CustardInterfaceStyle   // 鍵盤樣式
    var keyLayout: CustardInterfaceLayout // 佈局
    var keys: [CustardKeyPositionSpecifier: CustardInterfaceKey] // 按鍵
}
```

### CustardInterfaceKey（按鍵類型）

```swift
enum CustardInterfaceKey {
    case system(CustardInterfaceSystemKey) // 系統按鍵
    case custom(CustardInterfaceCustomKey) // 自訂按鍵
}
```

---

## 鍵盤樣式

| 樣式 | 說明 | 用途 |
|------|------|------|
| `tenkeyStyle` | Flick 滑動輸入 | 日文/台語 Flick |
| `pcStyle` | 長按變化選擇 | QWERTY 鍵盤 |

---

## 佈局類型

### gridFit（固定網格）

```swift
CustardInterfaceLayout.gridFit(.init(rowCount: 5, columnCount: 4))
```

- 畫面固定大小
- 按鍵均等分配
- 適合 Flick 鍵盤

### gridScroll（滾動網格）

```swift
CustardInterfaceLayout.gridScroll(.init(
    direction: .horizontal,
    rowCount: 3.0,
    columnCount: 8.0
))
```

- 可滾動超出畫面
- 適合符號/表情選擇器

---

## 位置指定

### GridFitPositionSpecifier

```swift
GridFitPositionSpecifier(x: 1, y: 2, width: 1, height: 2)
```

| 參數 | 說明 |
|------|------|
| x | 水平位置（左=0） |
| y | 垂直位置（上=0） |
| width | 寬度（預設 1） |
| height | 高度（預設 1） |

---

## 系統按鍵

```swift
enum CustardInterfaceSystemKey {
    case changeKeyboard    // 🌐 切換輸入法
    case enter             // Enter
    case upperLower        // 大小寫切換
    case nextCandidate     // 下一個候選詞
    // Flick 專用
    case flickKogaki       // 小ﾞﾟ
    case flickKutoten      // ､｡!?
    case flickHiraTab      // 平假名 Tab
    case flickAbcTab       // ABC Tab
    case flickStar123Tab   // ☆123 Tab
}
```

---

## 自訂按鍵

### CustardInterfaceCustomKey

```swift
struct CustardInterfaceCustomKey {
    var design: CustardKeyDesign              // 外觀設計
    var press_actions: [CodableActionData]    // 點擊動作
    var longpress_actions: CodableLongpressActionData // 長按動作
    var variations: [CustardInterfaceVariation] // 變化（Flick/長按）
}
```

### 快速建構 Flick 輸入

```swift
// 方式 1：陣列
.flickSimpleInputs(center: "1", subs: ["☆", "♪", "→", "←"])

// 方式 2：指定方向
.flickSimpleInputs(
    center: "a",
    left: "á",
    top: "à",
    right: "â",
    bottom: "ā"
)
```

---

## 標籤樣式

```swift
enum CustardKeyLabelStyle {
    case text(String)           // 純文字
    case systemImage(String)    // SF Symbol
    case mainAndSub(String, String)  // 主字 + 副字
    case mainAndDirections(String, CustardKeyDirectionalLabel) // 主字 + 四向
}
```

### mainAndSub（垂直副字）

```
┌─────────┐
│    1    │  ← 主字（大）
│   ☆♪→   │  ← 副字（小）
└─────────┘
```

### mainAndDirections（四向提示）

```
┌─────────┐
│    á    │  ← 上
│ à  a  â │  ← 左 中心 右
│    ā    │  ← 下
└─────────┘
```

---

## 變化類型

```swift
enum VariationType {
    case flickVariation(FlickDirection) // Flick 滑動變化
    case longpressVariation             // 長按選擇變化
}
```

### FlickDirection

```swift
enum FlickDirection: String {
    case left, top, right, bottom
}
```

---

## 按鍵顏色

```swift
enum ColorType: String {
    case normal       // 一般按鍵
    case special      // 特殊功能鍵
    case selected     // 選中狀態
    case unimportant  // 次要按鍵
}
```

---

## 動作類型

### 基本動作

| 動作 | 說明 |
|------|------|
| `.input(String)` | 輸入文字 |
| `.delete(Int)` | 刪除字元 |
| `.moveCursor(Int)` | 移動游標 |
| `.moveTab(TabData)` | 切換 Tab |

### Tab 切換

```swift
enum TabData {
    case system(SystemTab)  // 系統 Tab
    case custom(String)     // 自訂 Tab（by identifier）
}

enum SystemTab {
    case user_japanese, user_english
    case flick_japanese, flick_english, flick_numbersymbols
    case qwerty_japanese, qwerty_english, qwerty_numbers, qwerty_symbols
    case last_tab, clipboard_history_tab, emoji_tab
}
```

### 進階動作

| 動作 | 說明 |
|------|------|
| `.replaceLastCharacters([String: String])` | 替換最後字元 |
| `.toggleCapsLockState` | 切換大小寫 |
| `.smartDeleteDefault` | 智慧刪除 |
| `.toggleCursorBar` | 切換游標列 |

---

## 長按動作

```swift
struct CodableLongpressActionData {
    var start: [CodableActionData]   // 開始時執行
    var `repeat`: [CodableActionData] // 重複執行
}
```

範例：

```swift
// 長按刪除（重複）
.init(repeat: [.delete(1)])

// 長按顯示游標列（開始時）
.init(start: [.toggleCursorBar])
```

---

## 完整範例

### 希臘文 Flick 鍵盤

```swift
Custard(
    identifier: "flick_greek",
    language: .el_GR,
    input_style: .direct,
    metadata: .init(custard_version: .v1_0, display_name: "希臘語"),
    interface: .init(
        keyStyle: .tenkeyStyle,
        keyLayout: .gridFit(.init(rowCount: 5, columnCount: 4)),
        keys: [
            // 系統按鍵
            .gridFit(.init(x: 0, y: 0)): .system(.flickStar123Tab),
            .gridFit(.init(x: 0, y: 3)): .system(.changeKeyboard),
            .gridFit(.init(x: 4, y: 0)): .custom(.flickDelete()),
            .gridFit(.init(x: 4, y: 1)): .custom(.flickSpace()),
            .gridFit(.init(x: 4, y: 2, height: 2)): .system(.enter),

            // 字母按鍵
            .gridFit(.init(x: 2, y: 0)): .custom(
                .flickSimpleInputs(center: "α", subs: ["β", "γ"], centerLabel: "αβγ")
            ),
            .gridFit(.init(x: 3, y: 0)): .custom(
                .flickSimpleInputs(center: "δ", subs: ["ε", "ζ"], centerLabel: "δεζ")
            ),
            // ...
        ]
    )
)
```

---

## 台語鍵盤對照

| CustardKit 概念 | 台語鍵盤實作 |
|----------------|-------------|
| `Custard` | `FlickLayout` |
| `CustardInterfaceCustomKey` | `FlickKeyDef` |
| `CustardInterfaceSystemKey` | `FlickSystemKey` |
| `CustardKeyPositionSpecifier` | `FlickKeyPosition` |
| `CustardKeyLabelStyle` | `FlickLabelStyle` |
| `flickSimpleInputs` | `vowelTone` / `consonantGroup` |
| `VariationType.flickVariation` | `FlickDirection` |

---

## 設計啟發

### 值得參考

| CustardKit 設計 | 應用 |
|----------------|------|
| 便捷建構方法 `flickSimpleInputs` | 簡化佈局定義 |
| 標籤樣式分離（mainAndSub/mainAndDirections） | 不同按鍵類型渲染 |
| 位置+尺寸一體指定 | 跨行/跨列按鍵 |
| 系統按鍵獨立 enum | 減少重複定義 |

### 差異

| 差異點 | CustardKit | 台語鍵盤 |
|--------|-----------|---------|
| 聲調處理 | 無 | 長按第 8 聲 |
| 佈局複雜度 | 高（完全自訂） | 低（預設佈局） |
| JSON 匯出 | 支援 | 不需要 |
