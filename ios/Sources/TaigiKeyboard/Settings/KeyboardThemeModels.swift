// 中文: 鍵盤主題模型 — 主題身分 id、內建主題、使用者自訂主題、解析後的渲染主題。
// 中文: BuiltInTheme(內建主題)於 PR-2b 加入,提供 light/dark 兩套 6 角色顏色。

import Foundation
import SwiftUI

// MARK: - Theme identity

// 中文: 主題身分。"default" = 既有自由配色 buffer(SharedSettings.colorSettings);
// 中文: 未自訂用戶解析為全 nil → KeyboardKit adaptive + Liquid Glass。其餘 id 為
// 中文: UserTheme 的 UUID 字串(PR-3)或 built-in 主題 id(PR-2b)。
enum ThemeId {
    /// The legacy free-pick buffer. Uncustomized users resolve to all-nil →
    /// KeyboardKit adaptive colors + Liquid Glass; customized users keep their
    /// `colorSettings` look without any migration.
    static let `default` = "default"

    /// Whether `id` is a user theme. User-theme ids are `UUID` strings; the
    /// `default` buffer and built-in ids are not. Distinguishes themes that own
    /// their appearance (incl. explicit shadow) from `default` / built-in themes
    /// that inherit KeyboardKit's standard look.
    // 中文: id 是否為自訂主題(自訂 = UUID;default / built-in 不是)。用來判斷主題是否自帶外觀(含明確陰影)。
    static func isUserTheme(_ id: String) -> Bool {
        UUID(uuidString: id) != nil
    }
}

// MARK: - Built-in theme

/// A read-only, app-bundled theme: a named palette with light and/or dark
/// 6-role color variants, resolved against the system `colorScheme` at render
/// time. `light`/`dark` are concrete `KeyboardColorSettings` (every role set);
/// a `nil` variant (e.g. Nord, which ships dark-only by design) falls back to
/// the other variant.
// 中文: 內建主題(唯讀,隨 app 打包)。每個主題帶 light/dark 兩套具名 6 角色配色,
// 中文: 渲染時依系統 colorScheme 解析;某一 variant 為 nil 時 fallback 另一套。
struct BuiltInTheme: Equatable {
    let id: String
    let displayName: String
    let light: KeyboardColorSettings?
    let dark: KeyboardColorSettings?

    /// Asset name for the card preview screenshot (sized to match the 齒盤佈局
    /// page's `layout_*_preview` assets). `nil` → fall back to the live color
    /// swatch. Scaffold themes set this and leave `light`/`dark` nil until their
    /// palettes are authored.
    // 中文: 卡片預覽截圖的 asset 名(尺寸對齊齒盤佈局頁)。nil 則退回即時色塊 swatch。
    var previewImageName: String? = nil

    /// Picks the variant for `scheme`, falling back to the other variant when
    /// one is absent. `.default` (all-nil → KeyboardKit adaptive) is the final
    /// fallback only for a malformed entry with neither variant.
    // 中文: 依 colorScheme 取對應 variant;缺一套時退到另一套;兩套皆缺才退 .default。
    func colors(for scheme: ColorScheme) -> KeyboardColorSettings {
        switch scheme {
        case .dark: dark ?? light ?? .default
        case .light: light ?? dark ?? .default
        @unknown default: light ?? dark ?? .default
        }
    }
}

// MARK: - Theme appearance bundle

/// The full set of appearance values a theme captures: 6-role colors plus the
/// key-shadow intensity and the five size scalars (key height / key font /
/// candidate font / corner radius / border width).
///
/// One bundle is the unit of (a) what a `UserTheme` stores, (b) what the
/// `ThemeResolver` returns, and (c) what the renderer reads — so the values are
/// never spread field-by-field across resolver / snapshot / editor.
///
/// Font is intentionally NOT part of a theme: it is a GLOBAL setting
/// (`SharedSettings.fontType`), so switching themes never changes the font.
///
/// `colors` stays OPTIONAL per role (reuses `KeyboardColorSettings`): a `nil`
/// role inherits KeyboardKit's adaptive color, preserving the "customize 2 of 6"
/// behavior. The defaults match `ThemeDefaults`.
// 中文: 主題外觀整包 — 6 角色配色 + 陰影 + 5 尺寸 scalar。字型不屬於主題(全域設定),切主題不改字型。
// 中文: 同一個型別同時是「UserTheme 儲存的內容」「resolver 回傳的結果」「render 讀的值」,避免各欄到處平鋪。
struct ThemeAppearance: Codable, Equatable {
    var colors: KeyboardColorSettings
    var keyShadowIntensity: Double
    var keyHeightScale: Double
    var keyFontSizeScale: Double
    var candidateTextSizeScale: Double
    var keyCornerRadius: Double
    var keyBorderWidth: Double

    /// Factory appearance — all-nil adaptive colors, flat shadow, unity scales,
    /// project-default corner radius / border. Used as the base for built-in
    /// themes (which only define colors) and as the missing-field fallback when
    /// decoding.
    // 中文: 原廠外觀。內建主題(只定義配色)以此為底;decode 缺欄位也退回這裡。
    static let `default` = ThemeAppearance(
        colors: .default,
        keyShadowIntensity: 0,
        keyHeightScale: 1,
        keyFontSizeScale: 1,
        candidateTextSizeScale: 1,
        keyCornerRadius: 6,
        keyBorderWidth: 0,
    )

}

// 中文: 向前相容 decode 放在 extension,讓 struct 仍自動合成 memberwise init
// 中文: (ThemeAppearance.default / legacyAppearance / 測試等皆以 memberwise 建構)。
extension ThemeAppearance {
    /// Forward-compatible decode: any field absent in a stored theme falls back
    /// to the project default, so a future appearance field never strands themes
    /// written by an older build. (Encode + memberwise init are synthesized.)
    // 中文: 缺欄位退回 default,未來新增外觀欄位不會讓舊主題檔解碼失敗。
    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        let fallback = ThemeAppearance.default
        colors = try container.decodeIfPresent(KeyboardColorSettings.self, forKey: .colors) ?? fallback.colors
        keyShadowIntensity = try container.decodeIfPresent(Double.self, forKey: .keyShadowIntensity) ?? fallback.keyShadowIntensity
        keyHeightScale = try container.decodeIfPresent(Double.self, forKey: .keyHeightScale) ?? fallback.keyHeightScale
        keyFontSizeScale = try container.decodeIfPresent(Double.self, forKey: .keyFontSizeScale) ?? fallback.keyFontSizeScale
        candidateTextSizeScale = try container.decodeIfPresent(Double.self, forKey: .candidateTextSizeScale) ?? fallback.candidateTextSizeScale
        keyCornerRadius = try container.decodeIfPresent(Double.self, forKey: .keyCornerRadius) ?? fallback.keyCornerRadius
        keyBorderWidth = try container.decodeIfPresent(Double.self, forKey: .keyBorderWidth) ?? fallback.keyBorderWidth
        // 中文: 字型改為全域設定後不再屬於主題。舊 user_themes.json 帶 "fontType" key 仍可解碼(Codable 忽略未知 key)。
    }
}

// MARK: - User-created theme

/// A user-created, named, persisted keyboard theme: identity + name + the full
/// `ThemeAppearance` bundle + timestamps.
///
/// No backward-compat decode is needed: the user-theme write path (CRUD) has
/// never shipped (PR-3) and no seeding/migration ever wrote `user_themes.json`
/// (verified — the only `userThemeStore` mutators are the unused CRUD wrappers),
/// so no legacy envelope can exist on any device. Schema growth is handled
/// forward by `ThemeAppearance`'s `decodeIfPresent` decoder.
// 中文: 不需向後相容 decode — user-theme 寫入路徑(CRUD)從未 ship,也無 seeding 寫過檔,
// 中文: 故無 legacy 格式存在;未來欄位成長由 ThemeAppearance 的 decodeIfPresent 向前相容處理。
struct UserTheme: Codable, Equatable, Identifiable {
    let id: UUID
    var name: String
    var appearance: ThemeAppearance
    var createdAt: Date
    var updatedAt: Date
}
