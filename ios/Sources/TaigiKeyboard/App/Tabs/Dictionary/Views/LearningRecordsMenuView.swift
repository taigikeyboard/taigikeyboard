import SwiftUI

/// Learning Records subpage — one row per kind (word frequency, learned
/// phrases), each opening that kind's `LearningRecordsView`, then Delete
/// Learning Records: it empties every learning store at once (the next-word
/// association no page lists included); the custom words stay.
struct LearningRecordsMenuView: View {
    @Environment(DisplayLanguageStore.self) private var lang

    private let userData: any UserDataClient = CompositionRoot.userData

    /// While the clear runs: a list opened now could read the rows it is
    /// emptying, so the kind rows wait with the button.
    @State private var isClearing = false
    /// The store's own diagnostic when the clear failed, as on desktop; a
    /// clear that landed says nothing (USER 2026-10-16).
    @State private var clearFailureDetail: String?

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

            // Runs on the tap, no confirmation (USER 2026-10-16).
            Section {
                Button(role: .destructive) {
                    clearLearningRecords()
                } label: {
                    Text(lang.string(.dictionaryClearLearningRecords))
                }
                .disabled(isClearing)
            }
        }
        .navigationTitle(lang.string(.dictionaryLearningRecords))
        .navigationBarTitleDisplayMode(.large)
        .alert(
            lang.string(.dictionaryClearLearningRecordsFailed),
            isPresented: Binding(
                get: { clearFailureDetail != nil },
                set: {
                    if !$0 {
                        clearFailureDetail = nil
                    }
                },
            ),
            presenting: clearFailureDetail,
        ) { _ in
            Button(lang.string(.commonOk), role: .cancel) {}
        } message: { detail in
            Text(detail)
        }
    }

    private func clearLearningRecords() {
        guard !isClearing else { return }
        isClearing = true
        Task {
            defer { isClearing = false }
            do {
                try await userData.clearLearningRecords()
            } catch {
                clearFailureDetail = error.localizedDescription
            }
        }
    }
}
