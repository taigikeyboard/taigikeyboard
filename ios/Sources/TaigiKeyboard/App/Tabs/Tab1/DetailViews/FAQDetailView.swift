import SwiftUI

/// FAQ 詳細頁面
///
/// 根據 JSON 資料驅動顯示，支援文字、圖片、in-app 導航連結。
struct FAQDetailView: View {
    let faq: FeatureContent
    @ObservedObject var viewModel: SetupGuideViewModel
    @StateObject private var languageManager = LanguageManager.shared

    var body: some View {
        Form {
            ForEach(faq.paragraphs.indices, id: \.self) { index in
                let paragraph = faq.paragraphs[index]

                Section {
                    paragraphView(paragraph)
                }

                // Render navigation attachment as a separate section (after the paragraph)
                if case .navigation(let navText, let destination, let navIcon) = paragraph.attachment {
                    Section {
                        NavigationLink {
                            navigationDestination(destination)
                        } label: {
                            Label(
                                languageManager.text(navText.asLocalizedText),
                                systemImage: navIcon.ios
                            )
                        }
                    }
                }
            }
        }
        .navigationTitle(languageManager.text(faq.title.asLocalizedText))
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
                    .clipShape(RoundedRectangle(cornerRadius: 8))
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

    @ViewBuilder
    private func navigationDestination(_ destination: String) -> some View {
        switch destination {
        case "setup_guide":
            SetupGuideView(viewModel: viewModel)
        case "feedback":
            FeedbackDetailView()
        default:
            EmptyView()
        }
    }
}
