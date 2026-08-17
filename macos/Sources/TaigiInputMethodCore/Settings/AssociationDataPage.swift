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
            message = .failure("讀袂著詞關聯", error)
        }
    }

    func delete(_ row: AssociationRow) async {
        await perform("咧刪…") { _ = try await self.store.delete(row.pair) }
    }

    func deleteAll() async {
        await perform("咧刪…") { _ = try await self.store.deleteAll() }
    }

    func exportCSV(in window: NSWindow) async {
        activity = .working("咧匯出…")
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
            message = .failure("匯出失敗", error)
        }
    }

    func importCSV(in window: NSWindow) async {
        guard let url = await UserDataFilePanels.chooseFileToOpen(
            contentTypes: [.commaSeparatedText, .plainText],
            in: window,
        ) else { return }

        activity = .working("咧匯入…")
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
                message = .notUTF8()
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
            message = UserDataPageMessage(
                title: "匯入完成",
                detail: "處理 \(processed) 筆,略過 \(parsed.count - processed) 筆。",
            )
            await load()
        } catch {
            message = .failure("匯入失敗", error)
        }
    }

    private func perform(_ label: String, _ body: () async throws -> Void) async {
        activity = .working(label)
        defer { activity = .idle }
        do {
            try await body()
            await load()
        } catch {
            message = .failure("寫袂入詞關聯", error)
        }
    }
}

struct AssociationDataPage: View {
    @State private var model: AssociationDataPageModel
    @AppStorage(SettingsStore.Keys.isAssociationRecordingEnabled.name)
    private var isRecordingEnabled = SettingsStore.Keys.isAssociationRecordingEnabled.defaultValue

    init(store: UserAssociationStore) {
        _model = State(initialValue: AssociationDataPageModel(store: store))
    }

    var body: some View {
        Form {
            Section {
                Toggle("記錄詞語關聯", isOn: $isRecordingEnabled)
            } footer: {
                Text("台語鍵盤佇 macOS 猶未用關聯來推薦後一个詞,毋過學著的資料會先留咧。")
            }

            Section {
                UserDataFilterField(prompt: "揣漢字抑是羅馬字", text: $model.filter)
                if model.rows.isEmpty {
                    Text(model.filter.isEmpty ? "猶未學著半組。" : "無合的組。")
                        .foregroundStyle(.secondary)
                }
                ForEach(model.rows, id: \.pair) { row in
                    HStack {
                        VStack(alignment: .leading, spacing: 2) {
                            Text("\(row.pair.previousTl) → \(row.pair.nextTl)")
                                .font(.caption)
                                .foregroundStyle(.secondary)
                            Text("\(row.pair.previous) → \(row.pair.next)")
                        }
                        Spacer()
                        Text("\(row.count)")
                            .foregroundStyle(.secondary)
                            .monospacedDigit()
                    }
                    .contextMenu {
                        Button("袂記得這組", role: .destructive) {
                            Task { await model.delete(row) }
                        }
                    }
                }
            } header: {
                Text("學著的詞組")
            }

            UserDataActionsSection(
                exportTitle: "匯出 CSV",
                importTitle: "匯入 CSV",
                clearTitle: "清掉全部關聯",
                clearConfirmation: "beh 清掉全部詞關聯?",
                onExport: { Task { await UserDataFilePanels.withSettingsWindow(model.exportCSV) } },
                onImport: { Task { await UserDataFilePanels.withSettingsWindow(model.importCSV) } },
                onClear: { Task { await model.deleteAll() } },
            )
        }
        .formStyle(.grouped)
        .navigationTitle("詞關聯")
        .reloadWhenFilterSettles(model.filter) { await model.load() }
        .userDataPageChrome(activity: model.activity, message: $model.message)
    }
}
