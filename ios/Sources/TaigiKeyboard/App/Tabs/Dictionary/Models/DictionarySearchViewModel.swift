import Foundation

/// ViewModel for the Manage Dictionaries page search bar.
///
/// Owns published state + debounce; delegates the search pipeline to
/// `DictionarySearchService`.
@MainActor
final class DictionarySearchViewModel: ObservableObject {
    @Published var searchText = ""
    // The View only renders the first N results.
    @Published var results: [DictionarySearchResult] = []
    // Stays true through the debounce window, not just the request itself.
    @Published var isSearching = false

    private var searchTask: Task<Void, Never>?
    private let service: DictionarySearchService

    init(service: DictionarySearchService = CompositionRoot.dictionarySearchService) {
        self.service = service
    }

    /// Debounce 300ms then run the search.
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

            let newResults = await service.search(query: query)
            guard !Task.isCancelled else { return }
            results = newResults
            isSearching = false
        }
    }
}
