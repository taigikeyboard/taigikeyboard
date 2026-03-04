import KeyboardKit
import SwiftUI

/// Appearance settings subpage
///
/// Provides font selection, 4 sliders for key height, key font size, candidate text size,
/// and key corner radius, with a keyboard preview anchored at the bottom.
struct AppearanceSettingsView: View {
    @StateObject private var languageManager = LanguageManager.shared
    @StateObject private var fontManager = FontManager.shared
    @Environment(\.colorScheme) private var colorScheme

    private let settings = SharedSettings.shared

    @State private var keyHeightScale: Double
    @State private var keyFontSizeScale: Double
    @State private var candidateTextSizeScale: Double
    @State private var keyCornerRadius: Double
    @State private var keyBorderWidth: Double
    @State private var selectedFontType: FontType

    // Color state
    @State private var keyboardBackground: Color
    @State private var keyText: Color
    @State private var normalKeyFill: Color
    @State private var specialKeyFill: Color
    @State private var candidateText: Color
    @State private var candidateBackground: Color

    /// Tracks which colors have been explicitly customized (non-nil = customized).
    @State private var savedColors: KeyboardColorSettings

    private static let defaultKeyboardBackground = Color.keyboardBackground
    private static let defaultKeyText = Color.keyboardButtonForeground
    private static let defaultNormalKeyFill = Color.keyboardButtonBackground
    private static let defaultSpecialKeyFill = Color.keyboardDarkButtonBackground
    private static let defaultCandidateText = Color(.label)
    private static let defaultCandidateBackground = Color.keyboardBackground

    private static let defaultKeyHeightScale: Double = 1.0
    private static let defaultKeyFontSizeScale: Double = 1.0
    private static let defaultCandidateTextSizeScale: Double = 1.0
    private static let defaultKeyCornerRadius: Double = 6.0
    private static let defaultKeyBorderWidth: Double = 0
    private static let defaultFontType: FontType = .openHuninn

    private let scaleRange: ClosedRange<Double> = 0.85...1.15
    private let scaleStep: Double = 0.01
    private let radiusRange: ClosedRange<Double> = 0...15
    private let radiusStep: Double = 0.5
    private let borderWidthRange: ClosedRange<Double> = 0...3
    private let borderWidthStep: Double = 0.5

    init() {
        let s = SharedSettings.shared
        _keyHeightScale = State(initialValue: s.keyHeightScale)
        _keyFontSizeScale = State(initialValue: s.keyFontSizeScale)
        _candidateTextSizeScale = State(initialValue: s.candidateTextSizeScale)
        _keyCornerRadius = State(initialValue: s.keyCornerRadius)
        _keyBorderWidth = State(initialValue: s.keyBorderWidth)
        _selectedFontType = State(initialValue: s.fontType)

        let c = s.colorSettings
        _keyboardBackground = State(initialValue: c.backgroundColor?.color ?? Self.defaultKeyboardBackground)
        _keyText = State(initialValue: c.keyTextColor?.color ?? Self.defaultKeyText)
        _normalKeyFill = State(initialValue: c.normalKeyFillColor?.color ?? Self.defaultNormalKeyFill)
        _specialKeyFill = State(initialValue: c.specialKeyFillColor?.color ?? Self.defaultSpecialKeyFill)
        _candidateText = State(initialValue: c.candidateTextColor?.color ?? Self.defaultCandidateText)
        _candidateBackground = State(initialValue: c.candidateBackgroundColor?.color ?? Self.defaultCandidateBackground)
        _savedColors = State(initialValue: c)
    }

    var body: some View {
        VStack(spacing: 0) {
            Form {
                // Font picker
                Section {
                    NavigationLink {
                        AppearanceFontPickerView(
                            selectedFont: $selectedFontType,
                            onChange: { newValue in
                                fontManager.updateFontType(newValue)
                            }
                        )
                    } label: {
                        HStack {
                            Text(languageManager.text(Tab2Texts.customFont))
                            Spacer()
                            Text(fontDisplayName(selectedFontType))
                                .foregroundColor(.secondary)
                        }
                    }
                }

                // Keyboard overall: background color + height
                Section(header: Text(languageManager.text(Tab2Texts.keyboardSection))) {
                    colorRow(
                        label: languageManager.text(Tab2Texts.colorKeyboardBackground),
                        color: $keyboardBackground,
                        defaultColor: Self.defaultKeyboardBackground,
                        keyPath: \.backgroundColor
                    )
                    scaleSliderRow(
                        label: languageManager.text(Tab2Texts.keyHeight),
                        value: $keyHeightScale,
                        defaultValue: Self.defaultKeyHeightScale,
                        onChanged: { settings.keyHeightScale = $0 }
                    )
                }

                // Key section: colors + font size + corner radius + border width
                Section(header: Text(languageManager.text(Tab2Texts.colorKeySection))) {
                    colorRow(
                        label: languageManager.text(Tab2Texts.colorKeyText),
                        color: $keyText,
                        defaultColor: Self.defaultKeyText,
                        keyPath: \.keyTextColor
                    )
                    colorRow(
                        label: languageManager.text(Tab2Texts.colorNormalKeyFill),
                        color: $normalKeyFill,
                        defaultColor: Self.defaultNormalKeyFill,
                        keyPath: \.normalKeyFillColor
                    )
                    colorRow(
                        label: languageManager.text(Tab2Texts.colorSpecialKeyFill),
                        color: $specialKeyFill,
                        defaultColor: Self.defaultSpecialKeyFill,
                        keyPath: \.specialKeyFillColor
                    )
                    scaleSliderRow(
                        label: languageManager.text(Tab2Texts.keyFontSize),
                        value: $keyFontSizeScale,
                        defaultValue: Self.defaultKeyFontSizeScale,
                        onChanged: { settings.keyFontSizeScale = $0 }
                    )
                    radiusSliderRow()
                    borderWidthSliderRow()
                }

                // Candidate section: colors + text size
                Section(header: Text(languageManager.text(Tab2Texts.candidateSection))) {
                    colorRow(
                        label: languageManager.text(Tab2Texts.colorCandidateText),
                        color: $candidateText,
                        defaultColor: Self.defaultCandidateText,
                        keyPath: \.candidateTextColor
                    )
                    colorRow(
                        label: languageManager.text(Tab2Texts.colorCandidateBackground),
                        color: $candidateBackground,
                        defaultColor: Self.defaultCandidateBackground,
                        keyPath: \.candidateBackgroundColor
                    )
                    scaleSliderRow(
                        label: languageManager.text(Tab2Texts.candidateTextSize),
                        value: $candidateTextSizeScale,
                        defaultValue: Self.defaultCandidateTextSizeScale,
                        onChanged: { settings.candidateTextSizeScale = $0 }
                    )
                }

                // Reset all appearance settings
                Section {
                    Button(role: .destructive) {
                        UIImpactFeedbackGenerator(style: .medium).impactOccurred()
                        resetAllAppearance()
                    } label: {
                        Text(languageManager.text(Tab2Texts.appearanceResetAll))
                    }
                }
            }

            // Keyboard preview anchored at bottom
            KeyboardPreviewPanel(
                keyHeightScale: keyHeightScale,
                keyFontSizeScale: keyFontSizeScale,
                candidateTextSizeScale: candidateTextSizeScale,
                keyCornerRadius: keyCornerRadius,
                fontType: selectedFontType,
                colorScheme: colorScheme
            )
        }
        .navigationTitle(languageManager.text(Tab2Texts.appearanceSettings))
        .navigationBarTitleDisplayMode(.inline)
    }

    // MARK: - Scale Slider Row

    @ViewBuilder
    private func scaleSliderRow(
        label: String,
        value: Binding<Double>,
        defaultValue: Double = 1.0,
        onChanged: @escaping (Double) -> Void
    ) -> some View {
        HStack {
            Text(label)
                .fixedSize()
            Slider(value: value, in: scaleRange, step: scaleStep)
                .onChange(of: value.wrappedValue) { _, newValue in
                    onChanged(newValue)
                }
            if value.wrappedValue != defaultValue {
                Button {
                    value.wrappedValue = defaultValue
                    onChanged(defaultValue)
                } label: {
                    Image(systemName: "arrow.counterclockwise")
                        .foregroundColor(.secondary)
                }
                .buttonStyle(.plain)
            }
        }
    }

    // MARK: - Radius Slider Row

    @ViewBuilder
    private func radiusSliderRow() -> some View {
        HStack {
            Text(languageManager.text(Tab2Texts.keyCornerRadius))
                .fixedSize()
            Slider(value: $keyCornerRadius, in: radiusRange, step: radiusStep)
                .onChange(of: keyCornerRadius) { _, newValue in
                    settings.keyCornerRadius = newValue
                }
            if keyCornerRadius != Self.defaultKeyCornerRadius {
                Button {
                    keyCornerRadius = Self.defaultKeyCornerRadius
                    settings.keyCornerRadius = Self.defaultKeyCornerRadius
                } label: {
                    Image(systemName: "arrow.counterclockwise")
                        .foregroundColor(.secondary)
                }
                .buttonStyle(.plain)
            }
        }
    }

    // MARK: - Border Width Slider Row

    @ViewBuilder
    private func borderWidthSliderRow() -> some View {
        HStack {
            Text(languageManager.text(Tab2Texts.keyBorderWidth))
                .fixedSize()
            Slider(value: $keyBorderWidth, in: borderWidthRange, step: borderWidthStep)
                .onChange(of: keyBorderWidth) { _, newValue in
                    settings.keyBorderWidth = newValue
                }
            if keyBorderWidth != Self.defaultKeyBorderWidth {
                Button {
                    keyBorderWidth = Self.defaultKeyBorderWidth
                    settings.keyBorderWidth = Self.defaultKeyBorderWidth
                } label: {
                    Image(systemName: "arrow.counterclockwise")
                        .foregroundColor(.secondary)
                }
                .buttonStyle(.plain)
            }
        }
    }

    // MARK: - Helpers

    private func fontDisplayName(_ font: FontType) -> String {
        switch font {
        case .system: return languageManager.text(Tab2Texts.fontSystemDefault)
        case .openHuninn: return languageManager.text(Tab2Texts.fontOpenHuninn)
        case .iansui: return languageManager.text(Tab2Texts.fontIansui)
        }
    }

    private func formatPercent(_ value: Double) -> String {
        "\(Int(round(value * 100)))%"
    }

    // MARK: - Color Row

    @ViewBuilder
    private func colorRow(
        label: String,
        color: Binding<Color>,
        defaultColor: Color,
        keyPath: WritableKeyPath<KeyboardColorSettings, CodableColor?>
    ) -> some View {
        HStack {
            ColorPicker(label, selection: color, supportsOpacity: false)
                .onChange(of: color.wrappedValue) { _, newValue in
                    var cs = settings.colorSettings
                    cs[keyPath: keyPath] = CodableColor(newValue)
                    settings.colorSettings = cs
                    savedColors = cs
                }

            if savedColors[keyPath: keyPath] != nil {
                Button {
                    color.wrappedValue = defaultColor
                    var cs = settings.colorSettings
                    cs[keyPath: keyPath] = nil
                    settings.colorSettings = cs
                    savedColors = cs
                } label: {
                    Image(systemName: "arrow.counterclockwise")
                        .foregroundColor(.secondary)
                }
                .buttonStyle(.plain)
            }
        }
    }

    // MARK: - Reset All

    private func resetAllAppearance() {
        // Reset font
        selectedFontType = Self.defaultFontType
        fontManager.updateFontType(Self.defaultFontType)

        // Reset sliders
        keyHeightScale = Self.defaultKeyHeightScale
        settings.keyHeightScale = Self.defaultKeyHeightScale
        keyFontSizeScale = Self.defaultKeyFontSizeScale
        settings.keyFontSizeScale = Self.defaultKeyFontSizeScale
        candidateTextSizeScale = Self.defaultCandidateTextSizeScale
        settings.candidateTextSizeScale = Self.defaultCandidateTextSizeScale
        keyCornerRadius = Self.defaultKeyCornerRadius
        settings.keyCornerRadius = Self.defaultKeyCornerRadius
        keyBorderWidth = Self.defaultKeyBorderWidth
        settings.keyBorderWidth = Self.defaultKeyBorderWidth

        // Reset colors
        keyboardBackground = Self.defaultKeyboardBackground
        keyText = Self.defaultKeyText
        normalKeyFill = Self.defaultNormalKeyFill
        specialKeyFill = Self.defaultSpecialKeyFill
        candidateText = Self.defaultCandidateText
        candidateBackground = Self.defaultCandidateBackground
        settings.colorSettings = .default
        savedColors = .default
    }
}

// MARK: - Font Picker Subpage

private struct AppearanceFontPickerView: View {
    @StateObject private var languageManager = LanguageManager.shared
    @Binding var selectedFont: FontType
    var onChange: (FontType) -> Void

    private let options: [(font: FontType, text: LocalizedText)] = [
        (.system, Tab2Texts.fontSystemDefault),
        (.openHuninn, Tab2Texts.fontOpenHuninn),
        (.iansui, Tab2Texts.fontIansui)
    ]

    var body: some View {
        Form {
            Section {
                ForEach(options, id: \.font) { option in
                    Button {
                        selectedFont = option.font
                        onChange(option.font)
                    } label: {
                        HStack {
                            Text(languageManager.text(option.text))
                                .foregroundColor(.primary)
                            Spacer()
                            if selectedFont == option.font {
                                Image(systemName: "checkmark")
                                    .foregroundColor(.accentColor)
                            }
                        }
                    }
                }
            }
        }
        .navigationTitle(languageManager.text(Tab2Texts.customFont))
        .navigationBarTitleDisplayMode(.inline)
    }
}

// MARK: - Keyboard Preview (using real TaigiKeyboardView)

/// A display-only keyboard preview that renders the real `TaigiKeyboardView`.
/// All appearance settings (key height, font size, candidate text size, corner radius, font)
/// propagate automatically through `SharedSettings` → `CustomLayoutService` / `ButtonFontProvider`.
private struct KeyboardPreviewPanel: View {
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

        TaigiKeyboardView(
            services: services,
            layout: layout,
            emojiKeyboardView: { AnyView(EmptyView()) },
            calloutStyle: Self.createCalloutStyle(fontType: fontType),
            autocompleteContext: previewState.autocompleteContext,
            keyboardContext: previewState.keyboardContext,
            composingManager: composingManager,
            onSuggestionTap: { _ in },
            onTranslateToggle: { },
            initialInputMode: SharedSettings.shared.inputMode == .english ? .tl : nil
        )
        .keyboardState(previewState)
        .onAppear { configurePreviewContext() }
        .onChange(of: colorScheme) { _, newValue in
            previewState.keyboardContext.colorScheme = newValue
        }
    }

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

    private static func createCalloutStyle(fontType: FontType) -> Callouts.CalloutStyle {
        switch fontType {
        case .system:
            return .standard
        case .openHuninn:
            let name = KeyboardModels.Fonts.openHuninnFontName
            return Callouts.CalloutStyle(
                actionItemFont: KeyboardFont.custom(name, size: 20, weight: .regular),
                inputItemFont: KeyboardFont.custom(name, size: 32, weight: .light)
            )
        case .iansui:
            let name = KeyboardModels.Fonts.iansuiFontName
            return Callouts.CalloutStyle(
                actionItemFont: KeyboardFont.custom(name, size: 20, weight: .regular),
                inputItemFont: KeyboardFont.custom(name, size: 32, weight: .light)
            )
        }
    }
}
