import SwiftUI

/// 操作說明視圖
struct UserGuideView: View {
    @Environment(\.dismiss) var dismiss
    @State private var hasAppeared = false
    @State private var isLoading = true

    var body: some View {
        NavigationView {
            ZStack {
                // Background
                Color.Theme.surfacePrimary
                    .ignoresSafeArea()

                // Loading State
                if isLoading {
                    UserGuideLoadingView()
                        .opacity(hasAppeared ? 1 : 0)
                        .scaleEffect(hasAppeared ? 1 : 0.9)
                        .animation(
                            ThemeAnimation.smooth.delay(0.2),
                            value: hasAppeared
                        )
                }

                // Web Content
                VStack(spacing: 0) {
                    UserGuideWebView(
                        url: URL(string: "https://www.taigikeyboard.tw/")!,
                        onLoadingChange: { loading in
                            withAnimation(ThemeAnimation.smooth) {
                                isLoading = loading
                            }
                        }
                    )
                    .themedCard()
                    .padding(.horizontal, 16)
                    .padding(.bottom, 16)
                    .opacity(isLoading ? 0 : 1)
                    .scaleEffect(isLoading ? 0.95 : 1)
                    .animation(
                        ThemeAnimation.smooth.delay(0.3),
                        value: isLoading
                    )
                }
            }
            .navigationTitle("")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarTrailing) {
                    UserGuideCloseButton(action: { dismiss() })
                        .opacity(hasAppeared ? 1 : 0)
                        .scaleEffect(hasAppeared ? 1 : 0.8)
                        .animation(
                            ThemeAnimation.bouncy.delay(0.4),
                            value: hasAppeared
                        )
                }
            }
        }
        .navigationViewStyle(.stack)
        .onAppear {
            withAnimation {
                hasAppeared = true
            }
        }
    }
}

/// 載入中視圖
struct UserGuideLoadingView: View {
    @State private var isAnimating = false

    var body: some View {
        VStack(spacing: 24) {
            // Retro Loading Circle
            ZStack {
                Circle()
                    .fill(Color.Theme.surfaceSecondary)
                    .overlay(
                        Circle()
                            .stroke(Color.Theme.cardStroke, lineWidth: 1)
                    )
                    .frame(width: 80, height: 80)

                // Rotating Progress Indicator
                Circle()
                    .trim(from: 0, to: 0.7)
                    .stroke(
                        Color.Theme.accent,
                        style: StrokeStyle(lineWidth: 3, lineCap: .round)
                    )
                    .frame(width: 32, height: 32)
                    .rotationEffect(Angle(degrees: isAnimating ? 360 : 0))
                    .animation(
                        Animation.linear(duration: 1).repeatForever(autoreverses: false),
                        value: isAnimating
                    )
            }

            Text("載入中...")
                .font(Font.Theme.caption)
                .foregroundColor(Color.Theme.textSecondary)
        }
        .onAppear {
            isAnimating = true
        }
    }
}
