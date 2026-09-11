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
    /// The typeface both scripts render in — one of the bundled roster, or one
    /// the user added (`CandidateFontSelection`). Not a point value like the
    /// rest of this type, but it belongs here for the two reasons the sizes do:
    /// every measurement below is taken in it, and a cell bakes it into the
    /// labels it builds — so a change to it has to reach the panel cache's
    /// equality check (`CandidatePanel.panel(for:)`) and rebuild the panels.
    ///
    /// The whole SELECTION rather than the shared roster's case: two custom
    /// fonts are two typefaces, and a value that could not tell them apart
    /// would draw the second in the first's widths and keep its panels.
    let fontSelection: CandidateFontSelection
    let candidateFontSize: CGFloat
    let annotationFontSize: CGFloat
    let candidateAnnotationGap: CGFloat
    /// The digit hint's own size. Text-anchored like the annotation, and half
    /// again upstream's 8pt at the reference size: the digits are what a bare
    /// `1`…`9` keypress now aims at (`ComposingKeyIntent`), so they have to be
    /// legible at a glance rather than merely present.
    let indexFontSize: CGFloat
    /// Air between the digit's slot and the candidate beside it.
    let indexCandidateGap: CGFloat
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
    /// A whole LIST with no annotation is a different case: it has nothing to
    /// stack, and the panel renders it at the one-line height instead
    /// (`forContent(hasAnnotations:)`).
    ///
    /// Resolved at construction rather than per read: the fonts it measures
    /// are fixed here, and the layouts read this once per cell they place.
    let itemHeight: CGFloat
    /// How far Tahoe pulls a hairline in from the window's rounded edge, so
    /// the separator does not touch the curve. A chrome-scaled visual constant,
    /// not a function of the radius it clears — the same value also shortens
    /// the chevron's and the page arrow's separators, split across both ends
    /// (`CandidateChevronView`, `CandidatePageArrowView`), which no radius
    /// would give the right answer for.
    let tahoeSeparatorInset: CGFloat

    /// The face the candidate column is set in. The cells' labels and the
    /// width arithmetic below both go through this, so a measurement is always
    /// taken in the font the text is actually drawn in.
    var candidateFont: NSFont {
        fontSelection.font(ofSize: candidateFontSize)
    }

    /// The face the annotation column is set in.
    var annotationFont: NSFont {
        fontSelection.font(ofSize: annotationFontSize)
    }

    /// The face the digit hint is set in — the system font, never the user's
    /// candidate typeface: the digit names a key on their keyboard rather than
    /// belonging to the Taigi text, and a CJK face can set ASCII digits at a
    /// width the fixed slot was not measured for.
    var indexFont: NSFont {
        .systemFont(ofSize: indexFontSize)
    }

    /// The radius Tahoe rounds the WINDOW to — a capsule for a window of
    /// inline cells, a fixed rounded rectangle for one of stacked cells.
    ///
    /// Upstream takes `itemHeight / 2` unconditionally
    /// (`MacishBasePanel.swift`), which reads as macOS 26's capsule because
    /// every upstream cell is one line ~30pt tall — the size range the system
    /// itself capsules (large controls). A stacked cell is two lines and runs
    /// 45-65pt across the size ladder, where the same formula draws a stadium
    /// that dwarfs the text inside it (USER 2026-08-25, real device).
    ///
    /// Fixed rather than scaled by either knob: the chrome knob is how much
    /// air the window keeps, and the text knob how big the glyphs are —
    /// neither is a licence to change the container's shape language, and a
    /// radius that moved with them would round the window differently at every
    /// setting.
    var tahoeContainerCornerRadius: CGFloat {
        switch cellArrangement {
        case .inline: itemHeight / 2
        case .stacked: Self.stackedContainerCornerRadius
        }
    }

    /// How far Tahoe insets the selection from the cell's edge, so the
    /// highlight nests inside the window's own curve rather than touching it.
    /// A stacked cell takes the wider inset: its container radius no longer
    /// grows with the cell, so the nesting has to be visible at 16pt.
    var tahoeHighlightInset: CGFloat {
        switch cellArrangement {
        case .inline: Self.inlineHighlightInset
        case .stacked: Self.stackedHighlightInset
        }
    }

    /// The radius the selection is drawn at: concentric with the window, which
    /// macOS 26 asks for of nested shapes — the inner radius is the outer one
    /// less the padding between them. Stated unclamped; the box it is drawn in
    /// clamps it through `cornerRadius(_:fitting:)`.
    var tahoeHighlightCornerRadius: CGFloat {
        tahoeContainerCornerRadius - tahoeHighlightInset
    }

    /// `radius`, held to what a box of `size` can round: a radius past half
    /// the shorter side would draw a shape wider than the box it rounds. The
    /// one clamp both the window and the selection go through, so a fixed
    /// radius (`tahoeContainerCornerRadius`) has a single guard rather than one
    /// per drawing site.
    static func cornerRadius(_ radius: CGFloat, fitting size: CGSize) -> CGFloat {
        min(radius, min(size.width, size.height) / 2)
    }

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
    private static let baseIndexFontSize: CGFloat = 10
    private static let baseIndexCandidateGap: CGFloat = 2
    /// What the slot adds to the digit's own size — upstream's `indexFontSize
    /// + 2` (`MacishCandidateItemView.swift:22-24`), enough for a digit's
    /// bearing on either side.
    private static let indexSlotPadding: CGFloat = 2
    private static let baseHorizontalPadding: CGFloat = 9
    private static let baseVerticalPadding: CGFloat = 12
    private static let baseStackedLineGap: CGFloat = 2
    private static let baseTahoeSeparatorInset: CGFloat = 8
    /// What a window of two-line cells rounds to — see
    /// `tahoeContainerCornerRadius` for why it is a constant and not a scale.
    private static let stackedContainerCornerRadius: CGFloat = 16
    /// Upstream's inset, which a capsule needs no more of than a hairline's
    /// worth (`MacishCandidateItemView.swift`).
    private static let inlineHighlightInset: CGFloat = 2
    private static let stackedHighlightInset: CGFloat = 4

    init(
        textSize: CandidateTextSizeChoice,
        windowSize: CandidateWindowSizeChoice,
        fontSelection: CandidateFontSelection = .default,
        cellArrangement: CandidateCellArrangement = .inline,
    ) {
        let textScale = textSize.candidateFontSize / Self.baseCandidateFontSize
        self.textSize = textSize
        self.windowSize = windowSize
        self.fontSelection = fontSelection
        candidateFontSize = textSize.candidateFontSize
        annotationFontSize = (Self.baseAnnotationFontSize * textScale).rounded()
        candidateAnnotationGap = (Self.baseCandidateAnnotationGap * textScale).rounded()
        indexFontSize = (Self.baseIndexFontSize * textScale).rounded()
        indexCandidateGap = (Self.baseIndexCandidateGap * textScale).rounded()
        stackedLineGap = (Self.baseStackedLineGap * textScale).rounded()
        horizontalPadding = (Self.baseHorizontalPadding * windowSize.chromeScale).rounded()
        verticalPadding = (Self.baseVerticalPadding * windowSize.chromeScale).rounded()
        tahoeSeparatorInset = (Self.baseTahoeSeparatorInset * windowSize.chromeScale).rounded()
        self.cellArrangement = cellArrangement
        switch cellArrangement {
        case .inline:
            // The point size is what the four bundled faces need: their line
            // boxes fit the row it gives. A face the user brought, or one the
            // OS supplies, has no such guarantee — tall ascenders, stacked
            // diacritics and a fallback glyph all draw outside it — so those
            // are given their own measured line box instead. Bundled
            // selections keep the arithmetic they shipped with, exactly.
            if fontSelection.requiresLineBoxMeasurement {
                let lineBox = max(candidateFontSize, Self.lineHeight(of: fontSelection.font(ofSize: candidateFontSize)))
                itemHeight = (lineBox + verticalPadding).rounded(.up)
            } else {
                itemHeight = candidateFontSize + verticalPadding
            }
        case .stacked:
            itemHeight = (Self.lineHeight(of: fontSelection.font(ofSize: candidateFontSize))
                + Self.lineHeight(of: fontSelection.font(ofSize: annotationFontSize))
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
            fontSelection: fontSelection,
            cellArrangement: arrangement,
        )
    }

    /// The same sizes, resolved for the content a panel is about to show: a
    /// stacked list in which NO cell carries an annotation — 羅馬字, or
    /// 漢羅濫's one-script cells — has nothing to put on a second line, and
    /// renders as inline cells (USER 2026-09-02: no second-line air). One
    /// annotated cell keeps the stacked height for the whole list. Re-resolved
    /// per show and per in-place update — `CandidateBasePanel.metrics` has when.
    func forContent(hasAnnotations: Bool) -> CandidateMetrics {
        guard cellArrangement == .stacked, !hasAnnotations else { return self }
        return arranged(.inline)
    }

    /// The line box `font` draws in — AppKit's own answer rather than
    /// font-metric arithmetic, because what a stacked cell has to hold is
    /// exactly what the text system lays a `NSTextField` line out in.
    ///
    /// Answered for the chosen face itself: a stacked cell's height is fixed at
    /// construction, so it has to be the height of the font the labels are
    /// actually set in, not of the system font.
    /// Not memoized, unlike `primaryColumnFloor`: this runs in the
    /// initializer, which is not main-actor isolated, so a shared cache would
    /// need a lock of its own — and the measurement is two per stacked metrics
    /// value against a lock on every candidate window.
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
        2 * horizontalPadding + indexColumnWidth + primaryColumnFloor
    }

    /// The slot the key is centred in: wide enough for every form it can take,
    /// so the column keeps one width as the live key changes under the user
    /// (`CandidateIndexLabel.widestLabelForms`). Measured at the index font,
    /// which is the system's — the slot has to hold `⌥9`, not just `9`.
    ///
    /// Cached per size for the reason `primaryColumnFloor` is: every cell of
    /// every keystroke's list reads it, and the answer cannot change.
    var indexWidth: CGFloat {
        if let cached = Self.indexWidths[indexFontSize] {
            return cached
        }
        let font = indexFont
        let widest = CandidateIndexLabel.widestLabelForms
            .map { ceil(($0 as NSString).size(withAttributes: [.font: font]).width) }
            .max() ?? indexFontSize
        let width = widest + Self.indexSlotPadding
        Self.indexWidths[indexFontSize] = width
        return width
    }

    private static var indexWidths: [CGFloat: CGFloat] = [:]

    /// What the key column costs a cell: its slot plus the gap after it.
    /// Charged to every cell, including the ones whose position carries no
    /// key, because the slot is what keeps the candidates on one x.
    var indexColumnWidth: CGFloat {
        indexWidth + indexCandidateGap
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
        let key = PrimaryColumnFloorKey(fontSelection: fontSelection, fontSize: candidateFontSize)
        if let cached = Self.primaryColumnFloors[key] {
            return cached
        }
        let floor = max(candidateFontSize, measurePrimaryWidth("永"))
        Self.primaryColumnFloors[key] = floor
        return floor
    }

    /// What a cached floor was measured for.
    private struct PrimaryColumnFloorKey: Hashable {
        let fontSelection: CandidateFontSelection
        let fontSize: CGFloat
    }

    private static var primaryColumnFloors: [PrimaryColumnFloorKey: CGFloat] = [:]

    /// Drops every cached floor because the registration list moved
    /// (`FontRegistryObserver`). Re-measuring is one glyph per face and size.
    static func forgetPrimaryColumnFloors() {
        primaryColumnFloors.removeAll()
    }

    /// The width a cell wants for `cell`. Font-based rather than going through
    /// a template view: with fixed chrome widths, the sum IS the fitting size,
    /// and a shared template view would drag its Auto Layout state into every
    /// measurement.
    func measureWidth(_ cell: CandidateCellContent) -> CGFloat {
        cellWidth(for: cell, primaryWidth: measurePrimaryWidth(cell.text))
    }

    /// `measureWidth` for a cell whose primary text is already measured: the
    /// vertical layout measures every primary once for its column and hands
    /// the widths back in, so a two-hundred-row list is not measured twice.
    func cellWidth(for cell: CandidateCellContent, primaryWidth: CGFloat) -> CGFloat {
        let text = max(primaryColumnFloor, primaryWidth)
        switch cellArrangement {
        case .inline:
            return horizontalPadding + indexColumnWidth + text
                + annotationWidth(cell.annotation) + horizontalPadding
        case .stacked:
            // The two scripts are on top of each other, so the cell is as wide
            // as the WIDER of them — not as wide as both plus a gap.
            return horizontalPadding + indexColumnWidth
                + max(text, annotationTextWidth(cell.annotation)) + horizontalPadding
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
        max(candidateFontSize, cellWidth - horizontalPadding - indexColumnWidth - trailingInset)
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
