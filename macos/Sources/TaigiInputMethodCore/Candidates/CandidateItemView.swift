// One candidate cell: the candidate, and its other script beside it.

import AppKit

/// One cell of the candidate window, ported from MacishType's
/// `MacishCandidateItemView` (`references/MacishType/macos/MacishType/
/// MacishCandidateWindow/MacishCandidateItemView.swift`; MIT, © 2026 Luke
/// Chang) with one deliberate departure: no index column. Upstream numbers
/// every cell with the key that picks it; the `⌃1`…`⌃9` chords still work here
/// — bare digits could not be used, being the numeric tone markers of TL and
/// POJ — but they are not drawn, because a modifier badge beside every
/// candidate is noise the reader has to look past (USER 2026-08-21).
///
/// The annotation column is upstream's, and carries the candidate's other
/// script — see `CandidateCellContent`. The metrics the cell renders at are
/// fixed at construction (`CandidateMetrics`): the constraints below capture
/// them, so a size change rebuilds cells rather than mutating them.
final class CandidateItemView: NSView {
    let style: CandidateWindowStyle
    private let metrics: CandidateMetrics
    /// Tahoe insets the highlight into a pill; Sequoia paints the whole cell.
    private var contentInset: CGFloat { style == .tahoe ? 2 : 0 }
    /// Tahoe's pill: a separate view under the labels, so the cell's own layer
    /// can stay untouched. Nil on Sequoia.
    private var highlightView: NSView?

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

    /// Extra room the row leaves at its right edge — the vertical layout widens
    /// it so text stays clear of an overlay scroller. Starts at the metrics'
    /// horizontal padding, which is also what the constraint is built from.
    var trailingInset: CGFloat {
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

    init(style: CandidateWindowStyle, metrics: CandidateMetrics) {
        self.style = style
        self.metrics = metrics
        trailingInset = metrics.horizontalPadding
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

        candidateLabel.font = .systemFont(ofSize: metrics.candidateFontSize)
        candidateLabel.lineBreakMode = .byTruncatingTail
        candidateLabel.translatesAutoresizingMaskIntoConstraints = false

        annotationLabel.font = .systemFont(ofSize: metrics.annotationFontSize)
        annotationLabel.lineBreakMode = .byTruncatingTail
        annotationLabel.translatesAutoresizingMaskIntoConstraints = false

        // A cell narrower than both scripts want truncates the annotation
        // first: the candidate is what the user is choosing between, and the
        // annotation is there to disambiguate it. Only the ANNOTATION is
        // lowered — the candidate keeps the standard resistance rather than
        // `.required`, so a candidate too long for a clamped cell truncates
        // instead of breaking the cell's own geometry.
        annotationLabel.setContentCompressionResistancePriority(.defaultLow, for: .horizontal)

        addSubview(candidateLabel)
        addSubview(annotationLabel)

        trailingConstraint = annotationLabel.trailingAnchor.constraint(
            lessThanOrEqualTo: trailingAnchor, constant: -trailingInset,
        )
        annotationGapConstraint = annotationLabel.leadingAnchor.constraint(
            equalTo: candidateLabel.trailingAnchor, constant: 0,
        )
        annotationZeroWidthConstraint = annotationLabel.widthAnchor.constraint(equalToConstant: 0)
        annotationZeroWidthConstraint.priority = .defaultHigh
        annotationZeroWidthConstraint.isActive = true
        primaryColumnWidthConstraint = candidateLabel.widthAnchor.constraint(
            greaterThanOrEqualToConstant: metrics.candidateFontSize,
        )
        // Column alignment is a preference, not a promise: in a cell clamped
        // narrower than the column wants (the horizontal packer's row limit,
        // the vertical window's column cap) the cell's own geometry wins and
        // the text truncates, rather than Auto Layout breaking a constraint.
        primaryColumnWidthConstraint.priority = .defaultHigh

        NSLayoutConstraint.activate([
            candidateLabel.leadingAnchor.constraint(
                equalTo: leadingAnchor, constant: metrics.horizontalPadding,
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

    func configure(_ cell: CandidateCellContent) {
        candidateLabel.stringValue = cell.text
        annotationLabel.stringValue = cell.annotation ?? ""

        // Reconfigured rather than rebuilt: cells are recycled across pages and
        // across renumbering, so a cell that had an annotation and now has none
        // must give the width back — and the reverse must take it again.
        let hasAnnotation = cell.annotation != nil
        annotationGapConstraint.constant = hasAnnotation ? metrics.candidateAnnotationGap : 0
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
        if let pill = highlightView, isHighlighted {
            let inset = bounds.insetBy(dx: contentInset, dy: contentInset)
            pill.frame = inset
            pill.layer?.cornerRadius = inset.height / 2
        }
    }

    private func updateAppearance() {
        if isHighlighted {
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
