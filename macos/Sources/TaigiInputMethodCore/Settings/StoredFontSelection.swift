// The typeface selection as it is STORED: four keys, read and written as one.

import Foundation

/// What `fontType` and its three companion keys say together, before anything
/// has tried to load a file or find a family.
///
/// `fontType` holds a bundled face's raw value, `CandidateFontSelection
/// .customRawValue` with `customFontFile` naming the stored file, or
/// `CandidateFontSelection.installedRawValue` with `installedFontFamily`
/// naming the family and `installedFontFace` the weight of it ("" = its
/// default face). They are separate keys in one defaults domain, so every
/// reader tolerates one without the others — and one writer sets all four, so
/// a save can never leave `fontType` naming a custom or installed typeface
/// with nothing beside it, a stale companion beside another kind, or a face of
/// one family beside another family.
///
/// Mirrors `StoredFontSelection` in
/// `desktop/crates/taigi-desktop-core/src/settings/font_selection.rs`. What is
/// NOT here is turning the value into something drawable: that needs the font
/// library and Core Text, and lives with `SettingsStore.candidateFontSelection`.
enum StoredFontSelection: Hashable {
    case builtIn(CandidateFontChoice)
    /// The library file name the user selected (`CustomFontLibrary`).
    case customFile(String)
    /// The OS-installed family the user selected, by the name the OS reports,
    /// and which of its faces by its style name ("" = the family's default
    /// face — what picking its row means).
    case installedFamily(String, face: String = "")

    /// Decodes the four keys. A `custom` or `installed` with nothing beside
    /// it, or a `fontType` this build does not know, reads as the default face
    /// rather than as a half-written selection: the keys are separate writes,
    /// and a reader must survive landing between them.
    init(fontType: String, customFontFile: String, installedFontFamily: String, installedFontFace: String) {
        switch fontType {
        case CandidateFontSelection.customRawValue where !customFontFile.isEmpty:
            self = .customFile(customFontFile)
        case CandidateFontSelection.installedRawValue where !installedFontFamily.isEmpty:
            self = .installedFamily(installedFontFamily, face: installedFontFace)
        default:
            self = .builtIn(CandidateFontChoice(rawValue: fontType) ?? .system)
        }
    }

    /// What `fontType` holds for this selection.
    var fontType: String {
        switch self {
        case let .builtIn(choice): choice.rawValue
        case .customFile: CandidateFontSelection.customRawValue
        case .installedFamily: CandidateFontSelection.installedRawValue
        }
    }

    /// What `customFontFile` holds for this selection — "" unless a custom
    /// file is selected, so a bundled or installed pick clears a stale one.
    var customFontFile: String {
        if case let .customFile(fileName) = self {
            fileName
        } else {
            ""
        }
    }

    /// What `installedFontFamily` holds for this selection — "" unless an
    /// installed family is selected.
    var installedFontFamily: String {
        if case let .installedFamily(family, _) = self {
            family
        } else {
            ""
        }
    }

    /// What `installedFontFace` holds for this selection — "" unless an
    /// installed family is selected at a picked weight, so any other pick
    /// clears it.
    var installedFontFace: String {
        if case let .installedFamily(_, face) = self {
            face
        } else {
            ""
        }
    }

    /// Whether the two name the same typeface, whatever weight of it: what a
    /// picker row — one per family — is compared to the stored selection by.
    func isSameTypeface(as other: Self) -> Bool {
        if case let .installedFamily(family, _) = self, case let .installedFamily(otherFamily, _) = other {
            return family == otherFamily
        }
        return self == other
    }
}
