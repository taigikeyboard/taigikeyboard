// Custom Dictionary: the words the user added themselves.

import SwiftUI
import UniformTypeIdentifiers

/// Reads and writes the custom dictionary on behalf of the page, one page of
/// rows at a time (`UserDataPagedList`).
@MainActor
@Observable
final class CustomDictionaryPageModel {
    private(set) var list = UserDataPagedList<CustomDictionaryRow>()
    private(set) var activity: UserDataPageActivity = .idle
    /// Once changed, makes every load in flight stale at once
    /// (`UserDataPagedList.invalidate`), not when the settled filter's own
    /// load starts.
    var filter = "" {
        didSet {
            if oldValue != filter {
                list.invalidate()
            }
        }
    }

    var message: UserDataPageMessage?

    /// The table's selection, held by the list so a load drops it once its
    /// row is off screen (`UserDataPagedList.selectedID`).
    var selectedRowID: CustomDictionaryRow.ID? {
        get { list.selectedID }
        set { list.selectedID = newValue }
    }

    /// Delete All is waiting on its confirmation. The dialog's Cancel, Escape
    /// and dismissal all just set this back to false.
    ///
    /// Confirmed rather than run on the click (USER 2026-10-02), as on Windows
    /// and Linux (desktop-core `Confirmation`): the row that runs it is one
    /// among the pane's, so the click is easy to make by accident, and there
    /// is no undo — `−` acts on one row, this empties the table.
    var isConfirmingDeleteAll = false

    func pageBackward() async {
        guard list.step(by: -1) else { return }
        await load()
    }

    func pageForward() async {
        guard list.step(by: 1) else { return }
        await load()
    }

    private let client: any UserDataClient

    init(client: any UserDataClient) {
        self.client = client
    }

    /// Reloads the page on screen.
    func load() async {
        let load = list.beginLoad()
        let (filter, offset) = (filter, load.offset)
        do {
            let listing = try await UserDataRequests.run(on: client) {
                try $0.list(filter: filter, limit: UserDataListMetrics.pageSize, offset: offset)
            }
            list.land(listing, from: load)
        } catch {
            guard list.isCurrent(load) else { return }
            // Not an empty list: "the dictionary is empty" and "the dictionary
            // could not be read" look identical on screen, and only one of them
            // is worth the user doing something about.
            message = .failure(.desktopCustomDictReadFailed, error)
        }
    }

    /// Back to page one, then load. What a filter change asks for.
    func loadFirstPage() async {
        list.rewind()
        await load()
    }

    func save(_ row: CustomDictionaryRow) async {
        await perform(.desktopProgressWorking) { try await UserDataRequests.run(on: self.client) { try $0.save(row) } }
    }

    func delete(_ row: CustomDictionaryRow) async {
        let id = row.id
        await perform(.desktopProgressWorking) { try await UserDataRequests.run(on: self.client) { try $0.delete(id: id) } }
    }

    func deleteAll() async {
        await perform(.desktopProgressWorking) { try await UserDataRequests.run(on: self.client) { try $0.deleteAll() } }
    }

    /// Asks before Delete All runs. Nothing to ask while the page is busy —
    /// it could not start anyway.
    func askDeleteAll() {
        guard !activity.isWorking else { return }
        isConfirmingDeleteAll = true
    }

    /// Runs Delete All once confirmed, at most once.
    ///
    /// The flag is taken synchronously, before the dialog's dismissal writes
    /// false through its binding — a `Task` that read it only once it started
    /// would find it already gone. The returned task is what a test awaits.
    @discardableResult
    func confirmDeleteAll() -> Task<Void, Never>? {
        guard isConfirmingDeleteAll else { return nil }
        isConfirmingDeleteAll = false
        return Task { await deleteAll() }
    }

    func exportCSV(in window: NSWindow) async {
        guard beginWork(.desktopProgressWorking) else { return }
        defer { activity = .idle }
        do {
            let csv = try await UserDataRequests.run(on: client) { try $0.exportCSV() }
            _ = try await UserDataFilePanels.write(
                csv,
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
        guard beginWork(.desktopProgressWorking) else { return }
        defer { activity = .idle }
        guard let url = await UserDataFilePanels.chooseFileToOpen(
            contentTypes: [.commaSeparatedText, .plainText],
            in: window,
        ) else { return }
        await importCSV(at: url)
    }

    /// Imports `url` and reloads, whatever the import answered: the engine
    /// commits a large file in chunks (`IMPORT_CHUNK_SIZE`), so one that
    /// fails partway has still added rows. The caller holds the work slot;
    /// internal so a test can drive it without a file panel.
    func importCSV(at url: URL) async {
        do {
            // Off the main actor: reading and importing up to 5 MB of CSV
            // there would freeze the very window that is showing the progress
            // spinner for it.
            let result = try await UserDataRequests.run(on: client) { try $0.importCSV(at: url) }
            message = .imported(result.imported, skipped: result.skipped)
        } catch {
            message = .failure(.commonImportFailed, error)
        }
        await load()
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
        activity.begin(label)
    }

    /// Runs one write and reloads, whatever it answered, as desktop-core's
    /// `write_outcome` does: the list on screen is what the store holds even
    /// after a failure. A reload that fails too replaces the write's alert
    /// with its own, as on Windows.
    private func perform(_ label: StringKey, _ body: () async throws -> Void) async {
        guard beginWork(label) else { return }
        defer { activity = .idle }
        do {
            try await body()
        } catch {
            message = .failure(.desktopCustomDictWriteFailed, error)
        }
        await load()
    }
}

struct CustomDictionaryPage: View {
    @Environment(DisplayLanguageStore.self) private var language

    @State private var model: CustomDictionaryPageModel
    @State private var editing: CustomDictionaryRow?
    @AppStorage(SettingsStore.Keys.isCustomDictEnabled.name)
    private var isCustomDictEnabled = SettingsStore.Keys.isCustomDictEnabled.defaultValue

    init(client: any UserDataClient) {
        _model = State(initialValue: CustomDictionaryPageModel(client: client))
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
                    Text(language.string(.desktopEntriesSection))
                    Spacer()
                    Text(model.list.countLabel)
                        .foregroundStyle(.secondary)
                }
            }

            UserDataActionsSection(
                exportTitle: .dictionaryExportCSV,
                importTitle: .dictionaryImportCSV,
                deleteTitle: .dictionaryDeleteAll,
                onExport: { Task { await UserDataFilePanels.withSettingsWindow(model.exportCSV) } },
                onImport: { Task { await UserDataFilePanels.withSettingsWindow(model.importCSV) } },
                onDelete: { model.askDeleteAll() },
            )
        }
        .formStyle(.grouped)
        // A confirmation dialog, not a second `.alert`: the chrome's alert
        // is the page's one alert, and two on one chain do not stack
        // (`UserDataPageChrome`). Delete is the destructive button; Cancel,
        // Escape and a dismissal all leave the store alone.
        .confirmationDialog(
            language.string(.dictionaryDeleteAll),
            isPresented: $model.isConfirmingDeleteAll,
            titleVisibility: .visible,
        ) {
            Button(language.string(.commonDelete), role: .destructive) { model.confirmDeleteAll() }
            Button(language.string(.commonCancel), role: .cancel) {}
        } message: {
            Text(language.string(.dictionaryDeleteAllMessage))
        }
        .reloadWhenFilterSettles(model.filter) { await model.loadFirstPage() }
        .sheet(item: $editing) { row in
            CustomDictionaryEntrySheet(row: row) { edited in
                Task { await model.save(edited) }
            }
        }
        .userDataPageChrome(activity: model.activity, message: $model.message)
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
        Table(model.list.rows, selection: $model.selectedRowID) {
            TableColumn(language.string(.dictionaryRomanLabel)) { row in
                Text(row.roman)
                    .foregroundStyle(.secondary)
            }
            TableColumn(language.string(.dictionaryHanziLabel)) { row in
                Text(row.hanji)
            }
        }
        .tableStyle(.inset)
        // Every row the same colour (USER 2026-08-24). The striping is what
        // AppKit gives a data table by default; this list is short and reads
        // as settings content, not as a spreadsheet. Selection is unaffected —
        // this governs only the unselected rows' backgrounds.
        .alternatingRowBackgrounds(.disabled)
        .frame(height: UserDataListMetrics.pagedTableHeight)
        // The empty case as an overlay rather than in place of the table: the
        // filter box above stays reachable, and the columns stay put while a
        // filter is narrowed to nothing and widened again.
        .overlay {
            if model.list.rows.isEmpty {
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
            UserDataListEmptySymbol(
                symbolName: UserDataListMetrics.emptyStateSymbolName,
                accessibilityLabelKey: .dictionaryCustomDictEmpty,
            )
        } else {
            Text(language.string(.dictionaryNoResults))
                .foregroundStyle(.secondary)
        }
    }

    /// The `+` / `−` pair under the table, with the pager at its trailing end.
    private var entryTableControls: some View {
        UserDataListControls(
            add: (.dictionaryAddEntry, { editing = CustomDictionaryRow(roman: "", hanji: "") }),
            isRemoveEnabled: model.list.selectedRow != nil,
            onRemove: {
                guard let selectedRow = model.list.selectedRow else { return }
                Task { await model.delete(selectedRow) }
            },
        ) {
            UserDataListPager(
                page: model.list.page,
                pageCount: model.list.pageCount,
                onBackward: { Task { await model.pageBackward() } },
                onForward: { Task { await model.pageForward() } },
            )
        }
    }

    private func row(for id: CustomDictionaryRow.ID?) -> CustomDictionaryRow? {
        model.list.rows.first { $0.id == id }
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
    @State private var hanji: String
    private let original: CustomDictionaryRow
    private let onSave: (CustomDictionaryRow) -> Void

    init(row: CustomDictionaryRow, onSave: @escaping (CustomDictionaryRow) -> Void) {
        original = row
        self.onSave = onSave
        _roman = State(initialValue: row.roman)
        _hanji = State(initialValue: row.hanji)
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            Text(language.string(original.roman.isEmpty ? .dictionaryAddEntry : .dictionaryEditEntry))
                .font(.headline)
            Form {
                TextField(language.string(.dictionaryRomanLabel), text: $roman)
                TextField(language.string(.dictionaryHanziLabel), text: $hanji)
            }
            .formStyle(.grouped)
            HStack {
                Spacer()
                Button(language.string(.commonCancel), role: .cancel) { dismiss() }
                Button(language.string(.commonSave)) {
                    var edited = original
                    edited.roman = roman.trimmingCharacters(in: .whitespacesAndNewlines)
                    edited.hanji = hanji.trimmingCharacters(in: .whitespacesAndNewlines)
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
