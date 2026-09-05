// Home tab — setup guide / 打字教學 / 功能設定 / 外部連結 / FAQ 全部走 JSON 驅動。

import KeyboardKit
import SwiftUI

/// Home tab.
///
/// Setup guide, typing guides, feature settings, links, and FAQ.
// 首頁 tab。內容區段:setup guide → 前 6 筆 features 當打字教學 → 其餘當功能設定
// → 外部連結與 in-app 導覽 → 版本資訊 → FAQ。features / faqs 由 FeatureContentLoader 提供。
struct HomeTab: View {
    @ObservedObject var viewModel: SetupGuideViewModel
    @Environment(DisplayLanguageStore.self) private var lang

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
                        Label(lang.string(.homeSetupGuide), systemImage: "keyboard.badge.ellipsis")
                    }
                } header: {
                    Text(lang.string(.homeSetupKeyboard))
                        .font(AppStyle.sectionHeaderFont)
                }

                // Typing guide
                Section {
                    ForEach(Array(FeatureContentLoader.features.prefix(6))) { feature in
                        NavigationLink {
                            FeatureDetailView(feature: feature)
                        } label: {
                            Label {
                                Text(feature.title.resolve(for: lang.language))
                            } icon: {
                                Image(latinSystemName: feature.icon.ios)
                                    .foregroundStyle(AppStyle.warningOrange)
                            }
                        }
                    }
                } header: {
                    Text(lang.string(.homeTypingGuide))
                        .font(AppStyle.sectionHeaderFont)
                }

                // Feature settings
                Section {
                    ForEach(Array(FeatureContentLoader.features.dropFirst(6))) { feature in
                        NavigationLink {
                            FeatureDetailView(feature: feature)
                        } label: {
                            Label {
                                Text(feature.title.resolve(for: lang.language))
                            } icon: {
                                Image(latinSystemName: feature.icon.ios)
                                    .foregroundStyle(AppStyle.accentBlue)
                            }
                        }
                    }
                } header: {
                    Text(lang.string(.homeNewFeatures))
                        .font(AppStyle.sectionHeaderFont)
                }

                // Links and contact
                Section {
                    // External links
                    Link(destination: URL(string: "https://www.taigikeyboard.tw/")!) {
                        Label(lang.string(.homeUserGuide), systemImage: "arrow.up.right.square")
                    }

                    Link(destination: URL(string: "https://taigikeyboard.tw/privacypolicy")!) {
                        Label(lang.string(.homePrivacyPolicy), systemImage: "arrow.up.right.square")
                    }

                    Link(destination: URL(string: "https://apps.apple.com/app/id6751871806?action=write-review")!) {
                        Label(lang.string(.homeRateUs), systemImage: "arrow.up.right.square")
                    }

                    // In-app navigation
                    NavigationLink {
                        CopyrightView()
                    } label: {
                        Label(lang.string(.homeCopyrightNotice), systemImage: "doc.text")
                    }

                    NavigationLink {
                        AboutDeveloperView()
                    } label: {
                        Label(lang.string(.homeAboutDeveloper), systemImage: "info.circle")
                    }

                    NavigationLink {
                        VersionHistoryDetailView()
                    } label: {
                        Label(lang.string(.homeVersionHistory), systemImage: "clock.arrow.circlepath")
                    }

                    // Version info
                    HStack {
                        Label(lang.string(.homeVersion), systemImage: "info.circle")
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
                                Text(faq.title.resolve(for: lang.language))
                            } icon: {
                                Image(latinSystemName: faq.icon.ios)
                            }
                        }
                    }
                } header: {
                    Text(lang.string(.homeFaq))
                        .font(AppStyle.sectionHeaderFont)
                }
            }
            .navigationTitle(lang.string(.homeAppHeaderTitle))
            .navigationBarTitleDisplayMode(.large)
        }
    }
}
