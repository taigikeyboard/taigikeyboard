// The expand affordance at the collapsed row's edge: separator plus chevron.

import AppKit

/// The "there is more" control at the right edge of a collapsed expandable
/// window, from MacishType's `MacishChevronView` (`references/MacishType/
/// macos/MacishType/MacishCandidateWindow/MacishChevronView.swift`; MIT,
/// © 2026 Luke Chang) at the fixed 16pt font metrics.
final class CandidateChevronView: NSView {
    var onClick: (() -> Void)?

    private let separator = NSBox()
    private let imageView: NSImageView

    private static let spacing: CGFloat = 5
    private static let imageWidth: CGFloat = 16
    private static let padding: CGFloat = 6

    init(style: CandidateWindowStyle) {
        let configuration = NSImage.SymbolConfiguration(pointSize: 11, weight: .medium)
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
            // Tahoe insets the hairline from the capsule's curve, like the
            // page-arrow separator.
            separator.heightAnchor.constraint(
                equalTo: heightAnchor, constant: style == .tahoe ? -8 : 0,
            ),
            imageView.leadingAnchor.constraint(
                equalTo: separator.trailingAnchor, constant: Self.spacing,
            ),
            imageView.widthAnchor.constraint(equalToConstant: Self.imageWidth),
            imageView.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -Self.padding),
            imageView.centerYAnchor.constraint(equalTo: centerYAnchor),
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
