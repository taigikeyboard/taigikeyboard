# 主 APP UI 設計指南

> **關鍵字**: `UI`, `設計`, `風格`, `跨平台`

本文件記錄 iOS 和 Android 主 APP 的 UI 設計規範，確保兩平台視覺一致性。

---

# iOS 專區

---

## 設計原則

### 整體風格
- 簡潔、現代的設計語言
- 遵循 Apple Human Interface Guidelines
- 使用主題色系統（ThemeTokens）確保一致性
- 全螢幕呈現重要流程（Onboarding）
- **支援 Light/Dark Mode**：自動跟隨系統設定

### 參考來源
- [azookey](https://github.com/ensan-hcl/azookey) - iOS 日文鍵盤 APP

---

## 間距規範（Apple HIG）

使用 8pt 倍數系統：

| 用途 | 間距 |
|------|------|
| Section 之間 | 32pt |
| 水平 padding | 20pt |
| 頂部 padding | 24pt |
| 底部 padding | 40pt |
| Section header 到內容 | 8pt |
| 卡片內部 padding | 16pt |
| 列表項目垂直 padding | 14pt |
| 圖示與文字間距 | 12pt |

---

## 顏色系統（支援 Light/Dark Mode）

### 動態顏色定義

顏色使用 `UIColor { trait in ... }` 實現動態切換：

```swift
static var surfacePrimary: Color {
    Color(UIColor { trait in
        trait.userInterfaceStyle == .dark
            ? UIColor(red: 0.11, green: 0.11, blue: 0.12, alpha: 1)
            : UIColor(red: 0.961, green: 0.941, blue: 0.910, alpha: 1)
    })
}
```

### 顏色對照表

| 名稱 | 用途 | Light | Dark |
|------|------|-------|------|
| `surfacePrimary` | 主要背景 | #f5f0e8 米色 | #1c1c1e 深灰 |
| `surfaceSecondary` | 卡片背景 | #fbf9f5 淺白 | #2c2c2e 卡片深灰 |
| `accent` | 主要強調色 | #6e8a7d 青灰 | #8ba697 亮青灰 |
| `accentSecondary` | 次要強調色 | #610a10 深紅 | #d95959 亮紅 |
| `accentTertiary` | 第三強調色 | #fab3ad 粉紅 | #fabeba 亮粉紅 |
| `textPrimary` | 主要文字 | #2c2827 深棕 | #ffffff 白色 |
| `textSecondary` | 次要文字 | #57675c 灰綠 | #8e8e93 系統灰 |
| `cardStroke` | 卡片邊框 | 20% opacity | 15% opacity |

### 顏色使用原則
- Section 標題使用 `textSecondary`
- 列表項目圖示統一使用 `accent`
- chevron 箭頭使用 `textSecondary`
- 數字編號圓圈：白色文字 + `accent` 背景
- SegmentedControl 未選取文字：`textPrimary`（非固定黑色）

---

## 元件規範

### Section Header

```swift
LocalizedTextView(title)
    .themeFontHeadline()
    .foregroundColor(Color.Theme.textSecondary)
```

### 列表項目（ListRow）

標準結構：
```
[圖示/編號] [標題文字] [Spacer] [chevron.right]
```

規格：
- 圖示大小：16pt，寬度固定 24pt
- chevron 大小：12pt
- 水平 padding：16pt
- 垂直 padding：14pt
- 分隔線 padding：16pt

#### 帶圖示的列表項目
```swift
HStack(spacing: 12) {
    Image(systemName: icon)
        .foregroundColor(Color.Theme.accent)
        .font(.system(size: 16))
        .frame(width: 24)

    LocalizedTextView(title)
        .themeFontBody()
        .foregroundColor(Color.Theme.textPrimary)

    Spacer()

    Image(systemName: "chevron.right")
        .foregroundColor(Color.Theme.textSecondary)
        .font(.system(size: 12))
}
.padding(.horizontal, 16)
.padding(.vertical, 14)
```

#### 帶編號的列表項目
```swift
HStack(spacing: 12) {
    Text("\(number)")
        .font(.system(size: 14, weight: .medium))
        .foregroundColor(.white)
        .frame(width: 22, height: 22)
        .background(Color.Theme.accent)
        .clipShape(Circle())

    LocalizedTextView(title)
        .themeFontBody()
        .foregroundColor(Color.Theme.textPrimary)

    Spacer()

    Image(systemName: "chevron.right")
        .foregroundColor(Color.Theme.textSecondary)
        .font(.system(size: 12))
}
```

### 卡片容器

```swift
VStack(spacing: 0) {
    // 列表項目...
}
.themedCard()
```

### 詳細頁面段落（卡片式）

```swift
ForEach(paragraphs.indices, id: \.self) { index in
    LocalizedTextView(paragraphs[index])
        .themeFontBody()
        .foregroundColor(Color.Theme.textPrimary)
        .lineSpacing(6)
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(16)
        .background(Color.Theme.surfaceSecondary)
        .clipShape(RoundedRectangle(cornerRadius: 12))
}
```

---

## 字體大小規範（Apple HIG）

| 用途 | 大小 | 方法 |
|------|------|------|
| Hero Title | 34pt | `themeFontHeroTitle()` |
| Title | 24pt | `themeFontTitle()` |
| Headline | 18pt | `themeFontHeadline()` |
| Body | 17pt | `themeFontBody()` |
| Caption | 14pt | `themeFontCaption()` |
| Footnote | 12pt | `themeFontFootnote()` |

> **注意**：Body 字體為 17pt，符合 Apple HIG 標準（非 16pt）

---

## 頁面結構

### Tab 頁面基本結構

```swift
NavigationStack {
    ScrollView {
        VStack(spacing: 32) {
            // Section 1
            VStack(alignment: .leading, spacing: 8) {
                SectionHeader(title: ...)
                VStack(spacing: 0) {
                    // 列表項目
                }
                .themedCard()
            }

            // Section 2...
        }
        .padding(.horizontal, 20)
        .padding(.top, 24)
        .padding(.bottom, 40)
    }
    .background(Color.Theme.surfacePrimary)
    .navigationTitle(...)
    .navigationBarTitleDisplayMode(.large)
}
```

### 詳細頁面結構

```swift
ScrollView {
    VStack(alignment: .leading, spacing: 16) {
        // 卡片式段落
    }
    .padding(.horizontal, 16)
    .padding(.vertical, 20)
}
.background(Color.Theme.surfacePrimary)
.navigationTitle(...)
.navigationBarTitleDisplayMode(.large)
```

---

## Tab2 佈局頁面（Tab2.swift）

### 水平排列風格（azooKey 風格）

```swift
HStack(spacing: 16) {
    // 左側：預覽圖
    Image(previewImage)
        .resizable()
        .aspectRatio(contentMode: .fit)
        .frame(width: 180, height: 120)
        .clipShape(RoundedRectangle(cornerRadius: 8))
        .overlay(
            RoundedRectangle(cornerRadius: 8)
                .stroke(Color.Theme.cardStroke, lineWidth: 1)
        )

    // 右側：標題置中
    LocalizedTextView(title)
        .themeFontBody()
        .foregroundColor(Color.Theme.textPrimary)
        .frame(maxWidth: .infinity)
}
.padding(16)
.themedCard()
```

### 設計要點
- 左側預覽圖，右側標題
- 預覽圖尺寸：180×120 pt
- 標題在剩餘空間置中
- 不顯示描述文字（只保留標題）
- 選中狀態使用 checkmark overlay

---

## 版權聲明頁面（CopyrightView）

### 單頁滾動式設計

```swift
ScrollView {
    VStack(spacing: 32) {
        // 每個項目一個獨立卡片
        CopyrightCard(...)
        CopyrightCard(...)
    }
    .padding(.horizontal, 20)
    .padding(.top, 24)
    .padding(.bottom, 40)
}
.background(Color.Theme.surfacePrimary)
.navigationTitle(...)
.navigationBarTitleDisplayMode(.large)
```

### 卡片結構（CopyrightCard）

```swift
VStack(alignment: .leading, spacing: 0) {
    // 標題區塊
    VStack(alignment: .leading, spacing: 8) {
        LocalizedTextView(title)
            .themeFontTitle()  // 24pt
            .foregroundColor(Color.Theme.textPrimary)

        LocalizedTextView(description)
            .themeFontBody()
            .foregroundColor(Color.Theme.textSecondary)

        LocalizedTextView(license)
            .themeFontCaption()
            .foregroundColor(Color.Theme.textSecondary)
            .italic()
    }
    .padding(20)

    Divider()
        .background(Color.Theme.cardStroke)

    // 授權條款連結
    Button { ... } label: {
        HStack {
            Text("授權條款")
                .themeFontBody()
                .foregroundColor(Color.Theme.textPrimary)  // 黑色，非 accent
            Spacer()
            Image(systemName: "arrow.up.right")
                .foregroundColor(Color.Theme.textSecondary)
        }
        .padding(.horizontal, 20)
        .padding(.vertical, 14)
    }

    Divider()
        .background(Color.Theme.cardStroke)
        .padding(.leading, 20)

    // 官方網站連結（可選）
    Button { ... } label: { ... }
}
.themedCard()
```

### 設計要點
- 從 Tab4 移到 Tab1 資源連結區塊
- 使用 `NavigationLink`（從右滑入），不用 `.sheet`
- 每個項目獨立卡片，區塊間距 32pt
- 標題使用 `themeFontTitle`（24pt）
- 連結垂直排列，中間有分隔線
- 連結文字顏色：`textPrimary`（黑色），不是 `accent`

---

## FAQ 子頁面（FAQDetailView）

### 帶導覽連結的段落結構

```swift
ScrollView {
    VStack(alignment: .leading, spacing: 16) {
        // 第一段
        paragraphView(paragraphs[0])

        // 導覽連結
        navigationLinkRow(
            icon: "keyboard.badge.ellipsis",
            title: HomeTabTexts.goToSetupGuide,
            destination: AnyView(SetupGuideView())
        )

        // 剩餘段落
        ForEach(1..<paragraphs.count, id: \.self) { index in
            paragraphView(paragraphs[index])
        }
    }
    .padding(.horizontal, 16)
    .padding(.vertical, 20)
}
```

### 導覽連結樣式

```swift
NavigationLink(destination: destination) {
    HStack(spacing: 12) {
        Image(systemName: icon)
            .foregroundColor(Color.Theme.accent)
            .font(.system(size: 16))
            .frame(width: 24)

        Text(title)
            .themeFontBody()
            .foregroundColor(Color.Theme.textPrimary)

        Spacer()

        Image(systemName: "chevron.right")
            .font(.system(size: 12))
            .foregroundColor(Color.Theme.textSecondary)
    }
    .padding(16)
    .background(Color.Theme.surfaceSecondary)
    .clipShape(RoundedRectangle(cornerRadius: 12))
}
.buttonStyle(.plain)
```

### 帶螢幕截圖的 FAQ

```swift
// 第一段
paragraphView(paragraphs[0])

// 螢幕截圖（橫幅型，填滿寬度）
Image("faq_tone_handling")
    .resizable()
    .aspectRatio(contentMode: .fit)
    .frame(maxWidth: .infinity)
    .clipShape(RoundedRectangle(cornerRadius: 8))
    .overlay(
        RoundedRectangle(cornerRadius: 8)
            .stroke(Color.Theme.cardStroke, lineWidth: 1)
    )

// 第二段
paragraphView(paragraphs[1])
```

---

## 導航方式

### 導航原則

- **子頁面導航**：統一使用 `NavigationLink`，動畫為從右側滑入（標準 push）
- **首次啟動流程**：使用 `fullScreenCover`，動畫為從下往上彈出
- **外部連結**：使用 `openURL` 開啟系統設定或網頁

### 導航類型對照

| 觸發位置 | 目標頁面 | 導航方式 | 動畫效果 |
|----------|----------|----------|----------|
| HomeTab「啟用方法」| SetupGuideView | `NavigationLink` | 從右滑入 |
| HomeTab「新功能」| FeatureDetailView | `NavigationLink` | 從右滑入 |
| HomeTab「處理中的問題」| IssueDetailView | `NavigationLink` | 從右滑入 |
| HomeTab「預計新功能」| UpcomingDetailView | `NavigationLink` | 從右滑入 |
| HomeTab「常見問題」| FAQDetailView | `NavigationLink` | 從右滑入 |
| 首次啟動 | OnboardingView | `fullScreenCover` | 從下彈出 |
| 資源連結 | 外部網頁 | `openURL` | 系統預設 |

### NavigationLink 用法

```swift
NavigationLink(destination: DetailView()) {
    ListRow()
}
.buttonStyle(.plain)
```

### fullScreenCover 用法（僅首次啟動）

```swift
.fullScreenCover(isPresented: $shouldShowOnboarding) {
    OnboardingView(onComplete: { ... })
}
```

---

## Onboarding 設計

### 使用情境

| 情境 | 入口 | 呈現方式 |
|------|------|----------|
| 首次啟動 | TaigiKeyboardApp | `fullScreenCover`（從下彈出）|
| 點擊「啟用方法」| HomeTab | `NavigationLink`（從右滑入）|

### 首次啟動（fullScreenCover）
- 使用 `.fullScreenCover` 全螢幕呈現
- 右上角 X 圖示關閉按鈕
- 顯示 OnboardingView → SetupKeyboardPage

### 從 HomeTab 進入（NavigationLink）
- 使用標準 `NavigationLink` 導航
- 系統自動提供返回按鈕
- 顯示 SetupGuideView

### 關閉按鈕

```swift
Button(action: onComplete) {
    Image(systemName: "xmark")
        .font(.system(size: 16, weight: .medium))
        .foregroundColor(Color.Theme.textSecondary)
        .padding(12)
        .background(Color.Theme.surfaceSecondary)
        .clipShape(Circle())
}
```

### 步驟卡片（SetupStepCard）

```swift
VStack(alignment: .leading, spacing: 12) {
    // 步驟標題
    HStack(spacing: 12) {
        Text("\(stepNumber)")
            .font(.system(size: 14, weight: .bold))
            .foregroundColor(.white)
            .frame(width: 24, height: 24)
            .background(Color.Theme.accent)
            .clipShape(Circle())

        LocalizedTextView(title)
            .themeFontBody()
            .foregroundColor(Color.Theme.textPrimary)

        Spacer()
    }

    // 截圖
    ScreenshotView(imageName: screenshotName)
}
.padding(16)
.background(Color.Theme.surfaceSecondary)
.clipShape(RoundedRectangle(cornerRadius: 12))
```

### 截圖規格

| 項目 | 規格 |
|------|------|
| 顯示高度 | 200pt |
| @2x 圖片 | 600 × 400 px |
| @3x 圖片 | 900 × 600 px |
| 圓角 | 8pt |
| 邊框 | 1pt `cardStroke` |

### 狀態指示器（StatusIndicator）

```swift
HStack(spacing: 6) {
    Image(systemName: isEnabled ? "checkmark.circle.fill" : "circle")
        .font(.system(size: 14))
        .foregroundColor(isEnabled ? Color.Theme.accent : Color.Theme.textSecondary)

    LocalizedTextView(isEnabled ? enabledText : disabledText)
        .themeFontCaption()
        .foregroundColor(isEnabled ? Color.Theme.accent : Color.Theme.textSecondary)
}
```

---

## 圖片資源規格

### 命名規範

**所有圖片檔案必須加上 `@3x` 後綴**：

```
✓ layout_standard_preview@3x.png
✓ setup_step1@3x.png
✗ layout_standard_preview.png  （錯誤）
```

### Contents.json 範本

```json
{
  "images" : [
    { "idiom" : "universal", "scale" : "1x" },
    { "idiom" : "universal", "scale" : "2x" },
    {
      "filename" : "image_name@3x.png",
      "idiom" : "universal",
      "scale" : "3x"
    }
  ],
  "info" : { "author" : "xcode", "version" : 1 }
}
```

### 佈局預覽圖（LayoutTab）

| 項目 | 規格 |
|------|------|
| 顯示尺寸 | 180 × 120 pt |
| @3x 圖片 | 540 × 360 px |

檔案：
- `layout_standard_preview@3x.png`
- `layout_phahtaigi_preview@3x.png`

### Onboarding 截圖

| 項目 | 規格 |
|------|------|
| 顯示方式 | 自適應高度 |
| @3x 圖片 | 視內容而定 |

檔案：
- `setup_step1@3x.png`（設定 → 鍵盤）
- `setup_step2@3x.png`（新增鍵盤）

### FAQ 截圖

| 項目 | 規格 |
|------|------|
| 顯示方式 | 填滿寬度，自適應高度 |
| 圓角 | 8pt |
| 邊框 | 1pt `cardStroke` |

檔案：
- `faq_tone_handling@3x.png`（聲調 1, 4 說明）

### 新功能截圖

| 項目 | 規格 |
|------|------|
| 顯示方式 | 輪播式（ImageSlideshowView）|
| @3x 圖片 | 視內容而定 |

檔案：
- `nextword_1@3x.png`
- `nextword_2@3x.png`
- `nextword_3@3x.png`

---

## 互動效果

### 按壓效果

列表項目使用 `pressableCardGesture`：
```swift
.contentShape(Rectangle())
.pressableCardGesture(isPressed: isPressed) { pressed in
    isPressed = pressed
}
```

按壓時背景色變為 `Color.Theme.surfaceSecondary`。

### 選中動畫（LayoutTab）

使用 `matchedGeometryEffect` 實現選中標記動畫：
```swift
Circle()
    .fill(Color.Theme.accent)
    .frame(width: 48, height: 48)
    .overlay(
        Image(systemName: "checkmark")
            .font(.system(size: 24, weight: .bold))
            .foregroundColor(.white)
    )
    .matchedGeometryEffect(id: checkmarkId, in: namespace)
```

---

## 本地化架構

### LocalizedText 結構

簡化版本，僅支援漢字（hanji）：

```swift
struct LocalizedText {
    let hanji: String

    init(hanji: String) {
        self.hanji = hanji
    }

    init(_ text: String) {
        self.hanji = text
    }
}
```

### LanguageManager

```swift
class LanguageManager: ObservableObject {
    static let shared = LanguageManager()

    func text(_ localizedText: LocalizedText) -> String {
        localizedText.hanji
    }
}
```

### 使用方式

**定義文字**：
```swift
enum MyTexts {
    static let title = LocalizedText(hanji: "標題")
    static let description = LocalizedText(hanji: "說明文字")
}
```

**在 View 中使用**：
```swift
// 方式 1：LocalizedTextView（推薦）
LocalizedTextView(MyTexts.title)
    .themeFontHeadline()

// 方式 2：LanguageManager
@StateObject private var languageManager = LanguageManager.shared

Text(languageManager.text(MyTexts.title))
```

### 文字定義檔案

| 檔案 | 用途 |
|------|------|
| `Localization/LocalizedText.swift` | 核心結構 |
| `Localization/AppInfoTexts.swift` | APP 資訊文字 |
| `Localization/TabTexts.swift` | Tab 標題 |
| `Localization/ActionTexts.swift` | 操作按鈕文字 |
| `Localization/KeyboardTexts.swift` | 鍵盤設定文字 |
| `Localization/OnboardingTexts.swift` | 引導流程文字 |
| `Localization/CopyrightTexts.swift` | 版權聲明文字 |

### 頁面專用文字

各 Tab 頁面在檔案內定義專用文字：

```swift
// HomeTab.swift
enum HomeTabTexts {
    static let setupKeyboard = LocalizedText(hanji: "啟用鍵盤")
    // ...
}

// LayoutTab.swift
enum LayoutTabTexts {
    static let selectLayout = LocalizedText(hanji: "選擇佈局")
    // ...
}
```

---

## 檔案對照

| 元件 | 檔案位置 |
|------|----------|
| Tab 容器 | `App/ContentView.swift` |
| Tab1 頭頁 | `App/Tabs/Tab1.swift` |
| Tab2 佈局 | `App/Tabs/Tab2.swift` |
| Tab3 詞庫 | `App/Tabs/Tab3.swift` |
| Tab4 設定 | `App/Tabs/Tab4.swift` |
| 啟用方法頁面 | `App/Tabs/Tab1.swift` → `SetupGuideView` |
| 版權聲明頁面 | `Copyright/Views/CopyrightView.swift` |
| FAQ 詳細頁面 | `App/Tabs/Tab1.swift` → `FAQDetailView` |
| Onboarding 主視圖 | `Onboarding/OnboardingView.swift` |
| 完成頁面 | `Onboarding/CompletedPage.swift` |
| 主題色 + 字體定義 | `Theme/ThemeTokens.swift` |
| 本地化核心 | `Localization/LocalizedText.swift` |
| 版權文字 | `Localization/CopyrightTexts.swift` |
| 圖片資源 | `App/Assets/Assets.xcassets/` |

---

## 變更記錄

### 2025-12-24
- 新增：強制 Light Mode → 改為支援 Light/Dark Mode
- 新增：Tab2 水平排列風格
- 新增：版權聲明頁面設計
- 新增：FAQ 子頁面帶導覽連結和截圖
- 新增：螢幕截圖命名規範
- 更新：Body 字體 16pt → 17pt
- 重新命名：HomeTab→Tab1、LayoutTab→Tab2、DictionaryTab→Tab3、SettingsTab→Tab4

---
---

# Android 專區

本節記錄 Android 主 APP 的 UI 設計規範，與 iOS 設計保持一致。

---

## 設計原則

### 整體風格
- 簡潔、現代的設計語言
- 遵循 Material Design Guidelines
- 使用 `colors.xml` 資源系統確保一致性
- **支援 Light/Dark Mode**：自動跟隨系統設定

### Tab 架構
- 使用 `BottomNavigationView` 實現底部導航
- Fragment 架構管理各 Tab 內容
- 與 iOS `TabView` 對應

---

## 顏色系統（支援 Light/Dark Mode）

### 顏色資源檔案

| 模式 | 檔案路徑 |
|------|----------|
| Light Mode | `res/values/colors.xml` |
| Dark Mode | `res/values-night/colors.xml` |

### 顏色對照表

| 名稱 | 用途 | Light | Dark |
|------|------|-------|------|
| `modern_surface_primary` | 主要背景 | #f5f0e8 | #1c1c1e |
| `modern_surface_secondary` | 卡片背景 | #E6fbf9f5 | #E62c2c2e |
| `modern_accent` | 主要強調色 | #6e8a7d | #8ba697 |
| `modern_accent_secondary` | 次要強調色 | #610a10 | #d95959 |
| `modern_text_primary` | 主要文字 | #2c2827 | #ffffff |
| `modern_text_secondary` | 次要文字 | #57675c | #8e8e93 |
| `modern_surface_card` | 卡片背景 | #fbf9f5 | #2c2c2e |

### 使用方式

```xml
android:background="@color/modern_surface_primary"
android:textColor="@color/modern_text_primary"
```

---

## 間距規範

使用 `res/values/dimens.xml` 定義：

| 用途 | 間距 |
|------|------|
| 卡片邊距 | 16dp (`card_margin`) |
| 卡片內部 padding | 20dp |
| Section 之間 | 32dp |
| 列表項目垂直 padding | 14dp |
| 圖示與文字間距 | 12dp |

---

## Tab 結構對照

| Tab | iOS | Android Fragment |
|-----|-----|------------------|
| Tab1 頭頁 | `Tab1.swift` | `Tab1Fragment.kt` |
| Tab2 佈局 | `Tab2.swift` | `Tab2Fragment.kt` |
| Tab3 詞庫 | `Tab3.swift` | `Tab3Fragment.kt` |
| Tab4 設定 | `Tab4.swift` | `Tab4Fragment.kt` |

---

## 元件規範

### BottomNavigationView 設定

```xml
<com.google.android.material.bottomnavigation.BottomNavigationView
    android:id="@+id/bottom_navigation"
    android:layout_width="match_parent"
    android:layout_height="wrap_content"
    android:background="@color/modern_surface_card"
    app:itemIconTint="@color/bottom_nav_color"
    app:itemTextColor="@color/bottom_nav_color"
    app:labelVisibilityMode="labeled"
    app:menu="@menu/bottom_navigation" />
```

### 卡片容器（CardView）

```xml
<androidx.cardview.widget.CardView
    android:layout_width="match_parent"
    android:layout_height="wrap_content"
    app:cardBackgroundColor="@color/modern_surface_card"
    app:cardCornerRadius="12dp"
    app:cardElevation="0dp">
    <!-- 內容 -->
</androidx.cardview.widget.CardView>
```

### Section Header

```xml
<TextView
    android:layout_width="match_parent"
    android:layout_height="wrap_content"
    android:textColor="@color/modern_text_secondary"
    android:textSize="18sp"
    android:textStyle="bold" />
```

### 列表項目（帶圖示）

```xml
<LinearLayout
    android:layout_width="match_parent"
    android:layout_height="wrap_content"
    android:orientation="horizontal"
    android:gravity="center_vertical"
    android:padding="16dp">

    <ImageView
        android:layout_width="24dp"
        android:layout_height="24dp"
        android:src="@drawable/ic_icon"
        app:tint="@color/modern_accent" />

    <TextView
        android:layout_width="0dp"
        android:layout_height="wrap_content"
        android:layout_weight="1"
        android:layout_marginStart="12dp"
        android:textColor="@color/modern_text_primary"
        android:textSize="17sp" />

    <ImageView
        android:layout_width="12dp"
        android:layout_height="12dp"
        android:src="@drawable/ic_chevron_right"
        app:tint="@color/modern_text_secondary" />
</LinearLayout>
```

### Switch 開關（Material）

```xml
<com.google.android.material.switchmaterial.SwitchMaterial
    android:layout_width="wrap_content"
    android:layout_height="wrap_content"
    app:thumbTint="@color/switch_thumb_color"
    app:trackTint="@color/switch_track_color" />
```

---

## Tab2 佈局頁面

### 水平排列風格

```xml
<LinearLayout
    android:layout_width="match_parent"
    android:layout_height="wrap_content"
    android:orientation="horizontal"
    android:padding="16dp">

    <!-- 左側：預覽圖 -->
    <ImageView
        android:layout_width="180dp"
        android:layout_height="120dp"
        android:scaleType="fitCenter"
        android:background="@drawable/rounded_border" />

    <!-- 右側：標題置中 -->
    <TextView
        android:layout_width="0dp"
        android:layout_height="match_parent"
        android:layout_weight="1"
        android:gravity="center"
        android:textSize="17sp" />
</LinearLayout>
```

### 選中狀態

使用 overlay + checkmark 顯示選中狀態：
- `overlaySelected`：半透明遮罩
- `checkmarkSelected`：accent 色打勾圖示

---

## 版權頁面

### 單頁滾動式設計

從 `ViewPager2` 改為 `ScrollView` + `LinearLayout`：

```xml
<ScrollView
    android:layout_width="match_parent"
    android:layout_height="match_parent"
    android:background="@color/modern_surface_primary">

    <LinearLayout
        android:id="@+id/copyright_container"
        android:layout_width="match_parent"
        android:layout_height="wrap_content"
        android:orientation="vertical"
        android:padding="20dp" />
</ScrollView>
```

### 卡片動態建立

```kotlin
private fun setupCopyrightCards() {
    val container = findViewById<LinearLayout>(R.id.copyright_container)
    copyrightPages.forEach { page ->
        val cardView = createCopyrightCard(page, typeface)
        container.addView(cardView)
    }
}
```

---

## Onboarding 設計

### 簡化為單頁

移除 `WelcomeFragment` 和 `CompletedFragment`，只保留 `SetupFragment`：

```kotlin
if (savedInstanceState == null) {
    supportFragmentManager.beginTransaction()
        .replace(R.id.onboarding_container, SetupFragment())
        .commit()
}
```

---

## 檔案對照

| 元件 | 檔案位置 |
|------|----------|
| Tab 容器 | `settings/SettingsMainActivity.kt` |
| Tab1 頭頁 | `settings/Tab1Fragment.kt` |
| Tab2 佈局 | `settings/Tab2Fragment.kt` |
| Tab3 詞庫 | `settings/Tab3Fragment.kt` |
| Tab4 設定 | `settings/Tab4Fragment.kt` |
| 版權聲明頁面 | `settings/CopyrightActivity.kt` |
| Onboarding | `onboarding/OnboardingActivity.kt` |
| 底部導航選單 | `res/menu/bottom_navigation.xml` |
| Tab 容器布局 | `res/layout/activity_main_tabs.xml` |
| 顏色定義 | `res/values/colors.xml` |
| Dark Mode 顏色 | `res/values-night/colors.xml` |

### 布局檔案

| Fragment | 布局檔案 |
|----------|----------|
| Tab1Fragment | `res/layout/fragment_tab1.xml` |
| Tab2Fragment | `res/layout/fragment_tab2.xml` |
| Tab3Fragment | `res/layout/fragment_tab3.xml` |
| Tab4Fragment | `res/layout/fragment_tab4.xml` |

### Drawable 資源

| 用途 | 檔案 |
|------|------|
| 右箭頭 | `ic_chevron_right.xml` |
| 聊天氣泡 | `ic_chat_bubble.xml` |
| 切換圖示 | `ic_swap_horiz.xml` |
| 地球圖示 | `ic_globe.xml` |
| 外部連結 | `ic_external_link.xml` |
| 隱私圖示 | `ic_privacy.xml` |
| 星星圖示 | `ic_star.xml` |
| 文件圖示 | `ic_document.xml` |
| 歷史圖示 | `ic_history.xml` |
| Accent 圓形 | `circle_accent.xml` |

---

## 字串資源

所有 Tab 文字定義在 `res/values/strings.xml`：

| 前綴 | 用途 |
|------|------|
| `tab1_*` | Tab1 頭頁文字 |
| `tab2_*` | Tab2 佈局文字 |
| `tab3_*` | Tab3 詞庫文字 |
| `tab4_*` | Tab4 設定文字 |

---

## 變更記錄

### 2025-12-24
- 新增：4 Tab 結構（頭頁、佈局、詞庫、設定）
- 新增：BottomNavigationView 底部導航
- 新增：Light/Dark Mode 支援
- 重構：版權頁面從 ViewPager2 改為 ScrollView
- 簡化：Onboarding 從 3 頁改為 1 頁
- 新增：多個 drawable 圖示資源
- 重新命名：Tab 檔案統一為 Tab1~Tab4 命名
