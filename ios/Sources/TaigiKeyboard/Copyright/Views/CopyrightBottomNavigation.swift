import SwiftUI

/// 底部導航列
struct CopyrightBottomNavigation: View {
    @Binding var currentPage: Int
    let totalPages: Int
    let onDismiss: () -> Void
    @State private var previousPressed = false
    @State private var nextPressed = false

    private var isLastPage: Bool {
        currentPage == totalPages - 1
    }

    private var canGoPrevious: Bool {
        currentPage > 0
    }

    var body: some View {
        HStack(spacing: 20) {
            Button(action: {
                if canGoPrevious {
                    withAnimation(.spring(response: 0.7, dampingFraction: 0.8)) {
                        currentPage -= 1
                    }
                }
            }) {
                HStack(spacing: 8) {
                    Image(systemName: "chevron.left")
                        .font(.system(size: 16, weight: .medium))
                        .offset(x: previousPressed ? -1 : 0)
                        .animation(.spring(response: 0.3, dampingFraction: 0.7), value: previousPressed)

                    LocalizedTextView(AppTexts.guidePreviousPage)
                        .font(.system(size: 16, weight: .medium))
                }
                .foregroundColor(canGoPrevious ? Color.Theme.textPrimary : Color.Theme.textSecondary)
                .padding(.horizontal, 24)
                .padding(.vertical, 14)
                .background(
                    Capsule()
                        .fill(Color.Theme.surfaceSecondary)
                        .overlay(
                            Capsule()
                                .stroke(Color.Theme.cardStroke, lineWidth: 1)
                        )
                )
            }
            .disabled(!canGoPrevious)
            .opacity(canGoPrevious ? 1 : 0.5)
            .pressable($previousPressed)
            .animation(.spring(response: 0.3, dampingFraction: 0.7), value: previousPressed)

            Spacer()

            Button(action: {
                if isLastPage {
                    onDismiss()
                } else {
                    withAnimation(.spring(response: 0.7, dampingFraction: 0.8)) {
                        currentPage += 1
                    }
                }
            }) {
                HStack(spacing: 8) {
                    LocalizedTextView(isLastPage ? AppTexts.done : AppTexts.guideNextPage)
                        .font(.system(size: 16, weight: .semibold))

                    Image(systemName: isLastPage ? "checkmark" : "chevron.right")
                        .font(.system(size: 16, weight: .medium))
                        .offset(x: nextPressed ? 1 : 0)
                        .animation(.spring(response: 0.3, dampingFraction: 0.7), value: nextPressed)
                }
                .foregroundColor(.white)
                .padding(.horizontal, 28)
                .padding(.vertical, 14)
                .background(
                    Capsule()
                        .fill(isLastPage ? Color.Theme.accentSecondary : Color.Theme.accent)
                )
            }
            .pressable($nextPressed)
            .animation(.spring(response: 0.3, dampingFraction: 0.7), value: nextPressed)
        }
        .frame(height: 60)
    }
}
