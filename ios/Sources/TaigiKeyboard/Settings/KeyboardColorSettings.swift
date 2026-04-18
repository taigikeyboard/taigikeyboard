import Foundation
import SwiftUI
import UIKit

// MARK: - Codable Color

/// A color value that persists a single static RGBA to UserDefaults.
///
/// This deliberately stores one color for both light and dark modes.
/// When no custom color is set (`KeyboardColorSettings` field is `nil`),
/// the keyboard falls back to KeyboardKit's dynamic adaptive colors.
struct CodableColor: Codable, Equatable {
    let red: Double
    let green: Double
    let blue: Double
    let alpha: Double

    var color: Color {
        Color(red: red, green: green, blue: blue, opacity: alpha)
    }

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

struct KeyboardColorSettings: Codable, Equatable {
    var backgroundColor: CodableColor?
    var keyTextColor: CodableColor?
    var normalKeyFillColor: CodableColor?
    var specialKeyFillColor: CodableColor?
    var candidateTextColor: CodableColor?
    var candidateBackgroundColor: CodableColor?

    static let `default` = KeyboardColorSettings()
}
