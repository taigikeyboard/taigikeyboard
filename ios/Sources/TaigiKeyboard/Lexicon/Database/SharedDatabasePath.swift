// 解析 App Group 共享容器內 SQLite 檔案路徑,讓 keyboard extension 與主 app 共用同一份 DB。

import Foundation

/// Resolves on-disk paths for SQLite databases stored in the App Group container.
///
/// All user-data SQLite repositories (custom dictionary, user frequency,
/// next-word association) share the same container so the keyboard extension
/// can read what the host app writes. Centralized here so the directory-create
/// + path-append boilerplate is in one place.
// App Group 容器路徑解析器 — 集中所有 user-data SQLite 路徑邏輯。
enum SharedDatabasePath {
    /// Returns the absolute path to `<appGroupContainer>/<filename>`,
    /// creating the container directory if needed.
    // 回傳 App Group 容器內指定檔名的絕對路徑;若目錄不存在會自動建立。
    static func resolve(filename: String) throws -> String {
        guard let containerURL = SharedSettings.sharedContainerURL else {
            throw LexiconError.databaseNotFound
        }
        try FileManager.default.createDirectory(
            at: containerURL,
            withIntermediateDirectories: true,
        )
        return containerURL.appendingPathComponent(filename).path
    }
}
