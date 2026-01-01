# Composing 組字管理

> **類型**: 功能
> **關鍵字**: `Composing`, `rawInput`, `composingText`, `ComposingManager`
> **相關**: autocomplete.md, tone.md

---

## 重點摘要

- 維護「雙狀態」：`rawInput`（搜尋用）+ `composingText`（顯示用）
- 輸入 `gua2` → rawInput=`gua2`, composingText=`guá`

---

## 核心概念

### 雙狀態模型

| 狀態 | 用途 | 範例 |
|------|------|------|
| `rawInput` | Trie 搜尋（數字聲調） | `gua2` |
| `composingText` | UI 顯示（調符） | `guá` |

### 為什麼需要雙狀態

- Trie 索引格式：`tl:gua2`、`poj:goa2`
- 使用者期望看到調符，不是數字

---

## 核心操作

| 操作 | 說明 |
|------|------|
| `appendCharacter` | 追加字元，更新雙狀態 |
| `deleteBackward` | 刪除字元，需處理聲調還原 |
| `commitComposition` | 確認組字，輸出文字 |
| `selectSuggestion` | 選擇候選詞，取代組字 |

### appendCharacter 流程

1. 更新 rawInput（保留原始 ASCII）
2. 檢查字元組合（POJ: `oo`→`o͘`, `nn`→`ⁿ`）
3. 檢查聲調轉換（數字→調符）
4. 更新 composingText
5. 同步到輸入框

### deleteBackward 流程

1. 嘗試聲調還原（`guá`→`gua`）
2. 成功：rawInput 刪除數字，composingText 更新
3. 失敗：兩者都刪除最後字元
4. 特殊：`ⁿ` 對應 rawInput 的 `nn`（2 字元）

---

## 字元組合轉換（POJ 限定）

| 輸入 | 轉換 | Unicode |
|------|------|---------|
| `oo` | `o͘` | U+006F + U+0358 |
| `nn` | `ⁿ` | U+207F |

- TL 模式不轉換，保持原樣

---

## 聲調處理

| 聲調 | 處理 | 範例 |
|------|------|------|
| 1, 4 | 保留數字 | `gua1`→`gua1` |
| 2,3,5,6,7,8,9 | 轉調符 | `gua2`→`guá` |

---

## 平台對照

| 項目 | iOS | Android |
|------|-----|---------|
| 組字管理 | `ComposingManager.swift` | `ComposingManager.kt` |
| 聲調轉換 | `ToneConverter.swift` | `ToneConverter.kt` |
| 調符映射 | `ToneMappings.swift` | `ToneConverterModels.kt` |

---

## 測試案例

| 輸入序列 | rawInput | composingText |
|----------|----------|---------------|
| g u a | `gua` | `gua` |
| g u a 2 | `gua2` | `guá` |
| g u a 1 | `gua1` | `gua1` |
| h o o 2 (POJ) | `hoo2` | `hó͘` |

| 刪除前 | 刪除後 rawInput | 刪除後 composingText |
|--------|-----------------|----------------------|
| `guá` / `gua2` | `gua` | `gua` |
| `Phiaⁿ` / `Phiann` | `Phia` | `Phia` |
