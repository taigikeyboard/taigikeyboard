// Which typeface the candidate window draws its two scripts in.

import AppKit

/// The typeface a user can pick for the candidate window.
///
/// CROSS-PLATFORM INVARIANT — mirrors ios/Sources/TaigiKeyboard/Settings/SettingsModels.swift:29.
/// Drift causes silent divergence. The roster and the raw values are iOS's
/// `FontType` verbatim, for the reason `SettingsStore.Keys` spells its keys the
/// iOS way: macOS keeps its own defaults domain, so this is not shared storage
/// — it is a deliberate schema alignment, so one choice has one name across the
/// platforms. What is deliberately NOT mirrored is the default; that divergence
/// is stated where the default lives, on `SettingsStore.Keys.fontType`.
///
/// `String` raw values so the choice persists through `UserDefaults` and
/// `@AppStorage`; an unknown stored value reads back as the default
/// (`SettingsStore.choice`).
///
/// Scope: this setting draws the CANDIDATE WINDOW. The settings window stays on
/// the system font like every other AppKit form, and the composing text in the
/// host app is the host's to draw — an input method does not choose it.
enum CandidateFontChoice: String, CaseIterable, Sendable {
    /// The OS system font — what the window drew in before this setting, and
    /// the macOS default.
    case system
    /// jf open 粉圓.
    case openHuninn
    /// 芫荽 (Iansui).
    case iansui
    /// 源樣明體 (GenYoMin), serif.
    case genYoMin
    /// 源樣烏體 (GenYoGothic), sans-serif.
    case genYoGothic

    /// The PostScript name to ask AppKit for, or nil for the system font.
    ///
    /// CROSS-PLATFORM INVARIANT — mirrors ios/Sources/TaigiKeyboard/Styling/KeyboardFonts.swift:25-31.
    /// Drift causes silent divergence. These are the names inside the font files
    /// `bundle-app.sh` copies into `Contents/Resources/Fonts`, which
    /// `ATSApplicationFontsPath` activates at launch.
    var postScriptName: String? {
        switch self {
        case .system: nil
        case .openHuninn: "jf-openhuninn-2.1"
        case .iansui: "Iansui-Regular"
        case .genYoMin: "GenYoMin2TW-R"
        case .genYoGothic: "GenYoGothic2TW-R"
        }
    }

    /// The picker row's label, resolved at the call site through the display
    /// language store so a language switch re-renders it.
    var labelKey: StringKey {
        switch self {
        case .system: .commonFontSystemDefault
        case .openHuninn: .commonFontOpenHuninn
        case .iansui: .commonFontIansui
        case .genYoMin: .commonFontGenYoMin
        case .genYoGothic: .commonFontGenYoGothic
        }
    }

    /// This choice at `size`, falling back to the system font when the bundled
    /// face did not activate. Silent on purpose: this runs once per label and
    /// once per measurement, so a line here would be a line per keystroke —
    /// `reportUnavailableFonts()` says it once instead, at launch.
    func font(ofSize size: CGFloat) -> NSFont {
        Self.font(named: postScriptName, ofSize: size)
    }

    /// The resolution itself, by name rather than by case — so the fallback has
    /// a seam a test can reach, no case being able to name a face that fails to
    /// activate while the bundle is intact.
    ///
    /// The name is checked, not just the nil-ness: `NSFont(name:)` SUBSTITUTES
    /// rather than fails for some names (`scripts/make-menubar-icon.swift`
    /// guards the same way), and a substituted face is a typeface the user did
    /// not pick — the system font is the honest answer to "this did not
    /// activate".
    static func font(named postScriptName: String?, ofSize size: CGFloat) -> NSFont {
        guard let postScriptName,
              let font = NSFont(name: postScriptName, size: size),
              font.fontName == postScriptName
        else {
            return .systemFont(ofSize: size)
        }
        return font
    }

    /// Logs the choices whose font did not activate — a mis-assembled bundle
    /// (`Contents/Resources/Fonts` incomplete, or `ATSApplicationFontsPath`
    /// gone from Info.plist), which otherwise shows only as a picker whose
    /// options all draw the same.
    ///
    /// Called at launch: font activation is a fact about the bundle, settled
    /// before any candidate is drawn, so asking once beats asking per cell.
    static func reportUnavailableFonts() {
        let missing = allCases.compactMap(\.postScriptName).filter {
            font(named: $0, ofSize: NSFont.systemFontSize).fontName != $0
        }
        guard !missing.isEmpty else { return }
        logger.error(
            "[FONT] not activated: \(missing.joined(separator: ", ")) "
                + "— is Contents/Resources/Fonts complete?",
        )
    }

    private static let logger = DebugLogger(category: "CandidateFont")
}
