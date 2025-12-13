import SwiftUI

/// 設定步驟列表
struct SetupStepsView: View {
    @EnvironmentObject var languageManager: LanguageManager

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            StepRow(number: 1, icon: "hand.tap.fill", text: AppTexts.onboardingStep1Settings)
            StepRow(number: 2, icon: "switch.2", text: AppTexts.onboardingStep2AddKeyboard)
            StepRow(number: 3, icon: "switch.2", text: AppTexts.onboardingStep3FullAccess)
        }
    }
}

struct StepRow: View {
    let number: Int
    let icon: String
    let text: LocalizedText
    @EnvironmentObject var languageManager: LanguageManager

    var body: some View {
        HStack(spacing: 14) {
            ZStack {
                Circle()
                    .fill(Color.Theme.accent.opacity(0.12))
                    .frame(width: 32, height: 32)

                Text("\(number)")
                    .font(.system(size: 14, weight: .semibold))
                    .foregroundColor(Color.Theme.accent)
            }

            HStack(spacing: 10) {
                Image(systemName: icon)
                    .font(.system(size: 15, weight: .medium))
                    .foregroundColor(Color.Theme.textSecondary)
                    .frame(width: 20)

                LocalizedTextView(text)
                    .themeFontBody()
                    .foregroundColor(Color.Theme.textPrimary)
            }

            Spacer()
        }
    }
}
