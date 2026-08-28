// 中文: 展開候選 overlay 的 row 排版工具。
// 中文: 依預先量測好的寬度把 suggestions 拆成多行,寬度不夠就換行。

import CoreGraphics
import KeyboardKit

/// Arranges candidate suggestions into wrapping rows based on pre-measured widths.
///
/// UI-adjacent helper: depends on `AutocompleteSuggestion` (KeyboardKit type).
/// Not shared-core — inputs (available width, spacing, measurement) are injected.
// 中文: 展開候選的 wrapping row 排版命名空間 — 不歸 shared-core,所有量測由外部注入。
enum ExpandedCandidateRowLayout {
    // 中文: 一個候選詞在排版後的位置資訊 — 原始 index + 量測寬度。
    struct RowItem {
        let suggestion: AutocompleteSuggestion
        let originalIndex: Int
        let measuredWidth: CGFloat
    }

    // 中文: 把 suggestions 依寬度排成多行 — 累計寬度超過 availableWidth 就換行。
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

        if !currentRow.isEmpty { rows.append(currentRow) }
        return rows
    }
}
