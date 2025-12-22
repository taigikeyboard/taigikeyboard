# 台語鍵盤主 APP 架構規劃

本文件規劃台語鍵盤主 APP 的未來架構，參考 azooKey 的設計模式，並結合台語鍵盤的特有需求。

---

## 1. 現況分析

### 目前已有功能

| 功能 | 檔案 | 狀態 |
|------|------|------|
| Onboarding 流程 | `OnboardingView.swift` | 完成 |
| 鍵盤設定 | `SettingsView.swift` | 完成 |
| 詞庫管理 | `DictionarySettingsView.swift` | 完成 |
| 首頁（卡片式） | `ContentView.swift` | 完成 |
| Deep Link | `taigikeyboard://settings` | 基本支援 |

### 目前架構特點

- 單頁卡片式首頁設計
- Sheet 呈現子頁面
- MVVM + Service 層
- App Groups 設定同步

### 待改進之處

1. 首頁資訊密度低，需多次點擊才能到達功能
2. 缺乏使用指南/教學內容
3. 未來功能（主題、自訂按鍵）無擴展空間
4. 無法快速存取常用設定

---

## 2. 目標架構

### 設計原則

1. **漸進式改進** - 不一次大改，分階段實施
2. **保留現有功能** - 現有 Settings、Dictionary 設定可重用
3. **台語特色優先** - 強調 POJ/TL、詞庫等台語特有功能
4. **簡潔為主** - 避免過度設計，符合 YAGNI

### 導航架構選項

#### 方案 A：保留單頁 + 強化（建議短期）

維持現有卡片式首頁，但：
- 將常用設定（輸入模式切換）直接放在首頁
- 增加「使用小技巧」區塊
- 優化視覺層次

```
ContentView (首頁)
├── 快捷設定區（POJ/TL 切換、字體選擇）
├── 功能卡片（設定、詞庫、教學）
├── 使用小技巧（可展開）
└── 資源連結
```

#### 方案 B：TabView 架構（建議中長期）

參考 azooKey，採用底部 Tab 導航：

```
TabView
├── Home Tab (首頁/狀態)
│   ├── 鍵盤啟用狀態
│   ├── 快捷設定
│   └── 最新消息
├── Learn Tab (學習/教學)
│   ├── 使用指南
│   ├── 台語輸入技巧
│   └── FAQ
├── Settings Tab (設定)
│   ├── 輸入設定
│   ├── 詞庫管理
│   ├── 外觀設定
│   └── 關於
└── [未來] Theme Tab (主題)
```

---

## 3. 建議實施計畫

### Phase 1：優化現有首頁

**目標**：不改變架構，提升使用體驗

**變更內容**：

1. **新增快捷設定區塊**
   - POJ/TL 一鍵切換（SegmentedPicker）
   - 字體快速選擇
   - 不需進入設定頁面即可調整

2. **新增使用技巧區塊**
   - 「你知道嗎？」每次顯示一個小技巧
   - 聲調輸入方式提示
   - 雙擊 OO/NN 功能介紹

3. **優化鍵盤狀態顯示**
   - 更明顯的啟用/未啟用狀態
   - Full Access 狀態提示

**預估影響範圍**：
- `ContentView.swift` - 新增區塊
- 新增 `QuickSettingsView.swift`
- 新增 `TipsCardView.swift`

### Phase 2：導入 TabView 架構

**目標**：為未來功能擴展建立基礎

**檔案結構**：

```
App/
├── TaigiKeyboardApp.swift          # App 入口（現有）
├── AppRootView.swift               # 根視圖（現有）
├── MainTabView.swift               # [新增] TabView 容器
├── Home/
│   ├── HomeTabView.swift           # [新增] 首頁 Tab
│   └── KeyboardStatusCard.swift    # [新增] 鍵盤狀態卡片
├── Learn/
│   ├── LearnTabView.swift          # [新增] 學習 Tab
│   ├── UserGuideView.swift         # [新增] 使用指南
│   └── TipsListView.swift          # [新增] 技巧列表
├── Settings/
│   ├── SettingsTabView.swift       # [重構] 設定 Tab
│   ├── InputSettingsView.swift     # [拆分自 SettingsView]
│   ├── DictionarySettingsView.swift # [現有]
│   ├── AppearanceSettingsView.swift # [新增] 外觀設定
│   └── AboutView.swift             # [新增] 關於頁面
├── Onboarding/                     # [現有] 維持不變
└── Components/                     # [新增] 可重用組件
    ├── IconNavigationLink.swift
    ├── SettingRow.swift
    └── StatusBadge.swift
```

**Tab 定義**：

```swift
enum MainTab: String, CaseIterable {
    case home = "首頁"
    case learn = "學習"
    case settings = "設定"

    var icon: String {
        switch self {
        case .home: return "house.fill"
        case .learn: return "book.fill"
        case .settings: return "gearshape.fill"
        }
    }
}
```

### Phase 3：進階功能（未來）

視需求逐步加入：

1. **主題系統**
   - Theme Tab
   - 自訂色彩
   - 鍵盤預覽

2. **自訂按鍵**
   - 常用詞快捷鍵
   - 自訂符號鍵

3. **使用者詞庫**
   - 匯入/匯出
   - 詞頻管理

---

## 4. 導航模式

### NavigationStack + Enum Path

參考 azooKey，使用類型安全的導航：

```swift
// SettingsTabView.swift
enum SettingsPath: Hashable {
    case input
    case dictionary
    case appearance
    case about
    case licenses
}

struct SettingsTabView: View {
    @State private var path: [SettingsPath] = []

    var body: some View {
        NavigationStack(path: $path) {
            List {
                // 設定列表
            }
            .navigationDestination(for: SettingsPath.self) { destination in
                switch destination {
                case .input:
                    InputSettingsView()
                case .dictionary:
                    DictionarySettingsView()
                case .appearance:
                    AppearanceSettingsView()
                case .about:
                    AboutView()
                case .licenses:
                    LicensesView()
                }
            }
        }
    }
}
```

### Deep Link 擴展

```swift
// 支援更多 Deep Link
enum DeepLink {
    case settings
    case settingsInput
    case settingsDictionary
    case learn
    case onboarding

    init?(url: URL) {
        guard url.scheme == "taigikeyboard" else { return nil }
        switch url.host {
        case "settings":
            if url.path == "/input" {
                self = .settingsInput
            } else if url.path == "/dictionary" {
                self = .settingsDictionary
            } else {
                self = .settings
            }
        case "learn":
            self = .learn
        case "onboarding":
            self = .onboarding
        default:
            return nil
        }
    }
}
```

---

## 5. 狀態管理

### App 級別狀態

```swift
@MainActor
class AppState: ObservableObject {
    @Published var isKeyboardEnabled: Bool = false
    @Published var hasFullAccess: Bool = false
    @Published var selectedTab: MainTab = .home
    @Published var showOnboarding: Bool = false

    // 現有 OnboardingViewModel 可整合至此

    func checkKeyboardStatus() {
        // 檢查鍵盤狀態
    }
}
```

### 設定管理

維持現有 `SharedSettings` 架構，但考慮：

```swift
// 未來可考慮使用 @Observable (iOS 17+)
@Observable
class SettingsManager {
    var inputMode: InputMode = .tl
    var fontType: FontType = .openHuninn
    // ...
}
```

---

## 6. UI 組件規劃

### 可重用組件

| 組件 | 用途 | 參考 |
|------|------|------|
| `IconNavigationLink` | 帶圖標的導航項目 | azooKey |
| `SettingRow` | 設定列表項目 | 現有 SettingsView |
| `StatusBadge` | 狀態標記（已啟用/未啟用） | - |
| `KeyboardPreview` | 鍵盤預覽（未來用於主題） | azooKey |
| `TipCard` | 使用技巧卡片 | - |

### 樣式系統

維持現有 Theme 系統：
- `TaigiTheme` - 色彩定義
- `TaigiTypography` - 字體樣式
- `TaigiAnimation` - 動畫參數

---

## 7. 學習/教學內容規劃

### Learn Tab 內容結構

```
學習 Tab
├── 快速入門
│   ├── 如何啟用鍵盤
│   ├── 基本輸入方式
│   └── 聲調輸入
├── 進階技巧
│   ├── 雙擊快捷輸入（OO/NN）
│   ├── 詞庫切換
│   └── 漢字與羅馬字並行輸出
├── 常見問題
│   ├── 為什麼需要完整存取權限？
│   ├── 如何切換 POJ/TL？
│   └── 詞庫說明
└── 台語學習資源（外部連結）
```

### 內容呈現方式

- 使用 Markdown 或靜態 View
- 支援圖片說明
- 可展開/收合的 FAQ 格式

---

## 8. 實施優先順序

| 優先順序 | 項目 | 複雜度 | 價值 |
|----------|------|--------|------|
| 1 | 首頁快捷設定區塊 | 低 | 高 |
| 2 | 使用技巧卡片 | 低 | 中 |
| 3 | 設定頁面分類整理 | 中 | 中 |
| 4 | 導入 TabView 架構 | 中 | 高 |
| 5 | Learn Tab 完整內容 | 中 | 中 |
| 6 | 主題系統 | 高 | 中 |
| 7 | 自訂按鍵 | 高 | 低 |

---

## 9. 與現有程式碼整合

### 可直接重用

- `OnboardingView.swift` - Onboarding 流程
- `OnboardingViewModel.swift` - 狀態管理
- `SharedSettings.swift` - 設定儲存
- `DictionarySettingsView.swift` - 詞庫設定
- Theme 相關（`TaigiTheme.swift` 等）

### 需要重構

- `ContentView.swift` → 拆分為 `HomeTabView` + 組件
- `SettingsView.swift` → 拆分為多個子頁面

### 新增

- `MainTabView.swift` - Tab 容器
- `LearnTabView.swift` - 學習 Tab
- `Components/` - 可重用組件目錄

---

## 10. 注意事項

1. **向後相容** - Deep Link `taigikeyboard://settings` 需維持運作
2. **記憶體管理** - 維持 View 與 Controller 分離原則
3. **App Groups** - 設定同步機制不變
4. **iOS 版本** - 確認 NavigationStack 需求（iOS 16+）
5. **測試** - 每個階段完成後進行手動測試

---

## 參考資料

- [azooKey UI/UX 分析](./azookey-ui.md)
- [KeyboardKit 文檔](https://keyboardkit.github.io/KeyboardKitDocs/)
- [現有專案結構](./files.md)
