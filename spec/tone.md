# Tone 聲調處理

> **功能代號**: `Tone`
> **關鍵字**: `Tone`, `聲調`, `調符`, `ToneConverter`, `ToneMappings`

---

## 概述

台語有 8 個聲調，其中聲調 1（陰平）和 4（陰入）在傳統標記中不加調號。本鍵盤以數字形式顯示聲調 1 和 4，方便使用者辨識。

---

## 聲調對照表

| 聲調 | 調號 | 顯示範例 | 說明 |
|------|------|----------|------|
| 1 | (無) | `gua1` | 陰平，顯示數字 |
| 2 | ́ (acute) | `guá` | 陰上 |
| 3 | ̀ (grave) | `guà` | 陰去 |
| 4 | (無) | `at4` | 陰入，顯示數字，韻尾 -p/-t/-k/-h |
| 5 | ̂ (circumflex) | `guâ` | 陽平 |
| 6 | ̌ (caron) | `guǎ` | 陽上 |
| 7 | ̄ (macron) | `guā` | 陽去 |
| 8 | ̍ (vertical line) | `gua̍` | 陽入 |
| 9 | ̆ / ̋ | `guă` / `gua̋` | 輕聲（POJ/TL） |

---

## 核心邏輯

### 1. 詞庫生成（`to_numeric_tone`）

無調號音節根據韻尾自動補上聲調：

```python
if syllable[-1] in "ptkh":
    syllable = syllable + "4"  # 入聲韻尾 → 陰入
else:
    syllable = syllable + "1"  # 其餘 → 陰平
```

### 2. 輸入顯示（`ToneConverter`）

| 輸入 | rawInput | composingText |
|------|----------|---------------|
| `gua1` | `gua1` | `gua1` |
| `at4` | `at4` | `at4` |
| `gua2` | `gua2` | `guá` |

聲調 1/4 保留數字顯示，其餘轉為調符。

### 3. 搜尋正規化（`InputNormalizer`）

所有聲調數字保留，直接用於 Trie 搜尋：

| rawInput | 正規化結果 |
|----------|------------|
| `gua1` | `gua1` |
| `at4` | `at4` |
| `gua2` | `gua2` |

---

## 相關檔案

| 層面 | Android | iOS |
|------|---------|-----|
| 顯示轉換 | `ToneConverter.kt` | `POJToneConverter.swift` / `TLToneConverter.swift` |
| 搜尋正規化 | `InputNormalizer.kt` | `InputNormalizer.swift` |
| 詞庫生成 | `dictionary/common/romanization.py` | - |

---

## 測試案例

### 顯示

| 輸入 | 顯示 (POJ/TL) |
|------|---------------|
| `gua1` | `gua1` |
| `at4` | `at4` |
| `gua2` | `guá` |
| `hoo2` (POJ) | `hó͘` |
| `hoo2` (TL) | `hóo` |

### 搜尋

| 輸入 | 正規化 | 匹配詞庫 |
|------|--------|----------|
| `gua1` | `gua1` | `gua1...` |
| `at4` | `at4` | `at4...` |
| `gua2` | `gua2` | `gua2...` |
