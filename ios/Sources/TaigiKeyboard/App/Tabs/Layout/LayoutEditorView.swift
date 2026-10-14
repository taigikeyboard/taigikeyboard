import SwiftUI

/// Custom-layout editor: base chips, the board and the spare tray. Tap two keys to swap them; tap
/// a key, then a spare, to replace it. Nothing is stored until Save, which names the layout and
/// applies it; Back discards the draft.
struct LayoutEditorView: View {
    @Environment(DisplayLanguageStore.self) private var lang
    @Environment(\.dismiss) private var dismiss

    @State private var draft: CustomLayout
    @State private var selectedSlot: Int?
    @State private var showsNameAlert = false
    @State private var pendingName = ""
    private let isEditing: Bool

    init(editing layout: CustomLayout? = nil) {
        _draft = State(initialValue: layout ?? Self.newDraft())
        isEditing = layout != nil
    }

    var body: some View {
        ScrollView {
            VStack(spacing: 24) {
                baseChips

                LayoutKeyGrid(rows: draft.rows(withGlobe: false), selectedSlot: selectedSlot, onTapSlot: tapKey)
                    .aspectRatio(GalleryCardMetrics.previewAspectRatio, contentMode: .fit)
                    .clipShape(RoundedRectangle(cornerRadius: AppStyle.previewCornerRadius))
                    .padding(.horizontal, AppStyle.horizontalPadding)

                spareTray

                Button(lang.string(.themeEditorResetAll), role: .destructive) {
                    restart(from: draft.base)
                }
                .font(AppStyle.bodyFont)
                .disabled(draft.isUnchanged)
            }
            .padding(.vertical, AppStyle.horizontalPadding)
        }
        .background(Color(.systemGroupedBackground))
        .navigationTitle(isEditing ? draft.name : lang.string(.layoutEditorTitleNew))
        .navigationBarTitleDisplayMode(.inline)
        .toolbar {
            ToolbarItem(placement: .confirmationAction) {
                Button(lang.string(.commonSave)) {
                    pendingName = draft.name
                    showsNameAlert = true
                }
            }
        }
        .alert(lang.string(.layoutNameHeader), isPresented: $showsNameAlert) {
            TextField(lang.string(.layoutEditorTitleNew), text: $pendingName)
            Button(lang.string(.commonSave)) { commit() }
            Button(lang.string(.commonCancel), role: .cancel) {}
        }
    }

    // MARK: - Sections

    private var baseChips: some View {
        ScrollView(.horizontal, showsIndicators: false) {
            HStack(spacing: 8) {
                ForEach(KeyboardLayoutType.romanizationLayouts, id: \.self) { base in
                    let isSelected = base == draft.base
                    Button {
                        restart(from: base)
                    } label: {
                        Text(lang.string(base.displayNameKey))
                            .font(AppStyle.captionFont)
                            .foregroundStyle(isSelected ? Color.white : Color.primary)
                            .padding(.horizontal, 14)
                            .padding(.vertical, 8)
                            .background(isSelected ? AppStyle.accentBlue : Color(.secondarySystemGroupedBackground), in: Capsule())
                    }
                    .buttonStyle(.plain)
                    .accessibilityAddTraits(isSelected ? .isSelected : [])
                }
            }
            .padding(.horizontal, AppStyle.horizontalPadding)
        }
    }

    /// Spares light up once a key is selected; tapping one puts it on that key.
    @ViewBuilder
    private var spareTray: some View {
        let spares = draft.spareKeys
        if !spares.isEmpty {
            LazyVGrid(columns: [GridItem(.adaptive(minimum: 44), spacing: 8)], spacing: 8) {
                ForEach(spares, id: \.self) { spare in
                    Button {
                        replaceSelectedKey(with: spare)
                    } label: {
                        Text(verbatim: spare)
                            .font(AppStyle.bodyFont)
                            .foregroundStyle(Color.primary)
                            .frame(maxWidth: .infinity, minHeight: 44)
                            .background(Color(.secondarySystemGroupedBackground), in: RoundedRectangle(cornerRadius: 8))
                    }
                    .buttonStyle(.plain)
                    .disabled(selectedSlot == nil)
                    .opacity(selectedSlot == nil ? 0.4 : 1)
                }
            }
            .padding(.horizontal, AppStyle.horizontalPadding)
        }
    }

    // MARK: - Editing

    private func tapKey(_ slot: Int) {
        withAnimation(.snappy(duration: 0.2)) {
            if let selected = selectedSlot, selected != slot {
                draft.swapKeys(selected, slot)
                selectedSlot = nil
            } else {
                selectedSlot = selectedSlot == slot ? nil : slot
            }
        }
    }

    private func replaceSelectedKey(with spare: String) {
        guard let selectedSlot else { return }
        withAnimation(.snappy(duration: 0.2)) {
            draft.replaceKey(at: selectedSlot, with: spare)
            self.selectedSlot = nil
        }
    }

    private func restart(from base: KeyboardLayoutType) {
        withAnimation(.snappy(duration: 0.2)) {
            draft.reset(to: base)
            selectedSlot = nil
        }
    }

    /// Names the draft from the alert (blank → "New Layout"), saves and applies it, then closes.
    /// A failed write keeps the editor open with the draft intact.
    private func commit() {
        let trimmed = pendingName.trimmingCharacters(in: .whitespacesAndNewlines)
        draft.name = trimmed.isEmpty ? lang.string(.layoutEditorTitleNew) : trimmed
        if SharedSettings.shared.saveCustomLayout(draft) {
            dismiss()
        }
    }

    /// A new layout starts from the layout in use — its base table in the current script — or
    /// from PhahTaigi while the TPS layout is on.
    private static func newDraft() -> CustomLayout {
        let settings = SharedSettings.shared
        if let active = settings.activeCustomLayout {
            return CustomLayout(base: active.base, script: active.script)
        }
        let layout = settings.keyboardLayoutType
        return CustomLayout(
            base: layout == .tps ? .phahTaigi : layout,
            script: LayoutScript(inputMode: settings.romanizationInputMode),
        )
    }
}
