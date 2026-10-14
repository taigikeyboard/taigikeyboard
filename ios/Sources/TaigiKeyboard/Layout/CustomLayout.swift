import Foundation

// A user-made romanization layout: a built-in key table with its keys rearranged.

/// The script of the key table a custom layout was made from — Lohankha and the MOE layouts have
/// different TL and POJ tables (`TaigiLayouts.Alphabetic.table`).
enum LayoutScript: String, Codable {
    case tl
    case poj

    /// POJ for `.poj`; TL for every other mode (English and TPS never pick a romanization table).
    init(inputMode: InputMode) {
        self = inputMode == .poj ? .poj : .tl
    }

    var inputMode: InputMode {
        switch self {
        case .tl: .tl
        case .poj: .poj
        }
    }
}

/// A custom layout over the `base` × `script` table, below its digit row:
/// - `keys` fills the character keys in reading order; nil marks a removed key.
/// - `functionKeys` fills the function keys in reading order; each stays in its own row.
/// - `hasEmojiKey` false hides the emoji key, the one function key that can be removed.
/// The globe key is not part of it: the keyboard puts it right after 123 when the device needs it.
/// The editor only swaps, trades a key for a spare or removes one, so a key never appears twice.
struct CustomLayout: Codable, Identifiable, Hashable {
    /// A key the editor can select: a slot of `keys`, or a slot of `functionKeys`.
    enum Place: Hashable {
        case key(Int)
        case function(Int)
    }

    /// A function key as stored in a record.
    enum FunctionKey: String, Codable {
        case shift, backspace, space, `return`, translate, numeric, symbolic, alphabetic, emoji

        /// nil for a character key and for the globe key.
        init?(_ keyDef: KeyDef) {
            switch keyDef {
            case .shift: self = .shift
            case .backspace: self = .backspace
            case .space: self = .space
            case .return: self = .return
            case .translate: self = .translate
            case .numeric: self = .numeric
            case .symbolic: self = .symbolic
            case .alphabetic: self = .alphabetic
            case .emoji: self = .emoji
            case .char, .globe: return nil
            }
        }

        var keyDef: KeyDef {
            switch self {
            case .shift: .shift
            case .backspace: .backspace
            case .space: .space
            case .return: .return
            case .translate: .translate
            case .numeric: .numeric
            case .symbolic: .symbolic
            case .alphabetic: .alphabetic
            case .emoji: .emoji
            }
        }
    }

    let id: UUID
    var name: String
    private(set) var base: KeyboardLayoutType
    private(set) var script: LayoutScript
    private(set) var keys: [String?]
    private(set) var functionKeys: [FunctionKey]
    private(set) var hasEmojiKey: Bool

    /// A removed key's place in `editorRows`; never on the keyboard.
    static let removedSlot = KeyDef.char("")

    /// The keys of every romanization table, both scripts.
    private static let romanizationKeys = KeyboardLayoutType.romanizationLayouts.flatMap { base in
        [LayoutScript.tl, .poj].flatMap { characterKeys(of: baseTable(base, $0)) }
    }

    private static let symbolPageKeys = Array(TaigiLayouts.Numeric.iPhone.joined()).filter(\.isCharacter)

    /// Finals spelled the same in TL and POJ, most frequent first (share of dictionary syllables
    /// whose final contains them). No built-in table has them; each is a plain key.
    private static let trayFinals = [
        "an", "ia", "in", "au", "ong", "ian", "un", "ai", "iu", "ann", "ang", "ui", "inn", "iau", "iong", "er", "ir",
    ]

    /// Letters offered in the spare tray: a–z, o͘, `-`, then every multi-letter key of the
    /// romanization layouts (MOE2 `ph`, `tsh`, `ch` …), whatever the base, then `trayFinals`.
    private static let trayLetters: [String] = {
        let symbols = Set(symbolPageKeys.compactMap(\.characterLabel))
        let single = "abcdefghijklmnopqrstuvwxyz".map(String.init) + ["o\u{0358}", "-"]
        return unique(single + romanizationKeys.compactMap(\.characterLabel).filter { !symbols.contains($0) } + trayFinals)
    }()

    /// Punctuation and symbols offered in the spare tray: the common symbol page (the 123 key).
    private static let traySymbols = symbolPageKeys.compactMap(\.characterLabel).filter { !trayLetters.contains($0) }

    /// Every key the tray can offer, so a moved key keeps its full-width form. The symbol page
    /// comes first: its `(` `)` carry full-width forms that MOE1's own keys leave out (a base's
    /// own keys still win, `filledRows`).
    private static let catalog: [String: KeyDef] = Dictionary(
        (symbolPageKeys + romanizationKeys + trayFinals.map { KeyDef.char($0) })
            .compactMap { key in key.characterLabel.map { ($0, key) } },
        uniquingKeysWith: { first, _ in first },
    )

    /// A fresh layout: the `base` × `script` table, unchanged.
    init(id: UUID = UUID(), name: String = "", base: KeyboardLayoutType, script: LayoutScript) {
        self.id = id
        self.name = name
        self.base = base
        self.script = script
        keys = Self.labels(of: Self.baseTable(base, script))
        functionKeys = Self.functionKeys(of: Self.baseTable(base, script))
        hasEmojiKey = true
    }

    /// A record saved before function keys could move or the emoji key could go keeps the base's.
    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        id = try container.decode(UUID.self, forKey: .id)
        name = try container.decode(String.self, forKey: .name)
        base = try container.decode(KeyboardLayoutType.self, forKey: .base)
        script = try container.decode(LayoutScript.self, forKey: .script)
        keys = try container.decode([String?].self, forKey: .keys)
        functionKeys = try container.decodeIfPresent([FunctionKey].self, forKey: .functionKeys)
            ?? Self.functionKeys(of: Self.baseTable(base, script))
        hasEmojiKey = try container.decodeIfPresent(Bool.self, forKey: .hasEmojiKey) ?? true
    }

    /// Whether a decoded record can be shown: an editable base, one entry per slot, every key
    /// known, none twice, each row's function keys the base row's. Records failing it are dropped
    /// on load.
    var isValid: Bool {
        guard base != .tps else { return false }
        let placed = keys.compactMap(\.self)
        return keys.count == baseLabels.count
            && placed.allSatisfy { Self.catalog[$0] != nil }
            && Set(placed).count == placed.count
            && functionKeysByRow(functionKeys) == functionKeysByRow(baseFunctionKeys)
    }

    /// The selected layout in `layouts` while its base is the stored layout: TPS entered through
    /// the input mode suspends it until leaving TPS restores the base.
    static func active(in layouts: [CustomLayout], selectedId: UUID?, layoutType: KeyboardLayoutType) -> CustomLayout? {
        guard let selectedId else { return nil }
        return layouts.first { $0.id == selectedId && $0.base == layoutType }
    }

    /// Whether the board still matches the base table.
    var isUnchanged: Bool {
        hasEmojiKey && keys == baseLabels.map(Optional.some) && functionKeys == baseFunctionKeys
    }

    /// The key table the keyboard shows: removed keys leave their row (the rest keep their width),
    /// and a row left empty goes too.
    func rows(withGlobe: Bool) -> [[KeyDef]] {
        var rows = filledRows
            .map { $0.filter { $0 != Self.removedSlot && (hasEmojiKey || $0 != .emoji) } }
            .filter { !$0.isEmpty }
        if withGlobe, var bottomRow = rows.popLast() {
            bottomRow.insert(.globe, at: (bottomRow.firstIndex(of: .numeric) ?? 0) + 1)
            rows.append(bottomRow)
        }
        return rows
    }

    /// The key table the editor shows: every place stays on the board, a removed one included
    /// (see `removedPlaces`).
    var editorRows: [[KeyDef]] {
        filledRows
    }

    /// The key at `place` on the editor board.
    func keyDef(at place: Place) -> KeyDef? {
        let rows = editorRows
        for (row, places) in Self.places(of: rows).enumerated() {
            if let column = places.firstIndex(of: place) {
                return rows[row][column]
            }
        }
        return nil
    }

    var removedPlaces: Set<Place> {
        var places = Set(keys.indices.filter { keys[$0] == nil }.map(Place.key))
        if !hasEmojiKey, let emojiPlace {
            places.insert(emojiPlace)
        }
        return places
    }

    /// The emoji key's place.
    var emojiPlace: Place? {
        functionKeys.firstIndex(of: .emoji).map(Place.function)
    }

    /// Spare letters, multi-letter keys included.
    var spareLetters: [String] {
        spares(from: Self.trayLetters)
    }

    /// Spare punctuation and symbols.
    var spareSymbols: [String] {
        spares(from: Self.traySymbols)
    }

    /// The place of each key in `rows`: the character keys below the digit row number the slots of
    /// `keys` and the function keys those of `functionKeys`, in reading order; nil for the digit
    /// row and the globe key.
    static func places(of rows: [[KeyDef]]) -> [[Place?]] {
        var nextKey = 0
        var nextFunction = 0
        return rows.enumerated().map { row, keys in
            keys.map { key in
                guard row > 0 else { return nil }
                if key.isCharacter {
                    defer { nextKey += 1 }
                    return .key(nextKey)
                }
                guard FunctionKey(key) != nil else { return nil }
                defer { nextFunction += 1 }
                return .function(nextFunction)
            }
        }
    }

    /// Character keys swap anywhere; function keys only within their row.
    func canSwap(_ first: Place, _ second: Place) -> Bool {
        switch (first, second) {
        case let (.key(firstIndex), .key(secondIndex)):
            firstIndex != secondIndex
        case let (.function(firstIndex), .function(secondIndex)):
            firstIndex != secondIndex && baseFunctionRows[firstIndex] == baseFunctionRows[secondIndex]
        default:
            false
        }
    }

    mutating func swap(_ first: Place, _ second: Place) {
        guard canSwap(first, second) else { return }
        switch (first, second) {
        case let (.key(firstIndex), .key(secondIndex)): keys.swapAt(firstIndex, secondIndex)
        case let (.function(firstIndex), .function(secondIndex)): functionKeys.swapAt(firstIndex, secondIndex)
        default: break
        }
    }

    /// Puts `spare` on a character place (a removed one too); the key it replaces returns to the tray.
    mutating func replace(_ place: Place, with spare: String) {
        guard case let .key(index) = place, Self.catalog[spare] != nil, !keys.contains(spare) else { return }
        keys[index] = spare
    }

    /// Whether the key at `place` can go to the tray: a character key, or the emoji key.
    func canRemove(_ place: Place) -> Bool {
        switch place {
        case let .key(index): keys[index] != nil
        case .function: place == emojiPlace && hasEmojiKey
        }
    }

    /// Takes the key at `place` off the board; it returns to the spare tray.
    mutating func remove(_ place: Place) {
        guard canRemove(place) else { return }
        switch place {
        case let .key(index): keys[index] = nil
        case .function: hasEmojiKey = false
        }
    }

    /// Puts the emoji key back in its place.
    mutating func restoreEmojiKey() {
        hasEmojiKey = true
    }

    /// Restarts from `base`, keeping the script.
    mutating func reset(to base: KeyboardLayoutType) {
        self.base = base
        keys = baseLabels
        functionKeys = baseFunctionKeys
        hasEmojiKey = true
    }

    // MARK: - Private

    private var baseLabels: [String] {
        Self.labels(of: Self.baseTable(base, script))
    }

    private var baseFunctionKeys: [FunctionKey] {
        Self.functionKeys(of: Self.baseTable(base, script))
    }

    /// The row of each `functionKeys` slot in the base table.
    private var baseFunctionRows: [Int] {
        Self.baseTable(base, script).enumerated().dropFirst().flatMap { row, keys in
            keys.compactMap(FunctionKey.init).map { _ in row }
        }
    }

    /// Each row's function keys, order ignored — what a reorder must keep.
    private func functionKeysByRow(_ keys: [FunctionKey]) -> [Int: [String]] {
        let rows = baseFunctionRows
        guard keys.count == rows.count else { return [:] }
        return Dictionary(grouping: zip(rows, keys), by: { $0.0 }).mapValues { $0.map(\.1.rawValue).sorted() }
    }

    /// The base table with `keys` and `functionKeys` in their slots and `removedSlot` where a
    /// character key was removed. A character key the base has keeps the base's form (MOE1 `(`
    /// stays half-width); others come from `catalog`.
    private var filledRows: [[KeyDef]] {
        var rows = Self.baseTable(base, script)
        let baseKeys = Dictionary(
            Self.characterKeys(of: rows).compactMap { key in key.characterLabel.map { ($0, key) } },
            uniquingKeysWith: { first, _ in first },
        )
        for (row, places) in Self.places(of: rows).enumerated() {
            for (column, place) in places.enumerated() {
                switch place {
                case let .key(slot):
                    rows[row][column] = keys[slot].map { baseKeys[$0] ?? Self.catalog[$0] ?? .char($0) } ?? Self.removedSlot
                case let .function(slot):
                    rows[row][column] = functionKeys[slot].keyDef
                case nil:
                    continue
                }
            }
        }
        return rows
    }

    /// Spares from `pool`: the keys not on the board.
    private func spares(from pool: [String]) -> [String] {
        let onBoard = Set(keys.compactMap(\.self))
        return pool.filter { !onBoard.contains($0) }
    }

    /// The iPhone (globe-less) table.
    private static func baseTable(_ base: KeyboardLayoutType, _ script: LayoutScript) -> [[KeyDef]] {
        TaigiLayouts.Alphabetic.table(base, script: script, withGlobe: false)
    }

    /// The character keys `places` numbers, in slot order.
    private static func characterKeys(of rows: [[KeyDef]]) -> [KeyDef] {
        rows.dropFirst().flatMap { $0.filter(\.isCharacter) }
    }

    private static func labels(of rows: [[KeyDef]]) -> [String] {
        characterKeys(of: rows).compactMap(\.characterLabel)
    }

    /// The function keys `places` numbers, in slot order.
    private static func functionKeys(of rows: [[KeyDef]]) -> [FunctionKey] {
        rows.dropFirst().flatMap { $0.compactMap(FunctionKey.init) }
    }

    private static func unique(_ labels: [String]) -> [String] {
        var seen = Set<String>()
        return labels.filter { seen.insert($0).inserted }
    }
}

extension KeyDef {
    var isCharacter: Bool {
        characterLabel != nil
    }

    /// The half-width label of a `.char` key; nil for function keys.
    var characterLabel: String? {
        if case let .char(label, _) = self {
            return label
        }
        return nil
    }
}

// Raw-value Codable, so a stored record keeps the layout id string shared with `SharedSettings`.
extension KeyboardLayoutType: Codable {}
