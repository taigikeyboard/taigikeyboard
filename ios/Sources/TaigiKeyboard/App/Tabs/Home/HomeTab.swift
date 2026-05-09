// 中文: Home tab — setup guide / 打字教學 / 功能設定 / 外部連結 / FAQ 全部走 JSON 驅動。

import KeyboardKit
import SwiftUI

/// Home tab.
///
/// Setup guide, typing guides, feature settings, links, and FAQ.
// 中文: 首頁 tab。內容區段:setup guide → 前 6 筆 features 當打字教學 → 其餘當功能設定
// 中文: → 外部連結與 in-app 導覽 → 版本資訊 → FAQ。features / faqs 由 FeatureContentLoader 提供。
struct HomeTab: View {
    @ObservedObject var viewModel: SetupGuideViewModel

    private var appVersion: String {
        Bundle.main.object(forInfoDictionaryKey: "CFBundleShortVersionString") as? String ?? "1.0"
    }

    var body: some View {
        NavigationStack {
            Form {
                // Setup guide
                Section {
                    NavigationLink {
                        SetupGuideView(viewModel: viewModel)
                    } label: {
                        Label(HomeTexts.setupGuide, systemImage: "keyboard.badge.ellipsis")
                    }
                } header: {
                    Text(HomeTexts.setupKeyboard)
                        .font(AppStyle.sectionHeaderFont)
                }

                // Typing guide
                Section {
                    ForEach(Array(FeatureContentLoader.features.prefix(6))) { feature in
                        NavigationLink {
                            FeatureDetailView(feature: feature)
                        } label: {
                            Label {
                                Text(feature.title)
                            } icon: {
                                Image(latinSystemName: feature.icon.ios)
                                    .foregroundStyle(AppStyle.warningOrange)
                            }
                        }
                    }
                } header: {
                    Text(HomeTexts.typingGuide)
                        .font(AppStyle.sectionHeaderFont)
                }

                // Feature settings
                Section {
                    ForEach(Array(FeatureContentLoader.features.dropFirst(6))) { feature in
                        NavigationLink {
                            FeatureDetailView(feature: feature)
                        } label: {
                            Label {
                                Text(feature.title)
                            } icon: {
                                Image(latinSystemName: feature.icon.ios)
                                    .foregroundStyle(AppStyle.accentBlue)
                            }
                        }
                    }
                } header: {
                    Text(HomeTexts.newFeatures)
                        .font(AppStyle.sectionHeaderFont)
                }

                // Links and contact
                Section {
                    // External links
                    Link(destination: URL(string: "https://www.taigikeyboard.tw/")!) {
                        Label(HomeTexts.userGuide, systemImage: "arrow.up.right.square")
                    }

                    Link(destination: URL(string: "https://taigikeyboard.tw/privacypolicy")!) {
                        Label(HomeTexts.privacyPolicy, systemImage: "arrow.up.right.square")
                    }

                    Link(destination: URL(string: "https://apps.apple.com/app/id6751871806?action=write-review")!) {
                        Label(HomeTexts.rateUs, systemImage: "arrow.up.right.square")
                    }

                    // In-app navigation
                    NavigationLink {
                        CopyrightView()
                    } label: {
                        Label(HomeTexts.copyrightNotice, systemImage: "doc.text")
                    }

                    NavigationLink {
                        AboutDeveloperView()
                    } label: {
                        Label(HomeTexts.aboutDeveloper, systemImage: "info.circle")
                    }

                    NavigationLink {
                        VersionHistoryDetailView()
                    } label: {
                        Label(HomeTexts.versionHistory, systemImage: "clock.arrow.circlepath")
                    }

                    // Version info
                    HStack {
                        Label(HomeTexts.version, systemImage: "info.circle")
                        Spacer()
                        Text(appVersion)
                            .foregroundColor(.secondary)
                    }
                }

                // FAQ
                Section {
                    ForEach(FeatureContentLoader.faqs) { faq in
                        NavigationLink {
                            FAQDetailView(faq: faq, viewModel: viewModel)
                        } label: {
                            Label {
                                Text(faq.title)
                            } icon: {
                                Image(latinSystemName: faq.icon.ios)
                            }
                        }
                    }
                } header: {
                    Text(HomeTexts.faq)
                        .font(AppStyle.sectionHeaderFont)
                }
            }
            .navigationTitle(HomeTexts.appHeaderTitle)
            .navigationBarTitleDisplayMode(.large)
        }
    }
}
