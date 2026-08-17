// The 詞庫 tab: which dictionaries are searched, and the user's own data.

import SwiftUI

/// The dictionary half of the settings window.
///
/// A `NavigationStack` so the four management pages push in place, mirroring
/// the iOS Tab3 structure (`ios/.../App/Tabs/Dictionary/DictionaryTab.swift`).
/// Their filter fields and actions live in the content area rather than the
/// window toolbar, which belongs to the `[一般] [詞庫]` tabs — a page putting
/// controls there would be competing with them for the same strip.
struct DictionarySettingsPane: View {
    /// The stores the pages read and write. Named here rather than reached for
    /// inside each page so the whole tab is driven by one composition root,
    /// and a test could hand it its own.
    let stores: UserDataStores

    /// Read for the search's input mode and custom-dictionary switch. Named
    /// here rather than defaulted inside the service so the whole tab is
    /// driven by one composition root.
    let settingsProvider: any EngineSettingsProvider

    @Environment(DisplayLanguageStore.self) private var language

    var body: some View {
        NavigationStack {
            Form {
                DictionarySearchSection(service: DictionarySearchService(
                    customDictionaryStore: stores.customDictionary,
                    settingsProvider: settingsProvider,
                ))

                Section {
                    NavigationLink(language.string(.dictionaryCustomDictionary)) {
                        CustomDictionaryPage(store: stores.customDictionary)
                    }
                    NavigationLink(language.string(.dictionaryFrequencyManagement)) {
                        FrequencyDataPage(store: stores.frequency)
                    }
                    NavigationLink(language.string(.dictionaryAssociationManagement)) {
                        AssociationDataPage(store: stores.association)
                    }
                    NavigationLink(language.string(.dictionaryBackupRestore)) {
                        DataManagementPage(stores: stores)
                    }
                } header: {
                    Text(language.string(.dictionaryDataManagement))
                }

                Section {
                    NavigationLink(language.string(.macosDictionarySourcesLink)) {
                        DictionaryTogglesView()
                    }
                } header: {
                    Text(language.string(.macosDictionarySourcesSection))
                } footer: {
                    Text(language.string(.macosDictionarySourcesFooter))
                }
            }
            .formStyle(.grouped)
            .navigationTitle(language.string(.navTabDictionary))
        }
    }
}
