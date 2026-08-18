// 詞關聯: which word the user tends to type after which.

import SwiftUI
import UniformTypeIdentifiers

@MainActor
@Observable
final class AssociationDataPageModel {
    static let displayLimit = 100

    private(set) var rows: [AssociationRow] = []
    private(set) var activity: UserDataPageActivity = .idle
    var filter = ""
    var message: UserDataPageMessage?

    /// Which load the rows on screen came from. A query runs off the main
    /// actor and cannot be cancelled once it is on the store's queue, so a
    /// load started under an older filter can still come back after a newer
    /// one has — and would put rows on screen that do not match what is in the
    /// box. The newest load wins by number, not by arrival.
    private var loadGeneration = 0

    private let store: UserAssociationStore

    init(store: UserAssociationStore) {
        self.store = store
    }

    func load() async {
        loadGeneration += 1
        let generation = loadGeneration
        do {
            let loaded = try await store.rows(filter: filter, limit: Self.displayLimit)
            guard generation == loadGeneration else { return }
            rows = loaded
        } catch {
            message = .failure(.macosAssociationReadFailed, error)
        }
    }

    func delete(_ row: AssociationRow) async {
        await perform(.macosProgressDeleting) { _ = try await self.store.delete(row.pair) }
    }

    func deleteAll() async {
        await perform(.macosProgressDeleting) { _ = try await self.store.deleteAll() }
    }

    func exportCSV(in window: NSWindow) async {
        activity = .working(.macosProgressExporting)
        defer { activity = .idle }
        do {
            let csv = try await UserDataCSV.encodeAssociation(store.rows().map {
                UserDataCSV.AssociationCSVRow(
                    previousWord: $0.pair.previous,
                    previousTl: $0.pair.previousTl,
                    nextWord: $0.pair.next,
                    nextTl: $0.pair.nextTl,
                    count: $0.count,
                )
            })
            _ = try await UserDataFilePanels.write(
                Data(csv.utf8),
                suggestedName: UserDataFilePanels.exportFileName(
                    prefix: "taigi_associations",
                    extension: "csv",
                ),
                contentTypes: [.commaSeparatedText],
                in: window,
            )
        } catch {
            message = .failure(.commonExportFailed, error)
        }
    }

    func importCSV(in window: NSWindow) async {
        guard let url = await UserDataFilePanels.chooseFileToOpen(
            contentTypes: [.commaSeparatedText, .plainText],
            in: window,
        ) else { return }

        activity = .working(.macosProgressImporting)
        defer { activity = .idle }
        do {
            // Off the main actor: the read and the parse are both
            // proportional to the file, and the window showing the spinner is
            // the one they would otherwise block.
            let decoded = try await Task.detached { () throws -> [UserDataCSV.AssociationCSVRow]? in
                guard let text = try String(data: Data(contentsOf: url), encoding: .utf8) else {
                    return nil
                }
                return UserDataCSV.decodeAssociation(text)
            }.value
            guard let parsed = decoded else {
                message = .notUTF8
                return
            }
            let processed = try await store.batchImportMerge(parsed.map {
                AssociationRow(
                    pair: AssociationPair(
                        previous: $0.previousWord,
                        previousTl: $0.previousTl,
                        next: $0.nextWord,
                        nextTl: $0.nextTl,
                    ),
                    count: $0.count,
                )
            })
            message = .imported(processed, skipped: parsed.count - processed)
            await load()
        } catch {
            message = .failure(.commonImportFailed, error)
        }
    }

    private func perform(_ label: StringKey, _ body: () async throws -> Void) async {
        activity = .working(label)
        defer { activity = .idle }
        do {
            try await body()
            await load()
        } catch {
            message = .failure(.macosAssociationWriteFailed, error)
        }
    }
}

struct AssociationDataPage: View {
    @Environment(DisplayLanguageStore.self) private var language

    @State private var model: AssociationDataPageModel
    @AppStorage(SettingsStore.Keys.isAssociationRecordingEnabled.name)
    private var isRecordingEnabled = SettingsStore.Keys.isAssociationRecordingEnabled.defaultValue

    init(store: UserAssociationStore) {
        _model = State(initialValue: AssociationDataPageModel(store: store))
    }

    var body: some View {
        Form {
            Section {
                Toggle(language.string(.dictionaryAssociationRecordingEnabled), isOn: $isRecordingEnabled)
            } footer: {
                Text(language.string(.macosAssociationNotUsedFooter))
            }

            Section {
                UserDataFilterField(text: $model.filter)
                if model.rows.isEmpty {
                    Text(language.string(model.filter.isEmpty ? .dictionaryNoData : .dictionaryNoResults))
                        .foregroundStyle(.secondary)
                }
                ForEach(model.rows, id: \.pair) { row in
                    HStack {
                        VStack(alignment: .leading, spacing: 2) {
                            Text("\(row.pair.previousTl) → \(row.pair.nextTl)")
                                .font(.subheadline)
                                .foregroundStyle(.secondary)
                            Text("\(row.pair.previous) → \(row.pair.next)")
                        }
                        Spacer()
                        Text("\(row.count)")
                            .foregroundStyle(.secondary)
                            .monospacedDigit()
                    }
                    .contextMenu {
                        Button(language.string(.macosForgetPair), role: .destructive) {
                            Task { await model.delete(row) }
                        }
                    }
                }
            } header: {
                Text(language.string(.macosLearnedPairsSection))
            }

            UserDataActionsSection(
                exportTitle: .dictionaryAssociationExportCSV,
                importTitle: .dictionaryAssociationImportCSV,
                clearTitle: .dictionaryClearAllAssociation,
                clearConfirmation: .dictionaryClearAssociationMessage,
                onExport: { Task { await UserDataFilePanels.withSettingsWindow(model.exportCSV) } },
                onImport: { Task { await UserDataFilePanels.withSettingsWindow(model.importCSV) } },
                onClear: { Task { await model.deleteAll() } },
            )
        }
        .formStyle(.grouped)
        .reloadWhenFilterSettles(model.filter) { await model.load() }
        .userDataPageChrome(activity: model.activity, message: $model.message)
    }
}
