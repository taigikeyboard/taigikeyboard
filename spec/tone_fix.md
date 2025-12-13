# 聲調修正功能規格

## 概述

台語有 8 個聲調，其中聲調 1 和 4 沒有調號標記（視為無聲調）。本功能處理聲調輸入、顯示、搜尋及修正。

## 聲調對照表

| 聲調 | 調號 | 說明 |
|------|------|------|
| 1 | (無) | 陰平，無調號 |
| 2 | ́ (acute) | 陰上 |
| 3 | ̀ (grave) | 陰去 |
| 4 | (無) | 陰入，無調號 |
| 5 | ̂ (circumflex) | 陽平 |
| 6 | ̌ (caron) | 陽上 |
| 7 | ̄ (macron) | 陽去 |
| 8 | ̍ (vertical line) | 陽入 |
| 9 | ̆ (breve) / ̋ (double acute) | 輕聲（POJ/TL 不同） |

---

## 功能一：聲調輸入與顯示

### 處理邏輯

**檔案**：`ComposingManager.kt`

| 輸入 | rawInput | composingText (顯示) |
|------|----------|----------------------|
| `gua2` | `gua2` | `guá` |
| `gua1` | `gua1` | `gua` |
| `gua4` | `gua4` | `gua` |

### 實作重點

```kotlin
// ComposingManager.appendCharacter()
if (toneNumber in 1..9) {
    newText = applyToneConversion(newText, toneNumber) ?: newText
}

// ToneConverter.convertPOJSyllable() / convertTLSyllable()
if (toneNumber == 0 || toneNumber == 1 || toneNumber == 4) {
    return baseForm  // 無調號，移除數字
}
```

---

## 功能二：搜尋正規化

### 處理邏輯

**檔案**：`InputNormalizer.kt`

| rawInput | 正規化結果 | 說明 |
|----------|------------|------|
| `gua1` | `gua` | 移除聲調 1 |
| `gua4` | `gua` | 移除聲調 4 |
| `gua2` | `gua2` | 保留聲調 2 |
| `gua12` | `gua2` | 移除 1，保留 2 |
| `gua42` | `gua2` | 移除 4，保留 2 |

### 實作重點

```kotlin
// InputNormalizer.normalizeSyllable()
// 移除聲調 1 和 4（視為無聲調）
val withoutTone1And4 = syllable.replace("1", "").replace("4", "")
```

---

## 功能三：聲調重選（刪除後重新輸入）

### 使用情境

使用者輸入 `gua2`（顯示 `guá`），想改成聲調 5：
1. 按 Backspace → 刪除聲調，顯示 `gua`
2. 輸入 `5` → 顯示 `guâ`

### 處理邏輯

**檔案**：`ComposingManager.kt`

```
刪除前：rawInput = "gua2", composingText = "guá"
刪除後：rawInput = "gua",  composingText = "gua"
輸入 5：rawInput = "gua5", composingText = "guâ"
```

### 實作重點

```kotlin
// ComposingManager.deleteBackward()
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

### attemptToneRestoration()

從 `composingText` 找到帶調號的字元，還原為基本字元：

```kotlin
// 使用 ToneConverterModels.pojToneMapping / tlToneMapping
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

**檔案**：`ComposingManager.kt`

```
刪除前：rawInput = "Phiann", composingText = "Phiaⁿ"
刪除後：rawInput = "Phia",   composingText = "Phia"
```

### 實作重點

```kotlin
// ComposingManager.deleteBackward()
val lastChar = composingText.lastOrNull()
// ⁿ 對應 rawInput 的 nn（2 個字元）
val rawDeleteCount = if (lastChar == 'ⁿ') 2 else 1

composingText = composingText.dropLast(1)
rawInput = rawInput.dropLast(rawDeleteCount)
```

---

## 相關檔案

| 檔案 | 職責 |
|------|------|
| `ComposingManager.kt` | 組字狀態管理、聲調輸入/刪除 |
| `ToneConverter.kt` | 數字聲調 → 調號轉換 |
| `ToneConverterModels.kt` | 調號映射表 |
| `InputNormalizer.kt` | 搜尋用正規化（移除聲調 1/4） |

---

## 測試案例

### 顯示測試

| 輸入序列 | 預期顯示 |
|----------|----------|
| `g` `u` `a` `1` | `gua` |
| `g` `u` `a` `4` | `gua` |
| `g` `u` `a` `2` | `guá` |
| `g` `u` `a` `1` `2` | `guá` |
| `g` `u` `a` `4` `2` | `guá` |

### 刪除測試

| 初始狀態 | 按 Backspace | 預期結果 |
|----------|--------------|----------|
| `guá` (rawInput: `gua2`) | 1 次 | `gua` (rawInput: `gua`) |
| `gua` (rawInput: `gua1`) | 1 次 | `gu` (rawInput: `gu`) |
| `Phiaⁿ` (rawInput: `Phiann`) | 1 次 | `Phia` (rawInput: `Phia`) |

### 搜尋測試

| rawInput | 正規化結果 |
|----------|------------|
| `gua1` | `gua` |
| `gua4` | `gua` |
| `gua12` | `gua2` |
| `gua42` | `gua2` |
| `hoo1-bo5` | `hooboo5` |
