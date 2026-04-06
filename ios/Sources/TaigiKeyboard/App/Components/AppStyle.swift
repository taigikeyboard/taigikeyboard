import SwiftUI

/// Centralized styling constants for the main app UI.
///
/// All shared font sizes, colors, and styles used across tabs and settings
/// are defined here. Change a value here → applies everywhere.
///
/// See rules/ui-style-guide.md for the cross-platform spec.
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

    /// Caption size (14pt).
    static let captionSize: CGFloat = 14

    /// Headline size (17pt, bold weight).
    static let headlineSize: CGFloat = 17

    // MARK: - Fonts

    /// Section header font (18pt Open Huninn).
    static var sectionHeaderFont: Font {
        KeyboardModels.Fonts.appFont(size: sectionHeaderSize)
    }

    /// Headline font (17pt Open Huninn, bold) for card/section titles.
    static var headlineFont: Font {
        KeyboardModels.Fonts.appFont(size: headlineSize).bold()
    }

    /// Body font (17pt Open Huninn) for row labels, body text, dialog text.
    static var bodyFont: Font {
        KeyboardModels.Fonts.appFont(size: bodySize)
    }

    /// Caption font (14pt Open Huninn) for trailing values, dates, annotations, tags, badges.
    static var captionFont: Font {
        KeyboardModels.Fonts.appFont(size: captionSize)
    }

    // MARK: - Colors

    /// Interactive blue for icons, links, info buttons. Android equivalent: primary.
    static var accentBlue: Color {
        .accentColor
    }

    /// Accent orange for warning and feature icons. Android equivalent: accentOrange.
    static var warningOrange: Color {
        .orange
    }
}
