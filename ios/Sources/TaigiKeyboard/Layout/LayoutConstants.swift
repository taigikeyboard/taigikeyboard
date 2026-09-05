// Key widths as a fraction of screen width, portrait and landscape. Values follow the stock iOS
// keyboard and KeyboardKit defaults; LayoutConverter turns them into itemWidth.

import CoreGraphics

enum LayoutConstants {
    // MARK: - Bottom Row

    /// System keys on the bottom row (123, emoji, globe).
    enum BottomSystemButton {
        /// 30% wider than a letter key (10%).
        static let portrait: CGFloat = 0.13

        static let landscape: CGFloat = 0.10
    }

    enum ReturnButton {
        static let portrait: CGFloat = 0.15

        static let landscape: CGFloat = 0.095
    }

    // MARK: - Letter Rows

    /// Shift and Backspace — wide enough for a comfortable hit target.
    static let shiftBackspace: CGFloat = 0.13
}
