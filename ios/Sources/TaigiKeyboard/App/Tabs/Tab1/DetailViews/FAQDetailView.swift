import SwiftUI

/// FAQ 詳細頁面
///
/// 顯示常見問題的詳細說明和操作指引。
struct FAQDetailView: View {
    let faq: FAQType
    @ObservedObject var viewModel: SetupGuideViewModel
    @StateObject private var languageManager = LanguageManager.shared

    var body: some View {
        Form {
            // 根據 FAQ 類型決定內容
            if faq == .installIssue {
                Section {
                    Text(languageManager.text(faq.answerParagraphs[0]))
                        .lineSpacing(6)
                }

                Section {
                    NavigationLink {
                        SetupGuideView(viewModel: viewModel)
                    } label: {
                        Label(languageManager.text(Tab1Texts.goToSetupGuide), systemImage: "keyboard.badge.ellipsis")
                    }
                }

                ForEach(1..<faq.answerParagraphs.count, id: \.self) { index in
                    Section {
                        Text(languageManager.text(faq.answerParagraphs[index]))
                            .lineSpacing(6)
                    }
                }
            } else if faq == .feedback {
                Section {
                    Text(languageManager.text(faq.answerParagraphs[0]))
                        .lineSpacing(6)
                }

                Section {
                    NavigationLink {
                        FeedbackDetailView()
                    } label: {
                        Label(languageManager.text(Tab1Texts.goToFeedback), systemImage: "envelope.fill")
                    }
                }

                ForEach(1..<faq.answerParagraphs.count, id: \.self) { index in
                    Section {
                        Text(languageManager.text(faq.answerParagraphs[index]))
                            .lineSpacing(6)
                    }
                }
            } else if faq == .toneHandling {
                Section {
                    Text(languageManager.text(faq.answerParagraphs[0]))
                        .lineSpacing(6)
                }

                Section {
                    Image("faq_tone_handling")
                        .resizable()
                        .aspectRatio(contentMode: .fit)
                        .frame(maxWidth: .infinity)
                        .clipShape(RoundedRectangle(cornerRadius: 8))
                }

                Section {
                    Text(languageManager.text(faq.answerParagraphs[1]))
                        .lineSpacing(6)
                }
            } else {
                ForEach(faq.answerParagraphs.indices, id: \.self) { index in
                    Section {
                        Text(languageManager.text(faq.answerParagraphs[index]))
                            .lineSpacing(6)
                    }
                }
            }
        }
        .navigationTitle(languageManager.text(faq.question))
        .navigationBarTitleDisplayMode(.large)
    }
}
