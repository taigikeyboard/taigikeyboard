// The typeface selection as it is STORED: three keys, read and written as one.

import Foundation

/// What `fontType` and its two companion keys say together, before anything
/// has tried to load a file or find a family.
///
/// `fontType` holds a bundled face's raw value, `CandidateFontSelection
/// .customRawValue` with `customFontFile` naming the stored file, or
/// `CandidateFontSelection.installedRawValue` with `installedFontFamily`
/// naming the family. They are separate keys in one defaults domain, so every
/// reader tolerates one without the others — and one writer sets all three, so
/// a save can never leave `fontType` naming a custom or installed typeface
/// with nothing beside it, or a stale companion beside a bundled face.
///
/// Mirrors `StoredFontSelection` in
/// `windows/crates/taigi-windows-core/src/settings/font_selection.rs`. What is
/// NOT here is turning the value into something drawable: that needs the font
/// library and Core Text, and lives with `SettingsStore.candidateFontSelection`.
enum StoredFontSelection: Hashable {
    case builtIn(CandidateFontChoice)
    /// The library file name the user selected (`CustomFontLibrary`).
    case customFile(String)
    /// The OS-installed family the user selected, by the name the OS reports.
    case installedFamily(String)

    /// Decodes the three keys. A `custom` or `installed` with nothing beside
    /// it, or a `fontType` this build does not know, reads as the default face
    /// rather than as a half-written selection: three keys are three writes,
    /// and a reader must survive landing between them.
    init(fontType: String, customFontFile: String, installedFontFamily: String) {
        switch fontType {
        case CandidateFontSelection.customRawValue where !customFontFile.isEmpty:
            self = .customFile(customFontFile)
        case CandidateFontSelection.installedRawValue where !installedFontFamily.isEmpty:
            self = .installedFamily(installedFontFamily)
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
        if case let .installedFamily(family) = self {
            family
        } else {
            ""
        }
    }
}
