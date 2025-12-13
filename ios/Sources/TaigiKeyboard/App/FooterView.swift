import SwiftUI

struct FooterView: View {
    var body: some View {
        VStack(spacing: 20) {
            RoundedRectangle(cornerRadius: 3)
                .fill(Color.Theme.accent)
                .frame(width: 48, height: 6)
                .padding(.top, 8)

            VStack(spacing: 8) {
                LocalizedTextView(AppTexts.copyright)
                    .themeFontFootnote()
                    .foregroundColor(Color.Theme.textSecondary.opacity(0.7))
                    .multilineTextAlignment(.center)
                    .opacity(0.8)

                if let version = Bundle.main.object(forInfoDictionaryKey: "CFBundleShortVersionString") as? String {
                    Text("v\(version)")
                        .font(.system(size: 10, weight: .regular))
                        .foregroundColor(Color.Theme.textSecondary)
                        .opacity(0.5)
                }
            }
        }
        .padding(.vertical, 20)
        .frame(maxWidth: .infinity)
    }
}
