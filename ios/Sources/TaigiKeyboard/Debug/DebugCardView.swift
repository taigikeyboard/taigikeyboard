import SwiftUI

/// Debug Zone 入口卡片（僅 DEBUG 模式顯示）
struct DebugCardView: View {
    @Binding var showDebug: Bool

    var body: some View {
        Button {
            showDebug = true
        } label: {
            Label("Debug Zone", systemImage: "ladybug")
        }
    }
}

#if DEBUG
struct DebugCardView_Previews: PreviewProvider {
    static var previews: some View {
        Form {
            DebugCardView(showDebug: .constant(false))
        }
    }
}
#endif
