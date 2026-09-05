// 外觀編輯器底部的鍵盤預覽面板。用一份 ThemeAppearance(草稿或預設 buffer)
// 透過 ThemePreviewEnvironment 渲染真實 TaigiKeyboardView,不讀寫全域設定。

import KeyboardKit
import SwiftUI

/// A display-only keyboard preview that renders the real `TaigiKeyboardView`
/// against a supplied `ThemeAppearance` (a draft user theme, or the default
/// buffer). Colors / sizes / shadow come from the appearance via
/// `ThemePreviewEnvironment`; row height + corner follow it through the injected
/// `CustomLayoutService` overload. Font is GLOBAL (not part of the theme), so
/// both the keys and the callout use `settings.fontType`. Nothing here writes
/// the live settings.
// 顯示用鍵盤預覽。外觀來自傳入的 appearance(草稿);字型走全域設定(非主題)。不接受輸入,候選列為靜態 mock。
struct KeyboardPreviewPanel: View {
    /// The appearance to render — bound draft (theme editor) or default-buffer appearance.
    let appearance: ThemeAppearance
    /// `true` for user-theme drafts (shadow slider 0 = flat); `false` for the
    /// default buffer (keeps KeyboardKit's standard shadow, matching the keyboard).
    let appliesThemeShadow: Bool
    let colorScheme: ColorScheme

    @State private var previewState = KeyboardState()
    @State private var composingManager = ComposingManager()

    var body: some View {
        let services = KeyboardServices(state: previewState)
        let layout = CustomLayoutService()
            .keyboardLayout(for: previewState.keyboardContext, appearance: appearance)
        let settings = ThemePreviewEnvironment(
            appearance: appearance,
            appliesThemeShadow: appliesThemeShadow,
        )

        TaigiKeyboardView(
            settings: settings,
            services: services,
            layout: layout,
            emojiKeyboardView: { AnyView(EmptyView()) },
            calloutStyle: .taigi(for: settings.fontType),
            autocompleteContext: previewState.autocompleteContext,
            keyboardContext: previewState.keyboardContext,
            composingManager: composingManager,
            onSuggestionTap: { _ in },
            onTranslateToggle: {},
            onCandidateDisplayModeChange: { _ in },
            initialInputMode: settings.inputMode == .english ? .tl : nil,
        )
        .keyboardState(previewState)
        .onAppear { configurePreviewContext() }
        .onChange(of: colorScheme) { _, newValue in
            previewState.keyboardContext.colorScheme = newValue
        }
    }

    // 在 onAppear 設定預覽用 KeyboardContext + 注入 mock 候選詞,讓使用者看到視覺效果。
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
}
