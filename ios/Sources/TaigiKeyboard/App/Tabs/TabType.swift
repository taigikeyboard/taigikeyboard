import SwiftUI

/// Tab type enum for the main app's 4 tabs.
enum TabType: Int, CaseIterable, Hashable {
    case home = 0
    case layout = 1
    case dictionary = 2
    case settings = 3

    /// SF Symbol name for this tab.
    var icon: String {
        switch self {
        case .home: "house.fill"
        case .layout: "keyboard"
        case .dictionary: "book.fill"
        case .settings: "gearshape.fill"
        }
    }

    /// Localized tab title.
    var title: String {
        switch self {
        case .home: Tab1Texts.tabTitle
        case .layout: Tab2Texts.tabBarTitle
        case .dictionary: Tab3Texts.tabBarTitle
        case .settings: Tab4Texts.tabBarTitle
        }
    }
}
