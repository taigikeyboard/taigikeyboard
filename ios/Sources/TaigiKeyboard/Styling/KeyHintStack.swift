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
    /// Side inset of a spread hint pair, as a fraction of the label font size.
    var hintSideInsetRatio: CGFloat = 0
    var hintColor: Color?
    let hintOpacity: Double

    var body: some View {
        GeometryReader { geometry in
            let placement = KeyLabelLayout.place(KeyLabelLayout.Input(
                keyHeight: geometry.size.height,
                labelFontSize: labelFont.pointSize,
                labelInk: KeyLabelLayout.ink(of: labelReference, font: labelFont),
                hintBand: KeyLabelLayout.ink(of: hintReference, font: hintFont).height,
            ))
            let scaledHintFont = hintFont.withSize(hintFont.pointSize * placement.hintScale)
            let sideInset = labelFont.pointSize * placement.labelScale * hintSideInsetRatio

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
                .overlay(alignment: hints.count > 1 ? .topLeading : .top) {
                    if let hint = hints.first {
                        hintText(hint, font: scaledHintFont, inkBottom: placement.hintInkBottom)
                            .padding(.leading, hints.count > 1 ? sideInset : 0)
                    }
                }
                .overlay(alignment: .topTrailing) {
                    if hints.count > 1 {
                        hintText(hints[1], font: scaledHintFont, inkBottom: placement.hintInkBottom)
                            .padding(.trailing, sideInset)
                    }
                }
        }
    }

    /// A hint whose ink bottom sits at `inkBottom` from the key top.
    private func hintText(_ hint: String, font: UIFont, inkBottom: CGFloat) -> some View {
        let baseline = KeyLabelLayout.baseline(for: KeyLabelLayout.ink(of: hint, font: font), inkBottom: inkBottom)
        return Text(hint)
            .font(Font(font))
            .foregroundColor(hintColor)
            .opacity(hintOpacity)
            .lineLimit(1)
            .alignmentGuide(.top) { $0[.firstTextBaseline] - baseline }
    }
}
