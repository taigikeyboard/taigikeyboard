// 主 App 的 TabView 容器,管 Home / Theme / Layout / Dictionary / Settings 五個 tab。

import KeyboardKit
import SwiftUI

/// Main content view.
///
/// TabView container with 5 tabs: Home, Theme, Layout, Dictionary, Settings.
// 主畫面 TabView,接收 SetupGuideViewModel 用於 Home tab 的鍵盤狀態追蹤。
// 監聽 .switchToSettingsTab 通知以支援 deep link 切到設定頁。
struct ContentView: View {
    @State private var selectedTab: TabType = .home
    @ObservedObject var viewModel: SetupGuideViewModel
    @Environment(DisplayLanguageStore.self) private var lang

    var body: some View {
        TabView(selection: $selectedTab) {
            // HomeTab: Home
            HomeTab(viewModel: viewModel)
                .tabItem {
                    Label {
                        Text(lang.string(TabType.home.titleKey))
                    } icon: {
                        Image(latinSystemName: TabType.home.icon)
                    }
                }
                .tag(TabType.home)

            // ThemeTab: Theme (2nd position — keyboard appearance / color theme)
            ThemeTab()
                .tabItem {
                    Label {
                        Text(lang.string(TabType.theme.titleKey))
                    } icon: {
                        Image(latinSystemName: TabType.theme.icon)
                    }
                }
                .tag(TabType.theme)

            // LayoutTab: Layout
            LayoutTab()
                .tabItem {
                    Label {
                        Text(lang.string(TabType.layout.titleKey))
                    } icon: {
                        Image(latinSystemName: TabType.layout.icon)
                    }
                }
                .tag(TabType.layout)

            // DictionaryTab: Dictionary
            DictionaryTab()
                .tabItem {
                    Label {
                        Text(lang.string(TabType.dictionary.titleKey))
                    } icon: {
                        Image(latinSystemName: TabType.dictionary.icon)
                    }
                }
                .tag(TabType.dictionary)

            // SettingsTab: Settings
            SettingsTab()
                .tabItem {
                    Label {
                        Text(lang.string(TabType.settings.titleKey))
                    } icon: {
                        Image(latinSystemName: TabType.settings.icon)
                    }
                }
                .tag(TabType.settings)
        }
        .environment(\.font, AppStyle.bodyFont)
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
            .environment(DisplayLanguageStore())
    }
}
