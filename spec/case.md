# Case 大小寫處理

> **類型**: 功能
> **關鍵字**: `Case`, `Shift`, `CapsLock`, `CaseTransformationService`
> **相關**: kk10.md, composing.md

---

## 重點摘要

- 大小寫狀態由 KeyboardKit 管理
- 本專案透過設定同步控制行為
- 支援聲調字母正確轉換（`á` → `Á`）

---

## 核心機制

```
用戶操作（Shift / 句首）
    ↓
KeyboardKit 內部管理 keyboardCase
    ↓
CaseTransformationService.transformForInput()
    ↓
輸出字元
```

---

## 自動大寫設定

### isAutoCapitalizationEnabled

| 設定 | 鍵盤顯示 | 句首輸出 | 手動 Shift | Caps Lock |
|------|---------|---------|-----------|-----------|
| `true` | 句首大寫 | 大寫 | 大寫 | 大寫 |
| `false` | 永遠小寫 | 小寫 | 大寫 | 大寫 |

---

## KeyboardKit 設定同步

```swift
func syncToKeyboardContext(_ context: KeyboardContext) {
    context.settings.isAutocapitalizationEnabled = isAutoCapitalizationEnabled

    if !isAutoCapitalizationEnabled {
        context.autocapitalizationTypeOverride = .none
        if context.keyboardCase == .uppercased {
            context.keyboardCase = .lowercased
        }
    }
}
```

---

## 核心元件

| 元件 | 檔案 | 說明 |
|------|------|------|
| `CaseTransformationService` | `CaseTransformationService.swift` | 統一轉換入口 |
| `ToneUtilities` | `ToneUtilities.swift` | 聲調字母轉換 |
| `ToneMappings` | `ToneMappings.swift` | POJ/TL 對照表 |
| `SharedSettings` | `SharedSettings.swift` | 設定與同步 |

---

## KeyboardKit 10 已知問題

### 問題 1：切換鍵盤後 keyboardCase 被覆蓋

- **現象**：符號鍵盤切回字母鍵盤，有機率變大寫
- **Workaround**：監聽 keyboardCase 變化，恢復為小寫

### 問題 2：候選詞大小寫不跟隨

- **現象**：混合大小寫輸入時候選詞顯示錯誤
- **解法**：`SuggestionCaseTransformer` 在 View 層處理

---

## 平台差異

| 項目 | iOS | Android |
|------|-----|---------|
| 框架 | KeyboardKit | 自行管理 |
| 控制方式 | 設定同步 | `updateCapsState()` |
| 即時生效 | NotificationCenter | DataStore Flow |
