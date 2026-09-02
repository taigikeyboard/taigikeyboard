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
    /// - 漢羅濫：單一 script — split cell 直接顯示 `text`（上游已拆分）；
    ///   未拆分的 NextWord 列漢字為主（無漢字時羅馬字）
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

        // CROSS-PLATFORM INVARIANT — mirrors the desktop split cells (§42 second
        // exception: macOS/Windows PresentedCandidate) and Android candidateCellText:
        // under 濫 every cell is single-script. Split cells (marked upstream in
        // TaigiAutocompleteService.buildContinuousSuggestions) carry their script in
        // `text`; un-split rows (NextWord predictions) render hanji-led. Drift causes
        // silent divergence (one platform re-joining the two scripts into one label).
        if candidateDisplayMode == .combined {
            if let subtitle = suggestion.subtitle, !subtitle.isEmpty {
                return subtitle
            }
            return suggestion.text
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
    /// - 漢羅濫：無副標題（split cell 上游已拆為單一 script）
    /// - 一般模式：`isTranslateSwapped` 決定副標題是羅馬字或漢字
    static func displaySubtitle(
        for suggestion: AutocompleteSuggestion,
        isTranslateSwapped: Bool,
        isTPSLayout: Bool,
        candidateDisplayMode: CandidateDisplayMode,
    ) -> String? {
        // Only side-by-side splits the two scripts into a title and a subtitle.
        if isTPSLayout || candidateDisplayMode != .sideBySide {
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
        // §42 漢羅濫 split cell: the `cellScript` marker is authoritative — the
        // cell already carries exactly the script it commits, so the swap / TPS
        // rewrites below must not touch it (a swapped rewrite would replace a
        // marked cell's text; the TPS fallback would re-render its roman).
        // 中文: 帶 cellScript 標記的 split cell 原樣送出,不做 swap / TPS 改寫。
        if suggestion.additionalInfo["cellScript"] != nil {
            return suggestion
        }

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
    /// 漢羅濫：split cell 只有單一 script（subtitle 為 nil），走預設量測 = `text` 於 title 字體。
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

        let textWidth = (text as NSString).size(withAttributes: [.font: titleFont]).width
        let subtitleWidth = subtitle.isEmpty
            ? 0
            : (subtitle as NSString).size(withAttributes: [.font: subtitleFont]).width
        return max(minimumCellWidth, max(textWidth, subtitleWidth) + cellHorizontalPadding)
    }

    // MARK: - Content-level subtitle presence

    /// Whether ANY cell in `suggestions` will actually render a subtitle line.
    ///
    /// Mirrors desktop §42 "one-script content is one line tall": the invisible
    /// subtitle spacer in `CandidateButtonView` / `ExpandedCandidateGridCell`
    /// renders only when the CONTENT has a subtitle somewhere — a mixed 並排
    /// list (one hanji-less literal among two-line cells) keeps the spacer so
    /// rows line up, while an all-single-line list (羅馬字 / 漢羅濫 / TPS)
    /// reserves nothing. The predicate matches the cells' own render condition
    /// (`displaySubtitle` non-empty and different from `displayTitle`).
    // 中文: 整份候選內容是否有任何 cell 會畫副標題 — 決定單行 cell 是否保留隱形副標空間。
    static func contentHasSubtitles(
        _ suggestions: [AutocompleteSuggestion],
        isTranslateSwapped: Bool,
        isTPSLayout: Bool,
        orMapsToER: Bool,
        candidateDisplayMode: CandidateDisplayMode,
    ) -> Bool {
        suggestions.contains { suggestion in
            guard let subtitle = displaySubtitle(
                for: suggestion,
                isTranslateSwapped: isTranslateSwapped,
                isTPSLayout: isTPSLayout,
                candidateDisplayMode: candidateDisplayMode,
            ), !subtitle.isEmpty else {
                return false
            }
            return subtitle != displayTitle(
                for: suggestion,
                isTranslateSwapped: isTranslateSwapped,
                isTPSLayout: isTPSLayout,
                orMapsToER: orMapsToER,
                candidateDisplayMode: candidateDisplayMode,
            )
        }
    }

    // MARK: - Private

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
