# Device 裝置自適應

> **類型**: 功能
> **關鍵字**: `Device`, `iPhone`, `iPad`, `ScreenSizeClass`
> **相關**: layout.md

---

## 重點摘要

- 裝置分級 + 預定義常數（非線性縮放）
- 4 個尺寸分級：phoneCompact / phoneRegular / phoneLarge / pad
- 候選詞字體根據分級調整

---

## 螢幕尺寸分級

| 分級 | 寬度條件 | 代表裝置 |
|------|----------|----------|
| `phoneCompact` | < 375pt | iPhone SE, mini |
| `phoneRegular` | 375-413pt | iPhone 14, 15 |
| `phoneLarge` | ≥ 414pt | iPhone Plus/Max |
| `pad` | iPad | iPad |

---

## 字體大小配置

| 屬性 | compact | regular | large | pad |
|------|---------|---------|-------|-----|
| primaryFontSize | 20 | 21 | 20 | 23 |
| secondaryFontSize | 15 | 16 | 15 | 17 |
| longCellPrimaryFontSize | 19 | 20 | 19 | 22 |
| longCellSecondaryFontSize | 13 | 14 | 13 | 15 |

---

## 影響範圍

| 元件 | 檔案 |
|------|------|
| 候選詞列 | `CandidateView.swift` |
| 展開網格 | `ExpandedCandidateOverlay.swift` |

### 不受影響

- 按鍵文字：KeyboardKit 自動處理
- Callout：KeyboardKit 標準樣式

---

## 測試裝置

### 必測機型

| 分級 | 裝置 | 寬度 |
|------|------|------|
| phoneCompact | iPhone 13 mini | 375pt |
| phoneRegular | iPhone 15 | 393pt |
| phoneLarge | iPhone 15 Pro Max | 430pt |
| pad | iPad Pro 11" | 834pt |

---

## 螢幕尺寸參考

| 裝置 | 邏輯寬度 | 分級 |
|------|----------|------|
| iPhone SE (3rd) | 375pt | phoneRegular |
| iPhone 13 mini | 375pt | phoneCompact |
| iPhone 14/15 | 390-393pt | phoneRegular |
| iPhone Plus/Max | 428-430pt | phoneLarge |
