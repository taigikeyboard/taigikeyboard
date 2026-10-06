// Learning Records: what the input method learned from the user's picks.
//
// The Custom Dictionary page's shape (`CustomDictionaryPage.swift`) over the
// engine's learning stores: a kind and an order picker over a filter, a paged
// table, a count sheet and `−`, then Delete Learning Records, which empties
// every learning store whichever kind is on screen. Nothing is added here by
// hand — a word the user wants is a custom word, so a listed row's word can be
// filed there (Add to Custom Dictionary).
// Design: `docs/architecture/learning-records-page-roadmap.md`.

import SwiftUI

/// Reads and writes the learning stores on behalf of the page, one page of
/// rows at a time (`UserDataPagedList`).
@MainActor
@Observable
final class LearningRecordsPageModel {
    /// The largest count the sheet offers. CROSS-PLATFORM INVARIANT — the
    /// engine clamps a set count to the same (`SetLearningRecordCount`).
    nonisolated static let maxCount = 1_000_000

    private(set) var list = UserDataPagedList<Taigi_Engine_LearningRecord>()
    private(set) var activity: UserDataPageActivity = .idle
    /// Word frequency or learned phrases. No next-word association: the
    /// desktop predicts no next word, so those rows rank nothing here.
    /// Each of the three, once changed, makes every load in flight stale
    /// at once (`UserDataPagedList.invalidate`) — not when the next load
    /// starts, which for the filter is after it settles. A new kind or
    /// order also drops the selection, as desktop-core's `Listing::rewind`
    /// does: row ids are per store, so the same id in the other store is a
    /// different word.
    var kind: Taigi_Engine_LearningRecordKind = .frequency {
        didSet {
            if oldValue != kind {
                list.invalidate()
                list.selectedID = nil
            }
        }
    }

    var order: Taigi_Engine_LearningRecordOrder = .mostUsed {
        didSet {
            if oldValue != order {
                list.invalidate()
                list.selectedID = nil
            }
        }
    }

    var filter = "" {
        didSet {
            if oldValue != filter {
                list.invalidate()
            }
        }
    }

    var message: UserDataPageMessage?

    /// Delete Learning Records is waiting on its confirmation; the dialog's
    /// Cancel, Escape and dismissal set this back to false. Asked first for
    /// Custom Dictionary's Delete All reason (`isConfirmingDeleteAll`).
    var isConfirmingClearAll = false

    /// The table's selection, held by the list so a load drops it once its
    /// row is off screen (`UserDataPagedList.selectedID`).
    var selectedRowID: Taigi_Engine_LearningRecord.ID? {
        get { list.selectedID }
        set { list.selectedID = newValue }
    }

    private let client: any UserDataClient

    init(client: any UserDataClient) {
        self.client = client
    }

    func pageBackward() async {
        guard list.step(by: -1) else { return }
        await load()
    }

    func pageForward() async {
        guard list.step(by: 1) else { return }
        await load()
    }

    /// Reloads the page on screen, of the kind and in the order picked now.
    func load() async {
        let load = list.beginLoad()
        let (kind, order, filter, offset) = (kind, order, filter, load.offset)
        do {
            let listing = try await UserDataRequests.run(on: client) {
                try $0.listLearningRecords(
                    kind: kind,
                    order: order,
                    filter: filter,
                    limit: UserDataListMetrics.pageSize,
                    offset: offset,
                )
            }
            list.land(listing, from: load)
        } catch {
            guard list.isCurrent(load) else { return }
            // "Nothing learned yet" and "could not be read" look identical
            // as an empty table; only one of them is worth an alert.
            message = .failure(.dictionaryLearningRecordsReadFailed, error)
        }
    }

    /// Back to page one, then load. What a new filter, kind or order asks
    /// for; the load it starts makes any older one stale.
    func loadFirstPage() async {
        list.rewind()
        await load()
    }

    func setCount(of record: Taigi_Engine_LearningRecord, to count: Int) async {
        await perform { try $0.setLearningRecordCount(record, count: count) }
    }

    /// Forgets `record`; the keyboard learns it again on the next pick.
    /// No confirmation, as deleting one custom word has none.
    func delete(_ record: Taigi_Engine_LearningRecord) async {
        await perform { try $0.deleteLearningRecord(record) }
    }

    /// Files the row's word in the custom dictionary; the engine then forgets
    /// a learned phrase and keeps a frequency row. The receipt is always said
    /// — added or already stored — as desktop-core's job does.
    func addToCustomDictionary(_ record: Taigi_Engine_LearningRecord) async {
        await perform(receipt: .done(.dictionaryLearningRecordsAddedToCustomDictionary)) {
            try $0.addLearningRecordToCustomDictionary(record)
            return true
        }
    }

    /// Asks before Delete Learning Records runs. Nothing to ask while the
    /// page is busy — it could not start anyway.
    func askClearAll() {
        guard !activity.isWorking else { return }
        isConfirmingClearAll = true
    }

    /// Runs Delete Learning Records once confirmed, at most once. The flag is
    /// taken synchronously, before the dialog's dismissal writes false through
    /// its binding (`CustomDictionaryPageModel.confirmDeleteAll`). The
    /// returned task is what a test awaits.
    @discardableResult
    func confirmClearAll() -> Task<Void, Never>? {
        guard isConfirmingClearAll else { return nil }
        isConfirmingClearAll = false
        return Task { await clearAll() }
    }

    /// Empties every learning store — word frequency, learned phrases, and the
    /// next-word association no page lists — in one request: the engine
    /// attempts each even when an earlier one fails, and the alert reports the
    /// failure rather than claiming the records are gone. The list reloads
    /// either way, as desktop-core's `clear_all_job` asks.
    func clearAll() async {
        await perform(
            receipt: .done(.dictionaryClearLearningRecordsDone),
            failure: .dictionaryClearLearningRecordsFailed,
        ) {
            try $0.clearLearningRecords()
            return true
        }
    }

    /// Claims the page's one work slot (`CustomDictionaryPageModel.beginWork`).
    /// Internal so a test can hold the slot and see a write refused.
    func beginWork() -> Bool {
        activity.begin(.desktopProgressWorking)
    }

    /// Runs one write and reloads, whatever it answered: the row moved, went,
    /// or was never there. A row already gone — deleted elsewhere, evicted,
    /// its id taken by another word — is said, not reported as a failure; a
    /// write that landed says `receipt`, when it has one, and one that failed
    /// is titled `failure`.
    /// One write at a time, as on Custom Dictionary
    /// (`CustomDictionaryPageModel.beginWork`).
    private func perform(
        receipt: UserDataPageMessage? = nil,
        failure: StringKey = .dictionaryLearningRecordsWriteFailed,
        _ write: @escaping @Sendable (any UserDataClient) throws -> Bool,
    ) async {
        guard beginWork() else { return }
        defer { activity = .idle }
        do {
            if try await UserDataRequests.run(on: client, write) == false {
                message = .done(.dictionaryLearningRecordGone)
            } else if let receipt {
                message = receipt
            }
        } catch {
            message = .failure(failure, error)
        }
        await load()
    }

    /// The note under the count field. Word frequency's ranking boost stops
    /// growing at count 40 (`engine/ranking/src/score.rs` `MAX_BOOST`); the
    /// other kinds make no such promise, so they say nothing.
    nonisolated static func countNoteKey(for kind: Taigi_Engine_LearningRecordKind) -> StringKey? {
        kind == .frequency ? .dictionaryLearningRecordsCountCapInfo : nil
    }

    /// The day a row was last used, `yyyy-MM-dd` in `timeZone` — the same
    /// shape in every display language, as the Windows and Linux pages draw
    /// it. Empty when the store held no readable time (`last_used_ms` 0).
    nonisolated static func lastUsedLabel(_ lastUsedMs: Int64, timeZone: TimeZone = .current) -> String {
        guard lastUsedMs > 0 else { return "" }
        let date = Date(timeIntervalSince1970: TimeInterval(lastUsedMs) / 1000)
        return date.formatted(Date.ISO8601FormatStyle(timeZone: timeZone).year().month().day())
    }
}

struct LearningRecordsPage: View {
    @Environment(DisplayLanguageStore.self) private var language

    @State private var model: LearningRecordsPageModel
    @State private var editing: Taigi_Engine_LearningRecord?

    init(client: any UserDataClient) {
        _model = State(initialValue: LearningRecordsPageModel(client: client))
    }

    var body: some View {
        Form {
            Section {
                Picker(language.string(.dictionaryLearningRecords), selection: $model.kind) {
                    Text(language.string(.dictionaryLearningRecordsFrequency))
                        .tag(Taigi_Engine_LearningRecordKind.frequency)
                    Text(language.string(.dictionaryLearningRecordsPhrases))
                        .tag(Taigi_Engine_LearningRecordKind.learnedPhrase)
                }
                Picker(language.string(.dictionaryLearningRecordsOrder), selection: $model.order) {
                    Text(language.string(.dictionaryLearningRecordsOrderMostUsed))
                        .tag(Taigi_Engine_LearningRecordOrder.mostUsed)
                    Text(language.string(.dictionaryLearningRecordsOrderMostRecent))
                        .tag(Taigi_Engine_LearningRecordOrder.mostRecent)
                }
            }

            Section {
                UserDataFilterField(text: $model.filter)
                recordTable
                recordTableControls
            } header: {
                HStack {
                    Text(language.string(.desktopEntriesSection))
                    Spacer()
                    Text(model.list.countLabel)
                        .foregroundStyle(.secondary)
                }
            }

            Section {
                WideActionRow(titleKey: .dictionaryClearLearningRecords, role: .destructive) {
                    model.askClearAll()
                }
            }
        }
        .formStyle(.grouped)
        // A confirmation dialog, not a second `.alert`: the chrome's alert is
        // the page's one alert (`CustomDictionaryPage`). Delete is the
        // destructive button; Cancel, Escape and a dismissal leave the stores
        // alone.
        .confirmationDialog(
            language.string(.dictionaryClearLearningRecords),
            isPresented: $model.isConfirmingClearAll,
            titleVisibility: .visible,
        ) {
            Button(language.string(.commonDelete), role: .destructive) { model.confirmClearAll() }
            Button(language.string(.commonCancel), role: .cancel) {}
        } message: {
            Text(language.string(.dictionaryClearLearningRecordsMessage))
        }
        .reloadWhenFilterSettles(model.filter) { await model.loadFirstPage() }
        .onChange(of: model.kind) { showFromFirstPage() }
        .onChange(of: model.order) { showFromFirstPage() }
        .sheet(item: $editing) { record in
            LearningRecordCountSheet(record: record) { count in
                Task { await model.setCount(of: record, to: count) }
            }
        }
        .userDataPageChrome(activity: model.activity, message: $model.message)
    }

    /// Another kind or order: its first page, the filter kept. The model
    /// has already dropped the selection.
    private func showFromFirstPage() {
        Task { await model.loadFirstPage() }
    }

    /// The rows, as Custom Dictionary states its own (`entryTable`): click
    /// selects, double-click edits the count, the selection is what `−`
    /// acts on, and a definite height so the table never scrolls inside the
    /// form.
    private var recordTable: some View {
        Table(model.list.rows, selection: $model.selectedRowID) {
            TableColumn(language.string(.dictionaryRomanLabel)) { record in
                Text(record.tl)
                    .foregroundStyle(.secondary)
            }
            TableColumn(language.string(.dictionaryHanziLabel)) { record in
                Text(record.text)
            }
            TableColumn(language.string(.dictionaryLearningRecordsCount)) { record in
                Text(verbatim: "\(record.count)")
                    .monospacedDigit()
            }
            .width(min: 40, ideal: 56)
            TableColumn(language.string(.dictionaryLearningRecordsLastUsed)) { record in
                Text(LearningRecordsPageModel.lastUsedLabel(record.lastUsedMs))
                    .monospacedDigit()
                    .foregroundStyle(.secondary)
            }
            .width(min: 80, ideal: 96)
        }
        .tableStyle(.inset)
        .alternatingRowBackgrounds(.disabled)
        .frame(height: UserDataListMetrics.pagedTableHeight)
        .overlay {
            if model.list.rows.isEmpty {
                emptyState
            }
        }
        .contextMenu(forSelectionType: Taigi_Engine_LearningRecord.ID.self) { ids in
            if let record = record(for: ids.first) {
                Button(language.string(.dictionaryLearningRecordsEditCount)) { editing = record }
                if record.canAddToCustomDictionary {
                    Button(language.string(.dictionaryLearningRecordsAddToCustomDictionary)) {
                        Task { await model.addToCustomDictionary(record) }
                    }
                }
                Button(language.string(.commonDelete), role: .destructive) {
                    Task { await model.delete(record) }
                }
            }
        } primaryAction: { ids in
            editing = record(for: ids.first)
        }
    }

    /// An empty store is a state — the tray, its sentence as the
    /// accessibility label; a filter matching nothing is a result, and says
    /// so in words (`CustomDictionaryPage.emptyState`).
    @ViewBuilder
    private var emptyState: some View {
        if model.filter.isEmpty {
            UserDataListEmptySymbol(
                symbolName: UserDataListMetrics.emptyStateSymbolName,
                accessibilityLabelKey: .dictionaryLearningRecordsEmpty,
            )
        } else {
            Text(language.string(.dictionaryNoResults))
                .foregroundStyle(.secondary)
        }
    }

    /// `−` under the table — nothing is added here by hand — with Add to
    /// Custom Dictionary beside it, and the pager at its trailing end.
    private var recordTableControls: some View {
        UserDataListControls(
            isRemoveEnabled: model.list.selectedRow != nil,
            onRemove: {
                guard let selectedRecord = model.list.selectedRow else { return }
                Task { await model.delete(selectedRecord) }
            },
            rowAction: addToCustomDictionaryControl,
        ) {
            UserDataListPager(
                page: model.list.page,
                pageCount: model.list.pageCount,
                onBackward: { Task { await model.pageBackward() } },
                onForward: { Task { await model.pageForward() } },
            )
        }
    }

    /// The button form of the context menu's Add to Custom Dictionary, so it
    /// is found without a right-click. Off unless the selected row is one the
    /// engine says can be added (one syllable or no Hanji is not).
    private var addToCustomDictionaryControl: (labelKey: StringKey, isEnabled: Bool, action: () -> Void) {
        (
            .dictionaryLearningRecordsAddToCustomDictionary,
            model.list.selectedRow?.canAddToCustomDictionary == true,
            {
                guard let selectedRecord = model.list.selectedRow else { return }
                Task { await model.addToCustomDictionary(selectedRecord) }
            },
        )
    }

    private func record(for id: Taigi_Engine_LearningRecord.ID?) -> Taigi_Engine_LearningRecord? {
        model.list.rows.first { $0.id == id }
    }
}

/// Edit one row's count: the word and its reading, the count as a field with
/// a stepper beside it, and — for word frequency — the note that counts past
/// 40 rank the same. A sheet, as `CustomDictionaryEntrySheet` is.
struct LearningRecordCountSheet: View {
    @Environment(\.dismiss) private var dismiss
    @Environment(DisplayLanguageStore.self) private var language

    /// The field's text rather than a bound number: a number-formatted field
    /// commits only when it ends editing, and a click on Save does not end
    /// it — the count typed last would be lost.
    @State private var countText: String
    private let record: Taigi_Engine_LearningRecord
    private let onSave: (Int) -> Void

    private static let countRange = 1 ... LearningRecordsPageModel.maxCount

    init(record: Taigi_Engine_LearningRecord, onSave: @escaping (Int) -> Void) {
        self.record = record
        self.onSave = onSave
        _countText = State(initialValue: String(max(1, record.count)))
    }

    /// The count typed, pulled into range; nil while the field holds no whole
    /// number of at least one.
    private var count: Int? {
        guard let typed = Int(countText.trimmingCharacters(in: .whitespaces)), typed >= 1 else { return nil }
        return min(typed, LearningRecordsPageModel.maxCount)
    }

    /// The stepper moves from the count typed, or from the stored one while
    /// the field holds none.
    private var stepperValue: Binding<Int> {
        Binding(
            get: { count ?? Int(clamping: max(1, record.count)) },
            set: { countText = String($0) },
        )
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            VStack(alignment: .leading, spacing: 4) {
                Text(language.string(.dictionaryLearningRecordsEditCount))
                    .font(.headline)
                Text(verbatim: "\(record.text)  \(record.tl)")
                    .foregroundStyle(.secondary)
            }
            Form {
                Section {
                    LabeledContent(language.string(.dictionaryLearningRecordsCount)) {
                        HStack(spacing: 4) {
                            TextField(language.string(.dictionaryLearningRecordsCount), text: $countText)
                                .labelsHidden()
                                .multilineTextAlignment(.trailing)
                                .frame(width: 96)
                            Stepper(
                                language.string(.dictionaryLearningRecordsCount),
                                value: stepperValue,
                                in: Self.countRange,
                            )
                            .labelsHidden()
                        }
                    }
                } footer: {
                    if let note = LearningRecordsPageModel.countNoteKey(for: record.kind) {
                        Text(language.string(note))
                            .foregroundStyle(.secondary)
                    }
                }
            }
            .formStyle(.grouped)
            HStack {
                Spacer()
                Button(language.string(.commonCancel), role: .cancel) { dismiss() }
                Button(language.string(.commonSave)) {
                    guard let count else { return }
                    onSave(count)
                    dismiss()
                }
                .keyboardShortcut(.defaultAction)
                .disabled(count == nil)
            }
        }
        .padding(20)
        .frame(width: 360)
    }
}
