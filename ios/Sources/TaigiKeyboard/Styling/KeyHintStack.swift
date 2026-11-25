import SwiftUI
import UIKit

/// A key label with one hint centred above it, or two hints spread to the key's sides,
/// placed by `KeyLabelLayout` on glyph-ink baselines inside the key's own height.
///
/// Created by: `TaigiButtonContent` (`standardHintContent`, `tpsHintContent`)
struct KeyHintStack: View {
    /// One hint (centred) or two (leading / trailing); extra entries are ignored.
    let hints: [String]
    let label: String
    let labelFont: UIFont
    let hintFont: UIFont
    /// Row-constant reference glyphs (see `KeyLabelLayout.Input`).
    let labelReference: String
    let hintReference: String
    /// true: each hint sits on its own ink bottom (floating tone diacritics, whose ink
    /// starts at different heights). false: every hint shares the reference glyph's
    /// baseline, like ordinary text (punctuation, bopomofo, digits).
    var alignsEachHintByInk = false
    /// Side inset of a spread hint pair, as a fraction of the label font size.
    var hintSideInsetRatio: CGFloat = 0
    var hintColor: Color?
    let hintOpacity: Double

    /// A hint may shrink to this factor to fit its share of the key width.
    private static let hintMinimumScaleFactor: CGFloat = 0.5

    var body: some View {
        GeometryReader { geometry in
            let placement = KeyLabelLayout.place(KeyLabelLayout.Input(
                keyHeight: geometry.size.height,
                labelFontSize: labelFont.pointSize,
                labelInk: KeyLabelLayout.ink(of: labelReference, font: labelFont),
                hintBand: KeyLabelLayout.ink(of: hintReference, font: hintFont).height,
            ))
            let isPair = hints.count > 1
            let sideInset = isPair ? labelFont.pointSize * placement.labelScale * hintSideInsetRatio : 0
            let hint = HintStyle(
                font: hintFont.withSize(hintFont.pointSize * placement.hintScale),
                // A spread pair gets half the key each, so the two never overlap on a narrow key.
                maxWidth: isPair ? max(0, geometry.size.width / 2 - sideInset) : geometry.size.width,
                inkBottom: placement.hintInkBottom,
            )

            // One overlay per text on a key-sized base: an overlay never resizes its base,
            // so each baseline lands exactly where placed (a ZStack would grow to the union
            // of the line boxes and re-centre everything).
            Color.clear
                .frame(width: geometry.size.width, height: geometry.size.height)
                .overlay(alignment: .top) {
                    Text(label)
                        .font(Font(labelFont.withSize(labelFont.pointSize * placement.labelScale)))
                        .lineLimit(1)
                        .alignmentGuide(.top) { $0[.firstTextBaseline] - placement.labelBaseline }
                }
                .overlay(alignment: isPair ? .topLeading : .top) {
                    if let first = hints.first {
                        hintText(first, style: hint).padding(.leading, sideInset)
                    }
                }
                .overlay(alignment: .topTrailing) {
                    if isPair {
                        hintText(hints[1], style: hint).padding(.trailing, sideInset)
                    }
                }
        }
    }

    private struct HintStyle {
        let font: UIFont
        let maxWidth: CGFloat
        /// Where the hint ink bottom sits, from the key top.
        let inkBottom: CGFloat
    }

    private func hintText(_ hint: String, style: HintStyle) -> some View {
        // Diacritics sit on their own ink bottom; text hints share the reference glyph's baseline.
        let inkGlyph = alignsEachHintByInk ? hint : hintReference
        let baseline = style.inkBottom - KeyLabelLayout.ink(of: inkGlyph, font: style.font).descent
        return Text(hint)
            .font(Font(style.font))
            .foregroundColor(hintColor)
            .opacity(hintOpacity)
            .lineLimit(1)
            .minimumScaleFactor(Self.hintMinimumScaleFactor)
            .frame(maxWidth: style.maxWidth)
            .alignmentGuide(.top) { $0[.firstTextBaseline] - baseline }
    }
}
