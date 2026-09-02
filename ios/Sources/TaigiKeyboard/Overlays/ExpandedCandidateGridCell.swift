// 中文: 展開候選詞 overlay 內單一候選 cell 的 SwiftUI view。
// 中文: 顯示主標題 + 副標題、按壓回饋、選中狀態,點選後送出 suggestion 並收合 overlay。

import KeyboardKit
import SwiftUI

/// Grid cell displaying a single candidate in the expanded overlay
// 中文: 展開候選詞 overlay 中顯示單一候選的 grid cell。
struct ExpandedCandidateGridCell: View {
    let suggestion: AutocompleteSuggestion
    let isTranslateSwapped: Bool
    let candidateDisplayMode: CandidateDisplayMode
    /// §42: whether ANY cell in the current content renders a subtitle — gates
    /// the invisible subtitle spacer below (computed once per list by the caller).
    let contentHasSubtitles: Bool
    let isTPSLayout: Bool
    let orMapsToER: Bool
    let isSelected: Bool
    /// 第一候選詞(engine ranker top, index 0) — 填滿鍵帽底色作視覺提示
    let isFirstCandidate: Bool
    let onTap: (AutocompleteSuggestion) -> Void

    @State private var isPressed: Bool = false
    @Environment(\.colorScheme) private var colorScheme
    @Environment(\.candidateViewStyle) private var style
    @Environment(\.candidateTheme) private var theme

    private var displayTitle: String {
        CandidateCellHelper.displayTitle(
            for: suggestion,
            isTranslateSwapped: isTranslateSwapped,
            isTPSLayout: isTPSLayout,
            orMapsToER: orMapsToER,
            candidateDisplayMode: candidateDisplayMode,
        )
    }

    private var displaySubtitle: String? {
        CandidateCellHelper.displaySubtitle(
            for: suggestion,
            isTranslateSwapped: isTranslateSwapped,
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
            isTranslateSwapped: isTranslateSwapped,
            isTPSLayout: isTPSLayout,
            orMapsToER: orMapsToER,
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

    var body: some View {
        Button(action: {
            onTap(CandidateCellHelper.suggestionToHandle(
                for: suggestion,
                isTranslateSwapped: isTranslateSwapped,
                isTPSLayout: isTPSLayout,
                orMapsToER: orMapsToER,
            ))
        }) {
            VStack(alignment: .center, spacing: 2) {
                Text(displayTitle)
                    .font(KeyboardFonts.globalFont(size: theme.primaryFontSize))
                    .fontWeight(.regular)
                    .foregroundColor(theme.primaryTextColor)
                    .lineLimit(1)
                    .truncationMode(.tail)

                if let subtitle = renderedSubtitle {
                    Text(subtitle)
                        .font(KeyboardFonts.globalFont(size: theme.secondaryFontSize))
                        .foregroundColor(theme.secondaryTextColor)
                        .lineLimit(1)
                        .truncationMode(.tail)
                } else if contentHasSubtitles {
                    SubtitleSpacer(fontSize: theme.secondaryFontSize)
                }
            }
            .padding(.vertical, CandidateViewModels.UI.expandedButtonVerticalPadding)
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .center)
            .background(
                RoundedRectangle(cornerRadius: 10)
                    .fill(backgroundColor)
                    .padding(.horizontal, 4)
                    .padding(.vertical, 2),
            )
            .scaleEffect(isPressed ? 0.95 : 1.0)
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .onLongPressGesture(minimumDuration: 0, maximumDistance: .infinity, pressing: { pressing in
            withAnimation(.easeInOut(duration: 0.1)) {
                isPressed = pressing
            }
        }, perform: {})
        .accessibilityLabel("\(displayTitle)\(displaySubtitle.map { ", " + $0 } ?? "")")
        .accessibilityAddTraits(isSelected ? .isSelected : [])
    }
}
