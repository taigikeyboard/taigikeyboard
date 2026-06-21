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
    /// classification (tab strip + page title follow OS locale, not the picker). All five are inline
    /// literals after their `*Texts` files were deleted across R2b-2…R2b-5.
    // 中文: tab strip + 頁面標題的 nav chrome。刻意非 i18n key — 不隨 app 顯示語言 picker live-switch,
    // 中文: 對齊 Android native R.string.tab_*(跟 OS locale,不跟 picker)。五個 tab 於 R2b-2…R2b-5
    // 中文: 刪除各自 *Texts 後皆改 inline 字面值。
    var title: String {
        switch self {
        case .home: "頭頁"
        case .theme: "主題"
        case .layout: "佈局"
        case .dictionary: "詞庫"
        case .settings: "設定"
        }
    }
}
