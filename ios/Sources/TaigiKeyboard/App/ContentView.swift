import KeyboardKit
import SwiftUI

/// Main content view.
///
/// TabView container with 4 tabs: Home, Layout, Dictionary, Settings.
struct ContentView: View {
    @State private var selectedTab: TabType = .home
    @ObservedObject var viewModel: SetupGuideViewModel

    var body: some View {
        TabView(selection: $selectedTab) {
            // HomeTab: Home
            HomeTab(viewModel: viewModel)
                .tabItem {
                    Label {
                        Text(TabType.home.title)
                    } icon: {
                        Image(latinSystemName: TabType.home.icon)
                    }
                }
                .tag(TabType.home)

            // LayoutTab: Layout
            LayoutTab()
                .tabItem {
                    Label {
                        Text(TabType.layout.title)
                    } icon: {
                        Image(latinSystemName: TabType.layout.icon)
                    }
                }
                .tag(TabType.layout)

            // DictionaryTab: Dictionary
            DictionaryTab()
                .tabItem {
                    Label {
                        Text(TabType.dictionary.title)
                    } icon: {
                        Image(latinSystemName: TabType.dictionary.icon)
                    }
                }
                .tag(TabType.dictionary)

            // SettingsTab: Settings
            SettingsTab()
                .tabItem {
                    Label {
                        Text(TabType.settings.title)
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
    }
}
