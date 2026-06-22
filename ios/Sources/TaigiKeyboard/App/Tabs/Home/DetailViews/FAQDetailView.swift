// 中文: FAQ 詳情頁,以 JSON 驅動;支援文字、圖片、輪播、In-app 導覽。

import SwiftUI

/// FAQ detail page.
///
/// JSON-driven display with text, images, and in-app navigation links.
// 中文: FAQ 詳情頁。faq 來自 FeatureContentLoader.faqs;
// 中文: viewModel 用於 navigation attachment 跳到 setup guide。
struct FAQDetailView: View {
    let faq: FeatureContent
    @ObservedObject var viewModel: SetupGuideViewModel
    @Environment(DisplayLanguageStore.self) private var lang

    var body: some View {
        Form {
            ForEach(faq.paragraphs.indices, id: \.self) { index in
                let paragraph = faq.paragraphs[index]

                Section {
                    paragraphView(paragraph)
                }

                // Render navigation attachment as a separate section (after the paragraph)
                if case let .navigation(navText, destination, navIcon) = paragraph.attachment {
                    Section {
                        NavigationLink {
                            navigationDestination(destination)
                        } label: {
                            Label(
                                navText.resolve(for: lang.language),
                                systemImage: navIcon.ios,
                            )
                        }
                    }
                }
            }
        }
        .navigationTitle(faq.title.resolve(for: lang.language))
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
                    .clipShape(RoundedRectangle(cornerRadius: AppStyle.smallCornerRadius))
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

    @ViewBuilder
    private func navigationDestination(_ destination: String) -> some View {
        switch destination {
        case "setup_guide":
            SetupGuideView(viewModel: viewModel)
        case "about_developer":
            AboutDeveloperView()
        default:
            EmptyView()
        }
    }
}
