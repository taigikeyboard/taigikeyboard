// 中文: 鍵盤擴充的最外層 SwiftUI View — 拼起候選列、KeyboardKit 鍵盤本體、四個 overlay 與背景。
// 中文: 變動的設定值(顏色 / 字型大小 / 邊框)透過 UserDefaults didChange 即時 sync 進 @State。

import KeyboardKit
import SwiftUI

// 中文: 主鍵盤 View,組裝 CandidateView + KeyboardView + Overlay + 背景。
struct TaigiKeyboardView: View {
    let settings: any KeyboardEnvironment
    let services: Keyboard.Services
    let layout: KeyboardLayout
    let emojiKeyboardView: () -> AnyView
    let calloutStyle: Callouts.CalloutStyle

    @ObservedObject var autocompleteContext: AutocompleteContext
    @ObservedObject var keyboardContext: KeyboardContext
    let composingManager: ComposingManager

    let onSuggestionTap: (Autocomplete.Suggestion) -> Void
    let onTranslateToggle: () -> Void
    var initialInputMode: InputMode?

    @StateObject private var expandState = CandidateExpandState()
    @State private var currentInputMode: InputMode
    @State private var panels = OverlayPanelState()
    // 中文: 設定編輯軸的 re-render 觸發器。colorScheme 變動由 keyboardContext(@ObservedObject)
    // 中文: 自動觸發;但 host app 改顏色/主題時 keyboardContext 不變,靠 didChange bump 此值強制重繪。
    @State private var settingsRevision = 0

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
        let useLiquidGlassBg = keyboardContext.isLiquidGlassEnabled
            && colors.backgroundColor == nil
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
                : (colors.backgroundColor?.color ?? Color.keyboardBackground),
        )
        // 中文: 把主題解析所用的 colorScheme 灌進 environment,讓仍讀 @Environment(\.colorScheme)
        // 中文: 的子 view(CandidateView / CandidateButtonView)與 resolver 同源,避免淺/深色混色。
        .environment(\.colorScheme, keyboardContext.colorScheme)
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
            // 中文: 重新解析主題外觀(selectedThemeId / colorSettings / 各尺寸 / fontType / themeRevision 任一變更皆觸發)。
            // 中文: 尺寸/字型/邊框/陰影全走 settings.snapshot(per-theme resolved),不再各自 @State 鏡像。
            settingsRevision &+= 1
        }
    }

    // MARK: - Keyboard with Overlays

    /// Core keyboard + overlay panels + state change handlers.
    /// Extracted from body to reduce type-checker complexity.
    // 中文: 把核心鍵盤 + 四個 overlay + 狀態 onChange 串在一起,從 body 抽出來壓低 type-checker 複雜度。
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
    // 中文: 組出 KeyboardKit KeyboardView 主體 — 按鍵內容 / 樣式 / candidate toolbar / callout。
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
                let borderWidth = p.settings.keyBorderWidth
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
        .keyboardCalloutActions(Callouts.taigiCalloutActions)
        .keyboardCalloutStyle(calloutStyle)
    }

    // 中文: 依當前 keyboardContext 與顏色設定組合出 CandidateView 樣式,套用使用者選的高度與背景。
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
