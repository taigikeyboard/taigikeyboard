import SwiftUI

/// Debug Zone - 詞關聯列表頁面
struct DebugAssociationView: View {
    @State private var allData: [NextWordService.AssociationEntry] = []
    @State private var filteredData: [NextWordService.AssociationEntry] = []
    @State private var searchText = ""
    @State private var isLoading = true
    @State private var showClearAlert = false

    var body: some View {
        VStack(spacing: 0) {
            // 搜尋框
            HStack {
                Image(systemName: "magnifyingglass")
                    .foregroundColor(.secondary)
                TextField("Search", text: $searchText)
                    .textFieldStyle(.plain)
                    .onChange(of: searchText) { _, newValue in
                        filterData(query: newValue)
                    }
                if !searchText.isEmpty {
                    Button(action: { searchText = "" }) {
                        Image(systemName: "xmark.circle.fill")
                            .foregroundColor(.secondary)
                    }
                }
            }
            .padding(12)
            .background(Color(.secondarySystemBackground))
            .cornerRadius(10)
            .padding(.horizontal, 16)
            .padding(.top, 8)

            // 統計資訊
            HStack {
                Text("Total: \(filteredData.count) / \(allData.count) entries")
                    .font(.caption)
                    .foregroundColor(.secondary)
                Spacer()
                Button(action: { showClearAlert = true }) {
                    Text("Clear All")
                        .font(.caption)
                        .foregroundColor(.red)
                }
            }
            .padding(.horizontal, 16)
            .padding(.vertical, 8)

            // 列表
            if isLoading {
                Spacer()
                ProgressView()
                Spacer()
            } else if filteredData.isEmpty {
                Spacer()
                Text("No data")
                    .foregroundColor(.secondary)
                Spacer()
            } else {
                List {
                    ForEach(filteredData, id: \.id) { item in
                        VStack(alignment: .leading, spacing: 4) {
                            HStack {
                                Text("\(item.prevWord) → \(item.nextWord)")
                                Spacer()
                                Text("\(item.count)")
                                    .foregroundColor(.secondary)
                                    .font(.callout)
                            }
                            if !item.nextTl.isEmpty {
                                Text("TL: \(item.nextTl)")
                                    .font(.caption)
                                    .foregroundColor(.secondary)
                            }
                        }
                    }
                }
                .listStyle(.plain)
            }
        }
        .onAppear {
            loadData()
        }
        .alert("Clear All Data", isPresented: $showClearAlert) {
            Button("Cancel", role: .cancel) {}
            Button("Clear", role: .destructive) {
                clearData()
            }
        } message: {
            Text("Are you sure you want to clear all association data?")
        }
    }

    private func loadData() {
        isLoading = true
        Task {
            let data = await NextWordService.shared.allAssociations()
            await MainActor.run {
                allData = data
                filteredData = data
                isLoading = false
            }
        }
    }

    private func filterData(query: String) {
        if query.isEmpty {
            filteredData = allData
        } else {
            filteredData = allData.filter { item in
                item.prevWord.localizedCaseInsensitiveContains(query) ||
                item.nextWord.localizedCaseInsensitiveContains(query) ||
                item.nextTl.localizedCaseInsensitiveContains(query)
            }
        }
    }

    private func clearData() {
        Task {
            await NextWordService.shared.clearAllAssociations()
            await MainActor.run {
                allData = []
                filteredData = []
            }
        }
    }
}

// MARK: - Identifiable Extension

extension NextWordService.AssociationEntry: Identifiable {
    var id: String {
        "\(prevWord)-\(nextWord)"
    }
}

#if DEBUG
struct DebugAssociationView_Previews: PreviewProvider {
    static var previews: some View {
        DebugAssociationView()
    }
}
#endif
