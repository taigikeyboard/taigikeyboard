// One candidate cell: the ⌃n chord label, the candidate, and its other script.

import AppKit

/// One cell of the candidate window, ported from MacishType's
/// `MacishCandidateItemView` (`references/MacishType/macos/MacishType/
/// MacishCandidateWindow/MacishCandidateItemView.swift`; MIT, © 2026 Luke
/// Chang) with one deliberate departure: the index column is sized by measuring
/// the widest chord label, where upstream pins it to `indexFontSize + 2`, which
/// fits one bare digit. This input method's chords are two glyphs (`⌃1`…`⌃9`),
/// because bare digits are the numeric tone markers of TL and POJ.
///
/// The annotation column is upstream's, and carries the candidate's other
/// script — see `CandidateCellContent`.
final class CandidateItemView: NSView {
    /// The chord that selects the candidate in each page slot. Rendered in the
    /// cell rather than derived elsewhere so the label under a candidate is
    /// always the chord that actually picks it.
    static let slotLabels = (1 ... HorizontalPageLayout.pageSize).map { "⌃\($0)" }

    /// Fixed metrics at the one font size this window renders. Upstream scales
    /// them off a configurable font size; this input method has no font-size
    /// setting, so the scaling machinery would be dead weight.
    enum Metrics {
        static let candidateFontSize: CGFloat = 16
        static let annotationFontSize: CGFloat = 12
        static let indexFontSize: CGFloat = 8
        static let leadingPadding: CGFloat = 4
        static let indexCandidateGap: CGFloat = 2
        static let candidateAnnotationGap: CGFloat = 11
        static let trailingPadding: CGFloat = 9
        static let verticalPadding: CGFloat = 12

        static var itemHeight: CGFloat { candidateFontSize + verticalPadding }
    }

    /// The index column's width: the widest chord label, measured once. A
    /// hard-coded constant would clip quietly the day the label font changes.
    @MainActor
    static let indexColumnWidth: CGFloat = {
        let font = NSFont.systemFont(ofSize: Metrics.indexFontSize)
        return slotLabels
            .map { ceil(($0 as NSString).size(withAttributes: [.font: font]).width) }
            .max() ?? Metrics.indexFontSize + 2
    }()

    /// The narrowest a cell renders: enough for one full-width glyph and no
    /// annotation. Measured rather than assumed equal to the font size, because
    /// a full-width advance can round up past it on some macOS versions — and
    /// the packing budget (`HorizontalPageLayout`) must agree with
    /// `measureWidth` about minimums or a page drops a column.
    @MainActor
    static let baseWidth: CGFloat = {
        chromeWidth + primaryColumnFloor + Metrics.trailingPadding
    }()

    /// Everything left of the candidate itself.
    @MainActor
    private static var chromeWidth: CGFloat {
        Metrics.leadingPadding + indexColumnWidth + Metrics.indexCandidateGap
    }

    /// The candidate column never renders narrower than one full-width glyph,
    /// which is what keeps single-character cells from collapsing.
    @MainActor
    private static let primaryColumnFloor: CGFloat = {
        max(Metrics.candidateFontSize, measurePrimaryWidth("永"))
    }()

    /// The width the cell wants for `cell`. Static and font-based rather than
    /// going through a template view: with fixed chrome widths, the sum IS the
    /// fitting size, and a shared template view would drag its Auto Layout
    /// state into every measurement.
    @MainActor
    static func measureWidth(_ cell: CandidateCellContent) -> CGFloat {
        chromeWidth
            + max(primaryColumnFloor, measurePrimaryWidth(cell.text))
            + annotationWidth(cell.annotation)
            + Metrics.trailingPadding
    }

    /// The candidate column's width for `text` alone — what the vertical layout
    /// aligns its rows on, so every annotation in the column starts at the same
    /// x. Measured at the candidate font, never the annotation's.
    @MainActor
    static func measurePrimaryWidth(_ text: String) -> CGFloat {
        let font = NSFont.systemFont(ofSize: Metrics.candidateFontSize)
        return ceil((text as NSString).size(withAttributes: [.font: font]).width)
    }

    /// The widest the candidate column can be in a cell `cellWidth` points
    /// across, leaving the chrome and `trailingInset` their room. A layout that
    /// aligns a column across rows clamps to this: a column wider than the cell
    /// cannot be honoured, and asking for it anyway would push the text past
    /// the cell's edge.
    @MainActor
    static func maximumPrimaryColumnWidth(inCellWidth cellWidth: CGFloat, trailingInset: CGFloat) -> CGFloat {
        max(Metrics.candidateFontSize, cellWidth - chromeWidth - trailingInset)
    }

    /// The gap plus the annotation itself, or nothing at all when there is no
    /// annotation — an absent second script must cost the cell no width. An
    /// empty string counts as absent, the way `CandidateCellContent` reads it:
    /// charging the gap for it would reserve room beside nothing.
    @MainActor
    static func annotationWidth(_ annotation: String?) -> CGFloat {
        guard let annotation, !annotation.isEmpty else { return 0 }
        let font = NSFont.systemFont(ofSize: Metrics.annotationFontSize)
        let text = ceil((annotation as NSString).size(withAttributes: [.font: font]).width)
        return Metrics.candidateAnnotationGap + text
    }

    let style: CandidateWindowStyle
    /// Tahoe insets the highlight into a pill; Sequoia paints the whole cell.
    private var contentInset: CGFloat { style == .tahoe ? 2 : 0 }
    /// Tahoe's pill: a separate view under the labels, so the cell's own layer
    /// can stay untouched. Nil on Sequoia.
    private var highlightView: NSView?

    private let indexLabel = NSTextField(labelWithString: "")
    private let candidateLabel = NSTextField(labelWithString: "")
    private let annotationLabel = NSTextField(labelWithString: "")

    /// Which candidate this cell shows, in the absolute order of
    /// `CandidateWindowContent.cells`. The identity clicks and highlights
    /// speak in.
    var absoluteIndex: Int = 0

    /// Fires on mouse-up over the cell, unless the gesture was a window drag.
    /// A click SELECTS — committing stays on the keyboard until the mouse
    /// commit path earns its own owner/session validation.
    var onClick: (() -> Void)?

    var isHighlighted = false {
        didSet {
            guard isHighlighted != oldValue else { return }
            updateAppearance()
        }
    }

    /// No equality guard on purpose: `NSColor` compares dynamic colours equal
    /// across light and dark even though they RESOLVE differently, and the
    /// pill/backdrop snapshot a resolved `CGColor` — so every assignment
    /// repaints, which is what lets `syncTheme` refresh a cell after the
    /// panel's appearance changed under the same colour value.
    var highlightColor: NSColor = .selectedContentBackgroundColor {
        didSet {
            updateAppearance()
        }
    }

    /// Whether the chord label is visible. The vertical layout scrolls, so the
    /// `⌃n` chords address the nine rows around the viewport — rows outside
    /// that window keep the slot's width (labels stay column-aligned) but show
    /// nothing, because showing a chord that would not select them is a lie.
    var showsSlotLabel = true {
        didSet {
            guard showsSlotLabel != oldValue else { return }
            indexLabel.alphaValue = showsSlotLabel ? 1 : 0
        }
    }

    /// Extra room the row leaves at its right edge — the vertical layout widens
    /// it so text stays clear of an overlay scroller.
    var trailingInset: CGFloat = Metrics.trailingPadding {
        didSet {
            guard trailingInset != oldValue else { return }
            trailingConstraint.constant = -trailingInset
        }
    }

    private var trailingConstraint: NSLayoutConstraint!
    private var annotationGapConstraint: NSLayoutConstraint!
    /// Collapses the annotation slot when there is nothing in it: an empty
    /// `NSTextField` still reserves a little baseline padding, which would
    /// otherwise push the trailing edge a few points right.
    private var annotationZeroWidthConstraint: NSLayoutConstraint!
    /// The candidate column's floor. Raised by `setPrimaryColumnWidth` so the
    /// vertical layout's rows align their annotations on one x.
    private var primaryColumnWidthConstraint: NSLayoutConstraint!

    init(style: CandidateWindowStyle) {
        self.style = style
        super.init(frame: .zero)
        translatesAutoresizingMaskIntoConstraints = true
        wantsLayer = true

        if style == .tahoe {
            let pill = NSView()
            pill.wantsLayer = true
            pill.isHidden = true
            addSubview(pill)
            highlightView = pill
        }

        indexLabel.font = .systemFont(ofSize: Metrics.indexFontSize)
        indexLabel.alignment = .center
        indexLabel.translatesAutoresizingMaskIntoConstraints = false

        candidateLabel.font = .systemFont(ofSize: Metrics.candidateFontSize)
        candidateLabel.lineBreakMode = .byTruncatingTail
        candidateLabel.translatesAutoresizingMaskIntoConstraints = false

        annotationLabel.font = .systemFont(ofSize: Metrics.annotationFontSize)
        annotationLabel.lineBreakMode = .byTruncatingTail
        annotationLabel.translatesAutoresizingMaskIntoConstraints = false

        // A cell narrower than both scripts want truncates the annotation
        // first: the candidate is what the user is choosing between, and the
        // annotation is there to disambiguate it. Only the ANNOTATION is
        // lowered — the candidate keeps the standard resistance rather than
        // `.required`, so a candidate too long for a clamped cell truncates
        // instead of breaking the cell's own geometry.
        annotationLabel.setContentCompressionResistancePriority(.defaultLow, for: .horizontal)

        addSubview(indexLabel)
        addSubview(candidateLabel)
        addSubview(annotationLabel)

        trailingConstraint = annotationLabel.trailingAnchor.constraint(
            lessThanOrEqualTo: trailingAnchor, constant: -Metrics.trailingPadding,
        )
        annotationGapConstraint = annotationLabel.leadingAnchor.constraint(
            equalTo: candidateLabel.trailingAnchor, constant: 0,
        )
        annotationZeroWidthConstraint = annotationLabel.widthAnchor.constraint(equalToConstant: 0)
        annotationZeroWidthConstraint.priority = .defaultHigh
        annotationZeroWidthConstraint.isActive = true
        primaryColumnWidthConstraint = candidateLabel.widthAnchor.constraint(
            greaterThanOrEqualToConstant: Metrics.candidateFontSize,
        )
        // Column alignment is a preference, not a promise: in a cell clamped
        // narrower than the column wants (the horizontal packer's row limit,
        // the vertical window's column cap) the cell's own geometry wins and
        // the text truncates, rather than Auto Layout breaking a constraint.
        primaryColumnWidthConstraint.priority = .defaultHigh

        NSLayoutConstraint.activate([
            indexLabel.leadingAnchor.constraint(
                equalTo: leadingAnchor, constant: Metrics.leadingPadding,
            ),
            indexLabel.widthAnchor.constraint(equalToConstant: Self.indexColumnWidth),
            indexLabel.centerYAnchor.constraint(equalTo: centerYAnchor),
            candidateLabel.leadingAnchor.constraint(
                equalTo: indexLabel.trailingAnchor, constant: Metrics.indexCandidateGap,
            ),
            candidateLabel.centerYAnchor.constraint(equalTo: centerYAnchor),
            primaryColumnWidthConstraint,
            annotationGapConstraint,
            annotationLabel.firstBaselineAnchor.constraint(equalTo: candidateLabel.firstBaselineAnchor),
            trailingConstraint,
        ])
        updateAppearance()
    }

    @available(*, unavailable)
    required init?(coder _: NSCoder) { fatalError() }

    func configure(slotLabel: String, cell: CandidateCellContent) {
        indexLabel.stringValue = slotLabel
        candidateLabel.stringValue = cell.text
        annotationLabel.stringValue = cell.annotation ?? ""

        // Reconfigured rather than rebuilt: cells are recycled across pages and
        // across renumbering, so a cell that had an annotation and now has none
        // must give the width back — and the reverse must take it again.
        let hasAnnotation = cell.annotation != nil
        annotationGapConstraint.constant = hasAnnotation ? Metrics.candidateAnnotationGap : 0
        if annotationZeroWidthConstraint.isActive == hasAnnotation {
            annotationZeroWidthConstraint.isActive = !hasAnnotation
        }
        updateAppearance()
    }

    /// Widens the candidate column to `width`, so rows sharing a column start
    /// their annotations at the same x — the vertical layout's alignment
    /// (`MacishVerticalPanel.swift:119-131`). Ignored when narrower than the
    /// one-glyph floor.
    func setPrimaryColumnWidth(_ width: CGFloat) {
        let target = max(Metrics.candidateFontSize, width)
        guard primaryColumnWidthConstraint.constant != target else { return }
        primaryColumnWidthConstraint.constant = target
    }

    override func mouseUp(with event: NSEvent) {
        // A drag that happened to end over a cell is the window being moved,
        // not a choice being made.
        guard (window as? CandidateWindowDragging)?.didDrag != true else { return }
        _ = event
        onClick?()
    }

    override func layout() {
        super.layout()
        if let pill = highlightView, isHighlighted {
            let inset = bounds.insetBy(dx: contentInset, dy: contentInset)
            pill.frame = inset
            pill.layer?.cornerRadius = inset.height / 2
        }
    }

    private func updateAppearance() {
        if isHighlighted {
            indexLabel.textColor = .white
            candidateLabel.textColor = .white
            annotationLabel.textColor = .white
            // Resolved under the panel's own appearance: `cgColor` snapshots a
            // dynamic colour against the CURRENT drawing appearance, which is
            // not this window's unless said so — a forced-dark panel would
            // otherwise pin its highlight at the light resolution.
            effectiveAppearance.performAsCurrentDrawingAppearance {
                if let pill = highlightView {
                    layer?.backgroundColor = nil
                    let inset = bounds.insetBy(dx: contentInset, dy: contentInset)
                    pill.frame = inset
                    pill.layer?.cornerRadius = inset.height / 2
                    pill.layer?.backgroundColor = highlightColor.cgColor
                    pill.isHidden = false
                } else {
                    layer?.backgroundColor = highlightColor.cgColor
                }
            }
        } else {
            indexLabel.textColor = .secondaryLabelColor
            candidateLabel.textColor = .labelColor
            annotationLabel.textColor = .secondaryLabelColor
            if let pill = highlightView {
                pill.isHidden = true
            } else {
                layer?.backgroundColor = nil
            }
        }
    }
}

/// How a cell asks its window whether the mouse gesture that ended on it was
/// really a drag. A protocol so the cell does not name the panel class.
@MainActor
protocol CandidateWindowDragging: AnyObject {
    var didDrag: Bool { get }
}
