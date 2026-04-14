import KeyboardKit
import SwiftUI

/// Home tab.
///
/// Setup guide, typing guides, feature settings, links, and FAQ.
struct Tab1: View {
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
                        Label(Tab1Texts.setupGuide, systemImage: "keyboard.badge.ellipsis")
                    }
                } header: {
                    Text(Tab1Texts.setupKeyboard)
                        .font(AppStyle.sectionHeaderFont)
                }

                // Typing guide
                Section {
                    ForEach(Array(FeatureContentLoader.features.prefix(6))) { feature in
                        NavigationLink {
                            FeatureDetailView(feature: feature)
                        } label: {
                            Label {
                                Text(feature.title.asString)
                            } icon: {
                                Image(systemName: feature.icon.ios)
                                    .foregroundStyle(AppStyle.warningOrange)
                            }
                        }
                    }
                } header: {
                    Text(Tab1Texts.typingGuide)
                        .font(AppStyle.sectionHeaderFont)
                }

                // Feature settings
                Section {
                    ForEach(Array(FeatureContentLoader.features.dropFirst(6))) { feature in
                        NavigationLink {
                            FeatureDetailView(feature: feature)
                        } label: {
                            Label {
                                Text(feature.title.asString)
                            } icon: {
                                Image(systemName: feature.icon.ios)
                                    .foregroundStyle(AppStyle.accentBlue)
                            }
                        }
                    }
                } header: {
                    Text(Tab1Texts.newFeatures)
                        .font(AppStyle.sectionHeaderFont)
                }

                // Links and contact
                Section {
                    // External links
                    Link(destination: URL(string: "https://www.taigikeyboard.tw/")!) {
                        Label(Tab1Texts.userGuide, systemImage: "arrow.up.right.square")
                    }

                    Link(destination: URL(string: "https://taigikeyboard.tw/privacypolicy")!) {
                        Label(Tab1Texts.privacyPolicy, systemImage: "arrow.up.right.square")
                    }

                    Link(destination: URL(string: "https://apps.apple.com/app/id6751871806?action=write-review")!) {
                        Label(Tab1Texts.rateUs, systemImage: "arrow.up.right.square")
                    }

                    // In-app navigation
                    NavigationLink {
                        CopyrightView()
                    } label: {
                        Label(Tab1Texts.copyrightNotice, systemImage: "doc.text")
                    }

                    NavigationLink {
                        FeedbackDetailView()
                    } label: {
                        Label(Tab1Texts.contactUs, systemImage: "heart")
                    }

                    NavigationLink {
                        VersionHistoryDetailView()
                    } label: {
                        Label(Tab1Texts.versionHistory, systemImage: "clock.arrow.circlepath")
                    }

                    // Version info
                    HStack {
                        Label(Tab1Texts.version, systemImage: "info.circle")
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
                                Text(faq.title.asString)
                            } icon: {
                                Image(systemName: faq.icon.ios)
                            }
                        }
                    }
                } header: {
                    Text(Tab1Texts.faq)
                        .font(AppStyle.sectionHeaderFont)
                }
            }
            .navigationTitle(Tab1Texts.appHeaderTitle)
            .navigationBarTitleDisplayMode(.large)
        }
    }
}
