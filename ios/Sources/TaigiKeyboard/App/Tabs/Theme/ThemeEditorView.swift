import PhotosUI
import SwiftUI

/// The user-theme editor: the full appearance bundle + a live draft preview
/// pinned at the bottom. Pushed as a child page (uses the parent `NavigationStack`);
/// reuses `ThemeColorRow` / `ThemeSliderRow`. A gradient's direction is set by
/// dragging on the preview (`GradientDirectionOverlay`), not by a form row.
///
/// Three sections, one per visual surface (USER 2026-09-19): **Background** (type
/// Solid / Gradient / Photo and its rows — the keyboard and the candidate bar share this
/// one surface), **Keys** (fills, text, shape, size), **Candidate Bar** (text color + size),
/// then Reset to Defaults. Font is a global setting, not part of a theme, so the editor
/// has no font control. The photo comes from `PhotosPicker` (no library permission
/// needed) and is stored through `SharedSettings.saveThemeImage`.
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
    @State private var pickedPhoto: PhotosPickerItem?

    init(editing: UserTheme? = nil) {
        _viewModel = StateObject(wrappedValue: ThemeEditorViewModel(editing: editing))
    }

    var body: some View {
        VStack(spacing: 0) {
            Form {
                // Background: one surface for keyboard + candidate bar.
                Section(header: Text(lang.string(.themeBackgroundSection))) {
                    Picker("", selection: viewModel.backgroundKindBinding) {
                        Text(lang.string(.themeBackgroundTypeSolid)).tag(ThemeBackground.Kind.solid)
                        Text(lang.string(.themeBackgroundTypeGradient)).tag(ThemeBackground.Kind.gradient)
                        Text(lang.string(.themeBackgroundTypePhoto)).tag(ThemeBackground.Kind.image)
                    }
                    .pickerStyle(.segmented)
                    .labelsHidden()

                    switch viewModel.backgroundKind {
                    case .solid:
                        ThemeColorRow(
                            label: lang.string(.themeColorKeyboardBackground),
                            color: viewModel.solidBackgroundBinding,
                            onReset: viewModel.isSolidBackgroundCustomized ? { viewModel.resetSolidBackground() } : nil,
                        )
                    case .gradient:
                        // The two stops ARE the gradient, not overrides of a seed → no reset arrow.
                        ColorPicker(lang.string(.themeGradientStartColor), selection: viewModel.gradientStopBinding(0), supportsOpacity: false)
                        ColorPicker(lang.string(.themeGradientEndColor), selection: viewModel.gradientStopBinding(1), supportsOpacity: false)
                    // No Direction row: the pointer on the preview below is the direction control.
                    case .image:
                        ThemePhotoRow(
                            label: lang.string(viewModel.photo == nil ? .themePhotoPick : .themePhotoChange),
                            file: viewModel.photo?.file,
                            selection: $pickedPhoto,
                        )
                        if viewModel.photo != nil {
                            sliderRow(lang.string(.themePhotoDim), viewModel.photoDimBinding,
                                      ThemeImageBackground.dimRange, ThemeImageBackground.dimStep,
                                      defaultValue: ThemeImageBackground.defaultDim)
                        }
                    }
                }

                // Keys: fills + text, then shape, then size.
                Section(header: Text(lang.string(.themeColorKeySection))) {
                    colorRow(lang.string(.themeColorNormalKeyFill), \.normalKeyFillColor)
                    colorRow(lang.string(.themeColorSpecialKeyFill), \.specialKeyFillColor)
                    colorRow(lang.string(.themeColorKeyText), \.keyTextColor)
                    sliderRow(lang.string(.themeKeyCornerRadius), \.keyCornerRadius,
                              ThemeSliderRanges.radius, ThemeSliderRanges.radiusStep)
                    sliderRow(lang.string(.themeKeyBorderWidth), \.keyBorderWidth,
                              ThemeSliderRanges.borderWidth, ThemeSliderRanges.borderWidthStep)
                    sliderRow(lang.string(.themeKeyShadow), \.keyShadowIntensity,
                              ThemeSliderRanges.shadow, ThemeSliderRanges.shadowStep)
                    sliderRow(lang.string(.themeKeyHeight), \.keyHeightScale,
                              ThemeSliderRanges.scale, ThemeSliderRanges.scaleStep)
                    sliderRow(lang.string(.themeKeyFontSize), \.keyFontSizeScale,
                              ThemeSliderRanges.scale, ThemeSliderRanges.scaleStep)
                }

                // Candidates: text color + size (the bar shares the background surface).
                Section(header: Text(lang.string(.themeCandidateSection))) {
                    colorRow(lang.string(.themeColorCandidateText), \.candidateTextColor)
                    sliderRow(lang.string(.themeCandidateTextSize), \.candidateTextSizeScale,
                              ThemeSliderRanges.scale, ThemeSliderRanges.scaleStep)
                }

                // Reset the draft appearance to the seed (name kept). Draft-only —
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
            // `appliesThemeShadow: true`. While the background is a gradient the
            // preview doubles as the direction control: drag on it to set the angle.
            KeyboardPreviewPanel(
                appearance: viewModel.appearance,
                appliesThemeShadow: true,
                colorScheme: colorScheme,
            )
            .overlay {
                if viewModel.backgroundKind == .gradient {
                    GradientDirectionOverlay(
                        label: lang.string(.themeGradientDirection),
                        angle: viewModel.gradientAngleBinding,
                    )
                }
            }
        }
        .navigationTitle(viewModel.isEditing ? lang.string(.themeEditorTitleEdit) : lang.string(.themeEditorTitleNew))
        .navigationBarTitleDisplayMode(.inline)
        .toolbar {
            ToolbarItem(placement: .confirmationAction) {
                Button(lang.string(.commonSave)) {
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
        .task(id: pickedPhoto) {
            guard let pickedPhoto, let data = try? await pickedPhoto.loadTransferable(type: Data.self) else { return }
            // Downsample + encode + write off the main actor; only the file name comes back.
            let file = await Task.detached(priority: .userInitiated) { ThemeImageCache.shared.store.save(data) }.value
            if let file {
                viewModel.setPhoto(file: file)
            }
            self.pickedPhoto = nil
        }
        // A photo picked then discarded (Back, or replaced before Save) is an orphan file
        // until a theme mutation sweeps; sweep on the way out so it never lingers.
        .onDisappear { SharedSettings.shared.sweepThemeImages() }
        .alert(lang.string(.themeNameHeader), isPresented: $showsNameAlert) {
            TextField(lang.string(.themeNamePlaceholder), text: $pendingName)
            Button(lang.string(.commonSave)) { commit() }
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
        // CROSS-PLATFORM INVARIANT — mirrors android ThemeEditorScreen.kt:343 `ifEmpty { resolve(THEME_EDITOR_TITLE_NEW) }`.
        // Persisted name freezes the creation-language label (an editable user value); release falls back to "New Theme".
        viewModel.name = trimmed.isEmpty ? lang.string(.themeEditorTitleNew) : trimmed
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
    ) -> some View {
        ThemeColorRow(
            label: label,
            color: viewModel.colorBinding(keyPath),
            onReset: viewModel.isColorCustomized(keyPath) ? { viewModel.resetColor(keyPath) } : nil,
        )
    }

    private func sliderRow(
        _ label: String,
        _ keyPath: WritableKeyPath<ThemeAppearance, Double>,
        _ range: ClosedRange<Double>,
        _ step: Double,
    ) -> some View {
        sliderRow(label, viewModel.scalarBinding(keyPath), range, step, defaultValue: ThemeAppearance.default[keyPath: keyPath])
    }

    private func sliderRow(
        _ label: String,
        _ value: Binding<Double>,
        _ range: ClosedRange<Double>,
        _ step: Double,
        defaultValue: Double,
    ) -> some View {
        ThemeSliderRow(
            label: label,
            value: value,
            range: range, step: step,
            defaultValue: defaultValue,
            onChanged: { _ in },
        )
    }
}
