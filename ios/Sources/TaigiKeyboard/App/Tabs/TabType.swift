// 中文: 主 App 五個 tab 的型別 enum,提供 SF Symbol 名稱與本地化標題。

import SwiftUI

/// Tab type enum for the main app's 5 tabs.
// 中文: 主 App 五個 tab 的識別 enum。Int rawValue 同時是 tab 索引(主題置於第 2 位)。
enum TabType: Int, CaseIterable, Hashable {
    case home = 0
    case theme = 1
    case layout = 2
    case dictionary = 3
    case settings = 4

    /// SF Symbol name for this tab.
    // 中文: 該 tab 對應的 SF Symbol 名稱。
    var icon: String {
        switch self {
        case .home: "house.fill"
        case .theme: "paintpalette.fill"
        case .layout: "keyboard"
        case .dictionary: "book.fill"
        case .settings: "gearshape.fill"
        }
    }

    /// Tab title.
    ///
    /// Tab-strip + page chrome. Deliberately NOT an i18n key — it does not live-switch with the in-app
    /// display-language picker, matching Android's hand-written native `R.string.tab_*` nav-chrome
    /// classification (tab strip + page title follow OS locale, not the picker). `.layout` / `.settings`
    /// are inline literals after their `*Texts` files were deleted in R2b-2/R2b-3; the other two keep
    /// their `*Texts` refs until their own migration rounds inline likewise.
    // 中文: tab strip + 頁面標題的 nav chrome。刻意非 i18n key — 不隨 app 顯示語言 picker live-switch,
    // 中文: 對齊 Android native R.string.tab_*(跟 OS locale,不跟 picker)。theme/layout/settings 於
    // 中文: R2b-2/R2b-3 刪除 *Texts 後改 inline 字面值;其餘兩個待各自遷移輪次比照處理。
    var title: String {
        switch self {
        case .home: HomeTexts.tabTitle
        case .theme: "主題"
        case .layout: "佈局"
        case .dictionary: DictionaryTexts.tabBarTitle
        case .settings: "設定"
        }
    }
}
