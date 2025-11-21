import SwiftUI

/// 引導流程動作按鈕
struct OnboardingActionButton: View {
    let text: LocalizedText
    let action: () -> Void
    @EnvironmentObject var languageManager: LanguageManager
    @State private var isPressed = false

    var body: some View {
        Button(action: action) {
            LocalizedTextView(text)
                .font(Font.Theme.headline)
                .foregroundColor(.white)
                .frame(maxWidth: .infinity)
                .padding(.vertical, 16)
                .background(
                    RoundedRectangle(cornerRadius: 12)
                        .fill(Color.Theme.accent)
                        .shadow(
                            color: Color.Theme.accent.opacity(0.3),
                            radius: 8,
                            x: 0,
                            y: 4
                        )
                )
        }
        .buttonStyle(PlainButtonStyle())
        .pressable($isPressed, scaleEffect: 0.96, minimumDistance: 0)
    }
}
