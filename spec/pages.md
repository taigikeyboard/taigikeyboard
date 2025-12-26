# iOS / Android 頁面與本地化對照

> **關鍵字**: `pages`, `localization`, `對照`, `一致性`

---

## 檔案結構對照

### iOS 結構

```
ios/Sources/TaigiKeyboard/
├── App/
│   ├── TaigiKeyboardApp.swift          # App 入口
│   ├── ContentView.swift               # TabView 容器
│   ├── Tabs/
│   │   ├── TabType.swift               # Tab 類型定義
│   │   ├── Tab1.swift                  # 頭頁（含子頁面）
│   │   ├── Tab2.swift                  # 齒佈
│   │   ├── Tab3.swift                  # 詞庫
│   │   └── Tab4.swift                  # 設定
│   └── Components/
│       ├── ListCard.swift              # 卡片元件
│       └── ImageSlideshowView.swift    # 圖片輪播
├── Localization/
│   ├── LocalizedText.swift             # 本地化核心
│   ├── Tab1Texts.swift                 # Tab1 文字
│   ├── Tab2Texts.swift                 # Tab2 文字
│   ├── Tab3Texts.swift                 # Tab3 文字
│   └── Tab4Texts.swift                 # Tab4 文字
├── Onboarding/
│   ├── OnboardingView.swift
│   ├── OnboardingViewModel.swift
│   └── OnboardingPreferences.swift
├── Copyright/
│   ├── CopyrightView.swift
│   ├── CopyrightPageView.swift
│   └── CopyrightHeader.swift
├── Settings/
│   ├── DictionarySettingsView.swift
│   ├── SettingsComponents.swift
│   └── SharedSettings.swift
└── Debug/
    ├── DebugView.swift
    ├── DebugCardView.swift
    ├── DebugFrequencyView.swift
    └── DebugAssociationView.swift
```

### Android 結構

```
android/app/src/main/java/com/siansiansu/taigikeyboard/
├── localization/                        # ✅ 本地化（對應 iOS Localization/）
│   ├── LocalizedText.kt                # 核心結構（data class）
│   ├── DisplayLanguage.kt              # 語言枚舉 + InputMode
│   ├── LanguageManager.kt              # 語言管理（使用 StateFlow）
│   ├── LocalizedTextView.kt            # UI 擴展
│   ├── Tab1Texts.kt                    # Tab1 文字（對應 iOS）
│   ├── Tab2Texts.kt                    # Tab2 文字
│   ├── Tab3Texts.kt                    # Tab3 文字
│   └── Tab4Texts.kt                    # Tab4 文字
├── settings/
│   ├── SettingsMainActivity.kt         # Activity + BottomNav 容器
│   ├── Tab1Fragment.kt                 # 頭頁
│   ├── Tab2Fragment.kt                 # 齒佈
│   ├── Tab3Fragment.kt                 # 詞庫
│   ├── Tab4Fragment.kt                 # 設定
│   ├── DetailActivity.kt               # 通用詳情頁面（支援多段落、圖片、導航）
│   ├── SetupGuideActivity.kt           # ✅ 啟用方法頁面（對應 iOS SetupGuideView）
│   ├── ImageSlideshowView.kt           # ✅ 圖片輪播元件
│   ├── CopyrightActivity.kt            # 版權頁面
│   ├── CopyrightData.kt                # 版權資料
│   └── DebugActivity.kt                # Debug 頁面
└── onboarding/
    ├── OnboardingActivity.kt
    └── SetupFragment.kt
```

---

## 本地化檔案對照

### iOS（分離式）

| 檔案 | 內容 | 行數 |
|------|------|------|
| `LocalizedText.swift` | 核心結構、LanguageManager | ~50 |
| `Tab1Texts.swift` | Tab1 所有文字（頭頁、功能、問題、FAQ、版權） | ~210 |
| `Tab2Texts.swift` | Tab2 所有文字（齒佈選擇） | ~18 |
| `Tab3Texts.swift` | Tab3 所有文字（詞庫管理） | ~52 |
| `Tab4Texts.swift` | Tab4 所有文字（設定） | ~50 |

### Android（分離式 - ✅ 已完成重構）

| 檔案 | 內容 | 狀態 |
|------|------|------|
| `LocalizedText.kt` | 核心 data class | ✅ |
| `DisplayLanguage.kt` | 語言枚舉 + InputMode | ✅ |
| `LanguageManager.kt` | 語言管理（StateFlow） | ✅ |
| `Tab1Texts.kt` | Tab1 所有文字（對應 iOS） | ✅ |
| `Tab2Texts.kt` | Tab2 所有文字 | ✅ |
| `Tab3Texts.kt` | Tab3 所有文字 | ✅ |
| `Tab4Texts.kt` | Tab4 所有文字 | ✅ |

---

## Tab1 子頁面對照

| iOS 頁面 | iOS 檔案 | Android 對應 | 狀態 |
|----------|----------|--------------|------|
| SetupGuideView | Tab1.swift 內 | SetupGuideActivity.kt | ✅ |
| FeatureDetailView | Tab1.swift 內 | DetailActivity.kt | ✅ |
| IssueDetailView | Tab1.swift 內 | DetailActivity.kt | ✅ |
| UpcomingDetailView | Tab1.swift 內 | DetailActivity.kt | ✅ |
| FAQDetailView | Tab1.swift 內 | DetailActivity.kt | ✅ |
| FeedbackDetailView | Tab1.swift 內 | DetailActivity.kt | ✅ |
| VersionHistoryDetailView | Tab1.swift 內 | DetailActivity.kt | ✅ |
| CopyrightView | Copyright/CopyrightView.swift | CopyrightActivity.kt | ✅ |

---

## DetailActivity 功能對照

| 功能 | iOS | Android | 狀態 |
|------|-----|---------|------|
| 多段落顯示 | ✅ `[LocalizedText]` | ✅ `List<LocalizedText>` | ✅ |
| 圖片輪播 | ✅ ImageSlideshowView | ✅ ImageSlideshowView | ✅ |
| 導航連結 | ✅ NavigationLink | ✅ createNavigationRow | ✅ |

### 圖片輪播使用場景

| 功能說明 | 圖片 | iOS | Android |
|----------|------|-----|---------|
| 連紲建議詞 | nextword_1/2/3 | ✅ | ✅ |

### 導航連結使用場景

| 功能說明 | 導航目標 | iOS | Android |
|----------|----------|-----|---------|
| 異用字開關 | 詞庫頁面 (Tab3) | ✅ | ✅ |
| 詞庫管理 | 詞庫頁面 (Tab3) | ✅ | ✅ |

---

## 元件對照表

| iOS 元件 | Android 元件 | 用途 |
|----------|--------------|------|
| `ImageSlideshowView.swift` | `ImageSlideshowView.kt` | 圖片自動輪播 |
| `LocalizedTextView` | `LocalizedTextView.kt` | 本地化文字顯示 |
| `ListCard` | CardView (XML) | 列表卡片 |

---

## 檔案對照表

| iOS 檔案 | Android 檔案 | 對應關係 |
|----------|--------------|----------|
| `TaigiKeyboardApp.swift` | - | iOS only |
| `ContentView.swift` | `SettingsMainActivity.kt` | Tab 容器 |
| `Tab1.swift` | `Tab1Fragment.kt` | 頭頁 |
| `Tab2.swift` | `Tab2Fragment.kt` | 齒佈 |
| `Tab3.swift` | `Tab3Fragment.kt` | 詞庫 |
| `Tab4.swift` | `Tab4Fragment.kt` | 設定 |
| `TabType.swift` | - | iOS only (enum) |
| `LocalizedText.swift` | `LocalizedText.kt` | 核心結構 ✅ |
| `Tab1Texts.swift` | `Tab1Texts.kt` | Tab1 文字 ✅ |
| `Tab2Texts.swift` | `Tab2Texts.kt` | Tab2 文字 ✅ |
| `Tab3Texts.swift` | `Tab3Texts.kt` | Tab3 文字 ✅ |
| `Tab4Texts.swift` | `Tab4Texts.kt` | Tab4 文字 ✅ |
| `ImageSlideshowView.swift` | `ImageSlideshowView.kt` | 圖片輪播 ✅ |
| `OnboardingView.swift` | `OnboardingActivity.kt` | Onboarding |
| `SetupGuideView` (Tab1 內) | `SetupGuideActivity.kt` | 啟用方法 ✅ |
| `CopyrightView.swift` | `CopyrightActivity.kt` | 版權頁面 |
| `DebugView.swift` | `DebugActivity.kt` | Debug 頁面 |
| `DictionarySettingsView.swift` | (Tab3Fragment 內) | 詞庫設定 |

---

## 重構完成狀態

### ✅ Phase 1: 分離本地化檔案

- [x] 建立 `localization/` 資料夾
- [x] 建立 `Tab1Texts.kt`、`Tab2Texts.kt`、`Tab3Texts.kt`、`Tab4Texts.kt`
- [x] 將 AppTexts 內容按 Tab 分類移動
- [x] 更新所有 import

### ✅ Phase 2: 同步文字內容

- [x] 將 iOS Tab1Texts 的文字同步到 Android
- [x] 支援多段落格式（`List<LocalizedText>`）
- [x] Tab2/Tab3/Tab4 Fragment 使用 Tab*Texts

### ✅ Phase 3: 增強 DetailActivity

- [x] 支援多段落顯示
- [x] 支援圖片顯示（ImageSlideshowView）
- [x] 支援內部導航連結（navigateToDictionarySettings）

### ✅ 額外完成

- [x] SetupGuideActivity（啟用方法頁面）
- [x] SettingsMainActivity 支援 EXTRA_START_TAB 外部導航
