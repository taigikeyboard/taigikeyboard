// 中文: 自訂主題編輯器。命名 + 全外觀(6 配色 + 5 尺寸 + 陰影)+ 即時草稿預覽。字型為全域設定,不在編輯器內。
// 中文: 以 sheet 呈現,內含自有 NavigationStack(取消/儲存 toolbar)。Save 後自動套用。

import SwiftUI

/// The user-theme editor: name + the full appearance bundle + a live draft
/// preview. Presented as a sheet; reuses `ThemeColorRow` / `ThemeSliderRow`.
/// Adds a shadow slider (user-theme-only feature). Font is a global setting, not
/// part of a theme, so the editor has no font control.
/// Save persists via the view model and auto-applies; Cancel discards.
// 中文: 自訂主題編輯器。共用控制列 + 陰影 slider(自訂主題專屬)。Save 落盤並自動套用,Cancel 丟棄。
struct ThemeEditorView: View {
    @Environment(\.dismiss) private var dismiss
    @Environment(\.colorScheme) private var colorScheme
    @StateObject private var viewModel: ThemeEditorViewModel
    @State private var showsCapAlert = false

    init(editing: UserTheme? = nil) {
        _viewModel = StateObject(wrappedValue: ThemeEditorViewModel(editing: editing))
    }

    var body: some View {
        NavigationStack {
            VStack(spacing: 0) {
                Form {
                    // Name
                    Section(header: Text(ThemeTexts.themeNameHeader)) {
                        TextField(ThemeTexts.themeNamePlaceholder, text: $viewModel.name)
                    }

                    // Keyboard overall: background color + height
                    Section(header: Text(ThemeTexts.keyboardSection)) {
                        colorRow(ThemeTexts.colorKeyboardBackground, \.backgroundColor,
                                 ThemeDefaults.keyboardBackground)
                        sliderRow(ThemeTexts.keyHeight, \.keyHeightScale,
                                  ThemeSliderRanges.scale, ThemeSliderRanges.scaleStep)
                    }

                    // Key: colors + font size + corner radius + border width + shadow
                    Section(header: Text(ThemeTexts.colorKeySection)) {
                        colorRow(ThemeTexts.colorKeyText, \.keyTextColor,
                                 ThemeDefaults.keyText)
                        colorRow(ThemeTexts.colorNormalKeyFill, \.normalKeyFillColor,
                                 ThemeDefaults.normalKeyFill)
                        colorRow(ThemeTexts.colorSpecialKeyFill, \.specialKeyFillColor,
                                 ThemeDefaults.specialKeyFill)
                        sliderRow(ThemeTexts.keyFontSize, \.keyFontSizeScale,
                                  ThemeSliderRanges.scale, ThemeSliderRanges.scaleStep)
                        sliderRow(ThemeTexts.keyCornerRadius, \.keyCornerRadius,
                                  ThemeSliderRanges.radius, ThemeSliderRanges.radiusStep)
                        sliderRow(ThemeTexts.keyBorderWidth, \.keyBorderWidth,
                                  ThemeSliderRanges.borderWidth, ThemeSliderRanges.borderWidthStep)
                        sliderRow(ThemeTexts.keyShadow, \.keyShadowIntensity,
                                  ThemeSliderRanges.shadow, ThemeSliderRanges.shadowStep)
                    }

                    // Candidate: colors + text size
                    Section(header: Text(ThemeTexts.candidateSection)) {
                        colorRow(ThemeTexts.colorCandidateText, \.candidateTextColor,
                                 ThemeDefaults.candidateText)
                        colorRow(ThemeTexts.colorCandidateBackground, \.candidateBackgroundColor,
                                 ThemeDefaults.candidateBackground)
                        sliderRow(ThemeTexts.candidateTextSize, \.candidateTextSizeScale,
                                  ThemeSliderRanges.scale, ThemeSliderRanges.scaleStep)
                    }
                }

                // Live draft preview. User themes own shadow (slider 0 = flat) →
                // `appliesThemeShadow: true`.
                KeyboardPreviewPanel(
                    appearance: viewModel.appearance,
                    appliesThemeShadow: true,
                    colorScheme: colorScheme,
                )
            }
            .navigationTitle(viewModel.isEditing ? ThemeTexts.editorTitleEdit : ThemeTexts.editorTitleNew)
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button(ThemeTexts.editorCancel) { dismiss() }
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button(ThemeTexts.editorSave) {
                        if viewModel.save() {
                            dismiss()
                        } else {
                            showsCapAlert = true
                        }
                    }
                    .disabled(!viewModel.isSaveEnabled)
                }
            }
            .alert(ThemeTexts.capReachedTitle, isPresented: $showsCapAlert) {
                Button(ThemeTexts.capReachedOK, role: .cancel) {}
            } message: {
                Text(ThemeTexts.capReachedMessage)
            }
        }
    }

    // MARK: - Row builders (bind into the draft)

    private func colorRow(
        _ label: String,
        _ keyPath: WritableKeyPath<KeyboardColorSettings, CodableColor?>,
        _ defaultColor: Color,
    ) -> some View {
        ThemeColorRow(
            label: label,
            color: viewModel.colorBinding(keyPath, default: defaultColor),
            defaultColor: defaultColor,
            isCustomized: viewModel.isColorCustomized(keyPath),
            onChange: { _ in },
            onReset: { viewModel.resetColor(keyPath) },
        )
    }

    private func sliderRow(
        _ label: String,
        _ keyPath: WritableKeyPath<ThemeAppearance, Double>,
        _ range: ClosedRange<Double>,
        _ step: Double,
    ) -> some View {
        ThemeSliderRow(
            label: label,
            value: viewModel.scalarBinding(keyPath),
            range: range, step: step,
            defaultValue: ThemeAppearance.default[keyPath: keyPath],
            onChanged: { _ in },
        )
    }
}
