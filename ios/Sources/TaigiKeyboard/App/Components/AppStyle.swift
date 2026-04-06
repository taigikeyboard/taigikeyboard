import SwiftUI

/// Centralized styling constants for the main app UI.
///
/// All shared font sizes, colors, and styles used across tabs and settings
/// are defined here. Change a value here → applies everywhere.
///
/// See rules/ui-style-guide.md for the cross-platform spec.
enum AppStyle {
    /// Section header font (18pt Open Huninn).
    static var sectionHeaderFont: Font {
        KeyboardModels.Fonts.appFont(size: 18)
    }

    /// Body font (17pt Open Huninn) for row labels, body text, dialog text.
    static var bodyFont: Font {
        KeyboardModels.Fonts.appFont(.body)
    }

    /// Caption font (14pt Open Huninn) for trailing values, dates, annotations, tags, badges.
    static var captionFont: Font {
        KeyboardModels.Fonts.appFont(size: 14)
    }

    // MARK: - Colors

    /// Interactive blue for icons, links, info buttons. Android equivalent: primary.
    static var accentBlue: Color { .accentColor }

    /// Accent orange for warning and feature icons. Android equivalent: accentOrange.
    static var warningOrange: Color { .orange }
}

/// Shared section header used across all tabs and settings screens.
struct SectionHeader: View {
    let text: String

    var body: some View {
        Text(text)
            .font(AppStyle.sectionHeaderFont)
            .foregroundStyle(.secondary)
    }
}
