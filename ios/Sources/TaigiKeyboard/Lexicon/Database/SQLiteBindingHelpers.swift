import Foundation
import SQLite3

/// Shared helpers for `sqlite3_*` binding boilerplate used by SQLite
/// repositories in this module. Keeps `-1, TRANSIENT` out of every call site.
extension OpaquePointer? {
    func bindText(_ index: Int32, _ value: String) {
        sqlite3_bind_text(self, index, value, -1, SQLiteConnectionManager.sqliteTransient)
    }
}
