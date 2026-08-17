// Looking a word up, at the top of the 詞庫 tab.

import AppKit
import SwiftUI

@MainActor
@Observable
final class DictionarySearchModel {
    /// How many of the results the list shows. The engine is asked for more so
    /// the 教育部-first sort has something to reorder; the rest are one scroll
    /// away in the dictionaries themselves.
    static let visibleResultLimit = 5

    /// Long enough that typing a word does not run a query per letter, short
    /// enough that stopping feels like an answer arriving.
    private static let debounce = Duration.milliseconds(300)

    var query = ""
    private(set) var results: [DictionarySearchResult] = []
    private(set) var isSearching = false

    private let service: DictionarySearchService
    /// Which search the results on screen came from. A search runs synchronous
    /// FFI on a background task that cannot be stopped once it has started, so
    /// an older one can still come back after a newer one — the newest wins by
    /// number, not by arrival.
    private var searchGeneration = 0

    init(service: DictionarySearchService) {
        self.service = service
    }

    func searchAfterTyping() async {
        // Claimed FIRST, before the debounce: a search already past its own
        // debounce and inside the engine is not cancellable, and if this one
        // waited until after the sleep to take a number, that older search
        // would come back to find its number still current and publish its
        // results under this query.
        searchGeneration += 1
        let generation = searchGeneration

        let trimmed = query.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else {
            results = []
            isSearching = false
            return
        }

        isSearching = true
        try? await Task.sleep(for: Self.debounce)
        guard !Task.isCancelled else { return }

        let service = service
        let found = await Task.detached { service.search(trimmed) }.value

        // Checked again on the way back: `Task.detached` is unstructured, so
        // cancelling the view's task does not stop it, and a newer search may
        // have started while this one was in the engine.
        guard !Task.isCancelled, generation == searchGeneration else { return }
        results = found
        isSearching = false
    }
}

/// The search field and its results, at the top of the 詞庫 tab's root.
///
/// In the content area rather than the window toolbar, which belongs to the
/// `[一般] [詞庫]` tabs; and on the root rather than behind a navigation push,
/// because looking a word up is the thing this tab is most often opened for.
struct DictionarySearchSection: View {
    @State private var model: DictionarySearchModel

    init(service: DictionarySearchService) {
        _model = State(initialValue: DictionarySearchModel(service: service))
    }

    var body: some View {
        Section {
            TextField("揣台語詞", text: $model.query)
                .textFieldStyle(.roundedBorder)

            if !model.query.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
                if model.results.isEmpty {
                    // Nothing while the query is still settling: "揣無" is an
                    // answer, and showing it before anything has been asked
                    // would be the wrong one.
                    if !model.isSearching {
                        Text("揣無這个詞。")
                            .foregroundStyle(.secondary)
                    }
                } else {
                    ForEach(model.results.prefix(DictionarySearchModel.visibleResultLimit)) { result in
                        DictionarySearchResultRow(result: result)
                    }
                }
            }
        } header: {
            Text("揣辭典")
        }
        .task(id: model.query) {
            await model.searchAfterTyping()
        }
    }
}

/// One result: what it says, where it came from, and where to read more.
struct DictionarySearchResultRow: View {
    let result: DictionarySearchResult

    @State private var didFailToOpen = false

    var body: some View {
        HStack(spacing: 8) {
            Text(result.roman)
                .foregroundStyle(.secondary)
            if let hanzi = result.hanzi {
                Text(hanzi)
            }
            ForEach(badgeLabels, id: \.self) { label in
                Text(label)
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .padding(.horizontal, 6)
                    .padding(.vertical, 2)
                    .background(.quaternary, in: RoundedRectangle(cornerRadius: 4))
            }
            Spacer()
            // Absent for the user's own entries, which the two sites have
            // nothing to say about — an empty menu would be a control that
            // does nothing.
            if result.moeURL != nil || result.chhoeURL != nil {
                Menu {
                    if let url = result.moeURL {
                        Button("教育部辭典") { open(url) }
                    }
                    if let url = result.chhoeURL {
                        Button("ChhoeTaigi 辭典") { open(url) }
                    }
                } label: {
                    Image(systemName: "arrow.up.forward.square")
                }
                .menuStyle(.borderlessButton)
                .fixedSize()
            }
        }
        .alert("拍袂開網頁", isPresented: $didFailToOpen) {
            Button("好") {}
        }
    }

    /// The badge for each source the row belongs to, in bit order, with the
    /// supplementary sources collapsed into one — they are one idea to the
    /// user, and four badges saying it would crowd out the word.
    private var badgeLabels: [String] {
        var seen = Set<String>()
        return result.sources.compactMap { source in
            let label = Self.badgeLabel(for: source)
            return seen.insert(label).inserted ? label : nil
        }
    }

    private static func badgeLabel(for source: DictionarySource) -> String {
        switch source {
        case .kautian: "教典"
        case .taigitv: "台語新詞"
        case .itaigi: "iTaigi"
        case .sitbut: "植物名彙"
        case .taihoa: "台華對照"
        case .taijit: "臺日"
        case .kungge: "工藝辭典"
        case .stti: "學科術語"
        case .lkk: "漢羅合用"
        case .khpoo, .khiin, .dev: "補充資料"
        case .custom: "自訂詞庫"
        }
    }

    private func open(_ url: URL) {
        // `open` answers false when nothing could handle the URL, and a menu
        // item that silently does nothing reads as a broken one.
        didFailToOpen = !NSWorkspace.shared.open(url)
    }
}
