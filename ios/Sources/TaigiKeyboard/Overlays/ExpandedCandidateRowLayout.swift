import CoreGraphics
import KeyboardKit

/// Arranges candidate suggestions into wrapping rows based on pre-measured widths.
///
/// UI-adjacent helper: depends on `AutocompleteSuggestion` (KeyboardKit type).
/// Not shared-core — inputs (available width, spacing, measurement) are injected.
enum ExpandedCandidateRowLayout {
    struct RowItem {
        let suggestion: AutocompleteSuggestion
        let originalIndex: Int
        let measuredWidth: CGFloat
    }

    static func arrangeRows(
        suggestions: [AutocompleteSuggestion],
        availableWidth: CGFloat,
        itemSpacing: CGFloat,
        measureCellWidth: (AutocompleteSuggestion) -> CGFloat,
    ) -> [[RowItem]] {
        var rows: [[RowItem]] = []
        var currentRow: [RowItem] = []
        var currentRowWidth: CGFloat = 0

        for (index, suggestion) in suggestions.enumerated() {
            let cellWidth = measureCellWidth(suggestion)
            let spacingNeeded = currentRow.isEmpty ? 0 : itemSpacing

            if !currentRow.isEmpty, (currentRowWidth + spacingNeeded + cellWidth) > availableWidth {
                rows.append(currentRow)
                currentRow = []
                currentRowWidth = 0
            }

            currentRow.append(RowItem(
                suggestion: suggestion,
                originalIndex: index,
                measuredWidth: cellWidth,
            ))
            currentRowWidth += (currentRow.count == 1 ? 0 : itemSpacing) + cellWidth
        }

        if !currentRow.isEmpty {
            rows.append(currentRow)
        }
        return rows
    }
}
