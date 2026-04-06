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
    @State private var isSymbolPanelExpanded = false
    @State private var isSettingsPanelExpanded = false

    /// Per-render-cycle cached settings and providers.
    /// Created once per body evaluation to avoid repeated UserDefaults reads.
    private struct RenderProviders {
        let settings: SettingsSnapshot
        let keyTextColor: Color
        let font: ButtonFontProvider
        let text: ButtonTextProvider
        let image: ButtonImageProvider

        init(keyboardContext: KeyboardContext) {
            settings = SharedSettings.shared.snapshot()
            keyTextColor = settings.colorSettings.keyTextColor?.color ?? Color(.label)
            font = ButtonFontProvider(keyboardContext: keyboardContext, settings: settings)
            text = ButtonTextProvider(keyboardContext: keyboardContext, settings: settings)
            image = ButtonImageProvider(keyboardContext: keyboardContext)
        }
    }

    var body: some View {
        let p = RenderProviders(keyboardContext: keyboardContext)

        // 根據 keyboardCase 轉換候選詞大小寫
        let suggestions = SuggestionCaseTransformer.transform(
            autocompleteContext.suggestions,
            composingText: composingManager.composingText,
            keyboardCase: keyboardContext.keyboardCase,
            inputMode: p.settings.inputMode
        )
        let frequentWords = CandidateView.getSharedFrequentWords(in: suggestions)
        let isTranslateSwapped = keyboardContext.isTranslateSwapped
        let selectedCandidateIndex = composingManager.selectedCandidateIndex
        let candidateStyle = Self.candidateStyle(for: keyboardContext, colorSettings: colorSettings)
        let useLiquidGlassBg = keyboardContext.isLiquidGlassEnabled
            && colorSettings.backgroundColor == nil

        keyboardWithOverlays(
            p: p,
            suggestions: suggestions,
            frequentWords: frequentWords,
            selectedCandidateIndex: selectedCandidateIndex,
            isTranslateSwapped: isTranslateSwapped,
            candidateStyle: candidateStyle
        )
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

    // MARK: - Keyboard with Overlays

    /// Core keyboard + overlay panels + state change handlers.
    /// Extracted from body to reduce type-checker complexity.
    @ViewBuilder
    private func keyboardWithOverlays(
        p: RenderProviders,
        suggestions: [Autocomplete.Suggestion],
        frequentWords: Set<String>,
        selectedCandidateIndex: Int,
        isTranslateSwapped: Bool,
        candidateStyle: CandidateView.Style
    ) -> some View {
        coreKeyboard(
            p: p,
            suggestions: suggestions,
            frequentWords: frequentWords,
            selectedCandidateIndex: selectedCandidateIndex,
            isTranslateSwapped: isTranslateSwapped,
            candidateStyle: candidateStyle
        )
        .overlay(
            Group {
                if expandState.isExpanded {
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
                        isExpanded: true,
                    )
                    .candidateViewStyle(candidateStyle)
                    .offset(y: 2)
                }
            },
            alignment: .topLeading,
        )
        .overlay(
            Group {
                if isLayoutPanelExpanded {
                    LayoutSelectionOverlay(
                        isExpanded: true,
                        onDismiss: {
                            isLayoutPanelExpanded = false
                        }
                    )
                    .offset(y: CandidateViewModels.UI.height)
                }
            },
            alignment: .topLeading,
        )
        .overlay(
            Group {
                if isSymbolPanelExpanded {
                    SymbolSelectionOverlay(
                        isExpanded: true,
                        onSymbolInsert: { symbol in
                            keyboardContext.textDocumentProxy.insertText(symbol)
                        },
                        onDismiss: {
                            withAnimation(.easeInOut(duration: 0.2)) {
                                isSymbolPanelExpanded = false
                            }
                        }
                    )
                    .offset(y: CandidateViewModels.UI.height)
                }
            },
            alignment: .topLeading,
        )
        .overlay(
            Group {
                if isSettingsPanelExpanded {
                    SettingsSelectionOverlay(
                        isExpanded: true,
                        onDismiss: {
                            isSettingsPanelExpanded = false
                        },
                        onOpenApp: { [unowned services] in
                            services.actionHandler.handle(.settings)
                        }
                    )
                    .offset(y: CandidateViewModels.UI.height)
                }
            },
            alignment: .topLeading,
        )
        .onChange(of: expandState.isExpanded) { _, isExpanded in
            if isExpanded {
                isLayoutPanelExpanded = false
                isSymbolPanelExpanded = false
                isSettingsPanelExpanded = false
            }
        }
        .onChange(of: composingManager.isComposing) { _, isComposing in
            if isComposing && isSymbolPanelExpanded {
                isSymbolPanelExpanded = false
            }
            if isComposing && isSettingsPanelExpanded {
                isSettingsPanelExpanded = false
            }
        }
    }

    // MARK: - Core Keyboard View

    /// Builds the KeyboardView with button content, style, and toolbar.
    /// Extracted from body to reduce type-checker complexity.
    @ViewBuilder
    private func coreKeyboard(
        p: RenderProviders,
        suggestions: [Autocomplete.Suggestion],
        frequentWords: Set<String>,
        selectedCandidateIndex: Int,
        isTranslateSwapped: Bool,
        candidateStyle: CandidateView.Style
    ) -> some View {
        // KeyboardKit 10: 使用 layout: 和 services: 參數
        KeyboardView(
            layout: layout,
            services: services,
            buttonContent: { params in
                TaigiButtonContent(
                    action: params.item.action,
                    keyboardContext: keyboardContext,
                    standardContent: params.view,
                    textProvider: p.text,
                    imageProvider: p.image,
                    fontProvider: p.font,
                    keyTextColor: p.keyTextColor,
                    inputMode: p.settings.inputMode
                )
            },
            buttonView: { params in
                let borderWidth = self.keyBorderWidth
                if borderWidth > 0, params.item.action != .none {
                    params.view.overlay(
                        RoundedRectangle(cornerRadius: p.settings.keyCornerRadius)
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
                    onSettingsTap: {
                        isSettingsPanelExpanded.toggle()
                        if isSettingsPanelExpanded {
                            isLayoutPanelExpanded = false
                            isSymbolPanelExpanded = false
                            expandState.collapse()
                        }
                    },
                    onLayoutTap: {
                        isLayoutPanelExpanded.toggle()
                        if isLayoutPanelExpanded {
                            isSymbolPanelExpanded = false
                            isSettingsPanelExpanded = false
                            expandState.collapse()
                        }
                    },
                    onSymbolTap: {
                        isSymbolPanelExpanded.toggle()
                        if isSymbolPanelExpanded {
                            isLayoutPanelExpanded = false
                            isSettingsPanelExpanded = false
                            expandState.collapse()
                        }
                    },
                    onDismissKeyboard: { [unowned services] in
                        isLayoutPanelExpanded = false
                        isSymbolPanelExpanded = false
                        isSettingsPanelExpanded = false
                        services.actionHandler.handle(.dismissKeyboard)
                    },
                    currentInputMode: currentInputMode,
                    onInputModeChange: { newMode in
                        currentInputMode = newMode
                        SharedSettings.shared.inputMode = newMode
                        // Close any open overlay panels
                        isSymbolPanelExpanded = false
                        isLayoutPanelExpanded = false
                        isSettingsPanelExpanded = false
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
            style.keyboardFont = p.font.buttonKeyboardFont(for: params.action)
            style.cornerRadius = p.settings.keyCornerRadius

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
