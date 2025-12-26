import SwiftUI

/// 主 APP Tab 類型定義
enum TabType: Int, CaseIterable, Hashable {
    case home = 0       // 頭頁
    case layout = 1     // 佈局
    case dictionary = 2 // 詞庫
    case settings = 3   // 設定

    /// Tab 圖標
    var icon: String {
        switch self {
        case .home: return "house.fill"
        case .layout: return "keyboard"
        case .dictionary: return "book.fill"
        case .settings: return "gearshape.fill"
        }
    }

    /// Tab 標題（LocalizedText）
    var title: LocalizedText {
        switch self {
        case .home: return Tab1Texts.tabTitle
        case .layout: return Tab2Texts.tabTitle
        case .dictionary: return Tab3Texts.tabTitle
        case .settings: return Tab4Texts.tabTitle
        }
    }
}
