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

    // 中文: 全部欄位為 nil 的預設值,完全沿用 KeyboardKit 動態色。
    static let `default` = KeyboardColorSettings()
}
