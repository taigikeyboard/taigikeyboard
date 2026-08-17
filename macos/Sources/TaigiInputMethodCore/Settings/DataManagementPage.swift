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
        activity = .working(.macosProgressBackingUp)
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
            message = .failure(.macosBackupFailed, error)
        }
    }

    func restore(in window: NSWindow) async {
        guard let url = await UserDataFilePanels.chooseFileToOpen(
            contentTypes: [UserDataFilePanels.backupContentType, .json],
            in: window,
        ) else { return }

        activity = .working(.macosProgressRestoring)
        defer { activity = .idle }
        do {
            let result = try await backupService.restore(from: Data(contentsOf: url))
            message = .restored(result)
        } catch {
            message = .failure(.macosRestoreFailed, error)
        }
    }
}

struct DataManagementPage: View {
    @Environment(DisplayLanguageStore.self) private var language

    @State private var model: DataManagementPageModel

    init(stores: UserDataStores) {
        _model = State(initialValue: DataManagementPageModel(stores: stores))
    }

    var body: some View {
        Form {
            Section {
                Button(language.string(.dictionaryExportBackup)) {
                    Task { await UserDataFilePanels.withSettingsWindow(model.export) }
                }
                Button(language.string(.dictionaryImportBackup)) {
                    Task { await UserDataFilePanels.withSettingsWindow(model.restore) }
                }
            } header: {
                Text(language.string(.macosBackupFileSection))
            } footer: {
                Text(language.string(.macosBackupFooter))
            }

            Section {
                Text(language.string(.macosImportRulesBody))
                    .foregroundStyle(.secondary)
            } header: {
                Text(language.string(.macosImportRulesSection))
            }
        }
        .formStyle(.grouped)
        .userDataPageChrome(activity: model.activity, message: $model.message)
    }
}
