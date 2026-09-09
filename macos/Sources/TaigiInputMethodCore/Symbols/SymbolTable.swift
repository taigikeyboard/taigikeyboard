// The symbol picker's table: three categories of insertable strings, decoded from the shared desktop JSON.

import Foundation

/// How the file groups its symbols. The picker shows them as ONE list, in
/// file order (USER 2026-09-09: a category to pick first 「會造成使用者的體驗
/// 中斷」), so the grouping is the file's own documentation and the
/// validation's unit — a closed roster, so the JSON cannot invent one.
enum SymbolCategoryID: String, Decodable, CaseIterable, Sendable {
    case punctuation
    case brackets
    case specialSymbols
}

/// One group and, in menu order, the exact string each of its cells inserts
/// — a bracket pair is one entry (`「」`), which is what lets one pick write
/// both halves (USER 2026-09-09).
struct SymbolCategory: Decodable, Equatable, Sendable {
    let id: SymbolCategoryID
    let symbols: [String]
}

/// The whole table, from `symbols/desktop-symbols.json` — the one source both
/// desktop platforms read, so the two pickers cannot drift (the file's own
/// comment says how each platform reaches it).
struct SymbolTable: Decodable, Equatable, Sendable {
    let categories: [SymbolCategory]

    /// The file's name inside the app bundle's `Resources`, where
    /// `bundle-app.sh` copies it from the repository root.
    static let fileName = "desktop-symbols.json"

    /// Why a table was refused, naming the offending value.
    enum ValidationError: Error, Equatable {
        case noCategories
        case duplicateCategory(SymbolCategoryID)
        case emptyCategory(SymbolCategoryID)
        case duplicateSymbol(String, in: SymbolCategoryID)
        case emptySymbol(in: SymbolCategoryID)
    }

    /// Validates on construction, so no table can exist that the picker
    /// cannot show: a duplicate would draw two cells for one pick, an empty
    /// symbol would consume a key and write nothing. Decoding goes through
    /// here too, so a broken JSON is refused when it is read — at launch of
    /// the shipped app, or in `SymbolTableTests` for the repository's copy.
    init(categories: [SymbolCategory]) throws {
        guard !categories.isEmpty else { throw ValidationError.noCategories }
        var seenIDs: Set<SymbolCategoryID> = []
        for category in categories {
            guard seenIDs.insert(category.id).inserted else {
                throw ValidationError.duplicateCategory(category.id)
            }
            guard !category.symbols.isEmpty else { throw ValidationError.emptyCategory(category.id) }
            var seenSymbols: Set<String> = []
            for symbol in category.symbols {
                guard !symbol.isEmpty else { throw ValidationError.emptySymbol(in: category.id) }
                guard seenSymbols.insert(symbol).inserted else {
                    throw ValidationError.duplicateSymbol(symbol, in: category.id)
                }
            }
        }
        self.categories = categories
    }

    init(from decoder: any Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        try self.init(categories: container.decode([SymbolCategory].self, forKey: .categories))
    }

    /// Only `categories` is read; the JSON's `comment` is for the reader of
    /// the file.
    private enum CodingKeys: String, CodingKey {
        case categories
    }

    static func load(from url: URL) throws -> SymbolTable {
        try JSONDecoder().decode(SymbolTable.self, from: Data(contentsOf: url))
    }

    /// The table the shipped bundle carries, or nil when it carries none —
    /// logged once, and the picker chord then does nothing rather than open
    /// an empty window. Missing in the tests, which run outside any bundle
    /// and inject the repository's copy instead.
    static let bundled: SymbolTable? = {
        guard let url = Bundle.main.resourceURL?.appendingPathComponent(fileName) else {
            logger.error("no resource directory to read \(fileName) from")
            return nil
        }
        do {
            return try load(from: url)
        } catch {
            logger.error("could not load \(fileName): \(String(describing: error))")
            return nil
        }
    }()

    private static let logger = DebugLogger(category: "SymbolTable")

    /// The category `id` names, or nil when the table has none.
    func category(_ id: SymbolCategoryID) -> SymbolCategory? {
        categories.first { $0.id == id }
    }

    /// Every symbol in menu order — the one list the picker shows.
    var symbols: [String] {
        categories.flatMap(\.symbols)
    }
}
