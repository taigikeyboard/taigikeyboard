import KeyboardKit
import SwiftUI

/// Appearance settings subpage
///
/// Provides font selection, 4 sliders for key height, key font size, candidate text size,
/// and key corner radius, with a keyboard preview anchored at the bottom.
struct AppearanceSettingsView: View {
    @Environment(\.colorScheme) private var colorScheme
    @StateObject private var viewModel = AppearanceSettingsViewModel()

    private let scaleRange: ClosedRange<Double> = 0.85 ... 1.15
    private let scaleStep: Double = 0.01
    private let radiusRange: ClosedRange<Double> = 0 ... 15
    private let radiusStep: Double = 0.5
    private let borderWidthRange: ClosedRange<Double> = 0 ... 3
    private let borderWidthStep: Double = 0.5

    var body: some View {
        VStack(spacing: 0) {
            Form {
                // Font picker
                Section {
                    NavigationLink {
                        AppearanceFontPickerView(
                            selectedFont: $viewModel.selectedFontType,
                            onChange: { viewModel.setFontType($0) },
                        )
                    } label: {
                        HStack {
                            Text(LayoutTexts.customFont)
                            Spacer()
                            Text(fontDisplayName(viewModel.selectedFontType))
                                .foregroundColor(.secondary)
                        }
                    }
                }

                // Keyboard overall: background color + height
                Section(header: Text(LayoutTexts.keyboardSection)) {
                    colorRow(
                        label: LayoutTexts.colorKeyboardBackground,
                        color: $viewModel.keyboardBackground,
                        defaultColor: AppearanceSettingsViewModel.Defaults.keyboardBackground,
                        keyPath: \.backgroundColor,
                    )
                    sliderRow(
                        label: LayoutTexts.keyHeight,
                        value: $viewModel.keyHeightScale,
                        in: scaleRange, step: scaleStep,
                        defaultValue: AppearanceSettingsViewModel.Defaults.keyHeightScale,
                        onChanged: { viewModel.setKeyHeightScale($0) },
                    )
                }

                // Key section: colors + font size + corner radius + border width
                Section(header: Text(LayoutTexts.colorKeySection)) {
                    colorRow(
                        label: LayoutTexts.colorKeyText,
                        color: $viewModel.keyText,
                        defaultColor: AppearanceSettingsViewModel.Defaults.keyText,
                        keyPath: \.keyTextColor,
                    )
                    colorRow(
                        label: LayoutTexts.colorNormalKeyFill,
                        color: $viewModel.normalKeyFill,
                        defaultColor: AppearanceSettingsViewModel.Defaults.normalKeyFill,
                        keyPath: \.normalKeyFillColor,
                    )
                    colorRow(
                        label: LayoutTexts.colorSpecialKeyFill,
                        color: $viewModel.specialKeyFill,
                        defaultColor: AppearanceSettingsViewModel.Defaults.specialKeyFill,
                        keyPath: \.specialKeyFillColor,
                    )
                    sliderRow(
                        label: LayoutTexts.keyFontSize,
                        value: $viewModel.keyFontSizeScale,
                        in: scaleRange, step: scaleStep,
                        defaultValue: AppearanceSettingsViewModel.Defaults.keyFontSizeScale,
                        onChanged: { viewModel.setKeyFontSizeScale($0) },
                    )
                    sliderRow(
                        label: LayoutTexts.keyCornerRadius,
                        value: $viewModel.keyCornerRadius,
                        in: radiusRange, step: radiusStep,
                        defaultValue: AppearanceSettingsViewModel.Defaults.keyCornerRadius,
                        onChanged: { viewModel.setKeyCornerRadius($0) },
                    )
                    sliderRow(
                        label: LayoutTexts.keyBorderWidth,
                        value: $viewModel.keyBorderWidth,
                        in: borderWidthRange, step: borderWidthStep,
                        defaultValue: AppearanceSettingsViewModel.Defaults.keyBorderWidth,
                        onChanged: { viewModel.setKeyBorderWidth($0) },
                    )
                }

                // Candidate section: colors + text size
                Section(header: Text(LayoutTexts.candidateSection)) {
                    colorRow(
                        label: LayoutTexts.colorCandidateText,
                        color: $viewModel.candidateText,
                        defaultColor: AppearanceSettingsViewModel.Defaults.candidateText,
                        keyPath: \.candidateTextColor,
                    )
                    colorRow(
                        label: LayoutTexts.colorCandidateBackground,
                        color: $viewModel.candidateBackground,
                        defaultColor: AppearanceSettingsViewModel.Defaults.candidateBackground,
                        keyPath: \.candidateBackgroundColor,
                    )
                    sliderRow(
                        label: LayoutTexts.candidateTextSize,
                        value: $viewModel.candidateTextSizeScale,
                        in: scaleRange, step: scaleStep,
                        defaultValue: AppearanceSettingsViewModel.Defaults.candidateTextSizeScale,
                        onChanged: { viewModel.setCandidateTextSizeScale($0) },
                    )
                }

                // Reset all appearance settings
                Section {
                    Button(role: .destructive) {
                        UIImpactFeedbackGenerator(style: .medium).impactOccurred()
                        viewModel.resetAllAppearance()
                    } label: {
                        Text(LayoutTexts.appearanceResetAll)
                    }
                }
            }

            // Keyboard preview anchored at bottom
            KeyboardPreviewPanel(
                keyHeightScale: viewModel.keyHeightScale,
                keyFontSizeScale: viewModel.keyFontSizeScale,
                candidateTextSizeScale: viewModel.candidateTextSizeScale,
                keyCornerRadius: viewModel.keyCornerRadius,
                fontType: viewModel.selectedFontType,
                colorScheme: colorScheme,
            )
        }
        .navigationTitle(LayoutTexts.appearanceSettings)
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
        case .system: LayoutTexts.fontSystemDefault
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
                    viewModel.applyColorChange(keyPath, to: newValue)
                }

            if viewModel.savedColors[keyPath: keyPath] != nil {
                Button {
                    color.wrappedValue = defaultColor
                    viewModel.resetColor(keyPath)
                } label: {
                    Image(systemName: "arrow.counterclockwise")
                        .foregroundColor(.secondary)
                }
                .buttonStyle(.plain)
            }
        }
    }
}

// MARK: - Font Picker Subpage

private struct AppearanceFontPickerView: View {
    @Binding var selectedFont: FontType
    var onChange: (FontType) -> Void

    private let options: [(font: FontType, text: String)] = [
        (.system, LayoutTexts.fontSystemDefault),
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
        .navigationTitle(LayoutTexts.customFont)
        .navigationBarTitleDisplayMode(.inline)
    }
}
