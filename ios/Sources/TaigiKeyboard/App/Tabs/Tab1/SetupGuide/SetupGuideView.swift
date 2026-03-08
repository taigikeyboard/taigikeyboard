import SwiftUI

/// 設定引導視圖
///
/// 顯示鍵盤啟用步驟，共用於全螢幕引導和頭頁。
struct SetupGuideView: View {
    @ObservedObject var viewModel: SetupGuideViewModel
    @StateObject private var languageManager = LanguageManager.shared
    @Environment(\.openURL) private var openURL

    /// 全螢幕模式（與 Android SetupGuideActivity isFullScreen 對應）
    var isFullScreen: Bool = false

    /// 全螢幕模式關閉 callback（僅在 isFullScreen = true 時使用）
    var onComplete: (() -> Void)? = nil

    var body: some View {
        Form {
            // 全螢幕模式標題
            if isFullScreen {
                Section {
                    Text(languageManager.text(Tab1Texts.setupGuide))
                        .font(KeyboardModels.Fonts.appFont(.largeTitle))
                        .fontWeight(.bold)
                }
            }

            // 說明文字
            Section {
                Text(languageManager.text(Tab1Texts.setupGuideDescription))
                    .lineSpacing(4)
            }

            // 步驟說明
            Section {
                SetupGuideStepRow(
                    stepNumber: 1,
                    title: languageManager.text(Tab1Texts.setupGuideStep1Settings),
                    screenshotName: "setup_step1"
                )

                SetupGuideStepRow(
                    stepNumber: 2,
                    title: languageManager.text(Tab1Texts.setupGuideStep2AddKeyboard),
                    screenshotName: "setup_step2"
                )
            }

            // 完成說明
            Section {
                Text(languageManager.text(Tab1Texts.setupGuideCompletedMessage))
            }

            // 前往設定按鈕
            Section {
                Button {
                    if let url = URL(string: UIApplication.openSettingsURLString) {
                        openURL(url)
                    }
                } label: {
                    Label(languageManager.text(Tab1Texts.setupGuideGoToSettings), systemImage: "gearshape.fill")
                        .frame(maxWidth: .infinity, alignment: .center)
                }
            }

            // 警告訊息
            Section {
                Label {
                    Text(languageManager.text(Tab1Texts.setupInfoMessage))
                        .lineSpacing(4)
                } icon: {
                    Image(systemName: "exclamationmark.triangle.fill")
                        .foregroundColor(.orange)
                }

                Label {
                    Text(languageManager.text(Tab1Texts.setupBrandWarning))
                        .lineSpacing(4)
                } icon: {
                    Image(systemName: "exclamationmark.triangle.fill")
                        .foregroundColor(.orange)
                }
            }

            // 關閉按鈕（僅全螢幕模式顯示）
            if isFullScreen, let onComplete = onComplete {
                Section {
                    Button(role: .destructive) {
                        onComplete()
                    } label: {
                        Label(languageManager.text(Tab1Texts.setupGuideCloseButton), systemImage: "xmark")
                            .frame(maxWidth: .infinity, alignment: .center)
                    }
                }
            }
        }
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

// MARK: - 步驟列

private struct SetupGuideStepRow: View {
    let stepNumber: Int
    let title: String
    let screenshotName: String

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            // 步驟標題
            HStack(spacing: 12) {
                Text("\(stepNumber)")
                    .font(KeyboardModels.Fonts.appFont(size: 14).bold())
                    .foregroundColor(.white)
                    .frame(width: 24, height: 24)
                    .background(Color.accentColor)
                    .clipShape(Circle())

                Text(title)
            }

            // 截圖
            if let uiImage = UIImage(named: screenshotName) {
                Image(uiImage: uiImage)
                    .resizable()
                    .aspectRatio(contentMode: .fit)
                    .frame(maxWidth: .infinity)
                    .clipShape(RoundedRectangle(cornerRadius: 8))
            }
        }
        .padding(.vertical, 4)
    }
}
