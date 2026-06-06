// 中文: Theme Tab(主題)— 鍵盤外觀/配色主頁面。目前承載既有的「外觀設定」editor;
// 中文: 後續 PR(PR-2b)會把 tab root 換成主題選擇器 Shelf,並把外觀設定降為子頁。

import SwiftUI

/// Theme tab.
///
/// Hosts the keyboard appearance / color editor. The theme-picker shelf
/// (built-in + user themes) lands in a later PR; for now this scaffolds the
/// dedicated 2nd-position tab and carries the moved `AppearanceSettingsView`.
// 中文: 主題 Tab View — 目前直接呈現外觀設定 editor。
struct ThemeTab: View {
    var body: some View {
        NavigationStack {
            AppearanceSettingsView()
        }
    }
}
