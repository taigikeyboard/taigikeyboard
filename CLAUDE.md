# CLAUDE.md

此檔案提供給 **Claude Code (claude.ai/code)** 處理本專案程式碼時的指引。

## 專案概述

**台語鍵盤** - 跨平台台語輸入法
- iOS: 使用 Swift 與 KeyboardKit 開發
- Android: 基於 FlorisBoard 的 Kotlin 實作
- 支援台語白話字（POJ/TL）與漢字輸入
- 聲調變化與自動完成功能

## 專案結構

```
taigikeyboard/
├── android/     # Android 版本 (Kotlin + FlorisBoard)
├── ios/         # iOS 版本 (Swift + KeyboardKit)
└── CLAUDE.md    # 本檔案
```

## 核心開發原則

### 必須遵守的規則

1. **不擅自實作** - 任何功能實作前必須先與使用者確認
2. **不擅自建立檔案** - 建立新檔案前必須獲得使用者同意
3. **不 over-design** - 保持簡單直接的解決方案
4. **不隨便移除功能** - 移除任何功能前必須與使用者確認
5. **遵循 YAGNI 原則** - 只實作當前需要的功能

## 溝通原則

- 使用繁體中文回答
- 回答簡潔直接
- 遇到問題先分析，提供解決方案供使用者選擇
- 修改前說明影響範圍

---

# iOS 專案指引

## 編譯與測試

```bash
cd ios
xcodebuild -project TaigiKeyboard.xcodeproj \
  -scheme TaigiKeyboard \
  -configuration Debug \
  -destination 'id=00008101-001A30D62E60001E' \
  build
```

## KeyboardKit 開發規則

- **實作前必須先查閱 KeyboardKit 官方文檔**
- 官方文檔：https://keyboardkit.github.io/KeyboardKitDocs/
- 原始碼：`/Users/alexsu/Library/Developer/Xcode/DerivedData/TaigiKeyboard-*/SourcePackages/checkouts/KeyboardKit/`

## iOS 記憶體管理

### 關鍵規則

1. **SwiftUI View 與 Controller 分離** - View 不可直接持有 Controller
2. **setupKeyboardView 安全模式** - 忽略 controller 參數，使用 `self.state` 和 `self.services`
3. **Service 類別的 Delegate** - 必須使用 `weak` reference
4. **任何記憶體相關修改必須特別說明風險**

---

# Android 專案指引

## 編譯與測試

```bash
cd android
./gradlew assembleDebug
```

## 安裝到裝置

```bash
adb install app/build/outputs/apk/debug/app-debug.apk
```
