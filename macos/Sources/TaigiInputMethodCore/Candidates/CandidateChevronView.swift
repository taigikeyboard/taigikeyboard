// The expand affordance at the collapsed row's edge: separator plus chevron.

import AppKit

/// The "there is more" control at the right edge of a collapsed expandable
/// window, from MacishType's `MacishChevronView` (`references/MacishType/
/// macos/MacishType/MacishCandidateWindow/MacishChevronView.swift`; MIT,
/// © 2026 Luke Chang). Its base values are upstream's at 16pt; the symbol AND
/// the width it reserves both follow the text scale, as upstream scales them
/// (`MacishChevronView.swift:58-67`) — scaling the glyph alone would clip it.
final class CandidateChevronView: NSView {
    var onClick: (() -> Void)?

    private let separator = NSBox()
    private let imageView: NSImageView

    private static let baseSpacing: CGFloat = 5
    private static let baseImageWidth: CGFloat = 16
    private static let basePadding: CGFloat = 6
    private static let baseSymbolPointSize: CGFloat = 11

    private let spacing: CGFloat
    private let imageWidth: CGFloat
    private let padding: CGFloat

    init(style: CandidateWindowStyle, metrics: CandidateMetrics) {
        spacing = metrics.scaledSymbolMetric(Self.baseSpacing)
        imageWidth = metrics.scaledSymbolMetric(Self.baseImageWidth)
        padding = metrics.scaledSymbolMetric(Self.basePadding)
        let configuration = NSImage.SymbolConfiguration(
            pointSize: metrics.scaledSymbolMetric(Self.baseSymbolPointSize), weight: .medium,
        )
        imageView = NSImageView(
            image: NSImage(systemSymbolName: "chevron.down", accessibilityDescription: nil)!
                .withSymbolConfiguration(configuration)!,
        )
        super.init(frame: .zero)
        translatesAutoresizingMaskIntoConstraints = true
        wantsLayer = true

        separator.boxType = .separator
        separator.translatesAutoresizingMaskIntoConstraints = false
        addSubview(separator)

        imageView.contentTintColor = .tertiaryLabelColor
        imageView.translatesAutoresizingMaskIntoConstraints = false
        addSubview(imageView)

        NSLayoutConstraint.activate([
            separator.leadingAnchor.constraint(equalTo: leadingAnchor),
            separator.centerYAnchor.constraint(equalTo: centerYAnchor),
            // Tahoe insets the hairline from the window's rounded edge, like
            // the page-arrow separator.
            separator.heightAnchor.constraint(
                equalTo: heightAnchor, constant: style == .tahoe ? -metrics.tahoeSeparatorInset : 0,
            ),
            imageView.leadingAnchor.constraint(
                equalTo: separator.trailingAnchor, constant: spacing,
            ),
            imageView.widthAnchor.constraint(equalToConstant: imageWidth),
            imageView.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -padding),
            imageView.centerYAnchor.constraint(equalTo: centerYAnchor),
        ])
    }

    @available(*, unavailable)
    required init?(coder _: NSCoder) { fatalError() }

    override var intrinsicContentSize: NSSize {
        NSSize(
            width: 1 + spacing + imageWidth + padding,
            height: NSView.noIntrinsicMetric,
        )
    }

    /// The expand transition fades the chevron's content while the view itself
    /// carries the separator through — both are animated as one alpha.
    func setContentAlpha(_ alpha: CGFloat) {
        separator.alphaValue = alpha
        imageView.alphaValue = alpha
    }

    override func mouseUp(with _: NSEvent) {
        guard (window as? CandidateWindowDragging)?.didDrag != true else { return }
        onClick?()
    }
}
