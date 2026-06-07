// 中文: Theme Tab(主題)— 鍵盤外觀/配色主頁面。root = 主題選擇器 gallery(ThemePickerView);
// 中文: 既有「外觀設定」editor 經 Custom Themes 的「Create New…」卡進入。

import SwiftUI

/// Theme tab.
///
/// Root is the theme-picker gallery (`ThemePickerView`): a `Custom Themes` shelf
/// whose `Create New…` card links into the appearance editor
/// (`AppearanceSettingsView`), plus placeholder category shelves. The page is
/// layout-only for now; real themes return in a later pass.
// 中文: 主題 Tab View — 呈現主題選擇器 gallery。
struct ThemeTab: View {
    var body: some View {
        NavigationStack {
            ThemePickerView()
        }
    }
}
