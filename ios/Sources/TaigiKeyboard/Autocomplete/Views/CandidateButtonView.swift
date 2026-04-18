import KeyboardKit
import SwiftUI

/// 單個候選詞按鈕視圖
struct CandidateButtonView: View {
    let suggestion: Autocomplete.Suggestion
    let isTranslateSwapped: Bool
    let isTPSLayout: Bool
    let orMapsToER: Bool
    let isSelected: Bool
    let onTap: (Autocomplete.Suggestion) -> Void

    @State private var isPressed = false
    @Environment(\.colorScheme) private var colorScheme
    @Environment(\.candidateViewStyle) private var style

    private var displayTitle: String {
        CandidateCellHelper.displayTitle(
            for: suggestion,
            isTranslateSwapped: isTranslateSwapped,
            isTPSLayout: isTPSLayout,
            orMapsToER: orMapsToER,
        )
    }

    private var displaySubtitle: String? {
        CandidateCellHelper.displaySubtitle(
            for: suggestion,
            isTranslateSwapped: isTranslateSwapped,
            isTPSLayout: isTPSLayout,
        )
    }

    private var backgroundColor: Color {
        style.itemStyle.resolvedBackgroundColor(
            for: colorScheme,
            isSelected: isSelected,
            isPressed: isPressed,
            isLiquidGlassEnabled: style.isLiquidGlassEnabled,
        )
    }

    private var cornerRadius: CGFloat {
        style.itemStyle.cornerRadius ?? 8
    }

    var body: some View {
        Button(action: {
            onTap(CandidateCellHelper.suggestionToHandle(
                for: suggestion,
                isTranslateSwapped: isTranslateSwapped,
                isTPSLayout: isTPSLayout,
                orMapsToER: orMapsToER,
            ))
        }) {
            VStack(alignment: .center, spacing: 0) {
                Text(displayTitle)
                    .font(KeyboardFonts.globalFont(size: CandidateCellHelper.titleFontSize))
                    .fontWeight(.regular)
                    .foregroundColor(CandidateViewModels.Colors.primaryTextColor)
                    .lineLimit(1)

                if let subtitle = displaySubtitle, !subtitle.isEmpty, subtitle != displayTitle {
                    Text(subtitle)
                        .font(KeyboardFonts.globalFont(size: CandidateCellHelper.subtitleFontSize))
                        .foregroundColor(CandidateViewModels.Colors.secondaryTextColor)
                        .lineLimit(1)
                }
            }
            .padding(.horizontal, style.itemStyle.horizontalPadding)
            .padding(.vertical, style.itemStyle.verticalPadding)
        }
        .background(
            RoundedRectangle(cornerRadius: cornerRadius)
                .fill(backgroundColor)
                .padding(.horizontal, -2)
                .padding(.vertical, -4),
        )
        .offset(y: 5) // 讓整個候選詞項目背景往下移動，與候選列下沿對齊
        .scaleEffect(isPressed ? 0.95 : 1.0)
        .buttonStyle(PlainButtonStyle())
        .onLongPressGesture(minimumDuration: 0, maximumDistance: .infinity, pressing: { pressing in
            withAnimation(.easeInOut(duration: 0.1)) {
                isPressed = pressing
            }
        }, perform: {})
    }
}
