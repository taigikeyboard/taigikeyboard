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
    let isExpanded: Bool
    /// 是否為 TPS 佈局模式
    let isTPSLayout: Bool
    /// TPS 模式下 `or` 是否映射為 ㄜ
    let orMapsToER: Bool

    @Environment(\.candidateViewStyle) private var style
    @Environment(\.colorScheme) private var colorScheme
    @State private var currentPage: Int = 0
    @State private var isUpButtonPressed: Bool = false
    @State private var isDownButtonPressed: Bool = false
    @State private var isTranslateButtonPressed: Bool = false

    private var isLiquidGlassEnabled: Bool {
        style.itemStyle.cornerRadius == 9 && style.backgroundColor == nil
    }

    private let logger = DebugLogger(category: "ExpandedCandidateOverlay")

    var body: some View {
        if isExpanded {
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
    }

    // MARK: - Row Layout

    private struct CandidateRowItem {
        let suggestion: Autocomplete.Suggestion
        let originalIndex: Int
        let measuredWidth: CGFloat
    }

    /// Arrange candidates into rows using pixel-based measurement
    private var arrangedRows: [[CandidateRowItem]] {
        let controlPanelWidth: CGFloat = 60
        let horizontalPadding: CGFloat = 16 // 8 left + 8 right
        let availableWidth = UIScreen.main.bounds.width - controlPanelWidth - horizontalPadding
        let itemSpacing = CandidateViewModels.UI.expandedItemSpacing

        var rows: [[CandidateRowItem]] = []
        var currentRow: [CandidateRowItem] = []
        var currentRowWidth: CGFloat = 0

        for (index, suggestion) in suggestions.enumerated() {
            let cellWidth = CandidateCellHelper.measuredCellWidth(
                for: suggestion,
                isTPSLayout: isTPSLayout,
                orMapsToER: orMapsToER,
            )
            let spacingNeeded = currentRow.isEmpty ? 0 : itemSpacing

            if !currentRow.isEmpty, (currentRowWidth + spacingNeeded + cellWidth) > availableWidth {
                rows.append(currentRow)
                currentRow = []
                currentRowWidth = 0
            }

            currentRow.append(CandidateRowItem(suggestion: suggestion, originalIndex: index, measuredWidth: cellWidth))
            currentRowWidth += (currentRow.count == 1 ? 0 : itemSpacing) + cellWidth
        }

        if !currentRow.isEmpty { rows.append(currentRow) }
        return rows
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
        .onChange(of: isExpanded) { _, expanded in
            if expanded { currentPage = 0 }
        }
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

    private func candidateRow(rowItems: [CandidateRowItem]) -> some View {
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
                ControlButton(
                    iconName: "arrowtriangle.up.fill",
                    yOffset: 2,
                    isPressed: $isUpButtonPressed,
                    action: {
                        scrollToPreviousPage { id in proxy.scrollTo(id, anchor: .top) }
                    },
                )

                ControlButton(
                    iconName: "arrowtriangle.down.fill",
                    yOffset: 16,
                    isPressed: $isDownButtonPressed,
                    action: {
                        scrollToNextPage { id in proxy.scrollTo(id, anchor: .top) }
                    },
                )

                // Hide translate button for TPS layout (always hanzi-only)
                if !isTPSLayout {
                    ControlButton(
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
            if isLiquidGlassEnabled {
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

// MARK: - Reusable Control Button

/// Icon button with press feedback animation for the expanded overlay control panel
private struct ControlButton: View {
    let iconName: String
    let yOffset: CGFloat
    @Binding var isPressed: Bool
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            Image(systemName: iconName)
                .font(KeyboardFonts.globalFont(size: 20))
                .foregroundColor(CandidateViewModels.Colors.primaryTextColor)
                .frame(width: 45, height: 45, alignment: .center)
                .background(isPressed ? Color.gray.opacity(0.3) : Color.clear)
                .scaleEffect(isPressed ? 0.95 : 1.0)
                .contentShape(Rectangle())
                .offset(y: yOffset)
        }
        .buttonStyle(.plain)
        .onLongPressGesture(minimumDuration: 0, maximumDistance: .infinity, pressing: { pressing in
            withAnimation(.easeInOut(duration: 0.1)) {
                isPressed = pressing
            }
        }, perform: {})
    }
}

// MARK: - Fixed Column Divider

/// Vertical and horizontal divider lines for the right-side control panel area
struct FixedColumnDivider: View {
    var body: some View {
        GeometryReader { geometry in
            let cellHeight: CGFloat = 58

            Path { path in
                path.move(to: CGPoint(x: 0, y: 0))
                path.addLine(to: CGPoint(x: 0, y: geometry.size.height))

                var y: CGFloat = cellHeight
                while y < geometry.size.height {
                    path.move(to: CGPoint(x: 0, y: y))
                    path.addLine(to: CGPoint(x: geometry.size.width, y: y))
                    y += cellHeight
                }
            }
            .stroke(CandidateViewModels.Colors.separatorColor, lineWidth: 0.5)
        }
        .frame(width: 60)
    }
}

/// Grid cell displaying a single candidate in the expanded overlay
struct ExpandedCandidateGridCell: View {
    let suggestion: Autocomplete.Suggestion
    let isTranslateSwapped: Bool
    let isTPSLayout: Bool
    let orMapsToER: Bool
    let isSelected: Bool
    let onTap: (Autocomplete.Suggestion) -> Void

    @State private var isPressed: Bool = false
    @Environment(\.colorScheme) private var colorScheme
    @Environment(\.candidateViewStyle) private var style

    private var displayTitle: String {
        CandidateCellHelper.displayTitle(
            for: suggestion,
            isTranslateSwapped: isTranslateSwapped,
            isTPSLayout: isTPSLayout,
            orMapsToER: orMapsToER,
        )
    }

    private var displaySubtitle: String? {
        CandidateCellHelper.displaySubtitle(
            for: suggestion,
            isTranslateSwapped: isTranslateSwapped,
            isTPSLayout: isTPSLayout,
        )
    }

    private var backgroundColor: Color {
        style.itemStyle.resolvedBackgroundColor(
            for: colorScheme,
            isSelected: isSelected,
            isPressed: isPressed,
            isLiquidGlassEnabled: CandidateCellHelper.isLiquidGlassEnabled(cornerRadius: style.itemStyle.cornerRadius),
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
                    .font(KeyboardFonts.globalFont(size: CandidateCellHelper.titleFontSize))
                    .fontWeight(.regular)
                    .foregroundColor(CandidateViewModels.Colors.primaryTextColor)
                    .lineLimit(1)
                    .truncationMode(.tail)

                if let subtitle = displaySubtitle, !subtitle.isEmpty, subtitle != displayTitle {
                    Text(subtitle)
                        .font(KeyboardFonts.globalFont(size: CandidateCellHelper.subtitleFontSize))
                        .foregroundColor(CandidateViewModels.Colors.secondaryTextColor)
                        .lineLimit(1)
                        .truncationMode(.tail)
                } else {
                    Text(" ")
                        .font(KeyboardFonts.globalFont(size: CandidateViewModels.UI.secondaryFontSize))
                        .opacity(0)
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
    }
}
