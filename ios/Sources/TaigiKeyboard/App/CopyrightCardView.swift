import SwiftUI

struct CopyrightCardView: View {
    @Binding var showCopyright: Bool
    @State private var isPressed = false

    var body: some View {
        ListCardView(
            stripColor: Color.Theme.accentTertiary,
            title: AppTexts.copyrightNotice,
            isPressed: isPressed,
            isLast: true,
            action: {
                withAnimation(ThemeAnimation.smooth) {
                    showCopyright = true
                }
            }
        )
        .pressableCardGesture(isPressed: isPressed) { pressed in
            isPressed = pressed
        }
    }
}
