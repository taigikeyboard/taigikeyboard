// 備份還原: the one file that carries every piece of user data to another
// machine.

import SwiftUI
import UniformTypeIdentifiers

@MainActor
@Observable
final class DataManagementPageModel {
    private(set) var activity: UserDataPageActivity = .idle
    var message: UserDataPageMessage?

    private let backupService: BackupService

    init(stores: UserDataStores) {
        backupService = BackupService(stores: stores)
    }

    func export(in window: NSWindow) async {
        activity = .working("咧備份…")
        defer { activity = .idle }
        do {
            _ = try await UserDataFilePanels.write(
                backupService.export(),
                suggestedName: UserDataFilePanels.exportFileName(
                    prefix: "taigi_backup",
                    extension: "taigi",
                ),
                contentTypes: [UserDataFilePanels.backupContentType],
                in: window,
            )
        } catch {
            message = .failure("備份失敗", error)
        }
    }

    func restore(in window: NSWindow) async {
        guard let url = await UserDataFilePanels.chooseFileToOpen(
            contentTypes: [UserDataFilePanels.backupContentType, .json],
            in: window,
        ) else { return }

        activity = .working("咧還原…")
        defer { activity = .idle }
        do {
            let result = try await backupService.restore(from: Data(contentsOf: url))
            message = UserDataPageMessage(
                title: result.hasFailure ? "還原一部份" : "還原完成",
                detail: Self.detail(for: result),
            )
        } catch {
            message = .failure("還原失敗", error)
        }
    }

    /// One line per category, and a failed one says so.
    ///
    /// The three databases cannot be restored in one transaction, so a single
    /// number would have to stand for "nothing to restore", "everything was
    /// already there" and "it did not work" at once.
    private static func detail(for result: BackupImportResult) -> String {
        [
            line("自訂詞庫", result.customDictionary, verb: "新增"),
            line("詞頻", result.frequency, verb: "處理"),
            line("詞關聯", result.association, verb: "處理"),
        ].joined(separator: "\n")
    }

    private static func line(
        _ name: String,
        _ outcome: BackupCategoryOutcome,
        verb: String,
    ) -> String {
        switch outcome {
        case let .restored(count): "\(name):\(verb) \(count) 筆"
        case let .failed(reason): "\(name):失敗(\(reason))"
        }
    }
}

struct DataManagementPage: View {
    @State private var model: DataManagementPageModel

    init(stores: UserDataStores) {
        _model = State(initialValue: DataManagementPageModel(stores: stores))
    }

    var body: some View {
        Form {
            Section {
                Button("匯出備份…") {
                    Task { await UserDataFilePanels.withSettingsWindow(model.export) }
                }
                Button("匯入備份…") {
                    Task { await UserDataFilePanels.withSettingsWindow(model.restore) }
                }
            } header: {
                Text("備份檔案")
            } footer: {
                Text(
                    "備份內底有自訂詞庫、詞頻佮詞關聯,攏是你拍字的紀錄,請家己保管好。"
                        + "這台電腦的資料會綴 Time Machine 做備份;欲徙去別台電腦,愛用這个檔案。",
                )
            }

            Section {
                Text("匯入的時,已經有的資料袂消失:自訂詞庫加新的,詞頻佮詞關聯取較大的次數。")
                    .foregroundStyle(.secondary)
            } header: {
                Text("匯入按怎算")
            }
        }
        .formStyle(.grouped)
        .navigationTitle("備份還原")
        .userDataPageChrome(activity: model.activity, message: $model.message)
    }
}
