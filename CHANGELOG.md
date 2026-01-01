# Changelog

## v3.3.10 (develop-kk10)

### Android

#### 效能優化
- **候選詞顯示重構**：從 LinearLayout 改為 RecyclerView + DiffUtil + ListAdapter
- **Debounce + Cancel 機制**：候選詞更新加入防抖和取消機制，避免重複計算
- **UI 更新效能提升**：從 ~145ms 降至 4-6ms

#### Bug 修復
- **isTranslateSwapped 切換失效**：RecyclerView 重構後，DiffUtil 無法偵測狀態變化，加入 `notifyDataSetChanged()`
- **Dark Mode 圖示不可見**：修復 `ic_translate`、`ic_keyboard_arrow_up`、`ic_keyboard_arrow_down`、`ic_backspace` 的 fillColor
- **符號鍵盤重複數字列**：移除 SYMBOLS 模式多餘的 `number_row` extension
- **英文拼字檢查超時**：加入 2 秒 timeout 防止 coroutine 卡住

#### 功能調整
- **搜尋音節上限**：從 3 個音節提升至 4 個音節
- **移除 Recent Emoji**：刪除 `EmojiHistory.kt`、`EmojiHistoryManager.kt`、`RECENTLY_USED` 分類

#### 新增檔案
- `CandidateAdapter.kt`：RecyclerView 候選詞 Adapter
- `item_candidate.xml`：候選詞項目 Layout

### 共用

#### 詞庫腳本
- `02_create_app_db.sh`：音節限制從 `<= 3` 改為 `<= 4`
- `03_create_trie_db.sh`：音節限制從 `<= 3` 改為 `<= 4`

---

## v3.3.9

### iOS

#### 新功能
- **Flick 鍵盤**：新增日式滑動輸入佈局
- **英文模式**：獨立 `EnglishAutocompleteService`，支援純英文輸入
- **大小寫轉換**：新增 `CaseTransformationService`、`SuggestionCaseTransformer`

#### 架構重構
- **KeyboardKit 10 升級**：重構 Action Handler、Layout、Styling
- **佈局系統**：新增 `TaigiLayouts`、`LayoutConverter`、`DeviceConfiguration`
- **Tab1 拆分**：分離 FAQ、Feature、Copyright 等子頁面
- **主題簡化**：移除自定義 Theme，改用 Apple 標準 Form + Section

#### 移除
- `ThemeTokens`、`ThemeComponents`、`ButtonStyles`
- `AlphabeticLayoutBuilder`、`BottomRowBuilder`
- `CopyrightView`（舊版）、`SettingsComponents`

### Android

#### 新功能
- **英文模式**：新增 `EnglishAutocompleteService`、Smartbar 英文候選詞
- **InputNormalizer**：聲調 1/4 自動補齊、POJ o͘ (U+0358) 轉換

#### 依賴升級
- `compileSdk` 35 → 36
- `core-ktx` 1.15.0 → 1.17.0
- `activity` 1.10.1 → 1.12.2
- `compose-bom` 2024.10.01 → 2025.12.01
- `lifecycle` 2.8.7 → 2.10.0
- `datastore` 1.0.0 → 1.2.0
- `serialization-json` 1.6.0 → 1.8.0

#### 清理
- 移除未使用：Room、KSP、runtime-livedata
- 移除 `ThemeUtils.kt`、自定義色彩
- Lint 修復：`UseAppTint`、`MissingDefaultResource`、`Locale` 棄用

#### 工具
- 新增 `gradle-versions-plugin` 依賴檢查

### 共用
- 更新詞庫測試腳本
- 新增文件：`case.md`、`device.md`、`flow.md`、`layout.md`、`flick.md`
