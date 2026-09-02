// 中文: 鍵盤擴充的最外層 SwiftUI View — 拼起候選列、KeyboardKit 鍵盤本體、四個 overlay 與背景。
// 中文: 變動的設定值(顏色 / 字型大小 / 邊框)透過 UserDefaults didChange 即時 sync 進 @State。

import KeyboardKit
import SwiftUI

// 中文: 主鍵盤 View,組裝 CandidateView + KeyboardView + Overlay + 背景。
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
    // 中文: 設定編輯軸的 re-render 觸發器。colorScheme 變動由 keyboardContext(@ObservedObject)
    // 中文: 自動觸發;但 host app 改顏色/主題時 keyboardContext 不變,靠 didChange bump 此值強制重繪。
    @State private var settingsRevision = 0
    // 中文: extension 程序自己的 i18n 顯示語言 store(host 是另一程序)。view-owned @State 讓直接建構
    // 中文: TaigiKeyboardView 的路徑(如 KeyboardPreviewPanel)自帶 store;注入 environment 供 overlay 讀取。
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
    // 中文: 一次 render 內共用的 settings snapshot 與 button content 各 provider,避免重複讀 UserDefaults。
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
        // 中文: 建立 settingsRevision → body 的失效邊。body 每次重算顏色(無快取),colorScheme 軸由
        // 中文: keyboardContext(@ObservedObject)自動觸發;設定編輯軸則靠 didChange bump 此值。必須在 body
        // 中文: 讀取它,re-render 才保證觸發(避免依賴「@State 寫入即失效」此一未明確保證的行為)。
        // 中文: 必須用 `let _ =` 宣告形式 — ViewBuilder body 不接受裸 `_ =` 運算式(會被當成 View)。
        let _ = settingsRevision
        // 中文: 單次解析,colorScheme 取自 keyboardContext(KK 已隨系統 trait 同步)。
        // 中文: 六個顏色 sink 全讀這份 p.settings.colorSettings,避免重複解析或來源分裂。
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
        // 中文: 把主題解析所用的 colorScheme 灌進 environment,讓仍讀 @Environment(\.colorScheme)
        // 中文: 的子 view(CandidateView / CandidateButtonView)與 resolver 同源,避免淺/深色混色。
        .environment(\.colorScheme, keyboardContext.colorScheme)
        // 中文: 把 extension 的顯示語言 store 灌進 environment;overlay 以 @Environment(DisplayLanguageStore.self)
        // 中文: 讀取並 live-switch。套在 overlay 已掛載之後仍會傳入(與上面 colorScheme 同路徑)。
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
            // 中文: host app 改設定 → bump settingsRevision 強制重繪;body 以 live keyboardContext.colorScheme
            // 中文: 重新解析主題外觀(selectedThemeId / colorSettings / 各尺寸 / themeRevision 任一變更皆觸發)。
            // 中文: 尺寸/邊框/陰影走 settings.snapshot(per-theme resolved);字型為全域設定亦由 snapshot 帶入。不再各自 @State 鏡像。
            settingsRevision &+= 1
            // 中文: 同程序內盡力同步顯示語言。跨程序(host 改語言)的可靠重讀點在 overlay .onAppear,因
            // 中文: UserDefaults.didChangeNotification 依 Apple 契約只在本程序寫入時送出,外部程序寫入不保證觸發
            // 中文: (Codex pre-impl F1c)。此處冪等:語言未變則 store didSet 不動作。
            displayLanguageStore.syncFromSettings()
        }
    }

    // MARK: - Keyboard with Overlays

    /// Core keyboard + overlay panels + state change handlers.
    /// Extracted from body to reduce type-checker complexity.
    // 中文: 把核心鍵盤 + 四個 overlay + 狀態 onChange 串在一起,從 body 抽出來壓低 type-checker 複雜度。
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
    // 中文: 組出 KeyboardKit KeyboardView 主體 — 按鍵內容 / 樣式 / candidate toolbar / callout。
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

    // 中文: 依當前 keyboardContext 與顏色設定組合出 CandidateView 樣式,套用使用者選的高度與背景。
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
