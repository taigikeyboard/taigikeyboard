# App UI 設計指南

> **類型**: 功能
> **關鍵字**: `App`, `Tab`, `Settings`, `UI`
> **相關**: theme.md

---

## 重點摘要

- iOS/Android 主 App UI 設計規範
- 4 Tab 結構：頭頁、佈局、詞庫、設定
- 支援 Light/Dark Mode（iOS）

---

## Tab 結構

| Tab | iOS | Android |
|-----|-----|---------|
| Tab1 頭頁 | `Tab1.swift` | `Tab1Fragment.kt` |
| Tab2 佈局 | `Tab2.swift` | `Tab2Fragment.kt` |
| Tab3 詞庫 | `Tab3.swift` | `Tab3Fragment.kt` |
| Tab4 設定 | `Tab4.swift` | `Tab4Fragment.kt` |

---

## iOS 間距規範（8pt 倍數）

| 用途 | 間距 |
|------|------|
| Section 之間 | 32pt |
| 水平 padding | 20pt |
| 頂部 padding | 24pt |
| 卡片內部 | 16pt |
| 列表項目垂直 | 14pt |

---

## 字體大小（Apple HIG）

| 用途 | 大小 | 方法 |
|------|------|------|
| Hero Title | 34pt | `themeFontHeroTitle()` |
| Title | 24pt | `themeFontTitle()` |
| Headline | 18pt | `themeFontHeadline()` |
| Body | 17pt | `themeFontBody()` |
| Caption | 14pt | `themeFontCaption()` |

---

## 導航方式

| 類型 | 用途 | 方式 |
|------|------|------|
| 子頁面 | 詳細頁面 | `NavigationLink` |
| 首次啟動 | Onboarding | `fullScreenCover` |
| 外部連結 | 網頁/設定 | `openURL` |

---

## 元件規範

### 列表項目

```
[圖示/編號] [標題] [Spacer] [chevron.right]
```

- 圖示：16pt，寬度 24pt
- chevron：12pt
- 水平 padding：16pt
- 垂直 padding：14pt

### 卡片容器

- cornerRadius：12pt
- elevation：0
- strokeWidth：1pt

---

## 本地化架構

| 檔案 | 用途 |
|------|------|
| `LocalizedText.swift` | 核心結構 |
| `Tab1Texts.swift` | Tab1 文字 |
| `Tab2Texts.swift` | Tab2 文字 |
| `Tab3Texts.swift` | Tab3 文字 |
| `Tab4Texts.swift` | Tab4 文字 |

---

## 圖片資源

### 命名規範

- 必須加 `@3x` 後綴
- 例：`layout_standard_preview@3x.png`

### 尺寸規格

| 類型 | 顯示尺寸 | @3x 圖片 |
|------|----------|----------|
| 佈局預覽 | 180×120pt | 540×360px |
| 截圖 | 自適應 | 視內容 |

---

## 平台檔案對照

### iOS

| 元件 | 檔案 |
|------|------|
| Tab 容器 | `ContentView.swift` |
| 主題色 | `ThemeTokens.swift` |
| 本地化 | `LocalizedText.swift` |
| 圖片資源 | `Assets.xcassets/` |

### Android

| 元件 | 檔案 |
|------|------|
| Tab 容器 | `SettingsMainActivity.kt` |
| 底部導航 | `bottom_navigation.xml` |
| 顏色 | `colors.xml` |
| 字串 | `strings.xml` |
