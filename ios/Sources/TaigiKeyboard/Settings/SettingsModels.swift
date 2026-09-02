// 中文: 設定頁 UI 顯示用的領域型別 — InputMode 顯示名、字體 (FontType)、鍵盤排版 (KeyboardLayoutType)。
// 中文: InputMode 本體在 InputMode.swift (Foundation-only),這裡只擺顯示用 extension 與 UI-only enum。

import Foundation

// MARK: - Input Mode Display Name

/// Platform-side localization for `InputMode` (defined in `InputMode.swift`
/// as a Foundation-only shared-core candidate).
// 中文: InputMode 的 UI 端 displayName i18n key。
extension InputMode {
    // 中文: 設定頁顯示用名稱的 i18n key (POJ / TL / English / TPS)。View 端用 lang.string(mode.displayNameKey)
    // 中文: 解析,確保語言切換即時更新。Reactive: 在 call site 解析,非 class-load-time getter (對齊 FontType.displayNameKey)。
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

/// Font type
// 中文: 鍵盤字體選擇 enum。除了 system 外,其餘走自訂 PostScript 字體。
// 中文: 字型是「全域」設定(非每主題);String raw → 自動合成編解碼。
enum FontType: String, CaseIterable, Codable {
    // 中文: 系統預設字體。
    case system
    // 中文: jf open 粉圓 (Hân-jī 友善字型)。
    case openHuninn // jf open 粉圓
    // 中文: 芫荽 (Iansui),羅馬字配合的台語顯示字型。
    case iansui // 芫荽
    // 中文: 源樣明體 (Gen Yo Min),襯線字。
    case genYoMin // 源樣明體
    // 中文: 源樣烏體 (Gen Yo Gothic),非襯線字。
    case genYoGothic // 源樣烏體

    /// The factory default keyboard font. Single source for the `fontType`
    /// setting default, `resetToDefaults()`, and the appearance-settings default.
    // 中文: 原廠預設鍵盤字型。fontType 設定預設值、resetToDefaults、外觀設定預設值的單一來源。
    static let keyboardDefault: FontType = .openHuninn

    /// PostScript font name for custom fonts, nil for system
    // 中文: 自訂字體的 PostScript 名稱;system 回傳 nil 走系統預設。
    var customFontName: String? {
        switch self {
        case .system: nil
        case .openHuninn: KeyboardFonts.openHuninnFontName
        case .iansui: KeyboardFonts.iansuiFontName
        case .genYoMin: KeyboardFonts.genYoMinFontName
        case .genYoGothic: KeyboardFonts.genYoGothicFontName
        }
    }

    // 中文: 設定頁顯示用名稱的 i18n key。View 端用 lang.string(font.displayNameKey) 解析,確保語言切換即時更新。
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
// 中文: 候選詞顯示模式 — 漢羅並排 (預設) / 羅馬字 / 漢羅濫。raw value 四平台一致,勿改。
// `public` like `InputMode`: the `RustEngineBridge` NextWord entry points are
// public and take it as a defaulted parameter.
public enum CandidateDisplayMode: String, CaseIterable, Codable {
    case sideBySide
    case combined
    case romanOnly

    /// Whether the cell shows any Hanji — `false` only under `.romanOnly`. Gates
    /// the effective 括號標註 flag and that setting's enabled state.
    var showsHanji: Bool { self != .romanOnly }

    /// Only side-by-side has a lead script the 文/A key can flip; the other two
    /// fix it, so the key is inert and the stored swap waits for the way back.
    var allowsSwapToggle: Bool { self == .sideBySide }

    /// Effective swap for a stored flag. `.combined` lists the pair hanji-first
    /// as split single-script cells: forcing the pair on is a compatibility
    /// projection, so every reader of the pair behaves as today's hanji-first
    /// mode (`behavioral-invariants.md` §42) while the committed script comes
    /// from each cell's `cellScript` marker, not the pair. `.romanOnly` has no
    /// Hanji to lead with.
    // 中文: 推導 swap — 合用恆 true(投影到既有 pair)、羅馬字恆 false、並排照 stored。
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

/// Keyboard layout type
// 中文: 鍵盤排版 enum。phahTaigi 為主排版;tps 為注音符號排版,inputMode == .tps 時 1:1 連動。
enum KeyboardLayoutType: String, CaseIterable {
    // 中文: 「拍台語」原生排版 (預設)。
    case phahTaigi
    // 中文: 標準 QWERTY 排版。
    case qwerty
    // 中文: TPS 注音符號排版 (與 InputMode.tps 1:1 連動)。
    case tps // Taiwanese Phonetic Symbols
    // 中文: 教育部排版方案 1。
    case moe1 // MOE input method layout 1
    // 中文: 教育部排版方案 2。
    case moe2 // MOE input method layout 2
}
