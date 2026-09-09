// One candidate cell: the candidate, and its other script beside it.

import AppKit

/// One cell of the candidate window, ported from MacishType's
/// `MacishCandidateItemView` (`references/MacishType/macos/MacishType/
/// MacishCandidateWindow/MacishCandidateItemView.swift`; MIT, © 2026 Luke
/// Chang), index column included.
///
/// The column was dropped in the original port (USER 2026-08-21): the only key
/// that picked a candidate was the `⌃1`…`⌃9` chord, and a modifier badge beside
/// every candidate is noise the reader has to look past. The column came back
/// once bare `1`…`9` could select wherever the digit cannot be a tone marker
/// (`ComposingKeyIntent`, 2026-08-24), and since 2026-08-28 it draws the live
/// key set (`CandidateSlotKeySet` — bare `q w d f z x v y ;` under Standard,
/// `1`…`9` under Telex), so the label names a key the user can just press,
/// and the column earns its width.
///
/// The annotation column is upstream's, and carries the candidate's other
/// script — see `CandidateCellContent`. The metrics the cell renders at are
/// fixed at construction (`CandidateMetrics`): the constraints below capture
/// them, so a size change rebuilds cells rather than mutating them.
final class CandidateItemView: NSView {
    let style: CandidateWindowStyle
    private let metrics: CandidateMetrics
    /// Tahoe's highlight: a separate view under the labels, so the cell's own
    /// layer can stay untouched. Nil on Sequoia.
    private var highlightView: NSView?

    private let indexLabel = NSTextField(labelWithString: "")
    private let candidateLabel = NSTextField(labelWithString: "")
    private let annotationLabel = NSTextField(labelWithString: "")

    /// The digit currently drawn, or `""` for a position no digit names. The
    /// slot's width is charged to the cell either way, so blanking the text is
    /// the whole of "no digit here" — a cell that gave the width back would
    /// break the column its neighbours align on.
    var indexLabelText: String { indexLabel.stringValue }

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

    /// Whether this is the §34 literal cell — what the user is currently
    /// typing, not a candidate the engine offered. It names no key, so its
    /// text centres across the whole cell rather than stepping around the key
    /// column the other cells align on (USER 2026-09-09). It draws NO fill of
    /// its own: a tint was tried on 2026-09-09 and taken back out the same day
    /// (USER: 「背景底色強調效果不好,恢復第一個位置的背景底色」).
    var isLiteralCell = false {
        didSet {
            guard isLiteralCell != oldValue else { return }
            updateStackedTextArea()
        }
    }

    /// No equality guard on purpose: `NSColor` compares dynamic colours equal
    /// across light and dark even though they RESOLVE differently, and the
    /// highlight/backdrop snapshot a resolved `CGColor` — so every assignment
    /// repaints, which is what lets `syncTheme` refresh a cell after the
    /// panel's appearance changed under the same colour value.
    var highlightColor: NSColor = .selectedContentBackgroundColor {
        didSet {
            updateAppearance()
        }
    }

    /// Extra room the row leaves at its right edge — the vertical layout widens
    /// it so text stays clear of an overlay scroller. Starts at the metrics'
    /// horizontal padding, which is also what the constraint is built from.
    var trailingInset: CGFloat {
        didSet {
            guard trailingInset != oldValue else { return }
            trailingConstraint?.constant = -trailingInset
        }
    }

    /// The inline arrangement's constraints — nil in a stacked cell, whose two
    /// lines have no column to align, no gap to collapse and no trailing edge
    /// to inset.
    private var trailingConstraint: NSLayoutConstraint?
    private var annotationGapConstraint: NSLayoutConstraint?
    /// Collapses the annotation slot when there is nothing in it: an empty
    /// `NSTextField` still reserves a little baseline padding, which would
    /// otherwise push the trailing edge a few points right.
    private var annotationZeroWidthConstraint: NSLayoutConstraint?
    /// The candidate column's floor. Raised by `setPrimaryColumnWidth` so the
    /// vertical layout's rows align their annotations on one x.
    private var primaryColumnWidthConstraint: NSLayoutConstraint?

    /// The stacked arrangement's constraints — nil in an inline cell. Together
    /// they take the second line's height back when the cell has only one
    /// script, so the line it does carry sits in the MIDDLE of the cell rather
    /// than on the upper line of a pair (USER 2026-09-09, for the §34 literal
    /// cell under 漢羅對應). The cell's own height is untouched, so the row
    /// still lines up.
    private var stackedLineGapConstraint: NSLayoutConstraint?
    private var stackedAnnotationHeightConstraint: NSLayoutConstraint?

    /// Where the stacked cell's text area starts. An ordinary cell begins it
    /// after the key column, so the candidates of a row line up with each
    /// other; the literal cell names no key, so it centres over the WHOLE
    /// cell instead — which is the cell its own fill covers (USER 2026-09-09).
    private var stackedTextAfterKeyColumnConstraint: NSLayoutConstraint?
    private var stackedTextAcrossCellConstraint: NSLayoutConstraint?

    init(style: CandidateWindowStyle, metrics: CandidateMetrics) {
        self.style = style
        self.metrics = metrics
        trailingInset = metrics.horizontalPadding
        super.init(frame: .zero)
        translatesAutoresizingMaskIntoConstraints = true
        wantsLayer = true

        if style == .tahoe {
            let highlight = NSView()
            highlight.wantsLayer = true
            highlight.isHidden = true
            addSubview(highlight)
            highlightView = highlight
        }

        indexLabel.font = metrics.indexFont
        indexLabel.alignment = .center
        indexLabel.translatesAutoresizingMaskIntoConstraints = false

        candidateLabel.font = metrics.candidateFont
        candidateLabel.lineBreakMode = .byTruncatingTail
        candidateLabel.translatesAutoresizingMaskIntoConstraints = false

        annotationLabel.font = metrics.annotationFont
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

        NSLayoutConstraint.activate([
            indexLabel.leadingAnchor.constraint(equalTo: leadingAnchor, constant: metrics.horizontalPadding),
            indexLabel.centerYAnchor.constraint(equalTo: centerYAnchor),
            indexLabel.widthAnchor.constraint(equalToConstant: metrics.indexWidth),
        ])

        switch metrics.cellArrangement {
        case .inline: activateInlineConstraints()
        case .stacked: activateStackedConstraints()
        }
        updateAppearance()
    }

    @available(*, unavailable)
    required init?(coder _: NSCoder) { fatalError() }

    /// The two scripts side by side on one baseline: the candidate leads, the
    /// annotation follows it, and the cell is as wide as both together.
    private func activateInlineConstraints() {
        let trailing = annotationLabel.trailingAnchor.constraint(
            lessThanOrEqualTo: trailingAnchor, constant: -trailingInset,
        )
        let gap = annotationLabel.leadingAnchor.constraint(
            equalTo: candidateLabel.trailingAnchor, constant: 0,
        )
        let zeroWidth = annotationLabel.widthAnchor.constraint(equalToConstant: 0)
        zeroWidth.priority = .defaultHigh
        zeroWidth.isActive = true
        let columnWidth = candidateLabel.widthAnchor.constraint(
            greaterThanOrEqualToConstant: metrics.candidateFontSize,
        )
        // Column alignment is a preference, not a promise: in a cell clamped
        // narrower than the column wants (the horizontal packer's row limit,
        // the vertical window's column cap) the cell's own geometry wins and
        // the text truncates, rather than Auto Layout breaking a constraint.
        columnWidth.priority = .defaultHigh
        trailingConstraint = trailing
        annotationGapConstraint = gap
        annotationZeroWidthConstraint = zeroWidth
        primaryColumnWidthConstraint = columnWidth

        NSLayoutConstraint.activate([
            candidateLabel.leadingAnchor.constraint(
                equalTo: indexLabel.trailingAnchor, constant: metrics.indexCandidateGap,
            ),
            candidateLabel.centerYAnchor.constraint(equalTo: centerYAnchor),
            columnWidth,
            gap,
            annotationLabel.firstBaselineAnchor.constraint(equalTo: candidateLabel.firstBaselineAnchor),
            trailing,
        ])
    }

    /// The two scripts on top of each other, centred: the candidate above, the
    /// annotation under it, and the cell as wide as the wider of the two.
    ///
    /// The pair is centred through a layout guide spanning both labels rather
    /// than by pinning either of them to an edge: the cell's height comes from
    /// its frame (the panels place cells by hand), so a vertical constraint to
    /// an edge would fight a frame the labels do not get a say in.
    private func activateStackedConstraints() {
        // The area the two lines live in: what the cell leaves once the key
        // column has its slot. They centre in IT rather than in the cell, since
        // the key only ever takes width off the leading edge — centring in
        // the cell would push the pair right of the space it occupies. The
        // literal cell is the exception: it names no key, so its text centres
        // across the whole cell, which is the cell its fill covers.
        let textGuide = NSLayoutGuide()
        addLayoutGuide(textGuide)

        let padding = metrics.horizontalPadding
        let afterKeyColumn = textGuide.leadingAnchor.constraint(
            equalTo: indexLabel.trailingAnchor, constant: metrics.indexCandidateGap,
        )
        let acrossCell = textGuide.leadingAnchor.constraint(
            equalTo: leadingAnchor, constant: padding,
        )
        stackedTextAfterKeyColumnConstraint = afterKeyColumn
        stackedTextAcrossCellConstraint = acrossCell
        let lineGap = annotationLabel.topAnchor.constraint(
            equalTo: candidateLabel.bottomAnchor, constant: metrics.stackedLineGap,
        )
        // Inactive while there is a second line to draw; `configure` turns it
        // on for a one-script cell, which is what centres that cell's single
        // line in the pair's box.
        let annotationHeight = annotationLabel.heightAnchor.constraint(equalToConstant: 0)
        stackedLineGapConstraint = lineGap
        stackedAnnotationHeightConstraint = annotationHeight

        NSLayoutConstraint.activate([
            textGuide.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -padding),
            candidateLabel.centerXAnchor.constraint(equalTo: textGuide.centerXAnchor),
            candidateLabel.leadingAnchor.constraint(greaterThanOrEqualTo: textGuide.leadingAnchor),
            candidateLabel.trailingAnchor.constraint(lessThanOrEqualTo: textGuide.trailingAnchor),
            annotationLabel.centerXAnchor.constraint(equalTo: textGuide.centerXAnchor),
            annotationLabel.leadingAnchor.constraint(greaterThanOrEqualTo: textGuide.leadingAnchor),
            annotationLabel.trailingAnchor.constraint(lessThanOrEqualTo: textGuide.trailingAnchor),
            lineGap,
            textGuide.topAnchor.constraint(equalTo: candidateLabel.topAnchor),
            textGuide.bottomAnchor.constraint(equalTo: annotationLabel.bottomAnchor),
            textGuide.centerYAnchor.constraint(equalTo: centerYAnchor),
        ])
        updateStackedTextArea()
    }

    /// Points the stacked text area at the key column or at the whole cell,
    /// whichever this cell's kind calls for. A no-op on an inline cell, which
    /// has no such guide.
    private func updateStackedTextArea() {
        guard let stackedTextAfterKeyColumnConstraint, let stackedTextAcrossCellConstraint
        else { return }
        // Outgoing arm first: both are required equations on the same anchor,
        // and activating one while the other still stands is a conflict Auto
        // Layout would have to break.
        let (outgoing, incoming) = isLiteralCell
            ? (stackedTextAfterKeyColumnConstraint, stackedTextAcrossCellConstraint)
            : (stackedTextAcrossCellConstraint, stackedTextAfterKeyColumnConstraint)
        outgoing.isActive = false
        incoming.isActive = true
    }

    func configure(_ cell: CandidateCellContent) {
        candidateLabel.stringValue = cell.text
        annotationLabel.stringValue = cell.annotation ?? ""

        // Reconfigured rather than rebuilt: cells are recycled across pages and
        // across renumbering, so a cell that had an annotation and now has none
        // must give the width back — and the reverse must take it again.
        // A stacked cell keeps its FRAME whether or not there is a second
        // line, so its rows stay aligned — but it gives the empty line's
        // height back, so the one script it does carry centres in the cell
        // instead of sitting on the upper line (USER 2026-09-09: the §34
        // literal cell under 漢羅對應 carries no pair to align with). Only the
        // inline slot has width to give back.
        let hasAnnotation = cell.annotation != nil
        annotationGapConstraint?.constant = hasAnnotation ? metrics.candidateAnnotationGap : 0
        if let annotationZeroWidthConstraint, annotationZeroWidthConstraint.isActive == hasAnnotation {
            annotationZeroWidthConstraint.isActive = !hasAnnotation
        }
        stackedLineGapConstraint?.constant = hasAnnotation ? metrics.stackedLineGap : 0
        if let stackedAnnotationHeightConstraint,
           stackedAnnotationHeightConstraint.isActive == hasAnnotation {
            stackedAnnotationHeightConstraint.isActive = !hasAnnotation
        }
        updateAppearance()
    }

    /// Sets the digit that picks this cell, or `""` where none does. Every
    /// panel renumbers as its viewport moves — the vertical list as it
    /// scrolls, the expandable grid as the selection changes rows — so the
    /// no-op case is the common one, and writing `stringValue` dirties the
    /// text field's layout whether or not the string changed.
    func setIndexLabel(_ digit: String) {
        guard indexLabel.stringValue != digit else { return }
        indexLabel.stringValue = digit
    }

    /// Widens the candidate column to `width`, so rows sharing a column start
    /// their annotations at the same x — the vertical layout's alignment
    /// (`MacishVerticalPanel.swift:119-131`). Ignored when narrower than the
    /// one-glyph floor.
    func setPrimaryColumnWidth(_ width: CGFloat) {
        guard let primaryColumnWidthConstraint else { return }
        let target = max(metrics.candidateFontSize, width)
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
        if let highlightView, isHighlighted {
            layoutHighlight(highlightView)
        }
    }

    /// Places the Tahoe highlight and rounds it concentrically with the window
    /// (`CandidateMetrics.tahoeHighlightCornerRadius`). Sequoia never reaches
    /// here — it builds no highlight view and paints the whole cell instead.
    /// Held to the frame the cell was actually given rather than to the
    /// metrics' item height, which is what the layouts place cells at but not
    /// what a future one has to.
    private func layoutHighlight(_ highlight: NSView) {
        let inset = bounds.insetBy(dx: metrics.tahoeHighlightInset, dy: metrics.tahoeHighlightInset)
        highlight.frame = inset
        highlight.layer?.cornerRadius = CandidateMetrics.cornerRadius(
            metrics.tahoeHighlightCornerRadius, fitting: inset.size,
        )
    }

    private func updateAppearance() {
        indexLabel.textColor = isHighlighted ? .white : .secondaryLabelColor
        candidateLabel.textColor = isHighlighted ? .white : .labelColor
        annotationLabel.textColor = isHighlighted ? .white : .secondaryLabelColor

        guard isHighlighted else {
            if let highlightView {
                highlightView.isHidden = true
            } else {
                layer?.backgroundColor = nil
            }
            return
        }
        // Resolved under the panel's own appearance: `cgColor` snapshots a
        // dynamic colour against the CURRENT drawing appearance, which is not
        // this window's unless said so — a forced-dark panel would otherwise
        // pin its highlight at the light resolution.
        effectiveAppearance.performAsCurrentDrawingAppearance {
            if let highlightView {
                layer?.backgroundColor = nil
                layoutHighlight(highlightView)
                highlightView.layer?.backgroundColor = highlightColor.cgColor
                highlightView.isHidden = false
            } else {
                layer?.backgroundColor = highlightColor.cgColor
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
