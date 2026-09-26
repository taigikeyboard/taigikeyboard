// The theme editor's photo position control: the drag / pinch surface over the live preview
// and the finger → focus / zoom math behind it.

import SwiftUI

/// Maps gestures on the live keyboard preview to a photo's `focusX` / `focusY` / `zoom`. The
/// photo is aspect-filled times `zoom`: at zoom 1 at most one axis overflows the surface,
/// zoomed in both do, and only overflowing axes move. The photo follows the finger: its left
/// edge sits at `(surface − cover) × focus`, so a drag of `d` points moves the focus by
/// `d / (surface − cover)` (negative overflow → dragging right lowers `focusX`). A pinch
/// scales `zoom` about the focus point. Clamping is `ThemeImageBackground`'s.
enum PhotoPositionDrag {
    /// One VoiceOver adjustable step, as a fraction of the overflowing axis.
    static let accessibilityStep: Double = 0.1
    /// Overflow below this many points is treated as none (rounding in the cover scale).
    private static let minimumOverflow: CGFloat = 0.5

    /// How far the photo at `zoom` overflows `surface` on each axis (≤ 0 points; 0 = fits
    /// exactly). Independent of focus, so the centred cover rect measures it.
    private static func overflow(imageSize: CGSize, surface: CGSize, zoom: Double) -> CGSize {
        let cover = ThemeImageBackground.coverRect(
            imageSize: imageSize, in: CGRect(origin: .zero, size: surface),
            focus: ThemeImageBackground.centredFocus, zoom: zoom,
        )
        return CGSize(width: surface.width - cover.width, height: surface.height - cover.height)
    }

    /// The axes `photo` can move along over `surface` (empty when it fits exactly).
    static func axes(_ photo: ThemeImageBackground, imageSize: CGSize, surface: CGSize) -> Axis.Set {
        axes(overflowing: overflow(imageSize: imageSize, surface: surface, zoom: photo.zoom))
    }

    private static func axes(overflowing overflow: CGSize) -> Axis.Set {
        var axes: Axis.Set = []
        if -overflow.width >= minimumOverflow {
            axes.insert(.horizontal)
        }
        if -overflow.height >= minimumOverflow {
            axes.insert(.vertical)
        }
        return axes
    }

    /// The one axis VoiceOver steps: vertical when it moves (a portrait photo on the wide
    /// keyboard), else horizontal, else nil.
    static func accessibilityAxis(of axes: Axis.Set) -> Axis? {
        if axes.contains(.vertical) {
            return .vertical
        }
        return axes.contains(.horizontal) ? .horizontal : nil
    }

    /// `start` after dragging the photo by `translation`; each axis that does not overflow
    /// keeps its focus.
    static func dragged(
        _ start: ThemeImageBackground, by translation: CGSize,
        imageSize: CGSize, surface: CGSize,
    ) -> ThemeImageBackground {
        let overflow = overflow(imageSize: imageSize, surface: surface, zoom: start.zoom)
        let axes = axes(overflowing: overflow)
        return start.with(focus: CGPoint(
            x: axes.contains(.horizontal) ? start.focusX + translation.width / overflow.width : start.focusX,
            y: axes.contains(.vertical) ? start.focusY + translation.height / overflow.height : start.focusY,
        ))
    }

    /// `start` after a pinch of `magnification` (1 = unchanged); focus kept.
    static func zoomed(_ start: ThemeImageBackground, by magnification: CGFloat) -> ThemeImageBackground {
        start.with(zoom: start.zoom * magnification)
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

/// The gesture surface laid over the live preview while the background is a photo: swallows
/// the preview keys' touches, draws a double-headed arrow along each axis the photo can
/// move, and writes every gesture through `PhotoPositionDrag` into `photo` — one finger
/// drags (the photo follows it), two fingers pinch the zoom. A pinch freezes the drag; when
/// it ends the drag re-bases, so a finger left down never jumps the photo. For VoiceOver it
/// is one adjustable element labelled `label` that steps the vertical (else horizontal)
/// axis. Mirrors `GradientDirectionOverlay`.
struct PhotoPositionOverlay: View {
    let label: String
    /// The photo's pixel size (its aspect is all the math needs).
    let imageSize: CGSize
    @Binding var photo: ThemeImageBackground

    /// The photo and translation the current drag is measured from; nil until the drag's
    /// first change, and again after a pinch so the drag re-bases.
    @State private var dragBase: (photo: ThemeImageBackground, translation: CGSize)?
    /// The photo when the current pinch began.
    @State private var pinchStart: ThemeImageBackground?

    var body: some View {
        GeometryReader { geometry in
            let axes = PhotoPositionDrag.axes(photo, imageSize: imageSize, surface: geometry.size)
            PhotoPositionArrow(axes: axes)
                .equatable()
                .contentShape(Rectangle())
                .gesture(
                    SimultaneousGesture(
                        DragGesture(minimumDistance: 0)
                            .onChanged { value in
                                guard pinchStart == nil else { return }
                                let base = dragBase ?? (photo, value.translation)
                                if dragBase == nil {
                                    dragBase = base
                                }
                                update(to: PhotoPositionDrag.dragged(
                                    base.photo,
                                    by: CGSize(width: value.translation.width - base.translation.width,
                                               height: value.translation.height - base.translation.height),
                                    imageSize: imageSize, surface: geometry.size,
                                ))
                            }
                            .onEnded { _ in dragBase = nil },
                        MagnifyGesture()
                            .onChanged { value in
                                let start = pinchStart ?? photo
                                if pinchStart == nil {
                                    pinchStart = start
                                    dragBase = nil
                                }
                                update(to: PhotoPositionDrag.zoomed(start, by: value.magnification))
                            }
                            .onEnded { _ in pinchStart = nil },
                    ),
                )
                .accessibilityElement(children: .ignore)
                .accessibilityLabel(label)
                .modifier(PhotoPositionAccessibility(axis: PhotoPositionDrag.accessibilityAxis(of: axes), photo: $photo))
        }
    }

    /// Writes `next` only when it differs: a drag held at an edge or a pinch past the zoom
    /// cap would otherwise republish the unchanged theme every frame.
    private func update(to next: ThemeImageBackground) {
        if next != photo {
            photo = next
        }
    }
}

/// The overlay's VoiceOver value + adjustable action for `axis`; hidden when nothing moves.
private struct PhotoPositionAccessibility: ViewModifier {
    let axis: Axis?
    @Binding var photo: ThemeImageBackground

    func body(content: Content) -> some View {
        if let axis {
            content
                .accessibilityValue("\(Int(((axis == .horizontal ? photo.focusX : photo.focusY) * 100).rounded()))%")
                .accessibilityAdjustableAction { direction in
                    photo = PhotoPositionDrag.stepped(photo, along: axis, increment: direction == .increment)
                }
        } else {
            content.accessibilityHidden(true)
        }
    }
}

/// A double-headed arrow through the centre along each of `axes` (a cross when both, nothing
/// when neither). `Equatable` on the axes alone so edits to other controls (which re-render
/// the whole editor body) skip the redraw.
private struct PhotoPositionArrow: View, Equatable {
    let axes: Axis.Set

    private static let strokeWidth: CGFloat = 3
    private static let headLength: CGFloat = 10
    /// Half the arrow length as a fraction of the preview's shorter side.
    private static let halfLengthFraction: CGFloat = 0.25

    var body: some View {
        Canvas { context, size in
            let center = size.center
            let halfLength = min(size.width, size.height) * Self.halfLengthFraction
            var path = Path()
            if axes.contains(.horizontal) {
                Self.addArrow(to: &path, center: center, halfLength: halfLength, along: CGVector(dx: 1, dy: 0))
            }
            if axes.contains(.vertical) {
                Self.addArrow(to: &path, center: center, halfLength: halfLength, along: CGVector(dx: 0, dy: 1))
            }
            // White on a dark halo reads on any photo.
            context.addFilter(.shadow(color: .black.opacity(0.6), radius: 2))
            context.stroke(path, with: .color(.white), style: StrokeStyle(lineWidth: Self.strokeWidth, lineCap: .round))
        }
    }

    /// A double-headed arrow through `center` along the unit vector `along`.
    private static func addArrow(to path: inout Path, center: CGPoint, halfLength: CGFloat, along: CGVector) {
        let across = CGVector(dx: along.dy, dy: along.dx)
        for sign in [1.0, -1.0] {
            let tip = CGPoint(x: center.x + along.dx * halfLength * sign, y: center.y + along.dy * halfLength * sign)
            path.move(to: center)
            path.addLine(to: tip)
            // Arrowhead: two strokes swept back from the tip at 45°.
            for side in [1.0, -1.0] {
                path.move(to: tip)
                path.addLine(to: CGPoint(
                    x: tip.x - (along.dx * sign - across.dx * side) * headLength,
                    y: tip.y - (along.dy * sign - across.dy * side) * headLength,
                ))
            }
        }
    }
}
