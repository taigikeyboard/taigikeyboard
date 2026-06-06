// 中文: 鍵盤主題模型 — 主題身分 id、使用者自訂主題、解析後的渲染主題。
// 中文: built-in 主題表於 PR-2b 加入;此檔為 PR-2a 的主題核心型別。

import Foundation

// MARK: - Theme identity

// 中文: 主題身分。"default" = 既有自由配色 buffer(SharedSettings.colorSettings);
// 中文: 未自訂用戶解析為全 nil → KeyboardKit adaptive + Liquid Glass。其餘 id 為
// 中文: UserTheme 的 UUID 字串(PR-3)或 built-in 主題 id(PR-2b)。
enum ThemeId {
    /// The legacy free-pick buffer. Uncustomized users resolve to all-nil →
    /// KeyboardKit adaptive colors + Liquid Glass; customized users keep their
    /// `colorSettings` look without any migration.
    static let `default` = "default"
}

// MARK: - User-created theme

/// A user-created, named, persisted keyboard theme.
///
/// `colors` is OPTIONAL per role (it reuses `KeyboardColorSettings`): a `nil`
/// role inherits KeyboardKit's adaptive color, so a theme that customizes only
/// some roles stays adaptive for the rest — preserving the "customize 2 of 6"
/// behavior that the legacy free-pick buffer already supports.
// 中文: 使用者自訂主題。colors 每個角色可為 nil(沿用 KeyboardColorSettings 語意),
// 中文: 保留「只自訂部分角色」時其餘維持 adaptive 的行為。keyShadowIntensity 渲染接線於 PR-3。
struct UserTheme: Codable, Equatable, Identifiable {
    let id: UUID
    var name: String
    var colors: KeyboardColorSettings
    /// 0 = flat (project default). Independent key-shadow control (fork H1);
    /// the editor slider + render wiring land in PR-3.
    var keyShadowIntensity: Double
    var createdAt: Date
    var updatedAt: Date
}

// MARK: - Resolved render theme

/// The fully-resolved theme the renderer consumes for one (theme, colorScheme)
/// pair: concrete 6-role colors plus the key-shadow intensity. Returning the
/// bundle (not just colors) lets PR-3 add the shadow render without re-touching
/// the resolver signature.
// 中文: 渲染端實際消費的解析結果。colors 餵 6 個顏色 sink;keyShadowIntensity 於 PR-3 接入按鍵陰影。
struct ResolvedKeyboardTheme: Equatable {
    let colors: KeyboardColorSettings
    let keyShadowIntensity: Double

    static let `default` = ResolvedKeyboardTheme(colors: .default, keyShadowIntensity: 0)
}
