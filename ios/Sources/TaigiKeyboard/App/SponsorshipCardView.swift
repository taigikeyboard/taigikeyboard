import SwiftUI

struct SponsorshipCardView: View {
    @Binding var showSponsorship: Bool
    @State private var isPressed = false

    var body: some View {
        ListCardView(
            stripColor: Color.Theme.accentSecondary,
            title: AppTexts.sponsorship,
            isPressed: isPressed,
            isLast: true,
            action: {
                withAnimation(ThemeAnimation.smooth) {
                    showSponsorship = true
                }
            }
        )
        .pressableCardGesture(isPressed: isPressed) { pressed in
            isPressed = pressed
        }
    }
}
