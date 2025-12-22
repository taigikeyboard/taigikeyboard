# azooKey 主 APP UI/UX 分析

參考來源：`references/azookey`

此文件記錄 azooKey 主 APP（非鍵盤）的 UI/UX 實作，供未來開發台語鍵盤主 APP 時參考。

---

## 1. 檔案結構

```
MainApp/
├── ContentView.swift                 # 主要入口，TabView 容器
├── MainApp.swift                     # App 根結構
├── Customize/                        # 自訂功能區域
│   ├── CustomizeTabView.swift       # 自訂 Tab
│   ├── ManageCustardView.swift      # 管理自訂標籤
│   ├── EditingTabBarView.swift      # 編輯標籤欄
│   ├── EditingGridFitCustardView.swift
│   └── EditingScrollCustardView.swift
├── Theme/                           # 佈景主題區域
│   ├── ThemeTab.swift              # 佈景主題 Tab
│   ├── ThemeEditView.swift         # 編輯佈景
│   └── ThemeShareView.swift        # 分享佈景
├── Setting/                         # 設定區域
│   ├── SettingTab.swift            # 設定 Tab
│   ├── BooleanSetting/             # 布林設定組件
│   ├── KeyboardLayout/             # 鍵盤配置設定
│   ├── FontSizeSetting/            # 字體大小設定
│   └── [多個其他設定模塊]
├── Tips/                           # 教學和說明區域
│   ├── TipsView.swift              # Tips Tab
│   ├── Articles/                   # 個別文章
│   └── News/                       # 新聞/通知項目
├── General/                        # 通用 UI 組件
│   ├── BottomSheetView.swift       # 底層工作表
│   ├── DraggableView.swift         # 可拖拽組件
│   ├── DisclosuringList.swift      # 可展開列表
│   ├── ImageSlideshowView.swift    # 圖片幻燈片
│   ├── KeyboardPreview.swift       # 鍵盤預覽
│   ├── IconNavigationLink.swift    # 圖標導航連結
│   ├── LargeButtonStyle.swift      # 大型按鈕樣式
│   ├── LiquidLabelStyle.swift      # 液體標籤樣式
│   ├── FallbackLink.swift          # 外部連結
│   ├── Focus.swift                 # 焦點效果修飾符
│   └── Pickers/                    # 選擇器
├── EnableAzooKeyView/              # Onboarding 入門視圖
├── InternalSetting/                # 內部設定管理
└── Utils/                          # 工具函數
```

---

## 2. 導航架構

### TabView 結構

ContentView 使用 enum-based TabView 管理 4 個主標籤：

| Tab | 圖標 | 功能 |
|-----|------|------|
| Tips | `lightbulb.fill` | 使用指南、FAQ |
| Theme | `photo` | 佈景主題管理 |
| Customize | `gearshape.2.fill` | 自訂功能 |
| Settings | `wrench.fill` | 設定選項 |

### NavigationStack 模式

各 Tab 內部使用 `NavigationStack` + `Enum Path` 實現類型安全的導航：

```swift
enum Path: Hashable {
    case edit(index: Int?)
    case information(String)
    case zenzaiSettings
}

@State private var path: [Path] = []

NavigationStack(path: $path) {
    // 內容
    .navigationDestination(for: Path.self) { destination in
        switch destination {
        case let .edit(index):
            ThemeEditView(index: index, manager: $manager)
        case let .information(identifier):
            InformationView(identifier: identifier)
        case .zenzaiSettings:
            ZenzaiSettingsView()
        }
    }
}
```

### Deep Link 支援

- Scheme: `azooKey://`
- 範例: `azooKey://settings/zenzai`
- 在 ContentView 的 `onOpenURL` 處理
- 通過 `appStates.deepLink` 狀態管理

---

## 3. 主要畫面功能

### 3.1 Tips Tab

- 使用 `NavigationStack` 進行導航
- 分為多個 Section：鍵盤啟用、新聞通知、常用技巧、故障排除
- 使用 `IconNavigationLink` 組件構建列表項目

### 3.2 Theme Tab

- 明暗模式分別設定主題
- `MatchedGeometryEffect` 實現選中動畫
- `KeyboardPreview` 縮放預覽（scale 0.6）
- Context Menu 支援編輯、刪除操作

### 3.3 Customize Tab

展示 3 個主要功能區域：
1. Custom Tab（自訂標籤）
2. Tab Bar（標籤欄）
3. Custom Keys（自訂按鍵）

使用 `ImageSlideshowView` 展示教學圖片。

### 3.4 Settings Tab

- **最複雜的 Tab**，包含 15+ 個 Section
- 使用 `searchable()` 支援搜尋功能
- 使用 `SearchQueriedView` 實現動態篩選
- 主要設定項目：
  - KeyboardLayout（Flick/Qwerty）
  - LiveConversion 設定
  - CustomKeys 設定
  - Sound & Haptics
  - User Dictionary
  - 開源授權、版本資訊

---

## 4. Onboarding 流程

`EnableAzooKeyView` 分 4 步驟：

| 步驟 | 說明 |
|------|------|
| `.menu` | 開始菜單 |
| `.append` | 添加鍵盤步驟 |
| `.setting` | 初始設定 |
| `.finish` | 完成（含實際輸入測試） |

特點：
- 支援「恢復進度」
- 最後一步有 TextField 讓用戶實際測試鍵盤

---

## 5. 狀態管理

### App 級別狀態

```swift
@StateObject private var appStates = MainAppStates()

class MainAppStates: ObservableObject {
    @Published var isKeyboardActivated: Bool
    @Published var requireFirstOpenView: Bool
    @Published var custardManager: CustardManager
    @Published var internalSettingManager: InternalSettingManager
    @Published var requestReviewManager: RequestReviewManager
}
```

### 環境值使用

```swift
@Environment(\.requestReview)      // StoreKit review
@Environment(\.colorScheme)        // 暗黑模式偵測
@EnvironmentObject var appStates   // 跨層級傳遞
```

---

## 6. 可重用 UI 組件

### IconNavigationLink

帶圖標的導航連結，用於列表項目。

### ImageSlideshowView

使用 `TimelineView` 實現自動輪轉：

```swift
struct ImageSlideshowView: View {
    var body: some View {
        TimelineView(.periodic(from: .now, by: 2.5)) { context in
            let selection = Int(context.date.timeIntervalSince1970 / 2.5) % pictures.count
            Image(pictures[selection])
        }
    }
}
```

### DraggableView

通用拖拽重排組件：
- 支援拖拽、選擇、重新排序
- 視覺反饋（分隔符、陰影）
- 用於標籤欄編輯、佈景主題排序

### KeyboardPreview

鍵盤預覽組件，可縮放顯示。

### DisclosuringList

可展開/收合的列表組件。

### LiquidLabelStyle

毛玻璃效果標籤樣式：

```swift
struct LiquidLabelStyle: LabelStyle {
    func makeBody(configuration: Configuration) -> some View {
        Label(...)
            .padding(8)
            .background(Capsule().foregroundStyle(.regularMaterial))
    }
}
```

---

## 7. SwiftUI 技巧

### 自訂 ViewModifier

```swift
// 搜尋篩選
extension View {
    @ViewBuilder func searchKeys(_ keys: String...) -> some View { }
    func inheritSearchKeys() -> some View { }
}

// 焦點效果
extension View {
    func focus(_ color: Color = .accentColor, focused: Bool) -> some View { }
}

// 背景事件
extension View {
    func onEnterBackground(perform action: ...) -> some View { }
    func onEnterForeground(perform action: ...) -> some View { }
}
```

### MatchedGeometryEffect 動畫

```swift
@Namespace private var namespace

// 選中標記動畫
.matchedGeometryEffect(id: "selected_theme_checkmark", in: namespace)
.animation(.easeIn(duration: 0.15), value: manager.selectedIndex)
```

### 條件式 UI

```swift
// 根據選項數量自適應 UI
if types.count > 3 {
    // Picker 模式
} else {
    // Segmented Picker + Preview 模式
}

// 功能可用性檢查
if SemiStaticStates.shared.hasFullAccess {
    // 顯示需要 Full Access 的功能
}
```

---

## 8. UI/UX 設計決策

### 分層次設定組織

- Settings Tab 按功能分成 15+ 邏輯區域
- 相關設定分組在同一 Section
- 搜尋功能實現「扁平化」存取

### 漸進式資訊披露

- Tips Tab 使用多層 NavigationLink
- 複雜功能先顯示簡介，再提供詳細編輯
- DisclosuringList 讓用戶決定何時展開

### 即時預覽

- KeyboardPreview 展示實際效果
- MatchedGeometryEffect 提供視覺連貫性
- 支援明暗模式分別設定

### 確認與反饋

- Toggle 帶警告 Alert（需要 Full Access 時）
- Context Menu 用於破壞性操作
- Progress View 用於長時間操作

### 檔案導入整合

```swift
.onOpenURL { url in
    if url.scheme == "azooKey" {
        // Deep Link 處理
    } else {
        // 檔案導入
        importFileURL = url
    }
}
```

---

## 9. 設計模式總結

| 模式 | 實現 | 用途 |
|------|------|------|
| State Management | @StateObject + @EnvironmentObject | 跨層級狀態共享 |
| Enum-based Routing | Path: Hashable Enum + NavigationStack | 類型安全導航 |
| Composition | ViewBuilder + generic constraints | 可重用組件 |
| Preference Keys | SearchKeysPreferenceKey | 自訂搜尋系統 |
| Binding Patterns | Binding<T> + mutating 函數 | 雙向數據流 |
| Protocol-based Settings | BoolKeyboardSettingKey | 通用設定框架 |
| Adaptive UI | 條件式 Layout 選擇 | 響應式設計 |

---

## 10. 未來參考重點

1. **TabView + NavigationStack** 架構可直接採用
2. **Onboarding 流程** 需根據台語鍵盤特性調整
3. **Settings 搜尋功能** 提升用戶體驗
4. **通用組件**（IconNavigationLink、DraggableView）可移植使用
5. **Deep Link** 支援可延後實作
6. **主題編輯** 的即時預覽設計值得參考
