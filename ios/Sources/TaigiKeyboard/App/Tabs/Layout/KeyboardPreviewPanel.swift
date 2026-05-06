// 中文: 外觀設定子頁底部的鍵盤預覽面板。重用真實 TaigiKeyboardView 並注入 mock 候選列。

import KeyboardKit
import SwiftUI

/// A display-only keyboard preview that renders the real `TaigiKeyboardView`.
/// All appearance settings (key height, font size, candidate text size, corner radius, font)
/// propagate automatically through the injected `KeyboardEnvironment` →
/// `CustomLayoutService` / `ButtonFontProvider`.
// 中文: 顯示用鍵盤預覽 View。外觀參數透過注入的 KeyboardEnvironment 自動傳遞,
// 中文: 不接受實際輸入,候選列為靜態 mock 項。
struct KeyboardPreviewPanel: View {
    let keyHeightScale: Double
    let keyFontSizeScale: Double
    let candidateTextSizeScale: Double
    let keyCornerRadius: Double
    let fontType: FontType
    let colorScheme: ColorScheme

    @State private var previewState = Keyboard.State()
    @StateObject private var composingManager = ComposingManager()

    var body: some View {
        let services = Keyboard.Services(state: previewState)
        let layout = CustomLayoutService()
            .keyboardLayout(for: previewState.keyboardContext)
        let settings = SharedSettings.shared

        TaigiKeyboardView(
            settings: settings,
            services: services,
            layout: layout,
            emojiKeyboardView: { AnyView(EmptyView()) },
            calloutStyle: Self.createCalloutStyle(fontType: fontType),
            autocompleteContext: previewState.autocompleteContext,
            keyboardContext: previewState.keyboardContext,
            composingManager: composingManager,
            onSuggestionTap: { _ in },
            onTranslateToggle: {},
            initialInputMode: settings.inputMode == .english ? .tl : nil,
        )
        .keyboardState(previewState)
        .onAppear { configurePreviewContext() }
        .onChange(of: colorScheme) { _, newValue in
            previewState.keyboardContext.colorScheme = newValue
        }
    }

    // 中文: 在 onAppear 設定預覽用 KeyboardContext + 注入 mock 候選詞,讓使用者看到視覺效果。
    private func configurePreviewContext() {
        let ctx = previewState.keyboardContext
        ctx.isLiquidGlassEnabled = false
        ctx.screenSize = UIScreen.main.bounds.size
        ctx.colorScheme = colorScheme
        previewState.autocompleteContext.suggestionsFromService = [
            .init(text: "mī-tê", title: "mī-tê", subtitle: "麵茶"),
            .init(text: "kú-nî", title: "kú-nî", subtitle: "久年"),
            .init(text: "gîm-á", title: "gîm-á", subtitle: "砛仔"),
        ]
    }

    // 中文: 依 fontType 產生對應的 KeyboardKit Callout 樣式,確保彈出 callout 的字型與按鍵一致。
    private static func createCalloutStyle(fontType: FontType) -> Callouts.CalloutStyle {
        switch fontType {
        case .system:
            return .standard
        case .openHuninn:
            let name = KeyboardFonts.openHuninnFontName
            return Callouts.CalloutStyle(
                actionItemFont: KeyboardFont.custom(name, size: 20, weight: .regular),
                inputItemFont: KeyboardFont.custom(name, size: 32, weight: .light),
            )
        case .iansui:
            let name = KeyboardFonts.iansuiFontName
            return Callouts.CalloutStyle(
                actionItemFont: KeyboardFont.custom(name, size: 20, weight: .regular),
                inputItemFont: KeyboardFont.custom(name, size: 32, weight: .light),
            )
        case .genYoMin, .genYoGothic:
            let name = fontType.customFontName!
            return Callouts.CalloutStyle(
                actionItemFont: KeyboardFont.custom(name, size: 20, weight: .regular),
                inputItemFont: KeyboardFont.custom(name, size: 32, weight: .light),
            )
        }
    }
}
