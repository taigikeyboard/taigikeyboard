// The candidate bar's contents: one horizontal row of choices.

import SwiftUI

/// One row of candidates, each labelled with the chord that selects it.
///
/// Horizontal and single-row on purpose: Taigi candidates are short, the bar
/// hangs off the caret inside someone else's document, and a tall list would
/// cover the text the user is reading while they choose.
struct CandidateBarView: View {
    let content: CandidateBarContent

    var body: some View {
        HStack(spacing: Metrics.cellSpacing) {
            ForEach(Array(content.labels.enumerated()), id: \.offset) { slot, label in
                cell(slot: slot, label: label)
            }
        }
        .padding(Metrics.barPadding)
        .background(.regularMaterial, in: RoundedRectangle(cornerRadius: Metrics.cornerRadius))
        // The panel is sized from this view's fitting size, which without this
        // would be the width the layout system is willing to compress it to.
        .fixedSize()
    }

    private func cell(slot: Int, label: String) -> some View {
        let isHighlighted = slot == content.highlightedSlot
        return HStack(spacing: Metrics.chordSpacing) {
            // `⌃1`…`⌃9` label the CURRENT page, so the chord under a candidate
            // is the one that selects it no matter how far down the list it is.
            Text("⌃\(slot + 1)")
                .font(.system(size: Metrics.chordFontSize, design: .monospaced))
                .foregroundStyle(.secondary)
            Text(label)
                .font(.system(size: Metrics.candidateFontSize))
        }
        .padding(.horizontal, Metrics.cellHorizontalPadding)
        .padding(.vertical, Metrics.cellVerticalPadding)
        .background(
            RoundedRectangle(cornerRadius: Metrics.cellCornerRadius)
                .fill(isHighlighted ? AnyShapeStyle(.selection) : AnyShapeStyle(.clear)),
        )
    }

    private enum Metrics {
        static let cellSpacing: CGFloat = 2
        static let chordSpacing: CGFloat = 4
        static let barPadding: CGFloat = 6
        static let cornerRadius: CGFloat = 8
        static let cellCornerRadius: CGFloat = 5
        static let cellHorizontalPadding: CGFloat = 6
        static let cellVerticalPadding: CGFloat = 3
        static let chordFontSize: CGFloat = 10
        static let candidateFontSize: CGFloat = 16
    }
}
