// 中文: Theme Tab(主題)下的「自訂外觀設定」頁 = 編輯「預設」主題 buffer(live-write SharedSettings)。
// 中文: 字型、配色、按鍵尺寸、候選列字級等可調項,底部錨定 KeyboardPreviewPanel 預覽。

import KeyboardKit
import SwiftUI

/// The "自訂外觀設定" subpage: edits the **default** theme buffer live (writes
/// `SharedSettings` immediately via `AppearanceSettingsViewModel`). Reuses the
/// shared `ThemeColorRow` / `ThemeSliderRow` controls. No shadow control — the
/// shadow slider is a user-theme-only feature in `ThemeEditorView`; the default
/// theme keeps KeyboardKit's standard shadow (`appliesThemeShadow: false`).
// 中文: 編輯「預設」主題 buffer(即時寫入)。共用 ThemeColorRow / ThemeSliderRow;無陰影控制(陰影是自訂主題專屬)。
struct AppearanceSettingsView: View {
    @Environment(\.colorScheme) private var colorScheme
    @StateObject private var viewModel = AppearanceSettingsViewModel()

    var body: some View {
        VStack(spacing: 0) {
            Form {
                // Font picker
                Section {
                    NavigationLink {
                        ThemeFontPickerView(
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
                    ThemeColorRow(
                        label: ThemeTexts.colorKeyboardBackground,
                        color: $viewModel.keyboardBackground,
                        defaultColor: AppearanceSettingsViewModel.Defaults.keyboardBackground,
                        isCustomized: viewModel.savedColors.backgroundColor != nil,
                        onChange: { viewModel.applyColorChange(\.backgroundColor, to: $0) },
                        onReset: { viewModel.resetColor(\.backgroundColor) },
                    )
                    ThemeSliderRow(
                        label: ThemeTexts.keyHeight,
                        value: $viewModel.keyHeightScale,
                        range: ThemeSliderRanges.scale, step: ThemeSliderRanges.scaleStep,
                        defaultValue: AppearanceSettingsViewModel.Defaults.keyHeightScale,
                        onChanged: { viewModel.setKeyHeightScale($0) },
                    )
                }

                // Key section: colors + font size + corner radius + border width
                Section(header: Text(ThemeTexts.colorKeySection)) {
                    ThemeColorRow(
                        label: ThemeTexts.colorKeyText,
                        color: $viewModel.keyText,
                        defaultColor: AppearanceSettingsViewModel.Defaults.keyText,
                        isCustomized: viewModel.savedColors.keyTextColor != nil,
                        onChange: { viewModel.applyColorChange(\.keyTextColor, to: $0) },
                        onReset: { viewModel.resetColor(\.keyTextColor) },
                    )
                    ThemeColorRow(
                        label: ThemeTexts.colorNormalKeyFill,
                        color: $viewModel.normalKeyFill,
                        defaultColor: AppearanceSettingsViewModel.Defaults.normalKeyFill,
                        isCustomized: viewModel.savedColors.normalKeyFillColor != nil,
                        onChange: { viewModel.applyColorChange(\.normalKeyFillColor, to: $0) },
                        onReset: { viewModel.resetColor(\.normalKeyFillColor) },
                    )
                    ThemeColorRow(
                        label: ThemeTexts.colorSpecialKeyFill,
                        color: $viewModel.specialKeyFill,
                        defaultColor: AppearanceSettingsViewModel.Defaults.specialKeyFill,
                        isCustomized: viewModel.savedColors.specialKeyFillColor != nil,
                        onChange: { viewModel.applyColorChange(\.specialKeyFillColor, to: $0) },
                        onReset: { viewModel.resetColor(\.specialKeyFillColor) },
                    )
                    ThemeSliderRow(
                        label: ThemeTexts.keyFontSize,
                        value: $viewModel.keyFontSizeScale,
                        range: ThemeSliderRanges.scale, step: ThemeSliderRanges.scaleStep,
                        defaultValue: AppearanceSettingsViewModel.Defaults.keyFontSizeScale,
                        onChanged: { viewModel.setKeyFontSizeScale($0) },
                    )
                    ThemeSliderRow(
                        label: ThemeTexts.keyCornerRadius,
                        value: $viewModel.keyCornerRadius,
                        range: ThemeSliderRanges.radius, step: ThemeSliderRanges.radiusStep,
                        defaultValue: AppearanceSettingsViewModel.Defaults.keyCornerRadius,
                        onChanged: { viewModel.setKeyCornerRadius($0) },
                    )
                    ThemeSliderRow(
                        label: ThemeTexts.keyBorderWidth,
                        value: $viewModel.keyBorderWidth,
                        range: ThemeSliderRanges.borderWidth, step: ThemeSliderRanges.borderWidthStep,
                        defaultValue: AppearanceSettingsViewModel.Defaults.keyBorderWidth,
                        onChanged: { viewModel.setKeyBorderWidth($0) },
                    )
                }

                // Candidate section: colors + text size
                Section(header: Text(ThemeTexts.candidateSection)) {
                    ThemeColorRow(
                        label: ThemeTexts.colorCandidateText,
                        color: $viewModel.candidateText,
                        defaultColor: AppearanceSettingsViewModel.Defaults.candidateText,
                        isCustomized: viewModel.savedColors.candidateTextColor != nil,
                        onChange: { viewModel.applyColorChange(\.candidateTextColor, to: $0) },
                        onReset: { viewModel.resetColor(\.candidateTextColor) },
                    )
                    ThemeColorRow(
                        label: ThemeTexts.colorCandidateBackground,
                        color: $viewModel.candidateBackground,
                        defaultColor: AppearanceSettingsViewModel.Defaults.candidateBackground,
                        isCustomized: viewModel.savedColors.candidateBackgroundColor != nil,
                        onChange: { viewModel.applyColorChange(\.candidateBackgroundColor, to: $0) },
                        onReset: { viewModel.resetColor(\.candidateBackgroundColor) },
                    )
                    ThemeSliderRow(
                        label: ThemeTexts.candidateTextSize,
                        value: $viewModel.candidateTextSizeScale,
                        range: ThemeSliderRanges.scale, step: ThemeSliderRanges.scaleStep,
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

            // Keyboard preview anchored at bottom. The default buffer keeps
            // KeyboardKit's standard shadow → `appliesThemeShadow: false`.
            KeyboardPreviewPanel(
                appearance: previewAppearance,
                appliesThemeShadow: false,
                colorScheme: colorScheme,
            )
        }
        .navigationTitle(ThemeTexts.appearanceSettings)
        .navigationBarTitleDisplayMode(.inline)
    }

    // 中文: 由 viewModel 即時組出供預覽用的外觀(sliders / 顏色一動就更新)。陰影固定 0(預設 buffer 無陰影控制)。
    private var previewAppearance: ThemeAppearance {
        ThemeAppearance(
            colors: viewModel.savedColors,
            keyShadowIntensity: 0,
            keyHeightScale: viewModel.keyHeightScale,
            keyFontSizeScale: viewModel.keyFontSizeScale,
            candidateTextSizeScale: viewModel.candidateTextSizeScale,
            keyCornerRadius: viewModel.keyCornerRadius,
            keyBorderWidth: viewModel.keyBorderWidth,
            fontType: viewModel.selectedFontType,
        )
    }
}
