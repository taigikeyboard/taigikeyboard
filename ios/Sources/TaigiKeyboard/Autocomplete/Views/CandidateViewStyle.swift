import KeyboardKit
import SwiftUI

// MARK: - CandidateView Style System

/// 台語鍵盤候選詞視圖的樣式系統，參考 KeyboardKit 的設計模式
extension CandidateView {

    /// 候選詞視圖的樣式配置，支援 iOS 26 Liquid Glass 效果
    struct Style: Codable, Equatable, Hashable {

        /// 創建自訂候選詞樣式
        ///
        /// - Parameters:
        ///   - height: 候選詞列高度，預設 48
        ///   - backgroundColor: 背景色，預設自適應
        ///   - itemStyle: 候選詞項目樣式
        ///   - expandButtonStyle: 展開按鈕樣式
        ///   - translateButtonStyle: 翻譯切換按鈕樣式
        init(
            height: CGFloat = CandidateViewModels.UI.height,
            backgroundColor: Color? = nil,
            itemStyle: ItemStyle = .standard,
            expandButtonStyle: ButtonStyle = .standard,
            translateButtonStyle: ButtonStyle = .standard
        ) {
            self.height = height
            self.backgroundColor = backgroundColor
            self.itemStyle = itemStyle
            self.expandButtonStyle = expandButtonStyle
            self.translateButtonStyle = translateButtonStyle
        }

        /// 候選詞列高度
        var height: CGFloat

        /// 背景色，nil 時使用自適應色彩
        var backgroundColor: Color?

        /// 候選詞項目樣式
        var itemStyle: ItemStyle

        /// 展開按鈕樣式
        var expandButtonStyle: ButtonStyle

        /// 翻譯切換按鈕樣式
        var translateButtonStyle: ButtonStyle
    }

    /// 候選詞項目的樣式配置
    struct ItemStyle: Codable, Equatable, Hashable {

        /// 創建候選詞項目樣式
        ///
        /// - Parameters:
        ///   - titleFont: 主標題字體
        ///   - titleColor: 主標題顏色
        ///   - subtitleFont: 副標題字體
        ///   - subtitleColor: 副標題顏色
        ///   - horizontalPadding: 水平內距
        ///   - verticalPadding: 垂直內距
        ///   - backgroundColor: 背景色
        ///   - selectedBackgroundColor: 選中狀態背景色
        ///   - cornerRadius: 圓角半徑
        init(
            titleFont: KeyboardFont = .body,
            titleColor: Color? = nil,
            subtitleFont: KeyboardFont = .footnote,
            subtitleColor: Color? = nil,
            horizontalPadding: Double = 8,
            verticalPadding: Double = 0,
            backgroundColor: Color? = nil,
            selectedBackgroundColor: Color? = nil,
            cornerRadius: CGFloat? = nil
        ) {
            self.titleFont = titleFont
            self.titleColor = titleColor
            self.subtitleFont = subtitleFont
            self.subtitleColor = subtitleColor
            self.horizontalPadding = horizontalPadding
            self.verticalPadding = verticalPadding
            self.backgroundColor = backgroundColor
            self.selectedBackgroundColor = selectedBackgroundColor
            self.cornerRadius = cornerRadius
        }

        /// 主標題字體
        var titleFont: KeyboardFont

        /// 主標題顏色，nil 時使用自適應色彩
        var titleColor: Color?

        /// 副標題字體
        var subtitleFont: KeyboardFont

        /// 副標題顏色，nil 時使用自適應色彩
        var subtitleColor: Color?

        /// 水平內距
        var horizontalPadding: Double

        /// 垂直內距
        var verticalPadding: Double

        /// 背景色，nil 時使用自適應色彩
        var backgroundColor: Color?

        /// 選中狀態背景色，nil 時使用自適應色彩
        var selectedBackgroundColor: Color?

        /// 圓角半徑，nil 時根據版本自動調整
        var cornerRadius: CGFloat?
    }

    /// 按鈕樣式配置（用於展開按鈕和翻譯切換按鈕）
    struct ButtonStyle: Codable, Equatable, Hashable {

        /// 創建按鈕樣式
        ///
        /// - Parameters:
        ///   - iconSize: 圖示大小
        ///   - iconColor: 圖示顏色
        ///   - backgroundColor: 背景色
        ///   - pressedBackgroundColor: 按下狀態背景色
        ///   - cornerRadius: 圓角半徑
        ///   - padding: 內距
        init(
            iconSize: CGFloat = 16,
            iconColor: Color? = nil,
            backgroundColor: Color? = nil,
            pressedBackgroundColor: Color? = nil,
            cornerRadius: CGFloat? = nil,
            padding: CGFloat = 8
        ) {
            self.iconSize = iconSize
            self.iconColor = iconColor
            self.backgroundColor = backgroundColor
            self.pressedBackgroundColor = pressedBackgroundColor
            self.cornerRadius = cornerRadius
            self.padding = padding
        }

        /// 圖示大小
        var iconSize: CGFloat

        /// 圖示顏色，nil 時使用自適應色彩
        var iconColor: Color?

        /// 背景色，nil 時使用自適應色彩
        var backgroundColor: Color?

        /// 按下狀態背景色，nil 時使用自適應色彩
        var pressedBackgroundColor: Color?

        /// 圓角半徑，nil 時根據版本自動調整
        var cornerRadius: CGFloat?

        /// 內距
        var padding: CGFloat
    }
}

// MARK: - Standard Styles

extension CandidateView.Style {

    /// 標準樣式
    static var standard: Self { .init() }

    /// iOS 26 Liquid Glass 樣式
    static var liquidGlass: Self {
        .init(
            itemStyle: .liquidGlass,
            expandButtonStyle: .liquidGlass,
            translateButtonStyle: .liquidGlass
        )
    }
}

extension CandidateView.ItemStyle {

    /// 標準項目樣式
    static var standard: Self { .init() }

    /// iOS 26 Liquid Glass 項目樣式
    static var liquidGlass: Self {
        .init(
            cornerRadius: 9 // 與 KeyboardKit 的 Liquid Glass 圓角一致
        )
    }
}

extension CandidateView.ButtonStyle {

    /// 標準按鈕樣式
    static var standard: Self { .init() }

    /// iOS 26 Liquid Glass 按鈕樣式
    static var liquidGlass: Self {
        .init(
            cornerRadius: 9 // 與 KeyboardKit 的 Liquid Glass 圓角一致
        )
    }
}

// MARK: - Style Environment

/// CandidateView 樣式的環境鍵
private struct CandidateViewStyleKey: EnvironmentKey {
    static let defaultValue = CandidateView.Style.standard
}

extension EnvironmentValues {

    /// 候選詞視圖樣式
    var candidateViewStyle: CandidateView.Style {
        get { self[CandidateViewStyleKey.self] }
        set { self[CandidateViewStyleKey.self] = newValue }
    }
}

extension View {

    /// 套用候選詞視圖樣式
    ///
    /// - Parameter style: 要套用的樣式
    /// - Returns: 套用樣式後的視圖
    func candidateViewStyle(_ style: CandidateView.Style) -> some View {
        environment(\.candidateViewStyle, style)
    }
}

// MARK: - Style Utilities

extension CandidateView.Style {

    /// 根據 KeyboardContext 動態選擇樣式
    ///
    /// - Parameter context: 鍵盤上下文
    /// - Returns: 適合的樣式
    static func adaptive(for context: KeyboardContext) -> Self {
        if context.isLiquidGlassEnabled {
            return .liquidGlass
        } else {
            return .standard
        }
    }
}

extension CandidateView.ItemStyle {

    /// 取得實際的背景色（支援 Liquid Glass）
    ///
    /// - Parameters:
    ///   - colorScheme: 顏色方案
    ///   - isSelected: 是否選中
    ///   - isPressed: 是否按下
    ///   - isLiquidGlassEnabled: 是否啟用 Liquid Glass
    /// - Returns: 實際的背景色
    func resolvedBackgroundColor(
        for colorScheme: ColorScheme,
        isSelected: Bool = false,
        isPressed: Bool = false,
        isLiquidGlassEnabled: Bool = false
    ) -> Color {
        // iOS 26 Liquid Glass 策略：使用透明背景讓系統 Liquid Glass 效果透出
        if isLiquidGlassEnabled {
            if isPressed || isSelected {
                // 按下或選中時：參考 KeyboardKit 的 backgroundColorPressedLiquid
                // 使用閒置顏色加 60% 透明度
                let idleColor = backgroundColor ?? Color.keyboardButtonBackgroundLiquid(for: colorScheme)
                return idleColor.opacity(0.6)
            } else {
                // 未選中時使用極低透明度，保持觸控功能同時讓系統 Liquid Glass 透出
                return Color.white.opacity(0.001)
            }
        } else {
            // 非 Liquid Glass 模式：參考 KeyboardKit 的 backgroundColorPressed
            if isPressed || isSelected {
                // 按壓時使用深色按鈕背景（與 KeyboardKit 一致）
                return selectedBackgroundColor ?? Color.keyboardDarkButtonBackground(for: colorScheme)
            } else {
                return backgroundColor ?? Color.clear
            }
        }
    }

}

extension CandidateView.ButtonStyle {

    /// 取得實際的背景色
    ///
    /// - Parameters:
    ///   - colorScheme: 顏色方案
    ///   - isPressed: 是否按下
    /// - Returns: 實際的背景色
    func resolvedBackgroundColor(
        for colorScheme: ColorScheme,
        isPressed: Bool = false
    ) -> Color {
        if isPressed {
            return pressedBackgroundColor ?? Color.gray.opacity(0.3)
        } else {
            return backgroundColor ?? Color.clear
        }
    }
}