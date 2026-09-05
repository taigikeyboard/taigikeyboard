// Pure-function utilities for a candidate cell — display text, commit text, and width
// measurement all live here.

import KeyboardKit
import SwiftUI

/// Pure-function helpers for a candidate cell's display.
///
/// Encapsulates display text / commit text / width measurement as pure functions. TPS mode and
/// the translate/hanji-roman swap are passed in by the caller rather than read from
/// `SharedSettings` directly, for testability and to avoid implicit coupling.
enum CandidateCellHelper {
    // MARK: - Constants

    static let minimumCellWidth: CGFloat = 44
    private static let cellHorizontalPadding: CGFloat = 20

    // MARK: - Display text

    /// The cell's main title, per display mode: TPS shows hanji (TPS symbols when there is none) and
    /// ignores `candidateDisplayMode`; 羅馬字 always shows the engine `roman` (`text`); 漢羅濫 is
    /// single-script — a split cell shows its own `text`, an un-split NextWord row is hanji-led;
    /// 並排 lets `isTranslateSwapped` pick the roman / hanji order.
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

    /// The cell's subtitle, per display mode.
    ///
    /// - TPS: no subtitle
    /// - 羅馬字: no subtitle (hanji not shown)
    /// - 漢羅濫: no subtitle (split cells are already single-script upstream)
    /// - default: `isTranslateSwapped` picks whether the subtitle is roman or hanji
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

    // MARK: - Commit suggestion

    /// The suggestion actually committed to the text proxy, resolved from layout / translate state.
    ///
    /// - TPS: prefers hanji; falls back to the TPS-symbol rendering when there is no hanji
    /// - default: `isTranslateSwapped = true` outputs hanji; otherwise outputs roman
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
        if CandidateCellScript.marker(for: suggestion) != nil {
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

    // MARK: - Cell width measurement

    /// Measures title and subtitle at their font sizes and returns max + padding. Always measures
    /// both, so a translate toggle never triggers a layout reflow. Under 漢羅濫 each cell is
    /// single-line: it measures the rendered title at the title font (a split cell's own `text`;
    /// an un-split NextWord row's hanji-led title) — measurement and render share one source.
    ///
    /// Font sizes are passed in by the caller from the `CandidateTheme` environment, so this stays
    /// free of a `SharedSettings` dependency.
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

        // §42 漢羅濫: every cell renders single-line at the TITLE font — a
        // marked split cell shows its own `text`, an un-split row (NextWord
        // prediction) shows the hanji-led title. Measure the rendered title so
        // measure and render share the source (`displayTitle`'s combined arm
        // ignores the swap flag, so `false` is safe here).
        if candidateDisplayMode == .combined {
            let titleText = displayTitle(
                for: suggestion,
                isTranslateSwapped: false,
                isTPSLayout: false,
                orMapsToER: orMapsToER,
                candidateDisplayMode: .combined,
            )
            let width = (titleText as NSString).size(withAttributes: [.font: titleFont]).width
            return max(minimumCellWidth, width + cellHorizontalPadding)
        }

        let textWidth = (text as NSString).size(withAttributes: [.font: titleFont]).width
        let subtitleWidth = subtitle.isEmpty
            ? 0
            : (subtitle as NSString).size(withAttributes: [.font: subtitleFont]).width
        return max(minimumCellWidth, max(textWidth, subtitleWidth) + cellHorizontalPadding)
    }

    // MARK: - Rendered subtitle (single spelling of the render predicate)

    /// The subtitle a cell will actually RENDER, or `nil`.
    ///
    /// Single spelling of the render predicate shared by `CandidateButtonView`,
    /// `ExpandedCandidateGridCell`, and `contentHasSubtitles`: a cell draws a
    /// subtitle line only when `displaySubtitle` is non-empty and differs from
    /// its `displayTitle` (a swapped hanji-less row's subtitle would repeat
    /// the title).
    static func renderedSubtitle(
        for suggestion: AutocompleteSuggestion,
        isTranslateSwapped: Bool,
        isTPSLayout: Bool,
        orMapsToER: Bool,
        candidateDisplayMode: CandidateDisplayMode,
    ) -> String? {
        guard let subtitle = displaySubtitle(
            for: suggestion,
            isTranslateSwapped: isTranslateSwapped,
            isTPSLayout: isTPSLayout,
            candidateDisplayMode: candidateDisplayMode,
        ), !subtitle.isEmpty else {
            return nil
        }
        let title = displayTitle(
            for: suggestion,
            isTranslateSwapped: isTranslateSwapped,
            isTPSLayout: isTPSLayout,
            orMapsToER: orMapsToER,
            candidateDisplayMode: candidateDisplayMode,
        )
        return subtitle == title ? nil : subtitle
    }

    // MARK: - Content-level subtitle presence

    /// Whether ANY cell in `suggestions` will actually render a subtitle line.
    ///
    /// Mirrors desktop §42 "one-script content is one line tall": the invisible
    /// subtitle spacer in `CandidateButtonView` / `ExpandedCandidateGridCell`
    /// renders only when the CONTENT has a subtitle somewhere — a mixed 並排
    /// list (one hanji-less literal among two-line cells) keeps the spacer so
    /// rows line up, while an all-single-line list (羅馬字 / 漢羅濫 / TPS)
    /// reserves nothing. Reads the cells' own render source
    /// (`renderedSubtitle`) so the two predicates cannot drift.
    static func contentHasSubtitles(
        _ suggestions: [AutocompleteSuggestion],
        isTranslateSwapped: Bool,
        isTPSLayout: Bool,
        orMapsToER: Bool,
        candidateDisplayMode: CandidateDisplayMode,
    ) -> Bool {
        suggestions.contains { suggestion in
            renderedSubtitle(
                for: suggestion,
                isTranslateSwapped: isTranslateSwapped,
                isTPSLayout: isTPSLayout,
                orMapsToER: orMapsToER,
                candidateDisplayMode: candidateDisplayMode,
            ) != nil
        }
    }

    // MARK: - Private

    /// TPS fallback: renders the roman romanization as TPS symbols.
    private static func tpsFallback(
        for suggestion: AutocompleteSuggestion,
        orMapsToER: Bool,
    ) -> String {
        RustEngineBridge.tlNumericToTPS(suggestion.text, orMapsToER: orMapsToER)
    }

    /// Replaces the commit text with `newText`, moving the original text to the subtitle to keep
    /// it as a hint. When `keepOriginalSubtitle = true` the subtitle keeps its own value (the TPS
    /// hanji-absent fallback case).
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

/// Invisible subtitle spacer — keeps a one-line cell's title aligned with its
/// two-line neighbors in a mixed list. The caller renders it only when the
/// surrounding content has a subtitle somewhere (§42: one-script content is
/// one line tall). Shared by `CandidateButtonView` and
/// `ExpandedCandidateGridCell`.
struct SubtitleSpacer: View {
    let fontSize: CGFloat

    var body: some View {
        Text(" ")
            .font(KeyboardFonts.globalFont(size: fontSize))
            .opacity(0)
    }
}
