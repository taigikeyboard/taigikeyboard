import SwiftUI

/// Custom-layout editor: base chips, the board (fixed) and the spare tray (scrolling). Drag a key
/// onto another to swap them (function keys within their row), a spare onto a key to replace it,
/// a key onto the tray to remove it. Taps do the same: tap two keys, or a key then a spare or the
/// trash. Nothing is stored until Save, which names the layout and applies it; Back discards the
/// draft.
struct LayoutEditorView: View {
    @Environment(DisplayLanguageStore.self) private var lang
    @Environment(\.dismiss) private var dismiss

    /// A board key under the finger: where it came from and where the finger is (`editorSpace`).
    private struct BoardDrag: Equatable {
        let source: CustomLayout.Place
        var location: CGPoint
    }

    /// The tray's frame in `editorSpace`: a board key dropped inside it is removed.
    private struct TrayFrame: PreferenceKey {
        static let defaultValue = CGRect.zero

        static func reduce(value: inout CGRect, nextValue: () -> CGRect) {
            value = nextValue()
        }
    }

    @State private var draft: CustomLayout
    @State private var selected: CustomLayout.Place?
    @State private var boardDrag: BoardDrag?
    /// Counts drags that changed the board, for the landing haptic.
    @State private var dropCount = 0
    @State private var keyFrames: [CustomLayout.Place: CGRect] = [:]
    @State private var trayFrame = CGRect.zero
    @State private var showsNameAlert = false
    @State private var pendingName = ""
    private let isEditing: Bool

    /// The drag payload of the tray's emoji chip: it only goes back to the emoji key's place.
    private static let emojiSpare = LayoutKeyGrid.accessibilityName(of: .emoji)

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
                    lifted: boardDrag?.source,
                    highlighted: dragTarget,
                    removedPlaceLabel: lang.string(.settingsSyllableSeparatorNone),
                    onTap: tap,
                    onDrag: drag,
                    onRelease: release,
                    onGestureEnd: endGesture,
                    onDropSpare: dropSpare,
                ),
            )
            .aspectRatio(GalleryCardMetrics.previewAspectRatio, contentMode: .fit)
            .clipShape(RoundedRectangle(cornerRadius: AppStyle.previewCornerRadius))
            .padding(.horizontal, AppStyle.horizontalPadding)

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
            .background(
                RoundedRectangle(cornerRadius: AppStyle.previewCornerRadius)
                    .strokeBorder(Color.red, lineWidth: 2)
                    .opacity(isOverTray ? 1 : 0)
                    .padding(.horizontal, AppStyle.horizontalPadding / 2),
            )
            .background(GeometryReader { proxy in
                Color.clear.preference(key: TrayFrame.self, value: proxy.frame(in: .named(LayoutKeyGrid.editorSpace)))
            })
        }
        .padding(.top, 8)
        .overlay(alignment: .topLeading) { liftedKey }
        .coordinateSpace(name: LayoutKeyGrid.editorSpace)
        .onPreferenceChange(LayoutKeyGrid.KeyFrames.self) { keyFrames = $0 }
        .onPreferenceChange(TrayFrame.self) { trayFrame = $0 }
        // A tick when a key lifts and when it reaches a new target; a soft tap when a drag lands.
        .sensoryFeedback(.selection, trigger: boardDrag?.source) { _, source in source != nil }
        .sensoryFeedback(.selection, trigger: dragTarget) { _, target in target != nil }
        .sensoryFeedback(.impact(flexibility: .soft), trigger: dropCount)
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

    /// Trash, the removed emoji key, letters, then punctuation and symbols. A spare is dragged
    /// onto a key, or tapped once a key is selected; the emoji chip goes straight back to its place.
    private var spareTray: some View {
        let canRemove = selected.map(draft.canRemove) ?? false
        return VStack(spacing: 16) {
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
                    .draggable(Self.emojiSpare)
                    .accessibilityLabel(Self.emojiSpare)
                }

                ForEach(draft.spareLetters, id: \.self, content: spareButton)
            }

            trayGrid {
                ForEach(draft.spareSymbols, id: \.self, content: spareButton)
            }
        }
        .padding(.horizontal, AppStyle.horizontalPadding)
    }

    /// Always draggable; a tap only does something once a character key is selected.
    private func spareButton(_ spare: String) -> some View {
        trayButton(action: { replaceSelected(with: spare) }) {
            Text(verbatim: spare)
        }
        .draggable(spare)
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

    /// The dragged key, lifted under the finger.
    @ViewBuilder
    private var liftedKey: some View {
        if let boardDrag, let keyDef = draft.keyDef(at: boardDrag.source) {
            LayoutKeyGrid.label(keyDef)
                .font(.system(size: 26))
                .foregroundStyle(Color.primary)
                .frame(width: 56, height: 60)
                .background(LayoutKeyGrid.characterKeyColor, in: RoundedRectangle(cornerRadius: 10))
                .shadow(color: .black.opacity(0.25), radius: 8, y: 4)
                .position(x: boardDrag.location.x, y: boardDrag.location.y - 36)
                .allowsHitTesting(false)
        }
    }

    // MARK: - Dragging

    /// The key a board drag would swap with, if it can.
    private var dragTarget: CustomLayout.Place? {
        guard let boardDrag else { return nil }
        return keyFrames.first { place, frame in
            frame.contains(boardDrag.location) && draft.canSwap(boardDrag.source, place)
        }?.key
    }

    /// Whether a board drag is over the tray with a key that can go there.
    private var isOverTray: Bool {
        guard let boardDrag else { return false }
        return trayFrame.contains(boardDrag.location) && draft.canRemove(boardDrag.source)
    }

    private func drag(_ place: CustomLayout.Place, to location: CGPoint) {
        if boardDrag == nil {
            selected = nil
        }
        boardDrag = BoardDrag(source: place, location: location)
    }

    /// Ends a drag where the finger lifted (swap, remove, or nothing), or treats a press that never
    /// lifted the key as a tap. A drag of another key, left by a cancelled gesture, never commits.
    private func release(_ place: CustomLayout.Place, at location: CGPoint) {
        guard boardDrag?.source == place else {
            boardDrag = nil
            tap(place)
            return
        }
        boardDrag?.location = location
        withAnimation(.snappy(duration: 0.2)) {
            if let dragTarget {
                draft.swap(place, dragTarget)
                dropCount += 1
            } else if isOverTray {
                draft.remove(place)
                dropCount += 1
            }
            boardDrag = nil
        }
    }

    /// The system cancelled a drag (no release): drop it without committing.
    private func endGesture() {
        boardDrag = nil
    }

    /// Takes a spare the tray offers; anything else (text dragged in from another app, a key
    /// already on the board) is refused.
    private func dropSpare(_ spare: String, on place: CustomLayout.Place) -> Bool {
        if spare == Self.emojiSpare {
            guard place == draft.emojiPlace, !draft.hasEmojiKey else { return false }
            restoreEmojiKey()
            dropCount += 1
            return true
        }
        guard case .key = place, draft.spareLetters.contains(spare) || draft.spareSymbols.contains(spare) else {
            return false
        }
        withAnimation(.snappy(duration: 0.2)) {
            draft.replace(place, with: spare)
            selected = nil
        }
        dropCount += 1
        return true
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
        guard let selected else { return }
        withAnimation(.snappy(duration: 0.2)) {
            draft.replace(selected, with: spare)
            self.selected = nil
        }
    }

    private func removeSelected() {
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

    private func restart(from base: KeyboardLayoutType) {
        withAnimation(.snappy(duration: 0.2)) {
            draft.reset(to: base)
            selected = nil
            boardDrag = nil
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
