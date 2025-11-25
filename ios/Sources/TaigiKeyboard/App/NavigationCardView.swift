import SwiftUI

struct NavigationCardView: View {
    @ObservedObject var viewModel: OnboardingViewModel
    @Binding var showSettings: Bool
    @State private var isPressed: [Bool] = [false, false, false]

    var body: some View {
        VStack(spacing: 0) {
            // Setup Guide
            ListCardView(
                stripColor: Color.Theme.accent,
                title: AppTexts.setupGuide,
                isPressed: isPressed[0],
                isLast: false,
                action: {
                    withAnimation(ThemeAnimation.smooth) {
                        viewModel.showOnboardingForSettings()
                    }
                }
            )
            .pressableCardGesture(isPressed: isPressed[0]) { pressed in
                isPressed[0] = pressed
            }

            // Keyboard Settings
            ListCardView(
                stripColor: Color.Theme.accentSecondary,
                title: AppTexts.keyboardSettings,
                isPressed: isPressed[1],
                isLast: false,
                action: {
                    withAnimation(ThemeAnimation.smooth) {
                        showSettings = true
                    }
                }
            )
            .pressableCardGesture(isPressed: isPressed[1]) { pressed in
                isPressed[1] = pressed
            }

            // User Guide - 使用系統瀏覽器開啟
            ListCardView(
                stripColor: Color.Theme.accent,
                title: AppTexts.userGuide,
                isPressed: isPressed[2],
                isLast: true,
                action: {
                    if let url = URL(string: "https://www.taigikeyboard.tw/") {
                        UIApplication.shared.open(url)
                    }
                }
            )
            .pressableCardGesture(isPressed: isPressed[2]) { pressed in
                isPressed[2] = pressed
            }
        }
    }
}
