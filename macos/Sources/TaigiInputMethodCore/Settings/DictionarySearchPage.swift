// The 揣辭典 pane: looking a word up across the enabled dictionaries.

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

/// The search field and its results. Its pane title comes from the sidebar
/// router (`SettingsSplitView`), like every other page's — the section
/// carries no header of its own, which would repeat the title just below it.
struct DictionarySearchPage: View {
    @Environment(DisplayLanguageStore.self) private var language

    @State private var model: DictionarySearchModel

    init(service: DictionarySearchService) {
        _model = State(initialValue: DictionarySearchModel(service: service))
    }

    var body: some View {
        Form {
            Section {
                UserDataFilterField(text: $model.query)

                if !model.query.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
                    if model.results.isEmpty {
                        // Nothing while the query is still settling: "揣無" is an
                        // answer, and showing it before anything has been asked
                        // would be the wrong one.
                        if !model.isSearching {
                            Text(language.string(.dictionaryNoResults))
                                .foregroundStyle(.secondary)
                        }
                    } else {
                        ForEach(model.results.prefix(DictionarySearchModel.visibleResultLimit)) { result in
                            DictionarySearchResultRow(result: result)
                        }
                    }
                }
            }
        }
        .formStyle(.grouped)
        .task(id: model.query) {
            await model.searchAfterTyping()
        }
    }
}

/// One result: what it says, where it came from, and where to read more.
struct DictionarySearchResultRow: View {
    @Environment(DisplayLanguageStore.self) private var language

    let result: DictionarySearchResult

    @State private var didFailToOpen = false

    var body: some View {
        HStack(spacing: 8) {
            Text(result.roman)
                .foregroundStyle(.secondary)
            if let hanzi = result.hanzi {
                Text(hanzi)
            }
            ForEach(badgeKeys, id: \.self) { key in
                Text(language.string(key))
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
                        Button(language.string(.dictionaryLookupMoe)) { open(url) }
                    }
                    if let url = result.chhoeURL {
                        Button(language.string(.dictionaryLookupChhoe)) { open(url) }
                    }
                } label: {
                    Image(systemName: "arrow.up.forward.square")
                }
                .menuStyle(.borderlessButton)
                .fixedSize()
            }
        }
        .alert(language.string(.macosOpenURLFailed), isPresented: $didFailToOpen) {
            Button(language.string(.commonOk)) {}
        }
    }

    /// The badge for each source the row belongs to, in bit order, with the
    /// supplementary sources collapsed into one — they are one idea to the
    /// user, and four badges saying it would crowd out the word.
    private var badgeKeys: [StringKey] {
        var seen = Set<StringKey>()
        return result.sources.compactMap { source in
            let key = Self.badgeKey(for: source)
            return seen.insert(key).inserted ? key : nil
        }
    }

    private static func badgeKey(for source: DictionarySource) -> StringKey {
        switch source {
        case .kautian: .dictionaryKautianTag
        case .taigitv: .dictionaryTaigitvTag
        case .itaigi: .dictionaryITaigiTag
        case .sitbut: .dictionarySitbutTag
        case .taihoa: .dictionaryTaihoaTag
        case .taijit: .dictionaryTaijitTag
        case .kungge: .dictionaryKunggeTag
        case .stti: .dictionarySttiTag
        case .lkk: .dictionaryLkkTag
        case .khpoo, .khiin, .dev: .dictionarySupplementSectionTitle
        case .custom: .dictionaryCustomDictionary
        }
    }

    private func open(_ url: URL) {
        // `open` answers false when nothing could handle the URL, and a menu
        // item that silently does nothing reads as a broken one.
        didFailToOpen = !NSWorkspace.shared.open(url)
    }
}
