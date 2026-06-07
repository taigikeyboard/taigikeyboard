// 中文: 內建主題靜態表 — 6 套 vim/編輯器配色,各帶 light/dark 兩套 6 角色顏色。
// 中文: hex 來源 docs/ui/theme-presets-brainstorm.md §5。id 為非 UUID、非 "default" 字串,
// 中文: 供 ThemeResolver 在非-UUID id 分支查表(免讀 user-theme 檔)。iOS-first 手寫表;
// 中文: Android 端落地時再評估改共用 JSON(brainstorm fork C)。

import Foundation

/// The app-bundled, read-only theme palettes. iOS source-of-truth for v3.6.2;
/// every value is gated on device dogfood for contrast/legibility per
/// `code-review-rules.md §9`.
///
/// Role mapping per `theme-presets-brainstorm.md §5`: `background` darkest →
/// `specialKeyFill` mid → `normalKeyFill` lightest; `keyText` and
/// `candidateText` share the palette's foreground tone.
// 中文: 內建主題集合。id 規則:唯一、非 "default"、非 UUID 字串(SharedSettings 以 UUID 判斷是否讀檔)。
enum BuiltInThemes {
    /// Shelf order shown to the user (after `Default`, before user themes).
    static let all: [BuiltInTheme] = [
        catppuccin, gruvbox, tokyoNight, solarized, nord, florisDefault,
    ]

    /// Looks up a built-in by id; `nil` when the id is not a built-in.
    static func theme(id: String) -> BuiltInTheme? {
        all.first { $0.id == id }
    }

    // MARK: - Palettes (hex per brainstorm §5)

    // 中文: Catppuccin — Latte(light)/ Mocha(dark)。MIT。
    private static let catppuccin = BuiltInTheme(
        id: "catppuccin",
        displayName: "Catppuccin",
        light: roles(bg: 0xEFF1F5, candidateBg: 0xE6E9EF, special: 0xCCD0DA, normal: 0xFFFFFF, text: 0x4C4F69),
        dark: roles(bg: 0x1E1E2E, candidateBg: 0x181825, special: 0x313244, normal: 0x45475A, text: 0xCDD6F4),
    )

    // 中文: Gruvbox — light / dark(暖色復古)。MIT。
    private static let gruvbox = BuiltInTheme(
        id: "gruvbox",
        displayName: "Gruvbox",
        light: roles(bg: 0xFBF1C7, candidateBg: 0xF2E5BC, special: 0xEBDBB2, normal: 0xFFFFFF, text: 0x3C3836),
        dark: roles(bg: 0x282828, candidateBg: 0x1D2021, special: 0x3C3836, normal: 0x504945, text: 0xEBDBB2),
    )

    // 中文: Tokyo Night — Day(light)/ Night(dark)。MIT。
    private static let tokyoNight = BuiltInTheme(
        id: "tokyoNight",
        displayName: "Tokyo Night",
        light: roles(bg: 0xE1E2E7, candidateBg: 0xD5D6DB, special: 0xC4C8DA, normal: 0xFFFFFF, text: 0x343B58),
        dark: roles(bg: 0x1A1B26, candidateBg: 0x16161E, special: 0x292E42, normal: 0x414868, text: 0xC0CAF5),
    )

    // 中文: Solarized — Light / Dark(經典 light/dark 配對)。BSD/MIT-style。
    private static let solarized = BuiltInTheme(
        id: "solarized",
        displayName: "Solarized",
        light: roles(bg: 0xEEE8D5, candidateBg: 0xFDF6E3, special: 0xE3DCC4, normal: 0xFDF6E3, text: 0x657B83),
        dark: roles(bg: 0x073642, candidateBg: 0x002B36, special: 0x0A3A45, normal: 0x0D4A57, text: 0x93A1A1),
    )

    // 中文: Nord — dark(Polar Night,官方僅深色)+ 自製 light(Snow Storm 色調,USER 2026-06-07)。MIT。
    private static let nord = BuiltInTheme(
        id: "nord",
        displayName: "Nord",
        light: roles(bg: 0xE5E9F0, candidateBg: 0xECEFF4, special: 0xD8DEE9, normal: 0xFFFFFF, text: 0x2E3440),
        dark: roles(bg: 0x2E3440, candidateBg: 0x2E3440, special: 0x3B4252, normal: 0x434C5E, text: 0xECEFF4),
    )

    // 中文: Floris Default — Day / Night(沿用既有 baseline)。apache-2.0。
    private static let florisDefault = BuiltInTheme(
        id: "florisDefault",
        displayName: "Floris Default",
        light: roles(bg: 0xE0E0E0, candidateBg: 0xF5F5F5, special: 0xD0D0D0, normal: 0xFFFFFF, text: 0x121212),
        dark: roles(bg: 0x212121, candidateBg: 0x212121, special: 0x313131, normal: 0x424242, text: 0xDCDCDC),
    )

    // MARK: - Helper

    /// Builds a fully-set 6-role `KeyboardColorSettings` from `0xRRGGBB` values.
    /// `keyText` and `candidateText` intentionally share one foreground tone.
    // 中文: 從 6 個 hex 組出全填的 6 角色配色;keyText 與 candidateText 共用前景色(§5 規則)。
    private static func roles(
        bg: UInt32,
        candidateBg: UInt32,
        special: UInt32,
        normal: UInt32,
        text: UInt32,
    ) -> KeyboardColorSettings {
        var colors = KeyboardColorSettings()
        colors.backgroundColor = CodableColor(hex: bg)
        colors.candidateBackgroundColor = CodableColor(hex: candidateBg)
        colors.specialKeyFillColor = CodableColor(hex: special)
        colors.normalKeyFillColor = CodableColor(hex: normal)
        colors.keyTextColor = CodableColor(hex: text)
        colors.candidateTextColor = CodableColor(hex: text)
        return colors
    }
}
