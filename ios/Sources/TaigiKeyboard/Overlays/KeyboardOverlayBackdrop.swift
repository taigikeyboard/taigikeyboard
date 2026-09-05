// 工具列 overlay(符號 / 版面 / 設定)的主題背景 — gradient 主題下重畫漸層,
// 讓 panel 與 gradient-painted 鍵盤連續(panel 偏移在工具列下方,需切片對齊座標)。

import SwiftUI

/// Paints a keyboard overlay panel's theme backdrop so it stays continuous with the
/// gradient-painted keyboard root.
///
/// The symbol / layout / settings panels are mounted as siblings ON TOP of the
/// gradient-painted keyboard (`TaigiKeyboardView` `.background { LinearGradient }`) and
/// are offset DOWN by the toolbar height, so they cover the keyboard keys — making them
/// transparent would show the keys, not the gradient. They must repaint the gradient.
///
/// Because a panel occupies `[topInset, fullKeyboardHeight]` of the keyboard, painting a
/// `.top → .bottom` gradient inside the panel's own bounds would restart the gradient at
/// the panel top and leave a visible seam against the transparent toolbar above it (which
/// shows the root gradient's `[0, topInset]` slice). To stay continuous, shift the gradient's
/// start point ABOVE the panel (a negative-y `UnitPoint`) so the panel shows exactly the
/// gradient's `[topInset, fullKeyboardHeight]` slice in-bounds — NOT an oversized frame +
/// `.offset` + `.clipped()`, which froze the keyboard extension on gradient themes (#429).
///
/// Flat / default themes keep today's `Color.keyboardBackground` (no behavior change); only
/// gradient themes gain the repaint. Mirrors `ExpandedCandidateOverlay.backgroundView`'s
/// gradient branch (that overlay spans the full keyboard, so it needs no slice offset).
struct KeyboardOverlayBackdrop: ViewModifier {
    /// Top→bottom gradient stops of the active theme, or nil for a flat / default theme.
    let gradientColors: [Color]?
    /// Full keyboard height (candidate bar + keyboard) the root gradient spans.
    let fullKeyboardHeight: CGFloat
    /// Height of the toolbar above the panel — the panel's vertical offset into the gradient.
    let topInset: CGFloat

    func body(content: Content) -> some View {
        content.background(alignment: .top) { backdrop }
    }

    @ViewBuilder
    private var backdrop: some View {
        if let gradientColors {
            // Plain in-bounds gradient (like ExpandedCandidateOverlay) with the start point
            // shifted above the panel so it paints the [topInset, fullKeyboardHeight] slice.
            // panelHeight > 0 guards transient/degenerate geometry (fall back to .top).
            let panelHeight = fullKeyboardHeight - topInset
            let startY = panelHeight > 0 ? -topInset / panelHeight : 0
            LinearGradient(
                colors: gradientColors,
                startPoint: UnitPoint(x: 0.5, y: startY),
                endPoint: .bottom,
            )
        } else {
            Color.keyboardBackground
        }
    }
}

extension View {
    /// Apply the gradient-continuous keyboard overlay backdrop. See `KeyboardOverlayBackdrop`.
    func keyboardOverlayBackdrop(
        gradientColors: [Color]?,
        fullKeyboardHeight: CGFloat,
        topInset: CGFloat,
    ) -> some View {
        modifier(
            KeyboardOverlayBackdrop(
                gradientColors: gradientColors,
                fullKeyboardHeight: fullKeyboardHeight,
                topInset: topInset,
            ),
        )
    }

    /// Mount this content as a toolbar overlay panel (symbol / layout / settings): shown only
    /// when `isExpanded`, sized to fill the keyboard below the toolbar, with the theme backdrop
    /// applied. Centralizes the geometry + inset wiring the three panels share — the panel sits
    /// `theme.height` (toolbar height) below the keyboard top, so that is both its size offset and
    /// the gradient's top inset.
    @ViewBuilder
    func keyboardOverlayPanel(isExpanded: Bool, theme: CandidateTheme) -> some View {
        if isExpanded {
            GeometryReader { geometry in
                let toolbarHeight = theme.height
                self
                    .frame(maxWidth: .infinity)
                    .frame(height: geometry.size.height - toolbarHeight)
                    .keyboardOverlayBackdrop(
                        gradientColors: theme.backgroundGradientColors,
                        fullKeyboardHeight: geometry.size.height,
                        topInset: toolbarHeight,
                    )
            }
        }
    }
}
