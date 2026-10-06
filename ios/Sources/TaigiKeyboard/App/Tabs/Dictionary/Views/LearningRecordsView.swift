import SwiftUI

/// One kind's Learning Records list (word frequency or learned phrases),
/// opened from its row on the Learning Records subpage.
/// What the keyboard learned from the user's picks, in the order picked: edit
/// one row's count, swipe to delete one row, swipe a row the other way to
/// add its word to the custom dictionary. No add by hand (a word
/// the user wants is a custom word) and no wipe here — Delete Learning
/// Records empties every kind, so it sits on the subpage above
/// (`LearningRecordsMenuView`).
struct LearningRecordsView: View {
    @Environment(DisplayLanguageStore.self) private var lang
    @StateObject private var viewModel: LearningRecordsViewModel

    @State private var filterText = ""
    /// The row the edit-count alert is open for.
    @State private var editingRecord: Taigi_Engine_LearningRecord?
    @State private var countInput = ""

    init(kind: Taigi_Engine_LearningRecordKind) {
        _viewModel = StateObject(wrappedValue: LearningRecordsViewModel(kind: kind))
    }

    /// The page title and the list header.
    private var title: String {
        lang.string(viewModel.kind.titleKey)
    }

    var body: some View {
        List {
            if viewModel.isLoading {
                Section {
                    ProgressView()
                        .frame(maxWidth: .infinity)
                }
            } else {
                // Order
                Section {
                    Picker(
                        selection: Binding(
                            get: { viewModel.order },
                            set: { viewModel.selectOrder($0) },
                        ),
                    ) {
                        Text(lang.string(.dictionaryLearningRecordsOrderMostUsed)).tag(Taigi_Engine_LearningRecordOrder.mostUsed)
                        Text(lang.string(.dictionaryLearningRecordsOrderMostRecent)).tag(Taigi_Engine_LearningRecordOrder.mostRecent)
                    } label: {
                        Text(lang.string(.dictionaryLearningRecordsOrder))
                    }
                    .pickerStyle(.menu)
                }

                // Record list
                Section {
                    if viewModel.records.isEmpty {
                        if viewModel.failedRead != nil {
                            retryRow
                        } else if filterText.isEmpty {
                            DictionaryEmptyState(message: lang.string(.dictionaryLearningRecordsEmpty))
                        } else {
                            Text(lang.string(.dictionaryNoResults))
                                .foregroundColor(.secondary)
                        }
                    } else {
                        ForEach(viewModel.records, id: \.id) { record in
                            recordRow(record)
                        }
                        if viewModel.failedRead != nil {
                            retryRow
                        } else if viewModel.hasMoreRows {
                            nextPageRow
                        }
                    }
                } header: {
                    Text(title)
                        .font(AppStyle.sectionHeaderFont)
                }
            }
        }
        .safeAreaInset(edge: .bottom) {
            SearchBar(text: $filterText, placeholder: lang.string(.dictionarySearchPlaceholder))
        }
        .onChange(of: filterText) { _, text in
            viewModel.filterChanged(text)
        }
        .navigationTitle(title)
        .navigationBarTitleDisplayMode(.large)
        .alert(
            lang.string(.dictionaryLearningRecordsEditCount),
            isPresented: Binding(
                get: { editingRecord != nil },
                set: {
                    if !$0 {
                        editingRecord = nil
                    }
                },
            ),
            presenting: editingRecord,
        ) { record in
            TextField(lang.string(.dictionaryLearningRecordsCount), text: $countInput)
                .keyboardType(.numberPad)
            Button(lang.string(.commonCancel), role: .cancel) {}
            Button(lang.string(.commonSave)) {
                guard let count = LearningRecordsViewModel.count(from: countInput) else { return }
                Task { await viewModel.setCount(record, to: count) }
            }
            .disabled(LearningRecordsViewModel.count(from: countInput) == nil)
        }
        .alert(
            viewModel.notice.map { lang.string($0.titleKey) } ?? "",
            isPresented: Binding(
                get: { viewModel.notice != nil },
                set: {
                    if !$0 {
                        viewModel.notice = nil
                    }
                },
            ),
            presenting: viewModel.notice,
        ) { _ in
            Button(lang.string(.commonOk), role: .cancel) {}
        } message: { notice in
            if let detail = notice.detail {
                Text(detail)
            }
        }
        .task {
            await viewModel.load()
        }
    }

    // MARK: - Rows

    /// The list end: shown, it asks for the next page — again after every
    /// load that lands (`pagingKey`).
    private var nextPageRow: some View {
        ProgressView()
            .frame(maxWidth: .infinity)
            .task(id: viewModel.pagingKey) {
                await viewModel.loadNextPage()
            }
    }

    /// A read failed: nothing is asked again until this is tapped, and the
    /// tap repeats that read.
    private var retryRow: some View {
        Button {
            Task { await viewModel.retry() }
        } label: {
            HStack {
                Image(latinSystemName: "arrow.clockwise")
                Text(lang.string(.dictionaryLearningRecordsReadFailed))
            }
            .foregroundColor(.secondary)
        }
    }

    private func recordRow(_ record: Taigi_Engine_LearningRecord) -> some View {
        Button {
            countInput = String(record.count)
            editingRecord = record
        } label: {
            HStack {
                VStack(alignment: .leading, spacing: 2) {
                    Text(record.text)
                        .font(AppStyle.bodyFont)
                        .foregroundColor(.primary)
                    Text(record.tl)
                        .font(AppStyle.captionFont)
                        .foregroundColor(.secondary)
                }
                Spacer()
                VStack(alignment: .trailing, spacing: 2) {
                    Text(String(record.count))
                        .font(AppStyle.bodyFont)
                        .monospacedDigit()
                        .foregroundColor(.primary)
                    Text(record.lastUsedLabel)
                        .font(AppStyle.captionFont)
                        .foregroundColor(.secondary)
                }
                Image(latinSystemName: "chevron.right")
                    .font(AppStyle.captionFont)
                    .foregroundColor(.secondary)
            }
        }
        .swipeActions(edge: .trailing) {
            Button(role: .destructive) {
                Task { await viewModel.delete(record) }
            } label: {
                Image(latinSystemName: "trash")
            }
        }
        .swipeActions(edge: .leading, allowsFullSwipe: false) {
            // Rows the engine says can be added: one syllable or no Hanji is not.
            if record.canAddToCustomDictionary {
                Button {
                    Task { await viewModel.addToCustomDictionary(record) }
                } label: {
                    Image(latinSystemName: "text.badge.plus")
                }
                .tint(.accentColor)
                .accessibilityLabel(lang.string(.dictionaryLearningRecordsAddToCustomDictionary))
            }
        }
    }
}

extension Taigi_Engine_LearningRecordKind {
    /// The kind's row on the Learning Records subpage and its list's title.
    var titleKey: StringKey {
        self == .frequency ? .dictionaryLearningRecordsFrequency : .dictionaryLearningRecordsPhrases
    }
}

private extension LearningRecordsNotice {
    var titleKey: StringKey {
        switch self {
        case .readFailed: .dictionaryLearningRecordsReadFailed
        case .writeFailed: .dictionaryLearningRecordsWriteFailed
        case .gone: .dictionaryLearningRecordGone
        case .addedToCustomDictionary: .dictionaryLearningRecordsAddedToCustomDictionary
        }
    }

    var detail: String? {
        switch self {
        case let .readFailed(detail), let .writeFailed(detail): detail
        case .gone, .addedToCustomDictionary: nil
        }
    }
}
