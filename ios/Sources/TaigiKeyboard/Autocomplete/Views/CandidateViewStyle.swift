// 中文: 候選詞 view 的樣式系統 — Style / ItemStyle 兩層,並支援 iOS 26 Liquid Glass。

import KeyboardKit
import SwiftUI

// 候選詞視圖樣式系統
//
// 定義候選詞視圖的樣式配置，支援 iOS 26 Liquid Glass 效果。

// MARK: - 樣式定義

extension CandidateView {
    /// 候選詞視圖的樣式配置，支援 iOS 26 Liquid Glass 效果
    struct Style: Codable, Equatable, Hashable {
        /// 創建自訂候選詞樣式
        ///
        /// - Parameters:
        ///   - height: 候選詞列高度，預設為 `CandidateTheme.standard.height`
        ///   - backgroundColor: 背景色，預設自適應
        ///   - itemStyle: 候選詞項目樣式
        init(
            height: CGFloat = CandidateTheme.standard.height,
            backgroundColor: Color? = nil,
            itemStyle: ItemStyle = .standard,
        ) {
            self.height = height
            self.backgroundColor = backgroundColor
            self.itemStyle = itemStyle
        }

        /// 候選詞列高度
        var height: CGFloat

        /// 背景色，nil 時使用自適應色彩
        var backgroundColor: Color?

        /// 候選詞項目樣式
        var itemStyle: ItemStyle
    }

    /// 候選詞項目的樣式配置
    struct ItemStyle: Codable, Equatable, Hashable {
        /// 創建候選詞項目樣式
        ///
        /// - Parameters:
        ///   - horizontalPadding: 水平內距
        ///   - verticalPadding: 垂直內距
        ///   - backgroundColor: 背景色
        ///   - selectedBackgroundColor: 選中狀態背景色
        ///   - cornerRadius: 圓角半徑
        init(
            horizontalPadding: Double = 8,
            verticalPadding: Double = 0,
            backgroundColor: Color? = nil,
            selectedBackgroundColor: Color? = nil,
            cornerRadius: CGFloat? = nil,
        ) {
            self.horizontalPadding = horizontalPadding
            self.verticalPadding = verticalPadding
            self.backgroundColor = backgroundColor
            self.selectedBackgroundColor = selectedBackgroundColor
            self.cornerRadius = cornerRadius
        }

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
}

// MARK: - 標準樣式

extension CandidateView.Style {
    /// 標準樣式
    static var standard: Self {
        .init()
    }

    /// iOS 26 Liquid Glass 樣式
    static var liquidGlass: Self {
        .init(itemStyle: .liquidGlass)
    }
}

extension CandidateView.ItemStyle {
    /// 標準項目樣式
    static var standard: Self {
        .init()
    }

    /// iOS 26 Liquid Glass 項目樣式
    static var liquidGlass: Self {
        .init(
            cornerRadius: 9, // 與 KeyboardKit 的 Liquid Glass 圓角一致
        )
    }
}

// MARK: - 樣式工具

extension CandidateView.Style {
    /// 根據 KeyboardContext 動態選擇樣式
    ///
    /// - Parameter context: 鍵盤上下文
    /// - Returns: 適合的樣式
    static func adaptive(for context: KeyboardContext) -> Self {
        if context.isLiquidGlassEnabled {
            .liquidGlass
        } else {
            .standard
        }
    }

    /// 是否啟用 iOS 26 Liquid Glass 模式
    ///
    /// 以 `itemStyle.cornerRadius == 9 && backgroundColor == nil` 作為單一判斷來源，
    /// 供 `CandidateView` / `ExpandedCandidateOverlay` / `CandidateButtonView` 共用，
    /// 避免同一規則散落各處。
    var isLiquidGlassEnabled: Bool {
        itemStyle.cornerRadius == 9 && backgroundColor == nil
    }

    /// 解析候選詞列的背景色（支援 iOS 26 Liquid Glass 透明效果）
    func resolvedBarBackground(for colorScheme: ColorScheme) -> Color {
        if isLiquidGlassEnabled {
            // iOS 26 Liquid Glass：使用極低透明度保持觸控功能，同時讓系統 Liquid Glass 透出
            return Color.white.opacity(0.001)
        }
        return backgroundColor ?? Color.keyboardBackground(for: colorScheme)
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
        isLiquidGlassEnabled: Bool = false,
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
