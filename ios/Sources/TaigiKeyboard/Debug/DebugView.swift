import SwiftUI

/// Debug Zone 主頁面
///
/// 顯示使用者學習資料，包括：
/// - User Frequency（詞頻）
/// - User Association（詞關聯）
///
/// 只在 DEBUG 模式下可進入
struct DebugView: View {
    @Environment(\.dismiss) var dismiss
    @State private var selectedTab = 0

    var body: some View {
        NavigationView {
            VStack(spacing: 0) {
                // Tab 選擇器
                Picker("", selection: $selectedTab) {
                    Text("User Frequency").tag(0)
                    Text("User Association").tag(1)
                }
                .pickerStyle(.segmented)
                .padding(.horizontal, 16)
                .padding(.vertical, 12)

                // 內容區域
                TabView(selection: $selectedTab) {
                    DebugFrequencyView()
                        .tag(0)

                    DebugAssociationView()
                        .tag(1)
                }
                .tabViewStyle(.page(indexDisplayMode: .never))
            }
            .background(Color.Theme.surfacePrimary)
            .navigationTitle("Debug Zone")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarLeading) {
                    Button(action: { dismiss() }) {
                        Image(systemName: "xmark")
                            .foregroundColor(Color.Theme.textPrimary)
                    }
                }
            }
        }
    }
}

#if DEBUG
struct DebugView_Previews: PreviewProvider {
    static var previews: some View {
        DebugView()
    }
}
#endif
