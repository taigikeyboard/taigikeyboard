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
    /// The hit whose lookups the dialog offers.
    @State private var selectedHit: DictionarySearchModel.Hit?

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
            // The pane's one control at its foot, sized up to stand out, as
            // mobile's search bar does.
            SearchField(placeholder: language.string(.dictionarySearchPlaceholder), text: $model.query)
                .controlSize(.large)
        }
        .padding(Metrics.padding)
        .background(.bar)
        .overlay(alignment: .top) { Divider() }
        // iOS and Android wait 300 ms (`DictionarySearchViewModel`).
        .reloadWhenFilterSettles(model.trimmedQuery, after: .milliseconds(300)) { await model.search() }
        .openURLFailureAlert($failedLink)
        .confirmationDialog(
            "",
            isPresented: Binding(get: { selectedHit != nil }, set: {
                if !$0 {
                    selectedHit = nil
                }
            }),
            presenting: selectedHit,
        ) { hit in
            if hit.hasMoeURL {
                Button(language.string(.dictionaryLookupMoe)) { open(hit.moeURL) }
            }
            if hit.hasChhoeURL {
                Button(language.string(.dictionaryLookupChhoe)) { open(hit.chhoeURL) }
            }
            Button(language.string(.commonCancel), role: .cancel) {}
        }
    }

    @ViewBuilder
    private var results: some View {
        if model.hits.isEmpty, !model.isSearching {
            Text(language.string(.dictionaryNoResults))
                .foregroundStyle(.secondary)
                .frame(maxWidth: .infinity, alignment: .leading)
        } else {
            // At most five one-line rows (the core's `VISIBLE_RESULT_LIMIT`),
            // all at full height, so no scroller. A new query's rows replace
            // the last one's when they arrive, as on iOS, Windows and Linux.
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

    /// One hit as a whole-row button: a click anywhere asks which page to
    /// look the reading up on, as iOS's row does (`confirmationDialog`).
    /// Disabled when the reading makes no URL.
    private func row(_ hit: DictionarySearchModel.Hit) -> some View {
        Button {
            selectedHit = hit
        } label: {
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
                Image(systemName: "arrow.up.forward.square")
                    .foregroundStyle(.secondary)
                    .accessibilityHidden(true)
            }
            .padding(.vertical, Metrics.rowInset)
            // The whole row, not only its text, takes the click.
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .disabled(!hit.hasMoeURL && !hit.hasChhoeURL)
        // Read as one element: the word, then its source badges.
        .accessibilityElement(children: .combine)
    }

    /// A lookup that silently does nothing reads as a broken one
    /// (`ExternalLinkButton`).
    /// Called from the lookup dialog: the failure alert is raised on the
    /// next turn of the main actor, once the dialog has gone, so the one
    /// presentation does not swallow the other.
    private func open(_ link: String) {
        if let url = URL(string: link), NSWorkspace.shared.open(url) {
            return
        }
        Task { @MainActor in failedLink = link }
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
        /// About a 36 pt row, a settings list's row height.
        static let rowInset: CGFloat = 10
    }
}
