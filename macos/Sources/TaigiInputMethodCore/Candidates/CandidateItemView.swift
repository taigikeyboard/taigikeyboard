// One candidate cell: the ⌃n chord label and the candidate text.

import AppKit

/// One cell of the candidate window, ported from MacishType's
/// `MacishCandidateItemView` (`references/MacishType/macos/MacishType/
/// MacishCandidateWindow/MacishCandidateItemView.swift`; MIT, © 2026 Luke
/// Chang) with two deliberate departures:
///
/// - No annotation column. Taigi candidates are one already-composed string
///   (`CandidateDocumentText` folds the romanization in when the user asks for
///   both scripts), and the column-alignment machinery upstream carries exists
///   to align annotations.
/// - The index column is sized by measuring the widest chord label. Upstream
///   pins it to `indexFontSize + 2`, which fits one bare digit; this input
///   method's chords are two glyphs (`⌃1`…`⌃9`), because bare digits are the
///   numeric tone markers of TL and POJ.
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
        static let indexFontSize: CGFloat = 8
        static let leadingPadding: CGFloat = 4
        static let indexCandidateGap: CGFloat = 2
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

    /// The narrowest a cell renders: enough for one full-width glyph. Measured
    /// rather than assumed equal to the font size, because a full-width advance
    /// can round up past it on some macOS versions — and the packing budget
    /// (`HorizontalPageLayout`) must agree with `measureWidth` about minimums
    /// or a page drops a column.
    @MainActor
    static let baseWidth: CGFloat = {
        let font = NSFont.systemFont(ofSize: Metrics.candidateFontSize)
        let fullWidthGlyph = ceil(("永" as NSString).size(withAttributes: [.font: font]).width)
        return Metrics.leadingPadding + indexColumnWidth + Metrics.indexCandidateGap
            + max(Metrics.candidateFontSize, fullWidthGlyph) + Metrics.trailingPadding
    }()

    /// The width the cell wants for `label`. Static and font-based rather than
    /// going through a template view: with one label and fixed chrome widths,
    /// the sum IS the fitting size, and a shared template view would drag its
    /// Auto Layout state into every measurement.
    @MainActor
    static func measureWidth(_ label: String) -> CGFloat {
        let font = NSFont.systemFont(ofSize: Metrics.candidateFontSize)
        let text = ceil((label as NSString).size(withAttributes: [.font: font]).width)
        return Metrics.leadingPadding + indexColumnWidth + Metrics.indexCandidateGap
            + max(Metrics.candidateFontSize, text) + Metrics.trailingPadding
    }

    let style: CandidateWindowStyle
    /// Tahoe insets the highlight into a pill; Sequoia paints the whole cell.
    private var contentInset: CGFloat { style == .tahoe ? 2 : 0 }
    /// Tahoe's pill: a separate view under the labels, so the cell's own layer
    /// can stay untouched. Nil on Sequoia.
    private var highlightView: NSView?

    private let indexLabel = NSTextField(labelWithString: "")
    private let candidateLabel = NSTextField(labelWithString: "")

    /// Which candidate this cell shows, in the absolute order of
    /// `CandidateWindowContent.labels`. The identity clicks and highlights
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

    var highlightColor: NSColor = .selectedContentBackgroundColor {
        didSet {
            guard highlightColor != oldValue else { return }
            updateAppearance()
        }
    }

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

        addSubview(indexLabel)
        addSubview(candidateLabel)

        NSLayoutConstraint.activate([
            indexLabel.leadingAnchor.constraint(
                equalTo: leadingAnchor, constant: Metrics.leadingPadding,
            ),
            indexLabel.widthAnchor.constraint(equalToConstant: Self.indexColumnWidth),
            indexLabel.centerYAnchor.constraint(equalTo: centerYAnchor),
            candidateLabel.leadingAnchor.constraint(
                equalTo: indexLabel.trailingAnchor, constant: Metrics.indexCandidateGap,
            ),
            candidateLabel.trailingAnchor.constraint(
                lessThanOrEqualTo: trailingAnchor, constant: -Metrics.trailingPadding,
            ),
            candidateLabel.centerYAnchor.constraint(equalTo: centerYAnchor),
        ])
        updateAppearance()
    }

    @available(*, unavailable)
    required init?(coder _: NSCoder) { fatalError() }

    func configure(slotLabel: String, candidate: String) {
        indexLabel.stringValue = slotLabel
        candidateLabel.stringValue = candidate
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
        } else {
            indexLabel.textColor = .secondaryLabelColor
            candidateLabel.textColor = .labelColor
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
