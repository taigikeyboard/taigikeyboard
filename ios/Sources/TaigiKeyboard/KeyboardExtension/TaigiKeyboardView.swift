import KeyboardKit
import SwiftUI

struct TaigiKeyboardView: View {
    let settings: any KeyboardEnvironment
    let services: Keyboard.Services
    let layout: KeyboardLayout
    let emojiKeyboardView: () -> AnyView
    let calloutStyle: Callouts.CalloutStyle

    @ObservedObject var autocompleteContext: AutocompleteContext
    @ObservedObject var keyboardContext: KeyboardContext
    @ObservedObject var composingManager: ComposingManager

    let onSuggestionTap: (Autocomplete.Suggestion) -> Void
    let onTranslateToggle: () -> Void
    var initialInputMode: InputMode?

    @StateObject private var expandState = CandidateExpandState()
    @State private var currentInputMode: InputMode
    @State private var colorSettings: KeyboardColorSettings
    @State private var keyFontSizeScale: CGFloat
    @State private var keyBorderWidth: CGFloat
    @State private var candidateTextSizeScale: CGFloat
    @State private var panels = OverlayPanelState()

    init(
        settings: any KeyboardEnvironment,
        services: Keyboard.Services,
        layout: KeyboardLayout,
        emojiKeyboardView: @escaping () -> AnyView,
        calloutStyle: Callouts.CalloutStyle,
        autocompleteContext: AutocompleteContext,
        keyboardContext: KeyboardContext,
        composingManager: ComposingManager,
        onSuggestionTap: @escaping (Autocomplete.Suggestion) -> Void,
        onTranslateToggle: @escaping () -> Void,
        initialInputMode: InputMode? = nil,
    ) {
        self.settings = settings
        self.services = services
        self.layout = layout
        self.emojiKeyboardView = emojiKeyboardView
        self.calloutStyle = calloutStyle
        self.autocompleteContext = autocompleteContext
        self.keyboardContext = keyboardContext
        self.composingManager = composingManager
        self.onSuggestionTap = onSuggestionTap
        self.onTranslateToggle = onTranslateToggle
        self.initialInputMode = initialInputMode
        _currentInputMode = State(initialValue: settings.inputMode)
        _colorSettings = State(initialValue: settings.colorSettings)
        _keyFontSizeScale = State(initialValue: settings.keyFontSizeScale)
        _keyBorderWidth = State(initialValue: settings.keyBorderWidth)
        _candidateTextSizeScale = State(initialValue: settings.candidateTextSizeScale)
    }

    /// Per-render-cycle cached settings and providers.
    /// Created once per body evaluation to avoid repeated UserDefaults reads.
    private struct RenderProviders {
        let settings: SettingsSnapshot
        let keyTextColor: Color
        let font: ButtonFontProvider
        let text: ButtonTextProvider
        let image: ButtonImageProvider

        init(keyboardContext: KeyboardContext, settings: SettingsSnapshot) {
            self.settings = settings
            keyTextColor = settings.colorSettings.keyTextColor?.color ?? Color(.label)
            font = ButtonFontProvider(keyboardContext: keyboardContext, settings: settings)
            text = ButtonTextProvider(keyboardContext: keyboardContext, settings: settings)
            image = ButtonImageProvider(keyboardContext: keyboardContext)
        }
    }

    var body: some View {
        let p = RenderProviders(keyboardContext: keyboardContext, settings: settings.snapshot())

        // Transform candidate case based on keyboardCase
        let suggestions = SuggestionCaseTransformer.transform(
            autocompleteContext.suggestions,
            composingText: composingManager.composingText,
            keyboardCase: keyboardContext.keyboardCase,
            inputMode: p.settings.inputMode,
        )
        let isTranslateSwapped = keyboardContext.isTranslateSwapped
        let selectedCandidateIndex = composingManager.selectedCandidateIndex
        let theme = CandidateTheme.resolved(
            candidateTextSizeScale: candidateTextSizeScale,
            colorSettings: colorSettings,
        )
        let candidateStyle = Self.candidateStyle(
            for: keyboardContext,
            colorSettings: colorSettings,
            height: theme.height,
        )
        // Distinct from `candidateStyle.isLiquidGlassEnabled`: that flag checks the
        // *candidate bar* background (`candidateBackgroundColor`); this flag checks
        // the *root keyboard* background (`backgroundColor`). Keep them independent.
        let useLiquidGlassBg = keyboardContext.isLiquidGlassEnabled
            && colorSettings.backgroundColor == nil
        let isTPSLayout = p.settings.keyboardLayoutType == .tps
        let orMapsToER = p.settings.isTpsOrMappedToER

        keyboardWithOverlays(
            p: p,
            suggestions: suggestions,
            selectedCandidateIndex: selectedCandidateIndex,
            isTranslateSwapped: isTranslateSwapped,
            candidateStyle: candidateStyle,
            candidateTheme: theme,
            isTPSLayout: isTPSLayout,
            orMapsToER: orMapsToER,
        )
        .candidateTheme(theme)
        .keyboardToolbarStyle(
            Keyboard.ToolbarStyle(
                // Use Liquid Glass pass-through only when enabled AND
                // no custom background color is set; otherwise clear
                // so the external .background() color shows through.
                backgroundColor: useLiquidGlassBg
                    ? .white.opacity(0.001)
                    : .clear,
            ),
        )
        .keyboardViewStyle(
            // Always make KK's internal background transparent so our
            // external .background() controls the color consistently
            // in both the keyboard extension and the preview panel.
            KeyboardViewStyle(
                background: useLiquidGlassBg
                    ? .color(Color.white.opacity(0.001))
                    : .color(.clear),
            ),
        )
        .background(
            useLiquidGlassBg
                ? Color.white.opacity(0.001)
                : (colorSettings.backgroundColor?.color ?? Color.keyboardBackground),
        )
        .onAppear {
            if let mode = initialInputMode {
                currentInputMode = mode
            }
        }
        .onReceive(NotificationCenter.default.publisher(for: UserDefaults.didChangeNotification)) { _ in
            let latest = settings.colorSettings
            if colorSettings != latest {
                colorSettings = latest
            }
            let latestScale = settings.keyFontSizeScale
            if keyFontSizeScale != latestScale {
                keyFontSizeScale = latestScale
            }
            let latestBorderWidth = settings.keyBorderWidth
            if keyBorderWidth != latestBorderWidth {
                keyBorderWidth = latestBorderWidth
            }
            let latestCandidateScale = settings.candidateTextSizeScale
            if candidateTextSizeScale != latestCandidateScale {
                candidateTextSizeScale = latestCandidateScale
            }
        }
    }

    // MARK: - Keyboard with Overlays

    /// Core keyboard + overlay panels + state change handlers.
    /// Extracted from body to reduce type-checker complexity.
    private func keyboardWithOverlays(
        p: RenderProviders,
        suggestions: [Autocomplete.Suggestion],
        selectedCandidateIndex: Int,
        isTranslateSwapped: Bool,
        candidateStyle: CandidateView.Style,
        candidateTheme: CandidateTheme,
        isTPSLayout: Bool,
        orMapsToER: Bool,
    ) -> some View {
        coreKeyboard(
            p: p,
            suggestions: suggestions,
            selectedCandidateIndex: selectedCandidateIndex,
            isTranslateSwapped: isTranslateSwapped,
            candidateStyle: candidateStyle,
            isTPSLayout: isTPSLayout,
            orMapsToER: orMapsToER,
        )
        .withKeyboardOverlays(
            panels: $panels,
            expandState: expandState,
            suggestions: suggestions,
            selectedCandidateIndex: selectedCandidateIndex,
            onSuggestionTap: onSuggestionTap,
            isTranslateSwapped: isTranslateSwapped,
            onTranslateToggle: onTranslateToggle,
            candidateStyle: candidateStyle,
            candidateTheme: candidateTheme,
            isTPSLayout: isTPSLayout,
            orMapsToER: orMapsToER,
            onSymbolInsert: { [keyboardContext] symbol in
                keyboardContext.textDocumentProxy.insertText(symbol)
            },
            onOpenSettingsApp: { [unowned services] in
                services.actionHandler.handle(.settings)
            },
        )
        .onChange(of: expandState.isExpanded) { _, isExpanded in
            if isExpanded {
                panels.closeAll()
            }
        }
        .onChange(of: composingManager.isComposing) { _, isComposing in
            guard isComposing else { return }
            if panels.isSymbolExpanded { panels.isSymbolExpanded = false }
            if panels.isSettingsExpanded { panels.isSettingsExpanded = false }
        }
    }

    // MARK: - Core Keyboard View

    /// Builds the KeyboardView with button content, style, and toolbar.
    /// Extracted from body to reduce type-checker complexity.
    private func coreKeyboard(
        p: RenderProviders,
        suggestions: [Autocomplete.Suggestion],
        selectedCandidateIndex: Int,
        isTranslateSwapped: Bool,
        candidateStyle: CandidateView.Style,
        isTPSLayout: Bool,
        orMapsToER: Bool,
    ) -> some View {
        // KeyboardKit 10: uses layout: and services: parameters
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
                    inputMode: p.settings.inputMode,
                )
            },
            buttonView: { params in
                let borderWidth = keyBorderWidth
                if borderWidth > 0, params.item.action != .none {
                    params.view.overlay(
                        RoundedRectangle(cornerRadius: p.settings.keyCornerRadius)
                            .strokeBorder(Color.black, lineWidth: borderWidth)
                            .padding(params.item.edgeInsets),
                    )
                } else {
                    params.view
                }
            },
            collapsedView: { $0.view },
            emojiKeyboard: { _ in
                // KeyboardKit 10: ISEmojiView requires explicit height
                emojiKeyboardView()
                    .frame(height: layout.totalHeight)
            },
            toolbar: { params in
                // Unified CandidateView; English mode passes in KeyboardKit's default view
                CandidateView(
                    suggestions: suggestions,
                    selectedCandidateIndex: selectedCandidateIndex,
                    onSuggestionTap: onSuggestionTap,
                    isTranslateSwapped: isTranslateSwapped,
                    onSettingsTap: {
                        let wasOpen = panels.isSettingsExpanded
                        panels.closeAll()
                        expandState.collapse()
                        if !wasOpen { panels.isSettingsExpanded = true }
                    },
                    onLayoutTap: {
                        let wasOpen = panels.isLayoutExpanded
                        panels.closeAll()
                        expandState.collapse()
                        if !wasOpen { panels.isLayoutExpanded = true }
                    },
                    onSymbolTap: {
                        let wasOpen = panels.isSymbolExpanded
                        panels.closeAll()
                        expandState.collapse()
                        if !wasOpen { panels.isSymbolExpanded = true }
                    },
                    onDismissKeyboard: { [unowned services] in
                        panels.closeAll()
                        services.actionHandler.handle(.dismissKeyboard)
                    },
                    currentInputMode: currentInputMode,
                    onInputModeChange: { newMode in
                        currentInputMode = newMode
                        settings.inputMode = newMode
                        panels.closeAll()
                    },
                    englishAutocompleteView: currentInputMode == .english ? AnyView(params.view) : nil,
                    isComposing: composingManager.isComposing,
                    isTPSLayout: isTPSLayout,
                    orMapsToER: orMapsToER,
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
            let colors = colorSettings
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
        .keyboardCalloutActions(Callouts.taigiCalloutActions)
        .keyboardCalloutStyle(calloutStyle)
    }

    private static func candidateStyle(
        for context: KeyboardContext,
        colorSettings: KeyboardColorSettings,
        height: CGFloat,
    ) -> CandidateView.Style {
        var style = CandidateView.Style.adaptive(for: context)
        style.height = height
        if let bg = colorSettings.candidateBackgroundColor?.color {
            style.backgroundColor = bg
        }
        return style
    }
}
