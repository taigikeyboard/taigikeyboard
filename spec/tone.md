# Tone 聲調處理

> **類型**: 功能
> **關鍵字**: `Tone`, `ToneConverter`, `ToneMappings`, `ToneRestoration`
> **相關**: composing.md

---

## 重點摘要

- 台語 8 聲調，聲調 1/4 不加調號
- 輸入數字聲調，顯示轉為調符（除 1/4）
- 支援 POJ 和 TL 兩種羅馬字系統

---

## 聲調對照表

| 聲調 | 調號 | 範例 | 說明 |
|------|------|------|------|
| 1 | (無) | `gua1` | 陰平，顯示數字 |
| 2 | ́ (acute) | `guá` | 陰上 |
| 3 | ̀ (grave) | `guà` | 陰去 |
| 4 | (無) | `at4` | 陰入，顯示數字 |
| 5 | ̂ (circumflex) | `guâ` | 陽平 |
| 6 | ̌ (caron) | `guǎ` | 陽上（少用） |
| 7 | ̄ (macron) | `guā` | 陽去 |
| 8 | ̍ (vertical) | `gua̍t` | 陽入 |
| 9 | ̆ / ̋ | `guă` | 輕聲 |

---

## 核心邏輯

### 1. 輸入顯示（ToneConverter）

| 輸入 | rawInput | composingText |
|------|----------|---------------|
| `gua1` | `gua1` | `gua1` |
| `gua2` | `gua2` | `guá` |
| `at4` | `at4` | `at4` |

### 2. 詞庫生成（to_numeric_tone）

```python
# 無調號音節根據韻尾補聲調
if syllable[-1] in "ptkh":
    syllable += "4"  # 入聲
else:
    syllable += "1"  # 陰平
```

### 3. 聲調還原（ToneRestoration）

刪除時需還原調符為基本字元：
- `guá` → `gua`
- `hó͘` → `ho͘` (POJ)

---

## POJ vs TL 差異

| 項目 | POJ | TL |
|------|-----|-----|
| 母音 oo | `o͘` (U+006F+U+0358) | `oo` |
| 鼻化 | `ⁿ` (U+207F) | `nn` |
| 聲調 8 | `o̍` | `o̍` |

---

## 平台對照

| 項目 | iOS | Android |
|------|-----|---------|
| 轉換器 | `POJToneConverter.swift`, `TLToneConverter.swift` | `ToneConverter.kt` |
| 映射表 | `ToneMappings.swift` | `ToneConverterModels.kt` |
| 還原 | `ToneRestoration.swift` | 內建於 ToneConverter |
| 工具 | `ToneUtilities.swift` | `ToneCharacterUtils.kt` |

---

## 測試案例

| 輸入 | POJ 顯示 | TL 顯示 |
|------|----------|---------|
| `gua1` | `gua1` | `gua1` |
| `gua2` | `guá` | `guá` |
| `hoo2` | `hó͘` | `hóo` |
| `phiann` | `phiaⁿ` | `phiann` |
