// The paging edge of the horizontal window: separator plus up/down chevrons.

import AppKit

/// The page-turn control at the right edge of a paged horizontal window,
/// ported from MacishType's `MacishPageArrowView` (`references/MacishType/
/// macos/MacishType/MacishCandidateWindow/MacishPageArrowView.swift`; MIT,
/// © 2026 Luke Chang) at the fixed 16pt font metrics — the font-size scaling
/// is dropped with the setting that drove it.
final class CandidatePageArrowView: NSView {
    var onPageUp: (() -> Void)?
    var onPageDown: (() -> Void)?

    /// Enabled arrows read as actionable, disabled ones as furniture — the
    /// tint is the affordance, so it tracks the page position.
    var canPageUp = false {
        didSet { upImageView.contentTintColor = canPageUp ? .secondaryLabelColor : .tertiaryLabelColor }
    }

    var canPageDown = false {
        didSet { downImageView.contentTintColor = canPageDown ? .secondaryLabelColor : .tertiaryLabelColor }
    }

    private let separator = NSBox()
    private let upImageView: NSImageView
    private let downImageView: NSImageView

    private static let spacing: CGFloat = 4
    private static let imageWidth: CGFloat = 16
    private static let padding: CGFloat = 7

    init(style: CandidateWindowStyle) {
        let configuration = NSImage.SymbolConfiguration(pointSize: 8, weight: .medium)
        upImageView = NSImageView(
            image: NSImage(systemSymbolName: "chevron.up", accessibilityDescription: nil)!
                .withSymbolConfiguration(configuration)!,
        )
        downImageView = NSImageView(
            image: NSImage(systemSymbolName: "chevron.down", accessibilityDescription: nil)!
                .withSymbolConfiguration(configuration)!,
        )
        super.init(frame: .zero)
        translatesAutoresizingMaskIntoConstraints = true
        wantsLayer = true

        separator.boxType = .separator
        separator.translatesAutoresizingMaskIntoConstraints = false
        addSubview(separator)

        upImageView.contentTintColor = .tertiaryLabelColor
        upImageView.translatesAutoresizingMaskIntoConstraints = false
        addSubview(upImageView)

        downImageView.contentTintColor = .tertiaryLabelColor
        downImageView.translatesAutoresizingMaskIntoConstraints = false
        addSubview(downImageView)

        NSLayoutConstraint.activate([
            separator.leadingAnchor.constraint(equalTo: leadingAnchor),
            separator.centerYAnchor.constraint(equalTo: centerYAnchor),
            // Tahoe's capsule window insets the separator so it does not touch
            // the curved edge; Sequoia's runs full height.
            separator.heightAnchor.constraint(
                equalTo: heightAnchor, constant: style == .tahoe ? -8 : 0,
            ),
            upImageView.leadingAnchor.constraint(
                equalTo: separator.trailingAnchor, constant: Self.spacing,
            ),
            upImageView.widthAnchor.constraint(equalToConstant: Self.imageWidth),
            upImageView.trailingAnchor.constraint(
                equalTo: trailingAnchor, constant: -Self.padding,
            ),
            upImageView.centerYAnchor.constraint(equalTo: centerYAnchor, constant: -3),
            downImageView.leadingAnchor.constraint(equalTo: upImageView.leadingAnchor),
            downImageView.widthAnchor.constraint(equalTo: upImageView.widthAnchor),
            downImageView.centerYAnchor.constraint(equalTo: centerYAnchor, constant: 4),
        ])
    }

    @available(*, unavailable)
    required init?(coder _: NSCoder) { fatalError() }

    override var intrinsicContentSize: NSSize {
        NSSize(
            width: 1 + Self.spacing + Self.imageWidth + Self.padding,
            height: NSView.noIntrinsicMetric,
        )
    }

    override func mouseUp(with event: NSEvent) {
        guard (window as? CandidateWindowDragging)?.didDrag != true else { return }
        let localPoint = convert(event.locationInWindow, from: nil)
        // NSView default coordinates: y grows upward, so above the middle is
        // the up arrow.
        if localPoint.y > bounds.midY {
            if canPageUp { onPageUp?() }
        } else {
            if canPageDown { onPageDown?() }
        }
    }
}
