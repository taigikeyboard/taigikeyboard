// The candidate window's background: vibrancy on Sequoia, glass on Tahoe.

import AppKit

/// Type-erased wrapper for the candidate window's background view, ported from
/// MacishType's `MacishBackdrop` (`references/MacishType/macos/MacishType/
/// MacishCandidateWindow/MacishBasePanel.swift:497-566`; MIT, © 2026 Luke
/// Chang). It encapsulates the version branch between `NSVisualEffectView`
/// (macOS 14+) and `NSGlassEffectView` (macOS 26+) so panels never see
/// `if #available`.
///
/// Corners are applied as a mask rather than a layer radius because the
/// horizontal window is asymmetric while paging arrows are shown: square-ish
/// on the candidate side, a half-height pill on the arrow side.
@MainActor
struct CandidateBackdrop {
    /// The view installed as the panel's `contentView`.
    let view: NSView
    /// Where the panel's own subviews go — the glass view requires content in
    /// a designated container rather than as direct subviews.
    let contentArea: NSView

    private let uniformCorners: (NSSize, CGFloat) -> Void
    private let asymmetricCorners: (NSSize, CGFloat, CGFloat) -> Void

    func applyUniformCorners(size: NSSize, radius: CGFloat) {
        uniformCorners(size, radius)
    }

    func applyAsymmetricCorners(size: NSSize, leftRadius: CGFloat, rightRadius: CGFloat) {
        asymmetricCorners(size, leftRadius, rightRadius)
    }

    /// Builds the backdrop for `style`, falling back to vibrancy for a
    /// `.tahoe` asked for below macOS 26 — `NSGlassEffectView` does not exist
    /// there to construct. Production never asks: the style comes from
    /// `CandidateWindowStyle.systemStyle`.
    static func make(style: CandidateWindowStyle) -> CandidateBackdrop {
        if style == .tahoe, #available(macOS 26, *) {
            let glass = GlassBackgroundView()
            return CandidateBackdrop(
                view: glass,
                contentArea: glass.container,
                uniformCorners: { glass.applyUniformCorners(size: $0, radius: $1) },
                asymmetricCorners: {
                    glass.applyAsymmetricCorners(size: $0, leftRadius: $1, rightRadius: $2)
                },
            )
        }
        let vibrancy = VibrancyBackgroundView()
        return CandidateBackdrop(
            view: vibrancy,
            contentArea: vibrancy,
            uniformCorners: { vibrancy.applyUniformCorners(size: $0, radius: $1) },
            asymmetricCorners: {
                vibrancy.applyAsymmetricCorners(size: $0, leftRadius: $1, rightRadius: $2)
            },
        )
    }
}

/// A rounded-rectangle path whose left and right corner radii differ — the
/// pill-ended shape the horizontal window takes while its paging edge shows.
private func asymmetricCornerPath(
    size: NSSize,
    leftRadius: CGFloat,
    rightRadius: CGFloat,
) -> CGPath {
    let rect = CGRect(origin: .zero, size: size)
    let left = min(leftRadius, rect.height / 2)
    let right = min(rightRadius, rect.height / 2)
    let path = CGMutablePath()
    path.move(to: CGPoint(x: left, y: rect.maxY))
    path.addLine(to: CGPoint(x: rect.maxX - right, y: rect.maxY))
    path.addArc(
        center: CGPoint(x: rect.maxX - right, y: rect.maxY - right),
        radius: right, startAngle: .pi / 2, endAngle: 0, clockwise: true,
    )
    path.addLine(to: CGPoint(x: rect.maxX, y: right))
    path.addArc(
        center: CGPoint(x: rect.maxX - right, y: right),
        radius: right, startAngle: 0, endAngle: -.pi / 2, clockwise: true,
    )
    path.addLine(to: CGPoint(x: left, y: 0))
    path.addArc(
        center: CGPoint(x: left, y: left),
        radius: left, startAngle: -.pi / 2, endAngle: -.pi, clockwise: true,
    )
    path.addLine(to: CGPoint(x: 0, y: rect.maxY - left))
    path.addArc(
        center: CGPoint(x: left, y: rect.maxY - left),
        radius: left, startAngle: .pi, endAngle: .pi / 2, clockwise: true,
    )
    path.closeSubpath()
    return path
}

/// The Sequoia backdrop: HUD vibrancy behind the window, corners cut with a
/// mask image so they can be asymmetric.
private final class VibrancyBackgroundView: NSVisualEffectView {
    private var cachedSize: NSSize = .zero
    private var cachedLeft: CGFloat = -1
    private var cachedRight: CGFloat = -1

    override init(frame: NSRect) {
        super.init(frame: frame)
        material = .hudWindow
        state = .active
        blendingMode = .behindWindow
        wantsLayer = true
        layer?.masksToBounds = true
    }

    @available(*, unavailable)
    required init?(coder _: NSCoder) {
        fatalError()
    }

    func applyUniformCorners(size: NSSize, radius: CGFloat) {
        applyAsymmetricCorners(size: size, leftRadius: radius, rightRadius: radius)
    }

    func applyAsymmetricCorners(size: NSSize, leftRadius: CGFloat, rightRadius: CGFloat) {
        guard size.width > 0, size.height > 0 else { return }
        // Rebuilding the mask image is the expensive part, and resize events
        // repeat the same geometry — repaint only when it actually changed.
        if size == cachedSize, leftRadius == cachedLeft, rightRadius == cachedRight {
            return
        }
        cachedSize = size
        cachedLeft = leftRadius
        cachedRight = rightRadius
        let path = asymmetricCornerPath(size: size, leftRadius: leftRadius, rightRadius: rightRadius)
        maskImage = NSImage(size: size, flipped: false) { _ in
            NSBezierPath(cgPath: path).fill()
            return true
        }
    }
}

/// The Tahoe backdrop: the macOS 26 glass material, corners via the view's own
/// radius when uniform and a shape-layer mask when asymmetric.
@available(macOS 26, *)
private final class GlassBackgroundView: NSGlassEffectView {
    fileprivate let container = NSView()
    private var maskLayer: CAShapeLayer?

    override init(frame: NSRect) {
        super.init(frame: frame)
        style = .regular
        contentView = container
        // PRIVATE API, inherited from upstream (`MacishBasePanel.swift:663-670`)
        // and kept deliberately: without it a small glass surface picks its own
        // light/dark scheme from what is behind it, flipping the text colour
        // against the window's appearance mid-typing. Guarded so a macOS build
        // that drops the selector degrades to that cosmetic flicker, not a
        // crash. Remove when a public equivalent appears.
        if responds(to: Selector(("_adaptiveAppearance"))) {
            setValue(1, forKey: "_adaptiveAppearance")
        }
        wantsLayer = true
        layer?.masksToBounds = true
    }

    @available(*, unavailable)
    required init?(coder _: NSCoder) {
        fatalError()
    }

    func applyUniformCorners(size _: NSSize, radius: CGFloat) {
        layer?.mask = nil
        maskLayer = nil
        cornerRadius = radius
    }

    func applyAsymmetricCorners(size: NSSize, leftRadius: CGFloat, rightRadius: CGFloat) {
        guard size.width > 0, size.height > 0 else { return }
        cornerRadius = 0
        let shape = maskLayer ?? {
            let shape = CAShapeLayer()
            layer?.mask = shape
            maskLayer = shape
            return shape
        }()
        shape.frame = CGRect(origin: .zero, size: size)
        shape.path = asymmetricCornerPath(size: size, leftRadius: leftRadius, rightRadius: rightRadius)
    }
}
