// 自訂詞庫: the words the user added themselves.

import SwiftUI
import UniformTypeIdentifiers

/// Reads and writes the custom dictionary on behalf of the page.
///
/// Rows are fetched bounded (`filter` + `LIMIT`) and replaced on every change
/// rather than held: this window outlives every visit to the page, and a
/// 30000-row dictionary kept in a view model would stay in memory for the life
/// of the input method.
@MainActor
@Observable
final class CustomDictionaryPageModel {
    /// What the list shows before the user filters. iOS shows the same number,
    /// and the point is the same: the list is for finding a word, and the CSV
    /// export is for reading all of them.
    static let displayLimit = 100

    private(set) var rows: [CustomDictionaryRow] = []
    private(set) var totalCount = 0
    private(set) var activity: UserDataPageActivity = .idle
    var filter = ""
    var message: UserDataPageMessage?

    /// Which load the rows on screen came from. A query runs off the main
    /// actor and cannot be cancelled once it is on the store's queue, so a
    /// load started under an older filter can still come back after a newer
    /// one has — and would put rows on screen that do not match what is in the
    /// box. The newest load wins by number, not by arrival.
    private var loadGeneration = 0

    private let store: CustomDictionaryStore

    init(store: CustomDictionaryStore) {
        self.store = store
    }

    func load() async {
        loadGeneration += 1
        let generation = loadGeneration
        do {
            let loaded = try await store.rows(filter: filter, limit: Self.displayLimit)
            guard generation == loadGeneration else { return }
            rows = loaded
            totalCount = try await store.count()
        } catch {
            // Not an empty list: "the dictionary is empty" and "the dictionary
            // could not be read" look identical on screen, and only one of them
            // is worth the user doing something about.
            message = .failure(.macosCustomDictReadFailed, error)
        }
    }

    func save(_ row: CustomDictionaryRow) async {
        await perform(.macosProgressSaving) { try await self.store.upsert(row) }
    }

    func delete(_ row: CustomDictionaryRow) async {
        await perform(.macosProgressDeleting) { _ = try await self.store.delete(id: row.id) }
    }

    func deleteAll() async {
        await perform(.macosProgressDeleting) { _ = try await self.store.deleteAll() }
    }

    func exportCSV(in window: NSWindow) async {
        guard beginWork(.macosProgressExporting) else { return }
        defer { activity = .idle }
        do {
            let csv = try await CustomDictionaryCSV.encode(store.allRows())
            _ = try await UserDataFilePanels.write(
                Data(csv.utf8),
                suggestedName: UserDataFilePanels.exportFileName(
                    prefix: "taigi_custom_dictionary",
                    extension: "csv",
                ),
                contentTypes: [.commaSeparatedText],
                in: window,
            )
        } catch {
            message = .failure(.commonExportFailed, error)
        }
    }

    func importCSV(in window: NSWindow) async {
        // The slot is taken before the file panel, not after it: the panel is
        // modal to the window, but the moment it closes the parse and the
        // batched writes are still running, and that is exactly the window a
        // second import could start in.
        guard beginWork(.macosProgressImporting) else { return }
        defer { activity = .idle }
        guard let url = await UserDataFilePanels.chooseFileToOpen(
            contentTypes: [.commaSeparatedText, .plainText],
            in: window,
        ) else { return }
        do {
            // Off the main actor: reading and parsing up to 5 MB of CSV
            // there would freeze the very window that is showing the progress
            // spinner for it.
            let rows = try await Task.detached {
                try CustomDictionaryCSV.decodeFile(
                    at: url,
                    entryLimit: CustomDictionaryStore.maxEntries,
                )
            }.value
            let result = try await store.batchImport(rows)
            message = .imported(result.imported, skipped: result.skipped)
            await load()
        } catch {
            message = .failure(.commonImportFailed, error)
        }
    }

    /// Takes the page's one work slot for `label`, or answers false because
    /// something else holds it.
    ///
    /// Mutual exclusion lives here rather than in the view's `.disabled`: a
    /// greyed-out control is an appearance, and this page deliberately delays
    /// showing that appearance so a millisecond-long write does not flash it
    /// (`UserDataPageChrome`). A guard that only existed in the view would be
    /// absent for exactly as long as the delay lasts — and the operation that
    /// matters most, a CSV import, spends that window parsing after its file
    /// panel has already closed.
    ///
    /// `@MainActor`, so the check and the claim cannot be interleaved.
    /// Internal so a test can drive the refusal without racing two real
    /// database writes to reproduce it.
    func beginWork(_ label: StringKey) -> Bool {
        guard !activity.isWorking else { return false }
        activity = .working(label)
        return true
    }

    private func perform(_ label: StringKey, _ body: () async throws -> Void) async {
        guard beginWork(label) else { return }
        defer { activity = .idle }
        do {
            try await body()
            await load()
        } catch {
            message = .failure(.macosCustomDictWriteFailed, error)
        }
    }
}

struct CustomDictionaryPage: View {
    @Environment(DisplayLanguageStore.self) private var language

    /// Needed only by the 刪除學習紀錄 row; the entries list reads
    /// `stores.customDictionary` through its own model.
    private let stores: UserDataStores

    @State private var model: CustomDictionaryPageModel
    @State private var editing: CustomDictionaryRow?

    /// The table's selection — the row `−` acts on, and the row a double
    /// click edits.
    @State private var selectedRowID: CustomDictionaryRow.ID?
    @State private var deleteOutcome: DeleteOutcome?
    @AppStorage(SettingsStore.Keys.isCustomDictEnabled.name)
    private var isCustomDictEnabled = SettingsStore.Keys.isCustomDictEnabled.defaultValue

    init(stores: UserDataStores) {
        self.stores = stores
        _model = State(initialValue: CustomDictionaryPageModel(store: stores.customDictionary))
    }

    var body: some View {
        Form {
            Section {
                Toggle(language.string(.dictionaryCustomDictEnabled), isOn: $isCustomDictEnabled)
            }

            Section {
                UserDataFilterField(text: $model.filter)
                entryTable
                entryTableControls
            } header: {
                HStack {
                    Text(language.string(.macosEntriesSection))
                    Spacer()
                    Text(countLabel)
                        .foregroundStyle(.secondary)
                }
            }

            UserDataActionsSection(
                exportTitle: .dictionaryExportCSV,
                importTitle: .dictionaryImportCSV,
                deleteTitle: .dictionaryDeleteAll,
                onExport: { Task { await UserDataFilePanels.withSettingsWindow(model.exportCSV) } },
                onImport: { Task { await UserDataFilePanels.withSettingsWindow(model.importCSV) } },
                onDelete: { Task { await model.deleteAll() } },
            )

            Section {
                WideActionRow(titleKey: .macosClearLearningRecords, role: .destructive) {
                    Task { await deleteLearningRecords() }
                }
            }
        }
        .formStyle(.grouped)
        // The receipt, and only the receipt: the row acts on its click
        // (USER 2026-08-25), and these records have no visible surface of their
        // own, so this alert is the whole of what the user is told.
        .alert(item: $deleteOutcome) { outcome in
            Alert(
                title: Text(language.string(outcome.titleKey)),
                message: outcome.diagnostic.map(Text.init),
                dismissButton: .default(Text(language.string(.commonOk))),
            )
        }
        .reloadWhenFilterSettles(model.filter) { await model.load() }
        .sheet(item: $editing) { row in
            CustomDictionaryEntrySheet(row: row) { edited in
                Task { await model.save(edited) }
            }
        }
        .userDataPageChrome(activity: model.activity, message: $model.message)
    }

    /// The receipt for `deleteLearningRecords`: a title, and on failure the
    /// store's own error text. That text is English and stays that way — it
    /// names a SQLite condition, not something the product has wording for,
    /// the same rule `UserDataPageMessage.failure` follows. Local to this pane
    /// rather than a `UserDataPageMessage` case because the success alert has
    /// no body at all, and there is no long-running work here to veil.
    private struct DeleteOutcome: Identifiable {
        let titleKey: StringKey
        /// `nil` on success — the alert then shows a title and nothing else.
        let diagnostic: String?

        var id: String { "\(titleKey.rawValue)|\(diagnostic ?? "")" }

        static let deleted = DeleteOutcome(
            titleKey: .macosClearLearningRecordsDone, diagnostic: nil,
        )

        static func failed(_ diagnostic: String) -> DeleteOutcome {
            DeleteOutcome(titleKey: .macosClearLearningRecordsFailed, diagnostic: diagnostic)
        }
    }

    /// Deletes both learning tables.
    ///
    /// Two calls rather than one transaction: they are separate database files,
    /// so there is no transaction that could span them. The second is attempted
    /// even when the first fails — a store that cannot be reached is no reason
    /// to leave the other one full — and the alert reports the failure rather
    /// than claiming the records are gone.
    ///
    /// The diagnostic names its table, because a bare SQLite string cannot say
    /// which of the two could not be emptied.
    private func deleteLearningRecords() async {
        var failures: [String] = []
        do {
            _ = try await stores.frequency.deleteAll()
        } catch {
            failures.append("user_frequency: \(error)")
        }
        do {
            _ = try await stores.association.deleteAll()
        } catch {
            failures.append("user_association: \(error)")
        }
        deleteOutcome = failures.isEmpty
            ? .deleted
            : .failed(failures.joined(separator: "\n"))
    }

    /// The entries, as the table macOS states a list of records with: click
    /// selects, double-click edits, and the selection is what the `−` button
    /// acts on. A `Table` rather than form rows drawn to look like one — the
    /// highlight, its dimming when the window resigns key, arrow-key
    /// traversal and the "row N of M" an assistive reader announces all come
    /// with the control and cannot be restated from outside it.
    ///
    /// A definite height, not a floor: a `Table` has no intrinsic content
    /// height, and one left free to grow inside the form's own scroll view has
    /// no bound at all.
    private var entryTable: some View {
        Table(model.rows, selection: $selectedRowID) {
            TableColumn(language.string(.dictionaryRomanLabel)) { row in
                Text(row.roman)
                    .foregroundStyle(.secondary)
            }
            TableColumn(language.string(.dictionaryHanziLabel)) { row in
                Text(row.hanzi)
            }
        }
        .tableStyle(.inset)
        // Every row the same colour (USER 2026-08-24). The striping is what
        // AppKit gives a data table by default; this list is short and reads
        // as settings content, not as a spreadsheet. Selection is unaffected —
        // this governs only the unselected rows' backgrounds.
        .alternatingRowBackgrounds(.disabled)
        .frame(height: Metrics.tableHeight)
        // The empty case as an overlay rather than in place of the table: the
        // filter box above stays reachable, and the columns stay put while a
        // filter is narrowed to nothing and widened again.
        .overlay {
            if model.rows.isEmpty {
                emptyState
            }
        }
        // `primaryAction` IS the double click. The menu keeps a one-click
        // route to both verbs for anyone who never discovers it.
        .contextMenu(forSelectionType: CustomDictionaryRow.ID.self) { ids in
            if let row = row(for: ids.first) {
                Button(language.string(.dictionaryEditEntry)) { editing = row }
                Button(language.string(.commonDelete), role: .destructive) {
                    Task { await model.delete(row) }
                }
            }
        } primaryAction: { ids in
            editing = row(for: ids.first)
        }
    }

    /// What an empty table shows.
    ///
    /// A symbol rather than a sentence for "nothing added yet" (USER
    /// 2026-08-24): an empty dictionary needs no explaining, and the wording
    /// was a string in five languages saying what the blank table already
    /// says. A filtered search that matches nothing DOES get words — that one
    /// is a result, not a state, and the user needs to know their filter is
    /// what emptied the list.
    ///
    /// The symbol carries the shared empty-state sentence as its accessibility
    /// label rather than showing it: a reader is still told what the blank
    /// table means, and no string had to be authored to keep that true — the
    /// key was already translated for iOS and Android, and its wording ("add a
    /// custom word with +") holds here now that this pane has a + of its own.
    @ViewBuilder
    private var emptyState: some View {
        if model.filter.isEmpty {
            Image(systemName: Self.emptyStateSymbolName)
                .font(.system(size: Metrics.emptyStateSymbolSize))
                .foregroundStyle(.tertiary)
                .accessibilityLabel(language.string(.dictionaryCustomDictEmpty))
        } else {
            Text(language.string(.dictionaryNoResults))
                .foregroundStyle(.secondary)
        }
    }

    /// The `+` / `−` pair under the table, where macOS puts the add and remove
    /// verbs for an editable list. `−` is disabled with nothing selected
    /// rather than hidden, so the pair keeps its shape.
    private var entryTableControls: some View {
        HStack(spacing: 4) {
            Button {
                editing = CustomDictionaryRow(roman: "", hanzi: "")
            } label: {
                controlGlyph("plus")
            }
            .accessibilityLabel(language.string(.dictionaryAddEntry))

            Button {
                guard let selectedRow else { return }
                Task { await model.delete(selectedRow) }
            } label: {
                controlGlyph("minus")
            }
            .disabled(selectedRow == nil)
            .accessibilityLabel(language.string(.commonDelete))

            Spacer()
        }
        // Small bordered buttons, the size AppKit gives the +/- bar under a
        // table. `.borderless` around a bare glyph left a hit target the size
        // of the symbol itself.
        .buttonStyle(.bordered)
        .controlSize(.small)
    }

    /// One button's glyph, sized so the button is as big as the control it
    /// imitates. The frame is what does that — `contentShape` only squares off
    /// the hit region inside whatever bounds the label already has, it cannot
    /// grow them.
    private func controlGlyph(_ symbolName: String) -> some View {
        Image(systemName: symbolName)
            .frame(width: 20, height: 14)
            .contentShape(Rectangle())
    }

    /// The selected row, or nil when the selection names a row the list no
    /// longer holds — filtered away, deleted, or reloaded out from under it.
    /// Nothing clears the id when that happens, and nothing has to: the ids
    /// are UUIDs, so a stale one can never match a different entry.
    private var selectedRow: CustomDictionaryRow? {
        row(for: selectedRowID)
    }

    private func row(for id: CustomDictionaryRow.ID?) -> CustomDictionaryRow? {
        model.rows.first { $0.id == id }
    }

    /// An empty tray, not the pane's own book: the book says "dictionary",
    /// which is the pane the user is already looking at, where what this
    /// draws has to say "and there is nothing in it" (USER 2026-08-24).
    static let emptyStateSymbolName = "tray"

    private enum Metrics {
        /// Tall enough to read as a list rather than a row or two, short
        /// enough that the buttons and the CSV actions under it stay on
        /// screen at the window's floor height.
        static let tableHeight: CGFloat = 220
        /// Large enough to read as a state rather than as a control the user
        /// is meant to press.
        static let emptyStateSymbolSize: CGFloat = 34
    }

    private var countLabel: String {
        model.totalCount > model.rows.count
            ? "\(model.rows.count) / \(model.totalCount)"
            : "\(model.totalCount)"
    }
}

/// Add or edit one entry.
///
/// A sheet rather than another pushed page: it is a two-field form the user
/// finishes and dismisses, and pushing it would put a back button where a
/// Cancel belongs.
struct CustomDictionaryEntrySheet: View {
    @Environment(\.dismiss) private var dismiss
    @Environment(DisplayLanguageStore.self) private var language

    @State private var roman: String
    @State private var hanzi: String
    private let original: CustomDictionaryRow
    private let onSave: (CustomDictionaryRow) -> Void

    init(row: CustomDictionaryRow, onSave: @escaping (CustomDictionaryRow) -> Void) {
        original = row
        self.onSave = onSave
        _roman = State(initialValue: row.roman)
        _hanzi = State(initialValue: row.hanzi)
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            Text(language.string(original.roman.isEmpty ? .dictionaryAddEntry : .dictionaryEditEntry))
                .font(.headline)
            Form {
                TextField(language.string(.dictionaryRomanLabel), text: $roman)
                TextField(language.string(.dictionaryHanziLabel), text: $hanzi)
            }
            .formStyle(.grouped)
            HStack {
                Spacer()
                Button(language.string(.commonCancel), role: .cancel) { dismiss() }
                Button(language.string(.dictionarySave)) {
                    var edited = original
                    edited.roman = roman.trimmingCharacters(in: .whitespacesAndNewlines)
                    edited.hanzi = hanzi.trimmingCharacters(in: .whitespacesAndNewlines)
                    edited.updatedAt = Date()
                    onSave(edited)
                    dismiss()
                }
                .keyboardShortcut(.defaultAction)
                // A romanization is what the entry is found by; without one
                // there is nothing to store it under.
                .disabled(roman.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
            }
        }
        .padding(20)
        .frame(width: 360)
    }
}
