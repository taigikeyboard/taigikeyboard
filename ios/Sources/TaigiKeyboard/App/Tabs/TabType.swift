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

    /// Localized tab title.
    // 中文: 該 tab 的本地化標題,從各 *Texts 取出。
    var title: String {
        switch self {
        case .home: HomeTexts.tabTitle
        case .theme: ThemeTexts.tabBarTitle
        case .layout: LayoutTexts.tabBarTitle
        case .dictionary: DictionaryTexts.tabBarTitle
        case .settings: SettingsTexts.tabBarTitle
        }
    }
}
