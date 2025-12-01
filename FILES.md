# 台語鍵盤 Android - 檔案索引

## 核心 (Core)
- `TaigiKeyboard.kt` - IME 主服務
- `TextInputManager.kt` - 文字輸入總管理器
- `PrefHelper.kt` - 偏好設定管理 (DataStore)
- `SubtypeManager.kt` - 語言/佈局管理

## 按鍵系統
- `KeyCode.kt` - 按鍵代碼定義
- `KeyView.kt` - 按鍵視圖與事件處理
- `KeyboardView.kt` - 鍵盤主視圖
- `LayoutManager.kt` - 佈局載入與合併（支援全形/半形切換）

## 台語輸入
- `ComposingManager.kt` - 組字管理
- `TaigiAutocompleteService.kt` - 候選詞搜尋
- `LexiconService.kt` - 字典查詢 (SQLite)
- `ToneConverter.kt` - POJ/TL 聲調轉換

## 智慧列
- `SmartbarManager.kt` - 候選詞與翻譯模式管理（含 isTranslateSwapped 快取）
- `SmartbarView.kt` - 智慧列視圖
- `CandidateOverlayView.kt` - 候選詞浮層

## 其他
- `MediaInputManager.kt` - Emoji 鍵盤
- `KeyPopupManager.kt` - 長按彈出視窗
- `KeyboardSettingsActivity.kt` - 設定頁面

## 佈局資源 (Assets)
- `assets/ime/text/characters/qwerty_{poj,tl}.json` - 字母鍵盤
- `assets/ime/text/characters/default_{halfwidth,fullwidth}.json` - 修飾鍵
- `assets/ime/text/characters/extended_popups/taigi_{poj,tl}.json` - 長按字元
- `assets/ime/text/symbols/western_{default,fullwidth}.json` - 符號鍵盤
- `assets/dictionaries/*.db` - 字典資料庫

## 修改指南
- 按鍵外觀: `KeyView.kt` + `values/colors.xml`
- 按鍵行為: `TextInputManager.sendKeyPress()`
- 佈局檔案: `assets/ime/text/*/`
- 翻譯切換: `SmartbarManager.toggleTranslateSwapped()` + `LayoutManager.computeLayoutFor()`
