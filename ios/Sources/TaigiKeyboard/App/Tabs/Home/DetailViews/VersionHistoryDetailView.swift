import SwiftUI

/// Version history page showing app update changelog.
struct VersionHistoryDetailView: View {
    @Environment(DisplayLanguageStore.self) private var lang

    var body: some View {
        Form {
            ForEach(VersionHistory.entries.indices, id: \.self) { index in
                let entry = VersionHistory.entries[index]
                Section {
                    VStack(alignment: .leading, spacing: 12) {
                        // Version number and date
                        HStack {
                            Text("v\(entry.version)")
                                .font(AppStyle.headlineFont)
                                .foregroundColor(AppStyle.accentBlue)

                            Spacer()

                            Text(entry.date)
                                .font(AppStyle.captionFont)
                                .foregroundColor(.secondary)
                        }

                        // Change list
                        VStack(alignment: .leading, spacing: 8) {
                            ForEach(entry.changes.indices, id: \.self) { changeIndex in
                                HStack(alignment: .top, spacing: 8) {
                                    Text("•")
                                        .foregroundColor(.secondary)
                                    Text(entry.changes[changeIndex])
                                }
                            }
                        }
                    }
                }
            }
        }
        .navigationTitle(lang.string(.homeVersionHistory))
        .navigationBarTitleDisplayMode(.large)
    }
}
