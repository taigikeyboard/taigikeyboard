import SwiftUI
import UIKit

// Draws a key table as plain keys: the custom-layout editor board, and the custom-layout card
// preview on the Layout tab and in the keyboard's layout picker. Not the KeyboardKit view, so
// a key can be selected or dragged and nothing is typed.

/// With an `interaction`, the keys at a `CustomLayout.Place` (every key below the digit row but
/// globe) take taps, drags and dropped spares; without one the grid is an inert preview.
struct LayoutKeyGrid: View {
    /// The editor board's state and callbacks.
    struct Interaction {
        var selected: CustomLayout.Place?
        /// Places drawn as a dashed outline: removed keys the editor can put back.
        var removed: Set<CustomLayout.Place> = []
        /// The key being dragged, drawn faded at its place.
        var lifted: CustomLayout.Place?
        /// The key a drag would land on.
        var highlighted: CustomLayout.Place?
        /// VoiceOver name of a removed key's place (the grid has no strings of its own).
        var removedPlaceLabel = ""
        var onTap: (CustomLayout.Place) -> Void
        /// A key moved past the drag threshold, then on every move while lifted; the point is in
        /// `editorSpace`.
        var onDrag: (CustomLayout.Place, CGPoint) -> Void
        /// The finger lifted off a key, dragged or not; the point is in `editorSpace`.
        var onRelease: (CustomLayout.Place, CGPoint) -> Void
        /// The key's gesture is over — ended or cancelled by the system. Runs after `onRelease`
        /// when it ended, alone when it was cancelled.
        var onGestureEnd: () -> Void
        /// A spare dragged from the tray landed on a place; returns whether it was taken.
        var onDropSpare: (String, CustomLayout.Place) -> Bool
    }

    let rows: [[KeyDef]]
    var interaction: Interaction?

    /// Card preview aspect: the built-in `*_preview` screenshots (720×454,
    /// `PreviewAssetGeneratorTests`), so a drawn card lines up with them.
    static let cardAspectRatio: CGFloat = 720.0 / 454.0

    /// The editor's coordinate space: drag points and `KeyFrames` are in it.
    static let editorSpace = "layoutEditor"

    /// Each place's frame in `editorSpace`, so the editor can tell which key a drag is over.
    struct KeyFrames: PreferenceKey {
        static let defaultValue: [CustomLayout.Place: CGRect] = [:]

        static func reduce(value: inout [CustomLayout.Place: CGRect], nextValue: () -> [CustomLayout.Place: CGRect]) {
            value.merge(nextValue()) { $1 }
        }
    }

    /// Key widths in character-key units; a row of ten character keys fills the board.
    private static let rowUnits: CGFloat = 10
    /// Finger travel that turns a press into a drag.
    private static let dragThreshold: CGFloat = 8

    /// The place a spare dragged from the tray hovers over.
    @State private var spareTarget: CustomLayout.Place?
    /// True while a finger is on a key; resets on its own when the gesture ends or is cancelled.
    @GestureState private var isPressing = false

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
        .onChange(of: isPressing) { _, isPressing in
            if !isPressing {
                interaction?.onGestureEnd()
            }
        }
    }

    // MARK: - Keys

    @ViewBuilder
    private func key(_ keyDef: KeyDef, place: CustomLayout.Place?, fontSize: CGFloat, gap: CGFloat) -> some View {
        let isSelected = place != nil && place == interaction?.selected
        // A removed key's place: a dashed outline the editor can fill again.
        let isRemoved = place.map { interaction?.removed.contains($0) ?? false } ?? false
        let isTarget = place != nil && (place == interaction?.highlighted || place == spareTarget)
        let shape = RoundedRectangle(cornerRadius: gap * 1.5)
        let fill = isSelected ? Color.accentColor : isRemoved ? Color.clear : Self.keyColor
        let face = Self.label(keyDef, size: fontSize, isDigit: keyDef.isCharacter && place == nil)
            .opacity(isRemoved ? 0 : 1)
            .lineLimit(1)
            .minimumScaleFactor(0.5)
            .foregroundStyle(isSelected ? Color.white : Color.primary)
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .background(fill, in: shape)
            .overlay(shape.strokeBorder(Color.secondary, style: StrokeStyle(lineWidth: 1, dash: [3])).opacity(isRemoved && !isSelected && !isTarget ? 1 : 0))
            .overlay(shape.strokeBorder(Color.accentColor, lineWidth: 2.5).opacity(isTarget ? 1 : 0))
            .opacity(place != nil && place == interaction?.lifted ? 0.3 : 1)
            .padding(gap / 2)

        if let place, let interaction {
            face
                .contentShape(Rectangle())
                .background(GeometryReader { proxy in
                    Color.clear.preference(key: KeyFrames.self, value: [place: proxy.frame(in: .named(Self.editorSpace))])
                })
                .gesture(
                    DragGesture(minimumDistance: 0, coordinateSpace: .named(Self.editorSpace))
                        .updating($isPressing) { _, isPressing, _ in isPressing = true }
                        .onChanged { value in
                            // The threshold only starts a drag; once lifted, every move counts.
                            if interaction.lifted == place
                                || hypot(value.translation.width, value.translation.height) > Self.dragThreshold
                            {
                                interaction.onDrag(place, value.location)
                            }
                        }
                        .onEnded { value in interaction.onRelease(place, value.location) },
                )
                .dropDestination(for: String.self) { spares, _ in
                    spares.first.map { interaction.onDropSpare($0, place) } ?? false
                } isTargeted: { isTargeted in
                    if isTargeted {
                        spareTarget = place
                    } else if spareTarget == place {
                        spareTarget = nil
                    }
                }
                // Touch handling is the gesture above; VoiceOver activates through the action.
                .accessibilityElement(children: .ignore)
                .accessibilityLabel(isRemoved ? interaction.removedPlaceLabel : Self.accessibilityName(of: keyDef))
                .accessibilityAddTraits(isSelected ? [.isButton, .isSelected] : .isButton)
                .accessibilityAction { interaction.onTap(place) }
        } else {
            face.accessibilityHidden(interaction != nil)
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

    /// A key's face: its text at `size` points, or its symbol in a `size` square. Also the key the
    /// editor floats under a drag.
    @ViewBuilder
    static func label(_ keyDef: KeyDef, size: CGFloat, isDigit: Bool = false) -> some View {
        switch keyDef {
        case let .char(label, _): text(label, size: size).opacity(isDigit ? 0.5 : 1)
        case .shift: symbol("shift", size: size)
        case .backspace: symbol("delete.left", size: size)
        case .return: symbol(ButtonImageProvider.returnKeySymbolName(for: nil), size: size)
        case .globe: symbol("globe", size: size)
        case .emoji: symbol("face.smiling", size: size)
        // The keyboard's own 文/A icon in its default (outlined) state (`ButtonImageProvider`).
        case .translate: symbol("character.square", size: size)
        case .numeric: text("123", size: size)
        case .symbolic: text("#+=", size: size)
        case .alphabetic: text("ABC", size: size)
        // Not EmptyView: modifiers on it draw nothing, so the space bar would vanish.
        case .space: Color.clear
        }
    }

    private static func text(_ label: String, size: CGFloat) -> some View {
        Text(verbatim: label).font(.system(size: size))
    }

    /// `Image(latinSystemName:)` is a `UIImage` with its point size baked in, so `.font` cannot
    /// shrink it; it is scaled into the square instead, or it overflows a small card's keys.
    private static func symbol(_ name: String, size: CGFloat) -> some View {
        Image(latinSystemName: name)
            .resizable()
            .scaledToFit()
            .frame(width: size, height: size)
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

    // The default keyboard theme's tones: every key, function keys included, white on a gray board.
    private static let boardColor = Color(UIColor { $0.userInterfaceStyle == .dark ? .systemGray6 : .systemGray4 })
    static let keyColor = Color(UIColor { $0.userInterfaceStyle == .dark ? .systemGray2 : .white })
}
