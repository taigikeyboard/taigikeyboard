import SwiftUI

/// Icon button with press feedback animation for the expanded overlay control panel
struct ExpandedCandidateControlButton: View {
    let iconName: String
    let yOffset: CGFloat
    @Binding var isPressed: Bool
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            Image(systemName: iconName)
                .font(KeyboardFonts.globalFont(size: 20))
                .foregroundColor(CandidateViewModels.Colors.primaryTextColor)
                .frame(width: 45, height: 45, alignment: .center)
                .background(isPressed ? Color.gray.opacity(0.3) : Color.clear)
                .scaleEffect(isPressed ? 0.95 : 1.0)
                .contentShape(Rectangle())
                .offset(y: yOffset)
        }
        .buttonStyle(.plain)
        .onLongPressGesture(minimumDuration: 0, maximumDistance: .infinity, pressing: { pressing in
            withAnimation(.easeInOut(duration: 0.1)) {
                isPressed = pressing
            }
        }, perform: {})
    }
}

/// Vertical and horizontal divider lines for the right-side control panel area
struct FixedColumnDivider: View {
    var body: some View {
        GeometryReader { geometry in
            let cellHeight: CGFloat = 58

            Path { path in
                path.move(to: CGPoint(x: 0, y: 0))
                path.addLine(to: CGPoint(x: 0, y: geometry.size.height))

                var y: CGFloat = cellHeight
                while y < geometry.size.height {
                    path.move(to: CGPoint(x: 0, y: y))
                    path.addLine(to: CGPoint(x: geometry.size.width, y: y))
                    y += cellHeight
                }
            }
            .stroke(CandidateViewModels.Colors.separatorColor, lineWidth: 0.5)
        }
        .frame(width: 60)
    }
}
