# azooKey 參考研究

> **類型**: 參考
> **關鍵字**: `azooKey`, `SwiftUI`, `Flick`, `CustardKit`
> **相關**: rime-reference.md, khiin-reference.md

---

## 重點摘要

- iOS 日語鍵盤，採用 SwiftUI 實作
- 雙軌佈局系統：CustardKit（Flick）+ UnifiedKey（QWERTY）
- 47 種動作類型的統一介面
- 主 App 採用 TabView + NavigationStack 架構

---

## 鍵盤架構

### 雙軌佈局系統

| 系統 | 適用 | 坐標 |
|------|------|------|
| CustardKit | Flick 鍵盤 | 整數 gridFit (x, y) |
| UnifiedKey | QWERTY | 浮點數 + 寬度 |

### CustardKit 範例

```swift
Custard(
    identifier: "japanese_flick",
    interface: CustardInterface(
        keyLayout: .gridFit(.init(rowCount: 5, columnCount: 4)),
        keys: [
            .gridFit(.init(x: 1, y: 1)): .custom(
                .flickSimpleInputs(
                    center: "あ", left: "い", top: "う",
                    right: "え", bottom: "お"
                )
            )
        ]
    )
)
```

### 按鍵模型協議

```swift
protocol UnifiedKeyModelProtocol {
    func pressActions(variableStates) -> [ActionType]
    func longPressActions(variableStates) -> LongpressActionType
    func variationSpace(variableStates) -> UnifiedVariationSpace
}
```

---

## 動作類型

| 分類 | 動作 | 用途 |
|------|------|------|
| 文字輸入 | `.input(String)` | 輸入文字 |
| 刪除 | `.delete(Int)` | 刪除字元 |
| 光標 | `.moveCursor(Int)` | 移動游標 |
| 頁籤 | `.moveTab(TabData)` | 切換鍵盤 |
| 狀態 | `.setBoolState(String, BoolOperation)` | Caps Lock |

### 變化空間

```swift
enum UnifiedVariationSpace {
    case none
    case fourWay([FlickDirection: UnifiedVariation])  // Flick 四向
    case linear([VariationElement], direction: ...)   // 長按彈出
}
```

---

## 主 App 架構

### TabView 結構

| Tab | 圖標 | 功能 |
|-----|------|------|
| Tips | `lightbulb.fill` | 使用指南 |
| Theme | `photo` | 佈景主題 |
| Customize | `gearshape.2.fill` | 自訂功能 |
| Settings | `wrench.fill` | 設定選項 |

### NavigationStack 模式

```swift
enum Path: Hashable {
    case edit(index: Int?)
    case information(String)
}

NavigationStack(path: $path) {
    .navigationDestination(for: Path.self) { destination in
        switch destination { ... }
    }
}
```

### Onboarding 流程

| 步驟 | 說明 |
|------|------|
| menu | 開始菜單 |
| append | 添加鍵盤 |
| setting | 初始設定 |
| finish | 完成測試 |

---

## 可重用組件

| 組件 | 用途 |
|------|------|
| IconNavigationLink | 帶圖標的導航連結 |
| ImageSlideshowView | 自動輪轉圖片 |
| DraggableView | 拖拽重排 |
| KeyboardPreview | 鍵盤預覽 |
| DisclosuringList | 可展開列表 |

---

## 台語鍵盤應用建議

### 值得參考

| azooKey 設計 | 台語鍵盤應用 |
|-------------|-------------|
| Flick 五向輸入 | 聲調變化（a → á/à/â/ā/a̍）|
| UnifiedKeyModelProtocol | 統一按鍵介面 |
| 狀態驅動渲染 | POJ/TL 切換 |
| TabView + NavigationStack | 主 App 架構 |
| Onboarding 流程 | 首次啟動引導 |
| 搜尋設定功能 | Settings Tab |

### 不需要實作

| azooKey 設計 | 理由 |
|-------------|------|
| CustardKit 自訂佈局 | 台語輸入需求固定 |
| 佈局編輯器 UI | 開發成本高 |
| 雙軌佈局系統 | 先專注 QWERTY |

---

## SwiftUI 技巧

### MatchedGeometryEffect 動畫

```swift
@Namespace private var namespace

.matchedGeometryEffect(id: "checkmark", in: namespace)
.animation(.easeIn(duration: 0.15), value: selectedIndex)
```

### TimelineView 輪播

```swift
TimelineView(.periodic(from: .now, by: 2.5)) { context in
    let index = Int(context.date.timeIntervalSince1970 / 2.5) % count
    Image(pictures[index])
}
```

### 自訂 ViewModifier

```swift
extension View {
    func focus(_ color: Color, focused: Bool) -> some View { }
    func onEnterBackground(perform: ...) -> some View { }
}
```

---

## 設計模式

| 模式 | 實現 |
|------|------|
| State Management | @StateObject + @EnvironmentObject |
| Enum-based Routing | Path Enum + NavigationStack |
| Composition | ViewBuilder + generic |
| Protocol-based Settings | BoolKeyboardSettingKey |

---

## azooKey 原始碼參考

| 目錄 | 內容 |
|------|------|
| `KeyboardViews/` | 鍵盤 UI 實作 |
| `KeyboardViews/View/UnifiedKey/` | 統一按鍵系統 |
| `KeyboardViews/Custard/` | 內建 Flick 佈局 |
| `MainApp/` | 主應用 UI |
| `MainApp/Setting/` | 設定功能 |
