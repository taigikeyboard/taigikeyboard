import SwiftUI
import UIKit

// Draws a key table as plain keys: the custom-layout editor board, and the custom-layout card
// preview on the Layout tab and in the keyboard's layout picker. Not the KeyboardKit view, so
// a key can be selected and nothing is typed.

/// The keys at a `CustomLayout.Place` (every key below the digit row but globe) are buttons when
/// `onTap` is set; every other key is drawn inert.
struct LayoutKeyGrid: View {
    let rows: [[KeyDef]]
    var selected: CustomLayout.Place?
    /// Places drawn as a dashed outline: removed keys the editor can put back.
    var removed: Set<CustomLayout.Place> = []
    var onTap: ((CustomLayout.Place) -> Void)?
    /// VoiceOver name of a removed key's place (the grid has no strings of its own).
    var removedPlaceLabel = ""

    /// Card preview aspect: the built-in `*_preview` screenshots (720×454,
    /// `PreviewAssetGeneratorTests`), so a drawn card lines up with them.
    static let cardAspectRatio: CGFloat = 720.0 / 454.0

    /// Key widths in character-key units; a row of ten character keys fills the board.
    private static let rowUnits: CGFloat = 10

    var body: some View {
        GeometryReader { geometry in
            let unit = geometry.size.width / Self.rowUnits
            let rowHeight = geometry.size.height / CGFloat(max(rows.count, 1))
            let gap = min(unit, rowHeight) * 0.1
            let places = CustomLayout.places(of: rows)
            VStack(spacing: 0) {
                ForEach(rows.indices, id: \.self) { row in
                    let widths = Self.widths(of: rows[row])
                    HStack(spacing: 0) {
                        ForEach(rows[row].indices, id: \.self) { column in
                            key(rows[row][column], place: places[row][column], fontSize: rowHeight * 0.4, gap: gap)
                                .frame(width: unit * widths[column], height: rowHeight)
                        }
                    }
                    .frame(maxWidth: .infinity)
                }
            }
        }
        .padding(2)
        .background(Self.boardColor)
    }

    // MARK: - Keys

    @ViewBuilder
    private func key(_ keyDef: KeyDef, place: CustomLayout.Place?, fontSize: CGFloat, gap: CGFloat) -> some View {
        let isSelected = place != nil && place == selected
        // A removed key's place: a dashed outline the editor can fill again.
        let isRemoved = place.map(removed.contains) ?? false
        let shape = RoundedRectangle(cornerRadius: gap * 1.5)
        let fill = isSelected ? Color.accentColor
            : isRemoved ? Color.clear
            : keyDef.isCharacter || keyDef == .space ? Self.characterKeyColor : Self.functionKeyColor
        let face = label(keyDef, isDigit: keyDef.isCharacter && place == nil)
            .opacity(isRemoved ? 0 : 1)
            .font(.system(size: fontSize))
            .lineLimit(1)
            .minimumScaleFactor(0.5)
            .foregroundStyle(isSelected ? Color.white : Color.primary)
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .background(fill, in: shape)
            .overlay(shape.strokeBorder(Color.secondary, style: StrokeStyle(lineWidth: 1, dash: [3])).opacity(isRemoved && !isSelected ? 1 : 0))
            .padding(gap / 2)

        if let place, let onTap {
            Button { onTap(place) } label: { face.contentShape(Rectangle()) }
                .buttonStyle(.plain)
                .accessibilityLabel(isRemoved ? removedPlaceLabel : Self.accessibilityName(of: keyDef))
                .accessibilityAddTraits(isSelected ? .isSelected : [])
        } else {
            face.accessibilityHidden(onTap != nil)
        }
    }

    /// VoiceOver name of a key: its text, or a symbol VoiceOver reads by its Unicode name
    /// (`Image(latinSystemName:)` carries no label of its own).
    static func accessibilityName(of keyDef: KeyDef) -> String {
        switch keyDef {
        case let .char(label, _): label
        case .shift: "⇧"
        case .backspace: "⌫"
        case .return: "⏎"
        case .globe: "🌐"
        case .emoji: "😊"
        case .space: "␣"
        case .translate: "文"
        case .numeric: "123"
        case .symbolic: "#+="
        case .alphabetic: "ABC"
        }
    }

    @ViewBuilder
    private func label(_ keyDef: KeyDef, isDigit: Bool) -> some View {
        switch keyDef {
        case let .char(label, _): Text(verbatim: label).opacity(isDigit ? 0.5 : 1)
        case .shift: Image(latinSystemName: "shift")
        case .backspace: Image(latinSystemName: "delete.left")
        case .return: Image(latinSystemName: ButtonImageProvider.returnKeySymbolName(for: nil))
        case .globe: Image(latinSystemName: "globe")
        case .emoji: Image(latinSystemName: "face.smiling")
        case .translate: Text(verbatim: "文")
        case .numeric: Text(verbatim: "123")
        case .symbolic: Text(verbatim: "#+=")
        case .alphabetic: Text(verbatim: "ABC")
        // Not EmptyView: modifiers on it draw nothing, so the space bar would vanish.
        case .space: Color.clear
        }
    }

    // MARK: - Geometry

    /// Key widths of one row in character-key units. Space takes what the row leaves (at least
    /// two keys wide); a row still wider than the board shrinks to fit it.
    private static func widths(of row: [KeyDef]) -> [CGFloat] {
        let fixedTotal = row.filter { $0 != .space }.map(fixedWidth(of:)).reduce(0, +)
        let spaceWidth = max(rowUnits - fixedTotal, 2)
        let widths = row.map { $0 == .space ? spaceWidth : fixedWidth(of: $0) }
        let scale = min(1, rowUnits / widths.reduce(0, +))
        return widths.map { $0 * scale }
    }

    private static func fixedWidth(of key: KeyDef) -> CGFloat {
        switch key {
        case .char, .space: 1
        case .shift, .backspace: 1.5
        case .return: 1.75
        case .numeric, .symbolic, .alphabetic, .globe, .emoji, .translate: 1.25
        }
    }

    // MARK: - Colors

    // System keyboard tones: light keys on a gray board, darker function keys.
    private static let boardColor = Color(UIColor { $0.userInterfaceStyle == .dark ? .systemGray6 : .systemGray4 })
    private static let characterKeyColor = Color(UIColor { $0.userInterfaceStyle == .dark ? .systemGray2 : .white })
    private static let functionKeyColor = Color(UIColor { $0.userInterfaceStyle == .dark ? .systemGray4 : .systemGray2 })
}
