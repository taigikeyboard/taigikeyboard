// 自訂詞庫: the words the user added themselves.

import SwiftUI
import UniformTypeIdentifiers

/// Reads and writes the custom dictionary on behalf of the page.
///
/// Rows are fetched bounded (`filter` + `LIMIT`) and replaced on every change
/// rather than held: this window outlives every visit to the page, and a
/// 30000-row dictionary kept in a view model would stay in memory for the life
/// of the input method.
@MainActor
@Observable
final class CustomDictionaryPageModel {
    /// What the list shows before the user filters. iOS shows the same number,
    /// and the point is the same: the list is for finding a word, and the CSV
    /// export is for reading all of them.
    static let displayLimit = 100

    private(set) var rows: [CustomDictionaryRow] = []
    private(set) var totalCount = 0
    private(set) var activity: UserDataPageActivity = .idle
    var filter = ""
    var message: UserDataPageMessage?

    /// Which load the rows on screen came from. A query runs off the main
    /// actor and cannot be cancelled once it is on the store's queue, so a
    /// load started under an older filter can still come back after a newer
    /// one has — and would put rows on screen that do not match what is in the
    /// box. The newest load wins by number, not by arrival.
    private var loadGeneration = 0

    private let store: CustomDictionaryStore

    init(store: CustomDictionaryStore) {
        self.store = store
    }

    func load() async {
        loadGeneration += 1
        let generation = loadGeneration
        do {
            let loaded = try await store.rows(filter: filter, limit: Self.displayLimit)
            guard generation == loadGeneration else { return }
            rows = loaded
            totalCount = try await store.count()
        } catch {
            // Not an empty list: "the dictionary is empty" and "the dictionary
            // could not be read" look identical on screen, and only one of them
            // is worth the user doing something about.
            message = .failure("讀袂著自訂詞庫", error)
        }
    }

    func save(_ row: CustomDictionaryRow) async {
        await perform("咧存…") { try await self.store.upsert(row) }
    }

    func delete(_ row: CustomDictionaryRow) async {
        await perform("咧刪…") { _ = try await self.store.delete(id: row.id) }
    }

    func deleteAll() async {
        await perform("咧刪…") { _ = try await self.store.deleteAll() }
    }

    func exportCSV(in window: NSWindow) async {
        activity = .working("咧匯出…")
        defer { activity = .idle }
        do {
            let csv = try await CustomDictionaryCSV.encode(store.allRows())
            _ = try await UserDataFilePanels.write(
                Data(csv.utf8),
                suggestedName: UserDataFilePanels.exportFileName(
                    prefix: "taigi_custom_dictionary",
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
            // Off the main actor: reading and parsing up to 5 MB of CSV
            // there would freeze the very window that is showing the progress
            // spinner for it.
            let rows = try await Task.detached {
                try CustomDictionaryCSV.decodeFile(
                    at: url,
                    entryLimit: CustomDictionaryStore.maxEntries,
                )
            }.value
            let result = try await store.batchImport(rows)
            message = UserDataPageMessage(
                title: "匯入完成",
                detail: "新增 \(result.imported) 筆,略過 \(result.skipped) 筆。",
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
            message = .failure("寫袂入自訂詞庫", error)
        }
    }
}

struct CustomDictionaryPage: View {
    @State private var model: CustomDictionaryPageModel
    @State private var editing: CustomDictionaryRow?
    @AppStorage(SettingsStore.Keys.isCustomDictEnabled.name)
    private var isCustomDictEnabled = SettingsStore.Keys.isCustomDictEnabled.defaultValue

    init(store: CustomDictionaryStore) {
        _model = State(initialValue: CustomDictionaryPageModel(store: store))
    }

    var body: some View {
        Form {
            Section {
                Toggle("用自訂詞庫", isOn: $isCustomDictEnabled)
            } footer: {
                Text("關掉了後,家己加的詞就袂出現佇候選。")
            }

            Section {
                UserDataFilterField(prompt: "揣羅馬字抑是漢字", text: $model.filter)
                if model.rows.isEmpty {
                    Text(model.filter.isEmpty ? "猶未加半个詞。" : "無合的詞。")
                        .foregroundStyle(.secondary)
                }
                ForEach(model.rows) { row in
                    Button {
                        editing = row
                    } label: {
                        HStack {
                            Text(row.roman)
                                .foregroundStyle(.secondary)
                            Text(row.hanzi)
                            Spacer()
                        }
                        .contentShape(Rectangle())
                    }
                    .buttonStyle(.plain)
                    .contextMenu {
                        Button("刪掉", role: .destructive) {
                            Task { await model.delete(row) }
                        }
                    }
                }
            } header: {
                HStack {
                    Text("詞條")
                    Spacer()
                    Text(countLabel)
                        .foregroundStyle(.secondary)
                    Button("加詞") { editing = CustomDictionaryRow(roman: "", hanzi: "") }
                }
            }

            UserDataActionsSection(
                exportTitle: "匯出 CSV",
                importTitle: "匯入 CSV",
                clearTitle: "刪掉全部",
                clearConfirmation: "beh 刪掉全部自訂詞?",
                onExport: { Task { await UserDataFilePanels.withSettingsWindow(model.exportCSV) } },
                onImport: { Task { await UserDataFilePanels.withSettingsWindow(model.importCSV) } },
                onClear: { Task { await model.deleteAll() } },
            )
        }
        .formStyle(.grouped)
        .navigationTitle("自訂詞庫")
        .reloadWhenFilterSettles(model.filter) { await model.load() }
        .sheet(item: $editing) { row in
            CustomDictionaryEntrySheet(row: row) { edited in
                Task { await model.save(edited) }
            }
        }
        .userDataPageChrome(activity: model.activity, message: $model.message)
    }

    private var countLabel: String {
        model.totalCount > model.rows.count
            ? "\(model.rows.count) / \(model.totalCount)"
            : "\(model.totalCount)"
    }
}

/// Add or edit one entry.
///
/// A sheet rather than another pushed page: it is a two-field form the user
/// finishes and dismisses, and pushing it would put a back button where a
/// Cancel belongs.
struct CustomDictionaryEntrySheet: View {
    @Environment(\.dismiss) private var dismiss

    @State private var roman: String
    @State private var hanzi: String
    private let original: CustomDictionaryRow
    private let onSave: (CustomDictionaryRow) -> Void

    init(row: CustomDictionaryRow, onSave: @escaping (CustomDictionaryRow) -> Void) {
        original = row
        self.onSave = onSave
        _roman = State(initialValue: row.roman)
        _hanzi = State(initialValue: row.hanzi)
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            Text(original.roman.isEmpty ? "加詞" : "改詞")
                .font(.headline)
            Form {
                TextField("羅馬字", text: $roman)
                TextField("漢字", text: $hanzi)
            }
            .formStyle(.grouped)
            HStack {
                Spacer()
                Button("取消", role: .cancel) { dismiss() }
                Button("存起來") {
                    var edited = original
                    edited.roman = roman.trimmingCharacters(in: .whitespacesAndNewlines)
                    edited.hanzi = hanzi.trimmingCharacters(in: .whitespacesAndNewlines)
                    edited.updatedAt = Date()
                    onSave(edited)
                    dismiss()
                }
                .keyboardShortcut(.defaultAction)
                // A romanization is what the entry is found by; without one
                // there is nothing to store it under.
                .disabled(roman.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
            }
        }
        .padding(20)
        .frame(width: 360)
    }
}
