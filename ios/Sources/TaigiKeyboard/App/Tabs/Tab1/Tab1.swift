import SwiftUI
import KeyboardKit

/// 頭頁 Tab
///
/// 顯示啟用方法、新功能、已知問題、網站紹介、FAQ。
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
                Section(languageManager.text(Tab1Texts.setupKeyboard)) {
                    NavigationLink {
                        SetupGuideView(viewModel: viewModel)
                    } label: {
                        Label(languageManager.text(Tab1Texts.setupGuide), systemImage: "keyboard.badge.ellipsis")
                    }
                }

                // 新功能資訊
                Section(languageManager.text(Tab1Texts.newFeatures)) {
                    ForEach(FeatureType.allCases, id: \.self) { feature in
                        NavigationLink {
                            FeatureDetailView(feature: feature)
                        } label: {
                            Label(languageManager.text(feature.title), systemImage: feature.icon)
                        }
                    }
                }

                // 處理中的問題
                Section(languageManager.text(Tab1Texts.knownIssues)) {
                    ForEach(IssueType.allCases, id: \.self) { issue in
                        NavigationLink {
                            IssueDetailView(issue: issue)
                        } label: {
                            HStack(spacing: 12) {
                                Text("\(issue.number)")
                                    .font(.system(size: 12, weight: .medium))
                                    .foregroundColor(.white)
                                    .frame(width: 20, height: 20)
                                    .background(Color.accentColor)
                                    .clipShape(Circle())

                                Text(languageManager.text(issue.title))
                            }
                        }
                    }
                }

                // 預計新功能
                Section(languageManager.text(Tab1Texts.upcomingFeatures)) {
                    ForEach(UpcomingType.allCases, id: \.self) { upcoming in
                        NavigationLink {
                            UpcomingDetailView(upcoming: upcoming)
                        } label: {
                            HStack(spacing: 12) {
                                Text("\(upcoming.number)")
                                    .font(.system(size: 12, weight: .medium))
                                    .foregroundColor(.white)
                                    .frame(width: 20, height: 20)
                                    .background(Color.accentColor)
                                    .clipShape(Circle())

                                Text(languageManager.text(upcoming.title))
                            }
                        }
                    }
                }

                // 網站紹介與聯繫
                Section {
                    // 外部連結
                    Link(destination: URL(string: "https://www.taigikeyboard.tw/")!) {
                        Label(languageManager.text(Tab1Texts.userGuide), systemImage: "globe")
                    }

                    Link(destination: URL(string: "https://taigikeyboard.tw/privacypolicy")!) {
                        Label(languageManager.text(Tab1Texts.privacyPolicy), systemImage: "hand.raised.fill")
                    }

                    Link(destination: URL(string: "https://apps.apple.com/app/id6751871806?action=write-review")!) {
                        Label(languageManager.text(Tab1Texts.rateUs), systemImage: "star.fill")
                    }

                    // 內部導覽
                    NavigationLink {
                        CopyrightView()
                    } label: {
                        Label(languageManager.text(Tab1Texts.copyrightNotice), systemImage: "doc.text.fill")
                    }

                    NavigationLink {
                        FeedbackDetailView()
                    } label: {
                        Label(languageManager.text(Tab1Texts.contactUs), systemImage: "envelope.fill")
                    }

                    NavigationLink {
                        VersionHistoryDetailView()
                    } label: {
                        Label(languageManager.text(Tab1Texts.versionHistory), systemImage: "clock.arrow.circlepath")
                    }

                    // 版本資訊
                    HStack {
                        Label(languageManager.text(Tab1Texts.version), systemImage: "info.circle.fill")
                        Spacer()
                        Text(appVersion)
                            .foregroundColor(.secondary)
                    }
                }

                // 常見問題
                Section(languageManager.text(Tab1Texts.faq)) {
                    ForEach(FAQType.allCases, id: \.self) { faq in
                        NavigationLink {
                            FAQDetailView(faq: faq, viewModel: viewModel)
                        } label: {
                            Label(languageManager.text(faq.question), systemImage: faq.icon)
                        }
                    }
                }
            }
            .navigationTitle(languageManager.text(Tab1Texts.appHeaderTitle))
            .navigationBarTitleDisplayMode(.large)
        }
    }
}
