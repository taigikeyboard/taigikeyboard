import SwiftUI

// Card-shelf building blocks shared by the Theme and Layout tabs: a titled horizontal shelf,
// a selectable preview card, a screenshot preview and a create-new card.

/// Shared card dimensions so the Theme and Layout shelves line up. `previewAspectRatio` matches
/// the generated `*_preview` assets (720×454, `PreviewAssetGeneratorTests`).
enum GalleryCardMetrics {
    static let width: CGFloat = 240
    static let previewAspectRatio: CGFloat = 720.0 / 454.0
    static let cardSpacing: CGFloat = 12
    static let shelfSpacing: CGFloat = 28
}

// MARK: - Shelf

/// One shelf: a gray section header above a horizontally scrolling row of cards.
struct GalleryShelf<Content: View>: View {
    let title: String
    @ViewBuilder let content: Content

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(title)
                .font(AppStyle.sectionHeaderFont)
                .foregroundStyle(.secondary)
                .padding(.horizontal, AppStyle.horizontalPadding)

            ScrollView(.horizontal, showsIndicators: false) {
                HStack(alignment: .top, spacing: GalleryCardMetrics.cardSpacing) {
                    content
                }
                .padding(.horizontal, AppStyle.horizontalPadding)
            }
        }
    }
}

// MARK: - Card

/// A selectable card: a tappable preview with the selection marker (a dimming mask, a blue
/// circle checkmark and a blue stroke) above a title row. Only the preview is the tap target,
/// so a trailing `accessory` (the custom-theme `…` menu) never selects the card.
struct GalleryCard<Preview: View, Accessory: View>: View {
    let title: String
    let isSelected: Bool
    let onTap: () -> Void
    @ViewBuilder let preview: Preview
    @ViewBuilder let accessory: Accessory

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            Button(action: onTap) {
                // Color.clear sets the aspect-ratio box; the preview is overlaid to fill it.
                Color.clear
                    .aspectRatio(GalleryCardMetrics.previewAspectRatio, contentMode: .fit)
                    .overlay(preview)
                    .overlay {
                        if isSelected {
                            RoundedRectangle(cornerRadius: AppStyle.previewCornerRadius)
                                .fill(Color.black.opacity(0.25))
                            Circle()
                                .fill(AppStyle.accentBlue)
                                .frame(width: 36, height: 36)
                                .overlay(
                                    Image(latinSystemName: "checkmark")
                                        .font(AppStyle.appFont(size: 16).bold())
                                        .foregroundColor(.white),
                                )
                        }
                    }
                    .clipShape(RoundedRectangle(cornerRadius: AppStyle.previewCornerRadius))
                    .frame(width: GalleryCardMetrics.width)
                    .overlay(
                        RoundedRectangle(cornerRadius: AppStyle.previewCornerRadius)
                            .stroke(isSelected ? AppStyle.accentBlue : Color.clear, lineWidth: 2.5),
                    )
            }
            .buttonStyle(.plain)

            HStack(spacing: 4) {
                Text(title)
                    .font(AppStyle.captionFont)
                    .fontWeight(.semibold)
                    .foregroundColor(.primary)
                    .lineLimit(1)

                Spacer(minLength: 0)

                accessory
            }
            .frame(width: GalleryCardMetrics.width)
        }
        .frame(width: GalleryCardMetrics.width, alignment: .leading)
    }
}

extension GalleryCard where Accessory == EmptyView {
    init(title: String, isSelected: Bool, onTap: @escaping () -> Void, @ViewBuilder preview: () -> Preview) {
        self.init(title: title, isSelected: isSelected, onTap: onTap, preview: preview, accessory: { EmptyView() })
    }
}

// MARK: - Screenshot preview

/// A bundled card screenshot filling the card's aspect box, or a neutral placeholder while the
/// asset is missing.
struct GalleryScreenshot: View {
    let imageName: String
    let title: String

    var body: some View {
        if let uiImage = UIImage(named: imageName) {
            Image(uiImage: uiImage)
                .resizable()
                .aspectRatio(contentMode: .fill)
        } else {
            Rectangle()
                .fill(Color(.tertiarySystemBackground))
                .overlay(
                    VStack(spacing: 6) {
                        Image(latinSystemName: "keyboard")
                            .font(AppStyle.appFont(size: 28))
                            .foregroundColor(.secondary)
                        Text(title)
                            .font(AppStyle.captionFont)
                            .foregroundColor(.secondary)
                            .lineLimit(1)
                    },
                )
        }
    }
}

// MARK: - Create New card

/// The leading card of a custom shelf: a gray panel with a centered "+" glyph. With a
/// `comingSoonBadge` the panel is dimmed under that badge and the card is not tappable.
struct GalleryCreateCard: View {
    let title: String
    var comingSoonBadge: String?
    var onTap: () -> Void = {}

    private let plusGlyphSize: CGFloat = 28

    var body: some View {
        Button(action: onTap) { content }
            .buttonStyle(.plain)
            .disabled(comingSoonBadge != nil)
    }

    private var content: some View {
        VStack(alignment: .leading, spacing: 6) {
            // Color.clear sets the aspect-ratio box; the panel is overlaid to fill it.
            Color.clear
                .aspectRatio(GalleryCardMetrics.previewAspectRatio, contentMode: .fit)
                .overlay(
                    ZStack {
                        RoundedRectangle(cornerRadius: AppStyle.previewCornerRadius)
                            .fill(Color(.systemGray4))

                        Image(latinSystemName: "plus")
                            .resizable()
                            .scaledToFit()
                            .frame(width: plusGlyphSize, height: plusGlyphSize)
                            .foregroundColor(.primary)

                        if let comingSoonBadge {
                            RoundedRectangle(cornerRadius: AppStyle.previewCornerRadius)
                                .fill(Color.black.opacity(0.5))
                            Text(comingSoonBadge)
                                .font(AppStyle.captionFont)
                                .fontWeight(.bold)
                                .foregroundColor(.white)
                                .padding(.horizontal, 12)
                                .padding(.vertical, 6)
                                .background(Color.black.opacity(0.7), in: Capsule())
                        }
                    },
                )
                .frame(width: GalleryCardMetrics.width)

            Text(title)
                .font(AppStyle.captionFont)
                .fontWeight(.semibold)
                .foregroundColor(comingSoonBadge == nil ? .primary : .secondary)
                .lineLimit(1)
        }
        .frame(width: GalleryCardMetrics.width, alignment: .leading)
    }
}
