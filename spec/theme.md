# Theme 主題設計

> **類型**: 功能
> **關鍵字**: `Theme`, `Color`, `Style`, `SPY×FAMILY`
> **相關**: app-ui.md

---

## 重點摘要

- SPY×FAMILY 1960 年代復古美學
- 復古扁平設計：無陰影、使用邊框
- iOS App 支援 Light/Dark Mode
- Android App 僅 Light Mode

---

## 配色系統

### 主要顏色

| 名稱 | Light | Dark | 說明 |
|------|-------|------|------|
| `accent` | #8da99b | #8ba697 | 青灰綠（Loid） |
| `accentSecondary` | #610a10 | #d95959 | 深紅（Yor） |
| `accentTertiary` | #fab3ad | #fabeba | 粉紅（Anya） |

### 背景與表面

| 名稱 | Light | Dark |
|------|-------|------|
| `surfacePrimary` | #f5f0e8 | #1c1c1e |
| `surfaceSecondary` | #fbf9f5 | #2c2c2e |

### 文字

| 名稱 | Light | Dark |
|------|-------|------|
| `textPrimary` | #2c2827 | #ffffff |
| `textSecondary` | #57675c | #8e8e93 |

---

## 卡片樣式

| 屬性 | 值 | 說明 |
|------|-----|------|
| cornerRadius | 10dp/pt | 復古方正 |
| elevation | 0 | 無陰影 |
| strokeWidth | 1dp/pt | 細邊框 |
| background | surfaceSecondary | 米白色 |

---

## 左側色條設計

- 寬度：4dp
- 配色邏輯：
  - 青灰綠：主要功能
  - 深紅：重要功能
  - 粉色：溫暖點綴

---

## 平台對照

| 項目 | iOS | Android |
|------|-----|---------|
| 顏色定義 | `ThemeTokens.swift` | `colors.xml` |
| 主題樣式 | View+Theme.swift | `themes.xml` |
| Dark Mode | 支援 | 不支援（App） |

---

## 設計哲學

### 避免現代風格

- ❌ 大量陰影
- ❌ 浮動卡片效果
- ❌ 過度圓潤圓角
- ❌ 統一 icon circles

### 改用復古元素

- ✅ 扁平設計 + 細邊框
- ✅ 左側色條（檔案夾風格）
- ✅ 虛線分隔線
- ✅ 左對齊標題
