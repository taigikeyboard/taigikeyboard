// 中文: 鍵盤外觀顏色設定的持久化型別。Codable + UserDefaults 序列化。
// 中文: nil 欄位代表未自訂,該位置回退到 KeyboardKit 預設的動態 (light/dark) 顏色。

import Foundation
import SwiftUI
import UIKit

// MARK: - Codable Color

/// A color value that persists a single static RGBA to UserDefaults.
///
/// This deliberately stores one color for both light and dark modes.
/// When no custom color is set (`KeyboardColorSettings` field is `nil`),
/// the keyboard falls back to KeyboardKit's dynamic adaptive colors.
// 中文: 可序列化的單一 RGBA 顏色。light/dark 共用同一組值 (刻意設計);未自訂時整個欄位為 nil,讓 KeyboardKit 動態色生效。
struct CodableColor: Codable, Equatable {
    let red: Double
    let green: Double
    let blue: Double
    let alpha: Double

    // 中文: 還原成 SwiftUI Color 給渲染層使用。
    var color: Color {
        Color(red: red, green: green, blue: blue, opacity: alpha)
    }

    // 中文: 從 SwiftUI Color 透過 UIColor 萃取 RGBA 分量,寫入持久化欄位。
    init(_ color: Color) {
        let uiColor = UIColor(color)
        var r: CGFloat = 0, g: CGFloat = 0, b: CGFloat = 0, a: CGFloat = 0
        uiColor.getRed(&r, green: &g, blue: &b, alpha: &a)
        red = Double(r)
        green = Double(g)
        blue = Double(b)
        alpha = Double(a)
    }

    /// Builds an opaque color from a `0xRRGGBB` literal (any high 8 bits are
    /// ignored — pass `0xRRGGBB`, not `0xAARRGGBB`). sRGB components, matching
    /// the `Color(red:green:blue:opacity:)` reconstruction in `color`. Used by
    /// the built-in theme table; no string parse, no failure path.
    // 中文: 從 0xRRGGBB 直接算 RGBA(alpha=1),高 8 bits 忽略。內建主題表專用,免字串解析、無失敗路徑。
    init(hex: UInt32) {
        red = Double((hex >> 16) & 0xFF) / 255.0
        green = Double((hex >> 8) & 0xFF) / 255.0
        blue = Double(hex & 0xFF) / 255.0
        alpha = 1.0
    }
}

// MARK: - Theme gradient

/// A vertical (top→bottom) keyboard-background gradient. `stops` are ordered
/// top→bottom and must have ≥2 entries to render; the render layer ignores a
/// gradient with fewer than 2 stops and falls back to the flat `backgroundColor`.
/// Built-in gradient themes (Standard Blue/Green/Purple) set this; flat themes
/// leave it nil. Synthesized `Codable` — an absent key in old JSON decodes to nil.
// 中文: 鍵盤背景的垂直漸層(top→bottom),≥2 stops 才渲染;<2 退回平面 backgroundColor。內建漸層主題用,平面主題 nil。
struct ThemeGradient: Codable, Equatable {
    let stops: [CodableColor]
}

// MARK: - Keyboard Color Settings

// 中文: 鍵盤六個可自訂顏色面;任何欄位為 nil 即代表「沿用 KeyboardKit 預設」。
struct KeyboardColorSettings: Codable, Equatable {
    // 中文: 鍵盤背景色。
    var backgroundColor: CodableColor?
    // 中文: 鍵帽文字色。
    var keyTextColor: CodableColor?
    // 中文: 一般鍵 (字母鍵) 填色。
    var normalKeyFillColor: CodableColor?
    // 中文: 特殊鍵 (Shift / Backspace / Enter 等) 填色。
    var specialKeyFillColor: CodableColor?
    // 中文: 候選詞文字色。
    var candidateTextColor: CodableColor?
    // 中文: 候選列背景色。
    var candidateBackgroundColor: CodableColor?
    // 中文: 鍵盤背景垂直漸層(top→bottom)。設了即蓋過平面 backgroundColor;候選列會轉透明讓漸層貫穿候選→底部。
    var backgroundGradient: ThemeGradient?

    // 中文: 全部欄位為 nil 的預設值,完全沿用 KeyboardKit 動態色。
    static let `default` = KeyboardColorSettings()

    /// Whether a renderable gradient is set (≥2 stops). Single source for the
    /// render branch, the liquid-glass gate, and the candidate-bar transparency.
    // 中文: 是否有可渲染的漸層(≥2 stops)。render 分支 / liquid-glass gate / 候選列透明 共用此單一判斷。
    var hasBackgroundGradient: Bool {
        (backgroundGradient?.stops.count ?? 0) >= 2
    }
}
