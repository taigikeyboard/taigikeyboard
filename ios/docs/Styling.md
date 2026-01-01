# Styling

按鈕樣式自訂模組，提供 KeyboardKit 10 相容的按鈕內容客製化。

## 架構說明

KeyboardKit 10 移除了 `StandardStyleService`，改用 `buttonContent` 參數和 view modifiers。
本模組透過 `TaigiButtonContent` 整合各 Provider，實現自訂按鈕內容。

## 檔案結構

```
Styling/
├── TaigiButtonContent.swift      # 核心：整合所有 Provider 的按鈕內容
├── Providers/
│   ├── ButtonFontProvider.swift  # 字型提供者（系統/粉圓/芫荽）
│   ├── ButtonTextProvider.swift  # 文字提供者（按鍵標籤）
│   └── ButtonImageProvider.swift # 圖片提供者（SF Symbols）
└── Helpers/
    └── ConfirmKeyTextHelper.swift # 確認鍵文字（組字模式）
```

## 運作流程

```
TaigiKeyboardView
    │
    └── KeyboardView(buttonContent:)
            │
            └── TaigiButtonContent
                    │
                    ├── 1. ButtonImageProvider.buttonImage(for:)
                    │      → 有圖片？顯示圖片
                    │
                    ├── 2. ButtonTextProvider.buttonText(for:)
                    │      → 有文字？顯示文字 + ButtonFontProvider 字型
                    │
                    └── 3. standardContent
                           → 使用 KeyboardKit 預設內容
```

## 使用方式

在 `TaigiKeyboardView` 中：

```swift
KeyboardView(
    layout: layout,
    services: services,
    buttonContent: { params in
        TaigiButtonContent(
            action: params.item.action,
            keyboardContext: keyboardContext,
            standardContent: params.view
        )
    }
)
.keyboardButtonStyle { params in
    var style = params.standardStyle()
    let fontProvider = ButtonFontProvider(keyboardContext: params.context)
    style.keyboardFont = fontProvider.buttonKeyboardFont(for: params.action)
    return style
}
```

## 注意事項

- `TaigiButtonContent` 的 `keyboardContext` 必須用 `@ObservedObject`，否則不會響應狀態變化
- 詳見 `/spec/kk10.md` 的「自訂字型解決方案」章節
