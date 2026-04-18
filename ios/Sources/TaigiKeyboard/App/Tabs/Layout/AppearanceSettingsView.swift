import KeyboardKit
import SwiftUI

/// Appearance settings subpage
///
/// Provides font selection, 4 sliders for key height, key font size, candidate text size,
/// and key corner radius, with a keyboard preview anchored at the bottom.
struct AppearanceSettingsView: View {
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

    private let scaleRange: ClosedRange<Double> = 0.85 ... 1.15
    private let scaleStep: Double = 0.01
    private let radiusRange: ClosedRange<Double> = 0 ... 15
    private let radiusStep: Double = 0.5
    private let borderWidthRange: ClosedRange<Double> = 0 ... 3
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
                                settings.fontType = newValue
                            },
                        )
                    } label: {
                        HStack {
                            Text(Tab2Texts.customFont)
                            Spacer()
                            Text(fontDisplayName(selectedFontType))
                                .foregroundColor(.secondary)
                        }
                    }
                }

                // Keyboard overall: background color + height
                Section(header: Text(Tab2Texts.keyboardSection)) {
                    colorRow(
                        label: Tab2Texts.colorKeyboardBackground,
                        color: $keyboardBackground,
                        defaultColor: Self.defaultKeyboardBackground,
                        keyPath: \.backgroundColor,
                    )
                    sliderRow(
                        label: Tab2Texts.keyHeight,
                        value: $keyHeightScale,
                        in: scaleRange, step: scaleStep,
                        defaultValue: Self.defaultKeyHeightScale,
                        onChanged: { settings.keyHeightScale = $0 },
                    )
                }

                // Key section: colors + font size + corner radius + border width
                Section(header: Text(Tab2Texts.colorKeySection)) {
                    colorRow(
                        label: Tab2Texts.colorKeyText,
                        color: $keyText,
                        defaultColor: Self.defaultKeyText,
                        keyPath: \.keyTextColor,
                    )
                    colorRow(
                        label: Tab2Texts.colorNormalKeyFill,
                        color: $normalKeyFill,
                        defaultColor: Self.defaultNormalKeyFill,
                        keyPath: \.normalKeyFillColor,
                    )
                    colorRow(
                        label: Tab2Texts.colorSpecialKeyFill,
                        color: $specialKeyFill,
                        defaultColor: Self.defaultSpecialKeyFill,
                        keyPath: \.specialKeyFillColor,
                    )
                    sliderRow(
                        label: Tab2Texts.keyFontSize,
                        value: $keyFontSizeScale,
                        in: scaleRange, step: scaleStep,
                        defaultValue: Self.defaultKeyFontSizeScale,
                        onChanged: { settings.keyFontSizeScale = $0 },
                    )
                    sliderRow(
                        label: Tab2Texts.keyCornerRadius,
                        value: $keyCornerRadius,
                        in: radiusRange, step: radiusStep,
                        defaultValue: Self.defaultKeyCornerRadius,
                        onChanged: { settings.keyCornerRadius = $0 },
                    )
                    sliderRow(
                        label: Tab2Texts.keyBorderWidth,
                        value: $keyBorderWidth,
                        in: borderWidthRange, step: borderWidthStep,
                        defaultValue: Self.defaultKeyBorderWidth,
                        onChanged: { settings.keyBorderWidth = $0 },
                    )
                }

                // Candidate section: colors + text size
                Section(header: Text(Tab2Texts.candidateSection)) {
                    colorRow(
                        label: Tab2Texts.colorCandidateText,
                        color: $candidateText,
                        defaultColor: Self.defaultCandidateText,
                        keyPath: \.candidateTextColor,
                    )
                    colorRow(
                        label: Tab2Texts.colorCandidateBackground,
                        color: $candidateBackground,
                        defaultColor: Self.defaultCandidateBackground,
                        keyPath: \.candidateBackgroundColor,
                    )
                    sliderRow(
                        label: Tab2Texts.candidateTextSize,
                        value: $candidateTextSizeScale,
                        in: scaleRange, step: scaleStep,
                        defaultValue: Self.defaultCandidateTextSizeScale,
                        onChanged: { settings.candidateTextSizeScale = $0 },
                    )
                }

                // Reset all appearance settings
                Section {
                    Button(role: .destructive) {
                        UIImpactFeedbackGenerator(style: .medium).impactOccurred()
                        resetAllAppearance()
                    } label: {
                        Text(Tab2Texts.appearanceResetAll)
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
                colorScheme: colorScheme,
            )
        }
        .navigationTitle(Tab2Texts.appearanceSettings)
        .navigationBarTitleDisplayMode(.inline)
    }

    // MARK: - Slider Row

    private func sliderRow(
        label: String,
        value: Binding<Double>,
        in range: ClosedRange<Double>,
        step: Double,
        defaultValue: Double,
        onChanged: @escaping (Double) -> Void,
    ) -> some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack {
                Text(label)
                Spacer()
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
            Slider(value: value, in: range, step: step)
                .onChange(of: value.wrappedValue) { _, newValue in
                    onChanged(newValue)
                }
        }
    }

    // MARK: - Helpers

    private func fontDisplayName(_ font: FontType) -> String {
        switch font {
        case .system: Tab2Texts.fontSystemDefault
        case .openHuninn: CommonTexts.fontOpenHuninn
        case .iansui: CommonTexts.fontIansui
        case .genYoMin: CommonTexts.fontGenYoMin
        case .genYoGothic: CommonTexts.fontGenYoGothic
        }
    }

    // MARK: - Color Row

    private func colorRow(
        label: String,
        color: Binding<Color>,
        defaultColor: Color,
        keyPath: WritableKeyPath<KeyboardColorSettings, CodableColor?>,
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
        settings.fontType = Self.defaultFontType

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
    @Binding var selectedFont: FontType
    var onChange: (FontType) -> Void

    private let options: [(font: FontType, text: String)] = [
        (.system, Tab2Texts.fontSystemDefault),
        (.openHuninn, CommonTexts.fontOpenHuninn),
        (.iansui, CommonTexts.fontIansui),
        (.genYoMin, CommonTexts.fontGenYoMin),
        (.genYoGothic, CommonTexts.fontGenYoGothic),
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
                            Text(option.text)
                                .foregroundColor(.primary)
                            Spacer()
                            if selectedFont == option.font {
                                Image(systemName: "checkmark")
                                    .foregroundColor(AppStyle.accentBlue)
                            }
                        }
                    }
                }
            }
        }
        .navigationTitle(Tab2Texts.customFont)
        .navigationBarTitleDisplayMode(.inline)
    }
}
