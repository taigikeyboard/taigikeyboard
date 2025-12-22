import SwiftUI

/// Debug Zone - 詞頻列表頁面
struct DebugFrequencyView: View {
    @State private var allData: [(word: String, count: Int)] = []
    @State private var filteredData: [(word: String, count: Int)] = []
    @State private var searchText = ""
    @State private var isLoading = true
    @State private var showClearAlert = false

    var body: some View {
        VStack(spacing: 0) {
            // 搜尋框
            HStack {
                Image(systemName: "magnifyingglass")
                    .foregroundColor(Color.Theme.textSecondary)
                TextField("Search", text: $searchText)
                    .textFieldStyle(.plain)
                    .onChange(of: searchText) { _, newValue in
                        filterData(query: newValue)
                    }
                if !searchText.isEmpty {
                    Button(action: { searchText = "" }) {
                        Image(systemName: "xmark.circle.fill")
                            .foregroundColor(Color.Theme.textSecondary)
                    }
                }
            }
            .padding(12)
            .background(Color.Theme.surfaceSecondary)
            .cornerRadius(10)
            .padding(.horizontal, 16)
            .padding(.top, 8)

            // 統計資訊
            HStack {
                Text("Total: \(filteredData.count) / \(allData.count) entries")
                    .font(.caption)
                    .foregroundColor(Color.Theme.textSecondary)
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
                    .foregroundColor(Color.Theme.textSecondary)
                Spacer()
            } else {
                List {
                    ForEach(filteredData, id: \.word) { item in
                        HStack {
                            Text(item.word)
                                .foregroundColor(Color.Theme.textPrimary)
                            Spacer()
                            Text("\(item.count)")
                                .foregroundColor(Color.Theme.textSecondary)
                                .font(.callout)
                        }
                        .listRowBackground(Color.Theme.surfacePrimary)
                    }
                }
                .listStyle(.plain)
            }
        }
        .background(Color.Theme.surfacePrimary)
        .onAppear {
            loadData()
        }
        .alert("Clear All Data", isPresented: $showClearAlert) {
            Button("Cancel", role: .cancel) {}
            Button("Clear", role: .destructive) {
                clearData()
            }
        } message: {
            Text("Are you sure you want to clear all frequency data?")
        }
    }

    private func loadData() {
        isLoading = true
        Task {
            // 使用 async 版本確保資料庫初始化
            let data = await UserFrequencyRepository.shared.getTopWordsAsync(limit: 10000)
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
                item.word.localizedCaseInsensitiveContains(query)
            }
        }
    }

    private func clearData() {
        do {
            try UserFrequencyService.deleteUserDatabase()
            allData = []
            filteredData = []
        } catch {
            // 忽略錯誤
        }
    }
}

#if DEBUG
struct DebugFrequencyView_Previews: PreviewProvider {
    static var previews: some View {
        DebugFrequencyView()
    }
}
#endif
