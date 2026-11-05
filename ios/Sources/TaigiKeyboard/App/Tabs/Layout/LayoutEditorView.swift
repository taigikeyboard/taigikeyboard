import SwiftUI

/// Custom-layout editor: base chips, the board (fixed) and the spare tray (scrolling), all by taps.
/// Tap a key to select it, then a spare and the swap button to replace it, or the trash to remove it. Tap a second key to
/// mark both, then the swap button below the board to swap them (function keys within their row). The selected character key's
/// long-press list sits in `LayoutCalloutStrip` below the board: tap two slots and the swap button to
/// swap or move, a slot, a tray character and the swap button to fill it, a slot then the trash to empty it. The undo
/// button steps back one edit at a time. Nothing is stored until Save,
/// whose sheet names the layout and picks where it shows (`LayoutUse`), then applies it; Back discards
/// the draft.
struct LayoutEditorView: View {
    @Environment(DisplayLanguageStore.self) private var lang
    @Environment(\.dismiss) private var dismiss

    @State private var draft: CustomLayout
    @State private var selected: CustomLayout.Place?
    /// The selected slot of the selected key's long-press list (past its end = an empty slot).
    @State private var selectedCallout: Int?
    /// The key tapped before `selected`, marked with it for the swap button; nil unless the two can swap.
    @State private var swapPartner: CustomLayout.Place?
    /// The slot tapped before `selectedCallout`, marked with it for the swap button.
    @State private var calloutPartner: Int?
    /// The tray character marked with the selected slot or key for the swap button.
    @State private var markedSpare: String?
    /// The draft before each edit, oldest first.
    @State private var history: [CustomLayout] = []
    /// Set while undo writes the draft, so that write adds no history.
    @State private var isUndoing = false
    @State private var keyFrames: [CustomLayout.Place: CGRect] = [:]
    @State private var showsSaveSheet = false
    @State private var pendingName = ""
    @State private var pendingUse = LayoutUse.universal
    /// Set once the save sheet stored the draft, so closing the sheet also closes the editor.
    @State private var didSave = false
    private let isEditing: Bool

    init(editing layout: CustomLayout? = nil) {
        _draft = State(initialValue: layout ?? Self.newDraft())
        isEditing = layout != nil
    }

    /// Below this content height (a phone in landscape) the board and long-press strip leave the
    /// tray no room, so the whole editor scrolls instead of the tray alone. Content height, not
    /// `verticalSizeClass`, so a short iPad window counts too; 480 = Material's compact-height
    /// breakpoint the Android editor uses.
    private static let wholeScrollMaxHeight: CGFloat = 480

    var body: some View {
        GeometryReader { proxy in
            // At most half the height, so a short (landscape) screen still shows a usable board. Never
            // negative: the first layout pass can report a zero width.
            let boardWidth = max(0, min(
                proxy.size.width - 2 * AppStyle.horizontalPadding,
                proxy.size.height * 0.5 * GalleryCardMetrics.previewAspectRatio,
            ))
            if proxy.size.height < Self.wholeScrollMaxHeight {
                ScrollView {
                    VStack(spacing: 16) {
                        editorHeader(boardWidth: boardWidth)
                        spareTray
                            .padding(.vertical, 8)
                    }
                }
            } else {
                VStack(spacing: 16) {
                    editorHeader(boardWidth: boardWidth)
                    ScrollView {
                        spareTray
                            .padding(.vertical, 8)
                    }
                }
            }
        }
        .padding(.top, 8)
        .onPreferenceChange(LayoutKeyGrid.KeyFrames.self) { keyFrames = $0 }
        .background(Color(.systemGroupedBackground))
        .navigationTitle(isEditing ? draft.name : lang.string(.layoutEditorTitleNew))
        .navigationBarTitleDisplayMode(.inline)
        .toolbar {
            ToolbarItem(placement: .confirmationAction) {
                Button(lang.string(.commonSave)) {
                    pendingName = draft.name
                    pendingUse = draft.use
                    showsSaveSheet = true
                }
            }
        }
        .onChange(of: selected) {
            selectedCallout = nil
            calloutPartner = nil
            markedSpare = nil
            if selected == nil {
                swapPartner = nil
            }
        }
        .onChange(of: draft) { previous, _ in
            swapPartner = nil
            calloutPartner = nil
            markedSpare = nil
            if !isUndoing {
                history.append(previous)
            }
            isUndoing = false
        }
        .sheet(isPresented: $showsSaveSheet, onDismiss: closeAfterSave) {
            LayoutSaveSheet(name: $pendingName, use: $pendingUse, onSave: commit)
                .environment(lang)
        }
    }

    /// The label of the selected character key; nil for a function key, a removed key or none.
    private var selectedLabel: String? {
        guard let selected, case .key = selected, let label = draft.keyDef(at: selected)?.characterLabel,
              !label.isEmpty else { return nil }
        return label
    }

    // MARK: - Sections

    /// Base chips, the board and the long-press strip: fixed above the tray, or scrolling with it.
    private func editorHeader(boardWidth: CGFloat) -> some View {
        VStack(spacing: 16) {
            baseChips

            LayoutKeyGrid(
                rows: draft.editorRows,
                interaction: LayoutKeyGrid.Interaction(
                    // Only what the next action works on is lit: the bubble's tail still names the key.
                    selected: selectedCallout == nil ? selected : nil,
                    swapPartner: swapPartner,
                    removed: draft.removedPlaces,
                    removedPlaceLabel: lang.string(.settingsSyllableSeparatorNone),
                    onTap: tap,
                ),
            )
            .frame(width: boardWidth, height: boardWidth / GalleryCardMetrics.previewAspectRatio)
            .clipShape(RoundedRectangle(cornerRadius: AppStyle.previewCornerRadius))

            calloutStrip

            // Fixed between the bubble and the tray, in thumb reach and always in view.
            HStack(spacing: 12) {
                Button(action: swapMarked) {
                    Image(latinSystemName: "arrow.left.arrow.right")
                        .font(AppStyle.bodyFont)
                        .frame(maxWidth: .infinity)
                }
                .buttonStyle(.borderedProminent)
                .disabled(swapPartner == nil && calloutPartner == nil && markedSpare == nil)
                .accessibilityLabel(lang.string(.themePhotoHintMove))

                Button(action: undo) {
                    Image(latinSystemName: "arrow.uturn.backward")
                        .font(AppStyle.bodyFont)
                        .frame(maxWidth: .infinity)
                }
                .buttonStyle(.bordered)
                .disabled(history.isEmpty)
                .accessibilityLabel(lang.string(.desktopMenuUndo))

                Button(action: removeSelected) {
                    Image(latinSystemName: "trash")
                        .font(AppStyle.bodyFont)
                        .frame(maxWidth: .infinity)
                }
                .buttonStyle(.bordered)
                .tint(.red)
                .disabled(!canRemove)
                .accessibilityLabel(lang.string(.commonDelete))
            }
            .buttonBorderShape(.capsule)
            .controlSize(.large)
            .frame(width: boardWidth)
        }
        // Inside the scroll content, so key frames stay put (no preference churn) while the editor scrolls.
        .coordinateSpace(name: LayoutKeyGrid.editorSpace)
    }

    /// Always in place, so the tray never moves: empty slots until a character key is selected.
    private var calloutStrip: some View {
        LayoutCalloutStrip(
            entries: selectedLabel == nil ? nil : selectedCallouts,
            selectedSlot: selectedCallout,
            partnerSlot: calloutPartner,
            tailX: selected.flatMap { keyFrames[$0]?.midX },
            onTapSlot: tapCallout,
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

    private var canRemove: Bool {
        selectedCallout.map(selectedCallouts.indices.contains) ?? (selected.map(draft.canRemove) ?? false)
    }

    /// The removed emoji key and spare letters, shown when any, then titled groups: tone-marked letters,
    /// one row per base; half-width symbols; full-width symbols. A tap puts a character on the selected key
    /// or long-press slot (where it fits); the emoji chip goes straight back to its place.
    private var spareTray: some View {
        VStack(spacing: 28) {
            if !draft.hasEmojiKey || !draft.spareLetters.isEmpty {
                letterGrid
            }
            traySection(lang.string(.layoutTrayTones)) { toneGrid }
            traySection(lang.string(.symbolHalfWidth)) {
                symbolGrid(draft.traySymbols.filter { !CustomLayout.isFullWidth($0) })
            }
            traySection(lang.string(.symbolFullWidth)) {
                symbolGrid(draft.traySymbols.filter(CustomLayout.isFullWidth))
            }
        }
        .padding(.horizontal, AppStyle.horizontalPadding)
    }

    private func traySection(_ title: String, @ViewBuilder content: () -> some View) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(title)
                .font(AppStyle.sectionHeaderFont)
                .foregroundStyle(.secondary)
            content()
        }
    }

    private var letterGrid: some View {
        trayGrid {
            if !draft.hasEmojiKey {
                trayButton(action: restoreEmojiKey) {
                    Image(latinSystemName: "face.smiling")
                }
                .accessibilityLabel(LayoutKeyGrid.accessibilityName(of: .emoji))
            }

            ForEach(draft.spareLetters, id: \.self, content: spareButton)
        }
    }

    private func symbolGrid(_ symbols: [String]) -> some View {
        trayGrid {
            ForEach(symbols, id: \.self, content: spareButton)
        }
    }

    /// One row per base, columns aligned.
    private var toneGrid: some View {
        let columns = CustomLayout.toneRows.map(\.count).max() ?? 1
        return Grid(horizontalSpacing: 8, verticalSpacing: 8) {
            ForEach(CustomLayout.toneRows, id: \.self) { row in
                GridRow {
                    ForEach(0 ..< columns, id: \.self) { column in
                        if row.indices.contains(column) {
                            spareButton(row[column])
                        } else {
                            Color.clear.gridCellUnsizedAxes([.horizontal, .vertical])
                        }
                    }
                }
            }
        }
    }

    /// A tap marks the spare for the swap button once it can go on the selected key or slot. One that
    /// cannot stays in place, dimmed, so the rows keep their columns.
    private func spareButton(_ spare: String) -> some View {
        let isDimmed = selected != nil && !canPlace(spare)
        return trayButton(isSelected: spare == markedSpare, action: { markSpare(spare) }) {
            Text(verbatim: spare)
        }
        .disabled(isDimmed)
        .opacity(isDimmed ? 0.3 : 1)
    }

    private func trayGrid(@ViewBuilder content: () -> some View) -> some View {
        LazyVGrid(columns: [GridItem(.adaptive(minimum: 44), spacing: 8)], spacing: 8, content: content)
    }

    private func trayButton(isSelected: Bool = false, action: @escaping () -> Void, @ViewBuilder label: () -> some View) -> some View {
        Button(action: action) {
            label()
                .font(AppStyle.bodyFont)
                .foregroundStyle(isSelected ? Color.white : Color.primary)
                .frame(maxWidth: .infinity, minHeight: 44)
                .background(isSelected ? AppStyle.accentBlue : Color(.secondarySystemGroupedBackground), in: RoundedRectangle(cornerRadius: 8))
                .animation(nil, value: isSelected)
        }
        .buttonStyle(.plain)
        .accessibilityAddTraits(isSelected ? .isSelected : [])
    }

    // MARK: - Editing

    /// A tap never edits, so looking at another key's long-press list changes nothing. A second key
    /// that can swap with the selected one (`CustomLayout.canSwap`: character keys anywhere, function
    /// keys within their row) marks the pair for the swap button; a tap after a pair starts over.
    private func tap(_ place: CustomLayout.Place) {
        let isPairing = swapPartner == nil && selected.map { draft.canSwap($0, place) } == true
        let isDeselecting = swapPartner == nil && selected == place
        swapPartner = isPairing ? selected : nil
        selected = isDeselecting ? nil : place
    }

    /// Marks `spare` for the swap button, clearing any pair; tapping the marked spare clears it.
    private func markSpare(_ spare: String) {
        guard canPlace(spare) else { return }
        swapPartner = nil
        calloutPartner = nil
        markedSpare = spare == markedSpare ? nil : spare
    }

    /// Whether `spare` can go on the selected slot (the key's list does not hold it yet), else the
    /// selected key.
    private func canPlace(_ spare: String) -> Bool {
        if selectedCallout != nil {
            return selectedLabel.map { draft.holdsCallout(spare, for: $0) } != true
        }
        guard let selected else { return false }
        return draft.canReplace(selected, with: spare)
    }

    /// Places the marked spare, else swaps the marked long-press slots, else the marked keys.
    private func swapMarked() {
        if let markedSpare {
            self.markedSpare = nil
            replaceSelected(with: markedSpare)
            return
        }
        if let calloutPartner, let selectedCallout {
            swapCallouts(calloutPartner, selectedCallout)
            return
        }
        guard let selected, let swapPartner else { return }
        withAnimation(.snappy(duration: 0.2)) {
            draft.swap(swapPartner, selected)
            self.selected = nil
        }
    }

    private func undo() {
        guard let previous = history.popLast() else { return }
        withAnimation(.snappy(duration: 0.2)) {
            isUndoing = true
            draft = previous
            selected = nil
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
            editCallouts(nextSlot: index) { $0.remove(at: index) }
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

    /// A second slot marks the pair for the swap button when either holds an entry, and a tap after a
    /// pair starts over. Tapping the selected slot clears it.
    private func tapCallout(_ index: Int) {
        let filled = selectedCallouts.indices
        let isPairing = calloutPartner == nil
            && selectedCallout.map { $0 != index && (filled.contains($0) || filled.contains(index)) } == true
        let isDeselecting = calloutPartner == nil && selectedCallout == index
        swapPartner = nil
        markedSpare = nil
        calloutPartner = isPairing ? selectedCallout : nil
        selectedCallout = isDeselecting ? nil : index
    }

    /// Two entries swap; an entry paired with an empty slot moves to the list's end (entries stay
    /// packed at the front).
    private func swapCallouts(_ first: Int, _ second: Int) {
        // Cleared here too: an entry already last leaves the draft unchanged.
        calloutPartner = nil
        let filled = selectedCallouts.indices
        editCallouts(nextSlot: nil) { list in
            if filled.contains(first), filled.contains(second) {
                list.swapAt(first, second)
            } else {
                list.append(list.remove(at: filled.contains(first) ? first : second))
            }
        }
    }

    /// Puts `spare` in slot `index`: replaces an entry (which stays selected), or adds one in an empty
    /// slot and moves the selection to the next empty slot, so several characters go in without picking each slot.
    private func fillCallout(at index: Int, with spare: String) {
        let isAdding = !selectedCallouts.indices.contains(index)
        editCallouts(nextSlot: isAdding ? CustomLayout.maxCallouts : index) { list in
            if isAdding {
                list.append(spare)
            } else {
                list[index] = spare
            }
        }
    }

    /// The selected key's long-press list as shown; empty while no character key is selected.
    private var selectedCallouts: [String] {
        selectedLabel.map(draft.callouts(for:)) ?? []
    }

    /// Applies `change` to the selected key's list, then selects slot `nextSlot` (clamped to the first
    /// empty slot, or the last one once the list is full), or no slot when nil. Filling and removing
    /// keep a slot selected: with none, the next tray tap would replace the key itself and drop its list.
    private func editCallouts(nextSlot: Int?, _ change: (inout [String]) -> Void) {
        guard let selectedLabel else { return }
        var list = selectedCallouts
        withAnimation(.snappy(duration: 0.2)) {
            change(&list)
            draft.setCallouts(list, for: selectedLabel)
            // Read from the edited draft: a dropped repeat can leave the list unchanged or shorter.
            selectedCallout = nextSlot.map { min($0, selectedCallouts.count, CustomLayout.maxCallouts - 1) }
        }
    }

    private func restart(from base: KeyboardLayoutType) {
        withAnimation(.snappy(duration: 0.2)) {
            draft.reset(to: base)
            selected = nil
        }
    }

    /// Names the draft from the sheet (blank → "New Layout") and sets its use, saves and applies it,
    /// then closes the sheet and the editor. A failed write keeps the sheet open with the draft intact.
    private func commit() {
        let trimmed = pendingName.trimmingCharacters(in: .whitespacesAndNewlines)
        // A copy, so a failed save leaves the draft as it was.
        var layout = draft
        layout.name = trimmed.isEmpty ? lang.string(.layoutEditorTitleNew) : trimmed
        layout.use = pendingUse
        if SharedSettings.shared.saveCustomLayout(layout) {
            didSave = true
            showsSaveSheet = false
        }
    }

    private func closeAfterSave() {
        if didSave {
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

// MARK: - Save sheet

/// The name field and where the layout shows (Universal / Tâi-lô / Pe̍h-ōe-jī), with Cancel and Save.
private struct LayoutSaveSheet: View {
    @Environment(DisplayLanguageStore.self) private var lang
    @Environment(\.dismiss) private var dismiss
    @Binding var name: String
    @Binding var use: LayoutUse
    let onSave: () -> Void

    var body: some View {
        NavigationStack {
            Form {
                TextField(lang.string(.layoutEditorTitleNew), text: $name)
                Picker(lang.string(.settingsInputMode), selection: $use) {
                    ForEach(LayoutUse.allCases, id: \.self) { use in
                        Text(lang.string(use.nameKey)).tag(use)
                    }
                }
                .pickerStyle(.menu)
            }
            .navigationTitle(lang.string(.layoutNameHeader))
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button(lang.string(.commonCancel)) { dismiss() }
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button(lang.string(.commonSave), action: onSave)
                }
            }
        }
        .presentationDetents([.medium])
    }
}
