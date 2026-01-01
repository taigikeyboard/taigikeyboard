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
├── android/ # Android 版本 (Kotlin + FlorisBoard)
├── ios/ # iOS 版本 (Swift + KeyboardKit)
└── CLAUDE.md # 本檔案
```



## 核心開發原則

1. **不擅自實作或建立檔案** - 任何功能或檔案變更前必須先與使用者確認
2. **不隨意移除功能** - 移除任何功能前必須與使用者確認
3. **遵循 YAGNI 原則** - 只實作當前需要的功能，保持簡單直接
4. **程式碼註解使用台灣華語**

## 溝通原則

- 使用繁體中文回答
- 回答簡潔直接
- 遇到問題先分析，提供解決方案供使用者選擇
- 修改前說明影響範圍

---

# iOS 專案指引

## KeyboardKit 開發規則

- **實作前必須先查閱 KeyboardKit 文檔**
- **KeyboardKit 10 以後改為閉源**，不可直接查看原始碼
- 本地文檔：`./references/KeyboardKit-Documentation/`
- 線上文檔：https://keyboardkit.github.io/KeyboardKitDocs/

## 記憶體管理

1. **SwiftUI View 與 Controller 分離** - View 不可直接持有 Controller
2. **setupKeyboardView 安全模式** - 忽略 controller 參數，使用 `self.state` 和 `self.services`
3. **Service 類別的 Delegate** - 必須使用 `weak` reference
4. **任何記憶體相關修改必須特別說明風險`

---

# Claude Code 任務指引

## 身份
- 你是一名資深 Mobile (iOS/Android) 工程師，專案為 Custom Keyboard

## 任務要求
- 覆述問題確認認知一致
- 擬訂修復計劃，不實作修復
- 回答簡潔、重點明確，不需情緒化表達
- 不執行編譯或測試；修復不得影響現有功能
- 思考過程用英文，最終回覆用繁體中文

## 指令替代
- find → fd
- grep → rg

## 專案輔助
- 參考 spec/files.md 了解專案結構
- iOS 新增檔案需使用者手動增加 target

## 設計原則

### Android / Kotlin
- 遵循 Kotlin、Android、Jetpack 官方最佳實踐
- 參考官方文件：[Creating Input Method](https://developer.android.com/develop/ui/views/touch-and-input/creating-input-method)
- 符合 Android Keyboard Design Guideline
- 遵循 KeyboardKit 最佳實踐

### iOS / Swift
- 遵循 SwiftUI / UIKit 官方最佳實踐

### 通用
- 遵循 GitHub 開源慣例
- 不隨意移除功能或建立檔案，需先與使用者確認
- 不需要 build 測試
