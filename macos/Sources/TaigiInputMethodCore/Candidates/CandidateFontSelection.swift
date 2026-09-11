// Which typeface the candidate window draws in: one of the bundled roster, one the user added, or one the Mac has.

import AppKit

/// The typeface the candidate window renders its two scripts in, resolved.
///
/// `CandidateFontChoice` is the roster the four platforms share, and stays
/// exactly that. This type is the desktop's extension of it: a Mac can also
/// draw in a font the user brought (`CustomFontLibrary`) or in any family the
/// OS has installed (`RegisteredFace.installedFamilies`), which no phone
/// keyboard can, so the extension lives here rather than as more cases in a
/// cross-platform enum.
///
/// Carried through `CandidateMetrics` rather than resolved at each label,
/// because it is what the panel cache compares (`CandidatePanel.panel(for:)`)
/// and what the column-floor cache is keyed on: two different custom fonts have
/// to be two different values, or the second would render in the first's widths.
enum CandidateFontSelection: Hashable, Sendable {
    case builtIn(CandidateFontChoice)
    case custom(CustomFont)
    /// A family the OS has installed, by the name it reports. Nothing is
    /// copied or registered for it: the OS is the authority on whether it
    /// exists, the way the library's directory is for a custom font.
    case installed(family: String)

    /// What a fresh install renders in.
    static let `default` = Self.builtIn(.system)

    /// What `fontType` holds while a custom font is selected. Not a
    /// `CandidateFontChoice` case: the roster is the shared one, and this
    /// spelling is deliberately outside it — an older build, or another
    /// platform, reads it back as an unknown value and falls to the system font
    /// (`SettingsStore.choice`), which is the honest answer to "a typeface this
    /// build cannot see".
    static let customRawValue = "custom"

    /// What `fontType` holds while an installed family is selected. Outside the
    /// roster for the same reason as `customRawValue`; not `"system"`, which is
    /// `CandidateFontChoice.system`'s own raw value.
    static let installedRawValue = "installed"

    /// How `RegisteredFace` is asked for this selection's face, or nil for the
    /// system font.
    var faceQuery: RegisteredFace.Query? {
        switch self {
        case let .builtIn(choice): choice.postScriptName.map(RegisteredFace.Query.postScript)
        case let .custom(font): .postScript(font.postScriptName)
        case let .installed(family): .family(family)
        }
    }

    /// This selection at `size`, falling back to the system font when the face
    /// did not activate — the bundled roster's rule, applied to the user's own
    /// fonts and to the OS's too.
    func font(ofSize size: CGFloat) -> NSFont {
        faceQuery.flatMap { RegisteredFace.font($0, ofSize: size) } ?? .systemFont(ofSize: size)
    }

    /// Whether the cell geometry has to measure this face's line box. A
    /// bundled face's is known to fit the height an inline row is given; a
    /// face the user added or the OS supplies carries no such guarantee, so
    /// its rows are sized from the font rather than from the point size
    /// (`CandidateMetrics.init`).
    var requiresLineBoxMeasurement: Bool {
        switch self {
        case .builtIn: false
        case .custom, .installed: true
        }
    }
}
