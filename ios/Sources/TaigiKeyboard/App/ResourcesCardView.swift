import SwiftUI

struct ResourcesCardView: View {
    @State private var isPressed: [Bool] = [false, false]

    private let appStoreReviewURL = "https://apps.apple.com/app/id6751871806?action=write-review"
    private let contactFormURL = "https://docs.google.com/forms/d/e/1FAIpQLSd7PEppQ9MdAptvoY-PaaXDlbbL9Gq9Y4lFjgU9sLz4ENiPoA/viewform?usp=header"

    var body: some View {
        VStack(spacing: 0) {
            // Contact Us - 使用系統瀏覽器開啟 Google Forms
            ListCardView(
                stripColor: Color.Theme.accent,
                title: AppTexts.contactUs,
                isPressed: isPressed[0],
                isLast: false,
                action: {
                    if let url = URL(string: contactFormURL) {
                        UIApplication.shared.open(url)
                    }
                }
            )
            .pressableCardGesture(isPressed: isPressed[0]) { pressed in
                isPressed[0] = pressed
            }

            // Rate Us
            ListCardView(
                stripColor: Color.Theme.accentTertiary,
                title: AppTexts.rateUs,
                isPressed: isPressed[1],
                isLast: true,
                action: {
                    if let url = URL(string: appStoreReviewURL) {
                        UIApplication.shared.open(url)
                    }
                }
            )
            .pressableCardGesture(isPressed: isPressed[1]) { pressed in
                isPressed[1] = pressed
            }
        }
    }
}
