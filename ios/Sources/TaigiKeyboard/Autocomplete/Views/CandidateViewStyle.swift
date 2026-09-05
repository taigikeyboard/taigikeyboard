// 候選詞 view 的樣式系統 — Style / ItemStyle 兩層,並支援 iOS 26 Liquid Glass。

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
    ///   - isFirstCandidate: 是否為第一候選詞(engine ranker top, index 0) — 填滿鍵帽底色作視覺提示
    ///   - isLiquidGlassEnabled: 是否啟用 Liquid Glass
    ///   - firstCandidateThemeColor: 漸層主題推導的第一候選 highlight 色(nil = 非漸層主題,走中性 fallback)
    ///   - pressedThemeColor: 漸層主題推導的按壓色(nil = 非漸層主題,走中性 fallback)
    /// - Returns: 實際的背景色
    func resolvedBackgroundColor(
        for colorScheme: ColorScheme,
        isSelected: Bool = false,
        isPressed: Bool = false,
        isFirstCandidate: Bool = false,
        isLiquidGlassEnabled: Bool = false,
        firstCandidateThemeColor: Color? = nil,
        pressedThemeColor: Color? = nil,
    ) -> Color {
        // 狀態優先序(全主題一致):實際按壓(isPressed)→ 深 pressed;第一候選(isFirstCandidate)
        // → 淺 highlight,即使它被選中 —— 打字時引擎把 selectedCandidateIndex 設為 0,第一候選
        // 恆為「選中」,但仍要顯示淺色 hint;其他被選中候選(硬體導航)→ 深 pressed;其餘 → idle。
        // 漸層主題用推導的主題色蓋過中性色;非漸層主題(themeColor == nil)走下方中性 KeyboardKit fallback。
        if isPressed, let pressedThemeColor {
            return pressedThemeColor
        }
        if isFirstCandidate, let firstCandidateThemeColor {
            return firstCandidateThemeColor
        }
        if isSelected, let pressedThemeColor {
            return pressedThemeColor
        }

        // 非漸層主題 fallback。同一優先序:pressed > firstCandidate > selected > idle。
        if isLiquidGlassEnabled {
            // iOS 26 Liquid Glass:透明背景讓系統效果透出;pressed/selected 用閒置色 +0.6,
            // 第一候選用較低 0.4 維持狀態層級。
            let pressedLook = (backgroundColor ?? Color.keyboardButtonBackgroundLiquid(for: colorScheme)).opacity(0.6)
            if isPressed { return pressedLook }
            if isFirstCandidate { return Color.keyboardButtonBackgroundLiquid(for: colorScheme).opacity(0.4) }
            if isSelected { return pressedLook }
            return Color.white.opacity(0.001)
        } else {
            // 參考 KeyboardKit backgroundColorPressed;第一候選填滿鍵帽底色(淺色 hint,對齊
            // Android key_bgColor 與 Rime 家族 highlighted-candidate 慣例)。
            let pressedLook = selectedBackgroundColor ?? Color.keyboardDarkButtonBackground(for: colorScheme)
            if isPressed { return pressedLook }
            if isFirstCandidate { return Color.keyboardButtonBackground }
            if isSelected { return pressedLook }
            return backgroundColor ?? Color.clear
        }
    }
}
