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
/// stay the base's. The editor only swaps, trades a key for a spare or removes one, so a key
/// never appears twice.
struct CustomLayout: Codable, Identifiable, Hashable {
    let id: UUID
    var name: String
    private(set) var base: KeyboardLayoutType
    private(set) var script: LayoutScript
    private(set) var keys: [String?]

    /// A removed key's place in `editorRows`; never on the keyboard.
    static let removedSlot = KeyDef.char("")

    /// Letters offered in the spare tray whatever the base, besides the base's own keys.
    private static let trayLetters = "abcdefghijklmnopqrstuvwxyz".map(String.init) + ["o\u{0358}", "-"]

    /// Punctuation and symbols offered in the spare tray: the common symbol page (the 123 key).
    private static let traySymbols = TaigiLayouts.Numeric.iPhone.joined()
        .compactMap(\.characterLabel)
        .filter { !trayLetters.contains($0) }

    /// Every key the tray can offer, so a moved key keeps its full-width form. The symbol page
    /// comes first: its `(` `)` carry full-width forms that MOE1's own keys leave out (a base's
    /// own keys still win, `rows(withGlobe:)`).
    private static let catalog: [String: KeyDef] = {
        let romanizationKeys = KeyboardLayoutType.romanizationLayouts.flatMap { base in
            [LayoutScript.tl, .poj].flatMap { characterKeys(of: baseTable(base, $0)) }
        }
        var catalog: [String: KeyDef] = [:]
        for key in Array(TaigiLayouts.Numeric.iPhone.joined()) + romanizationKeys {
            if let label = key.characterLabel, catalog[label] == nil {
                catalog[label] = key
            }
        }
        return catalog
    }()

    /// A fresh layout: the `base` × `script` table, unchanged.
    init(id: UUID = UUID(), name: String = "", base: KeyboardLayoutType, script: LayoutScript) {
        self.id = id
        self.name = name
        self.base = base
        self.script = script
        keys = Self.labels(of: Self.baseTable(base, script))
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

    /// Whether the keys still sit as in the base table.
    var isUnchanged: Bool {
        keys == baseLabels.map(Optional.some)
    }

    /// The key table the keyboard shows: removed keys leave their row (the rest keep their width),
    /// and a row left empty goes too.
    func rows(withGlobe: Bool) -> [[KeyDef]] {
        filledRows(withGlobe: withGlobe)
            .map { $0.filter { $0 != Self.removedSlot } }
            .filter { !$0.isEmpty }
    }

    /// The key table the editor shows: a removed key keeps its place as `removedSlot`, so every
    /// slot of `keys` stays on the board.
    var editorRows: [[KeyDef]] {
        filledRows(withGlobe: false)
    }

    /// Spare letters: the tray letters plus the base's own letter keys (MOE2 `ph`, `tsh` …).
    var spareLetters: [String] {
        spares(from: Self.trayLetters + baseLabels.filter { !Self.traySymbols.contains($0) })
    }

    /// Spare punctuation and symbols.
    var spareSymbols: [String] {
        spares(from: Self.traySymbols)
    }

    /// The index into `keys` of each key in `rows`: the character keys below the digit row, in
    /// reading order; nil for every other key.
    static func slotIndices(of rows: [[KeyDef]]) -> [[Int?]] {
        var next = 0
        return rows.enumerated().map { row, keys in
            keys.map { key in
                guard row > 0, key.isCharacter else { return nil }
                defer { next += 1 }
                return next
            }
        }
    }

    mutating func swapKeys(_ first: Int, _ second: Int) {
        keys.swapAt(first, second)
    }

    /// Puts `spare` on slot `index` (a removed one too); the key it replaces returns to the tray.
    mutating func replaceKey(at index: Int, with spare: String) {
        guard Self.catalog[spare] != nil, !keys.contains(spare) else { return }
        keys[index] = spare
    }

    /// Takes the key off slot `index`; it returns to the spare tray.
    mutating func removeKey(at index: Int) {
        keys[index] = nil
    }

    /// Restarts from `base`, keeping the script.
    mutating func reset(to base: KeyboardLayoutType) {
        self.base = base
        keys = Self.labels(of: Self.baseTable(base, script))
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
        for (row, slots) in Self.slotIndices(of: rows).enumerated() {
            for (column, slot) in slots.enumerated() {
                guard let slot else { continue }
                rows[row][column] = keys[slot].map { baseKeys[$0] ?? Self.catalog[$0] ?? .char($0) } ?? Self.removedSlot
            }
        }
        return rows
    }

    /// Spares from `pool`: the keys not on the board, each once.
    private func spares(from pool: [String]) -> [String] {
        let onBoard = Set(keys.compactMap(\.self))
        var seen = Set<String>()
        return pool.filter { !onBoard.contains($0) && seen.insert($0).inserted }
    }

    private static func baseTable(_ base: KeyboardLayoutType, _ script: LayoutScript) -> [[KeyDef]] {
        TaigiLayouts.Alphabetic.table(base, script: script, withGlobe: false)
    }

    /// The keys `slotIndices` numbers, in slot order.
    private static func characterKeys(of rows: [[KeyDef]]) -> [KeyDef] {
        rows.dropFirst().flatMap { $0.filter(\.isCharacter) }
    }

    private static func labels(of rows: [[KeyDef]]) -> [String] {
        characterKeys(of: rows).compactMap(\.characterLabel)
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
