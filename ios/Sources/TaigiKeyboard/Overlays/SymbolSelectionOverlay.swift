// 從工具列叫出的「符號選擇」overlay — 五個分類 tab 的符號網格,
// 點選後即從插入點送出符號。

import SwiftUI

/// Symbol selection overlay panel
///
/// Displays symbol grids across 5 category tabs in a scrollable tab bar,
/// allowing the user to insert symbols directly from the keyboard toolbar.
/// Follows the same overlay pattern as `LayoutSelectionOverlay`.
// 符號選擇面板 — 與 LayoutSelectionOverlay 採用相同的 overlay 樣式。
struct SymbolSelectionOverlay: View {
    let isExpanded: Bool
    let onSymbolInsert: (String) -> Void
    let onDismiss: () -> Void

    @State private var selectedTab: SymbolCategory = .fullWidth
    @Environment(\.candidateTheme) private var theme
    @Environment(DisplayLanguageStore.self) private var lang

    /// Grid columns based on selected tab's column count.
    // 依當前分類的 columnCount 動態建立 GridItem。
    private var columns: [GridItem] {
        Array(repeating: GridItem(.flexible(), spacing: 4), count: selectedTab.columnCount)
    }

    /// Flat symbol list with unique IDs for LazyVGrid rendering.
    // 把 [[String]] 攤平成附 ID 的 SymbolItem,供 LazyVGrid 渲染。
    private var flatSymbols: [SymbolItem] {
        SymbolData.rows(for: selectedTab).enumerated().flatMap { rowIndex, row in
            row.enumerated().map { colIndex, symbol in
                SymbolItem(id: "\(rowIndex)_\(colIndex)", symbol: symbol)
            }
        }
    }

    var body: some View {
        contentView.keyboardOverlayPanel(isExpanded: isExpanded, theme: theme)
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
        .onAppear {
            // Re-read the App Group display-language tag when the overlay appears — the reliable
            // cross-process resync point (host app may have changed the language).
            lang.syncFromSettings()
        }
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
            Text(lang.string(category.labelKey))
                .font(KeyboardFonts.globalFont(size: 13))
                .fontWeight(isSelected ? .semibold : .regular)
                .foregroundColor(isSelected ? .white : theme.primaryTextColor)
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
                .foregroundColor(theme.primaryTextColor)
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
