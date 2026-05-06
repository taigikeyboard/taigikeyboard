// 中文: 單一候選詞按鈕的 SwiftUI 視圖 — 處理顯示 / 副標題 / 按下狀態 / Liquid Glass 背景。

import KeyboardKit
import SwiftUI

/// 單個候選詞按鈕視圖
// 中文: 候選詞按鈕。Tap 後透過 onTap 把 commit 用的 suggestion 回呼給 caller。
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
    @Environment(\.candidateTheme) private var theme

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
                    .font(KeyboardFonts.globalFont(size: theme.primaryFontSize))
                    .fontWeight(.regular)
                    .foregroundColor(theme.primaryTextColor)
                    .lineLimit(1)

                if let subtitle = displaySubtitle, !subtitle.isEmpty, subtitle != displayTitle {
                    Text(subtitle)
                        .font(KeyboardFonts.globalFont(size: theme.secondaryFontSize))
                        .foregroundColor(theme.secondaryTextColor)
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
