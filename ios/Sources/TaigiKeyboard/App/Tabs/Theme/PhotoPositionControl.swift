// The theme editor's photo position control: the drag surface over the live preview and
// the finger → focus math behind it.

import SwiftUI

/// Maps a drag on the live keyboard preview to a photo's `focusX` / `focusY`. The photo is
/// aspect-filled, so at most one axis overflows the surface; only that axis moves. The
/// photo follows the finger: its left edge sits at `(surface − cover) × focus`, so a drag
/// of `d` points moves the focus by `d / (surface − cover)` (negative overflow → dragging
/// right lowers `focusX`). Clamping into 0…1 is `ThemeImageBackground`'s.
enum PhotoPositionDrag {
    /// One VoiceOver adjustable step, as a fraction of the overflowing axis.
    static let accessibilityStep: Double = 0.1
    /// Overflow below this many points is treated as none (rounding in the cover scale).
    private static let minimumOverflow: CGFloat = 0.5

    /// How far the aspect-filled photo overflows `surface` on each axis (≤ 0 points; 0 =
    /// fits exactly). Independent of focus, so the centred cover rect measures it.
    private static func overflow(imageSize: CGSize, surface: CGSize) -> CGSize {
        let cover = ThemeImageBackground.coverRect(imageSize: imageSize, in: CGRect(origin: .zero, size: surface), focus: ThemeImageBackground.centredFocus)
        return CGSize(width: surface.width - cover.width, height: surface.height - cover.height)
    }

    /// The axis the photo can move along over `surface`, or nil when it fits exactly.
    static func axis(imageSize: CGSize, surface: CGSize) -> Axis? {
        let overflow = overflow(imageSize: imageSize, surface: surface)
        if -overflow.height >= minimumOverflow {
            return .vertical
        }
        if -overflow.width >= minimumOverflow {
            return .horizontal
        }
        return nil
    }

    /// `start` after dragging the photo by `translation` along `axis` (the one `axis(…)`
    /// returned, so its overflow is non-zero).
    static func dragged(
        _ start: ThemeImageBackground, by translation: CGSize, along axis: Axis,
        imageSize: CGSize, surface: CGSize,
    ) -> ThemeImageBackground {
        let overflow = overflow(imageSize: imageSize, surface: surface)
        switch axis {
        case .horizontal: return start.with(focus: CGPoint(x: start.focusX + translation.width / overflow.width, y: start.focusY))
        case .vertical: return start.with(focus: CGPoint(x: start.focusX, y: start.focusY + translation.height / overflow.height))
        }
    }

    /// `photo` stepped one `accessibilityStep` along `axis` (increment = toward the right /
    /// bottom edge).
    static func stepped(_ photo: ThemeImageBackground, along axis: Axis, increment: Bool) -> ThemeImageBackground {
        let delta = increment ? accessibilityStep : -accessibilityStep
        switch axis {
        case .horizontal: return photo.with(focus: CGPoint(x: photo.focusX + delta, y: photo.focusY))
        case .vertical: return photo.with(focus: CGPoint(x: photo.focusX, y: photo.focusY + delta))
        }
    }
}

/// The drag surface laid over the live preview while the background is a photo: swallows
/// the preview keys' touches, draws a double-headed arrow along the axis the photo can
/// move, and writes every drag through `PhotoPositionDrag` into `photo`, so the photo
/// follows the finger. When the photo fits the preview exactly there is nothing to move:
/// no arrow, no gesture. For VoiceOver it is one adjustable element labelled `label` that
/// steps the movable axis. Mirrors `GradientDirectionOverlay`.
struct PhotoPositionOverlay: View {
    let label: String
    /// The photo's pixel size (its aspect is all the math needs).
    let imageSize: CGSize
    @Binding var photo: ThemeImageBackground

    /// The photo when the current drag began; the translation is applied to it.
    @State private var dragStart: ThemeImageBackground?

    var body: some View {
        GeometryReader { geometry in
            if let axis = PhotoPositionDrag.axis(imageSize: imageSize, surface: geometry.size) {
                PhotoPositionArrow(axis: axis)
                    .equatable()
                    .contentShape(Rectangle())
                    .gesture(
                        DragGesture(minimumDistance: 0)
                            .onChanged { value in
                                let start = dragStart ?? photo
                                dragStart = start
                                photo = PhotoPositionDrag.dragged(
                                    start, by: value.translation, along: axis,
                                    imageSize: imageSize, surface: geometry.size,
                                )
                            }
                            .onEnded { _ in dragStart = nil },
                    )
                    .accessibilityElement(children: .ignore)
                    .accessibilityLabel(label)
                    .accessibilityValue("\(Int(((axis == .horizontal ? photo.focusX : photo.focusY) * 100).rounded()))%")
                    .accessibilityAdjustableAction { direction in
                        photo = PhotoPositionDrag.stepped(photo, along: axis, increment: direction == .increment)
                    }
            }
        }
    }
}

/// A double-headed arrow through the centre along `axis`. `Equatable` on the axis alone so
/// edits to other controls (which re-render the whole editor body) skip the redraw.
private struct PhotoPositionArrow: View, Equatable {
    let axis: Axis

    private static let strokeWidth: CGFloat = 3
    private static let headLength: CGFloat = 10
    /// Half the arrow length as a fraction of the preview's shorter side.
    private static let halfLengthFraction: CGFloat = 0.25

    var body: some View {
        Canvas { context, size in
            let center = size.center
            let halfLength = min(size.width, size.height) * Self.halfLengthFraction
            let along = axis == .horizontal ? CGVector(dx: 1, dy: 0) : CGVector(dx: 0, dy: 1)
            let across = CGVector(dx: along.dy, dy: along.dx)

            var path = Path()
            for sign in [1.0, -1.0] {
                let tip = CGPoint(x: center.x + along.dx * halfLength * sign, y: center.y + along.dy * halfLength * sign)
                path.move(to: center)
                path.addLine(to: tip)
                // Arrowhead: two strokes swept back from the tip at 45°.
                for side in [1.0, -1.0] {
                    path.move(to: tip)
                    path.addLine(to: CGPoint(
                        x: tip.x - (along.dx * sign - across.dx * side) * Self.headLength,
                        y: tip.y - (along.dy * sign - across.dy * side) * Self.headLength,
                    ))
                }
            }
            // White on a dark halo reads on any photo.
            context.addFilter(.shadow(color: .black.opacity(0.6), radius: 2))
            context.stroke(path, with: .color(.white), style: StrokeStyle(lineWidth: Self.strokeWidth, lineCap: .round))
        }
    }
}
