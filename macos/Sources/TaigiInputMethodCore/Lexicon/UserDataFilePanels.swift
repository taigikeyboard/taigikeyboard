// Asking the user for a file to read, or a place to write one.

import AppKit
import UniformTypeIdentifiers

/// The open and save panels the Dictionary pages put in front of the settings
/// window.
///
/// Sheets on a named window, never free-standing panels: a free-standing panel
/// from an accessory process is a window the user has to find, and this one
/// belongs to the settings window they are looking at. AppKit panels rather
/// than SwiftUI's `fileImporter` for the same reason — the parent window here
/// is hand-built (`SettingsWindowController`), so naming it is the reliable
/// way to attach the sheet to it.
///
/// A page model reaches them through `UserDataFilePresenting`, so a test can
/// answer in their place: a panel is modal UI no test can click.
@MainActor
enum UserDataFilePanels {
    /// Runs `body` with the window the panels hang their sheets on.
    ///
    /// A page that somehow has no window simply does not open one, and does so
    /// before any work is announced — a veil raised over a panel that never
    /// appeared would never come down.
    static func withSettingsWindow(_ body: (NSWindow) async -> Void) async {
        guard let window = SettingsWindowController.shared.windowForSheets else { return }
        await body(window)
    }

    /// Asks for a file to read, and hands back what the user chose.
    static func chooseFileToOpen(
        contentTypes: [UTType],
        in window: NSWindow,
    ) async -> URL? {
        let panel = NSOpenPanel()
        panel.allowedContentTypes = contentTypes
        panel.allowsMultipleSelection = false
        panel.canChooseDirectories = false
        return await withCheckedContinuation { continuation in
            panel.beginSheetModal(for: window) { response in
                continuation.resume(returning: response == .OK ? panel.url : nil)
            }
        }
    }

    /// Asks where to write `data`, and writes it there.
    ///
    /// Returns the URL written, or `nil` when the user cancelled. Throws when
    /// the write itself failed — a save panel that dismissed is not a file
    /// that landed, and the page has to be able to say so.
    @discardableResult
    static func write(
        _ data: Data,
        suggestedName: String,
        contentTypes: [UTType],
        in window: NSWindow,
    ) async throws -> URL? {
        let panel = NSSavePanel()
        panel.allowedContentTypes = contentTypes
        panel.nameFieldStringValue = suggestedName
        panel.isExtensionHidden = false
        let url: URL? = await withCheckedContinuation { continuation in
            panel.beginSheetModal(for: window) { response in
                continuation.resume(returning: response == .OK ? panel.url : nil)
            }
        }
        guard let url else { return nil }
        // Atomic: a half-written export that overwrote the user's previous one
        // would lose both.
        try data.write(to: url, options: .atomic)
        return url
    }

    /// `taigi_frequency_2026-08-17.csv` and friends. POSIX-fixed so the date
    /// in the name is the same shape whatever calendar the user runs.
    static func exportFileName(prefix: String, extension fileExtension: String) -> String {
        "\(prefix)_\(dateFormatter.string(from: Date())).\(fileExtension)"
    }

    /// The `.taigi` backup, as `Info.plist` exports it (the same identifier
    /// iOS exports).
    static let backupContentType = UTType(exportedAs: "tw.taigikeyboard.backup", conformingTo: .json)

    /// The panels themselves, for a page model that takes `UserDataFilePresenting`.
    struct System: UserDataFilePresenting {
        func chooseFileToOpen(contentTypes: [UTType], in window: NSWindow) async -> URL? {
            await UserDataFilePanels.chooseFileToOpen(contentTypes: contentTypes, in: window)
        }

        func write(_ data: Data, suggestedName: String, contentTypes: [UTType], in window: NSWindow) async throws -> URL? {
            try await UserDataFilePanels.write(data, suggestedName: suggestedName, contentTypes: contentTypes, in: window)
        }
    }

    private static let dateFormatter: DateFormatter = {
        let formatter = DateFormatter()
        formatter.dateFormat = "yyyy-MM-dd"
        formatter.locale = Locale(identifier: "en_US_POSIX")
        return formatter
    }()
}

/// What a page model asks of the file panels (`UserDataFilePanels.System`).
@MainActor
protocol UserDataFilePresenting {
    /// The file the user chose to read, or `nil` when they cancelled.
    func chooseFileToOpen(contentTypes: [UTType], in window: NSWindow) async -> URL?
    /// Writes `data` where the user chose; `nil` when they cancelled, a throw
    /// when the write failed.
    @discardableResult
    func write(_ data: Data, suggestedName: String, contentTypes: [UTType], in window: NSWindow) async throws -> URL?
}
