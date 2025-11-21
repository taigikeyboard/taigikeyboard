import SwiftUI

/// 承諾項目列組件
struct CommitmentRow: View {
    let icon: String
    let text: LocalizedText
    let color: Color

    var body: some View {
        HStack(alignment: .top, spacing: 16) {
            Image(systemName: icon)
                .font(.system(size: 18, weight: .medium))
                .foregroundColor(Color.Theme.accent)
                .frame(width: 24, height: 24)

            Text(LanguageManager.shared.text(text))
                .font(Font.Theme.body)
                .foregroundColor(Color.Theme.textPrimary)
                .multilineTextAlignment(.leading)
                .lineSpacing(4)
                .frame(maxWidth: .infinity, alignment: .leading)
                .fixedSize(horizontal: false, vertical: true)

            Spacer(minLength: 0)
        }
        .padding(.vertical, 6)
    }
}
