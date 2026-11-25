// A single candidate button — handles display, subtitle, press state, and the
// Liquid Glass background. Tap forwards the commit suggestion via `onTap`.

import KeyboardKit
import SwiftUI

struct CandidateButtonView: View {
    let suggestion: AutocompleteSuggestion
    let isHanjiFirst: Bool
    let candidateDisplayMode: CandidateDisplayMode
    /// §42: whether ANY cell in the current content renders a subtitle — gates
    /// the invisible subtitle spacer below (computed once per list by the caller).
    let contentHasSubtitles: Bool
    let isTPSLayout: Bool
    let isSelected: Bool
    /// The top-ranked candidate (engine ranker index 0) — fills the keycap
    /// background as a visual hint.
    let isFirstCandidate: Bool
    let onTap: (AutocompleteSuggestion) -> Void

    @State private var isPressed = false
    @Environment(\.colorScheme) private var colorScheme
    @Environment(\.candidateViewStyle) private var style
    @Environment(\.candidateTheme) private var theme

    private var displayTitle: String {
        CandidateCellHelper.displayTitle(
            for: suggestion,
            isHanjiFirst: isHanjiFirst,
            isTPSLayout: isTPSLayout,
            candidateDisplayMode: candidateDisplayMode,
        )
    }

    private var displaySubtitle: String? {
        CandidateCellHelper.displaySubtitle(
            for: suggestion,
            isHanjiFirst: isHanjiFirst,
            isTPSLayout: isTPSLayout,
            candidateDisplayMode: candidateDisplayMode,
        )
    }

    /// The subtitle this cell actually renders — single-sourced in
    /// `CandidateCellHelper.renderedSubtitle` (same predicate
    /// `contentHasSubtitles` scans with).
    private var renderedSubtitle: String? {
        CandidateCellHelper.renderedSubtitle(
            for: suggestion,
            isHanjiFirst: isHanjiFirst,
            isTPSLayout: isTPSLayout,
            candidateDisplayMode: candidateDisplayMode,
        )
    }

    private var backgroundColor: Color {
        style.itemStyle.resolvedBackgroundColor(
            for: colorScheme,
            isSelected: isSelected,
            isPressed: isPressed,
            isFirstCandidate: isFirstCandidate,
            isLiquidGlassEnabled: style.isLiquidGlassEnabled,
            firstCandidateThemeColor: theme.firstCandidateHighlightColor,
            pressedThemeColor: theme.pressedCandidateColor,
        )
    }

    private var cornerRadius: CGFloat {
        style.itemStyle.cornerRadius ?? 8
    }

    /// Gap between the highlight and the bar's top / bottom edge, proportional to the
    /// candidate text size (2.1 pt at the 21 pt default).
    static func highlightInset(primaryFontSize: CGFloat) -> CGFloat {
        primaryFontSize * 0.1
    }

    var body: some View {
        Button(action: {
            onTap(CandidateCellHelper.suggestionToHandle(
                for: suggestion,
                isHanjiFirst: isHanjiFirst,
                isTPSLayout: isTPSLayout,
            ))
        }) {
            VStack(alignment: .center, spacing: 0) {
                Text(displayTitle)
                    .font(KeyboardFonts.globalFont(size: theme.primaryFontSize))
                    .fontWeight(.regular)
                    .foregroundColor(theme.primaryTextColor)
                    .lineLimit(1)

                if let subtitle = renderedSubtitle {
                    Text(subtitle)
                        .font(KeyboardFonts.globalFont(size: theme.secondaryFontSize))
                        .foregroundColor(theme.secondaryTextColor)
                        .lineLimit(1)
                } else if contentHasSubtitles {
                    SubtitleSpacer(fontSize: theme.secondaryFontSize)
                }
            }
            .padding(.horizontal, style.itemStyle.horizontalPadding)
            .padding(.vertical, style.itemStyle.verticalPadding)
            // Fill the bar height so the highlight is sized by the bar, not by the text:
            // the old -4 pt outset + 5 pt downward offset pushed its bottom past the bar's
            // clip edge.
            .frame(maxHeight: .infinity)
        }
        .background(
            RoundedRectangle(cornerRadius: cornerRadius)
                .fill(backgroundColor)
                .padding(.horizontal, -2)
                .padding(.vertical, Self.highlightInset(primaryFontSize: theme.primaryFontSize)),
        )
        .scaleEffect(isPressed ? 0.95 : 1.0)
        .buttonStyle(PlainButtonStyle())
        .onLongPressGesture(minimumDuration: 0, maximumDistance: .infinity, pressing: { pressing in
            withAnimation(.easeInOut(duration: 0.1)) {
                isPressed = pressing
            }
        }, perform: {})
        // Explicit label with `, ` separator (matches ExpandedCandidateGridCell) instead of the
        // auto-derived title+subtitle concat; announce the navigated selection so VoiceOver does
        // not leave the highlighted candidate state purely visual.
        .accessibilityLabel(Text(verbatim: "\(displayTitle)\(displaySubtitle.map { ", " + $0 } ?? "")"))
        .accessibilityAddTraits(isSelected ? .isSelected : [])
    }
}
