import Foundation

/// ViewModel for `CustomDictionaryView`.
///
/// Owns the custom dictionary list, CRUD, CSV import/export, and the enable
/// toggle. The filter goes to the engine and the rows are its answer, in
/// pages, as on the desktop: a new filter lists from the top and makes every
/// answer still in flight stale. The view binds to `@Published` state and
/// calls the async actions; form state (input fields, alert flags) stays in
/// the view.
@MainActor
final class CustomDictionaryViewModel: ObservableObject {
    @Published private(set) var entries: [CustomDictionaryEntry] = []
    /// Every stored word: 0 is the empty dictionary, not an empty search.
    @Published private(set) var total = 0
    /// The words the current filter matches — what paging runs up to.
    @Published private(set) var matchingTotal = 0
    /// Only until the first answer: later loads leave the rows on screen.
    @Published var isLoading = true
    @Published var isCustomDictEnabled: Bool
    /// The last read failed: the rows on screen stay, and nothing is asked
    /// again until `retry()`.
    @Published private(set) var hasReadFailed = false
    /// The generation the rows on screen answer. The next page is asked only
    /// while it is the current one.
    @Published private(set) var listedGeneration = -1

    private var filter = ""
    /// Bumped by every change of what is asked; an answer started under an
    /// older number is dropped.
    private var generation = 0
    private var isLoadingNextPage = false
    private var settleTask: Task<Void, Never>?

    private let userData: any UserDataClient
    private let settings: SharedSettings
    private let filterSettle: Duration

    init(
        userData: any UserDataClient = CompositionRoot.userData,
        settings: SharedSettings = .shared,
        filterSettle: Duration = LearningRecordsViewModel.filterSettle,
    ) {
        self.userData = userData
        self.settings = settings
        self.filterSettle = filterSettle
        isCustomDictEnabled = settings.isCustomDictEnabled
    }

    func setCustomDictEnabled(_ enabled: Bool) {
        isCustomDictEnabled = enabled
        settings.isCustomDictEnabled = enabled
    }

    /// Reads the listed rows again, so the list keeps its place; a failed
    /// read's retry too.
    func load() async {
        await reload(rowCount: max(Int(LearningRecordsViewModel.pageSize), entries.count)).value
    }

    /// The search box changed: answers for the old text are stale at once,
    /// and the reload waits for the box to settle.
    @discardableResult
    func filterChanged(_ filter: String) -> Task<Void, Never> {
        self.filter = filter
        generation += 1
        settleTask?.cancel()
        let settle = filterSettle
        let task = Task {
            try? await Task.sleep(for: settle)
            guard !Task.isCancelled else { return }
            await reload().value
        }
        settleTask = task
        return task
    }

    /// The filter matches rows not listed yet.
    var hasMoreRows: Bool {
        entries.count < matchingTotal
    }

    /// What the list-end sentinel's `.task(id:)` keys on: it changes after
    /// every load that lands, so the next page is asked again even when the
    /// last row kept its id.
    var pagingKey: [Int] {
        [listedGeneration, entries.count]
    }

    /// Asks for the next page while the filter matches more and the rows
    /// listed answer what is asked now.
    func loadNextPage() async {
        guard hasMoreRows,
              !hasReadFailed,
              listedGeneration == generation,
              !isLoadingNextPage
        else { return }
        isLoadingNextPage = true
        let generation = generation
        do {
            let page = try await userData.listCustomEntries(
                filter: filter,
                limit: LearningRecordsViewModel.pageSize,
                offset: UInt32(clamping: entries.count),
            )
            guard generation == self.generation else { return }
            isLoadingNextPage = false
            // The page overrides the rows from where it really starts
            // (`CustomDictionaryPage.offset`).
            let kept = Array(entries.prefix(page.offset))
            let listed = Set(kept.map(\.id))
            entries = kept + page.entries.filter { !listed.contains($0.id) }
            total = page.total
            matchingTotal = page.matchingTotal
        } catch {
            guard generation == self.generation else { return }
            isLoadingNextPage = false
            hasReadFailed = true
        }
    }

    func save(_ entry: CustomDictionaryEntry) async {
        try? await userData.save(entry)
        await load()
    }

    func delete(id: String) async {
        try? await userData.delete(id: id)
        await load()
    }

    func deleteAll() async {
        try? await userData.deleteAll()
        await load()
    }

    /// Lists `rowCount` rows from the top for what is asked now, in one
    /// request, and shows them in one go; every answer still in flight is
    /// stale from here. A failed read keeps the rows on screen.
    @discardableResult
    private func reload(rowCount: Int = Int(LearningRecordsViewModel.pageSize)) -> Task<Void, Never> {
        generation += 1
        let generation = generation
        // A write overtakes a filter still settling.
        settleTask?.cancel()
        isLoadingNextPage = false
        let filter = filter
        return Task {
            do {
                let page = try await userData.listCustomEntries(
                    filter: filter,
                    limit: UInt32(clamping: rowCount),
                    offset: 0,
                )
                guard generation == self.generation else { return }
                entries = page.entries
                total = page.total
                matchingTotal = page.matchingTotal
                listedGeneration = generation
                hasReadFailed = false
            } catch {
                guard generation == self.generation else { return }
                hasReadFailed = true
            }
            isLoading = false
        }
    }

    func exportCSV() async throws -> String {
        try await String(decoding: userData.exportCSV(), as: UTF8.self)
    }

    func importFile(url: URL) async throws -> (imported: Int, skipped: Int) {
        try await userData.importCSV(url: url)
    }
}
