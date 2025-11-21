import SwiftUI

/// 版權頁面頂部元件
struct CopyrightHeader: View {
    @Binding var currentPage: Int
    let totalPages: Int
    let onDismiss: () -> Void

    var body: some View {
        VStack(spacing: 24) {
            HStack {
                Spacer()

                CloseButton(action: onDismiss)
                .padding(.trailing, 24)
            }

            CopyrightProgressIndicator(
                currentPage: $currentPage,
                totalPages: totalPages
            )
            .padding(.horizontal, 24)
        }
        .padding(.top, 8)
    }
}

/// 頁面進度指示器
struct CopyrightProgressIndicator: View {
    @Binding var currentPage: Int
    let totalPages: Int

    var body: some View {
        VStack(spacing: 20) {
            GeometryReader { geometry in
                ZStack(alignment: .leading) {
                    Capsule()
                        .fill(Color.Theme.textSecondary.opacity(0.15))
                        .frame(height: 3)

                    Capsule()
                        .fill(Color.Theme.accent)
                        .frame(
                            width: max(0, geometry.size.width * CGFloat(currentPage + 1) / CGFloat(totalPages)),
                            height: 3
                        )
                        .animation(.spring(response: 0.7, dampingFraction: 0.8), value: currentPage)
                }
            }
            .frame(height: 3)

            Text("\(currentPage + 1) / \(totalPages)")
                .font(.system(size: 16, weight: .medium, design: .rounded))
                .foregroundColor(Color.Theme.textPrimary)
                .padding(.horizontal, 16)
                .padding(.vertical, 6)
                .background(
                    Capsule()
                        .fill(Color.Theme.surfaceSecondary)
                        .overlay(
                            Capsule()
                                .stroke(Color.Theme.cardStroke, lineWidth: 1)
                        )
                )
        }
    }
}
