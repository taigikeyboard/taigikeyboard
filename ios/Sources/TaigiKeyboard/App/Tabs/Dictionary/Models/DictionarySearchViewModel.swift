// Dictionary 分頁底部搜尋列的 ViewModel — 持有輸入文字、搜尋結果、loading 狀態。
// 含 300ms debounce,實際查詢委派給 DictionarySearchService。

import Foundation

/// ViewModel for the Dictionary tab search bar.
///
/// Owns published state + debounce; delegates the search pipeline to
/// `DictionarySearchService`.
// 詞典搜尋列 ViewModel,搭配 onChange + Task 實作 debounce 搜尋。
@MainActor
final class DictionarySearchViewModel: ObservableObject {
    // TextField 雙向綁定的搜尋字串。
    @Published var searchText = ""
    // 搜尋結果清單,View 取前 N 筆顯示。
    @Published var results: [DictionarySearchResult] = []
    // 搜尋進行中旗標,debounce 期間也保持為 true。
    @Published var isSearching = false

    private var searchTask: Task<Void, Never>?
    private let service: DictionarySearchService
    private let logger = DebugLogger(category: "DictionarySearchVM")

    init(service: DictionarySearchService = CompositionRoot.dictionarySearchService) {
        self.service = service
    }

    /// Debounce 300ms then run the search.
    // 文字變動觸發點,取消前一個 task → 等 300ms → 呼叫 service.search()。
    func onSearchTextChanged() {
        searchTask?.cancel()

        let query = searchText.trimmingCharacters(in: .whitespaces)
        guard !query.isEmpty else {
            results = []
            isSearching = false
            return
        }

        isSearching = true
        searchTask = Task {
            try? await Task.sleep(nanoseconds: 300_000_000)
            guard !Task.isCancelled else { return }

            do {
                let newResults = try await service.search(query: query)
                guard !Task.isCancelled else { return }
                results = newResults
                isSearching = false
            } catch {
                guard !Task.isCancelled else { return }
                logger.error("[SEARCH] Failed: \(error.localizedDescription)")
                results = []
                isSearching = false
            }
        }
    }
}
