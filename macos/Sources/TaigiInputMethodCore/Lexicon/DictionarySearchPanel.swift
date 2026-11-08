// The search pinned under the Manage Dictionaries toggles.

import AppKit
import SwiftUI

/// Manage Dictionaries' search (#792): the hits above an `NSSearchField`, the
/// way iOS's `DictionarySearchPanel` pops them up over its bar, each with its
/// source badges and a menu to look the reading up on the web. Its own view so
/// a keystroke re-renders the panel, not the toggle form above it.
struct DictionarySearchPanel: View {
    @Environment(DisplayLanguageStore.self) private var language

    @State private var model: DictionarySearchModel
    /// The page that could not be opened, for the alert.
    @State private var failedLink: String?

    /// `model` is the live search; tests pass one over a stand-in lookup.
    init(model: DictionarySearchModel = DictionarySearchModel()) {
        _model = State(initialValue: model)
    }

    var body: some View {
        VStack(spacing: Metrics.spacing) {
            // Nothing for an empty query, so the panel is the field alone.
            if !model.trimmedQuery.isEmpty {
                results
            }
            SearchField(placeholder: language.string(.dictionarySearchPlaceholder), text: $model.query)
        }
        .padding(Metrics.padding)
        .background(.bar)
        .overlay(alignment: .top) { Divider() }
        // iOS and Android wait 300 ms (`DictionarySearchViewModel`).
        .reloadWhenFilterSettles(model.trimmedQuery, after: .milliseconds(300)) { await model.search() }
        .openURLFailureAlert($failedLink)
    }

    @ViewBuilder
    private var results: some View {
        if model.hits.isEmpty, !model.isSearching {
            Text(language.string(.dictionaryNoResults))
                .foregroundStyle(.secondary)
                .frame(maxWidth: .infinity, alignment: .leading)
        } else {
            // At most five one-line rows (the core's `VISIBLE_RESULT_LIMIT`):
            // shorter than the other desktops' 200 px scroll cap, so no
            // scroller. A new query's rows replace the last one's when they
            // arrive, as on iOS, Windows and Linux.
            VStack(spacing: 0) {
                ForEach(model.hits, id: \.key) { hit in
                    row(hit)
                    if hit.key != model.hits.last?.key {
                        Divider()
                    }
                }
            }
        }
    }

    private func row(_ hit: DictionarySearchModel.Hit) -> some View {
        HStack(spacing: Metrics.spacing) {
            Text(verbatim: hit.roman)
            if hit.hasHanji {
                Text(verbatim: hit.hanji)
            }
            // A key this build does not know is dropped, not shown raw.
            ForEach(hit.badgeKeys.compactMap(StringKey.init(rawValue:)), id: \.self) { key in
                Badge(text: language.string(key))
            }
            Spacer()
            // None when the reading makes no URL.
            if hit.hasMoeURL || hit.hasChhoeURL {
                Menu {
                    if hit.hasMoeURL {
                        Button(language.string(.dictionaryLookupMoe)) { open(hit.moeURL) }
                    }
                    if hit.hasChhoeURL {
                        Button(language.string(.dictionaryLookupChhoe)) { open(hit.chhoeURL) }
                    }
                } label: {
                    Image(systemName: "arrow.up.forward.square")
                }
                .menuStyle(.borderlessButton)
                .menuIndicator(.hidden)
                .fixedSize()
            }
        }
        .padding(.vertical, Metrics.rowInset)
    }

    /// A lookup that silently does nothing reads as a broken one
    /// (`ExternalLinkButton`).
    private func open(_ link: String) {
        if let url = URL(string: link), NSWorkspace.shared.open(url) {
            return
        }
        failedLink = link
    }

    /// One source's compact label.
    private struct Badge: View {
        let text: String

        var body: some View {
            Text(verbatim: text)
                .font(.caption)
                .padding(.horizontal, 6)
                .padding(.vertical, 1)
                .background(.quaternary, in: Capsule())
        }
    }

    private enum Metrics {
        static let spacing: CGFloat = 8
        static let padding: CGFloat = 12
        static let rowInset: CGFloat = 6
    }
}
