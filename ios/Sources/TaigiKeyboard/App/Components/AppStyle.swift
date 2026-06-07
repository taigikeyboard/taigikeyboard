// 中文: 主 App UI 樣式常數的集中定義(字級、顏色、間距、圓角等)。
// 中文: 所有 Tab / Settings 共用,改這裡即可全 App 套用。詳見 .claude/rules/ui-style-guide.md。

import SwiftUI

/// Centralized styling constants for the main app UI.
///
/// All shared font sizes, colors, and styles used across tabs and settings
/// are defined here. Change a value here → applies everywhere.
///
/// See .claude/rules/ui-style-guide.md for the cross-platform spec.
// 中文: App UI 樣式常數的命名空間 enum。
enum AppStyle {
    // MARK: - Font Sizes

    /// Navigation bar large title size (34pt).
    static let navBarLargeTitleSize: CGFloat = 34

    /// Navigation bar inline title size (22pt).
    static let navBarInlineTitleSize: CGFloat = 22

    /// Section header size (18pt).
    static let sectionHeaderSize: CGFloat = 18

    /// Body text size (17pt).
    static let bodySize: CGFloat = 17

    /// Caption size (17pt — aligned with body size per USER 2026-06-08).
    static let captionSize: CGFloat = 17

    // MARK: - Font Helper

    /// Font name for the main app UI (fixed to Open Huninn).
    private static let appFontName = KeyboardFonts.openHuninnFontName

    /// Returns Open Huninn font at an explicit point size.
    /// All app UI fonts should go through this or the semantic properties below.
    // 中文: 取指定點數的 Open Huninn 字型。所有 App UI 字型都該走這個或下方語意屬性。
    static func appFont(size: CGFloat) -> Font {
        .custom(appFontName, size: size)
    }

    // MARK: - Semantic Fonts

    /// Section header font (18pt Open Huninn).
    static var sectionHeaderFont: Font {
        appFont(size: sectionHeaderSize)
    }

    /// Headline font (18pt Open Huninn, bold) for card/section titles.
    static var headlineFont: Font {
        appFont(size: sectionHeaderSize).bold()
    }

    /// Body font (17pt Open Huninn) for row labels, body text, dialog text.
    static var bodyFont: Font {
        appFont(size: bodySize)
    }

    /// Caption font (14pt Open Huninn) for trailing values, dates, annotations, tags, badges.
    static var captionFont: Font {
        appFont(size: captionSize)
    }

    // MARK: - Spacing

    /// Standard horizontal padding for sections and cards (16pt).
    static let horizontalPadding: CGFloat = 16

    /// Inner horizontal padding for compact elements like search bars (12pt).
    static let innerHorizontalPadding: CGFloat = 12

    /// Standard vertical padding between elements (8pt).
    static let verticalPadding: CGFloat = 8

    // MARK: - Corner Radius

    /// Card/container corner radius (12pt).
    static let cardCornerRadius: CGFloat = 12

    /// Preview/thumbnail corner radius (10pt).
    static let previewCornerRadius: CGFloat = 10

    /// Small image/badge corner radius (8pt).
    static let smallCornerRadius: CGFloat = 8

    // MARK: - Colors

    /// Interactive blue for icons, links, info buttons. Android equivalent: primary.
    // 中文: icon / link / info 按鈕用的互動藍。對應 Android 的 primary。
    static var accentBlue: Color {
        .accentColor
    }

    /// Accent orange for warning and feature icons. Android equivalent: accentOrange.
    // 中文: 警告與 feature icon 用的橘色。對應 Android 的 accentOrange。
    static var warningOrange: Color {
        .orange
    }
}
