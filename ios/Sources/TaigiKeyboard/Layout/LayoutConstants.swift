// 中文: 鍵盤按鍵寬度的百分比常數 — 直向 / 橫向兩套參數。
// 中文: 數值參考 iOS 標準鍵盤與 KeyboardKit 預設,LayoutConverter 用來算 itemWidth。

import CoreGraphics

/// 鍵盤佈局寬度比例常數
///
/// 這些數值基於 iOS 標準鍵盤測量與人體工學設計，
/// 參考 KeyboardKit 標準佈局實作。
enum LayoutConstants {
    // MARK: - Bottom Row (底部列)

    /// 底部系統按鍵寬度（123, 😀, 地球鍵等）
    enum BottomSystemButton {
        /// 直向模式寬度比例（佔螢幕寬度 13%）
        /// 比一般字符按鍵（10%）寬 30%
        static let portrait: CGFloat = 0.13

        /// 橫向模式寬度比例（佔螢幕寬度 10%）
        static let landscape: CGFloat = 0.10
    }

    /// Return 按鍵寬度
    enum ReturnButton {
        /// 直向模式寬度比例（佔螢幕寬度 15%）
        static let portrait: CGFloat = 0.15

        /// 橫向模式寬度比例（佔螢幕寬度 9.5%）
        static let landscape: CGFloat = 0.095
    }

    // MARK: - Letter Rows (字母列)

    /// Shift 和 Backspace 按鍵寬度
    /// 佔螢幕寬度 13%，提供足夠的點擊區域
    static let shiftBackspace: CGFloat = 0.13
}
