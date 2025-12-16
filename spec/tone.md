# 聲調處理

## 概述

台語有 8 個聲調，其中聲調 1 和 4 沒有調號標記（視為無聲調）。本功能處理聲調輸入、顯示、搜尋及修正。

---

## 聲調對照表

| 聲調 | 調號 | POJ 範例 | TL 範例 | 說明 |
|------|------|----------|---------|------|
| 1 | (無) | gua | gua | 陰平，無調號 |
| 2 | ́ (acute) | guá | guá | 陰上 |
| 3 | ̀ (grave) | guà | guà | 陰去 |
| 4 | (無) | gua | gua | 陰入，無調號 |
| 5 | ̂ (circumflex) | guâ | guâ | 陽平 |
| 6 | ̌ (caron) | guǎ | guǎ | 陽上 |
| 7 | ̄ (macron) | guā | guā | 陽去 |
| 8 | ̍ (vertical line) | gua̍ | gua̍ | 陽入 |
| 9 | ̆ / ̋ | guă | gua̋ | 輕聲（POJ: breve / TL: double acute） |

---

## 平台實作

### Android

```kotlin
// ToneConverter.kt
object ToneConverter {
    fun convertToToneMarks(input: String, mode: InputMode): String {
        val preprocessed = preprocess(input, mode)
        return when (mode) {
            InputMode.POJ -> convertPOJ(preprocessed)
            InputMode.TL -> convertTL(preprocessed)
        }
    }
}

// ToneConverterModels.kt
object ToneConverterModels {
    val pojNumberToToneMapping = mapOf("a2" to "á", ...)
    val tlNumberToToneMapping = mapOf("a2" to "á", ...)
    val pojToneMapping = mapOf("á" to "a", ...)  // 還原用
    val tlToneMapping = mapOf("á" to "a", ...)
}
```

### iOS

```swift
// ToneConverter.swift（統一介面）
enum ToneConverter {
    static func convertToToneMarks(_ input: String, mode: InputMode) -> String
    static func restoreTone(_ text: String, mode: InputMode) -> String?
}

// POJToneConverter.swift / TLToneConverter.swift（模式實作）
enum POJToneConverter {
    static func convert(_ input: String) -> String
    static func preprocess(_ input: String) -> String
}

// ToneMappings.swift（映射表）
enum ToneMappings {
    static let pojNumberToTone: [String: String]
    static let tlNumberToTone: [String: String]
    static let pojToneToBase: [String: String]  // 還原用
    static let tlToneToBase: [String: String]
}

// ToneRestoration.swift（還原邏輯）
enum ToneRestoration {
    static func restore(_ text: String, mode: InputMode) -> String?
}
```

### 平台一致性

| 項目 | Android | iOS |
|------|---------|-----|
| 架構 | 單一物件 | 分層（介面/實作/映射） |
| 聲調 1, 4 處理 | 不標記 | 不標記 |
| POJ `oo` → `o͘` | 支援 | 支援 |
| POJ `nn` → `ⁿ` | 支援 | 支援 |
| TL `oo` 處理 | 保持，第一個 o 加調 | 保持，第一個 o 加調 |
| 聲調還原 | `attemptToneRestoration()` | `ToneRestoration.restore()` |
| 映射表內容 | 完全一致 | 完全一致 |

---

## 功能一：聲調輸入與顯示

### 處理邏輯

| 輸入 | rawInput | composingText (顯示) |
|------|----------|----------------------|
| `gua2` | `gua2` | `guá` |
| `gua1` | `gua1` | `gua` |
| `gua4` | `gua4` | `gua` |

### 實作重點

```kotlin
// Android: ComposingManager.appendCharacter()
if (toneNumber in 1..9) {
    newText = applyToneConversion(newText, toneNumber) ?: newText
}

// ToneConverter.convertPOJSyllable() / convertTLSyllable()
if (toneNumber == 0 || toneNumber == 1 || toneNumber == 4) {
    return baseForm  // 無調號，移除數字
}
```

```swift
// iOS: ComposingManager.appendCharacter()
if let number = Int(char), (1...9).contains(number) {
    if let toneConvertedText = applyToneConversion(...) {
        finalDisplayText = toneConvertedText
    }
}

// POJToneConverter.convertSyllable() / TLToneConverter.convertSyllable()
if toneNumber == 0 || toneNumber == 1 || toneNumber == 4 {
    return baseForm  // 無調號，移除數字
}
```

---

## 功能二：搜尋正規化

### 處理邏輯

**檔案**：`InputNormalizer.kt` / `InputNormalizer.swift`

| rawInput | 正規化結果 | 說明 |
|----------|------------|------|
| `gua1` | `gua1` | 保留聲調 1（用於搜尋無聲調詞彙） |
| `gua4` | `gua` | 移除聲調 4（陰入由韻尾區分） |
| `gua2` | `gua2` | 保留聲調 2 |
| `gua42` | `gua2` | 移除 4，保留 2 |

### 實作重點

```kotlin
// Android: InputNormalizer.normalizeSyllable()
// 移除聲調 4（陰入，由入聲韻尾區分）
// 聲調 1 保留（用於搜尋無聲調詞彙）
val withoutTone4 = syllable.replace("4", "")
```

```swift
// iOS: InputNormalizer.normalizeSyllable()
// 移除聲調 4（陰入，由入聲韻尾區分）
// 聲調 1 保留（用於搜尋無聲調詞彙）
let withoutTone4 = syllable.replacingOccurrences(of: "4", with: "")
```

---

## 功能三：聲調重選（刪除後重新輸入）

### 使用情境

使用者輸入 `gua2`（顯示 `guá`），想改成聲調 5：
1. 按 Backspace → 刪除聲調，顯示 `gua`
2. 輸入 `5` → 顯示 `guâ`

### 處理邏輯

```
刪除前：rawInput = "gua2", composingText = "guá"
刪除後：rawInput = "gua",  composingText = "gua"
輸入 5：rawInput = "gua5", composingText = "guâ"
```

### 實作重點

```kotlin
// Android: ComposingManager.deleteBackward()
fun deleteBackward(ic: InputConnection): Boolean {
    // 嘗試聲調還原
    val restoredText = attemptToneRestoration()
    if (restoredText != null) {
        composingText = restoredText      // guá → gua
        rawInput = rawInput.dropLast(1)   // gua2 → gua
        return true
    }
    // 一般字元刪除...
}
```

```swift
// iOS: ComposingManager.deleteBackward()
func deleteBackward() {
    // 先嘗試聲調還原（composingText）
    if let restoredText = attemptToneRestoration() {
        // rawInput 刪除最後一個字元（聲調數字）
        let newRawInput = String(rawInput.dropLast())
        // 更新為還原後的文字
        updateComposingState(.composing(raw: newRawInput, display: restoredText))
        return
    }
    // 一般字元刪除...
}
```

### 聲調還原邏輯

```kotlin
// Android: attemptToneRestoration()
// 使用 ToneConverterModels.pojToneMapping / tlToneMapping
// "á" → "a", "ó͘" → "o͘", "n̂" → "n"
```

```swift
// iOS: ToneRestoration.restore()
// 使用 ToneMappings.pojToneToBase / tlToneToBase
// "á" → "a", "ó͘" → "o͘", "n̂" → "n"
```

---

## 功能四：聲調修正（連續輸入）

### 使用情境

使用者輸入 `gua12`（先按 1 再按 2）：
- 顯示：`guá`（2 覆蓋 1）
- 搜尋：`gua2`（1 被移除）

### 處理邏輯

| 操作 | rawInput | composingText | 搜尋正規化 |
|------|----------|---------------|------------|
| 輸入 `gua` | `gua` | `gua` | `gua` |
| 輸入 `1` | `gua1` | `gua` | `gua` |
| 輸入 `2` | `gua12` | `guá` | `gua2` |

---

## 功能五：鼻化音刪除

### 問題

POJ 模式下，`nn` 會轉換為 `ⁿ`（1 個 Unicode 字元 U+207F）。刪除時需要同步處理：

| composingText | rawInput | Unicode 長度 |
|---------------|----------|--------------|
| `ⁿ` | `nn` | 1 vs 2 |

### 處理邏輯

```
刪除前：rawInput = "Phiann", composingText = "Phiaⁿ"
刪除後：rawInput = "Phia",   composingText = "Phia"
```

### 實作重點

```kotlin
// Android: ComposingManager.deleteBackward()
val lastChar = composingText.lastOrNull()
// ⁿ 對應 rawInput 的 nn（2 個字元）
val rawDeleteCount = if (lastChar == 'ⁿ') 2 else 1

composingText = composingText.dropLast(1)
rawInput = rawInput.dropLast(rawDeleteCount)
```

```swift
// iOS: ComposingManager.deleteBackward()
let lastChar = composingText.last
// ⁿ 對應 rawInput 的 nn（2 個字元）
let rawDeleteCount = (lastChar == "ⁿ") ? 2 : 1

let newDisplayText = String(composingText.dropLast())
let newRawInput = String(rawInput.dropLast(rawDeleteCount))
```

---

## 功能六：字元組合轉換

### POJ 模式限定

| 輸入 | 轉換 | Unicode | 說明 |
|------|------|---------|------|
| `oo` | `o͘` | U+006F + U+0358 | POJ 母音 |
| `nn` | `ⁿ` | U+207F | 鼻化音 |

### 實作重點

```kotlin
// Android: ToneConverter.preprocess()
if (mode == InputMode.POJ) {
    result = result.replace("oo", "o͘")
    result = result.replace("Oo", "O͘")
    result = result.replace("OO", "O͘")
    result = convertNN(result)
}
```

```swift
// iOS: POJToneConverter.preprocess()
if settings.enableDoubleTapOO {
    result = result.replacingOccurrences(of: "oo", with: "o͘")
    result = result.replacingOccurrences(of: "Oo", with: "O͘")
    result = result.replacingOccurrences(of: "OO", with: "O͘")
}
if settings.enableDoubleTapNN {
    result = convertNN(result)
}
```

**TL 模式**：不做轉換，保持 `oo`（第一個 o 加調符）

---

## 功能七：母音定位

聲調標記需要放在正確的母音上。POJ 和 TL 有不同的規則。

### POJ 母音規則

1. 找到最後一個母音
2. 若有複合母音（雙母音），根據複雜規則選擇標記位置
3. 若無母音，標記在半母音（ng, n, m）上

```kotlin
// Android: ToneConverter.findPOJVowelRange()
private fun findPOJVowelRange(syllable: String): IntRange?
```

```swift
// iOS: VowelAnalyzer.findPOJVowelRange()
static func findPOJVowelRange(in syllable: String) -> Range<String.Index>?
```

### TL 母音規則

優先順序：
1. `a`
2. `oo`
3. 其他單母音（i, u, o, e）
4. 半母音（ng, n, m）

```kotlin
// Android: ToneConverter.findTLVowelRange()
private fun findTLVowelRange(syllable: String): IntRange?
```

```swift
// iOS: VowelAnalyzer.findTLVowelRange()
static func findTLVowelRange(in syllable: String) -> Range<String.Index>?
```

---

## 相關檔案

| 平台 | 檔案 | 職責 |
|------|------|------|
| Android | `ToneConverter.kt` | 數字聲調 → 調號轉換 |
| Android | `ToneConverterModels.kt` | 調號映射表、InputMode 列舉 |
| Android | `ToneCharacterUtils.kt` | 鍵盤彈出選單用聲調字元 |
| Android | `ComposingManager.kt` | 組字狀態管理、聲調刪除還原 |
| Android | `InputNormalizer.kt` | 搜尋用正規化（移除聲調 1/4） |
| iOS | `ToneConverter.swift` | 統一介面 |
| iOS | `POJToneConverter.swift` | POJ 模式轉換 |
| iOS | `TLToneConverter.swift` | TL 模式轉換 |
| iOS | `ToneMappings.swift` | 調號映射表 |
| iOS | `ToneRestoration.swift` | 聲調還原（Backspace 用） |
| iOS | `ToneUtilities.swift` | 聲調工具函數 |
| iOS | `ComposingManager.swift` | 組字狀態管理 |
| iOS | `InputNormalizer.swift` | 搜尋用正規化 |

---

## 測試案例

### 顯示測試

| 輸入序列 | 預期顯示 (POJ) | 預期顯示 (TL) |
|----------|----------------|---------------|
| `g` `u` `a` `1` | `gua` | `gua` |
| `g` `u` `a` `4` | `gua` | `gua` |
| `g` `u` `a` `2` | `guá` | `guá` |
| `g` `u` `a` `1` `2` | `guá` | `guá` |
| `g` `u` `a` `4` `2` | `guá` | `guá` |
| `h` `o` `o` `2` | `hó͘` | `hóo` |
| `g` `u` `a` `9` | `guă` | `gua̋` |

### 刪除測試

| 初始狀態 | 按 Backspace | 預期結果 |
|----------|--------------|----------|
| `guá` (rawInput: `gua2`) | 1 次 | `gua` (rawInput: `gua`) |
| `gua` (rawInput: `gua1`) | 1 次 | `gu` (rawInput: `gu`) |
| `Phiaⁿ` (rawInput: `Phiann`) | 1 次 | `Phia` (rawInput: `Phia`) |
| `hó͘` (rawInput: `hoo2`) | 1 次 | `ho͘` (rawInput: `hoo`) |

### 搜尋測試

| rawInput | 正規化結果 | 說明 |
|----------|------------|------|
| `gua1` | `gua1` | 保留聲調 1 |
| `gua4` | `gua` | 移除聲調 4 |
| `gua2` | `gua2` | 保留聲調 2 |
| `gua42` | `gua2` | 移除 4，保留 2 |
| `hoo1-bo5` | `hoo1boo5` | 保留聲調 1 和 5 |
