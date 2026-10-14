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
/// `base` × `script` table; function keys, the digit row and row shapes stay the base's.
/// The editor only swaps keys or trades one for a spare, so a key never appears twice.
struct CustomLayout: Codable, Identifiable, Hashable {
    let id: UUID
    var name: String
    private(set) var base: KeyboardLayoutType
    private(set) var script: LayoutScript
    private(set) var keys: [String]

    /// Letters offered in the spare tray whatever the base, besides the base's own keys.
    private static let trayLetters = "abcdefghijklmnopqrstuvwxyz".map(String.init) + ["o\u{0358}", "-"]

    /// Every character key of the romanization tables, so a moved key keeps its full-width form.
    private static let catalog: [String: KeyDef] = {
        var catalog: [String: KeyDef] = [:]
        for base in KeyboardLayoutType.romanizationLayouts {
            for script in [LayoutScript.tl, .poj] {
                for key in characterKeys(of: baseTable(base, script)) {
                    if case let .char(label, _) = key, catalog[label] == nil {
                        catalog[label] = key
                    }
                }
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

    /// Whether a decoded record can be shown: an editable base, one key per slot, every key known,
    /// none twice. Records failing it are dropped on load.
    var isValid: Bool {
        guard base != .tps else { return false }
        return keys.count == baseLabels.count
            && keys.allSatisfy { Self.catalog[$0] != nil }
            && Set(keys).count == keys.count
    }

    /// The selected layout in `layouts` while its base is the stored layout: TPS entered through
    /// the input mode suspends it until leaving TPS restores the base.
    static func active(in layouts: [CustomLayout], selectedId: UUID?, layoutType: KeyboardLayoutType) -> CustomLayout? {
        guard let selectedId else { return nil }
        return layouts.first { $0.id == selectedId && $0.base == layoutType }
    }

    /// Whether the keys still sit as in the base table.
    var isUnchanged: Bool {
        keys == baseLabels
    }

    /// The key table with this layout's keys in place.
    func rows(withGlobe: Bool) -> [[KeyDef]] {
        var rows = TaigiLayouts.Alphabetic.table(base, script: script, withGlobe: withGlobe)
        for (row, slots) in Self.slotIndices(of: rows).enumerated() {
            for (column, slot) in slots.enumerated() {
                if let slot {
                    rows[row][column] = Self.catalog[keys[slot]] ?? .char(keys[slot])
                }
            }
        }
        return rows
    }

    /// Keys that can replace one on the board: the tray letters plus the base's own keys, minus
    /// those already on it.
    var spareKeys: [String] {
        let onBoard = Set(keys)
        var seen = Set<String>()
        return (Self.trayLetters + baseLabels).filter { !onBoard.contains($0) && seen.insert($0).inserted }
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

    /// Puts `spare` on slot `index`; the key it replaces returns to the spare tray.
    mutating func replaceKey(at index: Int, with spare: String) {
        guard Self.catalog[spare] != nil, !keys.contains(spare) else { return }
        keys[index] = spare
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
