import KeyboardKit
import SwiftUI

/// Overlay that displays expanded candidate grid with navigation controls
struct ExpandedCandidateOverlay: View {
    let suggestions: [Autocomplete.Suggestion]
    let selectedCandidateIndex: Int
    let onSuggestionTap: (Autocomplete.Suggestion) -> Void
    let isTranslateSwapped: Bool
    let onTranslateToggle: () -> Void
    let onCollapse: () -> Void
    /// 是否為 TPS 佈局模式
    let isTPSLayout: Bool
    /// TPS 模式下 `or` 是否映射為 ㄜ
    let orMapsToER: Bool

    @Environment(\.candidateViewStyle) private var style
    @State private var currentPage: Int = 0
    @State private var isUpButtonPressed: Bool = false
    @State private var isDownButtonPressed: Bool = false
    @State private var isTranslateButtonPressed: Bool = false

    var body: some View {
        GeometryReader { geometry in
            candidateGridContent
                .frame(
                    maxWidth: .infinity,
                    minHeight: geometry.size.height,
                    maxHeight: .infinity,
                )
                .padding(.top, -4)
        }
    }

    // MARK: - Row Layout

    private var arrangedRows: [[ExpandedCandidateRowLayout.RowItem]] {
        let controlPanelWidth: CGFloat = 60
        let horizontalPadding: CGFloat = 16 // 8 left + 8 right
        let availableWidth = UIScreen.main.bounds.width - controlPanelWidth - horizontalPadding

        return ExpandedCandidateRowLayout.arrangeRows(
            suggestions: suggestions,
            availableWidth: availableWidth,
            itemSpacing: CandidateViewModels.UI.expandedItemSpacing,
            measureCellWidth: { suggestion in
                CandidateCellHelper.measuredCellWidth(
                    for: suggestion,
                    isTPSLayout: isTPSLayout,
                    orMapsToER: orMapsToER,
                )
            },
        )
    }

    // MARK: - Main Content

    private var candidateGridContent: some View {
        ScrollViewReader { proxy in
            ZStack {
                candidateScrollContent
                controlButtonPanel(proxy: proxy)
            }
        }
        .onAppear { currentPage = 0 }
        .background(backgroundView)
        .overlay(
            FixedColumnDivider()
                .padding(.top, 6)
                .padding(.trailing, 8)
                .padding(.bottom, 8),
            alignment: .topTrailing,
        )
    }

    // MARK: - Scroll Content

    private var candidateScrollContent: some View {
        let rows = arrangedRows
        return ScrollView {
            VStack(alignment: .leading, spacing: 0) {
                ForEach(Array(rows.enumerated()), id: \.offset) { rowIndex, rowItems in
                    VStack(spacing: 0) {
                        candidateRow(rowItems: rowItems)

                        if rowIndex < rows.count - 1 {
                            Divider()
                                .background(CandidateViewModels.Colors.separatorColor.opacity(0.3))
                                .padding(.leading, 8)
                                .padding(.trailing, 68)
                        }
                    }
                    .padding(.vertical, CandidateViewModels.UI.expandedRowSpacing / 2)
                }
            }
            .padding(.top, 6)
            .padding(.bottom, 8)
        }
    }

    private func candidateRow(rowItems: [ExpandedCandidateRowLayout.RowItem]) -> some View {
        HStack(spacing: CandidateViewModels.UI.expandedItemSpacing) {
            ForEach(rowItems, id: \.originalIndex) { item in
                ExpandedCandidateGridCell(
                    suggestion: item.suggestion,
                    isTranslateSwapped: isTranslateSwapped,
                    isTPSLayout: isTPSLayout,
                    orMapsToER: orMapsToER,
                    isSelected: selectedCandidateIndex == item.originalIndex,
                    onTap: { suggestion in
                        onSuggestionTap(suggestion)
                        onCollapse()
                    },
                )
                .id("candidate_\(item.originalIndex)")
                .frame(minWidth: item.measuredWidth, maxWidth: .infinity)
            }
            Spacer()
        }
        .frame(maxWidth: .infinity)
        .frame(minHeight: CandidateViewModels.UI.expandedMinRowHeight)
        .padding(.horizontal, 8)
        .padding(.trailing, 60)
    }

    // MARK: - Control Button Panel

    private func controlButtonPanel(proxy: ScrollViewProxy) -> some View {
        VStack(spacing: 0) {
            Button(action: { onCollapse() }) {
                Image(systemName: "chevron.up")
                    .font(KeyboardFonts.globalFont(size: 20))
                    .foregroundColor(CandidateViewModels.Colors.primaryTextColor)
                    .frame(width: 60, height: 56, alignment: .center)
                    .background(Color.clear)
                    .contentShape(Rectangle())
            }
            .buttonStyle(.plain)

            VStack(spacing: 3) {
                ExpandedCandidateControlButton(
                    iconName: "arrowtriangle.up.fill",
                    yOffset: 2,
                    isPressed: $isUpButtonPressed,
                    action: {
                        scrollToPreviousPage { id in proxy.scrollTo(id, anchor: .top) }
                    },
                )

                ExpandedCandidateControlButton(
                    iconName: "arrowtriangle.down.fill",
                    yOffset: 16,
                    isPressed: $isDownButtonPressed,
                    action: {
                        scrollToNextPage { id in proxy.scrollTo(id, anchor: .top) }
                    },
                )

                // Hide translate button for TPS layout (always hanzi-only)
                if !isTPSLayout {
                    ExpandedCandidateControlButton(
                        iconName: "translate",
                        yOffset: 25,
                        isPressed: $isTranslateButtonPressed,
                        action: { onTranslateToggle() },
                    )
                }
            }
            .padding(.top, 3)
        }
        .padding(.top, 6)
        .padding(.trailing, 8)
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topTrailing)
    }

    // MARK: - Background

    private var backgroundView: some View {
        Group {
            if style.isLiquidGlassEnabled {
                Color.keyboardBackground
            } else {
                (style.backgroundColor ?? Color.keyboardBackground)
                    .shadow(color: .black.opacity(0.15), radius: 6, x: 0, y: -2)
            }
        }
    }

    // MARK: - Pagination

    private func scrollToPreviousPage(_ scrollToAction: @escaping (String) -> Void) {
        let itemsPerPage = 20
        let newStartIndex = max(0, currentPage * itemsPerPage - itemsPerPage)

        if newStartIndex >= 0, newStartIndex < suggestions.count {
            currentPage = newStartIndex / itemsPerPage
            withAnimation(.easeInOut(duration: 0.3)) {
                scrollToAction("candidate_\(newStartIndex)")
            }
        }
    }

    private func scrollToNextPage(_ scrollToAction: @escaping (String) -> Void) {
        let itemsPerPage = 20
        let newStartIndex = min(suggestions.count - 1, (currentPage + 1) * itemsPerPage)

        if newStartIndex < suggestions.count {
            currentPage = newStartIndex / itemsPerPage
            withAnimation(.easeInOut(duration: 0.3)) {
                scrollToAction("candidate_\(newStartIndex)")
            }
        }
    }
}
