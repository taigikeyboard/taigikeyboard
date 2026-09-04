# Dynamic Font Download Plan

Date: 2026-04-18

## Goal

讓 Android 與 iOS 在「字型設定」頁面中，保留一個內建預設字型，其餘三個字型改為動態下載。

使用者可以：

- 在字型設定看到全部字型選項
- 點擊未安裝字型時下載檔案
- 下載完成後立即套用
- 對已下載字型執行刪除以釋放空間

UI 需延續目前設定頁風格，不另外引入新的視覺語言。

## Current State

### Android

- 字型選單在 [android/app/src/main/java/com/siansiansu/taigikeyboard/ui/tabs/layout/FontPickerContent.kt](android/app/src/main/java/com/siansiansu/taigikeyboard/ui/tabs/layout/FontPickerContent.kt)
- 字型解析集中在 [android/app/src/main/java/com/siansiansu/taigikeyboard/util/FontUtils.kt](android/app/src/main/java/com/siansiansu/taigikeyboard/util/FontUtils.kt)
- 設定值存於 `PrefHelper.fontType`
- 目前 `iansui`、`genYoMin`、`genYoGothic` 都直接從 `res/font` 載入
- 鍵盤本體、候選詞列、預覽、popup、overlay 都直接依賴 `FontUtils.getTypefaceByType(...)`

### iOS

- 字型設定頁在 [ios/Sources/TaigiKeyboard/App/Tabs/Layout/AppearanceSettingsView.swift](ios/Sources/TaigiKeyboard/App/Tabs/Layout/AppearanceSettingsView.swift)
- 字型型別定義在 [ios/Sources/TaigiKeyboard/Settings/SettingsTypes.swift](ios/Sources/TaigiKeyboard/Settings/SettingsTypes.swift)
- 使用者設定存於 [ios/Sources/TaigiKeyboard/Settings/SharedSettings.swift](ios/Sources/TaigiKeyboard/Settings/SharedSettings.swift)
- 主 app 透過 `UIAppFonts` 預載字型，定義於 [ios/Sources/TaigiKeyboard-Info.plist](ios/Sources/TaigiKeyboard-Info.plist)
- keyboard extension 透過 [ios/Sources/TaigiKeyboard/_Keyboard/FontRegistration.swift](ios/Sources/TaigiKeyboard/_Keyboard/FontRegistration.swift) 從 containing app bundle 註冊字型
- `KeyboardFonts`、`ButtonFontProvider`、candidate UI、preview 都假設字型已經在 bundle 或 process 內可用

## Key Finding

如果要真的做到「刪除字型可以釋放空間」，那三個目標字型不能再繼續隨 app bundle 一起出貨。

也就是說，這次需求不只是「新增下載功能」，還必須同步做下面兩件事：

1. Android 移除三個字型的 `res/font` 內建檔
2. iOS 移除三個字型的 bundle 資源與 `UIAppFonts` 項目

否則使用者就算刪掉下載檔，app 內建那份仍然存在，空間不會真的回收。

## Recommended Product Decision

建議保留 `Open Huninn` 為唯一內建字型，原因：

- 它已經是目前兩端的預設字型
- 導航列與整體 app 視覺已圍繞這個字型建立
- 刪除預設字型會讓首次體驗、fallback 與遷移成本變高

建議改為動態下載的三個字型：

- `Iansui-Regular.ttf`
- `GenYoMin2TW-R.otf`
- `GenYoGothic2TW-R.otf`

## Download Source Design

使用者看到的是 GitHub repo 頁面，但 app 實際下載不應使用 `tree` URL。

建議改成兩層結構：

1. 字型 repo 提供一份 manifest
2. app 下載 manifest 內指定的 raw 檔案 URL

### Recommended manifest

建議在 `taigikeyboard/fonts` repo 新增例如 `fonts-manifest.json`：

```json
{
  "version": 1,
  "fonts": [
    {
      "id": "iansui",
      "displayName": "芫荽",
      "fileName": "Iansui-Regular.ttf",
      "postScriptName": "Iansui-Regular",
      "downloadUrl": "https://raw.githubusercontent.com/taigikeyboard/fonts/main/fonts/Iansui-Regular.ttf",
      "sha256": "...",
      "sizeBytes": 0
    }
  ]
}
```

原因：

- 避免 app 端硬編碼 GitHub HTML 頁面
- 可驗證檔案大小與 hash
- 之後更新字型版本時不必改 app 邏輯
- Android 與 iOS 可以共用相同字型目錄資訊

如果暫時不想維護 manifest，也至少要在 app 端固定使用 `raw.githubusercontent.com` 的直接檔案 URL，而不是 repo tree URL。

## Cross-Platform Architecture

建議新增一個共通概念：`FontCatalog + FontInstallState + FontResolver`

### 1. FontCatalog

定義所有可選字型的固定 id 與 metadata：

- `system`
- `openHuninn`
- `iansui`
- `genYoMin`
- `genYoGothic`

每筆資料至少要有：

- `id`
- `displayName`
- `isBundled`
- `postScriptName`
- `fileName`
- `remoteUrl`

### 2. FontInstallState

區分 UI 與實際狀態：

- `bundled`
- `notInstalled`
- `downloading(progress?)`
- `installed`
- `failed(error?)`

### 3. FontResolver

任何要拿字型的地方，都不要再直接假設來源一定是 bundle/res。

應改成：

- 先看是否 `system`
- 再看是否 `bundled`
- 再看是否存在本地下載檔
- 不存在時 fallback 到 `openHuninn`

## Android Plan

### Storage

將下載字型放在 app 私有目錄，例如：

- `context.filesDir/fonts/`

不需要外部儲存權限。

### Data Model

建議新增：

- `DownloadableFontSpec`
- `InstalledFontRecord`
- `FontRepository`
- `FontDownloadManager`

`PrefHelper.fontType` 可先維持字串 id，不需要立刻改資料格式。

另新增一份字型安裝 metadata，例如：

- `installed_fonts.json`

內容至少包含：

- `fontId`
- `fileName`
- `localPath`
- `sha256`
- `installedAt`
- `version`

### Download Flow

建議流程：

1. 使用者點未安裝字型
2. 顯示確認 dialog，告知大小與將下載字型
3. 開始下載到暫存檔
4. 驗證檔案 hash
5. 原子搬移到正式目錄
6. 更新 metadata
7. 將 `prefs.fontType` 設為該字型
8. 通知預覽與鍵盤重新解析 typeface

### Font Loading Refactor

`FontUtils` 需從目前的 `res/font` 分支改成：

- `system` -> `Typeface.DEFAULT`
- `openHuninn` -> `res/font`
- downloadable fonts -> `Typeface.createFromFile(localPath)`，不存在則 fallback

需要一起掃描並改掉所有直接依賴 `R.font.genyomin2tw_r` / `R.font.genyogothic2tw_r` / `R.font.iansui_regular` 的地方。

已確認至少包含：

- `FontUtils`
- `SettingsOverlayContent`
- `KeyboardPreviewPanel`
- `CandidateAdapter`
- `CandidateOverlayAdapter`
- `KeyboardView`
- `KeyPopupManager`

### Delete Behavior

刪除已下載字型時：

1. 若該字型正被使用，先把 `prefs.fontType` 切回 `openHuninn`
2. 刪除檔案與 metadata
3. refresh preview / keyboard / candidate UI
4. UI 狀態回到 `notInstalled`

### Android UI

不建議做成完全新頁型態，建議延續現有 `SettingsCard + row`。

單列字型 row 建議包含：

- 左側：字型名稱
- 右側：狀態文字，例如 `已下載` / `下載中` / `未下載`
- 最右：選取 checkmark 或 chevron / 下載 icon / 刪除 icon

互動規則：

- `system`、`openHuninn`：點擊即選取
- `未下載`：點擊開啟下載確認
- `下載中`：顯示 spinner，不可重複點擊
- `已下載`：點擊即套用
- `已下載且非內建`：長按或 trailing action 顯示刪除確認

為了維持目前風格，Android 仍建議維持純 row list，不做卡片內複雜按鈕佈局。

## iOS Plan

### Storage

下載字型要放到 App Group container，不能只放 main app sandbox。

建議路徑：

- `SharedSettings.sharedContainerURL/AppSupport/Fonts/`

原因：

- 主 app 下載後，keyboard extension 才讀得到
- 使用者選取字型後，extension 可直接註冊同一份檔案

### Data Model

`FontType` raw value 可以維持不變，但要新增一個 catalog/repository 層，避免 enum 直接綁 bundle 字型。

建議新增：

- `FontCatalog`
- `FontInstallRecord`
- `FontInstallStore`
- `FontDownloadService`
- `RuntimeFontResolver`

### Download Flow

建議只在主 app 內下載，不在 keyboard extension 下載。

流程：

1. 在設定頁點未安裝字型
2. 主 app 用 `URLSessionDownloadTask` 下載至暫存
3. 驗證 hash
4. 搬移到 App Group Fonts 目錄
5. 使用 `CTFontManagerRegisterFontsForURL(..., .process, ...)` 立即註冊到 app process
6. 更新安裝 metadata
7. 寫入 `SharedSettings.fontType`

keyboard extension 在下次啟動或喚醒時，重新從 App Group 字型目錄註冊。

### Font Registration Refactor

目前的 [ios/Sources/TaigiKeyboard/_Keyboard/FontRegistration.swift](ios/Sources/TaigiKeyboard/_Keyboard/FontRegistration.swift) 是從 containing app bundle 找字型，這在動態下載後不夠用。

建議改成兩段：

1. 註冊 bundle 內建字型
2. 註冊 App Group 已下載字型

主 app 端的 `KeyboardFonts.globalFont/globalUIFont`、`ButtonFontProvider`、preview、candidate view 都改為先透過 resolver 拿可用的 PostScript font name。

### Bundle Cleanup

要達成空間釋放，iOS 需同步調整：

- `UIAppFonts` 只保留 `jf-openhuninn-2.1.ttf`
- Xcode target resources 移除三個動態字型檔
- `FontRegistration.fontFileNames` 改成只處理內建字型，下載字型改由 App Group 掃描

### Delete Behavior

刪除已下載字型時：

1. 若目前選中該字型，先切回 `.openHuninn`
2. 刪除 App Group 檔案
3. 刪除 metadata
4. 重新整理字型頁與 preview

`CTFontManager` 對已註冊字型的解除註冊需要保守處理；實作上可接受本次 process 繼續使用舊註冊字型，並在下次 app/extension 啟動後消失。產品行為上仍可立即切回 fallback 字型。

### iOS UI

沿用既有 `Form > Section > row` 結構即可，不建議另做自訂卡片樣式。

每列建議顯示：

- 字型名稱
- 次要狀態文字：`內建` / `未下載` / `已下載`
- 若正在下載，顯示 `ProgressView`
- 對已下載的非內建字型提供刪除 action

建議互動：

- 主操作仍是點 row
- 刪除可用 trailing swipe action 或 row 內小型 destructive button

若要和 Android 更一致，建議兩端都採「點 row 選取 / 下載，刪除走 confirmation action」。

## UI Recommendation

建議把字型列分成三種視覺狀態，但不新增新的版型：

### Built-in

- 名稱正常顯示
- 選中時顯示 check
- 次文字顯示 `預設`

### Downloadable, not installed

- 名稱正常顯示
- 次文字顯示 `未下載`
- 點擊後出現確認下載 dialog

### Downloadable, installed

- 名稱正常顯示
- 次文字顯示 `已下載`
- 選中時顯示 check
- 提供刪除操作

文案上建議避免出現「從 GitHub 下載」字樣直接暴露實作細節，對使用者顯示 `下載字型` 即可。

## Migration Plan

### Phase 1: 抽象層

- 建立 font catalog
- 建立 installed state / metadata store
- 建立 resolver
- 把所有字型使用點改成走 resolver

### Phase 2: 下載與刪除

- Android 建立下載器與 hash 驗證
- iOS 建立 App Group 字型下載與註冊
- 字型設定頁加入下載、套用、刪除狀態

### Phase 3: 移除 bundle 內三個字型

- Android 刪除三個 `res/font`
- iOS 刪除三個 bundled font resources 與 `UIAppFonts`
- 驗證 fallback 行為

### Phase 4: 驗證與回歸

- 切換字型後鍵盤主鍵、候選字、popup、設定頁、preview 都一致
- 刪除當前字型時自動回退到 `openHuninn`
- 冷啟 app 與 extension 後字型依然可用
- 未下載狀態下不會 crash
- 網路失敗與 hash 驗證失敗有明確提示

## Risks

### 1. iOS extension 與 main app 共用字型檔

若下載目錄沒放在 App Group，extension 會讀不到。

### 2. 目前字型使用點很多

Android 與 iOS 都不只一個 renderer 在吃字型。若只改設定頁，實際鍵盤與候選區會出現不一致。

### 3. 直接使用 GitHub tree URL 不可靠

HTML 頁面不是穩定下載介面，也不適合做 hash 驗證。

### 4. 刪除後仍被舊 process 持有

iOS 尤其要接受「本次 process 尚可短暫持有已註冊字型」這件事，因此產品邏輯要以 fallback 與下次啟動一致性為主。

## Testing Checklist

- 新安裝 app 時只看到內建字型可立即使用
- 三個可下載字型顯示 `未下載`
- 點擊下載後，成功安裝並立即可選
- 安裝後重開 app，狀態仍正確
- 喚醒 keyboard extension 後，字型正確生效
- 候選列、主鍵、popup、預覽都使用同一字型
- 刪除非當前字型成功
- 刪除當前字型後自動回退 `openHuninn`
- 離線下載失敗有錯誤提示
- hash 不符時不會套用損壞字型

## Open Questions To Confirm Before Implementation

1. 是否確認 `Open Huninn` 為唯一保留內建字型？
2. 是否願意在 `taigikeyboard/fonts` repo 新增 manifest 檔？
3. 刪除動作是否要限制「不能刪除目前正在使用的字型」，還是允許刪除並自動切回 `Open Huninn`？

## Recommended Implementation Order

1. 先做 catalog / resolver / installed-state 抽象
2. 再做 Android 下載與 UI
3. 再做 iOS App Group 下載與 runtime registration
4. 最後才移除 bundle/res 內建字型

這個順序比較安全，因為可以先在不拆資源的情況下驗證新的解析鏈路，等下載流程穩定後，再移除內建字型來達成真正的空間釋放。
