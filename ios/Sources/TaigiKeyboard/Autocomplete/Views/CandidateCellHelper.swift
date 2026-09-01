// 中文: 候選詞 cell 的純函式工具 — 顯示文字 / commit 文字 / 寬度量測都集中在這裡。
// 中文: TPS 模式、isTranslateSwapped 與 candidateDisplayMode 由呼叫端傳入,不直接讀 SharedSettings,方便測試。

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
    /// - TPS 模式：漢字為主標題（無漢字時 fallback 為方音符號）— TPS 不理會 candidateDisplayMode
    /// - 羅馬字模式：主標題永遠是 engine `roman`（`text`）
    /// - 漢羅合用：一個標籤 `漢字 羅馬字`（無漢字時只剩羅馬字）
    /// - 一般模式：`isTranslateSwapped` 決定羅馬字 / 漢字順序
    // Arm order mirrors Android SmartbarCandidateStrip.kt / macOS CandidateCellContent:
    // TPS → romanOnly → combined → swapped → default.
    static func displayTitle(
        for suggestion: AutocompleteSuggestion,
        isTranslateSwapped: Bool,
        isTPSLayout: Bool,
        orMapsToER: Bool,
        candidateDisplayMode: CandidateDisplayMode,
    ) -> String {
        if isTPSLayout {
            if let subtitle = suggestion.subtitle, !subtitle.isEmpty {
                return subtitle
            }
            return tpsFallback(for: suggestion, orMapsToER: orMapsToER)
        }

        if candidateDisplayMode == .romanOnly {
            return suggestion.text
        }

        // CROSS-PLATFORM INVARIANT — mirrors Android candidateCellText / macOS + Windows
        // CandidateCellContent.cell: hanji first, single ASCII space. Drift causes silent
        // divergence (a different order or separator on one platform).
        if candidateDisplayMode == .combined, let subtitle = suggestion.subtitle, !subtitle.isEmpty {
            return combinedLabel(hanji: subtitle, roman: suggestion.text)
        }

        if isTranslateSwapped, let subtitle = suggestion.subtitle, !subtitle.isEmpty {
            return subtitle
        }

        return suggestion.text
    }

    /// 計算顯示的副標題
    ///
    /// - TPS 模式：無副標題
    /// - 羅馬字模式：無副標題（漢字不顯示）
    /// - 漢羅合用：無副標題（漢羅併入主標題）
    /// - 一般模式：`isTranslateSwapped` 決定副標題是羅馬字或漢字
    static func displaySubtitle(
        for suggestion: AutocompleteSuggestion,
        isTranslateSwapped: Bool,
        isTPSLayout: Bool,
        candidateDisplayMode: CandidateDisplayMode,
    ) -> String? {
        if isTPSLayout || candidateDisplayMode == .romanOnly || candidateDisplayMode == .combined {
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
        for suggestion: AutocompleteSuggestion,
        isTranslateSwapped: Bool,
        isTPSLayout: Bool,
        orMapsToER: Bool,
    ) -> AutocompleteSuggestion {
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

    // MARK: - Cell 寬度量測

    /// 量測 title 與 subtitle 於對應字體大小的寬度，回傳 max + padding
    /// 一律兩者都量，避免 translate toggle 時佈局 reflow。
    /// 漢羅合用：量測合併後的單一標籤（title 字體），比兩者各自都寬。
    ///
    /// 字體大小由呼叫端從 `CandidateTheme` 環境傳入，避免這裡依賴 `SharedSettings`。
    static func measuredCellWidth(
        for suggestion: AutocompleteSuggestion,
        isTPSLayout: Bool,
        orMapsToER: Bool,
        candidateDisplayMode: CandidateDisplayMode,
        titleFontSize: CGFloat,
        subtitleFontSize: CGFloat,
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

        if candidateDisplayMode == .combined, !subtitle.isEmpty {
            let label = combinedLabel(hanji: subtitle, roman: text)
            let width = (label as NSString).size(withAttributes: [.font: titleFont]).width
            return max(minimumCellWidth, width + cellHorizontalPadding)
        }

        let textWidth = (text as NSString).size(withAttributes: [.font: titleFont]).width
        let subtitleWidth = subtitle.isEmpty
            ? 0
            : (subtitle as NSString).size(withAttributes: [.font: subtitleFont]).width
        return max(minimumCellWidth, max(textWidth, subtitleWidth) + cellHorizontalPadding)
    }

    // MARK: - Private

    /// 漢羅合用的單一標籤：漢字在前，單一半形空白分隔。displayTitle 與寬度量測共用。
    private static func combinedLabel(hanji: String, roman: String) -> String {
        "\(hanji) \(roman)"
    }

    /// TPS fallback：把羅馬字轉為方音符號顯示。
    private static func tpsFallback(
        for suggestion: AutocompleteSuggestion,
        orMapsToER: Bool,
    ) -> String {
        RustEngineBridge.tlNumericToTPS(suggestion.text, orMapsToER: orMapsToER)
    }

    /// 以 `newText` 取代原本的 commit text，並把原本的 text 移到 subtitle 以保留 hint。
    /// `keepOriginalSubtitle = true` 時 subtitle 保留原值（TPS 無漢字 fallback 的情境）。
    private static func replacingCommitText(
        of suggestion: AutocompleteSuggestion,
        with newText: String,
        keepOriginalSubtitle: Bool = false,
    ) -> AutocompleteSuggestion {
        let additionalDeleteCount = max(0, suggestion.text.count - newText.count)
        let subtitle = keepOriginalSubtitle ? suggestion.subtitle : suggestion.text
        return AutocompleteSuggestion(
            text: newText,
            title: newText,
            subtitle: subtitle,
            additionalDeleteCount: additionalDeleteCount,
            additionalInfo: suggestion.additionalInfo,
        )
    }
}
