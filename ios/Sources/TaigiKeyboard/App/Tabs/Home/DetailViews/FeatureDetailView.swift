import SwiftUI

/// Feature detail page.
///
/// JSON-driven display with text, images, slideshows, and external links.
struct FeatureDetailView: View {
    let feature: FeatureContent
    @Environment(DisplayLanguageStore.self) private var lang

    var body: some View {
        Form {
            ForEach(feature.paragraphs.indices, id: \.self) { index in
                let paragraph = feature.paragraphs[index]

                Section {
                    paragraphView(paragraph)
                }

                // Render link attachment as a separate section (after the paragraph)
                if case let .link(linkText, url) = paragraph.attachment {
                    Section {
                        Link(destination: URL(string: url)!) {
                            Label(linkText.resolve(for: lang.language), systemImage: "arrow.up.right.square")
                        }
                    }
                }
            }
        }
        .navigationTitle(feature.title.resolve(for: lang.language))
        .navigationBarTitleDisplayMode(.large)
    }

    @ViewBuilder
    private func paragraphView(_ paragraph: FeatureParagraph) -> some View {
        switch paragraph.attachment {
        case let .slideshow(images, interval):
            VStack(alignment: .leading, spacing: 12) {
                paragraphText(paragraph)
                ImageSlideshowView(imageNames: images, interval: interval)
                    .frame(maxWidth: .infinity)
            }

        case let .image(name):
            VStack(alignment: .leading, spacing: 12) {
                paragraphText(paragraph)
                Image(name)
                    .resizable()
                    .aspectRatio(contentMode: .fit)
                    .frame(maxWidth: .infinity)
            }

        case .navigation, .link, .none:
            paragraphText(paragraph)
        }
    }

    private func paragraphText(_ paragraph: FeatureParagraph) -> some View {
        Text(paragraph.text.resolve(for: lang.language))
            .lineSpacing(6)
            .fixedSize(horizontal: false, vertical: true)
    }
}
