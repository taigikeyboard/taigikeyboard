// 詞頻: how often the user has committed each word.

import SwiftUI
import UniformTypeIdentifiers

@MainActor
@Observable
final class FrequencyDataPageModel {
    static let displayLimit = 100

    private(set) var rows: [FrequencyRow] = []
    private(set) var activity: UserDataPageActivity = .idle
    var filter = ""
    var message: UserDataPageMessage?

    /// Which load the rows on screen came from. A query runs off the main
    /// actor and cannot be cancelled once it is on the store's queue, so a
    /// load started under an older filter can still come back after a newer
    /// one has — and would put rows on screen that do not match what is in the
    /// box. The newest load wins by number, not by arrival.
    private var loadGeneration = 0

    private let store: UserFrequencyStore

    init(store: UserFrequencyStore) {
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
            message = .failure("讀袂著詞頻", error)
        }
    }

    /// Forgets one READING of one word — the pair is the identity, so 重/tāng
    /// goes without taking 重/tîng with it.
    func delete(_ row: FrequencyRow) async {
        await perform("咧刪…") { _ = try await self.store.delete(word: row.word, tl: row.tl) }
    }

    func deleteAll() async {
        await perform("咧刪…") { _ = try await self.store.deleteAll() }
    }

    func exportCSV(in window: NSWindow) async {
        activity = .working("咧匯出…")
        defer { activity = .idle }
        do {
            let csv = try await UserDataCSV.encodeFrequency(store.allRows().map {
                UserDataCSV.FrequencyCSVRow(word: $0.word, tl: $0.tl, count: $0.count)
            })
            _ = try await UserDataFilePanels.write(
                Data(csv.utf8),
                suggestedName: UserDataFilePanels.exportFileName(
                    prefix: "taigi_frequency",
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
            let decoded = try await Task.detached { () throws -> [UserDataCSV.FrequencyCSVRow]? in
                guard let text = try String(data: Data(contentsOf: url), encoding: .utf8) else {
                    return nil
                }
                return UserDataCSV.decodeFrequency(text)
            }.value
            guard let parsed = decoded else {
                message = .notUTF8()
                return
            }
            let processed = try await store.batchImportMerge(parsed.map {
                FrequencyRow(word: $0.word, tl: $0.tl, count: $0.count, lastUsedMillis: 0)
            })
            // "處理" rather than "新增": the merge keeps whichever count is
            // higher, so a row that changed nothing is still a row that was
            // read and considered.
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
            message = .failure("寫袂入詞頻", error)
        }
    }
}

struct FrequencyDataPage: View {
    @State private var model: FrequencyDataPageModel
    @AppStorage(SettingsStore.Keys.isFrequencyRecordingEnabled.name)
    private var isRecordingEnabled = SettingsStore.Keys.isFrequencyRecordingEnabled.defaultValue

    init(store: UserFrequencyStore) {
        _model = State(initialValue: FrequencyDataPageModel(store: store))
    }

    var body: some View {
        Form {
            Section {
                Toggle("記錄選字詞頻", isOn: $isRecordingEnabled)
            } footer: {
                Text("關掉了後袂閣學新的,已經學著的猶原會影響排序。")
            }

            Section {
                UserDataFilterField(prompt: "揣漢字抑是羅馬字", text: $model.filter)
                if model.rows.isEmpty {
                    Text(model.filter.isEmpty ? "猶未學著半字。" : "無合的字。")
                        .foregroundStyle(.secondary)
                }
                ForEach(model.rows, id: \.identity) { row in
                    HStack {
                        VStack(alignment: .leading, spacing: 2) {
                            if !row.tl.isEmpty {
                                Text(row.tl)
                                    .font(.caption)
                                    .foregroundStyle(.secondary)
                            }
                            Text(row.word)
                        }
                        Spacer()
                        Text("\(row.count)")
                            .foregroundStyle(.secondary)
                            .monospacedDigit()
                    }
                    .contextMenu {
                        Button("袂記得這字", role: .destructive) {
                            Task { await model.delete(row) }
                        }
                    }
                }
            } header: {
                Text("學著的字")
            }

            UserDataActionsSection(
                exportTitle: "匯出 CSV",
                importTitle: "匯入 CSV",
                clearTitle: "清掉全部詞頻",
                clearConfirmation: "beh 清掉全部詞頻?",
                onExport: { Task { await UserDataFilePanels.withSettingsWindow(model.exportCSV) } },
                onImport: { Task { await UserDataFilePanels.withSettingsWindow(model.importCSV) } },
                onClear: { Task { await model.deleteAll() } },
            )
        }
        .formStyle(.grouped)
        .navigationTitle("詞頻")
        .reloadWhenFilterSettles(model.filter) { await model.load() }
        .userDataPageChrome(activity: model.activity, message: $model.message)
    }
}

extension FrequencyRow {
    /// The pair the row IS, for `ForEach` identity. Not the 漢字 alone: one
    /// 漢字 with two readings is two rows, and identifying them by the word
    /// would make SwiftUI treat them as duplicates of each other.
    var identity: FrequencyRowIdentity {
        FrequencyRowIdentity(word: word, tl: tl)
    }
}

struct FrequencyRowIdentity: Hashable, Sendable {
    let word: String
    let tl: String
}
