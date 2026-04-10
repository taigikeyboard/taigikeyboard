import KeyboardKit
import SwiftUI

/// 候選詞單元格輔助工具
///
/// 封裝候選詞顯示和處理的共用邏輯。
/// 支援 TPS（方音符號）模式：羅馬字會轉換為方音符號顯示。
enum CandidateCellHelper {
    // MARK: - 顯示文字計算

    /// 檢查是否為 TPS 佈局模式
    private static var isTPSLayout: Bool {
        SharedSettings.shared.keyboardLayoutType == .tps
    }

    /// 計算顯示的主標題
    ///
    /// TPS 模式：漢字為主標題（無漢字時 fallback 為方音符號）
    ///
    /// 一般模式：
    /// - isTranslateSwapped = false：羅馬字為主標題
    /// - isTranslateSwapped = true：漢字為主標題
    static func displayTitle(for suggestion: Autocomplete.Suggestion, isTranslateSwapped: Bool) -> String {
        // TPS mode: always show hanzi as title (fallback to TPS symbols if no hanzi)
        if isTPSLayout {
            if let subtitle = suggestion.subtitle, !subtitle.isEmpty {
                return subtitle
            }
            return TPSConverter.toTPS(suggestion.text, orMapsToER: SharedSettings.shared.isTpsOrMappedToER)
        }

        if isTranslateSwapped, let subtitle = suggestion.subtitle, !subtitle.isEmpty {
            return subtitle // 漢字為主標題
        }

        return suggestion.text
    }

    /// 計算顯示的副標題
    ///
    /// TPS 模式：無副標題（只顯示漢字）
    ///
    /// 一般模式：
    /// - isTranslateSwapped = false：漢字為副標題
    /// - isTranslateSwapped = true：羅馬字為副標題
    static func displaySubtitle(for suggestion: Autocomplete.Suggestion, isTranslateSwapped: Bool) -> String? {
        // TPS mode: no subtitle (hanzi-only display)
        if isTPSLayout {
            return nil
        }

        if isTranslateSwapped {
            return suggestion.text // 羅馬字為副標題
        }
        return suggestion.subtitle // 漢字為副標題
    }

    // MARK: - 建議處理

    /// 建立要處理的建議（處理漢羅交換和 TPS 模式）
    ///
    /// TPS 模式：輸出漢字（無漢字時 fallback 為方音符號）
    ///
    /// 一般模式：
    /// - isTranslateSwapped = false：輸出羅馬字
    /// - isTranslateSwapped = true：輸出漢字
    static func suggestionToHandle(for suggestion: Autocomplete.Suggestion, isTranslateSwapped: Bool) -> Autocomplete.Suggestion {
        // TPS mode: always output hanzi (fallback to TPS symbols if no hanzi)
        if isTPSLayout,
           let subtitle = suggestion.subtitle,
           !subtitle.isEmpty
        {
            let additionalDeleteCount = max(0, suggestion.text.count - subtitle.count)
            return Autocomplete.Suggestion(
                text: subtitle,
                title: subtitle,
                subtitle: suggestion.text,
                additionalDeleteCount: additionalDeleteCount,
                additionalInfo: suggestion.additionalInfo,
            )
        }

        // 漢字優先模式：輸出漢字
        if isTranslateSwapped,
           let subtitle = suggestion.subtitle,
           !subtitle.isEmpty
        {
            let additionalDeleteCount = max(0, suggestion.text.count - subtitle.count)
            return Autocomplete.Suggestion(
                text: subtitle,
                title: subtitle,
                subtitle: suggestion.text,
                additionalDeleteCount: additionalDeleteCount,
                additionalInfo: suggestion.additionalInfo,
            )
        }

        // TPS fallback: output TPS symbols when no hanzi
        if isTPSLayout {
            let tpsText = TPSConverter.toTPS(suggestion.text, orMapsToER: SharedSettings.shared.isTpsOrMappedToER)
            let additionalDeleteCount = max(0, suggestion.text.count - tpsText.count)
            return Autocomplete.Suggestion(
                text: tpsText,
                title: tpsText,
                subtitle: suggestion.subtitle,
                additionalDeleteCount: additionalDeleteCount,
                additionalInfo: suggestion.additionalInfo,
            )
        }

        // 一般模式：輸出羅馬字
        return suggestion
    }

    // MARK: - 字體大小計算

    /// 計算主標題字體大小
    ///
    /// TPS 模式顯示漢字，使用正常字體大小
    static func titleFontSize(isTranslateSwapped _: Bool) -> CGFloat {
        // TPS mode: displaying hanzi, use normal font size
        if isTPSLayout {
            return CandidateViewModels.UI.primaryFontSize
        }
        return CandidateViewModels.UI.primaryFontSize
    }

    /// 計算副標題字體大小
    ///
    /// TPS 模式無副標題，使用正常字體大小
    static func subtitleFontSize(isTranslateSwapped _: Bool) -> CGFloat {
        // TPS mode: no subtitle, use normal font size
        if isTPSLayout {
            return CandidateViewModels.UI.secondaryFontSize
        }
        return CandidateViewModels.UI.secondaryFontSize
    }

    // MARK: - Pixel-based cell width measurement

    static let minimumCellWidth: CGFloat = 44
    private static let cellHorizontalPadding: CGFloat = 20

    /// Measures both title and subtitle at their respective font sizes,
    /// returns max width + padding. Always measures both regardless of
    /// isTranslateSwapped so layout doesn't reflow on translate toggle.
    static func measuredCellWidth(for suggestion: Autocomplete.Suggestion) -> CGFloat {
        let titleFont = KeyboardFonts.globalUIFont(size: CandidateViewModels.UI.primaryFontSize)
        let subtitleFont = KeyboardFonts.globalUIFont(size: CandidateViewModels.UI.secondaryFontSize)

        let text = suggestion.text
        let subtitle = suggestion.subtitle ?? ""

        if SharedSettings.shared.keyboardLayoutType == .tps {
            // TPS: only hanzi title (or TPS-converted fallback), no subtitle
            let titleText = subtitle.isEmpty ? TPSConverter.toTPS(text, orMapsToER: SharedSettings.shared.isTpsOrMappedToER) : subtitle
            let w = (titleText as NSString).size(withAttributes: [.font: titleFont]).width
            return max(minimumCellWidth, w + cellHorizontalPadding)
        }

        let textW = (text as NSString).size(withAttributes: [.font: titleFont]).width
        let subtitleW = subtitle.isEmpty ? 0 : (subtitle as NSString).size(withAttributes: [.font: subtitleFont]).width
        return max(minimumCellWidth, max(textW, subtitleW) + cellHorizontalPadding)
    }

    // MARK: - 樣式判斷

    /// 判斷是否啟用 Liquid Glass 效果
    static func isLiquidGlassEnabled(cornerRadius: CGFloat?) -> Bool {
        cornerRadius == 9
    }
}
