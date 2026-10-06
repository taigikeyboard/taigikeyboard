import SwiftUI

/// Learning Records subpage — one row per kind (word frequency, learned
/// phrases), each opening that kind's `LearningRecordsView`, then Delete
/// Learning Records: it empties every learning store at once (the next-word
/// association no page lists included); the custom words stay.
struct LearningRecordsMenuView: View {
    @Environment(DisplayLanguageStore.self) private var lang

    private let userData: any UserDataClient = CompositionRoot.userData

    @State private var showClearAlert = false
    /// While the clear runs: a list opened now could read the rows it is
    /// emptying, so the kind rows wait with the button.
    @State private var isClearing = false
    @State private var clearResult: ClearLearningRecordsResult?

    var body: some View {
        List {
            Section {
                NavigationLink(destination: LearningRecordsView(kind: .frequency)) {
                    Text(lang.string(Taigi_Engine_LearningRecordKind.frequency.titleKey))
                }
                NavigationLink(destination: LearningRecordsView(kind: .learnedPhrase)) {
                    Text(lang.string(Taigi_Engine_LearningRecordKind.learnedPhrase.titleKey))
                }
            }
            .disabled(isClearing)

            Section {
                Button(role: .destructive) {
                    showClearAlert = true
                } label: {
                    Text(lang.string(.dictionaryClearLearningRecords))
                }
                .disabled(isClearing)
            }
        }
        .navigationTitle(lang.string(.dictionaryLearningRecords))
        .navigationBarTitleDisplayMode(.large)
        .alert(lang.string(.dictionaryClearLearningRecords), isPresented: $showClearAlert) {
            Button(lang.string(.commonCancel), role: .cancel) {}
            Button(lang.string(.commonDelete), role: .destructive) {
                clearLearningRecords()
            }
        } message: {
            Text(lang.string(.dictionaryClearLearningRecordsMessage))
        }
        .alert(
            clearResult.map { lang.string($0.titleKey) } ?? "",
            isPresented: Binding(
                get: { clearResult != nil },
                set: {
                    if !$0 {
                        clearResult = nil
                    }
                },
            ),
            presenting: clearResult,
        ) { _ in
            Button(lang.string(.commonOk), role: .cancel) {}
        } message: { result in
            if case let .failed(detail) = result {
                Text(detail)
            }
        }
    }

    private func clearLearningRecords() {
        guard !isClearing else { return }
        isClearing = true
        Task {
            defer { isClearing = false }
            do {
                try await userData.clearLearningRecords()
                clearResult = .done
            } catch {
                clearResult = .failed(detail: error.localizedDescription)
            }
        }
    }
}

/// What Delete Learning Records reports back: the store's own diagnostic
/// follows a failure, as on desktop.
private enum ClearLearningRecordsResult {
    case done
    case failed(detail: String)

    var titleKey: StringKey {
        switch self {
        case .done: .dictionaryClearLearningRecordsDone
        case .failed: .dictionaryClearLearningRecordsFailed
        }
    }
}
