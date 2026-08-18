// The expanded grid's geometry: column-spanned rows, pure arithmetic.

import CoreGraphics

/// The grid an expandable candidate window unfolds into, computed from
/// measured item widths — MacishType's `computeExpandedGrid` and its grid
/// navigation (`references/MacishType/macos/MacishType/MacishCandidateWindow/
/// MacishHorizontalExpandablePanel.swift:99-124`, `:700-745`; MIT, © 2026 Luke
/// Chang) extracted into a value type, for the same reason as
/// `HorizontalPageLayout`: which candidate a key lands on is geometry, and
/// geometry can be pinned without a window.
///
/// Unlike the collapsed row — whose cells take their measured widths — the
/// grid is column-quantized: every cell spans whole columns of a fixed width,
/// which is what lines the rows up under each other.
struct ExpandedGridLayout: Equatable {
    struct Cell: Equatable {
        let candidateIndex: Int
        let columnStart: Int
        let columnSpan: Int
    }

    let rows: [[Cell]]
    let columnCount: Int

    /// Lays `widths` into rows of `columnCount` columns. Each item spans the
    /// columns it needs (at least one, at most a full row), and moves to the
    /// next row when it does not fit — unless the row is empty, so an item
    /// needing the full width still gets a row.
    static func compute(widths: [CGFloat], columnWidth: CGFloat, columnCount: Int) -> ExpandedGridLayout {
        var rows: [[Cell]] = []
        var row: [Cell] = []
        var column = 0

        for (index, width) in widths.enumerated() {
            let span = max(1, min(columnCount, Int(ceil(width / columnWidth))))
            if column + span > columnCount, !row.isEmpty {
                rows.append(row)
                row = []
                column = 0
            }
            row.append(Cell(candidateIndex: index, columnStart: column, columnSpan: span))
            column += span
        }
        if !row.isEmpty {
            rows.append(row)
        }
        return ExpandedGridLayout(rows: rows, columnCount: columnCount)
    }

    func position(of candidateIndex: Int) -> (rowIndex: Int, cell: Cell)? {
        for (rowIndex, row) in rows.enumerated() {
            if let cell = row.first(where: { $0.candidateIndex == candidateIndex }) {
                return (rowIndex, cell)
            }
        }
        return nil
    }

    /// Where a vertical step of `rowStep` rows from `candidateIndex` lands:
    /// the cell of the target row whose columns overlap the current cell's, so
    /// the highlight moves straight down or up. Nil when the clamped target
    /// row is the one the selection is already on — the caller treats that as
    /// "no move", which at the top row is where a collapse begins.
    func verticalTarget(from candidateIndex: Int, rowStep: Int) -> Int? {
        guard let (rowIndex, cell) = position(of: candidateIndex) else { return nil }
        let targetRow = max(0, min(rowIndex + rowStep, rows.count - 1))
        guard targetRow != rowIndex else { return nil }
        return overlappingCandidate(
            inRow: targetRow,
            columnStart: cell.columnStart,
            columnEnd: cell.columnStart + cell.columnSpan,
            forward: rowStep > 0,
        ) ?? rows[targetRow].last?.candidateIndex
    }

    /// The first cell of `rowIndex` whose columns overlap
    /// `columnStart ..< columnEnd`, with the same backward tiebreak as the
    /// horizontal pages: stepping up prefers the neighbour the highlight
    /// visually sits over rather than the cell that merely begins under its
    /// left edge (`MacishHorizontalExpandablePanel.swift:713-728`).
    func overlappingCandidate(
        inRow rowIndex: Int,
        columnStart: Int,
        columnEnd: Int,
        forward: Bool,
    ) -> Int? {
        guard rows.indices.contains(rowIndex) else { return nil }
        let row = rows[rowIndex]
        for (position, cell) in row.enumerated() {
            let cellEnd = cell.columnStart + cell.columnSpan
            guard cell.columnStart < columnEnd, cellEnd > columnStart else { continue }
            if !forward, cell.columnStart < columnStart, position + 1 < row.count,
               row[position + 1].columnStart < columnEnd
            {
                return row[position + 1].candidateIndex
            }
            return cell.candidateIndex
        }
        return nil
    }
}
