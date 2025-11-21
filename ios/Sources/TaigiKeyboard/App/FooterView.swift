import SwiftUI

struct FooterView: View {
    var body: some View {
        VStack(spacing: 20) {
            RoundedRectangle(cornerRadius: 3)
                .fill(Color.Theme.accentTertiary)
                .frame(width: 48, height: 6)
                .padding(.top, 8)

            VStack(spacing: 12) {
                LocalizedTextView(AppTexts.copyright)
                    .font(Font.Theme.footnote)
                    .foregroundColor(Color.Theme.textSecondary.opacity(0.7))
                    .multilineTextAlignment(.center)
                    .opacity(0.8)
            }
        }
        .padding(.vertical, 20)
        .frame(maxWidth: .infinity)
    }
}
