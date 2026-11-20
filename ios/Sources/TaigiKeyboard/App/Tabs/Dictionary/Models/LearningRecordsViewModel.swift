import Foundation

/// ViewModel for `LearningRecordsView`: what the keyboard learned of one
/// kind, in pages the engine cuts
/// (`docs/architecture/learning-records-page-roadmap.md`).
///
/// The order and the filter go to the engine; the rows on screen are its
/// answer. A change of either starts a load from the top and makes every answer still in
/// flight stale — the newest request wins, as on the desktop's `Listing`. Form state (the edit alert's field) stays
/// in the view, and so does the notice: every load and write returns the
/// notice it earns rather than publishing it, because the notice alert
/// clears it from inside a SwiftUI view update, where a `@Published` write
/// is not allowed.
@MainActor
final class LearningRecordsViewModel: ObservableObject {
    /// Rows per engine page; the next page loads when the last row shows.
    /// `nonisolated`: `reload(rowCount:)`'s default argument reads it outside the main actor.
    nonisolated static let pageSize: UInt32 = 100
    /// The filter reloads once typing pauses this long — the desktop's
    /// `FILTER_SETTLE`, so a word typed letter by letter is one query.
    /// `nonisolated`: `init`'s default argument reads it outside the main actor.
    nonisolated static let filterSettle: Duration = .milliseconds(200)

    @Published private(set) var order: Taigi_Engine_LearningRecordOrder = .mostUsed
    @Published private(set) var records: [Taigi_Engine_LearningRecord] = []
    /// Only until the first answer: later loads leave the rows on screen.
    @Published private(set) var isLoading = true
    /// The read that failed and waits for `retry()`. With no rows listed the
    /// page says "could not read", never "nothing learned yet".
    @Published private(set) var failedRead: LearningRecordsRead?
    /// The rows the current filter matches — what paging runs up to.
    @Published private(set) var matchingTotal = 0
    /// The generation the rows on screen answer. The next page is asked
    /// only while it is the current one: rows of an older order or filter must not get the new query's next page appended.
    @Published private(set) var listedGeneration = -1

    private var filter = ""
    /// Bumped by every change of what is asked; an answer started under an
    /// older number is dropped.
    private var generation = 0
    private var isLoadingNextPage = false
    private var settleTask: Task<LearningRecordsNotice?, Never>?

    /// Frequency or learned phrases; fixed for the page's life.
    let kind: Taigi_Engine_LearningRecordKind
    private let userData: any UserDataClient
    private let filterSettle: Duration

    init(
        kind: Taigi_Engine_LearningRecordKind,
        userData: any UserDataClient = CompositionRoot.userData,
        filterSettle: Duration = LearningRecordsViewModel.filterSettle,
    ) {
        self.kind = kind
        self.userData = userData
        self.filterSettle = filterSettle
    }

    /// The first appearance lists the first page; a later one (back from
    /// another tab, where the keyboard may have learned more) reads the
    /// listed rows again, keeping the place in the list.
    func load() async -> LearningRecordsNotice? {
        guard listedGeneration >= 0 else {
            return await reload().value
        }
        return await reloadInPlace().value
    }

    /// Repeats the read that failed: the next page, or the listed rows from
    /// the first (the first page when none is listed).
    func retry() async -> LearningRecordsNotice? {
        switch failedRead {
        case .nextPage: await loadNextPage()
        case .list: await reloadInPlace().value
        case nil: nil
        }
    }

    @discardableResult
    func selectOrder(_ order: Taigi_Engine_LearningRecordOrder) -> Task<LearningRecordsNotice?, Never> {
        self.order = order
        return reload()
    }

    /// The search box changed: answers for the old text are stale at once,
    /// and the reload waits for the box to settle.
    @discardableResult
    func filterChanged(_ filter: String) -> Task<LearningRecordsNotice?, Never> {
        self.filter = filter
        generation += 1
        settleTask?.cancel()
        let settle = filterSettle
        let task = Task {
            try? await Task.sleep(for: settle)
            guard !Task.isCancelled else { return nil as LearningRecordsNotice? }
            return await reload().value
        }
        settleTask = task
        return task
    }

    /// The filter matches rows not listed yet.
    var hasMoreRows: Bool {
        records.count < matchingTotal
    }

    /// What the list-end sentinel's `.task(id:)` keys on: it changes after
    /// every load that lands, so the next page is asked again even when the
    /// last row kept its id.
    var pagingKey: [Int] {
        [listedGeneration, records.count]
    }

    /// Asks for the next page while the filter matches more and the rows
    /// listed answer what is asked now.
    func loadNextPage() async -> LearningRecordsNotice? {
        guard hasMoreRows,
              listedGeneration == generation,
              !isLoadingNextPage
        else { return nil }
        isLoadingNextPage = true
        let generation = generation
        let offset = UInt32(clamping: records.count)
        do {
            let page = try await userData.listLearningRecords(
                kind: kind,
                order: order,
                filter: filter,
                limit: Self.pageSize,
                offset: offset,
            )
            guard generation == self.generation else { return nil }
            isLoadingNextPage = false
            // The matches shrank under the list (the engine pulled the
            // offset back): the rows listed no longer line up with the store.
            guard page.offset == offset else {
                return await reloadInPlace().value
            }
            // A row the keyboard moved while the page was read is not listed twice.
            let listed = Set(records.map(\.id))
            records += page.records.filter { !listed.contains($0.id) }
            matchingTotal = Int(page.matchingTotal)
            failedRead = nil
            return nil
        } catch {
            guard generation == self.generation else { return nil }
            isLoadingNextPage = false
            failedRead = .nextPage
            return .readFailed(detail: error.localizedDescription)
        }
    }

    func setCount(_ record: Taigi_Engine_LearningRecord, to count: Int64) async -> LearningRecordsWrite {
        await write { try await $0.setLearningRecordCount(record, count: count) ? nil : .gone }
    }

    func delete(_ record: Taigi_Engine_LearningRecord) async -> LearningRecordsWrite {
        await write { try await $0.deleteLearningRecord(record) ? nil : .gone }
    }

    /// Files the learned phrase's word as a custom word; the engine then
    /// forgets the phrase (the reload drops its row). Offered only where the
    /// row's `canAddToCustomDictionary` is set — never on word frequency.
    /// A refusal keeps the row and says why.
    func addToCustomDictionary(_ record: Taigi_Engine_LearningRecord) async -> LearningRecordsWrite {
        await write {
            try await $0.addLearningRecordToCustomDictionary(record)
            return .addedToCustomDictionary
        }
    }

    /// The count the edit alert's field holds: a whole number of at least 1;
    /// anything else keeps Save disabled. A count above the ceiling is not
    /// refused: the engine clamps it to its `MAX_COUNT`.
    static func count(from text: String) -> Int64? {
        Int64(text.trimmingCharacters(in: .whitespaces)).flatMap { $0 >= 1 ? $0 : nil }
    }

    // MARK: - Loading

    /// Every write reloads: the row moved, went, or was never there. `change`
    /// answers the notice its outcome earns (a row already gone is said, not
    /// reported as a failure), or `nil` for none. The outcome is answered as
    /// soon as the write lands, with the reload it started.
    private func write(_ change: (any UserDataClient) async throws -> LearningRecordsNotice?) async -> LearningRecordsWrite {
        let outcome: LearningRecordsNotice?
        do {
            outcome = try await change(userData)
        } catch {
            outcome = .writeFailed(detail: error.localizedDescription)
        }
        return LearningRecordsWrite(outcome: outcome, reload: reloadInPlace())
    }

    /// Reloads as many rows as are listed, so the list keeps its place.
    private func reloadInPlace() -> Task<LearningRecordsNotice?, Never> {
        reload(rowCount: max(Int(Self.pageSize), records.count))
    }

    /// Lists `rowCount` rows from the top for what is asked now in one engine
    /// read (one snapshot, so no row comes twice); every answer still in
    /// flight is stale from here. A stale answer earns no notice.
    @discardableResult
    private func reload(rowCount: Int = Int(pageSize)) -> Task<LearningRecordsNotice?, Never> {
        generation += 1
        let generation = generation
        // An order change, a write or a retry overtakes a filter still settling.
        settleTask?.cancel()
        isLoadingNextPage = false
        let (order, filter) = (order, filter)
        return Task {
            var notice: LearningRecordsNotice?
            do {
                let page = try await userData.listLearningRecords(
                    kind: kind,
                    order: order,
                    filter: filter,
                    limit: UInt32(clamping: rowCount),
                    offset: 0,
                )
                guard generation == self.generation else { return nil }
                records = page.records
                matchingTotal = Int(page.matchingTotal)
                listedGeneration = generation
                failedRead = nil
            } catch {
                guard generation == self.generation else { return nil }
                failedRead = .list
                notice = .readFailed(detail: error.localizedDescription)
            }
            isLoading = false
            return notice
        }
    }
}

/// A read `LearningRecordsViewModel.retry()` repeats.
enum LearningRecordsRead {
    /// The page after the listed rows.
    case nextPage
    /// The listed rows, from the first.
    case list
}

/// What the page tells the user after a load or a write; the failures
/// carry the engine's own detail.
enum LearningRecordsNotice: Equatable {
    case readFailed(detail: String)
    case writeFailed(detail: String)
    /// An edit or delete found the row deleted, evicted, or its id taken.
    case gone
    /// A learned phrase's word was filed in the custom dictionary (the
    /// phrase is then forgotten here).
    case addedToCustomDictionary

    /// The notice the page shows once `self` is earned while `shown` is up:
    /// a read failure waits behind a notice still up (a write's own says
    /// more than its reload's); every other notice replaces it.
    func arriving(over shown: Self?) -> Self {
        if case .readFailed = self, let shown {
            return shown
        }
        return self
    }
}

/// A write's outcome notice, answered when the write lands, and the reload
/// of the listed rows it started.
struct LearningRecordsWrite {
    let outcome: LearningRecordsNotice?
    let reload: Task<LearningRecordsNotice?, Never>
}

extension Taigi_Engine_LearningRecord {
    /// The day the row was last used, `yyyy-MM-dd` in `timeZone` — the
    /// desktop's `last_used_label`; empty when the store held no readable time.
    func lastUsedLabel(in timeZone: TimeZone = .current) -> String {
        guard lastUsedMs > 0 else { return "" }
        return Date(timeIntervalSince1970: TimeInterval(lastUsedMs) / 1000)
            .formatted(Date.ISO8601FormatStyle(timeZone: timeZone).year().month().day())
    }
}
