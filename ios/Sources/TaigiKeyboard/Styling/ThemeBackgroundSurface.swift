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
/// filter, nothing re-rasterises per keystroke).
///
/// `Equatable` so call sites can `.equatable()` and skip the body when nothing changed.
struct ThemeBackgroundSurface: View, Equatable {
    let surface: ThemeSurface
    var slice: KeyboardSurfaceSlice?

    var body: some View {
        switch surface.background {
        case let .solid(color):
            color.color
        case let .gradient(gradient):
            let points = slice.map(gradient.unitPoints(in:)) ?? gradient.unitPoints
            LinearGradient(colors: gradient.colors, startPoint: points.start, endPoint: points.end)
        case let .image(image):
            if let slice {
                ThemeImageSlice(image: image, tone: tone, slice: slice)
            } else {
                ThemeImageFill(image: image, tone: tone)
            }
        }
    }

    private var tone: Color {
        surface.dimsTowardWhite ? .white : .black
    }
}

/// A desaturated, dimmed photo covering the whole surface (aspect fill, aligned by the
/// photo's focus). A missing file (deleted theme photo, provisioning failure) paints the
/// seed grey so the keyboard never renders see-through.
private struct ThemeImageFill: View {
    let image: ThemeImageBackground
    let tone: Color

    var body: some View {
        if let uiImage = ThemeImageCache.shared.image(for: image.file) {
            FocusedPhotoFill(image: Image(uiImage: uiImage), focus: image.focus)
                .overlay(tone.opacity(image.dim))
        } else {
            UserThemeSeed.solidColor.color
        }
    }
}

/// `image` aspect-filled and desaturated over this view and clipped to it, placed where
/// `ThemeImageBackground.coverRect` puts it for `focus`: a fractional alignment guide on
/// both the surface and the photo lines up the surface's `focus` point with the photo's,
/// so the photo's left edge lands at `(surface − photo) × focus.x` (same for y). Plain
/// `Image` layout, no oversized frame + `.offset` (#429).
struct FocusedPhotoFill: View {
    let image: Image
    let focus: CGPoint

    var body: some View {
        Color.clear
            .alignmentGuide(HorizontalAlignment.photoFocus) { $0.width * focus.x }
            .alignmentGuide(VerticalAlignment.photoFocus) { $0.height * focus.y }
            .overlay(alignment: Alignment(horizontal: .photoFocus, vertical: .photoFocus)) {
                image
                    .resizable()
                    .scaledToFill()
                    .saturation(ThemeImageBackground.saturation)
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
    let image: ThemeImageBackground
    let tone: Color
    let slice: KeyboardSurfaceSlice

    var body: some View {
        Canvas(rendersAsynchronously: false) { context, size in
            let bounds = CGRect(origin: .zero, size: size)
            guard let uiImage = ThemeImageCache.shared.image(for: image.file) else {
                context.fill(Path(bounds), with: .color(UserThemeSeed.solidColor.color))
                return
            }
            let photoRect = ThemeImageBackground.coverRect(imageSize: uiImage.size, in: slice.keyboardRect(width: size.width), focus: image.focus)
            var photo = context
            photo.addFilter(.saturation(ThemeImageBackground.saturation))
            photo.draw(Image(uiImage: uiImage), in: photoRect)
            context.fill(Path(bounds), with: .color(tone.opacity(image.dim)))
        }
    }
}
