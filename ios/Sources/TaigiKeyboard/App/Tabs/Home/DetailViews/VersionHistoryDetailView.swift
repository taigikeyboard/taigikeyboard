import SwiftUI

/// Version history page showing app update changelog.
struct VersionHistoryDetailView: View {

    var body: some View {
        Form {
            ForEach(Tab1Texts.versionHistoryEntries.indices, id: \.self) { index in
                let entry = Tab1Texts.versionHistoryEntries[index]
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
        .navigationTitle(Tab1Texts.versionHistory)
        .navigationBarTitleDisplayMode(.large)
    }
}
