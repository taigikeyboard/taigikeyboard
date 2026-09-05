import SwiftUI

/// Keyboard setup guide.
///
/// Shows activation steps. Shared between full-screen onboarding and HomeTab navigation.
struct SetupGuideView: View {
    @ObservedObject var viewModel: SetupGuideViewModel
    @Environment(\.openURL) private var openURL
    @Environment(DisplayLanguageStore.self) private var lang

    /// Full-screen mode (matches Android SetupGuideActivity.isFullScreen).
    var isFullScreen: Bool = false

    /// Dismiss callback (only used when isFullScreen = true).
    var onComplete: (() -> Void)?

    var body: some View {
        Form {
            // Full-screen title
            if isFullScreen {
                Section {
                    Text(lang.string(.homeSetupGuide))
                        .font(AppStyle.appFont(size: AppStyle.navBarLargeTitleSize))
                        .fontWeight(.bold)
                }
            }

            // Description
            Section {
                Text(lang.string(.homeSetupGuideDescription))
                    .lineSpacing(4)
            }

            // Steps
            Section {
                SetupGuideStepRow(
                    stepNumber: 1,
                    title: lang.string(.homeSetupGuideStep1Settings),
                    screenshotName: "setup_step1",
                )

                SetupGuideStepRow(
                    stepNumber: 2,
                    title: lang.string(.homeSetupGuideStep2AddKeyboard),
                    screenshotName: "setup_step2",
                )
            }

            // Completion message
            Section {
                Text(lang.string(.homeSetupGuideCompletedMessage))
            }

            // Open Settings button
            Section {
                Button {
                    if let url = URL(string: UIApplication.openSettingsURLString) {
                        openURL(url)
                    }
                } label: {
                    Label(lang.string(.homeSetupGuideGoToSettings), systemImage: "gearshape.fill")
                        .frame(maxWidth: .infinity, alignment: .center)
                }
            }

            // Warnings
            Section {
                Label {
                    Text(lang.string(.homeSetupInfoMessage))
                        .lineSpacing(4)
                } icon: {
                    Image(latinSystemName: "exclamationmark.triangle.fill")
                        .foregroundColor(AppStyle.warningOrange)
                }

                Label {
                    Text(lang.string(.homeSetupBrandWarning))
                        .lineSpacing(4)
                } icon: {
                    Image(latinSystemName: "exclamationmark.triangle.fill")
                        .foregroundColor(AppStyle.warningOrange)
                }
            }

            // Close button (full-screen only)
            if isFullScreen, let onComplete {
                Section {
                    Button(role: .destructive) {
                        onComplete()
                    } label: {
                        Label(lang.string(.homeSetupGuideCloseButton), systemImage: "xmark")
                            .frame(maxWidth: .infinity, alignment: .center)
                    }
                }
            }
        }
        .navigationTitle(isFullScreen ? "" : lang.string(.homeSetupGuide))
        .navigationBarTitleDisplayMode(.large)
        .navigationBarHidden(isFullScreen)
        .onReceive(NotificationCenter.default.publisher(for: UIApplication.didBecomeActiveNotification)) { _ in
            Task { @MainActor in
                viewModel.refresh()
            }
        }
    }
}

// MARK: - Step Row

// One row of the setup steps list (numbered circle + title + screenshot).
private struct SetupGuideStepRow: View {
    let stepNumber: Int
    let title: String
    let screenshotName: String

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            // Step title
            HStack(spacing: 12) {
                Text("\(stepNumber)")
                    .font(AppStyle.captionFont.bold())
                    .foregroundColor(.white)
                    .frame(width: 24, height: 24)
                    .background(AppStyle.accentBlue)
                    .clipShape(Circle())

                Text(title)
            }

            // Screenshot
            if let uiImage = UIImage(named: screenshotName) {
                Image(uiImage: uiImage)
                    .resizable()
                    .aspectRatio(contentMode: .fit)
                    .frame(maxWidth: .infinity)
                    .clipShape(RoundedRectangle(cornerRadius: AppStyle.smallCornerRadius))
            }
        }
        .padding(.vertical, 4)
    }
}
