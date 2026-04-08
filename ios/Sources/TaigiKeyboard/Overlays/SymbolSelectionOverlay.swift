import SwiftUI

/// Symbol selection overlay panel
///
/// Displays symbol grids across 7 category tabs in a scrollable tab bar,
/// allowing the user to insert symbols directly from the keyboard toolbar.
/// Follows the same overlay pattern as `LayoutSelectionOverlay`.
struct SymbolSelectionOverlay: View {
    let isExpanded: Bool
    let onSymbolInsert: (String) -> Void
    let onDismiss: () -> Void

    @State private var selectedTab: SymbolCategory = .fullWidth

    /// Grid columns based on selected tab's column count.
    private var columns: [GridItem] {
        Array(repeating: GridItem(.flexible(), spacing: 4), count: selectedTab.columnCount)
    }

    /// Flat symbol list with unique IDs for LazyVGrid rendering.
    private var flatSymbols: [SymbolItem] {
        SymbolData.rows(for: selectedTab).enumerated().flatMap { rowIndex, row in
            row.enumerated().map { colIndex, symbol in
                SymbolItem(id: "\(rowIndex)_\(colIndex)", symbol: symbol)
            }
        }
    }

    var body: some View {
        Group {
            if isExpanded {
                GeometryReader { geometry in
                    let toolbarHeight = CandidateViewModels.UI.height
                    contentView
                        .frame(maxWidth: .infinity)
                        .frame(height: geometry.size.height - toolbarHeight)
                }
            }
        }
    }

    // MARK: - Content

    private var contentView: some View {
        VStack(spacing: 0) {
            // Tab bar
            tabBar
                .padding(.top, 12)
                .padding(.bottom, 4)

            // Symbol grid
            ScrollView(.vertical, showsIndicators: false) {
                LazyVGrid(columns: columns, spacing: 4) {
                    ForEach(flatSymbols) { item in
                        symbolButton(item.symbol)
                    }
                }
                .padding(.horizontal, 8)
                .padding(.bottom, 8)
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .background(Color.keyboardBackground)
    }

    // MARK: - Tab Bar

    private var tabBar: some View {
        HStack(spacing: 4) {
            ForEach(SymbolCategory.allCases, id: \.self) { category in
                tabButton(category)
            }
        }
        .padding(.horizontal, 8)
    }

    private func tabButton(_ category: SymbolCategory) -> some View {
        let isSelected = selectedTab == category
        return Button(action: {
            selectedTab = category
        }) {
            Text(category.label)
                .font(KeyboardFonts.globalFont(size: 13))
                .fontWeight(isSelected ? .semibold : .regular)
                .foregroundColor(isSelected ? .white : CandidateViewModels.Colors.primaryTextColor)
                .frame(maxWidth: .infinity)
                .padding(.vertical, 12)
                .background(
                    RoundedRectangle(cornerRadius: 6)
                        .fill(isSelected ? Color.accentColor : Color.clear),
                )
        }
        .buttonStyle(.plain)
    }

    // MARK: - Symbol Button

    private func symbolButton(_ symbol: String) -> some View {
        Button(action: {
            onSymbolInsert(symbol)
        }) {
            Text(symbol)
                .font(.system(size: selectedTab.fontSize))
                .foregroundColor(CandidateViewModels.Colors.primaryTextColor)
                .frame(maxWidth: .infinity, minHeight: 40)
                .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
    }
}

/// A single symbol with a unique identity for use in ForEach/LazyVGrid.
private struct SymbolItem: Identifiable {
    let id: String
    let symbol: String
}
