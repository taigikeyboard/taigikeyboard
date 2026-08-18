// Which colour the candidate highlight paints with: the system's, or a fixed one.

import AppKit

/// The accent-colour choice the 外觀 pane's swatch row offers: 自動 — follow
/// the system accent, and the host app's own under Multicolour, which is the
/// behaviour MacishType ships — or one of the eight macOS accent colours,
/// presented as the same eight circles its site demos
/// (https://luke-chang.github.io/MacishType/).
///
/// `String` raw values persist through `UserDefaults` / `@AppStorage` like the
/// other candidate-window settings; unknown stored values read as `auto`.
enum CandidateAccentChoice: String, CaseIterable, Sendable {
    case auto
    case blue
    case purple
    case pink
    case red
    case orange
    case yellow
    case green
    case graphite

    /// The fixed colour this choice pins the highlight to, or nil for `auto`.
    ///
    /// The values are the raw `NSColor.controlAccentColor` of each system
    /// accent, exactly as MacishType's site states for its swatches, and the
    /// per-style brightness corrections are applied on top
    /// (`CandidateAccentColor.highlightColor`). Deliberately a FIXED colour:
    /// the system's own selection colour is dynamic — a dark-mode blue is a
    /// different blue — so a pinned swatch tracks the site's demo, close to
    /// but not byte-identical with what 自動 paints under the same accent.
    var overrideColor: NSColor? {
        switch self {
        case .auto: nil
        case .blue: NSColor(srgbRed: 0x00 / 255, green: 0x7A / 255, blue: 0xFF / 255, alpha: 1)
        case .purple: NSColor(srgbRed: 0x95 / 255, green: 0x3D / 255, blue: 0x96 / 255, alpha: 1)
        case .pink: NSColor(srgbRed: 0xF7 / 255, green: 0x4F / 255, blue: 0x9E / 255, alpha: 1)
        case .red: NSColor(srgbRed: 0xE0 / 255, green: 0x38 / 255, blue: 0x3E / 255, alpha: 1)
        case .orange: NSColor(srgbRed: 0xF7 / 255, green: 0x82 / 255, blue: 0x1B / 255, alpha: 1)
        case .yellow: NSColor(srgbRed: 0xFF / 255, green: 0xC7 / 255, blue: 0x26 / 255, alpha: 1)
        case .green: NSColor(srgbRed: 0x62 / 255, green: 0xBA / 255, blue: 0x46 / 255, alpha: 1)
        case .graphite: NSColor(srgbRed: 0x8C / 255, green: 0x8C / 255, blue: 0x8C / 255, alpha: 1)
        }
    }

    /// What the swatch's accessibility label and tooltip say.
    var labelKey: StringKey {
        switch self {
        case .auto: .settingsDisplayLanguageAutomatic
        case .blue: .macosCandidateAccentBlue
        case .purple: .macosCandidateAccentPurple
        case .pink: .macosCandidateAccentPink
        case .red: .macosCandidateAccentRed
        case .orange: .macosCandidateAccentOrange
        case .yellow: .macosCandidateAccentYellow
        case .green: .macosCandidateAccentGreen
        case .graphite: .macosCandidateAccentGraphite
        }
    }
}
