// Outermost SwiftUI view of the keyboard extension — candidate bar, the KeyboardKit keyboard,
// four overlays and the background. Live setting changes (colors / font size / borders) sync into
// @State via the UserDefaults didChange notification.

import KeyboardKit
import SwiftUI

struct TaigiKeyboardView: View {
    let settings: any KeyboardEnvironment
    let services: KeyboardServices
    let layout: KeyboardLayout
    let emojiKeyboardView: () -> AnyView
    let calloutStyle: KeyboardCalloutStyle

    @ObservedObject var autocompleteContext: AutocompleteContext
    @ObservedObject var keyboardContext: KeyboardContext
    let composingManager: ComposingManager

    let onSuggestionTap: (AutocompleteSuggestion) -> Void
    let onTranslateToggle: () -> Void
    let onCandidateDisplayModeChange: (CandidateDisplayMode) -> Void
    var initialInputMode: InputMode?

    @StateObject private var expandState = CandidateExpandState()
    @State private var currentInputMode: InputMode
    @State private var panels = OverlayPanelState()
    // Re-render trigger for the settings-edit axis. A colorScheme change already comes through
    // `keyboardContext`, but a host-app color/theme edit leaves it untouched, so didChange bumps this.
    @State private var settingsRevision = 0
    // This extension process owns its own display-language store (the host is a separate process).
    // View-owned @State so paths that build TaigiKeyboardView directly (e.g. KeyboardPreviewPanel)
    // carry one; injected into the environment for the overlays to read.
    @State private var displayLanguageStore = DisplayLanguageStore()

    init(
        settings: any KeyboardEnvironment,
        services: KeyboardServices,
        layout: KeyboardLayout,
        emojiKeyboardView: @escaping () -> AnyView,
        calloutStyle: KeyboardCalloutStyle,
        autocompleteContext: AutocompleteContext,
        keyboardContext: KeyboardContext,
        composingManager: ComposingManager,
        onSuggestionTap: @escaping (AutocompleteSuggestion) -> Void,
        onTranslateToggle: @escaping () -> Void,
        onCandidateDisplayModeChange: @escaping (CandidateDisplayMode) -> Void,
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
        self.onCandidateDisplayModeChange = onCandidateDisplayModeChange
        self.initialInputMode = initialInputMode
        _currentInputMode = State(initialValue: settings.inputMode)
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
        // Invalidation edge settingsRevision → body. Reading it inside body is what guarantees the
        // re-render; relying on "a @State write invalidates" alone is not a documented guarantee.
        // The `let _ =` declaration form is required — ViewBuilder rejects a bare `_ =` expression.
        let _ = settingsRevision
        // Resolved once from keyboardContext's colorScheme; all six color sinks read this one
        // `p.settings.colorSettings` so there is no duplicate resolution or split source.
        let p = RenderProviders(
            keyboardContext: keyboardContext,
            settings: settings.snapshot(for: keyboardContext.colorScheme),
        )
        let colors = p.settings.colorSettings

        // Transform candidate case based on keyboardCase
        let suggestions = SuggestionCaseTransformer.transform(
            autocompleteContext.suggestions,
            composingText: composingManager.composingText,
            keyboardCase: keyboardContext.keyboardCase,
            inputMode: p.settings.inputMode,
        )
        let isTranslateSwapped = keyboardContext.isTranslateSwapped
        // Read beside the swap on the same live path: both come from SharedSettings via the
        // KeyboardContext extension, so a mode change re-renders exactly like a swap does.
        let candidateDisplayMode = keyboardContext.candidateDisplayMode
        let selectedCandidateIndex = composingManager.selectedCandidateIndex
        let theme = CandidateTheme.resolved(
            candidateTextSizeScale: p.settings.candidateTextSizeScale,
            colorSettings: colors,
        )
        let candidateStyle = Self.candidateStyle(
            for: keyboardContext,
            colorSettings: colors,
            height: theme.height,
        )
        // Distinct from `candidateStyle.isLiquidGlassEnabled`: that flag checks the
        // *candidate bar* background (`candidateBackgroundColor`); this flag checks
        // the *root keyboard* background (`backgroundColor`). Keep them independent.
        // A gradient theme owns the whole background → never hand it to Liquid Glass.
        let useLiquidGlassBg = keyboardContext.isLiquidGlassEnabled
            && colors.backgroundColor == nil
            && !colors.hasBackgroundGradient
        let isTPSLayout = p.settings.keyboardLayoutType == .tps
        let orMapsToER = p.settings.isTpsOrMappedToER

        keyboardWithOverlays(
            p: p,
            suggestions: suggestions,
            selectedCandidateIndex: selectedCandidateIndex,
            isTranslateSwapped: isTranslateSwapped,
            candidateDisplayMode: candidateDisplayMode,
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
        .background {
            // Gradient themes paint a single top→bottom gradient spanning the
            // candidate bar down to the keyboard bottom (candidate bar is made
            // transparent in `candidateStyle`). Flat themes keep today's solid fill.
            if useLiquidGlassBg {
                Color.white.opacity(0.001)
            } else if colors.hasBackgroundGradient, let gradient = colors.backgroundGradient {
                LinearGradient(
                    colors: gradient.stops.map(\.color),
                    startPoint: .top,
                    endPoint: .bottom,
                )
            } else {
                colors.backgroundColor?.color ?? Color.keyboardBackground
            }
        }
        // Publish the colorScheme used for theme resolution so child views still reading
        // @Environment(\.colorScheme) (CandidateView / CandidateButtonView) share its source.
        .environment(\.colorScheme, keyboardContext.colorScheme)
        // Publish the display-language store; overlays read it via @Environment and live-switch.
        // Applying it after the overlays are mounted still propagates (same path as colorScheme above).
        .environment(displayLanguageStore)
        .onAppear {
            if let mode = initialInputMode {
                currentInputMode = mode
            }
        }
        .onReceive(
            NotificationCenter.default
                .publisher(for: UserDefaults.didChangeNotification)
                .receive(on: DispatchQueue.main),
        ) { _ in
            // A host-app settings edit bumps settingsRevision to force a redraw; body re-resolves the
            // theme from the live colorScheme. Sizes / borders / shadows / font all come from
            // `settings.snapshot` (per-theme resolved) rather than separate @State mirrors.
            settingsRevision &+= 1
            // Best-effort in-process language sync. Apple only contracts didChangeNotification for
            // writes from this process, so the reliable cross-process re-read is the overlay's
            // .onAppear. Idempotent: an unchanged language leaves the store's didSet inert.
            displayLanguageStore.syncFromSettings()
        }
    }

    // MARK: - Keyboard with Overlays

    /// Core keyboard + overlay panels + state change handlers.
    /// Extracted from body to reduce type-checker complexity.
    private func keyboardWithOverlays(
        p: RenderProviders,
        suggestions: [AutocompleteSuggestion],
        selectedCandidateIndex: Int,
        isTranslateSwapped: Bool,
        candidateDisplayMode: CandidateDisplayMode,
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
            candidateDisplayMode: candidateDisplayMode,
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
            candidateDisplayMode: candidateDisplayMode,
            onTranslateToggle: onTranslateToggle,
            onCandidateDisplayModeChange: onCandidateDisplayModeChange,
            candidateStyle: candidateStyle,
            candidateTheme: candidateTheme,
            isTPSLayout: isTPSLayout,
            orMapsToER: orMapsToER,
            onSymbolInsert: { [keyboardContext, unowned services] symbol in
                (services.actionHandler as? ActionHandler)?.beginInputEvent()
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
        suggestions: [AutocompleteSuggestion],
        selectedCandidateIndex: Int,
        isTranslateSwapped: Bool,
        candidateDisplayMode: CandidateDisplayMode,
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
                // The emoji-switch key renders KeyboardKit's `keyboardEmoji` TEMPLATE
                // asset, which ships a dark-luminosity variant selected by `\.colorScheme`.
                // Under a themed keyboard the variant must follow the THEME palette, not the
                // system appearance: a light theme in dark mode would otherwise pick the dark
                // (filled) variant and the dark keyText tint renders it as a black blob. Force
                // only this key's content scheme to the palette-matched value; every other key
                // keeps the system colorScheme (the keyboard-root environment, line ~172).
                .environment(
                    \.colorScheme,
                    Self.emojiAssetColorScheme(for: params.item.action, colors: p.settings.colorSettings)
                        ?? keyboardContext.colorScheme,
                )
            },
            buttonView: { params in
                let borderWidth = p.settings.keyBorderWidth
                if borderWidth > 0, params.item.action != .none {
                    // Border follows the key text color (role-first → adaptive
                    // Color(.label) when the theme leaves keyTextColor nil), so the
                    // 框線 outline stays visible on both light and adaptive-dark
                    // backgrounds (a hardcoded black border vanishes in dark mode).
                    params.view.overlay(
                        RoundedRectangle(cornerRadius: p.settings.keyCornerRadius)
                            .strokeBorder(p.keyTextColor, lineWidth: borderWidth)
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
                    candidateDisplayMode: candidateDisplayMode,
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

            // Per-theme key shadow, three-state (snapshot `keyShadowIntensity` is optional):
            //   nil → leave KK's standard button shadow (= HEAD; default + built-in themes).
            //   0   → explicit no shadow (a user theme whose shadow slider is at 0 → flat).
            //   >0  → explicit per-theme shadow of that point size.
            // The user-theme slider owns 0 = flat; the default theme must keep KK's standard
            // shadow, so its snapshot reports nil and we never touch `style.shadow`.
            // KK API: Keyboard.ButtonStyle.shadow / Keyboard.ButtonShadowStyle.noShadow.
            if let intensity = p.settings.keyShadowIntensity {
                style.shadow = intensity > 0
                    ? Keyboard.ButtonShadowStyle(color: .keyboardButtonShadow, size: intensity)
                    : .noShadow
            }

            // Apply resolved theme colors (single source: the per-render snapshot).
            let colors = p.settings.colorSettings
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
        .keyboardCalloutActions(TaigiCallouts.taigiCalloutActions)
        .keyboardCalloutStyle(calloutStyle)
    }

    /// The colorScheme the emoji-switch key's content should render in, so KeyboardKit's
    /// `keyboardEmoji` template asset picks the variant matching the active theme's palette
    /// instead of the system appearance. Returns nil for non-emoji keys and for the
    /// default/adaptive theme (keyTextColor nil) — both keep the system colorScheme.
    /// Dark keyText ⇒ light-palette theme ⇒ `.light` (light asset variant); light keyText
    /// (e.g. 暗眠山貓) ⇒ `.dark` (dark variant, correctly tinted light on a dark keycap).
    private static func emojiAssetColorScheme(
        for action: KeyboardAction,
        colors: KeyboardColorSettings,
    ) -> ColorScheme? {
        guard action == .keyboardType(.emojis), let keyText = colors.keyTextColor else { return nil }
        return keyText.isDark ? .light : .dark
    }

    private static func candidateStyle(
        for context: KeyboardContext,
        colorSettings: KeyboardColorSettings,
        height: CGFloat,
    ) -> CandidateView.Style {
        var style = CandidateView.Style.adaptive(for: context)
        style.height = height
        // Gradient themes: the candidate bar goes transparent so the root gradient
        // shows through candidate→bottom as one continuous fill (takes precedence
        // over any explicit candidate background). Flat themes keep their bar color.
        if colorSettings.hasBackgroundGradient {
            style.backgroundColor = .clear
        } else if let bg = colorSettings.candidateBackgroundColor?.color {
            style.backgroundColor = bg
        }
        return style
    }
}
