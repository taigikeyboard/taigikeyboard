# Autocomplete 自動完成

> **功能代號**: `Autocomplete`
> **關鍵字**: `Autocomplete`, `候選詞`, `getSuggestions`, `AutocompleteService`

---

## 概述

自動完成服務負責：
1. 根據使用者輸入搜尋候選詞
2. 判斷輸入類型（漢字、帶聲調羅馬字、無聲調羅馬字）
3. 整合候選詞列表（第 0 個位置為當前組字文字）

---

## 核心流程

```
使用者輸入
    ↓
ComposingManager 更新 rawInput / composingText
    ↓
觸發 autocomplete（KeyboardKit / FlorisBoard）
    ↓
AutocompleteService 讀取 rawInput
    ↓
determineInputType(rawInput)
    ↓
LexiconService.search(rawInput)
    ↓
建立候選詞列表（第 0 個 = composingText）
```

---

## 平台實作

### Android

```kotlin
// TaigiAutocompleteService.kt
suspend fun getSuggestions(rawInput: String, displayText: String): List<TaigiWord> {
    // 1. 判斷輸入類型（使用 rawInput）
    val inputType = determineInputType(rawInput)

    // 2. 搜尋詞典（使用 rawInput）
    val words = LexiconService.search(
        input = rawInput,
        inputType = inputType,
        inputMode = inputMode
    )

    // 3. 插入組字文字候選詞（使用 displayText）
    return buildList {
        add(createComposingTextWord(displayText))
        addAll(words)
    }
}
```

**呼叫方式**：
```kotlin
// ComposingManager 直接傳入 rawInput 和 composingText
autocompleteService.getSuggestions(rawInput, composingText)
```

### iOS

```swift
// AutocompleteService.swift
func autocomplete(_ text: String) async throws -> Autocomplete.ServiceResult {
    // 1. 從 ComposingManager 取得雙狀態
    let rawInput = composingManager.rawInput       // 搜尋用
    let displayText = composingManager.composingText  // 顯示用

    // 2. 判斷輸入類型（使用 rawInput）
    let inputType = determineInputType(rawInput)

    // 3. 搜尋詞典（使用 rawInput）
    let words = try await lexiconService.search(
        for: rawInput,
        inputType: inputType,
        inputMode: inputMode
    )

    // 4. 插入組字文字候選詞（使用 displayText）
    var suggestions = convertToSuggestions(words)
    suggestions.insert(createComposingTextSuggestion(displayText), at: 0)

    return Autocomplete.ServiceResult(inputText: text, suggestions: suggestions)
}
```

**autocompleteText 屬性**：
```swift
// KeyboardViewController.swift
// 必須回傳 rawInput，否則聲調 1/4 時 autocomplete 不會觸發
override var autocompleteText: String? {
    if let handler = actionHandler, handler.composingManager.isComposing {
        return handler.composingManager.rawInput  // 重要：使用 rawInput
    }
    return super.autocompleteText
}
```

---

## 輸入類型判斷

### determineInputType

| 輸入 | 類型 | 說明 |
|------|------|------|
| `我` | Hanzi | 包含漢字 |
| `guá` | RomanWithTone | 包含調符 |
| `gua2` | RomanWithTone | 包含數字聲調 (2,3,5,6,7,8,9) |
| `gua` | RomanWithoutTone | 無聲調 |
| `gua1` | RomanWithoutTone | 聲調 1 不視為有聲調 |
| `gua4` | RomanWithoutTone | 聲調 4 不視為有聲調 |

### containsNumericTone

兩平台一致：**排除 0, 1, 4**

```kotlin
// Android
private fun containsNumericTone(text: String): Boolean {
    return text.any { char ->
        char.isDigit() && char != '1' && char != '4' && char != '0'
    }
}
```

```swift
// iOS
private func containsNumericTone(_ text: String) -> Bool {
    text.contains { char in
        char.isNumber && char != "1" && char != "4" && char != "0"
    }
}
```

**為什麼排除 1 和 4？**
- 聲調 1 和 4 在台語羅馬字中不加調號
- 若將 `gua1` 視為 RomanWithTone，會影響無聲調搜尋邏輯
- Trie 搜尋時仍會正確使用完整的 `gua1` 作為 key

---

## 候選詞結構

### 第 0 個位置：當前組字文字

兩平台都將當前組字文字放在候選詞列表的第 0 個位置：

```kotlin
// Android
private fun createComposingTextWord(composingText: String): TaigiWord {
    return TaigiWord(
        id = 0,              // 特殊 ID
        roman = composingText,
        hanzi = null,
        lengthScore = null
    )
}
```

```swift
// iOS
private func createComposingTextSuggestion(_ composingText: String) -> Autocomplete.Suggestion {
    return Autocomplete.Suggestion(
        text: composingText,
        title: composingText,
        subtitle: nil,
        additionalInfo: ["isComposingText": "true"]
    )
}
```

### 候選詞索引

```
索引 0: 組字文字（displayText，如 guá）
索引 1: 候選詞 1（如 我）
索引 2: 候選詞 2（如 瓜）
...
```

---

## 關鍵設計決策

### 1. rawInput vs composingText 用途

| 狀態 | 搜尋用 | 顯示用 | 判斷輸入類型 | autocompleteText |
|------|--------|--------|-------------|------------------|
| rawInput | ✓ | | ✓ | ✓ (iOS) |
| composingText | | ✓ | | |

### 2. iOS autocompleteText 必須使用 rawInput

**原因**：KeyboardKit 根據 `autocompleteText` 變化決定是否觸發 autocomplete。

**問題情境**：
- 使用者輸入 `soo1`
- composingText: `Soo` → `Soo`（無變化，聲調 1 不加調號）
- rawInput: `Soo` → `Soo1`（有變化）

若 `autocompleteText` 回傳 `composingText`，KeyboardKit 偵測無變化，跳過 autocomplete。

### 3. 大小寫處理

iOS 根據使用者輸入的大小寫模式轉換候選詞：

```swift
private enum CasePattern {
    case lowercase      // 全小寫
    case capitalized    // 首字母大寫
    case uppercase      // 全大寫
}

private func determineCasePattern(from text: String) -> CasePattern
private func applyCaseTransform(to text: String, pattern: CasePattern) -> String
```

---

## 平台一致性檢查

| 項目 | Android | iOS | 一致 |
|------|---------|-----|------|
| 搜尋輸入來源 | rawInput | rawInput | ✓ |
| 判斷輸入類型來源 | rawInput | rawInput | ✓ |
| containsNumericTone 排除 | 0, 1, 4 | 0, 1, 4 | ✓ |
| 第 0 個候選詞 | composingText | composingText | ✓ |
| 去重邏輯 | 有 | 有 | ✓ |

---

## 相關檔案

| 平台 | 檔案 | 說明 |
|------|------|------|
| Android | `TaigiAutocompleteService.kt` | 自動完成服務 |
| Android | `LexiconService.kt` | 詞典搜尋 |
| iOS | `AutocompleteService.swift` | 自動完成服務 |
| iOS | `LexiconService.swift` | 詞典搜尋 |
| iOS | `KeyboardViewController.swift` | autocompleteText 屬性 |

---

## 測試案例

### 輸入與候選詞

| 輸入 rawInput | 輸入 composingText | 預期候選詞 |
|---------------|-------------------|-----------|
| `gua2` | `guá` | [guá, 我, 瓜, ...] |
| `soo1` | `soo` | [soo, 所, 鎖, ...] (只有聲調 1) |
| `soo` | `soo` | [soo, 所, 鎖, 蘇, ...] (所有聲調) |

### autocomplete 觸發

| 輸入序列 | autocompleteText 變化 | 是否觸發 |
|----------|----------------------|---------|
| s → so | s → so | ✓ |
| so → soo | so → soo | ✓ |
| soo → soo1 | soo → soo1 | ✓ |
| soo → soo2 | soo → soo2 | ✓ |
