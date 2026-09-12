// UI-facing settings domain types — InputMode display names, FontType, KeyboardLayoutType.

import Foundation

// MARK: - Input Mode Display Name

/// Platform-side localization for `InputMode` (defined in `InputMode.swift`
/// as a Foundation-only shared-core candidate).
extension InputMode {
    // Resolved at the call site (`lang.string(mode.displayNameKey)`) so a language switch applies immediately.
    var displayNameKey: StringKey {
        switch self {
        case .poj: .settingsPojMode
        case .tl: .settingsTlMode
        case .english: .settingsEnglishMode
        case .tps: .settingsTpsMode
        }
    }
}

// MARK: - Font Type

/// Keyboard font choice — a global setting, not per-theme; every case but `.system` uses a custom PostScript font.
enum FontType: String, CaseIterable, Codable {
    case system
    case openHuninn // jf open 粉圓
    case iansui // 芫荽
    case genYoMin // 源樣明體
    case genYoGothic // 源樣烏體

    /// The factory default keyboard font. Single source for the `fontType`
    /// setting default, `resetToDefaults()`, and the appearance-settings default.
    static let keyboardDefault: FontType = .openHuninn

    /// PostScript font name for custom fonts, nil for system
    var customFontName: String? {
        switch self {
        case .system: nil
        case .openHuninn: KeyboardFonts.openHuninnFontName
        case .iansui: KeyboardFonts.iansuiFontName
        case .genYoMin: KeyboardFonts.genYoMinFontName
        case .genYoGothic: KeyboardFonts.genYoGothicFontName
        }
    }

    // Reactive: resolved at the call site via the environment store, not a non-reactive getter (Codex Q4).
    // All five names live in the `common` namespace (`.system` = `commonFontSystemDefault`).
    var displayNameKey: StringKey {
        switch self {
        case .system: .commonFontSystemDefault
        case .openHuninn: .commonFontOpenHuninn
        case .iansui: .commonFontIansui
        case .genYoMin: .commonFontGenYoMin
        case .genYoGothic: .commonFontGenYoGothic
        }
    }
}

// MARK: - Candidate Display Mode

/// How TL/POJ candidate cells render (host 拍字設定 + in-keyboard settings overlay).
///
/// `sideBySide` = today's title/subtitle pair (the swap flag decides which
/// script leads). `romanOnly` = the cell shows only the romanization and the
/// derived swap / both-scripts pair reads `false` (see `SharedSettings`).
/// `combined` = each hanji-bearing candidate splits into adjacent single-script
/// 漢字 / 羅馬字 cells (no subtitle) and a tap commits that cell's script; the
/// derived swap projects `true` as a compatibility projection (see `SharedSettings`).
/// TPS ignores the mode. Raw values are the cross-platform storage contract
/// (Android `CandidateDisplayMode.storageValue`, desktop `SettingsStore`).
// `public` like `InputMode`: the `RustEngineBridge` NextWord entry points are
// public and take it as a defaulted parameter.
public enum CandidateDisplayMode: String, CaseIterable, Codable {
    case sideBySide
    case combined
    case romanOnly

    /// Whether the cell shows any Hanji — `false` only under `.romanOnly`. Gates
    /// the effective 括號標註 flag and that setting's enabled state.
    var showsHanji: Bool {
        self != .romanOnly
    }

    /// Only side-by-side has a lead script the 文/A key can flip; the other two
    /// fix it, so the key is hidden (bottom row + expanded overlay) and the
    /// stored swap waits for the way back.
    var allowsSwapToggle: Bool {
        self == .sideBySide
    }

    /// Effective swap for a stored flag. `.combined` lists the pair hanji-first
    /// as split single-script cells: forcing the pair on is a compatibility
    /// projection, so every reader of the pair behaves as today's hanji-first
    /// mode (`behavioral-invariants.md` §42) while the committed script comes
    /// from each cell's `cellScript` marker, not the pair. `.romanOnly` has no
    /// Hanji to lead with.
    // CROSS-PLATFORM INVARIANT — mirrors android/app/src/main/java/com/siansiansu/taigikeyboard/ime/core/settings/CandidateDisplayMode.kt effectiveTranslateSwapped,
    // macos/Sources/TaigiInputMethodCore/Settings/EngineSettings.swift, windows/crates/taigi-windows-core/src/settings/engine_settings.rs.
    // Drift causes silent divergence (one platform commits roman under 合用, or hanji under 羅馬字).
    func effectiveTranslateSwapped(stored: Bool) -> Bool {
        self == .combined || (stored && showsHanji)
    }

    /// Effective 括號標註 for a stored flag — off only where there is no Hanji
    /// to bracket; `.combined` keeps it for 漢字-cell commits (`漢字 (羅馬字)`,
    /// today's swapped output) — a 羅馬字 cell ignores it.
    func effectiveOutputBothScripts(stored: Bool) -> Bool {
        stored && showsHanji
    }

    // Resolved at the call site via the environment store (same reactive
    // pattern as `FontType.displayNameKey`).
    var displayNameKey: StringKey {
        switch self {
        case .sideBySide: .settingsCandidateDisplayModeSideBySide
        case .romanOnly: .settingsCandidateDisplayModeRomanOnly
        case .combined: .settingsCandidateDisplayModeCombined
        }
    }
}

// MARK: - Keyboard Layout Type

/// Keyboard layout. `phahTaigi` is the primary layout; `tps` is bound 1:1 to `InputMode.tps`.
enum KeyboardLayoutType: String, CaseIterable {
    case phahTaigi
    case qwerty
    case tps // Taiwanese Phonetic Symbols
    case moe1 // MOE input method layout 1
    case moe2 // MOE input method layout 2
}
