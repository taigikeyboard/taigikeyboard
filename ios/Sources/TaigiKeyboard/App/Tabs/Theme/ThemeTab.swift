// 中文: Theme Tab(主題)— 鍵盤外觀/配色主頁面。root = 主題選擇器 Shelf(PR-2b);
// 中文: 既有「外觀設定」editor 降為 Shelf 下的子頁(自訂外觀設定)。使用者自訂主題(+ 號)於 PR-3 加入。

import SwiftUI

/// Theme tab.
///
/// Root is the theme-picker shelf (`ThemePickerView`): pick a built-in theme or
/// `Default`. The full appearance editor (`AppearanceSettingsView`) is reached
/// via a link from the shelf. User-created themes ("+") land in PR-3.
// 中文: 主題 Tab View — 呈現主題選擇器 Shelf。
struct ThemeTab: View {
    var body: some View {
        NavigationStack {
            ThemePickerView()
        }
    }
}
