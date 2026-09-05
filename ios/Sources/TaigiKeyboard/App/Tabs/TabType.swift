import SwiftUI

/// Tab type enum for the main app's 5 tabs.
enum TabType: Int, CaseIterable, Hashable {
    case home = 0
    case theme = 1
    case layout = 2
    case dictionary = 3
    case settings = 4

    /// SF Symbol name for this tab.
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
