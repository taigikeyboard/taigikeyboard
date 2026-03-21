import KeyboardKit
import OSLog
import SwiftUI

/// 展開候選詞覆蓋層
///
/// 以網格形式顯示更多候選詞選項。
struct ExpandedCandidateOverlay: View {

    /// 所有候選詞建議
    let suggestions: [Autocomplete.Suggestion]
    /// 常用詞集合
    let frequentWords: Set<String>
    /// 當前選中的候選詞索引
    let selectedCandidateIndex: Int
    /// 點擊候選詞的回調
    let onSuggestionTap: (Autocomplete.Suggestion) -> Void
    /// 是否交換漢字與羅馬字顯示
    let isTranslateSwapped: Bool
    /// 切換翻譯模式的回調
    let onTranslateToggle: () -> Void
    /// 收合視圖的回調
    let onCollapse: () -> Void
    /// 是否處於展開狀態
    let isExpanded: Bool

    @Environment(\.candidateViewStyle) private var style
    @Environment(\.colorScheme) private var colorScheme
    @State private var currentPage: Int = 0
    @State private var upButtonPressed: Bool = false
    @State private var downButtonPressed: Bool = false
    @State private var translateButtonPressed: Bool = false


    private var isLiquidGlassEnabled: Bool {
        // 檢查是否為 Liquid Glass 樣式
        return style.itemStyle.cornerRadius == 9 && style.backgroundColor == nil
    }

    #if DEBUG
        private let logger = Logger(
            subsystem: LexiconConstants.Logging.subsystem,
            category: "ExpandedCandidateOverlay",
        )
    #endif

    var body: some View {
        Group {
            if isExpanded {
                GeometryReader { geometry in
                    candidateContentSection
                        .frame(
                            maxWidth: .infinity,
                            minHeight: geometry.size.height,
                            maxHeight: .infinity
                        )
                }
            } else {
                EmptyView()
            }
        }
    }

    /// Row item for pixel-based grid layout
    private struct RowItem {
        let suggestion: Autocomplete.Suggestion
        let originalIndex: Int
        let measuredWidth: CGFloat
    }

    /// Arrange candidates into rows using pixel-based measurement
    private var arrangedRows: [[RowItem]] {
        let availableWidth = UIScreen.main.bounds.width - 76  // 8+8 padding + 60 button panel
        let itemSpacing = CandidateViewModels.UI.expandedItemSpacing

        var rows: [[RowItem]] = []
        var currentRow: [RowItem] = []
        var currentRowWidth: CGFloat = 0

        for (index, suggestion) in suggestions.enumerated() {
            let cellWidth = CandidateCellHelper.measuredCellWidth(for: suggestion)
            let spacingNeeded = currentRow.isEmpty ? 0 : itemSpacing

            if !currentRow.isEmpty && (currentRowWidth + spacingNeeded + cellWidth) > availableWidth {
                rows.append(currentRow)
                currentRow = []
                currentRowWidth = 0
            }

            currentRow.append(RowItem(suggestion: suggestion, originalIndex: index, measuredWidth: cellWidth))
            currentRowWidth += (currentRow.count == 1 ? 0 : itemSpacing) + cellWidth
        }

        if !currentRow.isEmpty { rows.append(currentRow) }
        return rows
    }

    private var candidateContentSection: some View {
        candidateGridContent
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .padding(.top, -4) // 向上延伸網格高度 4px
            .onAppear {}
    }

    private var candidateGridContent: some View {
        ScrollViewReader { proxy in
            ZStack {
                ScrollView {
                    VStack(alignment: .leading, spacing: 0) {
                        ForEach(Array(arrangedRows.enumerated()), id: \.offset) { rowIndex, rowItems in
                            VStack(spacing: 0) {
                                HStack(spacing: CandidateViewModels.UI.expandedItemSpacing) {
                                    ForEach(rowItems, id: \.originalIndex) { item in
                                        ExpandedCandidateGridCell(
                                            suggestion: item.suggestion,
                                            isTranslateSwapped: isTranslateSwapped,
                                            isSelected: selectedCandidateIndex == item.originalIndex,
                                            onTap: { suggestion in
                                                onSuggestionTap(suggestion)
                                                onCollapse()
                                            }
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

                                if rowIndex < arrangedRows.count - 1 {
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

                VStack(spacing: 0) {
                    Button(action: {
                        onCollapse()
                    }) {
                        Image(systemName: "chevron.up")
                            .font(KeyboardModels.Fonts.globalFont(size: 20))
                            .foregroundColor(CandidateViewModels.Colors.primaryTextColor)
                            .frame(width: 60, height: 56, alignment: .center)
                            .background(Color.clear)
                            .contentShape(Rectangle())
                    }
                    .buttonStyle(.plain)

                    VStack(spacing: 3) {
                        Button(action: {
                            scrollToPreviousPage { id in
                                proxy.scrollTo(id, anchor: .top)
                            }
                        }) {
                            Image(systemName: "arrowtriangle.up.fill")
                                .font(KeyboardModels.Fonts.globalFont(size: 20))
                                .foregroundColor(CandidateViewModels.Colors.primaryTextColor)
                                .frame(width: 45, height: 45, alignment: .center)
                                .background(upButtonPressed ? Color.gray.opacity(0.3) : Color.clear)
                                .scaleEffect(upButtonPressed ? 0.95 : 1.0)
                                .contentShape(Rectangle())
                                .offset(y: 2)
                        }
                        .buttonStyle(.plain)
                        .onLongPressGesture(minimumDuration: 0, maximumDistance: .infinity, pressing: { pressing in
                            withAnimation(.easeInOut(duration: 0.1)) {
                                upButtonPressed = pressing
                            }
                        }, perform: {})

                        Button(action: {
                            scrollToNextPage { id in
                                proxy.scrollTo(id, anchor: .top)
                            }
                        }) {
                            Image(systemName: "arrowtriangle.down.fill")
                                .font(KeyboardModels.Fonts.globalFont(size: 20))
                                .foregroundColor(CandidateViewModels.Colors.primaryTextColor)
                                .frame(width: 45, height: 45, alignment: .center)
                                .background(downButtonPressed ? Color.gray.opacity(0.3) : Color.clear)
                                .scaleEffect(downButtonPressed ? 0.95 : 1.0)
                                .contentShape(Rectangle())
                                .offset(y: 16)
                        }
                        .buttonStyle(.plain)
                        .onLongPressGesture(minimumDuration: 0, maximumDistance: .infinity, pressing: { pressing in
                            withAnimation(.easeInOut(duration: 0.1)) {
                                downButtonPressed = pressing
                            }
                        }, perform: {})

                        // Hide translate button for TPS layout (always hanzi-only)
                        if SharedSettings.shared.keyboardLayoutType != .tps {
                            Button(action: {
                                onTranslateToggle()
                            }) {
                                Image(systemName: "translate")
                                    .font(KeyboardModels.Fonts.globalFont(size: 20))
                                    .foregroundColor(CandidateViewModels.Colors.primaryTextColor)
                                    .frame(width: 45, height: 45, alignment: .center)
                                    .background(translateButtonPressed ? Color.gray.opacity(0.3) : Color.clear)
                                    .scaleEffect(translateButtonPressed ? 0.95 : 1.0)
                                    .contentShape(Rectangle())
                                    .offset(y: 25)
                            }
                            .buttonStyle(.plain)
                            .onLongPressGesture(minimumDuration: 0, maximumDistance: .infinity, pressing: { pressing in
                                withAnimation(.easeInOut(duration: 0.1)) {
                                    translateButtonPressed = pressing
                                }
                            }, perform: {})
                        }
                    }
                    .padding(.top, 3)
                }
                .padding(.top, 6)
                .padding(.trailing, 8)
                .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topTrailing)
            }
        }
        .onAppear {
            currentPage = 0
        }
        .onChange(of: isExpanded) { _, expanded in
            if expanded {
                currentPage = 0
            }
        }
        .background(
            Group {
                if isLiquidGlassEnabled {
                    // iOS 26 Liquid Glass：使用 KeyboardKit 預設背景色（支援 dark mode）
                    Color.keyboardBackground
                } else {
                    // Use custom candidate background color if set, otherwise default
                    (style.backgroundColor ?? Color.keyboardBackground)
                        .shadow(color: .black.opacity(0.15), radius: 6, x: 0, y: -2)
                }
            }
        )
        .overlay(
            FixedColumnDivider()
                .padding(.top, 6)
                .padding(.trailing, 8)
                .padding(.bottom, 8),
            alignment: .topTrailing,
        )
    }

    /// 滾動到上一頁
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

    /// 滾動到下一頁
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

/// 固定欄位分隔線
///
/// 展開視圖右側的控制按鈕區域分隔線。
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


/// 展開視圖網格單元格
///
/// 顯示一般長度的候選詞。
struct ExpandedCandidateGridCell: View {
    let suggestion: Autocomplete.Suggestion
    let isTranslateSwapped: Bool
    let isSelected: Bool
    let onTap: (Autocomplete.Suggestion) -> Void

    @State private var isPressed: Bool = false
    @Environment(\.colorScheme) private var colorScheme
    @Environment(\.candidateViewStyle) private var style

    private var displayTitle: String {
        CandidateCellHelper.displayTitle(for: suggestion, isTranslateSwapped: isTranslateSwapped)
    }

    private var displaySubtitle: String? {
        CandidateCellHelper.displaySubtitle(for: suggestion, isTranslateSwapped: isTranslateSwapped)
    }

    private var backgroundColor: Color {
        style.itemStyle.resolvedBackgroundColor(
            for: colorScheme,
            isSelected: isSelected,
            isPressed: isPressed,
            isLiquidGlassEnabled: CandidateCellHelper.isLiquidGlassEnabled(cornerRadius: style.itemStyle.cornerRadius)
        )
    }

    var body: some View {
        Button(action: {
            onTap(CandidateCellHelper.suggestionToHandle(for: suggestion, isTranslateSwapped: isTranslateSwapped))
        }) {
            VStack(alignment: .center, spacing: 2) {
                Text(displayTitle)
                    .font(KeyboardModels.Fonts.globalFont(
                        size: CandidateCellHelper.titleFontSize(isTranslateSwapped: isTranslateSwapped)
                    ))
                    .fontWeight(.regular)
                    .foregroundColor(CandidateViewModels.Colors.primaryTextColor)
                    .lineLimit(1)
                    .truncationMode(.tail)

                if let subtitle = displaySubtitle, !subtitle.isEmpty, subtitle != displayTitle {
                    Text(subtitle)
                        .font(KeyboardModels.Fonts.globalFont(
                            size: CandidateCellHelper.subtitleFontSize(isTranslateSwapped: isTranslateSwapped)
                        ))
                        .foregroundColor(CandidateViewModels.Colors.secondaryTextColor)
                        .lineLimit(1)
                        .truncationMode(.tail)
                } else {
                    Text(" ")
                        .font(KeyboardModels.Fonts.globalFont(size: CandidateViewModels.UI.secondaryFontSize))
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

