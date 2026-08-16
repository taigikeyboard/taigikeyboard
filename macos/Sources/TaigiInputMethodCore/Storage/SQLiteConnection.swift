// One open SQLite file, with the prepare / bind / step boilerplate in one place.

import Foundation
import SQLite3

/// A value bound into a statement placeholder.
///
/// Every user-supplied value reaches SQLite through this type: the stores build
/// SQL with `?` placeholders only, never by interpolating a value into the
/// string (`.claude/rules/security-rules.md` § SQL).
enum SQLiteBinding: Sendable {
    case text(String)
    case integer(Int)
}

/// Reads one row of a result set by column index.
struct SQLiteRowReader {
    fileprivate let statement: OpaquePointer

    func text(_ column: Int32) -> String {
        sqlite3_column_text(statement, column).map { String(cString: $0) } ?? ""
    }

    func integer(_ column: Int32) -> Int {
        Int(sqlite3_column_int64(statement, column))
    }

    func integer64(_ column: Int32) -> Int64 {
        sqlite3_column_int64(statement, column)
    }
}

/// One connection to one database file.
///
/// Deliberately NOT thread-safe on its own. The store that owns a connection
/// runs every call to it on its own serial queue, which is what makes the
/// mutable `handle` sound; the `@unchecked Sendable` conformance is that
/// promise, and it is the only reason this type can be reached from inside a
/// queued block.
final class SQLiteConnection: @unchecked Sendable {
    enum Failure: Error, CustomStringConvertible {
        case open(String)
        case statement(String)

        var description: String {
            switch self {
            case let .open(message): "open failed: \(message)"
            case let .statement(message): "statement failed: \(message)"
            }
        }
    }

    /// Tells SQLite to copy the bound bytes rather than borrow them. Swift's
    /// `String` interop hands `sqlite3_bind_text` a buffer that is only valid
    /// for the duration of the call, so borrowing would read freed memory the
    /// moment the statement is stepped.
    private static let transient = unsafeBitCast(-1, to: sqlite3_destructor_type.self)

    private var handle: OpaquePointer?

    deinit {
        if let handle {
            sqlite3_close(handle)
        }
    }

    /// Opens `fileURL`, creating the file if it is not there yet.
    func open(at fileURL: URL) throws {
        var opened: OpaquePointer?
        let status = sqlite3_open_v2(
            fileURL.path,
            &opened,
            SQLITE_OPEN_READWRITE | SQLITE_OPEN_CREATE,
            nil,
        )
        guard status == SQLITE_OK, let opened else {
            let message = opened.map { String(cString: sqlite3_errmsg($0)) } ?? "status \(status)"
            if let opened {
                sqlite3_close(opened)
            }
            throw Failure.open("\(fileURL.lastPathComponent): \(message)")
        }
        handle = opened
    }

    /// Runs SQL that takes no parameters and returns no rows. Accepts several
    /// statements separated by `;`, which is what makes it the right call for
    /// DDL and for transaction control.
    func execute(_ sql: String) throws {
        guard let handle else { throw Failure.statement("connection is closed") }
        var errorMessage: UnsafeMutablePointer<CChar>?
        guard sqlite3_exec(handle, sql, nil, nil, &errorMessage) == SQLITE_OK else {
            let message = errorMessage.map { String(cString: $0) } ?? "unknown error"
            sqlite3_free(errorMessage)
            throw Failure.statement(message)
        }
    }

    /// Runs one parameterized statement that returns no rows.
    func run(_ sql: String, _ bindings: [SQLiteBinding] = []) throws {
        try withStatement(sql, bindings) { statement in
            let status = sqlite3_step(statement)
            guard status == SQLITE_DONE || status == SQLITE_ROW else {
                throw Failure.statement(errorMessage())
            }
        }
    }

    /// Runs one parameterized query and decodes every row it returns.
    func query<Row>(
        _ sql: String,
        _ bindings: [SQLiteBinding] = [],
        decoding decode: (SQLiteRowReader) -> Row,
    ) throws -> [Row] {
        var rows: [Row] = []
        try withStatement(sql, bindings) { statement in
            var status = sqlite3_step(statement)
            while status == SQLITE_ROW {
                rows.append(decode(SQLiteRowReader(statement: statement)))
                status = sqlite3_step(statement)
            }
            // A loop that only tests for `SQLITE_ROW` cannot tell the end of the
            // results from an I/O error or a locked database halfway through
            // them, and would hand back a partial answer as if it were the whole
            // one — which the ranking would then treat as "the user has never
            // used these words".
            guard status == SQLITE_DONE else {
                throw Failure.statement(errorMessage())
            }
        }
        return rows
    }

    /// The first column of the first row, or `nil` when the query returned no
    /// rows. Used for the row-count reads the capacity policy is driven by.
    func scalar(_ sql: String, _ bindings: [SQLiteBinding] = []) throws -> Int? {
        try query(sql, bindings) { $0.integer(0) }.first
    }

    /// `PRAGMA user_version`, which is how each store records the shape it
    /// created. Not parameterizable — SQLite takes the value as a literal — so
    /// the setter is deliberately `Int32` rather than a string.
    var userVersion: Int32 {
        get { ((try? scalar("PRAGMA user_version;")) ?? nil).map(Int32.init) ?? 0 }
        set { try? execute("PRAGMA user_version = \(newValue);") }
    }

    // MARK: - Private

    private func withStatement(
        _ sql: String,
        _ bindings: [SQLiteBinding],
        _ body: (OpaquePointer) throws -> Void,
    ) throws {
        guard let handle else { throw Failure.statement("connection is closed") }
        var statement: OpaquePointer?
        guard sqlite3_prepare_v2(handle, sql, -1, &statement, nil) == SQLITE_OK,
              let statement
        else {
            throw Failure.statement(errorMessage())
        }
        defer { sqlite3_finalize(statement) }

        for (offset, binding) in bindings.enumerated() {
            let index = Int32(offset + 1)
            let status = switch binding {
            case let .text(value): sqlite3_bind_text(statement, index, value, -1, Self.transient)
            case let .integer(value): sqlite3_bind_int64(statement, index, Int64(value))
            }
            guard status == SQLITE_OK else { throw Failure.statement(errorMessage()) }
        }
        try body(statement)
    }

    private func errorMessage() -> String {
        handle.map { String(cString: sqlite3_errmsg($0)) } ?? "connection is closed"
    }
}
