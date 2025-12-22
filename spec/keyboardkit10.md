# KeyboardKit 10 升級指南

從 KeyboardKit 9.9.0 升級到 KeyboardKit 10 的變更記錄。

## 升級流程

1. 先升級至 KeyboardKit 9.9，解決所有棄用警告
2. 升級至 10.0
3. 處理所有破壞性變更
4. 10.0 保留部分遷移棄用代碼協助過渡，但 10.1 會完全移除

## 系統需求變更

| 平台 | 最低版本 |
|------|----------|
| iOS | 16 |
| macOS | 13 |
| tvOS | 16 |
| watchOS | 10 |
| visionOS | 1 |

## 重大架構變更

### SDK 合併

- KeyboardKit 與 KeyboardKit Pro 合併為單一 SDK
- 所有 `import KeyboardKitPro` 必須改為 `import KeyboardKit`
- 從開源改為閉源模式

### 授權變更

- 不再支援編碼在 SDK 內的授權
- 須改用授權文件或 Gumroad 訂閱金鑰

### 專案授權決定

**使用免費版**，不購買 Pro。影響：
- Emoji Keyboard：繼續使用第三方 `ISEmojiView`
- Themes：使用專案自訂的 `View+Theme.swift`
- Autocomplete：使用專案自訂的 `AutocompleteService`
- 經確認，目前專案無使用任何 Pro 功能

## Breaking Changes

### 移除的 API

- **所有先前棄用的代碼已移除**
- `Keyboard.KeyboardCase.auto` 已移除
- `KeyboardContext.deviceTypeForKeyboardIsIpadPro` 已刪除
- `RemoteAutocompleteService` 移除，改用 `RemotePredictionRequest`

### Services 架構變更

- **Callout、Layout 和 Style Services 已移除**
- 改用值 (values) 和視圖修飾符 (view modifiers)
- 視圖現使用**環境注入**管理所有上下文，而非初始化注入

### 重命名

| 舊名稱 | 新名稱 |
|--------|--------|
| `LocalAutocompleteService` | `StandardAutocompleteService` |
| `NextWordPredictionRequest` | `RemotePredictionRequest` |

### API 簽名變更

- `Keyboard.BottomRow` 現需使用 `services` 初始化器
- `KeyboardView` 改用 `services` 參數而非個別服務參數
- `KeyboardLayout.deviceConfiguration` 轉換為非可選 (non-optional)
- `Locale.ContextMenu` 不再支持自定義菜單項視圖
- `GestureButton` 現在在 actions 中提供 geometry proxy

### Emoji Keyboard 重構

- Emoji keyboard 完全重寫
- 改善響應速度和記憶體管理
- 包含破壞性變更

### 屏幕遷移

- `KeyboardApp` 相關屏幕遷移至各相關命名空間
- Pro 設置屏幕進行重大重構

## 新功能

### Clipboard 命名空間

- 新增 `Clipboard` 命名空間
- 支援貼上系統剪貼簿和自訂片段
- 新增 `ClipsScreen`、`ClipboardContext`
- 新增 `.clipboard` 鍵盤類型

### Fonts 命名空間

- 支援 Unicode 字體
- 自動套用至鍵盤

### 新主題

- 新增 `KeyboardTheme.blueprint` 經典藍圖風格主題

### 新設置

- 雙擊 Shift 啟用大寫鎖定
- 雙擊空格條關閉句子

### 其他改進

- `Keyboard.InputType` 分離鍵盤類型與輸入類型
- `preferredKeyboardDeviceType` 優化 macOS/tvOS 顯示
- 二進制大小減少近 20%

## 10.1 版本新增

- iPad 支援 secondary swipe down actions
- Layout caching 效能改進（需手動啟用）
  ```swift
  Experiment.layoutCaching.setIsEnabled(true)
  ```
- `KeyboardApp.HomeScreen` 和 `KeyboardStatus.Section` 開放給非 Pro 用戶
- 新增 `Experiments` 和 `ExperimentsContext` 管理實驗功能

## 專案升級調整項目

根據 iOS 專案檢查結果，以下是需要調整的項目：

### 1. Services 架構調整（高優先）

KeyboardKit 10 移除了 Callout、Layout、Style Services，改用 view modifiers。

#### CustomLayoutService
- **檔案**: `ios/Sources/TaigiKeyboard/Layout/CustomLayoutService.swift`
- **現況**: 繼承 `KeyboardLayout.StandardLayoutService`
- **影響**: 需確認 `StandardLayoutService` 是否仍存在，或需改用其他方式

#### CustomStyleService
- **檔案**: `ios/Sources/TaigiKeyboard/Styling/CustomStyleService.swift`
- **現況**: 繼承 `KeyboardStyle.StandardStyleService`
- **方法**: `buttonText()`, `buttonImage()`, `buttonKeyboardFont()`, `buttonContentInsets()`
- **影響**: 需確認新的樣式設置方式

#### TaigiCalloutService
- **檔案**: `ios/Sources/TaigiKeyboard/Callouts/Callouts+TaigiActions.swift`
- **現況**: 實作 `CalloutService` 協議
- **方法**: `calloutActions(for:)`
- **影響**: 需改用 `.keyboardCalloutActions()` view modifier（目前已有使用）

#### Setup 中的服務設置
- **檔案**: `ios/Sources/TaigiKeyboard/_Keyboard/KeyboardViewController+Setup.swift:30-31`
```swift
services.layoutService = CustomLayoutService()
services.styleService = CustomStyleService(keyboardContext: state.keyboardContext)
```
- **影響**: 需確認 `services.layoutService` 和 `services.styleService` 是否仍可用

---

### 2. Keyboard.KeyboardCase.auto 移除（高優先）

- **檔案**: `ios/Sources/TaigiKeyboard/Actions/ActionHandler+CharacterInput.swift:24`
```swift
keyboardContext.keyboardCase == .auto ? .lowercased : ...
```
- **影響**: `.auto` 已移除，需改用其他邏輯處理大小寫

---

### 3. KeyboardView 初始化方式（中優先）

- **檔案**: `ios/Sources/TaigiKeyboard/_Keyboard/TaigiKeyboardView.swift:36-61`
- **現況**: 使用 `KeyboardView(state:, services:, ...)` 初始化
- **影響**: 需確認參數是否有變更，目前已使用 `state` 和 `services` 參數

---

### 4. Callout 樣式設置（中優先）

- **檔案**: `ios/Sources/TaigiKeyboard/_Keyboard/KeyboardViewController+Setup.swift:115-134`
- **現況**: 使用 `createCalloutStyle()` 創建 Callout 樣式
- **影響**: 需確認 Callout 樣式 API 是否有變更

- **檔案**: `ios/Sources/TaigiKeyboard/_Keyboard/TaigiKeyboardView.swift:62-63`
```swift
.keyboardCalloutActions(CustomCalloutActions.directBuilder)
.keyboardCalloutStyle(calloutStyle)
```
- **影響**: 需確認 view modifier 是否有 API 變更

---

### 5. 無需調整的項目（確認完成）

以下項目經檢查後**無需調整**：

| 項目 | 狀態 | 說明 |
|------|------|------|
| `import KeyboardKitPro` | ✅ 無使用 | 專案只使用 `import KeyboardKit` |
| `LocalAutocompleteService` | ✅ 無使用 | 使用自訂 `AutocompleteService` |
| `RemoteAutocompleteService` | ✅ 無使用 | 未使用遠端自動完成 |
| `NextWordPredictionRequest` | ✅ 無使用 | 使用自訂 NextWord 邏輯 |
| `deviceTypeForKeyboardIsIpadPro` | ✅ 無使用 | 使用 `context.deviceType` |
| `Keyboard.BottomRow` | ✅ 無使用 | 使用自訂 `BottomRowBuilder` |
| `Locale.ContextMenu` | ✅ 無使用 | 未使用自訂菜單項視圖 |
| Emoji Keyboard | ✅ 無影響 | 使用第三方 `ISEmojiView`，非 KeyboardKit Emoji |

---

### 6. 升級檢查清單

- [ ] 確認 `KeyboardLayout.StandardLayoutService` 在 v10 的替代方案
- [ ] 確認 `KeyboardStyle.StandardStyleService` 在 v10 的替代方案
- [ ] 確認 `CalloutService` 協議在 v10 的狀態
- [ ] 移除或替換 `Keyboard.KeyboardCase.auto` 的使用
- [ ] 測試 `KeyboardView` 初始化是否正常
- [ ] 測試 `.keyboardCalloutActions()` 和 `.keyboardCalloutStyle()` 是否正常
- [x] 確認授權方式 → 使用免費版，無需 Pro 授權
- [ ] 升級後執行完整功能測試

---

### 7. 專案檔案統計

- **使用 KeyboardKit 的檔案**: 38 個
- **自訂 Services**: 3 個（Layout、Style、Callout）
- **KeyboardContext 擴展**: 2 個（Composing、Translate）
- **主要協議實作**: 6 個

## 參考資料

### 官方公告與部落格
- [KeyboardKit 10 公告](https://keyboardkit.com/blog/2025/09/29/keyboardkit-10)
- [KeyboardKit 10 Developer Preview](https://danielsaidi.com/blog/2025/08/31/keyboardkit-10-developer-preview)
- [Liquid Glass 支援說明](https://keyboardkit.com/blog/2025/07/28/custom-ios-keyboard-extensions-and-liquid-glass)

### GitHub
- [GitHub Releases](https://github.com/KeyboardKit/KeyboardKit/releases)
- [10.0.0 Release Notes](https://github.com/KeyboardKit/KeyboardKit/releases/tag/10.0.0)
- [10.1.0 Release Notes](https://github.com/KeyboardKit/KeyboardKit/releases/tag/10.1.0)
- [RELEASE_NOTES.md](https://github.com/KeyboardKit/KeyboardKit/blob/master/RELEASE_NOTES.md)

### 官方文檔
- [KeyboardKit 官網](https://keyboardkit.com/)
- [KeyboardKit 文檔](https://docs.keyboardkit.com/documentation/keyboardkit/)
- [Swift Package Index](https://swiftpackageindex.com/KeyboardKit/KeyboardKit)
