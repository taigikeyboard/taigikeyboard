import SwiftUI

struct NavigationCardView: View {
    @ObservedObject var viewModel: OnboardingViewModel
    @Binding var showSettings: Bool
    @Binding var showDictionarySettings: Bool
    @State private var isPressed: [Bool] = [false, false, false]

    var body: some View {
        VStack(spacing: 0) {
            // 啟用方法
            ListCardView(
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

            // 齒盤設定
            ListCardView(
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

            // 詞庫管理
            ListCardView(
                title: AppTexts.dictionarySettings,
                isPressed: isPressed[2],
                isLast: true,
                action: {
                    withAnimation(ThemeAnimation.smooth) {
                        showDictionarySettings = true
                    }
                }
            )
            .pressableCardGesture(isPressed: isPressed[2]) { pressed in
                isPressed[2] = pressed
            }
        }
    }
}
