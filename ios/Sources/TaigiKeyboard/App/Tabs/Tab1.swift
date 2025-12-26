import SwiftUI
import KeyboardKit

/// Tab1: 頭頁
/// 內容：啟用方法、新功能、已知 Bug、網站紹介、FAQ
struct Tab1: View {
    @ObservedObject var viewModel: SetupGuideViewModel
    @StateObject private var languageManager = LanguageManager.shared

    var body: some View {
        NavigationStack {
            VStack(spacing: 0) {
                // Header Title: 固定在頂部
                LocalizedTextView(Tab1Texts.appHeaderTitle)
                    .themeFontTitle()
                    .fontWeight(.bold)
                    .foregroundColor(Color.Theme.accent)
                    .frame(maxWidth: .infinity, alignment: .center)
                    .padding(.vertical, 16)
                    .background(Color.Theme.surfacePrimary)

                ScrollView {
                    VStack(spacing: 32) {
                        // 啟用鍵盤區塊
                        setupKeyboardSection

                        // 新功能資訊
                        newFeaturesSection

                        // 處理中的問題
                        knownIssuesSection

                        // 預計新功能
                        upcomingFeaturesSection

                        // 網站紹介與聯繫
                        resourcesSection

                        // 常見問題
                        faqSection
                    }
                    .padding(.horizontal, 20)
                    .padding(.top, 8)
                    .padding(.bottom, 40)
                }
            }
            .background(Color.Theme.surfacePrimary)
            .navigationBarHidden(true)
        }
    }

    // MARK: - 啟用鍵盤區塊

    @ViewBuilder
    private var setupKeyboardSection: some View {
        VStack(alignment: .leading, spacing: 8) {
            SectionHeader(title: Tab1Texts.setupKeyboard)

            VStack(spacing: 0) {
                NavigationLink(destination: SetupGuideView(viewModel: viewModel)) {
                    SetupGuideListRow()
                }
                .buttonStyle(.plain)
            }
            .themedCard()
        }
    }

    // MARK: - 新功能資訊

    @ViewBuilder
    private var newFeaturesSection: some View {
        VStack(alignment: .leading, spacing: 8) {
            SectionHeader(title: Tab1Texts.newFeatures)

            VStack(spacing: 0) {
                ForEach(FeatureType.allCases, id: \.self) { feature in
                    NavigationLink(destination: FeatureDetailView(feature: feature)) {
                        FeatureListRow(
                            feature: feature,
                            isLast: feature == FeatureType.allCases.last
                        )
                    }
                    .buttonStyle(.plain)
                }
            }
            .themedCard()
        }
    }

    // MARK: - 處理中的問題

    @ViewBuilder
    private var knownIssuesSection: some View {
        VStack(alignment: .leading, spacing: 8) {
            SectionHeader(title: Tab1Texts.knownIssues)

            VStack(spacing: 0) {
                ForEach(IssueType.allCases, id: \.self) { issue in
                    NavigationLink(destination: IssueDetailView(issue: issue)) {
                        IssueListRow(
                            issue: issue,
                            isLast: issue == IssueType.allCases.last
                        )
                    }
                    .buttonStyle(.plain)
                }
            }
            .themedCard()
        }
    }

    // MARK: - 預計新功能

    @ViewBuilder
    private var upcomingFeaturesSection: some View {
        VStack(alignment: .leading, spacing: 8) {
            SectionHeader(title: Tab1Texts.upcomingFeatures)

            VStack(spacing: 0) {
                ForEach(UpcomingType.allCases, id: \.self) { upcoming in
                    NavigationLink(destination: UpcomingDetailView(upcoming: upcoming)) {
                        UpcomingListRow(
                            upcoming: upcoming,
                            isLast: upcoming == UpcomingType.allCases.last
                        )
                    }
                    .buttonStyle(.plain)
                }
            }
            .themedCard()
        }
    }

    // MARK: - 網站紹介與聯繫

    private var appVersion: String {
        Bundle.main.object(forInfoDictionaryKey: "CFBundleShortVersionString") as? String ?? "1.0"
    }

    @ViewBuilder
    private var resourcesSection: some View {
        VStack(alignment: .leading, spacing: 8) {
            VStack(spacing: 0) {
                ResourceLinkRow(
                    title: Tab1Texts.userGuide,
                    icon: "globe",
                    url: "https://www.taigikeyboard.tw/",
                    isLast: false
                )

                ResourceLinkRow(
                    title: Tab1Texts.privacyPolicy,
                    icon: "hand.raised.fill",
                    url: "https://taigikeyboard.tw/privacypolicy",
                    isLast: false
                )

                ResourceLinkRow(
                    title: Tab1Texts.rateUs,
                    icon: "star.fill",
                    url: "https://apps.apple.com/app/id6751871806?action=write-review",
                    isLast: false
                )

                // 版權聲明
                NavigationLink(destination: CopyrightView()) {
                    ResourceNavigationRow(
                        title: Tab1Texts.copyrightNotice,
                        icon: "doc.text.fill",
                        isLast: false
                    )
                }
                .buttonStyle(.plain)

                // 問題回報導覽到子頁面
                NavigationLink(destination: FeedbackDetailView()) {
                    ResourceNavigationRow(
                        title: Tab1Texts.contactUs,
                        icon: "envelope.fill",
                        isLast: false
                    )
                }
                .buttonStyle(.plain)

                // 版本紀錄
                NavigationLink(destination: VersionHistoryDetailView()) {
                    ResourceNavigationRow(
                        title: Tab1Texts.versionHistory,
                        icon: "clock.arrow.circlepath",
                        isLast: false
                    )
                }
                .buttonStyle(.plain)

                // 版本資訊
                VersionInfoRow(version: appVersion, isLast: true)
            }
            .themedCard()
        }
    }

    // MARK: - 常見問題

    @ViewBuilder
    private var faqSection: some View {
        VStack(alignment: .leading, spacing: 8) {
            SectionHeader(title: Tab1Texts.faq)

            VStack(spacing: 0) {
                ForEach(FAQType.allCases, id: \.self) { faq in
                    NavigationLink(destination: FAQDetailView(faq: faq, viewModel: viewModel)) {
                        FAQListRow(
                            faq: faq,
                            isLast: faq == FAQType.allCases.last
                        )
                    }
                    .buttonStyle(.plain)
                }
            }
            .themedCard()
        }
    }
}

// MARK: - 功能類型定義

enum FeatureType: Int, CaseIterable {
    case nextWord = 0
    case variant = 1
    case customFont = 2

    var title: LocalizedText {
        switch self {
        case .nextWord: return Tab1Texts.featureNextWord
        case .variant: return Tab1Texts.featureVariant
        case .customFont: return Tab1Texts.featureCustomFont
        }
    }

    var icon: String {
        switch self {
        case .nextWord: return "lightbulb"
        case .variant: return "arrow.left.arrow.right"
        case .customFont: return "book.fill"
        }
    }


    var detailParagraphs: [LocalizedText] {
        switch self {
        case .nextWord: return Tab1Texts.featureNextWordParagraphs
        case .variant: return Tab1Texts.featureVariantParagraphs
        case .customFont: return Tab1Texts.featureCustomFontParagraphs
        }
    }
}

// MARK: - 處理中問題類型定義

enum IssueType: Int, CaseIterable {
    case compatibility = 0
    case candidateDelay = 1
    case keyboardHeight = 2
    case darkMode = 3
    case iPadLayout = 4

    var number: Int { rawValue + 1 }

    var title: LocalizedText {
        switch self {
        case .compatibility: return Tab1Texts.issue1
        case .candidateDelay: return Tab1Texts.issue2
        case .keyboardHeight: return Tab1Texts.issue3
        case .darkMode: return Tab1Texts.issue4
        case .iPadLayout: return Tab1Texts.issue5
        }
    }

    var detailParagraphs: [LocalizedText] {
        switch self {
        case .compatibility: return Tab1Texts.issue1Paragraphs
        case .candidateDelay: return Tab1Texts.issue2Paragraphs
        case .keyboardHeight: return Tab1Texts.issue3Paragraphs
        case .darkMode: return Tab1Texts.issue4Paragraphs
        case .iPadLayout: return Tab1Texts.issue5Paragraphs
        }
    }
}

// MARK: - 預計新功能類型定義

enum UpcomingType: Int, CaseIterable {
    case voiceInput = 0
    case dictionaryExport = 1
    case themeCustomization = 2

    var number: Int { rawValue + 1 }

    var title: LocalizedText {
        switch self {
        case .voiceInput: return Tab1Texts.upcoming1
        case .dictionaryExport: return Tab1Texts.upcoming2
        case .themeCustomization: return Tab1Texts.upcoming3
        }
    }

    var detailParagraphs: [LocalizedText] {
        switch self {
        case .voiceInput: return Tab1Texts.upcoming1Paragraphs
        case .dictionaryExport: return Tab1Texts.upcoming2Paragraphs
        case .themeCustomization: return Tab1Texts.upcoming3Paragraphs
        }
    }
}

// MARK: - FAQ 類型定義

enum FAQType: Int, CaseIterable {
    case installIssue = 0
    case feedback = 1
    case toneHandling = 2

    var question: LocalizedText {
        switch self {
        case .installIssue: return Tab1Texts.faq1Question
        case .feedback: return Tab1Texts.faq2Question
        case .toneHandling: return Tab1Texts.faq3Question
        }
    }

    var icon: String {
        switch self {
        case .installIssue: return "keyboard.badge.ellipsis"
        case .feedback: return "envelope"
        case .toneHandling: return "textformat.123"
        }
    }

    var answerParagraphs: [LocalizedText] {
        switch self {
        case .installIssue: return Tab1Texts.faq1Paragraphs
        case .feedback: return Tab1Texts.faq2Paragraphs
        case .toneHandling: return Tab1Texts.faq3Paragraphs
        }
    }
}

// MARK: - 輔助組件

private struct SectionHeader: View {
    let title: LocalizedText

    var body: some View {
        LocalizedTextView(title)
            .themeFontBody()
            .foregroundColor(Color.Theme.textSecondary)
    }
}

private struct FeatureListRow: View {
    let feature: FeatureType
    let isLast: Bool
    @State private var isPressed = false

    var body: some View {
        VStack(spacing: 0) {
            HStack(spacing: 12) {
                Image(systemName: feature.icon)
                    .foregroundColor(Color.Theme.accent)
                    .font(.system(size: 16))
                    .frame(width: 24)

                LocalizedTextView(feature.title)
                    .themeFontBody()
                    .foregroundColor(Color.Theme.textPrimary)

                Spacer()

                Image(systemName: "chevron.right")
                    .foregroundColor(Color.Theme.textSecondary)
                    .font(.system(size: 12))
            }
            .padding(.horizontal, 16)
            .padding(.vertical, 14)
            .background(isPressed ? Color.Theme.surfaceSecondary : Color.clear)

            if !isLast {
                Divider()
                    .padding(.horizontal, 16)
            }
        }
        .contentShape(Rectangle())
        .pressableCardGesture(isPressed: isPressed) { pressed in
            isPressed = pressed
        }
    }
}

// MARK: - 功能詳細說明頁面

struct FeatureDetailView: View {
    let feature: FeatureType
    @StateObject private var languageManager = LanguageManager.shared

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 16) {
                ForEach(feature.detailParagraphs.indices, id: \.self) { index in
                    // 連紲建議詞第 1 段：文字 + 圖片在同一卡片
                    if feature == .nextWord && index == 0 {
                        VStack(alignment: .leading, spacing: 12) {
                            LocalizedTextView(feature.detailParagraphs[index])
                                .themeFontBody()
                                .foregroundColor(Color.Theme.textPrimary)
                                .lineSpacing(6)
                                .frame(maxWidth: .infinity, alignment: .leading)

                            ImageSlideshowView(
                                imageNames: ["nextword_1", "nextword_2", "nextword_3"],
                                interval: 1.5
                            )
                            .frame(maxWidth: .infinity)
                        }
                        .padding(16)
                        .background(Color.Theme.surfaceSecondary)
                        .clipShape(RoundedRectangle(cornerRadius: 12))
                    } else {
                        LocalizedTextView(feature.detailParagraphs[index])
                            .themeFontBody()
                            .foregroundColor(Color.Theme.textPrimary)
                            .lineSpacing(6)
                            .frame(maxWidth: .infinity, alignment: .leading)
                            .padding(16)
                            .background(Color.Theme.surfaceSecondary)
                            .clipShape(RoundedRectangle(cornerRadius: 12))
                    }

                    // 異用字、詞庫管理：在第 1 段後插入導覽到詞庫管理的項目
                    if (feature == .variant || feature == .customFont) && index == 0 {
                        NavigationLink(destination: DictionarySettingsView()) {
                            HStack(spacing: 12) {
                                Image(systemName: "book.fill")
                                    .foregroundColor(Color.Theme.accent)
                                    .font(.system(size: 16))
                                    .frame(width: 24)

                                LocalizedTextView(Tab1Texts.goToDictionarySettings)
                                    .themeFontBody()
                                    .foregroundColor(Color.Theme.textPrimary)

                                Spacer()

                                Image(systemName: "chevron.right")
                                    .foregroundColor(Color.Theme.textSecondary)
                                    .font(.system(size: 12))
                            }
                            .padding(.horizontal, 16)
                            .padding(.vertical, 14)
                            .background(Color.Theme.surfaceSecondary)
                            .clipShape(RoundedRectangle(cornerRadius: 12))
                        }
                        .buttonStyle(.plain)
                    }
                }
            }
            .padding(.horizontal, 16)
            .padding(.vertical, 20)
        }
        .background(Color.Theme.surfacePrimary)
        .navigationTitle(languageManager.text(feature.title))
        .navigationBarTitleDisplayMode(.large)
    }
}

private struct IssueListRow: View {
    let issue: IssueType
    let isLast: Bool
    @State private var isPressed = false

    var body: some View {
        VStack(spacing: 0) {
            HStack(spacing: 12) {
                Text("\(issue.number)")
                    .font(.system(size: 14, weight: .medium))
                    .foregroundColor(.white)
                    .frame(width: 22, height: 22)
                    .background(Color.Theme.accent)
                    .clipShape(Circle())

                LocalizedTextView(issue.title)
                    .themeFontBody()
                    .foregroundColor(Color.Theme.textPrimary)

                Spacer()

                Image(systemName: "chevron.right")
                    .foregroundColor(Color.Theme.textSecondary)
                    .font(.system(size: 12))
            }
            .padding(.horizontal, 16)
            .padding(.vertical, 14)
            .background(isPressed ? Color.Theme.surfaceSecondary : Color.clear)

            if !isLast {
                Divider()
                    .padding(.horizontal, 16)
            }
        }
        .contentShape(Rectangle())
        .pressableCardGesture(isPressed: isPressed) { pressed in
            isPressed = pressed
        }
    }
}

// MARK: - 處理中問題詳細頁面

struct IssueDetailView: View {
    let issue: IssueType
    @StateObject private var languageManager = LanguageManager.shared

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 16) {
                ForEach(issue.detailParagraphs.indices, id: \.self) { index in
                    LocalizedTextView(issue.detailParagraphs[index])
                        .themeFontBody()
                        .foregroundColor(Color.Theme.textPrimary)
                        .lineSpacing(6)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .padding(16)
                        .background(Color.Theme.surfaceSecondary)
                        .clipShape(RoundedRectangle(cornerRadius: 12))
                }
            }
            .padding(.horizontal, 16)
            .padding(.vertical, 20)
        }
        .background(Color.Theme.surfacePrimary)
        .navigationTitle(languageManager.text(issue.title))
        .navigationBarTitleDisplayMode(.large)
    }
}

private struct UpcomingListRow: View {
    let upcoming: UpcomingType
    let isLast: Bool
    @State private var isPressed = false

    var body: some View {
        VStack(spacing: 0) {
            HStack(spacing: 12) {
                Text("\(upcoming.number)")
                    .font(.system(size: 14, weight: .medium))
                    .foregroundColor(.white)
                    .frame(width: 22, height: 22)
                    .background(Color.Theme.accent)
                    .clipShape(Circle())

                LocalizedTextView(upcoming.title)
                    .themeFontBody()
                    .foregroundColor(Color.Theme.textPrimary)

                Spacer()

                Image(systemName: "chevron.right")
                    .foregroundColor(Color.Theme.textSecondary)
                    .font(.system(size: 12))
            }
            .padding(.horizontal, 16)
            .padding(.vertical, 14)
            .background(isPressed ? Color.Theme.surfaceSecondary : Color.clear)

            if !isLast {
                Divider()
                    .padding(.horizontal, 16)
            }
        }
        .contentShape(Rectangle())
        .pressableCardGesture(isPressed: isPressed) { pressed in
            isPressed = pressed
        }
    }
}

// MARK: - 預計新功能詳細頁面

struct UpcomingDetailView: View {
    let upcoming: UpcomingType
    @StateObject private var languageManager = LanguageManager.shared

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 16) {
                ForEach(upcoming.detailParagraphs.indices, id: \.self) { index in
                    LocalizedTextView(upcoming.detailParagraphs[index])
                        .themeFontBody()
                        .foregroundColor(Color.Theme.textPrimary)
                        .lineSpacing(6)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .padding(16)
                        .background(Color.Theme.surfaceSecondary)
                        .clipShape(RoundedRectangle(cornerRadius: 12))
                }
            }
            .padding(.horizontal, 16)
            .padding(.vertical, 20)
        }
        .background(Color.Theme.surfacePrimary)
        .navigationTitle(languageManager.text(upcoming.title))
        .navigationBarTitleDisplayMode(.large)
    }
}

private struct ResourceLinkRow: View {
    let title: LocalizedText
    let icon: String
    let url: String
    let isLast: Bool
    @State private var isPressed = false

    var body: some View {
        Button {
            if let linkURL = URL(string: url) {
                UIApplication.shared.open(linkURL)
            }
        } label: {
            HStack(spacing: 12) {
                Image(systemName: icon)
                    .foregroundColor(Color.Theme.accent)
                    .font(.system(size: 16))
                    .frame(width: 24)

                LocalizedTextView(title)
                    .themeFontBody()
                    .foregroundColor(Color.Theme.textPrimary)

                Spacer()

                // 外部連結使用 arrow.up.right
                Image(systemName: "arrow.up.right")
                    .foregroundColor(Color.Theme.textSecondary)
                    .font(.system(size: 12))
            }
            .padding(.horizontal, 16)
            .padding(.vertical, 14)
            .background(isPressed ? Color.Theme.surfaceSecondary : Color.clear)
        }
        .buttonStyle(.plain)
        .pressableCardGesture(isPressed: isPressed) { pressed in
            isPressed = pressed
        }

        if !isLast {
            Divider()
                .padding(.horizontal, 16)
        }
    }
}

/// 資源導覽列（用於 NavigationLink 內）
private struct ResourceNavigationRow: View {
    let title: LocalizedText
    let icon: String
    let isLast: Bool
    @State private var isPressed = false

    var body: some View {
        VStack(spacing: 0) {
            HStack(spacing: 12) {
                Image(systemName: icon)
                    .foregroundColor(Color.Theme.accent)
                    .font(.system(size: 16))
                    .frame(width: 24)

                LocalizedTextView(title)
                    .themeFontBody()
                    .foregroundColor(Color.Theme.textPrimary)

                Spacer()

                Image(systemName: "chevron.right")
                    .foregroundColor(Color.Theme.textSecondary)
                    .font(.system(size: 12))
            }
            .padding(.horizontal, 16)
            .padding(.vertical, 14)
            .background(isPressed ? Color.Theme.surfaceSecondary : Color.clear)

            if !isLast {
                Divider()
                    .padding(.horizontal, 16)
            }
        }
        .contentShape(Rectangle())
        .pressableCardGesture(isPressed: isPressed) { pressed in
            isPressed = pressed
        }
    }
}

/// 版本資訊列
private struct VersionInfoRow: View {
    let version: String
    let isLast: Bool

    var body: some View {
        VStack(spacing: 0) {
            HStack(spacing: 12) {
                Image(systemName: "info.circle.fill")
                    .foregroundColor(Color.Theme.accent)
                    .font(.system(size: 16))
                    .frame(width: 24)

                LocalizedTextView(Tab1Texts.version)
                    .themeFontBody()
                    .foregroundColor(Color.Theme.textPrimary)

                Spacer()

                Text(version)
                    .themeFontBody()
                    .foregroundColor(Color.Theme.textSecondary)
            }
            .padding(.horizontal, 16)
            .padding(.vertical, 14)

            if !isLast {
                Divider()
                    .padding(.horizontal, 16)
            }
        }
    }
}

// MARK: - 問題回報詳細頁面

struct FeedbackDetailView: View {
    @StateObject private var languageManager = LanguageManager.shared
    @Environment(\.openURL) private var openURL

    private let googleFormURL = "https://docs.google.com/forms/d/e/1FAIpQLSd7PEppQ9MdAptvoY-PaaXDlbbL9Gq9Y4lFjgU9sLz4ENiPoA/viewform?usp=header"
    private let supportURL = "https://portaly.cc/siansiansu/support"

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 16) {
                // 說明文字
                LocalizedTextView(Tab1Texts.feedbackDescription)
                    .themeFontBody()
                    .foregroundColor(Color.Theme.textPrimary)
                    .lineSpacing(6)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(16)
                    .background(Color.Theme.surfaceSecondary)
                    .clipShape(RoundedRectangle(cornerRadius: 12))

                // Google 表單按鈕
                Button {
                    if let url = URL(string: googleFormURL) {
                        openURL(url)
                    }
                } label: {
                    HStack(spacing: 12) {
                        Image(systemName: "doc.text.fill")
                            .foregroundColor(Color.Theme.accent)
                            .font(.system(size: 16))
                            .frame(width: 24)

                        LocalizedTextView(Tab1Texts.goToGoogleForm)
                            .themeFontBody()
                            .foregroundColor(Color.Theme.textPrimary)

                        Spacer()

                        Image(systemName: "arrow.up.right")
                            .foregroundColor(Color.Theme.textSecondary)
                            .font(.system(size: 12))
                    }
                    .padding(.horizontal, 16)
                    .padding(.vertical, 14)
                    .background(Color.Theme.surfaceSecondary)
                    .clipShape(RoundedRectangle(cornerRadius: 12))
                }
                .buttonStyle(.plain)

                // Email 聯絡
                LocalizedTextView(Tab1Texts.emailContact)
                    .themeFontBody()
                    .foregroundColor(Color.Theme.textPrimary)
                    .lineSpacing(6)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(16)
                    .background(Color.Theme.surfaceSecondary)
                    .clipShape(RoundedRectangle(cornerRadius: 12))

                // 贊助連結
                Button {
                    if let url = URL(string: supportURL) {
                        openURL(url)
                    }
                } label: {
                    HStack(spacing: 12) {
                        Image(systemName: "heart.fill")
                            .foregroundColor(Color.Theme.accent)
                            .font(.system(size: 16))
                            .frame(width: 24)

                        LocalizedTextView(Tab1Texts.supportUs)
                            .themeFontBody()
                            .foregroundColor(Color.Theme.textPrimary)

                        Spacer()

                        Image(systemName: "arrow.up.right")
                            .foregroundColor(Color.Theme.textSecondary)
                            .font(.system(size: 12))
                    }
                    .padding(.horizontal, 16)
                    .padding(.vertical, 14)
                    .background(Color.Theme.surfaceSecondary)
                    .clipShape(RoundedRectangle(cornerRadius: 12))
                }
                .buttonStyle(.plain)
            }
            .padding(.horizontal, 16)
            .padding(.vertical, 20)
        }
        .background(Color.Theme.surfacePrimary)
        .navigationTitle(languageManager.text(Tab1Texts.contactUs))
        .navigationBarTitleDisplayMode(.large)
    }
}

// MARK: - 版本紀錄詳細頁面

struct VersionHistoryDetailView: View {
    @StateObject private var languageManager = LanguageManager.shared

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 20) {
                ForEach(Tab1Texts.versionHistoryEntries.indices, id: \.self) { index in
                    let entry = Tab1Texts.versionHistoryEntries[index]
                    VStack(alignment: .leading, spacing: 12) {
                        // 版本號與日期
                        HStack {
                            Text("v\(entry.version)")
                                .themeFontHeadline()
                                .foregroundColor(Color.Theme.accent)

                            Spacer()

                            Text(entry.date)
                                .themeFontCaption()
                                .foregroundColor(Color.Theme.textSecondary)
                        }

                        // 變更內容
                        VStack(alignment: .leading, spacing: 8) {
                            ForEach(entry.changes.indices, id: \.self) { changeIndex in
                                HStack(alignment: .top, spacing: 8) {
                                    Text("•")
                                        .foregroundColor(Color.Theme.textSecondary)
                                    LocalizedTextView(entry.changes[changeIndex])
                                        .themeFontBody()
                                        .foregroundColor(Color.Theme.textPrimary)
                                }
                            }
                        }
                    }
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(16)
                    .background(Color.Theme.surfaceSecondary)
                    .clipShape(RoundedRectangle(cornerRadius: 12))
                }
            }
            .padding(.horizontal, 16)
            .padding(.vertical, 20)
        }
        .background(Color.Theme.surfacePrimary)
        .navigationTitle(languageManager.text(Tab1Texts.versionHistory))
        .navigationBarTitleDisplayMode(.large)
    }
}

private struct FAQListRow: View {
    let faq: FAQType
    let isLast: Bool
    @State private var isPressed = false

    var body: some View {
        VStack(spacing: 0) {
            HStack(spacing: 12) {
                Image(systemName: faq.icon)
                    .foregroundColor(Color.Theme.accent)
                    .font(.system(size: 16))
                    .frame(width: 24)

                LocalizedTextView(faq.question)
                    .themeFontBody()
                    .foregroundColor(Color.Theme.textPrimary)
                    .multilineTextAlignment(.leading)

                Spacer()

                Image(systemName: "chevron.right")
                    .foregroundColor(Color.Theme.textSecondary)
                    .font(.system(size: 12))
            }
            .padding(.horizontal, 16)
            .padding(.vertical, 14)
            .background(isPressed ? Color.Theme.surfaceSecondary : Color.clear)

            if !isLast {
                Divider()
                    .padding(.horizontal, 16)
            }
        }
        .contentShape(Rectangle())
        .pressableCardGesture(isPressed: isPressed) { pressed in
            isPressed = pressed
        }
    }
}

// MARK: - FAQ 詳細說明頁面

struct FAQDetailView: View {
    let faq: FAQType
    @ObservedObject var viewModel: SetupGuideViewModel
    @StateObject private var languageManager = LanguageManager.shared

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 16) {
                // 根據 FAQ 類型決定內容
                if faq == .installIssue {
                    // 第一段
                    paragraphView(faq.answerParagraphs[0])

                    // 導覽到「啟用方法」
                    navigationLinkRow(
                        icon: "keyboard.badge.ellipsis",
                        title: Tab1Texts.goToSetupGuide,
                        destination: AnyView(SetupGuideView(viewModel: viewModel))
                    )

                    // 剩餘段落
                    ForEach(1..<faq.answerParagraphs.count, id: \.self) { index in
                        paragraphView(faq.answerParagraphs[index])
                    }
                } else if faq == .feedback {
                    // 第一段
                    paragraphView(faq.answerParagraphs[0])

                    // 導覽到「問題回報」
                    navigationLinkRow(
                        icon: "envelope.fill",
                        title: Tab1Texts.goToFeedback,
                        destination: AnyView(FeedbackDetailView())
                    )

                    // 剩餘段落
                    ForEach(1..<faq.answerParagraphs.count, id: \.self) { index in
                        paragraphView(faq.answerParagraphs[index])
                    }
                } else if faq == .toneHandling {
                    // 第一段
                    paragraphView(faq.answerParagraphs[0])

                    // 螢幕截圖（橫幅型，填滿寬度）
                    Image("faq_tone_handling")
                        .resizable()
                        .aspectRatio(contentMode: .fit)
                        .frame(maxWidth: .infinity)
                        .clipShape(RoundedRectangle(cornerRadius: 8))
                        .overlay(
                            RoundedRectangle(cornerRadius: 8)
                                .stroke(Color.Theme.cardStroke, lineWidth: 1)
                        )

                    // 第二段
                    paragraphView(faq.answerParagraphs[1])
                } else {
                    // 其他 FAQ 正常顯示所有段落
                    ForEach(faq.answerParagraphs.indices, id: \.self) { index in
                        paragraphView(faq.answerParagraphs[index])
                    }
                }
            }
            .padding(.horizontal, 16)
            .padding(.vertical, 20)
        }
        .background(Color.Theme.surfacePrimary)
        .navigationTitle(languageManager.text(faq.question))
        .navigationBarTitleDisplayMode(.large)
    }

    @ViewBuilder
    private func paragraphView(_ text: LocalizedText) -> some View {
        LocalizedTextView(text)
            .themeFontBody()
            .foregroundColor(Color.Theme.textPrimary)
            .lineSpacing(6)
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(16)
            .background(Color.Theme.surfaceSecondary)
            .clipShape(RoundedRectangle(cornerRadius: 12))
    }

    @ViewBuilder
    private func navigationLinkRow(icon: String, title: LocalizedText, destination: AnyView) -> some View {
        NavigationLink(destination: destination) {
            HStack(spacing: 12) {
                Image(systemName: icon)
                    .foregroundColor(Color.Theme.accent)
                    .font(.system(size: 16))
                    .frame(width: 24)

                Text(languageManager.text(title))
                    .themeFontBody()
                    .foregroundColor(Color.Theme.textPrimary)

                Spacer()

                Image(systemName: "chevron.right")
                    .font(.system(size: 12))
                    .foregroundColor(Color.Theme.textSecondary)
            }
            .padding(16)
            .background(Color.Theme.surfaceSecondary)
            .clipShape(RoundedRectangle(cornerRadius: 12))
        }
        .buttonStyle(.plain)
    }
}

// MARK: - 啟用方法列表項目

private struct SetupGuideListRow: View {
    @State private var isPressed = false

    var body: some View {
        VStack(spacing: 0) {
            HStack(spacing: 12) {
                Image(systemName: "keyboard.badge.ellipsis")
                    .foregroundColor(Color.Theme.accent)
                    .font(.system(size: 16))
                    .frame(width: 24)

                LocalizedTextView(Tab1Texts.setupGuide)
                    .themeFontBody()
                    .foregroundColor(Color.Theme.textPrimary)

                Spacer()

                Image(systemName: "chevron.right")
                    .foregroundColor(Color.Theme.textSecondary)
                    .font(.system(size: 12))
            }
            .padding(.horizontal, 16)
            .padding(.vertical, 14)
            .background(isPressed ? Color.Theme.surfaceSecondary : Color.clear)
        }
        .contentShape(Rectangle())
        .pressableCardGesture(isPressed: isPressed) { pressed in
            isPressed = pressed
        }
    }
}

// MARK: - 啟用方法詳細頁面（共用於全螢幕 Setup Guide 和 Tab1）

struct SetupGuideView: View {
    @ObservedObject var viewModel: SetupGuideViewModel
    @StateObject private var languageManager = LanguageManager.shared
    @Environment(\.openURL) private var openURL

    /// 全螢幕模式（與 Android SetupGuideActivity isFullScreen 對應）
    var isFullScreen: Bool = false

    /// 全螢幕模式關閉 callback（僅在 isFullScreen = true 時使用）
    var onComplete: (() -> Void)? = nil

    var body: some View {
        ScrollView {
            VStack(spacing: 24) {
                // 全螢幕模式標題
                if isFullScreen {
                    LocalizedTextView(Tab1Texts.setupGuide)
                        .themeFontTitle()
                        .fontWeight(.bold)
                        .foregroundColor(Color.Theme.accent)
                        .frame(maxWidth: .infinity, alignment: .center)
                        .padding(.top, 40)
                }

                // 說明文字
                LocalizedTextView(Tab1Texts.setupGuideDescription)
                    .themeFontBody()
                    .foregroundColor(Color.Theme.textPrimary)
                    .multilineTextAlignment(.leading)
                    .lineSpacing(4)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(.horizontal, 20)
                    .padding(.top, isFullScreen ? 0 : 8)

                // 步驟說明（截圖式）
                VStack(spacing: 16) {
                    SetupGuideStepCard(
                        stepNumber: 1,
                        title: Tab1Texts.setupGuideStep1Settings,
                        screenshotName: "setup_step1"
                    )

                    SetupGuideStepCard(
                        stepNumber: 2,
                        title: Tab1Texts.setupGuideStep2AddKeyboard,
                        screenshotName: "setup_step2"
                    )

                    // 完成說明（無編號）
                    LocalizedTextView(Tab1Texts.setupGuideCompletedMessage)
                        .themeFontBody()
                        .foregroundColor(Color.Theme.textPrimary)
                }
                .padding(.horizontal, 20)

                // 前往設定按鈕
                Button {
                    if let url = URL(string: UIApplication.openSettingsURLString) {
                        openURL(url)
                    }
                } label: {
                    HStack {
                        Image(systemName: "gearshape.fill")
                        LocalizedTextView(Tab1Texts.setupGuideGoToSettings)
                    }
                    .themeFontBody()
                    .foregroundColor(.white)
                    .frame(maxWidth: .infinity)
                    .padding(.vertical, 16)
                    .background(Color.Theme.accent)
                    .clipShape(RoundedRectangle(cornerRadius: 12))
                }
                .padding(.horizontal, 24)

                // 警告訊息
                HStack(alignment: .top, spacing: 8) {
                    Image(systemName: "exclamationmark.triangle.fill")
                        .foregroundColor(Color.Theme.accent)
                        .font(.system(size: 17))

                    LocalizedTextView(Tab1Texts.setupInfoMessage)
                        .themeFontBody()
                        .foregroundColor(Color.Theme.textPrimary)
                        .lineSpacing(4)
                }
                .padding(.horizontal, 24)

                // 手機品牌警告訊息
                HStack(alignment: .top, spacing: 8) {
                    Image(systemName: "exclamationmark.triangle.fill")
                        .foregroundColor(Color.Theme.accent)
                        .font(.system(size: 17))

                    LocalizedTextView(Tab1Texts.setupBrandWarning)
                        .themeFontBody()
                        .foregroundColor(Color.Theme.textPrimary)
                        .lineSpacing(4)
                }
                .padding(.horizontal, 24)

                // 關閉按鈕（僅全螢幕模式顯示，底部中央紅色 X）
                if isFullScreen, let onComplete = onComplete {
                    Button(action: onComplete) {
                        HStack(spacing: 8) {
                            Image(systemName: "xmark")
                                .font(.system(size: 14, weight: .bold))
                            LocalizedTextView(Tab1Texts.setupGuideCloseButton)
                                .themeFontBody()
                        }
                        .foregroundColor(.white)
                        .padding(.horizontal, 24)
                        .padding(.vertical, 12)
                        .background(Color.Theme.accentSecondary)
                        .clipShape(RoundedRectangle(cornerRadius: 8))
                    }
                    .padding(.top, 8)
                }
            }
            .padding(.bottom, 40)
        }
        .background(Color.Theme.surfacePrimary)
        .navigationTitle(isFullScreen ? "" : languageManager.text(Tab1Texts.setupGuide))
        .navigationBarTitleDisplayMode(.large)
        .navigationBarHidden(isFullScreen)
        .onReceive(NotificationCenter.default.publisher(for: UIApplication.didBecomeActiveNotification)) { _ in
            Task { @MainActor in
                viewModel.refresh()
            }
        }
    }
}

// MARK: - 步驟卡片（SetupGuideView 專用）

private struct SetupGuideStepCard: View {
    let stepNumber: Int
    let title: LocalizedText
    let screenshotName: String

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            // 步驟標題
            HStack(spacing: 12) {
                Text("\(stepNumber)")
                    .font(.system(size: 14, weight: .bold))
                    .foregroundColor(.white)
                    .frame(width: 24, height: 24)
                    .background(Color.Theme.accent)
                    .clipShape(Circle())

                LocalizedTextView(title)
                    .themeFontBody()
                    .foregroundColor(Color.Theme.textPrimary)

                Spacer()
            }

            // 截圖（自適應高度）
            if let uiImage = UIImage(named: screenshotName) {
                Image(uiImage: uiImage)
                    .resizable()
                    .aspectRatio(contentMode: .fit)
                    .frame(maxWidth: .infinity)
            } else {
                RoundedRectangle(cornerRadius: 8)
                    .fill(Color.Theme.surfaceSecondary)
                    .frame(maxWidth: .infinity)
                    .frame(height: 140)
                    .overlay(
                        VStack(spacing: 8) {
                            Image(systemName: "photo")
                                .font(.system(size: 32))
                                .foregroundColor(Color.Theme.textSecondary)

                            Text("Screenshot")
                                .font(.caption)
                                .foregroundColor(Color.Theme.textSecondary)
                        }
                    )
            }
        }
    }
}


