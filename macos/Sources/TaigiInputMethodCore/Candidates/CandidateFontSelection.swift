// Which typeface the candidate window draws in: one of the bundled roster, or one the user added.

import AppKit

/// The typeface the candidate window renders its two scripts in, resolved.
///
/// `CandidateFontChoice` is the roster the four platforms share, and stays
/// exactly that. This type is the desktop's extension of it: a Mac can also
/// draw in a font the user brought (`CustomFontLibrary`), which no phone
/// keyboard can, so the extension lives here rather than as a fifth case in a
/// cross-platform enum.
///
/// Carried through `CandidateMetrics` rather than resolved at each label,
/// because it is what the panel cache compares (`CandidatePanel.panel(for:)`)
/// and what the column-floor cache is keyed on: two different custom fonts have
/// to be two different values, or the second would render in the first's widths.
enum CandidateFontSelection: Hashable, Sendable {
    case builtIn(CandidateFontChoice)
    case custom(CustomFont)

    /// What a fresh install renders in.
    static let `default` = Self.builtIn(.system)

    /// What `fontType` holds while a custom font is selected. Not a
    /// `CandidateFontChoice` case: the roster is the shared one, and this
    /// spelling is deliberately outside it — an older build, or another
    /// platform, reads it back as an unknown value and falls to the system font
    /// (`SettingsStore.choice`), which is the honest answer to "a typeface this
    /// build cannot see".
    static let customRawValue = "custom"

    /// The PostScript name to ask Core Text for, or nil for the system font.
    var postScriptName: String? {
        switch self {
        case let .builtIn(choice): choice.postScriptName
        case let .custom(font): font.postScriptName
        }
    }

    /// This selection at `size`, falling back to the system font when the face
    /// did not activate — the bundled roster's rule, applied to the user's own
    /// fonts too.
    func font(ofSize size: CGFloat) -> NSFont {
        CandidateFontChoice.font(named: postScriptName, ofSize: size)
    }

    /// Whether the face is one the user added. The cell geometry asks: a
    /// bundled face's line box is known to fit the height an inline row is
    /// given, and an arbitrary one's is not (`CandidateMetrics.init`).
    var isCustom: Bool {
        if case .custom = self {
            return true
        }
        return false
    }
}
