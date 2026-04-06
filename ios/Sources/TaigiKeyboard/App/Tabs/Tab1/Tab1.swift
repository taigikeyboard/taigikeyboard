import KeyboardKit
import SwiftUI

/// 頭頁 Tab
///
/// 顯示啟用方法、新功能、網站紹介、FAQ。
struct Tab1: View {
    @ObservedObject var viewModel: SetupGuideViewModel
    @StateObject private var languageManager = LanguageManager.shared

    private var appVersion: String {
        Bundle.main.object(forInfoDictionaryKey: "CFBundleShortVersionString") as? String ?? "1.0"
    }

    var body: some View {
        NavigationStack {
            Form {
                // 啟用鍵盤區塊
                Section {
                    NavigationLink {
                        SetupGuideView(viewModel: viewModel)
                    } label: {
                        Label(languageManager.text(Tab1Texts.setupGuide), systemImage: "keyboard.badge.ellipsis")
                    }
                } header: {
                    Text(languageManager.text(Tab1Texts.setupKeyboard))
                        .font(AppStyle.sectionHeaderFont)
                }

                // 拍字說明
                Section {
                    ForEach(Array(FeatureContentLoader.features.prefix(6))) { feature in
                        NavigationLink {
                            FeatureDetailView(feature: feature)
                        } label: {
                            Label {
                                Text(languageManager.text(feature.title.asLocalizedText))
                            } icon: {
                                Image(systemName: feature.icon.ios)
                                    .foregroundStyle(AppStyle.warningOrange)
                            }
                        }
                    }
                } header: {
                    Text(languageManager.text(Tab1Texts.typingGuide))
                        .font(AppStyle.sectionHeaderFont)
                }

                // 功能設定
                Section {
                    ForEach(Array(FeatureContentLoader.features.dropFirst(6))) { feature in
                        NavigationLink {
                            FeatureDetailView(feature: feature)
                        } label: {
                            Label {
                                Text(languageManager.text(feature.title.asLocalizedText))
                            } icon: {
                                Image(systemName: feature.icon.ios)
                                    .foregroundStyle(AppStyle.accentBlue)
                            }
                        }
                    }
                } header: {
                    Text(languageManager.text(Tab1Texts.newFeatures))
                        .font(AppStyle.sectionHeaderFont)
                }

                // 網站紹介與聯繫
                Section {
                    // 外部連結
                    Link(destination: URL(string: "https://www.taigikeyboard.tw/")!) {
                        Label(languageManager.text(Tab1Texts.userGuide), systemImage: "arrow.up.right.square")
                    }

                    Link(destination: URL(string: "https://taigikeyboard.tw/privacypolicy")!) {
                        Label(languageManager.text(Tab1Texts.privacyPolicy), systemImage: "arrow.up.right.square")
                    }

                    Link(destination: URL(string: "https://apps.apple.com/app/id6751871806?action=write-review")!) {
                        Label(languageManager.text(Tab1Texts.rateUs), systemImage: "arrow.up.right.square")
                    }

                    // 內部導覽
                    NavigationLink {
                        CopyrightView()
                    } label: {
                        Label(languageManager.text(Tab1Texts.copyrightNotice), systemImage: "doc.text")
                    }

                    NavigationLink {
                        FeedbackDetailView()
                    } label: {
                        Label(languageManager.text(Tab1Texts.contactUs), systemImage: "heart")
                    }

                    NavigationLink {
                        VersionHistoryDetailView()
                    } label: {
                        Label(languageManager.text(Tab1Texts.versionHistory), systemImage: "clock.arrow.circlepath")
                    }

                    // 版本資訊
                    HStack {
                        Label(languageManager.text(Tab1Texts.version), systemImage: "info.circle")
                        Spacer()
                        Text(appVersion)
                            .foregroundColor(.secondary)
                    }
                }

                // 常見問題
                Section {
                    ForEach(FeatureContentLoader.faqs) { faq in
                        NavigationLink {
                            FAQDetailView(faq: faq, viewModel: viewModel)
                        } label: {
                            Label {
                                Text(languageManager.text(faq.title.asLocalizedText))
                            } icon: {
                                Image(systemName: faq.icon.ios)
                            }
                        }
                    }
                } header: {
                    Text(languageManager.text(Tab1Texts.faq))
                        .font(AppStyle.sectionHeaderFont)
                }
            }
            .navigationTitle(languageManager.text(Tab1Texts.appHeaderTitle))
            .navigationBarTitleDisplayMode(.large)
        }
    }
}
