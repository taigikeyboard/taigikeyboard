# Hamster 與 KeyboardKit 大小寫實作分析

本文件整理 Hamster 專案與 KeyboardKit 9.9.0/文檔關於大小寫轉換的實作方式，作為改善 `isAutoCapitalizationEnabled` 的參考。

---

## 一、Hamster 大小寫轉換實作

### 1.1 KeyboardCase 定義

**檔案**: `Hamster/Packages/HamsterKeyboardKit/Sources/KeyboardKit/Casing/KeyboardCase.swift`

```swift
public enum KeyboardCase: String, Codable, Identifiable, Hashable {
    case auto          // 瞬時態，會自動替換為合適的狀態
    case capsLocked    // 大寫鎖定，不自動調整
    case lowercased    // 小寫
    case uppercased    // 大寫
}

// 狀態檢查
var isLowercased: Bool   // true 只有在 .lowercased
var isUppercased: Bool   // true 在 .uppercased 或 .capsLocked
```

### 1.2 核心屬性

**檔案**: `KeyboardContext.swift`

| 屬性 | 類型 | 說明 |
|------|------|------|
| `lockShiftState` | `Bool` | 控制是否鎖定 Shift 狀態（預設 true） |
| `isAutoCapitalizationEnabled` | `Bool` | 自動大寫開啟/關閉 |

### 1.3 自動大寫觸發邏輯

**檔案**: `KeyboardContext+KeyboardType.swift`

```swift
var preferredKeyboardType: KeyboardType {
    // 優先級 1: CapsLock 已鎖定 → 保持當前狀態
    if keyboardType.isAlphabetic(.capsLocked) { return keyboardType }
    if keyboardType.isChinesePrimaryKeyboard(.capsLocked) { return keyboardType }

    // 優先級 2: Shift 狀態被鎖定 → 保持當前狀態
    guard !lockShiftState else { return keyboardType }

    // 優先級 3: 根據文本內容觸發自動大寫
    if keyboardType.isAlphabetic {
        if let type = preferredAutocapitalizedKeyboardType { return type }
        // ... 其他邏輯
    }
}

private var preferredAutocapitalizedKeyboardType: KeyboardType? {
    // 關鍵：先檢查開關
    guard isAutoCapitalizationEnabled else { return nil }
    guard let proxyType = autocapitalizationType else { return nil }
    guard keyboardType.isAlphabetic else { return nil }

    let uppercased = KeyboardType.alphabetic(.uppercased)
    let lowercased = KeyboardType.alphabetic(.lowercased)

    switch proxyType {
    case .allCharacters:
        return uppercased
    case .sentences:
        return textDocumentProxy.isCursorAtNewSentenceWithTrailingWhitespace
            ? uppercased : lowercased
    case .words:
        return textDocumentProxy.isCursorAtNewWord
            ? uppercased : lowercased
    default:
        return lowercased
    }
}
```

### 1.4 Shift 鍵行為

**檔案**: `KeyboardAction+Actions.swift`

```swift
case .shift(let currentState): return {
    switch currentState {
    case .lowercased:
        $0?.setKeyboardCase(.uppercased)    // 小寫 → 大寫
    case .auto, .capsLocked, .uppercased:
        $0?.setKeyboardCase(.lowercased)    // 大寫/自動/鎖定 → 小寫
    }
}
```

**雙擊檢測** (`StandardKeyboardBehavior.swift`):

```swift
private var isDoubleShiftTap: Bool {
    let date = Date().timeIntervalSinceReferenceDate
    let lastDate = lastShiftCheck.timeIntervalSinceReferenceDate
    let isDoubleTap = (date - lastDate) < doubleTapThreshold  // 預設 0.5 秒
    lastShiftCheck = isDoubleTap ? Date().addingTimeInterval(-1) : Date()
    return isDoubleTap
}

open func shouldSwitchToCapsLock(
    after gesture: KeyboardGesture,
    on action: KeyboardAction
) -> Bool {
    switch action {
    case .shift: return isDoubleShiftTap
    default: return false
    }
}
```

### 1.5 切換鍵盤時的狀態處理

**問題點**：切換到符號鍵盤時，沒有明確處理大小寫狀態

```swift
// KeyboardContext+KeyboardType.swift
var preferredKeyboardType: KeyboardType {
    // 中文鍵盤：大寫 → 小寫
    if keyboardType.isChinesePrimaryKeyboard,
       keyboardType.isChinesePrimaryKeyboard(.uppercased) {
        return .chinese(.lowercased)
    }

    // 自訂鍵盤：大寫 → 小寫
    if keyboardType.isCustom,
       case .custom(let named, let current) = keyboardType,
       current.isUppercased {
        return .custom(named: named, case: .lowercased)
    }

    // ⚠️ 符號鍵盤 (.classifySymbolic, .numeric, .symbolic) 沒有處理
    return keyboardType
}
```

### 1.6 鍵盤類型切換實現

**檔案**: `KeyboardInputViewController.swift`

```swift
open func setKeyboardCase(_ casing: KeyboardCase) {
    if keyboardContext.keyboardType.isChinesePrimaryKeyboard {
        keyboardContext.setKeyboardType(.chinese(casing))
        return
    }
    if case .custom(let name, _) = keyboardContext.keyboardType {
        keyboardContext.setKeyboardType(.custom(named: name, case: casing))
        return
    }
    keyboardContext.setKeyboardType(.alphabetic(casing))
}

func tryChangeToPreferredKeyboardTypeAfterTextDidChange() {
    let context = keyboardContext
    let shouldSwitch = keyboardBehavior.shouldSwitchToPreferredKeyboardTypeAfterTextDidChange()
    guard shouldSwitch else { return }
    setKeyboardType(context.preferredKeyboardType)
}
```

---

## 二、KeyboardKit 9.9.0 關閉大寫開關實作

### 2.1 isAutocapitalizationEnabled 定義

**檔案**: `KeyboardSettings.swift`

```swift
@AppStorage("\(settingsPrefix)isAutocapitalizationEnabled", store: .keyboardSettings)
public var isAutocapitalizationEnabled = true {
    didSet { onAutocapitalizationEnabledChanged() }
}
```

- 持久化儲存（`@AppStorage`）
- 預設值 `true`
- 變更時觸發回調

### 2.2 兩層防護機制

**第一層：preferredAutocapitalizedCase**

```swift
// KeyboardContext+KeyboardCase.swift
private extension KeyboardContext {
    var preferredAutocapitalizedCase: Keyboard.KeyboardCase? {
        // 關鍵：第一個檢查
        guard settings.isAutocapitalizationEnabled else { return nil }
        guard let autocapitalizationType else { return nil }
        if locale.isRightToLeft { return .lowercased }

        switch autocapitalizationType {
        case .allCharacters: return .uppercased
        case .sentences:
            return textDocumentProxy.shouldApplySentenceAutocapitalization
                ? .uppercased : .lowercased
        case .words:
            return textDocumentProxy.shouldApplyWordAutocapitalization
                ? .uppercased : .lowercased
        default: return .lowercased
        }
    }
}
```

**第二層：preferredKeyboardCase**

```swift
public extension KeyboardContext {
    var preferredKeyboardCase: Keyboard.KeyboardCase {
        if keyboardCase == .capsLocked { return .capsLocked }
        if let val = preferredAutocapitalizedCase { return val }
        return keyboardCase
    }
}
```

### 2.3 同步機制

**檔案**: `KeyboardContext+Sync.swift`

```swift
extension KeyboardContext {
    func syncAutocapitalizationWithSetting() {
        let noAutocap = Keyboard.AutocapitalizationType.none
        let value = settings.isAutocapitalizationEnabled ? nil : noAutocap
        if autocapitalizationTypeOverride != value {
            autocapitalizationTypeOverride = value
        }
    }
}
```

當 `isAutocapitalizationEnabled = false` 時：
- 設定 `autocapitalizationTypeOverride = .none`
- 覆蓋 UITextDocumentProxy 的自動大寫設定

### 2.4 Shift 的標準處理

**檔案**: `Keyboard+StandardKeyboardBehavior.swift`

```swift
open func preferredKeyboardCase(
    after gesture: Gesture,
    on action: KeyboardAction
) -> Keyboard.KeyboardCase {
    let current = keyboardContext.keyboardCase
    switch action {
    case .shift:
        guard gesture == .release else { return current }
        return isDoubleShiftTap ? .capsLocked : current
    default:
        return keyboardContext.preferredKeyboardCase
    }
}
```

---

## 三、KeyboardKit 文檔重點

### 3.1 KeyboardCase 官方定義

| Case | 說明 |
|------|------|
| `.uppercased` | 大寫狀態（按一次 shift） |
| `.lowercased` | 小寫狀態（預設） |
| `.capsLocked` | Caps Lock 狀態（雙點擊 shift） |

**重要屬性**：
- `isUppercasedOrCapslocked: Bool` - 檢查是否為大寫或 Caps Lock 狀態

### 3.2 AutocapitalizationType

| Case | 說明 |
|------|------|
| `.none` | 不自動大寫 |
| `.words` | 每個字開頭自動大寫 |
| `.sentences` | 每個句子開頭自動大寫 |
| `.allCharacters` | 所有字元自動大寫 |

### 3.3 KeyboardContext 核心屬性

| 屬性 | 說明 |
|------|------|
| `keyboardCase` | 當前大小寫狀態（@Published） |
| `autocapitalizationType` | 來自文字欄位的自動大寫類型 |
| `autocapitalizationTypeOverride` | 覆蓋自動大寫行為 |
| `preferredKeyboardCase` | 計算的優先大小寫（唯讀） |

### 3.4 相關設定

| 設定 | 說明 |
|------|------|
| `isAutocapitalizationEnabled` | 是否啟用自動大寫功能 |
| `isDoubleTapOnShiftToCapsLockEnabled` | 是否雙點擊 shift 啟用 Caps Lock |

### 3.5 Shift 按鈕設計要點

> Shift 按鈕需要攜帶當前 KeyboardCase，以確保在上下文狀態改變時正確更新鍵盤按鍵顯示。

```swift
case shift(Keyboard.KeyboardCase)
```

---

## 四、關鍵發現與問題分析

### 4.1 大小寫轉換流程

```
用戶操作
    ↓
Shift 單擊: .lowercased ↔ .uppercased
Shift 雙擊: → .capsLocked
    ↓
setKeyboardCase() → setKeyboardType()
    ↓
preferredKeyboardType (自動大寫規則)
    ↓
視圖更新 (KeyboardCase 映射圖示)
```

### 4.2 切換符號鍵盤問題

根據 git log (`切換到符號鍵盤會跑回大寫`)，問題可能出在：

1. **符號鍵盤沒有大小寫概念**，但邏輯沒有明確處理「切換回來時恢復狀態」
2. **依賴 `lockShiftState` 和自動大寫規則的交互**
3. **`tryChangeToPreferredKeyboardTypeAfterTextDidChange()` 的觸發時機**可能在切回時重新套用自動大寫

### 4.3 Hamster vs KeyboardKit 9.9.0 差異

| 項目 | Hamster | KeyboardKit 9.9.0 |
|------|---------|-------------------|
| 開關檢查位置 | `preferredAutocapitalizedKeyboardType` | `preferredAutocapitalizedCase` |
| 鎖定機制 | `lockShiftState` 屬性 | 無對應屬性 |
| 符號鍵盤處理 | 未明確處理 | 未明確處理 |
| CapsLock 優先 | 最高優先，不受影響 | 最高優先，不受影響 |

---

## 五、相關檔案路徑

### Hamster

| 檔案 | 功能 |
|------|------|
| `Casing/KeyboardCase.swift` | 大小寫狀態定義 |
| `Casing/KeyboardCase+Button.swift` | Shift 按鍵圖示映射 |
| `Keyboard/KeyboardContext.swift` | 鍵盤狀態上下文 |
| `Keyboard/KeyboardContext+KeyboardType.swift` | **自動大寫邏輯** |
| `Keyboard/KeyboardType.swift` | 鍵盤類型定義 |
| `Keyboard/StandardKeyboardBehavior.swift` | **Shift 雙擊邏輯** |
| `Actions/KeyboardAction+Actions.swift` | **Shift 單擊邏輯** |
| `Actions/StandardKeyboardActionHandler.swift` | Action 處理 |
| `Controller/KeyboardInputViewController.swift` | Controller 實現 |

### KeyboardKit 9.9.0

| 檔案 | 功能 |
|------|------|
| `_Keyboard/KeyboardContext.swift` | 上下文定義 |
| `_Keyboard/KeyboardSettings.swift` | `isAutocapitalizationEnabled` 定義 |
| `_Keyboard/Models/Keyboard+Case.swift` | KeyboardCase 定義 |
| `_Keyboard/KeyboardContext+KeyboardCase.swift` | `preferredKeyboardCase` 邏輯 |
| `_Keyboard/KeyboardContext+Sync.swift` | 同步邏輯 |
| `_Keyboard/Keyboard+StandardKeyboardBehavior.swift` | Shift/CapsLock 行為 |
| `Actions/Services/KeyboardAction+StandardActionHandler.swift` | ActionHandler 實作 |

---

## 六、改善方向建議

1. **追蹤符號鍵盤切換前的大小寫狀態**
   - 儲存切換前的 `keyboardCase`
   - 切回時根據 `isAutoCapitalizationEnabled` 決定是否恢復

2. **確認 `isManualShiftActive` 在切換鍵盤時的行為**
   - 檢查旗標是否被錯誤重置

3. **檢查 `preferredKeyboardType` 的調用時機**
   - 從符號鍵盤切回時，是否不應套用自動大寫規則

4. **參考 Hamster 的 `lockShiftState` 機制**
   - 考慮增加類似的鎖定狀態控制
