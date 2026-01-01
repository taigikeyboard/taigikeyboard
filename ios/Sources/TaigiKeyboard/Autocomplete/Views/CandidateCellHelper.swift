import KeyboardKit
import SwiftUI

/// 候選詞單元格輔助工具
///
/// 封裝候選詞顯示和處理的共用邏輯。
/// 支援 TPS（台灣方音符號）模式：羅馬字會轉換為方音符號顯示。
enum CandidateCellHelper {

    // MARK: - 顯示文字計算

    /// 檢查是否為 TPS 佈局模式
    private static var isTPSLayout: Bool {
        SharedSettings.shared.keyboardLayoutType == .tps
    }

    /// 計算顯示的主標題
    ///
    /// TPS 模式：
    /// - isTranslateSwapped = false：方音符號為主標題
    /// - isTranslateSwapped = true：漢字為主標題
    ///
    /// 一般模式：
    /// - isTranslateSwapped = false：羅馬字為主標題
    /// - isTranslateSwapped = true：漢字為主標題
    static func displayTitle(for suggestion: Autocomplete.Suggestion, isTranslateSwapped: Bool) -> String {
        if isTranslateSwapped, let subtitle = suggestion.subtitle, !subtitle.isEmpty {
            return subtitle  // 漢字為主標題
        }

        // TPS 模式：將羅馬字轉換為方音符號
        if isTPSLayout {
            return TPSConverter.toTPS(suggestion.text)
        }

        return suggestion.text
    }

    /// 計算顯示的副標題
    ///
    /// TPS 模式：
    /// - isTranslateSwapped = false：漢字為副標題
    /// - isTranslateSwapped = true：方音符號為副標題
    ///
    /// 一般模式：
    /// - isTranslateSwapped = false：漢字為副標題
    /// - isTranslateSwapped = true：羅馬字為副標題
    static func displaySubtitle(for suggestion: Autocomplete.Suggestion, isTranslateSwapped: Bool) -> String? {
        if isTranslateSwapped {
            // TPS 模式：方音符號為副標題
            if isTPSLayout {
                return TPSConverter.toTPS(suggestion.text)
            }
            return suggestion.text  // 羅馬字為副標題
        }
        return suggestion.subtitle  // 漢字為副標題
    }

    // MARK: - 建議處理

    /// 建立要處理的建議（處理漢羅交換和 TPS 模式）
    ///
    /// TPS 模式：
    /// - isTranslateSwapped = false：輸出方音符號
    /// - isTranslateSwapped = true：輸出漢字
    ///
    /// 一般模式：
    /// - isTranslateSwapped = false：輸出羅馬字
    /// - isTranslateSwapped = true：輸出漢字
    static func suggestionToHandle(for suggestion: Autocomplete.Suggestion, isTranslateSwapped: Bool) -> Autocomplete.Suggestion {
        // 漢字優先模式：輸出漢字
        if isTranslateSwapped,
           let subtitle = suggestion.subtitle,
           !subtitle.isEmpty {
            let additionalDeleteCount = max(0, suggestion.text.count - subtitle.count)
            return Autocomplete.Suggestion(
                text: subtitle,
                title: subtitle,
                subtitle: suggestion.text,
                additionalDeleteCount: additionalDeleteCount,
                additionalInfo: suggestion.additionalInfo
            )
        }

        // TPS 模式：輸出方音符號
        if isTPSLayout {
            let tpsText = TPSConverter.toTPS(suggestion.text)
            let additionalDeleteCount = max(0, suggestion.text.count - tpsText.count)
            return Autocomplete.Suggestion(
                text: tpsText,
                title: tpsText,
                subtitle: suggestion.subtitle,
                additionalDeleteCount: additionalDeleteCount,
                additionalInfo: suggestion.additionalInfo
            )
        }

        // 一般模式：輸出羅馬字
        return suggestion
    }

    // MARK: - 字體大小計算

    /// 計算主標題字體大小
    ///
    /// TPS 模式使用較小的字體（方音符號視覺上較大）
    static func titleFontSize(isTranslateSwapped: Bool) -> CGFloat {
        // TPS 模式且顯示方音符號時使用較小字體
        if isTPSLayout && !isTranslateSwapped {
            return CandidateViewModels.UI.tpsPrimaryFontSize
        }
        return CandidateViewModels.UI.primaryFontSize
    }

    /// 計算副標題字體大小
    ///
    /// TPS 模式使用較小的字體（方音符號視覺上較大）
    static func subtitleFontSize(isTranslateSwapped: Bool) -> CGFloat {
        // TPS 模式且副標題顯示方音符號時使用較小字體
        if isTPSLayout && isTranslateSwapped {
            return CandidateViewModels.UI.tpsSecondaryFontSize
        }
        return CandidateViewModels.UI.secondaryFontSize
    }

    // MARK: - 長詞字體大小計算

    /// 計算長詞主標題字體大小
    ///
    /// TPS 模式使用較小的字體（方音符號視覺上較大）
    static func longCellTitleFontSize(isTranslateSwapped: Bool) -> CGFloat {
        // TPS 模式且顯示方音符號時使用較小字體
        if isTPSLayout && !isTranslateSwapped {
            return CandidateViewModels.UI.tpsLongCellPrimaryFontSize
        }
        return CandidateViewModels.UI.longCellPrimaryFontSize
    }

    /// 計算長詞副標題字體大小
    ///
    /// TPS 模式使用較小的字體（方音符號視覺上較大）
    static func longCellSubtitleFontSize(isTranslateSwapped: Bool) -> CGFloat {
        // TPS 模式且副標題顯示方音符號時使用較小字體
        if isTPSLayout && isTranslateSwapped {
            return CandidateViewModels.UI.tpsLongCellSecondaryFontSize
        }
        return CandidateViewModels.UI.longCellSecondaryFontSize
    }

    // MARK: - 樣式判斷

    /// 判斷是否啟用 Liquid Glass 效果
    static func isLiquidGlassEnabled(cornerRadius: CGFloat?) -> Bool {
        cornerRadius == 9
    }
}
