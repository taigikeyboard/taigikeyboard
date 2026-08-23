// How big the candidate window renders: the two size choices, resolved to points.

import AppKit

/// The candidate text size a user can choose — resolved to the candidate
/// font's point size. `small` is the 16pt the window originally rendered at;
/// the ladder above it was raised a step (USER 2026-08-21: bigger text, less
/// whitespace — the window should spend its points on the glyphs).
///
/// `String` raw values so the choice persists through `UserDefaults` and
/// `@AppStorage`; an unknown stored value reads back as the default.
enum CandidateTextSizeChoice: String, CaseIterable, Sendable {
    case small
    case medium
    case large

    /// The candidate column's font size. The annotation font and the gaps
    /// scale off this — see `CandidateMetrics`. Three steps, not four: the
    /// 特大 tier was dropped (USER 2026-08-21); an install that stored it
    /// reads back as the default (`SettingsStore.choice`).
    var candidateFontSize: CGFloat {
        switch self {
        case .small: 16
        case .medium: 20
        case .large: 23
        }
    }
}

/// The candidate window's chrome size — resolved to a multiplier over the
/// cell paddings. Independent of the text choice: this knob is how much air
/// the window puts around the text, not how big the text is.
///
/// The whole ladder sits at or below upstream's paddings: MacishType's
/// original air is the LARGE end, and the default is tighter than it
/// (USER 2026-08-21: the window left too much whitespace around the text).
enum CandidateWindowSizeChoice: String, CaseIterable, Sendable {
    case small
    case medium
    case large

    var chromeScale: CGFloat {
        switch self {
        case .small: 0.7
        case .medium: 0.85
        case .large: 1.0
        }
    }
}

/// Every point value the candidate window's geometry is built from, resolved
/// once from the two size choices — plus the typeface they are measured in.
///
/// Upstream MacishType scales one mutable set of static metrics off a single
/// font size (`MacishCandidateItemView.updateFontSize`,
/// `references/MacishType/macos/MacishType/MacishCandidateWindow/
/// MacishCandidateItemView.swift:31-45`; MIT, © 2026 Luke Chang). This port
/// resolves an immutable value instead, threaded through the panels at
/// construction: cells capture metrics in Auto Layout constraints, so a size
/// change rebuilds the panels (`CandidatePanel.panel(for:)`) rather than
/// mutating metrics under views that would not follow.
///
/// Base values are upstream's at 16pt. The text choice scales the fonts and
/// the inter-column gap — text-anchored distances; the window choice scales
/// the paddings — the air around the text. Scaled values round to whole
/// points the way upstream rounds, so cell arithmetic stays crisp.
struct CandidateMetrics: Equatable, Sendable {
    /// What this value was resolved from, kept so a layout can ask for the
    /// same sizes under a different cell arrangement.
    let textSize: CandidateTextSizeChoice
    let windowSize: CandidateWindowSizeChoice
    /// The typeface both scripts render in. Not a point value like the rest of
    /// this type, but it belongs here for the two reasons the sizes do: every
    /// measurement below is taken in it, and a cell bakes it into the labels it
    /// builds — so a change to it has to reach the panel cache's equality check
    /// (`CandidatePanel.panel(for:)`) and rebuild the panels.
    let fontChoice: CandidateFontChoice
    let candidateFontSize: CGFloat
    let annotationFontSize: CGFloat
    let candidateAnnotationGap: CGFloat
    let horizontalPadding: CGFloat
    let verticalPadding: CGFloat
    /// Air between the two scripts of a stacked cell. Text-anchored like the
    /// inline gap it is the vertical counterpart of, and much tighter: two
    /// lines of one candidate read as one thing only while they sit close.
    let stackedLineGap: CGFloat
    /// Where this cell puts its second script. Carried in the metrics because
    /// the metrics are what a cell bakes into its constraints — the cell reads
    /// one value rather than two that could disagree.
    let cellArrangement: CandidateCellArrangement

    /// How tall one cell renders. An inline cell is one line of candidate; a
    /// stacked one is two line boxes plus the air between them — and keeps
    /// that height for a cell with no annotation, so a page's rows line up.
    ///
    /// Resolved at construction rather than per read: the fonts it measures
    /// are fixed here, and the layouts read this once per cell they place.
    let itemHeight: CGFloat
    /// How far Tahoe pulls a hairline in from the capsule's curve, so the
    /// separator does not touch the rounded edge. Chrome-scaled: the curve it
    /// clears is half the item height, which the chrome knob grows.
    let tahoeSeparatorInset: CGFloat

    /// The face the candidate column is set in. The cells' labels and the
    /// width arithmetic below both go through this, so a measurement is always
    /// taken in the font the text is actually drawn in.
    var candidateFont: NSFont { fontChoice.font(ofSize: candidateFontSize) }

    /// The face the annotation column is set in.
    var annotationFont: NSFont { fontChoice.font(ofSize: annotationFontSize) }

    /// `base`, stated at the reference 16pt, scaled to this metrics' text size
    /// and rounded to a whole point. What the chevron and page-arrow views
    /// size their symbols and reserved widths by, so the rounding policy lives
    /// here rather than in each of them — and the scale is the TEXT one, as
    /// upstream ties them (`MacishChevronView.swift:58-67`).
    func scaledSymbolMetric(_ base: CGFloat) -> CGFloat {
        (base * candidateFontSize / Self.baseCandidateFontSize).rounded()
    }

    /// Upstream's reference values at 16pt (`Base16Metrics`) — except the
    /// annotation font and its gap, both retuned from upstream's 12/11: the
    /// second script is a reading aid, not a footnote, so it renders larger
    /// and sits closer to the candidate it annotates (USER 2026-08-21,
    /// two rounds: subtitle bigger, then bigger still with less air).
    private static let baseCandidateFontSize: CGFloat = 16
    private static let baseAnnotationFontSize: CGFloat = 14
    private static let baseCandidateAnnotationGap: CGFloat = 7
    private static let baseHorizontalPadding: CGFloat = 9
    private static let baseVerticalPadding: CGFloat = 12
    private static let baseStackedLineGap: CGFloat = 2
    private static let baseTahoeSeparatorInset: CGFloat = 8

    init(
        textSize: CandidateTextSizeChoice,
        windowSize: CandidateWindowSizeChoice,
        fontChoice: CandidateFontChoice = .system,
        cellArrangement: CandidateCellArrangement = .inline,
    ) {
        let textScale = textSize.candidateFontSize / Self.baseCandidateFontSize
        self.textSize = textSize
        self.windowSize = windowSize
        self.fontChoice = fontChoice
        candidateFontSize = textSize.candidateFontSize
        annotationFontSize = (Self.baseAnnotationFontSize * textScale).rounded()
        candidateAnnotationGap = (Self.baseCandidateAnnotationGap * textScale).rounded()
        stackedLineGap = (Self.baseStackedLineGap * textScale).rounded()
        horizontalPadding = (Self.baseHorizontalPadding * windowSize.chromeScale).rounded()
        verticalPadding = (Self.baseVerticalPadding * windowSize.chromeScale).rounded()
        tahoeSeparatorInset = (Self.baseTahoeSeparatorInset * windowSize.chromeScale).rounded()
        self.cellArrangement = cellArrangement
        switch cellArrangement {
        case .inline:
            itemHeight = candidateFontSize + verticalPadding
        case .stacked:
            itemHeight = (Self.lineHeight(of: fontChoice.font(ofSize: candidateFontSize))
                + Self.lineHeight(of: fontChoice.font(ofSize: annotationFontSize))
                + stackedLineGap
                + verticalPadding).rounded(.up)
        }
    }

    /// The same sizes, resolved for a layout that arranges its cells
    /// differently.
    func arranged(_ arrangement: CandidateCellArrangement) -> CandidateMetrics {
        CandidateMetrics(
            textSize: textSize,
            windowSize: windowSize,
            fontChoice: fontChoice,
            cellArrangement: arrangement,
        )
    }

    /// The line box `font` draws in — AppKit's own answer rather than
    /// font-metric arithmetic, because what a stacked cell has to hold is
    /// exactly what the text system lays a `NSTextField` line out in.
    ///
    /// Answered for the chosen face itself: a stacked cell's height is fixed at
    /// construction, so it has to be the height of the font the labels are
    /// actually set in, not of the system font.
    private static func lineHeight(of font: NSFont) -> CGFloat {
        NSLayoutManager().defaultLineHeight(for: font)
    }
}

/// The width arithmetic the layouts run over cells, moved here from the cell
/// view with the metrics it depends on. `@MainActor` because the measuring
/// goes through AppKit fonts.
@MainActor
extension CandidateMetrics {
    /// The narrowest a cell renders: enough for one full-width glyph and no
    /// annotation. Measured rather than assumed equal to the font size, because
    /// a full-width advance can round up past it on some macOS versions — and
    /// the packing budget (`HorizontalPageLayout`) must agree with
    /// `measureWidth` about minimums or a page drops a column.
    var baseWidth: CGFloat {
        2 * horizontalPadding + primaryColumnFloor
    }

    /// The candidate column never renders narrower than one full-width glyph,
    /// which is what keeps single-character cells from collapsing.
    ///
    /// Cached per typeface and size, because `measureWidth` reads it once per
    /// candidate and a list runs to `maxDisplayCandidates` on every keystroke:
    /// measuring the same glyph two hundred times a keypress is work whose
    /// answer cannot change. The key is that pair rather than the whole metrics,
    /// since nothing else here moves the measurement — and it is the pair rather
    /// than the size because the measurement is taken in the FACE: the four
    /// bundled ones all render 「永」 at one em today, which is a fact about
    /// full-width CJK advances, not a licence to key on the size alone.
    var primaryColumnFloor: CGFloat {
        let key = PrimaryColumnFloorKey(fontChoice: fontChoice, fontSize: candidateFontSize)
        if let cached = Self.primaryColumnFloors[key] { return cached }
        let floor = max(candidateFontSize, measurePrimaryWidth("永"))
        Self.primaryColumnFloors[key] = floor
        return floor
    }

    /// What a cached floor was measured for.
    private struct PrimaryColumnFloorKey: Hashable {
        let fontChoice: CandidateFontChoice
        let fontSize: CGFloat
    }

    private static var primaryColumnFloors: [PrimaryColumnFloorKey: CGFloat] = [:]

    /// The width a cell wants for `cell`. Font-based rather than going through
    /// a template view: with fixed chrome widths, the sum IS the fitting size,
    /// and a shared template view would drag its Auto Layout state into every
    /// measurement.
    func measureWidth(_ cell: CandidateCellContent) -> CGFloat {
        let text = max(primaryColumnFloor, measurePrimaryWidth(cell.text))
        switch cellArrangement {
        case .inline:
            return horizontalPadding + text + annotationWidth(cell.annotation) + horizontalPadding
        case .stacked:
            // The two scripts are on top of each other, so the cell is as wide
            // as the WIDER of them — not as wide as both plus a gap.
            return horizontalPadding + max(text, annotationTextWidth(cell.annotation)) + horizontalPadding
        }
    }

    /// The candidate column's width for `text` alone — what the vertical layout
    /// aligns its rows on, so every annotation in the column starts at the same
    /// x. Measured at the candidate font, never the annotation's.
    func measurePrimaryWidth(_ text: String) -> CGFloat {
        ceil((text as NSString).size(withAttributes: [.font: candidateFont]).width)
    }

    /// The widest the candidate column can be in a cell `cellWidth` points
    /// across, leaving the leading padding and `trailingInset` their room. A layout that
    /// aligns a column across rows clamps to this: a column wider than the cell
    /// cannot be honoured, and asking for it anyway would push the text past
    /// the cell's edge.
    func maximumPrimaryColumnWidth(inCellWidth cellWidth: CGFloat, trailingInset: CGFloat) -> CGFloat {
        max(candidateFontSize, cellWidth - horizontalPadding - trailingInset)
    }

    /// The gap plus the annotation itself, or nothing at all when there is no
    /// annotation — an absent second script must cost the cell no width. An
    /// empty string counts as absent, the way `CandidateCellContent` reads it:
    /// charging the gap for it would reserve room beside nothing.
    func annotationWidth(_ annotation: String?) -> CGFloat {
        guard let annotation, !annotation.isEmpty else { return 0 }
        return candidateAnnotationGap + annotationTextWidth(annotation)
    }

    /// The annotation's own width, without the gap that separates it from an
    /// inline candidate — what a stacked cell measures, its second line having
    /// no candidate beside it to be separated from.
    func annotationTextWidth(_ annotation: String?) -> CGFloat {
        guard let annotation, !annotation.isEmpty else { return 0 }
        return ceil((annotation as NSString).size(withAttributes: [.font: annotationFont]).width)
    }
}
