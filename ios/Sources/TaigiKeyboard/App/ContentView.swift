import SwiftUI
import KeyboardKit

/// 主 APP 內容視圖
/// 使用 TabView 架構，包含 4 個主要 Tab
struct ContentView: View {
    @State private var selectedTab: TabType = .home
    @ObservedObject var viewModel: SetupGuideViewModel
    @StateObject private var languageManager = LanguageManager.shared

    var body: some View {
        TabView(selection: $selectedTab) {
            // Tab1: 頭頁
            Tab1(viewModel: viewModel)
                .tabItem {
                    Label {
                        Text(languageManager.text(TabType.home.title))
                    } icon: {
                        Image(systemName: TabType.home.icon)
                    }
                }
                .tag(TabType.home)

            // Tab2: 佈局
            Tab2()
                .tabItem {
                    Label {
                        Text(languageManager.text(TabType.layout.title))
                    } icon: {
                        Image(systemName: TabType.layout.icon)
                    }
                }
                .tag(TabType.layout)

            // Tab3: 詞庫
            Tab3()
                .tabItem {
                    Label {
                        Text(languageManager.text(TabType.dictionary.title))
                    } icon: {
                        Image(systemName: TabType.dictionary.icon)
                    }
                }
                .tag(TabType.dictionary)

            // Tab4: 設定
            Tab4()
                .tabItem {
                    Label {
                        Text(languageManager.text(TabType.settings.title))
                    } icon: {
                        Image(systemName: TabType.settings.icon)
                    }
                }
                .tag(TabType.settings)
        }
        .tint(Color.Theme.accent)
        .onReceive(NotificationCenter.default.publisher(for: .switchToSettingsTab)) { _ in
            selectedTab = .settings
        }
    }
}

struct ContentView_Previews: PreviewProvider {
    static var previews: some View {
        let bundleId = (Bundle.main.bundleIdentifier ?? "com.siansiansu.TaigiKeyboard") + ".TaigiKeyboardExtension"
        let keyboardStatus = KeyboardStatusContext(bundleId: bundleId)
        let viewModel = SetupGuideViewModel(keyboardStatus: keyboardStatus)

        ContentView(viewModel: viewModel)
    }
}
