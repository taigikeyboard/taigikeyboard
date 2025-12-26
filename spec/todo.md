# 待辦事項

> **關鍵字**: `todo`, `待辦`, `規劃`

---

## 當前待辦：Android 本地化重構

### Branch: develop-main-app

對齊 iOS 的本地化架構，將 Android 的 `AppTexts` 分離成 Tab1Texts ~ Tab4Texts。

---

## Phase 1: 分離本地化檔案 ✅ 已完成

| 項目 | 說明 | 狀態 |
|------|------|------|
| 建立 `localization/` 資料夾 | 新資料夾結構 | ✅ |
| 建立 `Tab1Texts.kt` | 頭頁文字（功能、問題、FAQ、版權） | ✅ |
| 建立 `Tab2Texts.kt` | 齒佈文字 | ✅ |
| 建立 `Tab3Texts.kt` | 詞庫文字 | ✅ |
| 建立 `Tab4Texts.kt` | 設定文字 | ✅ |
| 移動 `LocalizedText.kt` 核心部分 | 只保留 data class | ✅ |
| 移動 `LanguageManager.kt` | 語言管理（改用 StateFlow） | ✅ |
| 更新所有 import | 修正引用路徑 | ✅ |
| 刪除舊的 settings/ 本地化檔案 | 清理舊檔案 | ✅ |

---

## Phase 2: 同步 iOS 文字內容 ✅ 已完成

### Tab1 文字同步

| iOS Key | iOS 文字 | Android 狀態 | 備註 |
|---------|----------|--------------|------|
| `setupKeyboard` | 齒盤愛拍開才會當使用 | ✅ 已同步 | 區塊標題 |
| `newFeatures` | 最近做 ê 新功能 | ✅ 已同步 | |
| `knownIssues` | 當 leh 修理 ê 問題 | ✅ 已同步 | |
| `upcomingFeatures` | 未來安排欲做 ê 功能 | ✅ 已同步 | |
| `faq` | 捷問 ê 問題 | ✅ 已同步 | |

### 新功能內容同步

| 項目 | iOS 內容 | Android 狀態 |
|------|----------|--------------|
| `featureNextWord` | 連紲建議詞 | ✅ 一致 |
| `featureVariant` | 異用字開關 | ✅ 已同步 |
| `featureCustomFont` | 詞庫管理 | ✅ 已同步 |
| 支援多段落 | iOS 有 3 段落 | ✅ Android 已支援 |

### 處理中問題內容同步

| 項目 | iOS 內容 | Android 狀態 |
|------|----------|----------------|
| `issue1` | 自動大寫開關無一定有作用 | ✅ 已同步 |
| `issue2` | 標點符號無夠用 | ✅ 已同步 |
| `issue3` | 齒盤 ê 字小可仔閘到 | ✅ 已同步 |
| `issue4` | 詞庫有欠字 | ✅ 已同步 |
| `issue5` | 聲調轉換有 ê 字有問題 | ✅ 已同步 |

### 預計新功能內容同步

| 項目 | iOS 內容 | Android 狀態 |
|------|----------|----------------|
| `upcoming1` | 支持其他齒佈 | ✅ 已同步 |
| `upcoming2` | 自訂詞庫匯入匯出 | ✅ 已同步 |
| `upcoming3` | 連紲拍字 | ✅ 已同步 |

### FAQ 內容同步

| 項目 | iOS 內容 | Android 狀態 |
|------|----------|----------------|
| `faq1` | 齒盤裝好了後無出現 | ✅ 已同步 |
| `faq2` | 回報 ê 問題無消息 | ✅ 已同步 |
| `faq3` | 按怎拍聲調 1, 4 | ✅ 已同步 |

---

## Phase 3: 增強 DetailActivity

| 項目 | 說明 | 優先度 | 狀態 |
|------|------|--------|------|
| 支援多段落顯示 | `List<LocalizedText>` | 高 | ✅ 已完成 |
| 支援圖片顯示 | ImageSlideshowView 對應 | 中 | ⬜ |
| 支援導航連結 | NavigationLink 對應 | 中 | ⬜ |

---

## 已完成項目

### 本地化重構（已完成）

- ✅ 建立 `localization/` 資料夾
- ✅ 建立 `Tab1Texts.kt`（對應 iOS Tab1Texts.swift）
- ✅ 建立 `Tab2Texts.kt`（對應 iOS Tab2Texts.swift）
- ✅ 建立 `Tab3Texts.kt`（對應 iOS Tab3Texts.swift）
- ✅ 建立 `Tab4Texts.kt`（對應 iOS Tab4Texts.swift）
- ✅ 重構 `LocalizedText.kt`（只保留 data class）
- ✅ 重構 `LanguageManager.kt`（改用 StateFlow）
- ✅ 更新所有 import 引用
- ✅ 刪除舊的 settings/ 本地化檔案
- ✅ DetailActivity 支援多段落顯示
- ✅ Tab1 文字內容與 iOS 同步

### UI 一致性（已完成）

- ✅ Tab 結構對應（Tab1~Tab4）
- ✅ Tab 容器對應（TabView / BottomNavigationView）
- ✅ Dark Mode 顏色對應
- ✅ 版權頁面結構對應
- ✅ Onboarding 簡化對應
- ✅ 佈局預覽圖片對應
- ✅ Debug 模式對應
- ✅ 字體大小對應（Body 17sp, Section Header 17sp）
- ✅ surfaceSecondary 顏色修正
- ✅ BottomNavigationView 高度 80dp + icon 28dp
- ✅ Tab1 子頁面改用 DetailActivity（非 AlertDialog）

---

## 檔案對照（參考 spec/pages.md）

| iOS 檔案 | Android 檔案 | 狀態 |
|----------|--------------|------|
| `Tab1Texts.swift` | `Tab1Texts.kt` | ✅ 已建立 |
| `Tab2Texts.swift` | `Tab2Texts.kt` | ✅ 已建立 |
| `Tab3Texts.swift` | `Tab3Texts.kt` | ✅ 已建立 |
| `Tab4Texts.swift` | `Tab4Texts.kt` | ✅ 已建立 |
| `LocalizedText.swift` | `LocalizedText.kt` | ✅ 已重構 |

---

## 待辦項目

### 高優先度

- ⬜ 圖片顯示支援（DetailActivity ImageSlideshowView）
- ⬜ 導航連結支援（DetailActivity NavigationLink）

### 中優先度

- ⬜ Tab2/Tab3/Tab4 使用新的 Tab*Texts

---

## 注意事項

- iOS 使用 `enum Tab1Texts { static let ... }`
- Android 使用 `object Tab1Texts { val ... }`
- iOS 多段落使用 `[LocalizedText]`
- Android 使用 `List<LocalizedText>`
