# _Keyboard

鍵盤擴充功能的核心模組，包含 KeyboardKit 整合與主要 UI。

## 檔案結構

| 檔案 | 用途 |
|------|------|
| `KeyboardViewController.swift` | 主控制器，繼承 `KeyboardInputViewController` |
| `KeyboardViewController+Setup.swift` | 初始化設定：services、layout、callout style |
| `KeyboardViewController+Cleanup.swift` | 清理邏輯：記憶體釋放、資源回收 |
| `KeyboardViewController+TextInput.swift` | 文字輸入處理 |
| `KeyboardViewController+EmojiDelegate.swift` | Emoji 鍵盤代理 |
| `TaigiKeyboardView.swift` | 主要 SwiftUI 視圖，包含 `KeyboardView` |
| `KeyboardModels.swift` | 鍵盤相關常數與字型設定 |
| `Info.plist` | 鍵盤擴充功能設定 |

## 核心元件

### KeyboardViewController
- 生命週期管理：`viewDidLoad`、`viewWillDisappear`
- 服務初始化：ActionHandler、AutocompleteService、LayoutService
- 記憶體管理：追蹤 instance 數量（DEBUG 模式）

### TaigiKeyboardView
- 整合 KeyboardKit 的 `KeyboardView`
- 自訂 `buttonContent`：使用 `TaigiButtonContent`
- View modifiers：`keyboardButtonStyle`、`keyboardCalloutActions`、`keyboardCalloutStyle`
- 候選詞區塊：`CandidateView`、`ExpandedCandidateOverlay`

## 依賴關係

```
KeyboardViewController
    ├── TaigiKeyboardView (SwiftUI)
    │   ├── KeyboardView (KeyboardKit)
    │   ├── TaigiButtonContent
    │   └── CandidateView
    ├── ActionHandler
    ├── AutocompleteService
    └── EmojiService
```

## 注意事項

- 遵循 KeyboardKit 10 架構（見 `/spec/kk10.md`）
- `TaigiButtonContent` 需使用 `@ObservedObject` 觀察 `keyboardContext`
- 記憶體管理：delegate 使用 `weak` reference
