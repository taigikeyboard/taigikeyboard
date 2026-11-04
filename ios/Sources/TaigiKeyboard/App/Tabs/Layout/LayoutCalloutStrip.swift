import SwiftUI

/// The selected key's long-press list, drawn as the keyboard's callout bubble in a fixed slot below
/// the board, its tail under the key. It always holds `CustomLayout.maxCallouts` slots: the list's
/// entries, then empty dashed slots (all dashed while no character key is selected). Slots are
/// edited like board keys, by tapping: the editor decides what a tap does.
struct LayoutCalloutStrip: View {
    @Environment(DisplayLanguageStore.self) private var lang
    /// The selected key's list; nil while no character key is selected.
    let entries: [String]?
    let selectedSlot: Int?
    /// A second slot drawn selected, marked to swap with `selectedSlot`.
    let partnerSlot: Int?
    /// The selected key's horizontal centre in `LayoutKeyGrid.editorSpace`, where the tail points.
    let tailX: CGFloat?
    let onTapSlot: (Int) -> Void

    // Eight equal slots fill the bubble, so it reads centred under the board.
    private static let cellHeight: CGFloat = 44
    private static let tailSize = CGSize(width: 16, height: 8)

    var body: some View {
        HStack(spacing: 2) {
            ForEach(0 ..< CustomLayout.maxCallouts, id: \.self, content: slot)
        }
        .padding(6)
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

    /// An entry, or an empty dashed slot past the list's end; drawn selected in the board's colour.
    private func slot(_ index: Int) -> some View {
        let entry = entries.flatMap { $0.indices.contains(index) ? $0[index] : nil }
        let isSelected = index == selectedSlot || index == partnerSlot
        let shape = RoundedRectangle(cornerRadius: 8)
        return Text(verbatim: entry ?? "")
            .font(.system(size: 22))
            .foregroundStyle(isSelected ? Color.white : Color.primary)
            .frame(maxWidth: .infinity, minHeight: Self.cellHeight, maxHeight: Self.cellHeight)
            .background(isSelected ? Color.accentColor : Color.clear, in: shape)
            .overlay(shape.strokeBorder(Color.secondary, style: StrokeStyle(lineWidth: 1, dash: [3])).opacity(entry == nil && !isSelected ? 1 : 0))
            .animation(nil, value: isSelected)
            .contentShape(shape)
            .onTapGesture { onTapSlot(index) }
            .disabled(entries == nil)
            .accessibilityLabel(entry.map { Text(verbatim: $0) } ?? Text(lang.string(.settingsSyllableSeparatorNone)))
            .accessibilityAddTraits(isSelected ? [.isButton, .isSelected] : .isButton)
    }
}
