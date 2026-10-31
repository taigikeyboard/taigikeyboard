import SwiftUI

/// The selected key's long-press list, drawn as the keyboard's callout bubble in a fixed slot below
/// the board, its tail under the key. Entries work like board keys: tap two to swap them, or drag
/// one onto another; a spare from the tray replaces an entry, or joins the list on `+`, which also
/// takes typed text. With no character key selected the slot shows empty dashed cells.
struct LayoutCalloutStrip: View {
    struct Interaction {
        let onTapEntry: (Int) -> Void
        let onTapAdd: () -> Void
        /// A payload dropped on an entry (nil = on `+`); returns whether it was taken.
        let onDrop: (String, Int?) -> Bool
        let onReset: () -> Void
    }

    /// Where a drag hovers: an entry's index, or `+`.
    private enum DropSlot: Equatable {
        case entry(Int)
        case add
    }

    @Environment(DisplayLanguageStore.self) private var lang
    /// The selected key's list; nil while no character key is selected.
    let entries: [String]?
    let selectedIndex: Int?
    let canAdd: Bool
    let canReset: Bool
    /// The selected key's horizontal centre in `LayoutKeyGrid.editorSpace`, where the tail points.
    let tailX: CGFloat?
    let interaction: Interaction
    @State private var targeted: DropSlot?

    // Eight cells (a full list, or seven and `+`) fit the bubble on the narrowest iPhone.
    private static let cellSize = CGSize(width: 34, height: 44)
    private static let tailSize = CGSize(width: 16, height: 8)
    private static let placeholderCount = 5
    /// Marks an entry dragged out of the bubble, so a drop can tell it from a tray spare.
    private static let entryPayloadPrefix = "\u{F8FF}callout:"

    /// The index an entry payload carries; nil for anything else.
    static func entryIndex(ofPayload payload: String) -> Int? {
        guard payload.hasPrefix(entryPayloadPrefix) else { return nil }
        return Int(payload.dropFirst(entryPayloadPrefix.count))
    }

    var body: some View {
        HStack(spacing: 12) {
            bubble
            Button(action: interaction.onReset) {
                Image(latinSystemName: "arrow.counterclockwise")
                    .font(AppStyle.bodyFont)
            }
            .disabled(!canReset)
            .accessibilityLabel(lang.string(.themeEditorResetAll))
        }
    }

    private var bubble: some View {
        ScrollView(.horizontal, showsIndicators: false) {
            HStack(spacing: 2) {
                if let entries {
                    ForEach(Array(entries.enumerated()), id: \.element) { index, entry in
                        entryCell(entry, at: index)
                    }
                    if canAdd {
                        addCell
                    }
                } else {
                    ForEach(0 ..< Self.placeholderCount, id: \.self) { _ in
                        RoundedRectangle(cornerRadius: 8)
                            .strokeBorder(Color.secondary, style: StrokeStyle(lineWidth: 1, dash: [3]))
                            .frame(width: Self.cellSize.width, height: Self.cellSize.height)
                    }
                }
            }
            .padding(6)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .background {
            // The shadow on the shape alone, so the entries stay crisp.
            RoundedRectangle(cornerRadius: 12)
                .fill(LayoutKeyGrid.keyColor)
                .shadow(color: .black.opacity(0.15), radius: 4, y: 1)
        }
        .overlay(alignment: .topLeading) { tail }
        .accessibilityElement(children: .contain)
        .accessibilityLabel(lang.string(.layoutCalloutsTitle))
    }

    /// A triangle on the bubble's top edge, pointing up at the selected key.
    private var tail: some View {
        GeometryReader { proxy in
            if let tailX, entries != nil {
                let bubbleX = proxy.frame(in: .named(LayoutKeyGrid.editorSpace)).minX
                let x = min(max(tailX - bubbleX, 16), proxy.size.width - 16)
                Path { path in
                    path.move(to: CGPoint(x: x - Self.tailSize.width / 2, y: 0))
                    path.addLine(to: CGPoint(x: x, y: -Self.tailSize.height))
                    path.addLine(to: CGPoint(x: x + Self.tailSize.width / 2, y: 0))
                    path.closeSubpath()
                }
                .fill(LayoutKeyGrid.keyColor)
            }
        }
        .allowsHitTesting(false)
    }

    private func entryCell(_ entry: String, at index: Int) -> some View {
        let isSelected = index == selectedIndex
        let shape = RoundedRectangle(cornerRadius: 8)
        return Text(verbatim: entry)
            .font(.system(size: 22))
            .foregroundStyle(isSelected ? Color.white : Color.primary)
            .frame(width: Self.cellSize.width, height: Self.cellSize.height)
            .background(isSelected ? Color.accentColor : Color.clear, in: shape)
            .overlay(shape.strokeBorder(Color.accentColor, lineWidth: 2.5).opacity(targeted == .entry(index) ? 1 : 0))
            .contentShape(shape)
            .onTapGesture { interaction.onTapEntry(index) }
            .draggable(Self.entryPayloadPrefix + String(index)) {
                Text(verbatim: entry).font(.system(size: 22))
            }
            .dropDestination(for: String.self) { items, _ in
                items.first.map { interaction.onDrop($0, index) } ?? false
            } isTargeted: { targeted = $0 ? .entry(index) : nil }
            .accessibilityLabel(Text(verbatim: entry))
            .accessibilityAddTraits(isSelected ? .isSelected : [])
    }

    private var addCell: some View {
        let shape = RoundedRectangle(cornerRadius: 8)
        return Image(latinSystemName: "plus")
            .font(AppStyle.bodyFont)
            .foregroundStyle(Color.accentColor)
            .frame(width: Self.cellSize.width, height: Self.cellSize.height)
            .overlay(shape.strokeBorder(Color.accentColor, lineWidth: 2.5).opacity(targeted == .add ? 1 : 0))
            .contentShape(shape)
            .onTapGesture(perform: interaction.onTapAdd)
            .dropDestination(for: String.self) { items, _ in
                items.first.map { interaction.onDrop($0, nil) } ?? false
            } isTargeted: { targeted = $0 ? .add : nil }
            .accessibilityLabel(lang.string(.layoutCalloutsAddPlaceholder))
            .accessibilityAddTraits(.isButton)
    }
}
