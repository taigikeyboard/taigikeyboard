import SwiftUI

struct ResourcesCardView: View {
    @Binding var showContact: Bool
    @State private var isPressed: [Bool] = [false, false, false]
    @State private var showShareSheet = false

    private let appStoreURL = "https://www.taigikeyboard.tw/"
    private let appStoreReviewURL = "https://apps.apple.com/app/id6751871806?action=write-review"

    var body: some View {
        VStack(spacing: 0) {
            // Contact Us
            ListCardView(
                stripColor: Color.Theme.accent,
                title: AppTexts.contactUs,
                isPressed: isPressed[0],
                isLast: false,
                action: {
                    withAnimation(ThemeAnimation.smooth) {
                        showContact = true
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
                isLast: false,
                action: {
                    if let url = URL(string: appStoreReviewURL) {
                        UIApplication.shared.open(url)
                    }
                }
            )
            .pressableCardGesture(isPressed: isPressed[1]) { pressed in
                isPressed[1] = pressed
            }

            // Share to Friends
            ListCardView(
                stripColor: Color.Theme.accent,
                title: AppTexts.shareToFriends,
                isPressed: isPressed[2],
                isLast: true,
                action: {
                    withAnimation(ThemeAnimation.smooth) {
                        showShareSheet = true
                    }
                }
            )
            .pressableCardGesture(isPressed: isPressed[2]) { pressed in
                isPressed[2] = pressed
            }
        }
        .sheet(isPresented: $showShareSheet) {
            ShareSheet(items: [
                "台語齒盤 - 台語輸入法",
                URL(string: appStoreURL)!,
            ])
        }
    }
}
