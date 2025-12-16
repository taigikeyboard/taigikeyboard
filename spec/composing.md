# 雙狀態組字模型

## 概述

台語鍵盤使用「雙狀態組字模型」，維護兩種不同用途的文字狀態：

| 狀態 | 用途 | 範例 |
|------|------|------|
| **rawInput** | Trie 搜尋用（數字聲調） | `gua2` |
| **composingText** | UI 顯示用（調符） | `guá` |

這樣設計的原因：
1. **Trie 索引使用數字聲調**：`tl:gua2`、`poj:goa2`
2. **使用者期望看到調符**：輸入 `gua2` 時顯示 `guá`

---

## 平台實作

### Android

```kotlin
// ComposingManager.kt
private var rawInput: String = ""      // 搜尋用：gua2
private var composingText: String = "" // 顯示用：guá

fun appendCharacter(char: String, ic: InputConnection) {
    // rawInput 保留原始 ASCII，不做字元組合
    rawInput = rawInput + char

    // composingText 做完整轉換（聲調、字元組合）
    composingText = applyToneConversion(composingText + char)

    ic.setComposingText(composingText, 1)
}

fun getRawInput(): String? = if (isComposing) rawInput else null
```

**搜尋流程**：
```
TaigiAutocompleteService.getSuggestions(rawInput, composingText)
    ↓
LexiconService.search(rawInput)
    ↓
InputNormalizer.normalize(rawInput)  // gua2 → gua2（幾乎不變）
```

### iOS

```swift
// ComposingManager.swift
private enum ComposingState {
    case idle
    case composing(raw: String, display: String)  // 雙狀態
}

@Published public private(set) var rawInput: String = ""      // 搜尋用：gua2
@Published public private(set) var composingText: String = "" // 顯示用：guá

func appendCharacter(_ char: String) {
    // rawInput 保留原始 ASCII，不做字元組合
    let newRawInput = rawInput + char

    // composingText 做完整轉換（聲調、字元組合）
    var finalDisplayText = composingText + char
    finalDisplayText = checkCharacterCombination(...)
    finalDisplayText = applyToneConversion(...)

    updateComposingState(.composing(raw: newRawInput, display: finalDisplayText))
}
```

**搜尋流程**：
```
AutocompleteService.autocomplete(text)
    ↓
composingManager.rawInput           // 直接取得 gua2
    ↓
LexiconService.search(rawInput)
    ↓
InputNormalizer.normalize(rawInput)  // gua2 → gua2（幾乎不變）
```

### 平台一致性

| 項目 | Android | iOS |
|------|---------|-----|
| rawInput 維護 | 顯式 | 顯式 |
| 搜尋輸入來源 | `rawInput` | `rawInput` |
| InputNormalizer 負擔 | 輕（數字→數字） | 輕（數字→數字） |

兩平台現已採用相同的雙狀態架構。

---

## 核心操作

### 1. 追加字元（appendCharacter）

```
輸入 'g': rawInput="g",    composingText="g"
輸入 'u': rawInput="gu",   composingText="gu"
輸入 'a': rawInput="gua",  composingText="gua"
輸入 '2': rawInput="gua2", composingText="guá"
```

**處理步驟**：
1. 更新 rawInput（保留原始 ASCII）
2. 檢查字元組合（oo → o͘, nn → ⁿ）
3. 檢查聲調轉換（數字 → 調符）
4. 更新 composingText
5. 同步到輸入框

### 2. 刪除字元（deleteBackward）

```
刪除前: rawInput="gua2", composingText="guá"
刪除後: rawInput="gua",  composingText="gua"
```

**處理步驟**：
1. 嘗試聲調還原（`guá` → `gua`）
2. 若成功：rawInput 刪除最後一個數字，composingText 更新為還原後文字
3. 若失敗：兩者都刪除最後一個字元
4. 特殊處理：`ⁿ` 對應 rawInput 的 `nn`（2 個字元）

### 3. 確認組字（commitComposition）

```kotlin
// Android
fun commitComposition(ic: InputConnection) {
    rawInput = ""
    composingText = ""
    isComposing = false
    ic.finishComposingText()  // 提交當前 composingText
}
```

```swift
// iOS
func commitComposition() {
    let textToInsert = composingText
    updateComposingState(.idle)
    keyboardViewController?.textDocumentProxy.insertText(textToInsert)
}
```

### 4. 選擇候選詞（selectSuggestion）

```kotlin
// Android
fun selectSuggestion(suggestion: String, ic: InputConnection) {
    rawInput = ""
    composingText = ""
    isComposing = false
    ic.setComposingText(suggestion, 1)
    ic.finishComposingText()
}
```

---

## 字元組合轉換

### POJ 模式限定

| 輸入 | 轉換 | 說明 |
|------|------|------|
| `oo` | `o͘` | POJ 母音（U+006F + U+0358） |
| `nn` | `ⁿ` | 鼻化音（U+207F） |

**TL 模式**：不做轉換，保持 `oo`、`nn`

### rawInput 處理

Android 的 rawInput **不做字元組合轉換**，保留原始 ASCII：
- 使用者輸入 `hoo` → rawInput = `hoo`（不是 `ho͘`）
- 這樣 Trie 可以直接用 `poj:hoo2` 格式搜尋

---

## 聲調處理

### 聲調 1 和 4 特殊處理

| 聲調 | 說明 | 處理方式 |
|------|------|----------|
| 1 | 陰平，無調號 | 移除數字，不加調符 |
| 4 | 陰入，無調號 | 移除數字，不加調符 |
| 2,3,5,6,7,8,9 | 有調號 | 移除數字，加調符 |

```
輸入 gua1: rawInput="gua1", composingText="gua"（無調符）
輸入 gua2: rawInput="gua2", composingText="guá"（有調符）
輸入 gua4: rawInput="gua4", composingText="gua"（無調符）
```

### 聲調修正（連續輸入）

使用者可以連續輸入多個聲調數字：
```
輸入 gua12: rawInput="gua12", composingText="guá"
```

搜尋時 InputNormalizer 會處理：
```
gua12 → gua2（移除聲調 1）
gua42 → gua2（移除聲調 4）
```

### 聲調還原（刪除時）

當使用者刪除聲調時，需要還原為基本字元：

```swift
// iOS: ToneConverter.restoreTone()
// Android: attemptToneRestoration()

// 使用 toneMapping 表對照
"á" → "a"
"ó͘" → "o͘"  // POJ
"n̂" → "n"
```

---

## 候選詞選擇

### selectedCandidateIndex

兩平台都維護選中的候選詞索引，支援空白鍵循環選擇：

```kotlin
// Android
var selectedCandidateIndex: Int = 0

fun moveToNextCandidate(totalCandidates: Int): Boolean {
    selectedCandidateIndex = (selectedCandidateIndex + 1) % totalCandidates
    return true
}
```

### iOS 候選詞結構

iOS 第 0 個位置是當前組字文字：
```
索引 0: 組字文字（如 guá）
索引 1: 候選詞 1（如 我）
索引 2: 候選詞 2（如 瓜）
...
```

---

## 相關檔案

| 平台 | 檔案 | 說明 |
|------|------|------|
| Android | `ComposingManager.kt` | 組字管理（雙狀態：rawInput + composingText） |
| Android | `ToneConverter.kt` | 聲調轉換（數字→調符） |
| Android | `ToneConverterModels.kt` | 調符映射表 |
| Android | `InputNormalizer.kt` | 輸入正規化（搜尋用） |
| iOS | `ComposingManager.swift` | 組字管理（雙狀態：rawInput + composingText） |
| iOS | `ToneConverter.swift` | 聲調轉換 |
| iOS | `ToneMappings.swift` | 調符映射表 |
| iOS | `InputNormalizer.swift` | 輸入正規化（搜尋用） |

---

## 測試案例

### 輸入與顯示

| 輸入序列 | rawInput | composingText |
|----------|----------|---------------|
| `g` `u` `a` | `gua` | `gua` |
| `g` `u` `a` `2` | `gua2` | `guá` |
| `g` `u` `a` `1` | `gua1` | `gua` |
| `g` `u` `a` `1` `2` | `gua12` | `guá` |
| `h` `o` `o` `2` (POJ) | `hoo2` | `hó͘` |
| `P` `h` `i` `a` `n` `n` (POJ) | `Phiann` | `Phiaⁿ` |

### 刪除測試

| 初始 composingText | 初始 rawInput | 刪除後 composingText | 刪除後 rawInput |
|--------------------|---------------|----------------------|-----------------|
| `guá` | `gua2` | `gua` | `gua` |
| `gua` | `gua1` | `gu` | `gu` |
| `Phiaⁿ` | `Phiann` | `Phia` | `Phia` |
