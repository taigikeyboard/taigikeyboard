import KeyboardKit
import SwiftUI

/// 候選詞單元格輔助工具
///
/// 封裝候選詞顯示 / commit 文字 / 寬度量測等純函式邏輯。
/// TPS（方音符號）模式與翻譯漢羅交換皆由呼叫端以參數傳入，不在此直接讀 `SharedSettings`，
/// 以利單元測試並避免隱式耦合。
enum CandidateCellHelper {
    // MARK: - 常數

    static let minimumCellWidth: CGFloat = 44
    private static let cellHorizontalPadding: CGFloat = 20

    // MARK: - 顯示文字

    /// 計算顯示的主標題
    ///
    /// - TPS 模式：漢字為主標題（無漢字時 fallback 為方音符號）
    /// - 一般模式：`isTranslateSwapped` 決定羅馬字 / 漢字順序
    static func displayTitle(
        for suggestion: Autocomplete.Suggestion,
        isTranslateSwapped: Bool,
        isTPSLayout: Bool,
        orMapsToER: Bool,
    ) -> String {
        if isTPSLayout {
            if let subtitle = suggestion.subtitle, !subtitle.isEmpty {
                return subtitle
            }
            return tpsFallback(for: suggestion, orMapsToER: orMapsToER)
        }

        if isTranslateSwapped, let subtitle = suggestion.subtitle, !subtitle.isEmpty {
            return subtitle
        }

        return suggestion.text
    }

    /// 計算顯示的副標題
    ///
    /// - TPS 模式：無副標題
    /// - 一般模式：`isTranslateSwapped` 決定副標題是羅馬字或漢字
    static func displaySubtitle(
        for suggestion: Autocomplete.Suggestion,
        isTranslateSwapped: Bool,
        isTPSLayout: Bool,
    ) -> String? {
        if isTPSLayout {
            return nil
        }
        return isTranslateSwapped ? suggestion.text : suggestion.subtitle
    }

    // MARK: - Commit 建議

    /// 依 layout / translate 狀態決定實際 commit 給 textProxy 的 suggestion
    ///
    /// - TPS 模式：優先輸出漢字；無漢字則輸出 TPS 符號 fallback
    /// - 一般模式：`isTranslateSwapped = true` 輸出漢字；否則輸出羅馬字
    static func suggestionToHandle(
        for suggestion: Autocomplete.Suggestion,
        isTranslateSwapped: Bool,
        isTPSLayout: Bool,
        orMapsToER: Bool,
    ) -> Autocomplete.Suggestion {
        if isTPSLayout,
           let subtitle = suggestion.subtitle,
           !subtitle.isEmpty
        {
            return replacingCommitText(of: suggestion, with: subtitle)
        }

        if isTranslateSwapped,
           let subtitle = suggestion.subtitle,
           !subtitle.isEmpty
        {
            return replacingCommitText(of: suggestion, with: subtitle)
        }

        if isTPSLayout {
            let tpsText = tpsFallback(for: suggestion, orMapsToER: orMapsToER)
            return replacingCommitText(of: suggestion, with: tpsText, keepOriginalSubtitle: true)
        }

        return suggestion
    }

    // MARK: - 字體大小

    static var titleFontSize: CGFloat {
        CandidateViewModels.UI.primaryFontSize
    }

    static var subtitleFontSize: CGFloat {
        CandidateViewModels.UI.secondaryFontSize
    }

    // MARK: - Cell 寬度量測

    /// 量測 title 與 subtitle 於對應字體大小的寬度，回傳 max + padding
    /// 一律兩者都量，避免 translate toggle 時佈局 reflow。
    static func measuredCellWidth(
        for suggestion: Autocomplete.Suggestion,
        isTPSLayout: Bool,
        orMapsToER: Bool,
    ) -> CGFloat {
        let titleFont = KeyboardFonts.globalUIFont(size: titleFontSize)
        let subtitleFont = KeyboardFonts.globalUIFont(size: subtitleFontSize)

        let text = suggestion.text
        let subtitle = suggestion.subtitle ?? ""

        if isTPSLayout {
            let titleText = subtitle.isEmpty
                ? tpsFallback(for: suggestion, orMapsToER: orMapsToER)
                : subtitle
            let width = (titleText as NSString).size(withAttributes: [.font: titleFont]).width
            return max(minimumCellWidth, width + cellHorizontalPadding)
        }

        let textWidth = (text as NSString).size(withAttributes: [.font: titleFont]).width
        let subtitleWidth = subtitle.isEmpty
            ? 0
            : (subtitle as NSString).size(withAttributes: [.font: subtitleFont]).width
        return max(minimumCellWidth, max(textWidth, subtitleWidth) + cellHorizontalPadding)
    }

    // MARK: - Private

    /// TPS fallback：把羅馬字轉為方音符號顯示。
    private static func tpsFallback(
        for suggestion: Autocomplete.Suggestion,
        orMapsToER: Bool,
    ) -> String {
        TLToTPS.convert(suggestion.text, orMapsToER: orMapsToER)
    }

    /// 以 `newText` 取代原本的 commit text，並把原本的 text 移到 subtitle 以保留 hint。
    /// `keepOriginalSubtitle = true` 時 subtitle 保留原值（TPS 無漢字 fallback 的情境）。
    private static func replacingCommitText(
        of suggestion: Autocomplete.Suggestion,
        with newText: String,
        keepOriginalSubtitle: Bool = false,
    ) -> Autocomplete.Suggestion {
        let additionalDeleteCount = max(0, suggestion.text.count - newText.count)
        let subtitle = keepOriginalSubtitle ? suggestion.subtitle : suggestion.text
        return Autocomplete.Suggestion(
            text: newText,
            title: newText,
            subtitle: subtitle,
            additionalDeleteCount: additionalDeleteCount,
            additionalInfo: suggestion.additionalInfo,
        )
    }
}
