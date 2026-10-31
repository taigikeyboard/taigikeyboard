import SwiftUI

/// Custom-layout editor: base chips, the board (fixed) and the spare tray (scrolling), all by taps.
/// Tap two keys to swap them (function keys within their row), a key then a spare to replace it,
/// a key then the trash to remove it. The selected character key's long-press list sits in
/// `LayoutCalloutStrip` below the board: tap two slots to swap or move, a slot then a tray
/// character to fill it, a slot then the trash to empty it. Nothing is stored until Save, which
/// names the layout and applies it; Back discards the draft.
struct LayoutEditorView: View {
    @Environment(DisplayLanguageStore.self) private var lang
    @Environment(\.dismiss) private var dismiss

    @State private var draft: CustomLayout
    @State private var selected: CustomLayout.Place?
    /// The selected slot of the selected key's long-press list (past its end = an empty slot).
    @State private var selectedCallout: Int?
    @State private var keyFrames: [CustomLayout.Place: CGRect] = [:]
    @State private var showsNameAlert = false
    @State private var pendingName = ""
    private let isEditing: Bool

    init(editing layout: CustomLayout? = nil) {
        _draft = State(initialValue: layout ?? Self.newDraft())
        isEditing = layout != nil
    }

    var body: some View {
        VStack(spacing: 16) {
            baseChips

            LayoutKeyGrid(
                rows: draft.editorRows,
                interaction: LayoutKeyGrid.Interaction(
                    selected: selected,
                    removed: draft.removedPlaces,
                    removedPlaceLabel: lang.string(.settingsSyllableSeparatorNone),
                    onTap: tap,
                ),
            )
            .aspectRatio(GalleryCardMetrics.previewAspectRatio, contentMode: .fit)
            .clipShape(RoundedRectangle(cornerRadius: AppStyle.previewCornerRadius))
            .padding(.horizontal, AppStyle.horizontalPadding)

            calloutStrip

            ScrollView {
                VStack(spacing: 24) {
                    spareTray

                    Button(lang.string(.themeEditorResetAll), role: .destructive) {
                        restart(from: draft.base)
                    }
                    .font(AppStyle.bodyFont)
                    .disabled(draft.isUnchanged)
                }
                .padding(.vertical, 8)
            }
        }
        .padding(.top, 8)
        .coordinateSpace(name: LayoutKeyGrid.editorSpace)
        .onPreferenceChange(LayoutKeyGrid.KeyFrames.self) { keyFrames = $0 }
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
        .onChange(of: selected) { selectedCallout = nil }
        .alert(lang.string(.layoutNameHeader), isPresented: $showsNameAlert) {
            TextField(lang.string(.layoutEditorTitleNew), text: $pendingName)
            Button(lang.string(.commonSave)) { commit() }
            Button(lang.string(.commonCancel), role: .cancel) {}
        }
    }

    /// The label of the selected character key; nil for a function key, a removed key or none.
    private var selectedLabel: String? {
        guard let selected, case .key = selected, let label = draft.keyDef(at: selected)?.characterLabel,
              !label.isEmpty else { return nil }
        return label
    }

    // MARK: - Sections

    /// Always in place, so the tray never moves: empty slots until a character key is selected.
    private var calloutStrip: some View {
        LayoutCalloutStrip(
            entries: selectedLabel == nil ? nil : selectedCallouts,
            selectedSlot: selectedCallout,
            canReset: selectedLabel.map { draft.callouts[$0] != nil } ?? false,
            tailX: selected.flatMap { keyFrames[$0]?.midX },
            onTapSlot: tapCallout,
            onReset: {
                guard let selectedLabel else { return }
                withAnimation(.snappy(duration: 0.2)) {
                    draft.resetCallouts(for: selectedLabel)
                    selectedCallout = nil
                }
            },
        )
        .padding(.horizontal, AppStyle.horizontalPadding)
    }

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

    /// Three groups, gaps between them: trash, the removed emoji key and spare letters; tone-marked
    /// letters, one row per base; symbols, half-width then full-width. A tap puts a character on the
    /// selected key or long-press slot (where it fits); the emoji chip goes straight back to its place.
    private var spareTray: some View {
        let canRemove = selectedCallout.map(selectedCallouts.indices.contains) ?? (selected.map(draft.canRemove) ?? false)
        return VStack(spacing: 28) {
            trayGrid {
                trayButton(action: removeSelected) {
                    Image(latinSystemName: "trash")
                        .foregroundStyle(Color.red)
                }
                .disabled(!canRemove)
                .opacity(canRemove ? 1 : 0.4)
                .accessibilityLabel(lang.string(.commonDelete))

                if !draft.hasEmojiKey {
                    trayButton(action: restoreEmojiKey) {
                        Image(latinSystemName: "face.smiling")
                    }
                    .accessibilityLabel(LayoutKeyGrid.accessibilityName(of: .emoji))
                }

                ForEach(draft.spareLetters, id: \.self, content: spareButton)
            }

            toneGrid

            trayGrid {
                ForEach(draft.traySymbols, id: \.self, content: calloutSpareButton)
            }
        }
        .padding(.horizontal, AppStyle.horizontalPadding)
    }

    /// One row per base, columns aligned.
    private var toneGrid: some View {
        let columns = CustomLayout.toneRows.map(\.count).max() ?? 1
        return Grid(horizontalSpacing: 8, verticalSpacing: 8) {
            ForEach(CustomLayout.toneRows, id: \.self) { row in
                GridRow {
                    ForEach(0 ..< columns, id: \.self) { column in
                        if row.indices.contains(column) {
                            calloutSpareButton(row[column])
                        } else {
                            Color.clear.gridCellUnsizedAxes([.horizontal, .vertical])
                        }
                    }
                }
            }
        }
    }

    /// A spare that can also fill a long-press slot: one the selected key's list already holds stays
    /// in place, dimmed, so the rows keep their columns.
    private func calloutSpareButton(_ char: String) -> some View {
        let isHeld = selectedLabel.map { draft.holdsCallout(char, for: $0) } ?? false
        return spareButton(char)
            .disabled(isHeld)
            .opacity(isHeld ? 0.3 : 1)
    }

    /// A tap only does something once a character key or a long-press slot is selected.
    private func spareButton(_ spare: String) -> some View {
        trayButton(action: { replaceSelected(with: spare) }) {
            Text(verbatim: spare)
        }
    }

    private func trayGrid(@ViewBuilder content: () -> some View) -> some View {
        LazyVGrid(columns: [GridItem(.adaptive(minimum: 44), spacing: 8)], spacing: 8, content: content)
    }

    private func trayButton(action: @escaping () -> Void, @ViewBuilder label: () -> some View) -> some View {
        Button(action: action) {
            label()
                .font(AppStyle.bodyFont)
                .foregroundStyle(Color.primary)
                .frame(maxWidth: .infinity, minHeight: 44)
                .background(Color(.secondarySystemGroupedBackground), in: RoundedRectangle(cornerRadius: 8))
        }
        .buttonStyle(.plain)
    }

    // MARK: - Editing

    /// A second key swaps with the selected one where it can (`CustomLayout.canSwap`: character keys
    /// anywhere, function keys within their row); otherwise the tap moves or clears the selection.
    private func tap(_ place: CustomLayout.Place) {
        withAnimation(.snappy(duration: 0.2)) {
            if let current = selected, draft.canSwap(current, place) {
                draft.swap(current, place)
                selected = nil
            } else {
                selected = selected == place ? nil : place
            }
        }
    }

    private func replaceSelected(with spare: String) {
        if let index = selectedCallout {
            fillCallout(at: index, with: spare)
            return
        }
        guard let selected else { return }
        withAnimation(.snappy(duration: 0.2)) {
            draft.replace(selected, with: spare)
            self.selected = nil
        }
    }

    private func removeSelected() {
        if let index = selectedCallout {
            guard selectedCallouts.indices.contains(index) else { return }
            editCallouts { $0.remove(at: index) }
            return
        }
        guard let selected else { return }
        withAnimation(.snappy(duration: 0.2)) {
            draft.remove(selected)
            self.selected = nil
        }
    }

    private func restoreEmojiKey() {
        withAnimation(.snappy(duration: 0.2)) {
            draft.restoreEmojiKey()
            selected = nil
        }
    }

    // MARK: - Long-press list

    /// A second slot swaps with the selected one when both hold entries; an entry tapped with an
    /// empty slot moves to the list's end (entries stay packed at the front). Tapping the selected
    /// slot clears it.
    private func tapCallout(_ index: Int) {
        guard let current = selectedCallout, current != index else {
            selectedCallout = selectedCallout == index ? nil : index
            return
        }
        let filled = selectedCallouts.indices
        switch (filled.contains(current), filled.contains(index)) {
        case (true, true):
            editCallouts { $0.swapAt(current, index) }
        case (true, false):
            editCallouts { $0.append($0.remove(at: current)) }
        case (false, true):
            editCallouts { $0.append($0.remove(at: index)) }
        case (false, false):
            selectedCallout = index
        }
    }

    /// Puts `spare` in slot `index`: replaces an entry, or adds one in an empty slot and moves the
    /// selection to the next empty slot, so several characters go in one tap each.
    private func fillCallout(at index: Int, with spare: String) {
        let isAdding = !selectedCallouts.indices.contains(index)
        editCallouts { list in
            if isAdding {
                list.append(spare)
            } else {
                list[index] = spare
            }
        }
        if isAdding, selectedCallouts.count < CustomLayout.maxCallouts {
            selectedCallout = selectedCallouts.count
        }
    }

    /// The selected key's long-press list as shown; empty while no character key is selected.
    private var selectedCallouts: [String] {
        selectedLabel.map(draft.callouts(for:)) ?? []
    }

    /// Applies `change` to the selected key's list and clears the entry selection.
    private func editCallouts(_ change: (inout [String]) -> Void) {
        guard let selectedLabel else { return }
        var list = selectedCallouts
        withAnimation(.snappy(duration: 0.2)) {
            change(&list)
            draft.setCallouts(list, for: selectedLabel)
            selectedCallout = nil
        }
    }

    private func restart(from base: KeyboardLayoutType) {
        withAnimation(.snappy(duration: 0.2)) {
            draft.reset(to: base)
            selected = nil
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
