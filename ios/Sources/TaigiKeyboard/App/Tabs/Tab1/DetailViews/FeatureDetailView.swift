import SwiftUI

/// 新功能詳細頁面
///
/// 根據 JSON 資料驅動顯示，支援文字、圖片、輪播、外部連結。
struct FeatureDetailView: View {
    let feature: FeatureContent
    @StateObject private var languageManager = LanguageManager.shared

    var body: some View {
        Form {
            ForEach(feature.paragraphs.indices, id: \.self) { index in
                let paragraph = feature.paragraphs[index]

                Section {
                    paragraphView(paragraph)
                }

                // Render link attachment as a separate section (after the paragraph)
                if case .link(let linkText, let url) = paragraph.attachment {
                    Section {
                        Link(destination: URL(string: url)!) {
                            Label(languageManager.text(linkText.asLocalizedText), systemImage: "arrow.up.right.square")
                        }
                    }
                }
            }
        }
        .navigationTitle(languageManager.text(feature.title.asLocalizedText))
        .navigationBarTitleDisplayMode(.large)
    }

    @ViewBuilder
    private func paragraphView(_ paragraph: FeatureParagraph) -> some View {
        switch paragraph.attachment {
        case .slideshow(let images, let interval):
            VStack(alignment: .leading, spacing: 12) {
                paragraphText(paragraph)
                ImageSlideshowView(imageNames: images, interval: interval)
                    .frame(maxWidth: .infinity)
            }

        case .image(let name):
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
        Text(languageManager.text(paragraph.text.asLocalizedText))
            .lineSpacing(6)
            .fixedSize(horizontal: false, vertical: true)
    }
}
