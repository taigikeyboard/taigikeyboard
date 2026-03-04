import Combine
import KeyboardKit
import SwiftUI

struct TaigiKeyboardView: View {
    let services: Keyboard.Services
    let layout: KeyboardLayout
    let emojiKeyboardView: () -> AnyView
    let calloutStyle: Callouts.CalloutStyle

    @ObservedObject var autocompleteContext: AutocompleteContext
    @ObservedObject var keyboardContext: KeyboardContext
    @ObservedObject var composingManager: ComposingManager

    let onSuggestionTap: (Autocomplete.Suggestion) -> Void
    let onTranslateToggle: () -> Void
    var initialInputMode: InputMode? = nil

    @StateObject private var expandState = CandidateExpandState()
    @State private var currentInputMode: InputMode = SharedSettings.shared.inputMode
    @State private var colorSettings: KeyboardColorSettings = SharedSettings.shared.colorSettings
    @State private var keyFontSizeScale: CGFloat = SharedSettings.shared.keyFontSizeScale
    @State private var keyBorderWidth: CGFloat = SharedSettings.shared.keyBorderWidth
    @State private var isLayoutPanelExpanded = false

    var body: some View {
        // 根據 keyboardCase 轉換候選詞大小寫
        let suggestions = SuggestionCaseTransformer.transform(
            autocompleteContext.suggestions,
            composingText: composingManager.composingText,
            keyboardCase: keyboardContext.keyboardCase,
            inputMode: SharedSettings.shared.inputMode
        )
        let frequentWords = CandidateView.getSharedFrequentWords(in: suggestions)
        let isTranslateSwapped = keyboardContext.isTranslateSwapped

        // 響應式獲取選中狀態
        let selectedCandidateIndex = composingManager.selectedCandidateIndex

        // 根據 KeyboardContext 動態選擇候選詞樣式（含自訂候選詞背景色）
        let candidateStyle = Self.candidateStyle(for: keyboardContext, colorSettings: colorSettings)

        // Use Liquid Glass transparent pass-through only when enabled
        // AND no custom background color is set by the user.
        let useLiquidGlassBg = keyboardContext.isLiquidGlassEnabled
            && colorSettings.backgroundColor == nil

        // KeyboardKit 10: 使用 layout: 和 services: 參數
        KeyboardView(
            layout: layout,
            services: services,
            buttonContent: { params in
                // 使用自訂的按鈕內容，傳入標準視圖作為後備
                TaigiButtonContent(
                    action: params.item.action,
                    keyboardContext: keyboardContext,
                    standardContent: params.view,
                    keyFontSizeScale: keyFontSizeScale
                )
            },
            buttonView: { params in
                let borderWidth = self.keyBorderWidth
                if borderWidth > 0, params.item.action != .none {
                    params.view.overlay(
                        RoundedRectangle(cornerRadius: SharedSettings.shared.keyCornerRadius)
                            .strokeBorder(Color.black, lineWidth: borderWidth)
                            .padding(params.item.edgeInsets)
                    )
                } else {
                    params.view
                }
            },
            collapsedView: { $0.view },
            emojiKeyboard: { _ in
                // KeyboardKit 10: ISEmojiView 需要明確設置高度
                emojiKeyboardView()
                    .frame(height: layout.totalHeight)
            },
            toolbar: { params in
                // 統一使用 CandidateView，英文模式傳入 KeyboardKit 預設視圖
                CandidateView(
                    suggestions: suggestions,
                    frequentWords: frequentWords,
                    selectedCandidateIndex: selectedCandidateIndex,
                    onSuggestionTap: onSuggestionTap,
                    isTranslateSwapped: isTranslateSwapped,
                    onTranslateToggle: onTranslateToggle,
                    onSettingsTap: { [unowned services] in
                        services.actionHandler.handle(.settings)
                    },
                    onLayoutTap: {
                        withAnimation(.easeInOut(duration: 0.2)) {
                            isLayoutPanelExpanded.toggle()
                            if isLayoutPanelExpanded {
                                expandState.collapse()
                            }
                        }
                    },
                    onEmojiTap: {
                        keyboardContext.keyboardType = .emojis
                    },
                    currentInputMode: currentInputMode,
                    onInputModeChange: { newMode in
                        currentInputMode = newMode
                        SharedSettings.shared.inputMode = newMode
                    },
                    englishAutocompleteView: currentInputMode == .english ? AnyView(params.view) : nil,
                    isComposing: composingManager.isComposing
                )
                .environmentObject(expandState)
                .candidateViewStyle(candidateStyle)
            },
        )
        .keyboardButtonStyle { params in
            var style = params.standardStyle()
            let fontProvider = ButtonFontProvider(keyboardContext: params.context)
            style.keyboardFont = fontProvider.buttonKeyboardFont(for: params.action)
            style.cornerRadius = SharedSettings.shared.keyCornerRadius

            // Apply custom colors from SharedSettings
            let colors = self.colorSettings
            if let textColor = colors.keyTextColor?.color {
                style.foregroundColor = textColor
            }
            // Differentiate normal vs special key backgrounds.
            // Set both backgroundColor and background to cover KK10 rendering:
            // ButtonKey renders .background(style.background) on top of
            // .background(style.backgroundColor), so we must set both to
            // ensure the custom color shows regardless of which layer the
            // standard style populates.
            switch params.action {
            case .backspace, .shift, .nextKeyboard, .keyboardType, .dismissKeyboard, .settings,
                 .primary, .custom:
                if let fill = colors.specialKeyFillColor?.color {
                    style.backgroundColor = fill
                    style.background = .color(fill)
                }
            case .character, .space:
                if let fill = colors.normalKeyFillColor?.color {
                    style.backgroundColor = fill
                    style.background = .color(fill)
                }
            default:
                break
            }
            return style
        }
        .keyboardCalloutActions(Callouts.taigiToneActions)
        .keyboardCalloutStyle(calloutStyle)
        .overlay(
            ExpandedCandidateOverlay(
                suggestions: suggestions,
                frequentWords: frequentWords,
                selectedCandidateIndex: selectedCandidateIndex,
                onSuggestionTap: onSuggestionTap,
                isTranslateSwapped: isTranslateSwapped,
                onTranslateToggle: onTranslateToggle,
                onCollapse: {
                    expandState.collapse()
                },
                isExpanded: expandState.isExpanded,
            )
            .candidateViewStyle(candidateStyle)
            .offset(y: 2), // 稍微下移展開候選詞網格位置
            alignment: .topLeading,
        )
        .overlay(
            LayoutSelectionOverlay(
                isExpanded: isLayoutPanelExpanded,
                onDismiss: {
                    withAnimation(.easeInOut(duration: 0.2)) {
                        isLayoutPanelExpanded = false
                    }
                }
            )
            .offset(y: CandidateViewModels.UI.height),
            alignment: .topLeading,
        )
        .onChange(of: expandState.isExpanded) { _, isExpanded in
            if isExpanded {
                withAnimation(.easeInOut(duration: 0.2)) {
                    isLayoutPanelExpanded = false
                }
            }
        }
        .keyboardToolbarStyle(
            Keyboard.ToolbarStyle(
                // Use Liquid Glass pass-through only when enabled AND
                // no custom background color is set; otherwise clear
                // so the external .background() color shows through.
                backgroundColor: useLiquidGlassBg
                    ? .white.opacity(0.001)
                    : .clear
            )
        )
        .keyboardViewStyle(
            // Always make KK's internal background transparent so our
            // external .background() controls the color consistently
            // in both the keyboard extension and the preview panel.
            KeyboardViewStyle(
                background: useLiquidGlassBg
                    ? .color(Color.white.opacity(0.001))
                    : .color(.clear)
            )
        )
        .background(
            useLiquidGlassBg
                ? Color.white.opacity(0.001)
                : (colorSettings.backgroundColor?.color ?? Color.keyboardBackground)
        )
        .onAppear {
            if let mode = initialInputMode {
                currentInputMode = mode
            }
        }
        .onReceive(NotificationCenter.default.publisher(for: UserDefaults.didChangeNotification)) { _ in
            let latest = SharedSettings.shared.colorSettings
            if colorSettings != latest {
                colorSettings = latest
            }
            let latestScale = SharedSettings.shared.keyFontSizeScale
            if keyFontSizeScale != latestScale {
                keyFontSizeScale = latestScale
            }
            let latestBorderWidth = SharedSettings.shared.keyBorderWidth
            if keyBorderWidth != latestBorderWidth {
                keyBorderWidth = latestBorderWidth
            }
        }
    }

    private static func candidateStyle(
        for context: KeyboardContext,
        colorSettings: KeyboardColorSettings
    ) -> CandidateView.Style {
        var style = CandidateView.Style.adaptive(for: context)
        if let bg = colorSettings.candidateBackgroundColor?.color {
            style.backgroundColor = bg
        }
        return style
    }
}
