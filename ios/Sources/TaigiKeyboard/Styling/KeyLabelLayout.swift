import CoreGraphics
import CoreText
import UIKit

/// Vertical layout of a key that shows a small hint above its label (tone diacritics on
/// the number row, MOE punctuation hints, TPS callout hints).
///
/// Every gap is a fraction of the label font size and is measured from glyph ink, not
/// from line boxes, so it holds for every keyboard font (line metrics differ widely
/// between System and the bundled CJK fonts). The stack is centred in the key; when the
/// key is too short (landscape) the hint shrinks first, then the whole stack scales.
///
/// Used by: `TaigiButtonContent` (`standardHintContent`, `tpsHintContent`).
enum KeyLabelLayout {
    /// Ink extent of a glyph run relative to its baseline (y grows downward on screen).
    struct Ink: Equatable {
        /// Distance from the baseline up to the ink top.
        var ascent: CGFloat
        /// Distance from the baseline down to the ink bottom (negative when the ink floats
        /// above the baseline, as for a diacritic).
        var descent: CGFloat

        var height: CGFloat {
            ascent + descent
        }
    }

    /// Inputs at the unscaled font sizes.
    struct Input {
        var keyHeight: CGFloat
        var labelFontSize: CGFloat
        /// Ink of a reference label glyph ("0" for digits / Latin, "ㄅ" for TPS), so every
        /// key of a row puts its label on the same baseline.
        var labelInk: Ink
        /// Ink height reserved for the hint (a reference hint glyph), likewise row-constant.
        var hintBand: CGFloat
    }

    struct Placement: Equatable {
        /// Factor for the label font size.
        var labelScale: CGFloat
        /// Factor for the hint font size (≤ `labelScale`).
        var hintScale: CGFloat
        /// Label baseline, from the key top.
        var labelBaseline: CGFloat
        /// Where the bottom of each hint's ink sits, from the key top.
        var hintInkBottom: CGFloat
        /// Top of the reserved hint band, from the key top.
        var hintBandTop: CGFloat
    }

    /// Gap between hint ink and label ink, as a fraction of the label font size.
    static let gapRatio: CGFloat = 0.12
    /// Minimum space between the ink and the key's top / bottom edge.
    static let edgeRatio: CGFloat = 0.1
    /// The hint gives way first: it shrinks down to this factor before the label shrinks.
    static let minimumHintScale: CGFloat = 0.6

    static func place(_ input: Input) -> Placement {
        let height = input.keyHeight
        let size = input.labelFontSize
        let gap = size * gapRatio
        let edges = 2 * size * edgeRatio
        let label = input.labelInk.height
        let band = input.hintBand

        var hintScale: CGFloat = 1
        var scale: CGFloat = 1
        if band + gap + label + edges > height {
            let roomForHint = height - edges - gap - label
            hintScale = max(minimumHintScale, band > 0 ? roomForHint / band : 1)
            let needed = band * hintScale + gap + label + edges
            if needed > height {
                scale = height / needed
            }
        }

        let stack = scale * (band * hintScale + gap + label)
        let top = (height - stack) / 2
        let labelInkTop = top + scale * (band * hintScale + gap)
        return Placement(
            labelScale: scale,
            hintScale: scale * hintScale,
            labelBaseline: labelInkTop + scale * input.labelInk.ascent,
            hintInkBottom: labelInkTop - scale * gap,
            hintBandTop: top,
        )
    }

    // MARK: - Glyph measurement

    /// Glyph-path bounds of `text` in `font` (CoreText applies the same font fallback
    /// SwiftUI's `Text` does). Whitespace-only text has no ink.
    static func ink(of text: String, font: UIFont) -> Ink {
        guard !text.trimmingCharacters(in: .whitespaces).isEmpty else {
            return Ink(ascent: 0, descent: 0)
        }
        let line = CTLineCreateWithAttributedString(NSAttributedString(string: text, attributes: [.font: font]))
        let bounds = CTLineGetBoundsWithOptions(line, .useGlyphPathBounds)
        return Ink(ascent: bounds.maxY, descent: -bounds.minY)
    }

    /// Baseline that puts the bottom of `ink` at `inkBottom`.
    static func baseline(for ink: Ink, inkBottom: CGFloat) -> CGFloat {
        inkBottom - ink.descent
    }
}
