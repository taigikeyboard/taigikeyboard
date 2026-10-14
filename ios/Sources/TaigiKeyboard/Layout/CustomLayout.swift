import Foundation

// A user-made romanization layout: a built-in key table with its character keys rearranged.

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

/// `keys` replaces, in reading order, every character key below the digit row of the
/// `base` × `script` table; nil marks a removed key. Function keys, the digit row and row shapes
/// stay the base's, except the emoji key, which can be removed. The editor only swaps, trades a
/// key for a spare or removes one, so a key never appears twice.
struct CustomLayout: Codable, Identifiable, Hashable {
    /// A key the editor can select: a slot of `keys`, or the emoji key.
    enum Place: Hashable {
        case key(Int)
        case emoji
    }

    let id: UUID
    var name: String
    private(set) var base: KeyboardLayoutType
    private(set) var script: LayoutScript
    private(set) var keys: [String?]
    private(set) var hasEmojiKey: Bool

    /// A removed key's place in `editorRows`; never on the keyboard.
    static let removedSlot = KeyDef.char("")

    /// The keys of every romanization table, both scripts.
    private static let romanizationKeys = KeyboardLayoutType.romanizationLayouts.flatMap { base in
        [LayoutScript.tl, .poj].flatMap { characterKeys(of: baseTable(base, $0)) }
    }

    private static let symbolPageKeys = Array(TaigiLayouts.Numeric.iPhone.joined()).filter(\.isCharacter)

    /// Letters offered in the spare tray: a–z, o͘, `-`, then every multi-letter key of the
    /// romanization layouts (MOE2 `ph`, `tsh`, `ch` …), whatever the base.
    private static let trayLetters: [String] = {
        let symbols = Set(symbolPageKeys.compactMap(\.characterLabel))
        let single = "abcdefghijklmnopqrstuvwxyz".map(String.init) + ["o\u{0358}", "-"]
        return unique(single + romanizationKeys.compactMap(\.characterLabel).filter { !symbols.contains($0) })
    }()

    /// Punctuation and symbols offered in the spare tray: the common symbol page (the 123 key).
    private static let traySymbols = symbolPageKeys.compactMap(\.characterLabel).filter { !trayLetters.contains($0) }

    /// Every key the tray can offer, so a moved key keeps its full-width form. The symbol page
    /// comes first: its `(` `)` carry full-width forms that MOE1's own keys leave out (a base's
    /// own keys still win, `filledRows`).
    private static let catalog: [String: KeyDef] = Dictionary(
        (symbolPageKeys + romanizationKeys).compactMap { key in key.characterLabel.map { ($0, key) } },
        uniquingKeysWith: { first, _ in first },
    )

    /// A fresh layout: the `base` × `script` table, unchanged.
    init(id: UUID = UUID(), name: String = "", base: KeyboardLayoutType, script: LayoutScript) {
        self.id = id
        self.name = name
        self.base = base
        self.script = script
        keys = Self.labels(of: Self.baseTable(base, script))
        hasEmojiKey = true
    }

    /// A record saved before the emoji key could be removed has it.
    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        id = try container.decode(UUID.self, forKey: .id)
        name = try container.decode(String.self, forKey: .name)
        base = try container.decode(KeyboardLayoutType.self, forKey: .base)
        script = try container.decode(LayoutScript.self, forKey: .script)
        keys = try container.decode([String?].self, forKey: .keys)
        hasEmojiKey = try container.decodeIfPresent(Bool.self, forKey: .hasEmojiKey) ?? true
    }

    /// Whether a decoded record can be shown: an editable base, one entry per slot, every key
    /// known, none twice. Records failing it are dropped on load.
    var isValid: Bool {
        guard base != .tps else { return false }
        let placed = keys.compactMap(\.self)
        return keys.count == baseLabels.count
            && placed.allSatisfy { Self.catalog[$0] != nil }
            && Set(placed).count == placed.count
    }

    /// The selected layout in `layouts` while its base is the stored layout: TPS entered through
    /// the input mode suspends it until leaving TPS restores the base.
    static func active(in layouts: [CustomLayout], selectedId: UUID?, layoutType: KeyboardLayoutType) -> CustomLayout? {
        guard let selectedId else { return nil }
        return layouts.first { $0.id == selectedId && $0.base == layoutType }
    }

    /// Whether the board still matches the base table.
    var isUnchanged: Bool {
        hasEmojiKey && keys == baseLabels.map(Optional.some)
    }

    /// The key table the keyboard shows: removed keys leave their row (the rest keep their width),
    /// and a row left empty goes too.
    func rows(withGlobe: Bool) -> [[KeyDef]] {
        filledRows(withGlobe: withGlobe)
            .map { $0.filter { $0 != Self.removedSlot && (hasEmojiKey || $0 != .emoji) } }
            .filter { !$0.isEmpty }
    }

    /// The key table the editor shows: every place stays on the board, a removed one included
    /// (see `removedPlaces`).
    var editorRows: [[KeyDef]] {
        filledRows(withGlobe: false)
    }

    var removedPlaces: Set<Place> {
        var places = Set(keys.indices.filter { keys[$0] == nil }.map(Place.key))
        if !hasEmojiKey {
            places.insert(.emoji)
        }
        return places
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
    /// `keys` in reading order, the emoji key is `.emoji`, every other key nil.
    static func places(of rows: [[KeyDef]]) -> [[Place?]] {
        var next = 0
        return rows.enumerated().map { row, keys in
            keys.map { key in
                if key == .emoji {
                    return .emoji
                }
                guard row > 0, key.isCharacter else { return nil }
                defer { next += 1 }
                return .key(next)
            }
        }
    }

    /// Swaps two character keys; the emoji key stays where it is.
    mutating func swap(_ first: Place, _ second: Place) {
        guard case let .key(firstIndex) = first, case let .key(secondIndex) = second else { return }
        keys.swapAt(firstIndex, secondIndex)
    }

    /// Puts `spare` on a character place (a removed one too); the key it replaces returns to the tray.
    mutating func replace(_ place: Place, with spare: String) {
        guard case let .key(index) = place, Self.catalog[spare] != nil, !keys.contains(spare) else { return }
        keys[index] = spare
    }

    /// Takes the key at `place` off the board; it returns to the spare tray.
    mutating func remove(_ place: Place) {
        switch place {
        case let .key(index): keys[index] = nil
        case .emoji: hasEmojiKey = false
        }
    }

    /// Puts the emoji key back in its place.
    mutating func restoreEmojiKey() {
        hasEmojiKey = true
    }

    /// Restarts from `base`, keeping the script.
    mutating func reset(to base: KeyboardLayoutType) {
        self.base = base
        keys = Self.labels(of: Self.baseTable(base, script))
        hasEmojiKey = true
    }

    // MARK: - Private

    private var baseLabels: [String] {
        Self.labels(of: Self.baseTable(base, script))
    }

    /// The base table with `keys` in its slots and `removedSlot` where a key was removed. A key
    /// the base has keeps the base's form (MOE1 `(` stays half-width); others come from `catalog`.
    private func filledRows(withGlobe: Bool) -> [[KeyDef]] {
        var rows = TaigiLayouts.Alphabetic.table(base, script: script, withGlobe: withGlobe)
        let baseKeys = Dictionary(
            Self.characterKeys(of: rows).compactMap { key in key.characterLabel.map { ($0, key) } },
            uniquingKeysWith: { first, _ in first },
        )
        for (row, places) in Self.places(of: rows).enumerated() {
            for (column, place) in places.enumerated() {
                guard case let .key(slot) = place else { continue }
                rows[row][column] = keys[slot].map { baseKeys[$0] ?? Self.catalog[$0] ?? .char($0) } ?? Self.removedSlot
            }
        }
        return rows
    }

    /// Spares from `pool`: the keys not on the board, each once.
    private func spares(from pool: [String]) -> [String] {
        let onBoard = Set(keys.compactMap(\.self))
        return pool.filter { !onBoard.contains($0) }
    }

    private static func baseTable(_ base: KeyboardLayoutType, _ script: LayoutScript) -> [[KeyDef]] {
        TaigiLayouts.Alphabetic.table(base, script: script, withGlobe: false)
    }

    /// The keys `places` numbers, in slot order.
    private static func characterKeys(of rows: [[KeyDef]]) -> [KeyDef] {
        rows.dropFirst().flatMap { $0.filter(\.isCharacter) }
    }

    private static func labels(of rows: [[KeyDef]]) -> [String] {
        characterKeys(of: rows).compactMap(\.characterLabel)
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
