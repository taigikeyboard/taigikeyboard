// The 詞庫 tab shell: navigation home for the dictionary-management pages.

import SwiftUI

/// The dictionary half of the settings window.
///
/// A `NavigationStack` so the management pages (自訂詞庫 / 詞頻 / 詞關聯 /
/// 備份還原, landing in PR11–PR13) push in place, mirroring the iOS Tab3
/// structure (`ios/.../App/Tabs/Dictionary/DictionaryTab.swift`). Until they
/// land, the rows name what is coming and stay disabled — the tab exists now
/// so the window chrome, sizing and toolbar identity are settled once.
struct DictionarySettingsPane: View {
    /// The four management destinations, in the iOS Tab3 order.
    private static let dataManagementRows = ["自訂詞庫", "詞頻管理", "詞關聯管理", "備份還原"]

    var body: some View {
        NavigationStack {
            Form {
                Section {
                    ForEach(Self.dataManagementRows, id: \.self) { title in
                        LabeledContent(title) {
                            Text("即將推出")
                                .foregroundStyle(.secondary)
                        }
                    }
                } header: {
                    Text("資料管理")
                } footer: {
                    Text("詞庫管理功能會在後續更新提供。")
                }
            }
            .formStyle(.grouped)
        }
    }
}
