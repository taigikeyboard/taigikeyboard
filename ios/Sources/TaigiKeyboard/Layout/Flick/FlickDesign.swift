import SwiftUI
import KeyboardKit

// MARK: - Flick 動態設計系統（參考 azooKey Design.swift）

/// Flick 鍵盤動態設計
///
/// 所有數值根據螢幕寬度動態計算，而非固定值
/// 參考 azooKey 的 TabDependentDesign 和 Design
enum FlickDesign {

    // MARK: - 鍵盤高度計算

    /// 計算鍵盤高度（參考 azooKey: 51/74 * screenWidth + 12）
    /// - Parameters:
    ///   - screenWidth: 螢幕寬度
    ///   - isPad: 是否為 iPad
    /// - Returns: 鍵盤高度
    static func keyboardHeight(screenWidth: CGFloat, isPad: Bool) -> CGFloat {
        if isPad {
            // iPad: 15/31 * width + 12
            return 15 / 31 * screenWidth + 12
        } else {
            // iPhone: 51/74 * width + 12
            return 51 / 74 * screenWidth + 12
        }
    }

    /// 計算候選詞列高度（參考 azooKey keyboardBarHeight）
    static func candidateBarHeight(keyboardHeight: CGFloat, isPad: Bool) -> CGFloat {
        let adjustedHeight = keyboardHeight - 12
        if isPad {
            return adjustedHeight * 31 / 180
        } else {
            return adjustedHeight * 37 / 204
        }
    }

    /// 計算按鍵區高度（鍵盤高度 - 候選詞列高度）
    static func keysAreaHeight(screenWidth: CGFloat, isPad: Bool) -> CGFloat {
        let totalHeight = keyboardHeight(screenWidth: screenWidth, isPad: isPad)
        let barHeight = candidateBarHeight(keyboardHeight: totalHeight, isPad: isPad)
        return totalHeight - barHeight - 12  // 12 是 padding
    }

    // MARK: - 間距計算

    /// 垂直間距（參考 azooKey: screenWidth / 50）
    static func verticalSpacing(screenWidth: CGFloat) -> CGFloat {
        screenWidth / 50
    }

    /// 水平間距（參考 azooKey 的動態公式）
    static func horizontalSpacing(screenWidth: CGFloat, columnCount: CGFloat, keyWidth: CGFloat) -> CGFloat {
        guard columnCount > 1 else { return 0 }
        let coefficient = (5 + columnCount) / (7.5 + columnCount)
        return (screenWidth - keyWidth * columnCount) / (columnCount - 1) * coefficient
    }

    // MARK: - 按鍵尺寸計算

    /// 計算按鍵寬度（參考 azooKey keyViewWidth）
    static func keyWidth(screenWidth: CGFloat, columnCount: CGFloat) -> CGFloat {
        let coefficient: CGFloat = 5 / (5.1 + columnCount / 10)
        return screenWidth / columnCount * coefficient
    }

    /// 計算按鍵高度
    static func keyHeight(keysAreaHeight: CGFloat, rowCount: CGFloat, verticalSpacing: CGFloat) -> CGFloat {
        (keysAreaHeight - (rowCount - 1) * verticalSpacing) / rowCount
    }

    /// 計算完整的按鍵尺寸
    static func keySize(screenWidth: CGFloat, isPad: Bool, rowCount: Int, columnCount: Int) -> CGSize {
        let rows = CGFloat(rowCount)
        let cols = CGFloat(columnCount)

        let keysHeight = keysAreaHeight(screenWidth: screenWidth, isPad: isPad)
        let vSpacing = verticalSpacing(screenWidth: screenWidth)
        let kWidth = keyWidth(screenWidth: screenWidth, columnCount: cols)
        let kHeight = keyHeight(keysAreaHeight: keysHeight, rowCount: rows, verticalSpacing: vSpacing)

        return CGSize(width: kWidth, height: kHeight)
    }

    // MARK: - 字體大小計算

    /// 字體大小策略（參考 azooKey LabelFontSizeStrategy）
    enum FontSizeStrategy {
        case large      // 主要文字 (scale 1.0)
        case medium     // 中等 (scale 0.8)
        case small      // 較小 (scale 0.7)
        case xsmall     // 提示文字 (scale 0.6)
        case xxsmall    // 極小 (scale 0.2)

        var scale: CGFloat {
            switch self {
            case .large: return 1.0
            case .medium: return 0.8
            case .small: return 0.7
            case .xsmall: return 0.6
            case .xxsmall: return 0.2
            }
        }
    }

    /// 計算按鍵標籤字體大小（參考 azooKey getMaximumFontSize）
    /// - Parameters:
    ///   - text: 文字內容
    ///   - width: 可用寬度
    ///   - strategy: 字體大小策略
    /// - Returns: 適合的字體大小
    static func keyLabelFontSize(text: String, width: CGFloat, strategy: FontSizeStrategy) -> CGFloat {
        let maxFontSize: CGFloat
        if text.count == 1 {
            maxFontSize = 25 * strategy.scale
        } else {
            maxFontSize = 22 * strategy.scale
        }

        // 二分搜尋找到最大能放入的字體大小
        var lowerBound: CGFloat = 5
        var upperBound = maxFontSize

        while lowerBound < upperBound {
            let mid = (lowerBound + upperBound + 1) / 2
            let font = UIFont.systemFont(ofSize: mid, weight: .regular)
            let textSize = (text as NSString).size(withAttributes: [.font: font])

            if textSize.width < width * 0.95 {
                lowerBound = mid
            } else {
                upperBound = mid - 1
            }
        }

        return lowerBound
    }

    /// 取得主要標籤字體
    static func mainLabelFont(text: String, width: CGFloat, weight: Font.Weight = .regular) -> Font {
        let size = keyLabelFontSize(text: text, width: width, strategy: .large)
        return .system(size: size, weight: weight)
    }

    /// 取得提示標籤字體
    static func hintLabelFont(text: String, width: CGFloat, weight: Font.Weight = .regular) -> Font {
        let size = keyLabelFontSize(text: text, width: width, strategy: .xsmall)
        return .system(size: size, weight: weight)
    }

    /// 取得副標籤字體（用於 symbols 模式）
    static func subLabelFont(text: String, width: CGFloat, weight: Font.Weight = .regular) -> Font {
        let size = keyLabelFontSize(text: text, width: width, strategy: .xxsmall)
        return .system(size: size, weight: weight)
    }
}

// MARK: - Flick 顏色定義（azooKey 實際值）

enum FlickColors {
    // MARK: - 按鍵背景色（從 azooKey colorset 提取）

    /// 普通按鍵背景色 - azooKey NormalKeyColor
    /// Light: #FFFFFF, Dark: #1E1E1E
    static var normalKey: Color {
        Color(light: .white, dark: Color(white: 0.118))
    }

    /// 特殊按鍵背景色 - azooKey TabKeyColor_iOS15
    /// Light: RGB(0.776, 0.784, 0.810) = #C6C8CF
    /// Dark: RGB(0.110, 0.110, 0.118) = #1C1C1E
    static var specialKey: Color {
        Color(
            light: Color(red: 0.776, green: 0.784, blue: 0.810),
            dark: Color(red: 0.110, green: 0.110, blue: 0.118)
        )
    }

    /// 空白鍵背景色（同普通按鍵）
    static var spaceKey: Color { normalKey }

    /// Enter 鍵背景色
    static var enterKey: Color {
        Color(light: Color(hex: "#007AFF"), dark: Color(hex: "#0A84FF"))
    }

    /// 刪除鍵背景色（同特殊按鍵）
    static var deleteKey: Color { specialKey }

    /// 高亮背景色 - azooKey HighlightedKeyColor
    /// Light: RGB(0.929, 0.933, 0.949) = #EDEEF2
    /// Dark: RGB(0.169, 0.173, 0.180) = #2B2C2E
    static var highlighted: Color {
        Color(
            light: Color(red: 0.929, green: 0.933, blue: 0.949),
            dark: Color(red: 0.169, green: 0.173, blue: 0.180)
        )
    }

    // MARK: - 文字顏色

    /// 普通按鍵文字色
    static var normalText: Color {
        Color(light: .black, dark: .white)
    }

    /// 特殊按鍵文字色
    static var specialText: Color { normalText }

    /// Enter 鍵文字色
    static var enterText: Color { .white }

    /// 提示文字色（四向標籤）- 比主文字稍淡
    static var hintText: Color {
        Color(light: Color(white: 0.4), dark: Color(white: 0.6))
    }

    // MARK: - 鍵盤背景色

    /// 鍵盤背景色 - azooKey BackGroundColor_iOS15
    /// Light: display-p3 RGB(209, 210, 216) ≈ #D1D2D8
    /// Dark: #000000
    static var keyboardBackground: Color {
        Color(
            light: Color(red: 209/255, green: 210/255, blue: 216/255),
            dark: .black
        )
    }

    // MARK: - 提示氣泡顏色（參考 azooKey）

    /// 提示氣泡選中顏色（pointed）
    /// Light: 白色, Dark: systemGray4
    static var suggestPointed: Color {
        Color(light: .white, dark: Color(.systemGray4))
    }

    /// 提示氣泡未選中顏色（unpointed）
    static var suggestUnpointed: Color {
        Color(.systemGray5)
    }

    /// 提示氣泡陰影色
    /// Light: 灰色, Dark: 透明（無陰影）
    static var suggestShadow: Color {
        Color(light: .gray, dark: .clear)
    }
}

// MARK: - 按鍵陰影（參考 azooKey）

struct FlickKeyShadow {
    let color: Color
    let radius: CGFloat
    let x: CGFloat
    let y: CGFloat

    static let light = FlickKeyShadow(
        color: Color.black.opacity(0.25),
        radius: 0,
        x: 0,
        y: 1
    )

    static let dark = FlickKeyShadow(
        color: Color.black.opacity(0.4),
        radius: 0,
        x: 0,
        y: 1
    )

    static func adaptive(colorScheme: ColorScheme) -> FlickKeyShadow {
        colorScheme == .dark ? .dark : .light
    }
}

// MARK: - 按鍵邊框（參考 azooKey）

struct FlickKeyBorder {
    let color: Color
    let width: CGFloat

    /// 淺色模式：淡灰色邊框
    static let light = FlickKeyBorder(
        color: Color(white: 0.8),
        width: 0.5
    )

    /// 深色模式：深灰色邊框
    static let dark = FlickKeyBorder(
        color: Color(white: 0.3),
        width: 0.5
    )

    static func adaptive(colorScheme: ColorScheme) -> FlickKeyBorder {
        colorScheme == .dark ? .dark : .light
    }
}

// MARK: - 按鍵圓角

enum FlickKeyCorner {
    /// 標準圓角（參考 azooKey，固定為 6pt）
    static func radius(for keyHeight: CGFloat) -> CGFloat {
        6
    }
}

// MARK: - Color 擴展

extension Color {
    /// 根據亮/暗色模式返回對應顏色
    init(light: Color, dark: Color) {
        self.init(UIColor { traitCollection in
            traitCollection.userInterfaceStyle == .dark
                ? UIColor(dark)
                : UIColor(light)
        })
    }

    /// 從 HEX 字串建立顏色
    init(hex: String) {
        let hex = hex.trimmingCharacters(in: CharacterSet.alphanumerics.inverted)
        var int: UInt64 = 0
        Scanner(string: hex).scanHexInt64(&int)
        let a, r, g, b: UInt64
        switch hex.count {
        case 3: // RGB (12-bit)
            (a, r, g, b) = (255, (int >> 8) * 17, (int >> 4 & 0xF) * 17, (int & 0xF) * 17)
        case 6: // RGB (24-bit)
            (a, r, g, b) = (255, int >> 16, int >> 8 & 0xFF, int & 0xFF)
        case 8: // ARGB (32-bit)
            (a, r, g, b) = (int >> 24, int >> 16 & 0xFF, int >> 8 & 0xFF, int & 0xFF)
        default:
            (a, r, g, b) = (255, 0, 0, 0)
        }

        self.init(
            .sRGB,
            red: Double(r) / 255,
            green: Double(g) / 255,
            blue: Double(b) / 255,
            opacity: Double(a) / 255
        )
    }
}
