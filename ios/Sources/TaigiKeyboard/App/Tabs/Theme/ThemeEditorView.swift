import SwiftUI

/// The user-theme editor: the full appearance bundle + a live draft preview
/// pinned at the bottom. Pushed as a child page (uses the parent `NavigationStack`);
/// reuses `ThemeColorRow` / `ThemeSliderRow`. Adds a shadow slider
/// (user-theme-only feature) and a reset-to-defaults row. Font is a global
/// setting, not part of a theme, so the editor has no font control.
///
/// The name is entered in a `TextField` alert at save time, not inline — so the
/// editor has no inline text input and the software keyboard never appears to
/// squeeze the pinned preview. Rename later via the card's Edit action.
/// Save persists via the view model and auto-applies; back/pop discards.
struct ThemeEditorView: View {
    @Environment(\.dismiss) private var dismiss
    @Environment(\.colorScheme) private var colorScheme
    @Environment(DisplayLanguageStore.self) private var lang
    @StateObject private var viewModel: ThemeEditorViewModel
    @State private var showsCapAlert = false
    @State private var showsNameAlert = false
    @State private var pendingName = ""

    init(editing: UserTheme? = nil) {
        _viewModel = StateObject(wrappedValue: ThemeEditorViewModel(editing: editing))
    }

    var body: some View {
        VStack(spacing: 0) {
            Form {
                // Keyboard overall: background color + height
                Section(header: Text(lang.string(.themeKeyboardSection))) {
                    colorRow(lang.string(.themeColorKeyboardBackground), \.backgroundColor,
                             ThemeDefaults.keyboardBackground)
                    sliderRow(lang.string(.themeKeyHeight), \.keyHeightScale,
                              ThemeSliderRanges.scale, ThemeSliderRanges.scaleStep)
                }

                // Key: colors + font size + corner radius + border width + shadow
                Section(header: Text(lang.string(.themeColorKeySection))) {
                    colorRow(lang.string(.themeColorKeyText), \.keyTextColor,
                             ThemeDefaults.keyText)
                    colorRow(lang.string(.themeColorNormalKeyFill), \.normalKeyFillColor,
                             ThemeDefaults.normalKeyFill)
                    colorRow(lang.string(.themeColorSpecialKeyFill), \.specialKeyFillColor,
                             ThemeDefaults.specialKeyFill)
                    sliderRow(lang.string(.themeKeyFontSize), \.keyFontSizeScale,
                              ThemeSliderRanges.scale, ThemeSliderRanges.scaleStep)
                    sliderRow(lang.string(.themeKeyCornerRadius), \.keyCornerRadius,
                              ThemeSliderRanges.radius, ThemeSliderRanges.radiusStep)
                    sliderRow(lang.string(.themeKeyBorderWidth), \.keyBorderWidth,
                              ThemeSliderRanges.borderWidth, ThemeSliderRanges.borderWidthStep)
                    sliderRow(lang.string(.themeKeyShadow), \.keyShadowIntensity,
                              ThemeSliderRanges.shadow, ThemeSliderRanges.shadowStep)
                }

                // Candidate: colors + text size
                Section(header: Text(lang.string(.themeCandidateSection))) {
                    colorRow(lang.string(.themeColorCandidateText), \.candidateTextColor,
                             ThemeDefaults.candidateText)
                    colorRow(lang.string(.themeColorCandidateBackground), \.candidateBackgroundColor,
                             ThemeDefaults.candidateBackground)
                    sliderRow(lang.string(.themeCandidateTextSize), \.candidateTextSizeScale,
                              ThemeSliderRanges.scale, ThemeSliderRanges.scaleStep)
                }

                // Reset the draft appearance to defaults (name kept). Draft-only —
                // does not persist or change the applied theme until Save.
                Section {
                    Button(role: .destructive) {
                        viewModel.resetToDefaults()
                    } label: {
                        Text(lang.string(.themeEditorResetAll))
                            .frame(maxWidth: .infinity, alignment: .center)
                    }
                }
            }

            // Live draft preview, anchored at the bottom (mirrors where the real
            // keyboard sits on screen). The editor has no inline text input (name is
            // entered in an alert at save time), so the software keyboard never
            // appears to squeeze it. User themes own shadow (slider 0 = flat) →
            // `appliesThemeShadow: true`.
            KeyboardPreviewPanel(
                appearance: viewModel.appearance,
                appliesThemeShadow: true,
                colorScheme: colorScheme,
            )
        }
        .navigationTitle(viewModel.isEditing ? lang.string(.themeEditorTitleEdit) : lang.string(.themeEditorTitleNew))
        .navigationBarTitleDisplayMode(.inline)
        .toolbar {
            ToolbarItem(placement: .confirmationAction) {
                Button(lang.string(.themeEditorSave)) {
                    // Cap-check NEW themes up front so the cap alert and the name
                    // alert never present back-to-back.
                    guard viewModel.canSaveNewTheme else {
                        showsCapAlert = true
                        return
                    }
                    pendingName = viewModel.name
                    showsNameAlert = true
                }
            }
        }
        .alert(lang.string(.themeNameHeader), isPresented: $showsNameAlert) {
            TextField(lang.string(.themeNamePlaceholder), text: $pendingName)
            Button(lang.string(.themeEditorSave)) { commit() }
            Button(lang.string(.commonCancel), role: .cancel) {}
        }
        .alert(lang.string(.themeCapReachedTitle), isPresented: $showsCapAlert) {
            Button(lang.string(.commonOk), role: .cancel) {}
        } message: {
            Text(lang.string(.themeCapReachedMessage))
        }
    }

    /// Commits the draft with the alert-entered name (blank → default), then
    /// dismisses. The cap was pre-checked when Save was tapped; the `else` here is
    /// a belt-and-braces guard for a TOCTOU race.
    private func commit() {
        let trimmed = pendingName.trimmingCharacters(in: .whitespacesAndNewlines)
        // CROSS-PLATFORM INVARIANT — mirrors android ThemeEditorScreen.kt:343 `ifEmpty { resolve(THEME_DEFAULT_NAME) }`.
        // Persisted name freezes the creation-language label (an editable user value); release falls back to「新主題」.
        viewModel.name = trimmed.isEmpty ? lang.string(.themeDefaultName) : trimmed
        if viewModel.save() {
            dismiss()
        } else {
            showsCapAlert = true
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
