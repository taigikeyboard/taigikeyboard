// 中文: Theme Tab(主題)下的「外觀設定」頁。提供字型、配色、按鍵尺寸、候選列字級
// 中文: 等可調項目,並在底部錨定 KeyboardPreviewPanel 預覽。

import KeyboardKit
import SwiftUI

/// Appearance settings subpage
///
/// Provides font selection, 4 sliders for key height, key font size, candidate text size,
/// and key corner radius, with a keyboard preview anchored at the bottom.
// 中文: 外觀設定 SwiftUI 子頁。包含字型挑選、4 條尺寸 slider、配色 row,
// 中文: 與底部固定的 KeyboardPreviewPanel。
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
                            Text(ThemeTexts.customFont)
                            Spacer()
                            Text(viewModel.selectedFontType.displayName)
                                .foregroundColor(.secondary)
                        }
                    }
                }

                // Keyboard overall: background color + height
                Section(header: Text(ThemeTexts.keyboardSection)) {
                    colorRow(
                        label: ThemeTexts.colorKeyboardBackground,
                        color: $viewModel.keyboardBackground,
                        defaultColor: AppearanceSettingsViewModel.Defaults.keyboardBackground,
                        keyPath: \.backgroundColor,
                    )
                    sliderRow(
                        label: ThemeTexts.keyHeight,
                        value: $viewModel.keyHeightScale,
                        in: scaleRange, step: scaleStep,
                        defaultValue: AppearanceSettingsViewModel.Defaults.keyHeightScale,
                        onChanged: { viewModel.setKeyHeightScale($0) },
                    )
                }

                // Key section: colors + font size + corner radius + border width
                Section(header: Text(ThemeTexts.colorKeySection)) {
                    colorRow(
                        label: ThemeTexts.colorKeyText,
                        color: $viewModel.keyText,
                        defaultColor: AppearanceSettingsViewModel.Defaults.keyText,
                        keyPath: \.keyTextColor,
                    )
                    colorRow(
                        label: ThemeTexts.colorNormalKeyFill,
                        color: $viewModel.normalKeyFill,
                        defaultColor: AppearanceSettingsViewModel.Defaults.normalKeyFill,
                        keyPath: \.normalKeyFillColor,
                    )
                    colorRow(
                        label: ThemeTexts.colorSpecialKeyFill,
                        color: $viewModel.specialKeyFill,
                        defaultColor: AppearanceSettingsViewModel.Defaults.specialKeyFill,
                        keyPath: \.specialKeyFillColor,
                    )
                    sliderRow(
                        label: ThemeTexts.keyFontSize,
                        value: $viewModel.keyFontSizeScale,
                        in: scaleRange, step: scaleStep,
                        defaultValue: AppearanceSettingsViewModel.Defaults.keyFontSizeScale,
                        onChanged: { viewModel.setKeyFontSizeScale($0) },
                    )
                    sliderRow(
                        label: ThemeTexts.keyCornerRadius,
                        value: $viewModel.keyCornerRadius,
                        in: radiusRange, step: radiusStep,
                        defaultValue: AppearanceSettingsViewModel.Defaults.keyCornerRadius,
                        onChanged: { viewModel.setKeyCornerRadius($0) },
                    )
                    sliderRow(
                        label: ThemeTexts.keyBorderWidth,
                        value: $viewModel.keyBorderWidth,
                        in: borderWidthRange, step: borderWidthStep,
                        defaultValue: AppearanceSettingsViewModel.Defaults.keyBorderWidth,
                        onChanged: { viewModel.setKeyBorderWidth($0) },
                    )
                }

                // Candidate section: colors + text size
                Section(header: Text(ThemeTexts.candidateSection)) {
                    colorRow(
                        label: ThemeTexts.colorCandidateText,
                        color: $viewModel.candidateText,
                        defaultColor: AppearanceSettingsViewModel.Defaults.candidateText,
                        keyPath: \.candidateTextColor,
                    )
                    colorRow(
                        label: ThemeTexts.colorCandidateBackground,
                        color: $viewModel.candidateBackground,
                        defaultColor: AppearanceSettingsViewModel.Defaults.candidateBackground,
                        keyPath: \.candidateBackgroundColor,
                    )
                    sliderRow(
                        label: ThemeTexts.candidateTextSize,
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
                        Text(ThemeTexts.appearanceResetAll)
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
        .navigationTitle(ThemeTexts.appearanceSettings)
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
                        Image(latinSystemName: "arrow.counterclockwise")
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
                    Image(latinSystemName: "arrow.counterclockwise")
                        .foregroundColor(.secondary)
                }
                .buttonStyle(.plain)
            }
        }
    }
}

// MARK: - Font Picker Subpage

// 中文: 字型挑選子頁。列出 FontType.allCases,點選即更新 binding 與通知 viewModel。
private struct AppearanceFontPickerView: View {
    @Binding var selectedFont: FontType
    var onChange: (FontType) -> Void

    var body: some View {
        Form {
            Section {
                ForEach(FontType.allCases, id: \.self) { font in
                    Button {
                        selectedFont = font
                        onChange(font)
                    } label: {
                        HStack {
                            Text(font.displayName)
                                .foregroundColor(.primary)
                            Spacer()
                            if selectedFont == font {
                                Image(latinSystemName: "checkmark")
                                    .foregroundColor(AppStyle.accentBlue)
                            }
                        }
                    }
                }
            }
        }
        .navigationTitle(ThemeTexts.customFont)
        .navigationBarTitleDisplayMode(.inline)
    }
}
