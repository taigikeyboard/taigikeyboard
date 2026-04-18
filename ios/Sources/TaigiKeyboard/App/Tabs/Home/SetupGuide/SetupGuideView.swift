import SwiftUI

/// Keyboard setup guide.
///
/// Shows activation steps. Shared between full-screen onboarding and Tab1 navigation.
struct SetupGuideView: View {
    @ObservedObject var viewModel: SetupGuideViewModel
    @Environment(\.openURL) private var openURL

    /// Full-screen mode (matches Android SetupGuideActivity.isFullScreen).
    var isFullScreen: Bool = false

    /// Dismiss callback (only used when isFullScreen = true).
    var onComplete: (() -> Void)?

    var body: some View {
        Form {
            // Full-screen title
            if isFullScreen {
                Section {
                    Text(Tab1Texts.setupGuide)
                        .font(AppStyle.appFont(size: AppStyle.navBarLargeTitleSize))
                        .fontWeight(.bold)
                }
            }

            // Description
            Section {
                Text(Tab1Texts.setupGuideDescription)
                    .lineSpacing(4)
            }

            // Steps
            Section {
                SetupGuideStepRow(
                    stepNumber: 1,
                    title: Tab1Texts.setupGuideStep1Settings,
                    screenshotName: "setup_step1",
                )

                SetupGuideStepRow(
                    stepNumber: 2,
                    title: Tab1Texts.setupGuideStep2AddKeyboard,
                    screenshotName: "setup_step2",
                )
            }

            // Completion message
            Section {
                Text(Tab1Texts.setupGuideCompletedMessage)
            }

            // Open Settings button
            Section {
                Button {
                    if let url = URL(string: UIApplication.openSettingsURLString) {
                        openURL(url)
                    }
                } label: {
                    Label(Tab1Texts.setupGuideGoToSettings, systemImage: "gearshape.fill")
                        .frame(maxWidth: .infinity, alignment: .center)
                }
            }

            // Warnings
            Section {
                Label {
                    Text(Tab1Texts.setupInfoMessage)
                        .lineSpacing(4)
                } icon: {
                    Image(systemName: "exclamationmark.triangle.fill")
                        .foregroundColor(AppStyle.warningOrange)
                }

                Label {
                    Text(Tab1Texts.setupBrandWarning)
                        .lineSpacing(4)
                } icon: {
                    Image(systemName: "exclamationmark.triangle.fill")
                        .foregroundColor(AppStyle.warningOrange)
                }
            }

            // Close button (full-screen only)
            if isFullScreen, let onComplete {
                Section {
                    Button(role: .destructive) {
                        onComplete()
                    } label: {
                        Label(Tab1Texts.setupGuideCloseButton, systemImage: "xmark")
                            .frame(maxWidth: .infinity, alignment: .center)
                    }
                }
            }
        }
        .navigationTitle(isFullScreen ? "" : Tab1Texts.setupGuide)
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
