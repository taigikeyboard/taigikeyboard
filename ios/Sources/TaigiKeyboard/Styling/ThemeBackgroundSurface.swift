// Paints a ThemeSurface (solid / gradient / photo) as the keyboard surface for every custom-theme render site.

import SwiftUI

/// The keyboard surface for a custom theme.
///
/// `slice` says where this view sits inside the whole keyboard: `nil` means the view IS
/// the whole surface (keyboard root, expanded candidate overlay, custom-theme card); an
/// overlay panel mounted below the candidate bar passes the keyboard's full height and
/// the bar's height so the gradient / photo it repaints is exactly its own slice and stays
/// continuous with the keyboard above it. A sliced photo is drawn inside a `Canvas`, which
/// clips to its own bounds natively — never an oversized frame + `.offset` + `.clipped()`,
/// which froze the keyboard extension on gradient themes (#429). The whole-surface photo
/// uses plain `Image` modifiers (one texture upload, the saturation is a render-server
/// filter, nothing re-rasterises per keystroke). The photo is decoded off the main actor
/// (`ThemePhotoImage`); until it lands the surface paints the seed grey. `photoVariant`
/// picks the decode — `.thumbnail` for small previews (theme cards).
///
/// `Equatable` so call sites can `.equatable()` and skip the body when nothing changed.
struct ThemeBackgroundSurface: View, Equatable {
    let surface: ThemeSurface
    var slice: KeyboardSurfaceSlice?
    var photoVariant: ThemeImageVariant = .full

    var body: some View {
        switch surface.background {
        case let .solid(color):
            color.color
        case let .gradient(gradient):
            let points = slice.map(gradient.unitPoints(in:)) ?? gradient.unitPoints
            LinearGradient(colors: gradient.colors, startPoint: points.start, endPoint: points.end)
        case let .image(image):
            ThemePhotoImage(file: image.file, variant: photoVariant) { uiImage in
                if let slice {
                    ThemeImageSlice(uiImage: uiImage, image: image, tone: tone, slice: slice)
                } else {
                    FocusedPhotoFill(image: Image(uiImage: uiImage), focus: image.focus, zoom: image.zoom)
                        .overlay(tone.opacity(image.dim))
                }
            } placeholder: {
                // Not decoded yet, or the file is missing (deleted theme photo, provisioning
                // failure): the seed grey, so the keyboard never renders see-through.
                UserThemeSeed.solidColor.color
            }
        }
    }

    private var tone: Color {
        surface.dimsTowardWhite ? .white : .black
    }
}

/// `image` aspect-filled and desaturated over this view and clipped to it, placed where
/// `ThemeImageBackground.coverRect` puts it for `focus` and `zoom`: a fractional alignment
/// guide on both the surface and the photo lines up the surface's `focus` point with the
/// photo's, so the photo's left edge lands at `(surface − photo) × focus.x` (same for y).
/// `zoom` scales the photo about that shared point, which keeps it fixed, so the zoomed
/// left edge lands at `(surface − photo × zoom) × focus.x` — `coverRect`'s. Plain `Image`
/// layout plus a render transform, no oversized frame + `.offset` (#429).
struct FocusedPhotoFill: View {
    let image: Image
    let focus: CGPoint
    let zoom: CGFloat

    var body: some View {
        Color.clear
            .alignmentGuide(HorizontalAlignment.photoFocus) { $0.width * focus.x }
            .alignmentGuide(VerticalAlignment.photoFocus) { $0.height * focus.y }
            .overlay(alignment: Alignment(horizontal: .photoFocus, vertical: .photoFocus)) {
                image
                    .resizable()
                    .scaledToFill()
                    .saturation(ThemeImageBackground.saturation)
                    .scaleEffect(zoom, anchor: UnitPoint(x: focus.x, y: focus.y))
                    .alignmentGuide(HorizontalAlignment.photoFocus) { $0.width * focus.x }
                    .alignmentGuide(VerticalAlignment.photoFocus) { $0.height * focus.y }
            }
            .clipped()
    }
}

private extension HorizontalAlignment {
    enum PhotoFocus: AlignmentID {
        static func defaultValue(in context: ViewDimensions) -> CGFloat {
            context[HorizontalAlignment.center]
        }
    }

    static let photoFocus = HorizontalAlignment(PhotoFocus.self)
}

private extension VerticalAlignment {
    enum PhotoFocus: AlignmentID {
        static func defaultValue(in context: ViewDimensions) -> CGFloat {
            context[VerticalAlignment.center]
        }
    }

    static let photoFocus = VerticalAlignment(PhotoFocus.self)
}

/// The photo drawn as this view's slice of the whole keyboard: the cover rect is computed
/// over the keyboard frame (`KeyboardSurfaceSlice.keyboardRect`) and the `Canvas` shows
/// only what falls inside its own bounds.
private struct ThemeImageSlice: View {
    let uiImage: UIImage
    let image: ThemeImageBackground
    let tone: Color
    let slice: KeyboardSurfaceSlice

    var body: some View {
        Canvas(rendersAsynchronously: false) { context, size in
            let bounds = CGRect(origin: .zero, size: size)
            let photoRect = ThemeImageBackground.coverRect(imageSize: uiImage.size, in: slice.keyboardRect(width: size.width), focus: image.focus, zoom: image.zoom)
            var photo = context
            photo.addFilter(.saturation(ThemeImageBackground.saturation))
            photo.draw(Image(uiImage: uiImage), in: photoRect)
            context.fill(Path(bounds), with: .color(tone.opacity(image.dim)))
        }
    }
}
