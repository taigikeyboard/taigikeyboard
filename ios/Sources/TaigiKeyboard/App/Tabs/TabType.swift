// 主 App 五個 tab 的型別 enum,提供 SF Symbol 名稱與本地化標題。

import SwiftUI

/// Tab type enum for the main app's 5 tabs.
// 主 App 五個 tab 的識別 enum。Int rawValue 同時是 tab 索引(主題置於第 2 位)。
enum TabType: Int, CaseIterable, Hashable {
    case home = 0
    case theme = 1
    case layout = 2
    case dictionary = 3
    case settings = 4

    /// SF Symbol name for this tab.
    // 該 tab 對應的 SF Symbol 名稱。
    var icon: String {
        switch self {
        case .home: "house.fill"
        case .theme: "paintpalette.fill"
        case .layout: "keyboard"
        case .dictionary: "book.fill"
        case .settings: "gearshape.fill"
        }
    }

    /// i18n key for this tab's title (tab strip + top-level page chrome).
    ///
    /// Resolved at the call site via `DisplayLanguageStore`, so the title live-switches with the in-app
    /// display-language picker. Mirrors Android `i18n_nav_tab*` (`StringKey.NAV_TAB_*`).
    // 該 tab 標題的 i18n key(tab strip + 頁面 chrome)。在 call site 用 DisplayLanguageStore 解析,
    // 隨 app 顯示語言 picker live-switch。對齊 Android i18n_nav_tab*。
    var titleKey: StringKey {
        switch self {
        case .home: .navTabHome
        case .theme: .navTabTheme
        case .layout: .navTabLayout
        case .dictionary: .navTabDictionary
        case .settings: .navTabSettings
        }
    }
}
